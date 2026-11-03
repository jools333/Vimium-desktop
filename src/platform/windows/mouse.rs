use anyhow::Result;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::platform::windows::uia::WindowsUia;
use crate::platform::MouseBackend;

pub struct WindowsMouse;

impl WindowsMouse {
    pub fn new() -> Self {
        Self
    }
}

impl MouseBackend for WindowsMouse {
    fn click_at(&self, x: i32, y: i32) -> Result<()> {
        unsafe {
            // 1. First try programmatic activation via UI Automation (cursor never moves!)
            if let Ok(uia) = WindowsUia::new() {
                if let Ok(true) = uia.activate_at(x, y) {
                    return Ok(());
                }
            }

            // 2. Hardware click fallback: save original cursor position so cursor stays in place
            let mut orig_pos = POINT::default();
            let has_orig = GetCursorPos(&mut orig_pos).is_ok();

            // Perform instant click at target
            let _ = SetCursorPos(x, y);
            mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(5));
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);

            // Immediately restore cursor to original position
            if has_orig {
                let _ = SetCursorPos(orig_pos.x, orig_pos.y);
            }
        }
        Ok(())
    }
}
