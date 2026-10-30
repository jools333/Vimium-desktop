use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_BACK, VK_ESCAPE};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::element::{HintItem, UIElement};
use crate::core::hint::{HintMatcher, MatchResult};
use crate::platform::OverlayBackend;

const COLORKEY: u32 = 0x00FF00FF; // Magenta color key for transparency

struct OverlayState {
    matcher: HintMatcher,
    selected: Option<UIElement>,
    badge_brush: HBRUSH,
    border_pen: HPEN,
    font: HFONT,
    hwnd: HWND,
    is_closed: bool,
}

static OVERLAY_RUNNING: AtomicBool = AtomicBool::new(false);
static mut ACTIVE_OVERLAY_PTR: *mut OverlayState = std::ptr::null_mut();
static mut ACTIVE_HOOK: HHOOK = HHOOK(std::ptr::null_mut());

struct OverlayGuard;
impl Drop for OverlayGuard {
    fn drop(&mut self) {
        OVERLAY_RUNNING.store(false, Ordering::SeqCst);
        unsafe {
            if !ACTIVE_HOOK.0.is_null() {
                let _ = UnhookWindowsHookEx(ACTIVE_HOOK);
                ACTIVE_HOOK = HHOOK(std::ptr::null_mut());
            }
            ACTIVE_OVERLAY_PTR = std::ptr::null_mut();
        }
    }
}

pub struct WindowsOverlay;

impl WindowsOverlay {
    pub fn new() -> Self {
        Self
    }
}

// Low-level keyboard hook: intercepts physical keystrokes across the ENTIRE system
// completely independent of window focus or active keyboard layout (Russian/English)
unsafe extern "system" fn ll_keyboard_hook(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if code >= 0 && (wparam.0 == WM_KEYDOWN as usize || wparam.0 == WM_SYSKEYDOWN as usize) {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let vk = kb.vkCode;

            if !ACTIVE_OVERLAY_PTR.is_null() {
                let state = &mut *ACTIVE_OVERLAY_PTR;

                // Esc -> Cancel
                if vk == VK_ESCAPE.0 as u32 {
                    let _ = DestroyWindow(state.hwnd);
                    return LRESULT(1); // Consume key
                }

                // Backspace -> Erase last letter
                if vk == VK_BACK.0 as u32 {
                    state.matcher.backspace();
                    let _ = InvalidateRect(state.hwnd, None, true);
                    return LRESULT(1);
                }

                // Virtual Key codes VK_A..VK_Z (0x41..0x5A)
                // Physical keys on keyboard: always identical whether in Russian, English, etc.
                if (0x41..=0x5A).contains(&vk) {
                    let c = (b'a' + (vk - 0x41) as u8) as char;
                    match state.matcher.feed_char(c) {
                        MatchResult::Selected(element) => {
                            state.selected = Some(element);
                            let _ = DestroyWindow(state.hwnd);
                        }
                        MatchResult::Narrowed(_) => {
                            let _ = InvalidateRect(state.hwnd, None, true);
                        }
                        MatchResult::Mismatch => {}
                    }
                    return LRESULT(1); // Consume key so target app doesn't receive it
                }
            }
        }
        CallNextHookEx(ACTIVE_HOOK, code, wparam, lparam)
    }
}

