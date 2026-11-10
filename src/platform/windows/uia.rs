use anyhow::{Context, Result};
use std::collections::HashSet;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::StationsAndDesktops::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

use crate::core::config::BrowserMode;
use crate::core::element::{Rect, UIElement};
use crate::platform::AccessibilityBackend;

pub struct WindowsUia {
    uia: IUIAutomation,
    cache: IUIAutomationCacheRequest,
    cond: IUIAutomationCondition,
}

impl WindowsUia {
    /// Programmatically activates the element at the specified coordinates without moving the mouse cursor.
    /// Returns Ok(true) if successfully triggered via UI Automation patterns (Invoke, Select, Toggle, SetFocus),
    /// or Ok(false) if the control does not support programmatic activation.
    pub fn activate_at(&self, x: i32, y: i32) -> Result<bool> {
        unsafe {
            let point = POINT { x, y };
            let elem = match self.uia.ElementFromPoint(point) {
                Ok(e) => e,
                Err(_) => return Ok(false),
            };

            // 1. Try InvokePattern (buttons, links, menu items)
            if let Ok(pattern) = elem.GetCurrentPattern(UIA_InvokePatternId)
                && let Ok(invoke) = pattern.cast::<IUIAutomationInvokePattern>()
                && invoke.Invoke().is_ok()
            {
                return Ok(true);
            }

            // 2. Try SelectionItemPattern (tabs, list items, radio buttons)
            if let Ok(pattern) = elem.GetCurrentPattern(UIA_SelectionItemPatternId)
                && let Ok(sel) = pattern.cast::<IUIAutomationSelectionItemPattern>()
                && sel.Select().is_ok()
            {
                return Ok(true);
            }

            // 3. Try TogglePattern (checkboxes, toggle buttons)
            if let Ok(pattern) = elem.GetCurrentPattern(UIA_TogglePatternId)
                && let Ok(toggle) = pattern.cast::<IUIAutomationTogglePattern>()
                && toggle.Toggle().is_ok()
            {
                return Ok(true);
            }

            // 4. Try ExpandCollapsePattern (drop-downs, expandable tree items)
            if let Ok(pattern) = elem.GetCurrentPattern(UIA_ExpandCollapsePatternId)
                && let Ok(expand) = pattern.cast::<IUIAutomationExpandCollapsePattern>()
                && expand.Expand().is_ok()
            {
                return Ok(true);
            }

            // 5. Try SetFocus (for edit controls / address bars)
            let ctrl_type_id = elem.CurrentControlType().map(|c| c.0).unwrap_or(0);
            if ctrl_type_id == 50004 {
                // UIA_EditControlTypeId
                if elem.SetFocus().is_ok() {
                    return Ok(true);
                }
            }

            Ok(false)
        }
    }

    pub fn new() -> Result<Self> {
        Self::ensure_desktop_access();

        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)
                .context("Failed to create CUIAutomation instance")?;

            // Pre-configure high-performance batch cache request
            let cache = uia
                .CreateCacheRequest()
                .context("Failed to create cache request")?;

            cache.AddProperty(UIA_BoundingRectanglePropertyId)?;
            cache.AddProperty(UIA_ControlTypePropertyId)?;
            cache.AddProperty(UIA_NamePropertyId)?;
            cache.AddProperty(UIA_IsOffscreenPropertyId)?;

            let _ = cache.AddPattern(UIA_InvokePatternId);
            let _ = cache.AddPattern(UIA_TogglePatternId);
            let _ = cache.AddPattern(UIA_SelectionItemPatternId);
            let _ = cache.AddPattern(UIA_ExpandCollapsePatternId);

            let cond = uia
                .CreateTrueCondition()
                .context("Failed to create condition")?;

