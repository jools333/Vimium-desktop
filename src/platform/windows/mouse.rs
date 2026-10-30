use anyhow::Result;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

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
            // Move cursor to center of element
            SetCursorPos(x, y)?;

            // Small delay so target window detects hover
            std::thread::sleep(std::time::Duration::from_millis(30));

            // Send left button down & up
            mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(20));
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
        }
        Ok(())
    }
}
