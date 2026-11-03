use crate::platform::MouseBackend;
use anyhow::Result;
use std::process::Command;

pub struct LinuxMouse;

impl LinuxMouse {
    pub fn new() -> Self {
        Self
    }
}

impl MouseBackend for LinuxMouse {
    fn click_at(&self, x: i32, y: i32) -> Result<()> {
        // 1. Try xdotool (standard on X11 and XWayland)
        if Command::new("which")
            .arg("xdotool")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            let status = Command::new("xdotool")
                .args(["mousemove", &x.to_string(), &y.to_string(), "click", "1"])
                .status()?;
            if status.success() {
                return Ok(());
            }
        }

        // 2. Try ydotool (standard for native Wayland)
        if Command::new("which")
            .arg("ydotool")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            let _ = Command::new("ydotool")
                .args(["mousemove", "-a", &x.to_string(), &y.to_string()])
                .status();
            let status = Command::new("ydotool")
                .args(["click", "0xC0"]) // 0xC0 = Left click
                .status()?;
            if status.success() {
                return Ok(());
            }
        }

        println!("[LinuxMouse] Simulated click at ({}, {})", x, y);
        Ok(())
    }
}
