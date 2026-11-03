mod core;
mod platform;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::thread;
use std::time::Duration;

use crate::core::hint::assign_hints;
use crate::platform::{AccessibilityBackend, MouseBackend, OverlayBackend, Platform};

#[cfg(windows)]
use windows::Win32::System::Console::GetConsoleWindow;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};

#[derive(Parser, Debug)]
#[command(
    name = "vimium-desktop",
    author = "Antigravity Pair Programmer",
    version,
    about = "Vimium-style keyboard navigation for desktop applications (Windows & Linux)"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Delay in milliseconds before triggering (useful for CLI mode)
    #[arg(short, long, default_value_t = 0)]
    delay_ms: u64,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run as a background System Tray application (Default when double-clicked)
    Tray,
    /// Scan active window and taskbar, printing all clickable elements with coordinates
    Scan,
    /// Show hint badges over clickable elements, pick with keyboard and click
    Hints,
    /// Configure or check autostart at system launch
    Autostart {
        /// Enable autostart at system startup
        #[arg(long)]
        enable: bool,
        /// Disable autostart at system startup
        #[arg(long)]
        disable: bool,
        /// Check autostart status
        #[arg(long)]
        status: bool,
    },
}

fn run_hints(delay_ms: u64) -> Result<()> {
    if delay_ms > 0 {
        println!(
            "⏳ Waiting {} ms... Please switch to your target window!",
            delay_ms
        );
        thread::sleep(Duration::from_millis(delay_ms));
    }

    let mut platform = Platform::new();

    println!("🔍 Scanning active window & taskbar for interactive elements...");
    let elements = platform.get_interactive_elements()?;

    if elements.is_empty() {
        println!("⚠️ No interactive elements found.");
        return Ok(());
    }

    println!(
        "✨ Found {} interactive elements. Generating hints...",
        elements.len()
    );
    let hints = assign_hints(elements);

    println!("⌨️ Launching Vimium overlay (type letters to click, Esc to cancel)...");
    if let Some(selected) = platform.show_overlay(&hints)? {
        let (cx, cy) = selected.rect.center();
        println!(
            "🎯 Selected: '{}' [{}] at ({}, {}). Clicking...",
            selected.name, selected.role, cx, cy
        );
        platform.click_at(cx, cy)?;
        println!("✅ Click executed successfully!");
    } else {
        println!("❌ Action canceled by user.");
    }

    Ok(())
}

fn run_scan(delay_ms: u64) -> Result<()> {
    if delay_ms > 0 {
        println!("⏳ Waiting {} ms... Switch to target window!", delay_ms);
        thread::sleep(Duration::from_millis(delay_ms));
    }

    let platform = Platform::new();
    let elements = platform.get_interactive_elements()?;

    println!(
        "\n=== DETECTED INTERACTIVE ELEMENTS (Count: {}) ===",
        elements.len()
    );
    let hints = assign_hints(elements);
    for item in hints {
        println!(
            "  [{:^4}] {:<12} '{}' at ({}, {}) [{}x{}]",
            item.hint.to_uppercase(),
            item.element.role,
            item.element.name,
            item.element.rect.x,
            item.element.rect.y,
            item.element.rect.width,
            item.element.rect.height,
        );
    }
    println!("==================================================\n");
    Ok(())
}

fn run_tray() -> Result<()> {
    #[cfg(windows)]
    {
        // Hide the console window if spawned from Explorer
        unsafe {
            let console_hwnd = GetConsoleWindow();
            if !console_hwnd.0.is_null() {
                let _ = ShowWindow(console_hwnd, SW_HIDE);
            }
        }
        let app = platform::windows::TrayApplication::new();
        app.run()?;
    }

    #[cfg(not(windows))]
    {
        println!("Tray mode is currently optimized for Windows. Running in CLI mode...");
        run_hints(0)?;
    }

    Ok(())
}

fn handle_autostart(enable: bool, disable: bool, _status: bool) -> Result<()> {
    if enable {
        platform::autostart::set_autostart(true)?;
        let mut cfg = core::config::AppConfig::load();
        cfg.autostart = true;
        cfg.save();
        println!("✅ Autostart enabled: Vimium Desktop will run automatically on system boot.");
    } else if disable {
        platform::autostart::set_autostart(false)?;
        let mut cfg = core::config::AppConfig::load();
        cfg.autostart = false;
        cfg.save();
        println!("❌ Autostart disabled.");
    } else {
        let is_on = platform::autostart::is_autostart_enabled();
        println!(
            "Vimium Desktop autostart status: {}",
            if is_on {
                "ENABLED (Active)"
            } else {
                "DISABLED"
            }
        );
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let delay = cli.delay_ms;

    match cli.command.unwrap_or(Commands::Tray) {
        Commands::Tray => run_tray()?,
        Commands::Scan => run_scan(delay)?,
        Commands::Hints => run_hints(delay)?,
        Commands::Autostart {
            enable,
            disable,
            status,
        } => handle_autostart(enable, disable, status)?,
    }

    Ok(())
}
