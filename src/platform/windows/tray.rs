use anyhow::Result;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

use crate::core::config::{AppConfig, HotkeyPreset};
use crate::core::hint::assign_hints;
use crate::platform::windows::autostart;
use crate::platform::windows::mouse::WindowsMouse;
use crate::platform::windows::overlay::WindowsOverlay;
use crate::platform::windows::uia::WindowsUia;
use crate::platform::{MouseBackend, OverlayBackend};

const WM_TRAY_CALLBACK: u32 = WM_USER + 101;
const HOTKEY_ID: i32 = 2001;

// Menu Command IDs
const CMD_TRIGGER: usize = 3001;
const CMD_HOTKEY_ALT_F: usize = 3002;
const CMD_HOTKEY_ALT_SPACE: usize = 3003;
const CMD_HOTKEY_ALT_SEMI: usize = 3004;
const CMD_HOTKEY_CTRL_SHIFT_F: usize = 3005;
const CMD_TOGGLE_SKIP_BROWSERS: usize = 3006;
const CMD_TOGGLE_AUTOSTART: usize = 3007;
const CMD_EXIT: usize = 3009;

struct TrayAppState {
    config: AppConfig,
    #[allow(dead_code)]
    hwnd: HWND,
    nid: NOTIFYICONDATAW,
    uia: WindowsUia,
}

static mut APP_STATE_PTR: *mut TrayAppState = std::ptr::null_mut();

pub struct TrayApplication;

impl TrayApplication {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self) -> Result<()> {
        unsafe {
            // Ensure only one instance of Vimium Desktop runs at a time
            let mutex_name = w!("Local\\VimiumDesktopSingleInstanceMutex");
            if let Ok(m) = CreateMutexW(None, true, mutex_name) {
                let already_exists = GetLastError() == ERROR_ALREADY_EXISTS;
                if already_exists {
                    eprintln!("Vimium Desktop is already running in background.");
                    let _ = CloseHandle(m);
                    return Ok(());
                }
            }

            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let class_name = w!("VimiumTrayHiddenWindowClass");
            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(tray_wndproc),
                hInstance: HINSTANCE::default(),
                lpszClassName: class_name,
                ..Default::default()
            };

            let _ = RegisterClassExW(&wnd_class);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                w!("VimiumTrayWindow"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                HWND::default(),
                HMENU::default(),
                HINSTANCE::default(),
                None,
            )?;

            let mut config = AppConfig::load();

            // Sync autostart status with registry
            let registry_autostart = autostart::is_autostart_enabled();
            if config.autostart && !registry_autostart {
                let _ = autostart::set_autostart(true);
            } else if !config.autostart && registry_autostart {
                config.autostart = true;
            }
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_INFO,
                uCallbackMessage: WM_TRAY_CALLBACK,
                hIcon: LoadIconW(None, IDI_APPLICATION)?,
                ..Default::default()
            };

            // Set tooltip text
            let tip = format!("Vimium Desktop ({})", config.preset.name());
            let tip_wide: Vec<u16> = tip.encode_utf16().collect();
            for (i, &ch) in tip_wide.iter().enumerate().take(127) {
                nid.szTip[i] = ch;
            }

            // Balloon notification on start
            let info_title = "Vimium Desktop активен"
                .encode_utf16()
                .collect::<Vec<u16>>();
            let info_text = format!(
                "Нажмите {} в любом окне для навигации",
                config.preset.name()
            )
            .encode_utf16()
            .collect::<Vec<u16>>();
            for (i, &ch) in info_title.iter().enumerate().take(63) {
                nid.szInfoTitle[i] = ch;
            }
            for (i, &ch) in info_text.iter().enumerate().take(255) {
                nid.szInfo[i] = ch;
            }
            nid.dwInfoFlags = NIIF_INFO;

            let _ = Shell_NotifyIconW(NIM_ADD, &nid);

            let uia = WindowsUia::new()?;

            let mut app_state = TrayAppState {
                config,
                hwnd,
                nid,
                uia,
            };
            APP_STATE_PTR = &mut app_state as *mut _;

            // Register global hotkey
            register_current_hotkey(hwnd, app_state.config.preset);

            println!("Vimium Desktop started in system tray.");
            println!("Active hotkey: {}", app_state.config.preset.name());
            println!("Skip browsers: {}", app_state.config.skip_browsers);

            // Main tray message pump: runs continuously
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // Clean up on exit
            let _ = UnregisterHotKey(hwnd, HOTKEY_ID);
            let _ = Shell_NotifyIconW(NIM_DELETE, &app_state.nid);
            APP_STATE_PTR = std::ptr::null_mut();

            CoUninitialize();
            Ok(())
        }
    }
}

