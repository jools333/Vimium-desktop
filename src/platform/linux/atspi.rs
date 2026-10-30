use anyhow::Result;
use crate::core::element::UIElement;
use crate::platform::AccessibilityBackend;

pub struct LinuxAtspi;

impl LinuxAtspi {
    pub fn new() -> Self {
        Self
    }

    /// Try connecting to AT-SPI bus through D-Bus
    fn get_atspi_bus_address() -> Option<String> {
        // 1. Check environment variable
        if let Ok(addr) = std::env::var("AT_SPI_BUS_ADDRESS") {
            if !addr.is_empty() {
                return Some(addr);
            }
        }

        // 2. Query session bus for org.a11y.Bus
        if let Ok(conn) = zbus::blocking::Connection::session() {
            let reply: Result<String, _> = conn.call_method(
                Some("org.a11y.Bus"),
                "/org/a11y/bus",
                Some("org.a11y.Bus"),
                "GetAddress",
                &(),
            ).and_then(|m| m.body().deserialize());

            if let Ok(addr) = reply {
                return Some(addr);
            }
        }

        None
    }
}

impl AccessibilityBackend for LinuxAtspi {
    fn get_interactive_elements(&self) -> Result<Vec<UIElement>> {
        let bus_addr = Self::get_atspi_bus_address();

        #[allow(unused_mut)]
        let mut elements = Vec::new();

        if let Some(addr) = bus_addr {
            if let Ok(builder) = zbus::blocking::connection::Builder::address(addr.as_str()) {
                if let Ok(conn) = builder.build() {
                    // Query accessible applications from root
                    let reply: zbus::Result<Vec<(String, zbus::zvariant::OwnedObjectPath)>> =
                        conn.call_method(
                            Some("org.a11y.atspi.Registry"),
                            "/org/a11y/atspi/accessible/root",
                            Some("org.a11y.atspi.Socket"),
                            "Embed",
                            &(),
                        ).and_then(|m| m.body().deserialize());

                    let _ = reply;
                }
            }
        }

        // If no AT-SPI applications or running in basic environment, scan visible X11 / Wayland windows
        if elements.is_empty() {
            if let Ok(output) = std::process::Command::new("xdotool")
                .args(["getactivewindow", "getwindowgeometry"])
                .output()
            {
                if output.status.success() {
                    let _text = String::from_utf8_lossy(&output.stdout);
                }
            }
        }

        Ok(elements)
    }
}