unsafe extern "system" fn overlay_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut OverlayState;
            if !ptr.is_null() {
                let state = unsafe { &mut *ptr };
                let mut ps = PAINTSTRUCT::default();
                let hdc = unsafe { BeginPaint(hwnd, &mut ps) };

                // 1. Fill background with magenta (colorkey -> transparent)
                let bg_brush = unsafe { CreateSolidBrush(COLORREF(COLORKEY)) };
                let mut rect = RECT::default();
                unsafe {
                    let _ = GetClientRect(hwnd, &mut rect);
                    FillRect(hdc, &rect, bg_brush);
                    let _ = DeleteObject(bg_brush);
                }

                // 2. Setup GDI for drawing badges
                let old_font = unsafe { SelectObject(hdc, state.font) };
                let old_brush = unsafe { SelectObject(hdc, state.badge_brush) };
                let old_pen = unsafe { SelectObject(hdc, state.border_pen) };
                unsafe { SetBkMode(hdc, TRANSPARENT) };

                // 3. Draw active hints
                for item in state.matcher.items() {
                    if !state.matcher.is_hint_active(&item.hint) {
                        continue;
                    }

                    let elem_x = item.element.rect.x;
                    let elem_y = item.element.rect.y;

                    // Calculate badge dimensions
                    let text = item.hint.to_uppercase();
                    let text_len = text.len() as i32;
                    let badge_w = (text_len * 12 + 12).max(24);
                    let badge_h = 22;

                    let badge_rect = RECT {
                        left: elem_x,
                        top: elem_y,
                        right: elem_x + badge_w,
                        bottom: elem_y + badge_h,
                    };

                    // Draw yellow rounded rectangle badge
                    unsafe {
                        let _ = RoundRect(
                            hdc,
                            badge_rect.left,
                            badge_rect.top,
                            badge_rect.right,
                            badge_rect.bottom,
                            4,
                            4,
                        );
                    }

                    let typed_len = state.matcher.input_buffer.len();
                    let text_wide: Vec<u16> = text.encode_utf16().collect();

                    if typed_len > 0 && typed_len <= text.len() {
                        // Highlight matched prefix in bright red-orange
                        unsafe { SetTextColor(hdc, COLORREF(0x000045FF)) };
                        let mut prefix_rect = badge_rect;
                        prefix_rect.left += 4;
                        let _ = unsafe {
                            DrawTextW(
                                hdc,
                                &mut text_wide[..typed_len].to_vec(),
                                &mut prefix_rect,
                                DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                            )
                        };

                        // Remaining letters in black
                        unsafe { SetTextColor(hdc, COLORREF(0x00101010)) };
                        let mut rem_rect = badge_rect;
                        rem_rect.left += 4 + (typed_len as i32 * 12);
                        let _ = unsafe {
                            DrawTextW(
                                hdc,
                                &mut text_wide[typed_len..].to_vec(),
                                &mut rem_rect,
                                DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                            )
                        };
                    } else {
                        // Entire text in bold black
                        unsafe { SetTextColor(hdc, COLORREF(0x00101010)) };
                        let mut text_rect = badge_rect;
                        let _ = unsafe {
                            DrawTextW(
                                hdc,
                                &mut text_wide.clone(),
                                &mut text_rect,
                                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                            )
                        };
                    }
                }

                // Restore previous GDI objects
                unsafe {
                    SelectObject(hdc, old_font);
                    SelectObject(hdc, old_brush);
                    SelectObject(hdc, old_pen);
                    let _ = EndPaint(hwnd, &ps);
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN => {
            // User clicked mouse outside -> cancel overlay
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // NEVER call PostQuitMessage here, because that would terminate the main tray loop!
            // Just mark is_closed = true on this overlay's state.
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut OverlayState;
            if !ptr.is_null() {
                unsafe {
                    (*ptr).is_closed = true;
                }
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

impl OverlayBackend for WindowsOverlay {
    fn show_overlay(&mut self, hints: &[HintItem]) -> Result<Option<UIElement>> {
        if hints.is_empty() {
            return Ok(None);
        }

        if OVERLAY_RUNNING.swap(true, Ordering::SeqCst) {
            return Ok(None);
        }
        let _guard = OverlayGuard;

        unsafe {
            let class_name = w!("VimiumDesktopOverlay");
            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(overlay_wndproc),
                hInstance: HINSTANCE::default(),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                lpszClassName: class_name,
                ..Default::default()
            };

            let _ = RegisterClassExW(&wnd_class);

            // Create GDI resources
            // Yellow brush: #FFF785
            let badge_brush = CreateSolidBrush(COLORREF(0x0085F7FF));
            // Dark border pen: #1A1A1A
            let border_pen = CreatePen(PS_SOLID, 1, COLORREF(0x001A1A1A));
            // Bold sans-serif font
            let font = CreateFontW(
                15, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0,
                DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32, CLIP_DEFAULT_PRECIS.0 as u32,
                CLEARTYPE_QUALITY.0 as u32, DEFAULT_PITCH.0 as u32, w!("Segoe UI"),
            );

            let mut state = OverlayState {
                matcher: HintMatcher::new(hints.to_vec()),
                selected: None,
                badge_brush,
                border_pen,
                font,
                hwnd: HWND::default(),
                is_closed: false,
            };

            // Fullscreen dimensions across all monitors
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
                class_name,
                w!("Vimium Overlay"),
                WS_POPUP,
                x,
                y,
                width,
                height,
                None,
                None,
                HINSTANCE::default(),
                None,
            )?;

            state.hwnd = hwnd;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, &mut state as *mut _ as isize);
            ACTIVE_OVERLAY_PTR = &mut state as *mut _;

            // Install low-level keyboard hook to capture physical keys across whole system
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(ll_keyboard_hook),
                HINSTANCE::default(),
                0,
            )?;
            ACTIVE_HOOK = hook;

            // Make magenta transparent
            SetLayeredWindowAttributes(hwnd, COLORREF(COLORKEY), 0, LWA_COLORKEY)?;

            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);
            let _ = SetForegroundWindow(hwnd);
            let _ = SetFocus(hwnd);

            // Modal message pump for overlay: exits when is_closed is true
            while !state.is_closed {
                let mut msg = MSG::default();
                let ret = GetMessageW(&mut msg, None, 0, 0);
                if !ret.as_bool() || msg.message == WM_QUIT {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // Remove hook immediately
            if !ACTIVE_HOOK.0.is_null() {
                let _ = UnhookWindowsHookEx(ACTIVE_HOOK);
                ACTIVE_HOOK = HHOOK(std::ptr::null_mut());
            }
            ACTIVE_OVERLAY_PTR = std::ptr::null_mut();

            // Clean up GDI
            let _ = DeleteObject(badge_brush);
            let _ = DeleteObject(border_pen);
            let _ = DeleteObject(font);

            Ok(state.selected)
        }
    }
}