fn register_current_hotkey(hwnd: HWND, preset: HotkeyPreset) {
    let (mods, vk) = preset.to_win32_modifiers_and_vk();
    unsafe {
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID);
        let res = RegisterHotKey(hwnd, HOTKEY_ID, HOT_KEY_MODIFIERS(mods), vk);
        if let Err(e) = res {
            eprintln!(
                "Warning: Failed to register hotkey {}: {:?}",
                preset.name(),
                e
            );
        }
    }
}

fn trigger_vimium_hints() {
    unsafe {
        if APP_STATE_PTR.is_null() {
            return;
        }
        let state = &*APP_STATE_PTR;

        let fg = GetForegroundWindow();

        // 1. High-speed scan: active window + taskbar (sound, clock, start, apps)
        // Automatically skips browsers if skip_browsers is enabled!
        let elements = match state
            .uia
            .get_elements_for_window(fg, state.config.skip_browsers)
        {
            Ok(elems) if !elems.is_empty() => elems,
            _ => return,
        };

        // 2. Generate hints
        let hints = assign_hints(elements);

        // 3. Show overlay (overlay has its own inner loop that does NOT kill the outer tray loop)
        let mut overlay = WindowsOverlay::new();
        if let Ok(Some(selected)) = overlay.show_overlay(&hints) {
            let (cx, cy) = selected.rect.center();
            let mouse = WindowsMouse::new();
            let _ = mouse.click_at(cx, cy);
        }
    }
}

unsafe extern "system" fn tray_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_HOTKEY => {
                if wparam.0 == HOTKEY_ID as usize {
                    trigger_vimium_hints();
                }
                LRESULT(0)
            }
            WM_TRAY_CALLBACK => {
                let event = lparam.0 as u32;
                if event == WM_RBUTTONUP || event == WM_LBUTTONUP {
                    show_tray_menu(hwnd);
                }
                LRESULT(0)
            }
            WM_COMMAND => {
                let cmd_id = wparam.0 & 0xFFFF;
                if !APP_STATE_PTR.is_null() {
                    let state = &mut *APP_STATE_PTR;
                    match cmd_id {
                        CMD_TRIGGER => {
                            trigger_vimium_hints();
                        }
                        CMD_HOTKEY_ALT_F => {
                            state.config.preset = HotkeyPreset::AltF;
                            state.config.save();
                            register_current_hotkey(hwnd, state.config.preset);
                            update_tray_tip(state);
                        }
                        CMD_HOTKEY_ALT_SPACE => {
                            state.config.preset = HotkeyPreset::AltSpace;
                            state.config.save();
                            register_current_hotkey(hwnd, state.config.preset);
                            update_tray_tip(state);
                        }
                        CMD_HOTKEY_ALT_SEMI => {
                            state.config.preset = HotkeyPreset::AltSemicolon;
                            state.config.save();
                            register_current_hotkey(hwnd, state.config.preset);
                            update_tray_tip(state);
                        }
                        CMD_HOTKEY_CTRL_SHIFT_F => {
                            state.config.preset = HotkeyPreset::CtrlShiftF;
                            state.config.save();
                            register_current_hotkey(hwnd, state.config.preset);
                            update_tray_tip(state);
                        }
                        CMD_TOGGLE_SKIP_BROWSERS => {
                            state.config.skip_browsers = !state.config.skip_browsers;
                            state.config.save();
                            show_tray_notification(
                                &mut state.nid,
                                "Vimium Desktop",
                                if state.config.skip_browsers {
                                    "⚡ Пропуск браузеров включен"
                                } else {
                                    "Обход браузеров включен"
                                },
                            );
                        }
                        CMD_TOGGLE_AUTOSTART => {
                            state.config.autostart = !state.config.autostart;
                            let _ = autostart::set_autostart(state.config.autostart);
                            state.config.save();
                            show_tray_notification(
                                &mut state.nid,
                                "Vimium Desktop",
                                if state.config.autostart {
                                    "🚀 Автозапуск включен (приложение запускается вместе с Windows)"
                                } else {
                                    "Автозапуск отключен"
                                },
                            );
                        }
                        CMD_EXIT => {
                            PostQuitMessage(0);
                        }
                        _ => {}
                    }
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

fn show_tray_notification(nid: &mut NOTIFYICONDATAW, title: &str, message: &str) {
    let title_wide: Vec<u16> = title.encode_utf16().collect();
    let text_wide: Vec<u16> = message.encode_utf16().collect();
    nid.szInfoTitle = [0; 64];
    nid.szInfo = [0; 256];
    for (i, &ch) in title_wide.iter().enumerate().take(63) {
        nid.szInfoTitle[i] = ch;
    }
    for (i, &ch) in text_wide.iter().enumerate().take(255) {
        nid.szInfo[i] = ch;
    }
    nid.dwInfoFlags = NIIF_INFO;
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_INFO;
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, nid);
    }
}

