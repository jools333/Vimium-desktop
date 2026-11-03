pub mod atspi;
pub mod autostart;
pub mod mouse;
pub mod overlay;

use crate::core::element::{HintItem, UIElement};
use crate::platform::{AccessibilityBackend, MouseBackend, OverlayBackend};
use anyhow::Result;

pub struct Platform;

impl Platform {
    pub fn new() -> Self {
        Self
    }
}

impl AccessibilityBackend for Platform {
    fn get_interactive_elements(&self) -> Result<Vec<UIElement>> {
        atspi::LinuxAtspi::new().get_interactive_elements()
    }
}

impl MouseBackend for Platform {
    fn click_at(&self, x: i32, y: i32) -> Result<()> {
        mouse::LinuxMouse::new().click_at(x, y)
    }
}

impl OverlayBackend for Platform {
    fn show_overlay(&mut self, hints: &[HintItem]) -> Result<Option<UIElement>> {
        overlay::LinuxOverlay::new().show_overlay(hints)
    }
}
