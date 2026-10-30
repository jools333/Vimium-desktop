pub mod autostart;
pub mod mouse;
pub mod overlay;
pub mod tray;
pub mod uia;

use anyhow::Result;
use crate::core::element::{HintItem, UIElement};
use crate::platform::{AccessibilityBackend, MouseBackend, OverlayBackend};

pub use tray::TrayApplication;

pub struct Platform;

impl Platform {
    pub fn new() -> Self {
        Self
    }
}

impl AccessibilityBackend for Platform {
    fn get_interactive_elements(&self) -> Result<Vec<UIElement>> {
        uia::WindowsUia::new()?.get_interactive_elements()
    }
}

impl MouseBackend for Platform {
    fn click_at(&self, x: i32, y: i32) -> Result<()> {
        mouse::WindowsMouse::new().click_at(x, y)
    }
}

impl OverlayBackend for Platform {
    fn show_overlay(&mut self, hints: &[HintItem]) -> Result<Option<UIElement>> {
        overlay::WindowsOverlay::new().show_overlay(hints)
    }
}