fn update_tray_tip(state: &mut TrayAppState) {
    let tip = format!("Vimium Desktop ({})", state.config.preset.name());
    let tip_wide: Vec<u16> = tip.encode_utf16().collect();
    state.nid.szTip = [0; 128];
    for (i, &ch) in tip_wide.iter().enumerate().take(127) {
        state.nid.szTip[i] = ch;
    }
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &state.nid);
    }
}

unsafe fn show_tray_menu(hwnd: HWND) {
    unsafe {
        if APP_STATE_PTR.is_null() {
            return;
        }
        let state = &*APP_STATE_PTR;

        let mut cursor_pos = POINT::default();
        let _ = GetCursorPos(&mut cursor_pos);

        let hmenu = CreatePopupMenu().unwrap_or_default();
        let hsubmenu = CreatePopupMenu().unwrap_or_default();

        let trigger_text = format!("🎯 Вызвать подсказки ({})", state.config.preset.name());
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            CMD_TRIGGER,
            PCWSTR(
                trigger_text
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
        );
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());

        // Hotkey selector submenu
        let check_item = |preset: HotkeyPreset, cmd: usize, label: &str| {
            let flags = if state.config.preset == preset {
                MF_STRING | MF_CHECKED
            } else {
                MF_STRING
            };
            let wide = label
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect::<Vec<_>>();
            let _ = AppendMenuW(hsubmenu, flags, cmd, PCWSTR(wide.as_ptr()));
        };

        check_item(
            HotkeyPreset::AltF,
            CMD_HOTKEY_ALT_F,
            "Alt + F (По умолчанию)",
        );
        check_item(HotkeyPreset::AltSpace, CMD_HOTKEY_ALT_SPACE, "Alt + Пробел");
        check_item(
            HotkeyPreset::AltSemicolon,
            CMD_HOTKEY_ALT_SEMI,
            "Alt + ; (Hunt-n-Peck)",
        );
        check_item(
            HotkeyPreset::CtrlShiftF,
            CMD_HOTKEY_CTRL_SHIFT_F,
            "Ctrl + Shift + F",
        );

        let submenu_title = "⚙️ Выбрать горячую клавишу"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let _ = AppendMenuW(
            hmenu,
            MF_POPUP,
            hsubmenu.0 as usize,
            PCWSTR(submenu_title.as_ptr()),
        );

        // Browser toggle option
        let browser_flags = if state.config.skip_browsers {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        let browser_title = "⚡ Пропускать браузеры (Chrome, Edge и др.)"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let _ = AppendMenuW(
            hmenu,
            browser_flags,
            CMD_TOGGLE_SKIP_BROWSERS,
            PCWSTR(browser_title.as_ptr()),
        );

        // Autostart toggle option
        let autostart_flags = if state.config.autostart {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        let autostart_title = "🚀 Автозапуск при старте Windows"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let _ = AppendMenuW(
            hmenu,
            autostart_flags,
            CMD_TOGGLE_AUTOSTART,
            PCWSTR(autostart_title.as_ptr()),
        );

        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        let exit_title = "❌ Выход"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let _ = AppendMenuW(hmenu, MF_STRING, CMD_EXIT, PCWSTR(exit_title.as_ptr()));

        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(
            hmenu,
            TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
            cursor_pos.x,
            cursor_pos.y,
            0,
            hwnd,
            None,
        );

        let _ = DestroyMenu(hsubmenu);
        let _ = DestroyMenu(hmenu);
    }
}