            Ok(Self { uia, cache, cond })
        }
    }

    fn ensure_desktop_access() {
        unsafe {
            if let Ok(winsta) = OpenWindowStationW(w!("WinSta0"), false, 0x037F) {
                let _ = SetProcessWindowStation(winsta);
            }
            if let Ok(desk) = OpenDesktopW(w!("Default"), DESKTOP_CONTROL_FLAGS(0), false, 0x01FF) {
                let _ = SetThreadDesktop(desk);
            }
        }
    }

    /// Detects if the given window belongs to a web browser
    pub fn is_browser_window(hwnd: HWND) -> bool {
        unsafe {
            // Check process executable name
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid > 0 {
                let open_res = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
                if let Ok(handle) = open_res {
                    let mut img_buf = [0u16; 512];
                    let mut size = img_buf.len() as u32;
                    if QueryFullProcessImageNameW(
                        handle,
                        PROCESS_NAME_FORMAT(0),
                        PWSTR(img_buf.as_mut_ptr()),
                        &mut size,
                    )
                    .is_ok()
                    {
                        let img_str =
                            String::from_utf16_lossy(&img_buf[..size as usize]).to_lowercase();
                        let _ = CloseHandle(handle);
                        return img_str.ends_with("chrome.exe")
                            || img_str.ends_with("msedge.exe")
                            || img_str.ends_with("firefox.exe")
                            || img_str.ends_with("browser.exe") // Yandex Browser
                            || img_str.ends_with("brave.exe")
                            || img_str.ends_with("opera.exe")
                            || img_str.ends_with("opera_gx.exe")
                            || img_str.ends_with("vivaldi.exe")
                            || img_str.ends_with("arc.exe")
                            || img_str.ends_with("zen.exe")
                            || img_str.ends_with("waterfox.exe")
                            || img_str.ends_with("librewolf.exe")
                            || img_str.ends_with("chromium.exe")
                            || img_str.ends_with("thorium.exe")
                            || img_str.ends_with("floorp.exe")
                            || img_str.ends_with("sidekick.exe")
                            || img_str.ends_with("tor.exe");
                    }
                    let _ = CloseHandle(handle);
                }
            }
            false
        }
    }

    /// Fast scan of elements inside a specific window using pre-cached properties
    fn scan_window_elements(
        &self,
        hwnd: HWND,
        is_taskbar: bool,
        filter_browser_content: bool,
        results: &mut Vec<UIElement>,
        seen_rects: &mut HashSet<(i32, i32, i32, i32)>,
    ) -> Result<()> {
        unsafe {
            if hwnd.0.is_null() || !IsWindowVisible(hwnd).as_bool() {
                return Ok(());
            }

            let mut win_rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut win_rect);
            let win_w = win_rect.right - win_rect.left;
            let win_h = win_rect.bottom - win_rect.top;

            if win_w < 30 || win_h < 20 {
                return Ok(());
            }

            let mut content_rects: Vec<RECT> = Vec::new();
            if filter_browser_content {
                let _ = EnumChildWindows(
                    hwnd,
                    Some(enum_child_find_content),
                    LPARAM(&mut content_rects as *mut _ as isize),
                );
            }

            let root = match self.uia.ElementFromHandle(hwnd) {
                Ok(r) => r,
                Err(_) => return Ok(()),
            };

            // Single IPC call to fetch entire element tree with all properties cached
            let elements_array =
                match root.FindAllBuildCache(TreeScope_Descendants, &self.cond, &self.cache) {
                    Ok(arr) => arr,
                    Err(_) => return Ok(()),
                };

            let count = elements_array.Length().unwrap_or(0);

            for i in 0..count {
                let elem = match elements_array.GetElement(i) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                // Cached property access (instant in-memory, 0 RPC calls)
                if elem.CachedIsOffscreen().unwrap_or(BOOL(0)).as_bool() {
                    continue;
                }

                let rect = match elem.CachedBoundingRectangle() {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                let w = rect.right - rect.left;
                let h = rect.bottom - rect.top;

                // Minimum size filter
                if w < 8 || h < 8 {
                    continue;
                }

                let ctrl_type_id = elem.CachedControlType().map(|c| c.0).unwrap_or(0);
                let name = elem.CachedName().unwrap_or_default().to_string();

                if filter_browser_content {
                    // If element is the web document container itself (UIA_DocumentControlTypeId = 50030)
                    if ctrl_type_id == 50030 {
                        content_rects.push(rect);
                        continue;
                    }

                    // Native browser UI doesn't use Hyperlinks (they belong to webpage DOM)
                    if ctrl_type_id == 50005 {
                        continue;
                    }

                    // Only allow browser UI components:
                    // 50019: TabItem (tabs)
                    // 50004: Edit (address bar / omnibox)
                    // 50000: Button (back, forward, reload, new tab, extensions, profile, menu)
                    // 50031: SplitButton
                    // 50011: MenuItem
                    let is_allowed_browser_ui = matches!(
                        ctrl_type_id,
                        50019 | 50004 | 50000 | 50031 | 50011
                    );

                    if !is_allowed_browser_ui {
                        continue;
                    }

                    // Skip full-window container panes
                    if w >= win_w - 20 && h >= win_h - 40 {
                        continue;
                    }

                    // Check if inside target window bounds
                    if rect.right <= win_rect.left
                        || rect.left >= win_rect.right
                        || rect.bottom <= win_rect.top
                        || rect.top >= win_rect.bottom
                    {
                        continue;
                    }

                    // Filter out elements residing inside the web content viewport
                    let cx = (rect.left + rect.right) / 2;
                    let cy = (rect.top + rect.bottom) / 2;

                    let in_web_content = content_rects.iter().any(|cr| {
                        cx >= cr.left && cx <= cr.right && cy >= cr.top && cy <= cr.bottom
                    });

                    if in_web_content {
                        continue;
                    }

                    // Fallback geometric filter if content_rects wasn't populated:
                    // Top header chrome is within ~180px from window top; vertical tab strip within ~260px from window left.
                    if content_rects.is_empty() {
                        let is_in_top_chrome = rect.top < win_rect.top + 180;
                        let is_in_side_chrome = rect.left < win_rect.left + 260;
                        if !is_in_top_chrome && !is_in_side_chrome {
                            continue;
                        }
                    }
                } else if is_taskbar {
                    // Taskbar: only real buttons and menu items (Start, apps, volume, clock, tray items)
                    // Skip large toolbar/pane containers like (1621x40)
                    if (ctrl_type_id != 50000 && ctrl_type_id != 50011) || w > 300 {
                        continue;
                    }
                } else {
                    // App window: filter out full-window container panes
                    if w >= win_w - 20 && h >= win_h - 40 {
                        continue;
                    }

                    // Check if inside target window bounds
                    if rect.right <= win_rect.left
                        || rect.left >= win_rect.right
                        || rect.bottom <= win_rect.top
                        || rect.top >= win_rect.bottom
                    {
                        continue;
                    }

                    // Control type IDs:
                    // 50000: Button, 50002: CheckBox, 50003: ComboBox, 50004: Edit, 50005: Hyperlink,
                    // 50007: ListItem, 50011: MenuItem, 50013: RadioButton, 50019: TabItem,
                    // 50024: TreeItem, 50031: SplitButton, 50035: HeaderItem
                    let is_interactive = matches!(
                        ctrl_type_id,
                        50000
                            | 50002
                            | 50003
                            | 50004
                            | 50005
                            | 50007
                            | 50011
                            | 50013
                            | 50019
                            | 50024
                            | 50031
                            | 50035
                    );

                    // Skip generic container types without interactive flag
                    if matches!(ctrl_type_id, 50021 | 50026 | 50032 | 50033) && !is_interactive {
                        continue;
                    }

                    // Check pattern availability from cached patterns
                    let is_invokable = elem.GetCachedPattern(UIA_InvokePatternId).is_ok();
                    let is_toggleable = elem.GetCachedPattern(UIA_TogglePatternId).is_ok();
                    let is_selectable = elem.GetCachedPattern(UIA_SelectionItemPatternId).is_ok();
                    let is_expandable = elem.GetCachedPattern(UIA_ExpandCollapsePatternId).is_ok();

                    let is_clickable = is_interactive
                        || is_invokable
                        || is_toggleable
                        || is_selectable
                        || is_expandable;

                    if !is_clickable && (name.is_empty() || w > 600 || h > 150) {
                        continue;
                    }
                }

                // Deduplicate identical or nearly identical rectangles
                let key = (rect.left / 4, rect.top / 4, w / 4, h / 4);
                if !seen_rects.insert(key) {
                    continue;
                }

                let role = match ctrl_type_id {
                    50000 => "Button",
                    50002 => "CheckBox",
                    50003 => "ComboBox",
                    50004 => "Edit",
                    50005 => "Link",
                    50007 => "ListItem",
                    50011 => "MenuItem",
                    50013 => "RadioButton",
                    50019 => "Tab",
                    50024 => "TreeItem",
                    50031 => "SplitButton",
                    _ => "Control",
                };

                results.push(UIElement::new(
                    results.len(),
                    name,
                    role,
                    Rect::new(rect.left, rect.top, w, h),
                ));
            }

            Ok(())
        }
    }

    /// Scans active window AND the system taskbar + tray simultaneously.
    /// Handles browser windows according to the selected `BrowserMode`:
    /// - `TabsAndToolbar`: scans tabs, address bar, navigation bar, while skipping web content
    /// - `SkipAll`: skips browser window completely
    /// - `Full`: scans the entire browser tree including web DOM
    pub fn get_elements_for_window(
        &self,
        target_hwnd: HWND,
        browser_mode: BrowserMode,
    ) -> Result<Vec<UIElement>> {
        let mut results = Vec::with_capacity(128);
        let mut seen_rects = HashSet::with_capacity(256);

        unsafe {
            let tray_hwnd = FindWindowW(w!("Shell_TrayWnd"), None).unwrap_or(HWND::default());

            // 1. Scan target application window
            if !target_hwnd.0.is_null() && target_hwnd != tray_hwnd {
                let is_browser = Self::is_browser_window(target_hwnd);

                let should_scan = match browser_mode {
                    BrowserMode::SkipAll => !is_browser,
                    BrowserMode::TabsAndToolbar | BrowserMode::Full => true,
                };

                if should_scan {
                    let filter_browser_content =
                        is_browser && browser_mode == BrowserMode::TabsAndToolbar;
                    let _ = self.scan_window_elements(
                        target_hwnd,
                        false,
                        filter_browser_content,
                        &mut results,
                        &mut seen_rects,
                    );
                }
            }

            // 2. Scan primary taskbar + system tray (Sound, Clock, Pinned Apps, Start)
            if !tray_hwnd.0.is_null() {
                let _ = self.scan_window_elements(
                    tray_hwnd,
                    true,
                    false,
                    &mut results,
                    &mut seen_rects,
                );
            }

            // 3. Scan secondary taskbar (multi-monitor setups) if present
            let sec_tray =
                FindWindowW(w!("Shell_SecondaryTrayWnd"), None).unwrap_or(HWND::default());
            if !sec_tray.0.is_null() && sec_tray != tray_hwnd {
                let _ = self.scan_window_elements(
                    sec_tray,
                    true,
                    false,
                    &mut results,
                    &mut seen_rects,
                );
            }
        }

        Ok(results)
    }
}

unsafe extern "system" fn enum_child_find_content(child: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let mut class_buf = [0u16; 256];
        let len = GetClassNameW(child, &mut class_buf);
        if len > 0 {
            let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
            // Chromium web viewport: Chrome_RenderWidgetHostHWND
            // Firefox web viewport: MozillaContentWindowClass
            if class_name.contains("Chrome_RenderWidgetHostHWND")
                || class_name.contains("MozillaContentWindowClass")
            {
                let mut r = RECT::default();
                if GetWindowRect(child, &mut r).is_ok()
                    && r.right > r.left + 50
                    && r.bottom > r.top + 50
                {
                    let rects = &mut *(lparam.0 as *mut Vec<RECT>);
                    rects.push(r);
                }
            }
        }
        BOOL(1)
    }
}

impl AccessibilityBackend for WindowsUia {
    fn get_interactive_elements(&self) -> Result<Vec<UIElement>> {
        unsafe {
            let fg = GetForegroundWindow();
            let config = crate::core::config::AppConfig::load();
            self.get_elements_for_window(fg, config.browser_mode)
        }
    }
}
