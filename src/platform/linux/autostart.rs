use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

fn get_autostart_file_path() -> Result<PathBuf> {
    let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        anyhow::bail!("Could not determine user config directory ($HOME not set)");
    };

    Ok(config_dir.join("autostart").join("vimium-desktop.desktop"))
}

#[allow(dead_code)]
pub fn is_autostart_enabled() -> bool {
    if let Ok(path) = get_autostart_file_path() {
        path.exists()
    } else {
        false
    }
}

#[allow(dead_code)]
pub fn set_autostart(enable: bool) -> Result<()> {
    let desktop_path = get_autostart_file_path()?;

    if enable {
        if let Some(parent) = desktop_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create autostart directory: {:?}", parent))?;
        }

        let exe_path = std::env::current_exe().context("Failed to get current executable path")?;

        let content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Vimium Desktop\n\
             Comment=Vimium-style navigation for desktop applications\n\
             Exec=\"{}\"\n\
             Terminal=false\n\
             Hidden=false\n\
             NoDisplay=false\n\
             X-GNOME-Autostart-enabled=true\n",
            exe_path.to_string_lossy()
        );

        fs::write(&desktop_path, content)
            .with_context(|| format!("Failed to write desktop entry to {:?}", desktop_path))?;
    } else if desktop_path.exists() {
        let _ = fs::remove_file(&desktop_path);
    }

    Ok(())
}
