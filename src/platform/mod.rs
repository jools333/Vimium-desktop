use crate::core::element::{HintItem, UIElement};
use anyhow::Result;

pub trait AccessibilityBackend {
    /// Retrieve interactive/clickable elements from active window or desktop
    fn get_interactive_elements(&self) -> Result<Vec<UIElement>>;
}

pub trait MouseBackend {
    /// Simulate a mouse click at screen coordinates (x, y)
    fn click_at(&self, x: i32, y: i32) -> Result<()>;
}

pub trait OverlayBackend {
    /// Display hint badges overlay, handle user input, and return selected element (if any)
    fn show_overlay(&mut self, hints: &[HintItem]) -> Result<Option<UIElement>>;
}

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::Platform;
#[cfg(windows)]
pub use windows::autostart;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use linux::Platform;
#[cfg(target_os = "linux")]
pub use linux::autostart;
