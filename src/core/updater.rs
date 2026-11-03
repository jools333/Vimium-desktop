use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

const REPO_OWNER: &str = "jools333";
const REPO_NAME: &str = "Vimium-desktop";

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub asset_name: String,
    pub asset_size: u64,
    pub notes: String,
}

/// Parses semantic version string like "v0.1.0" or "0.1.0" into (major, minor, patch)
pub fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let clean = v.trim().trim_start_matches(['v', 'V']);
    let parts: Vec<&str> = clean.split('.').collect();
    if parts.len() < 2 {
        return None;
    }

    let major = parts.first()?.parse::<u64>().ok()?;
    let minor = parts.get(1)?.parse::<u64>().ok()?;
    let patch = parts
        .get(2)
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(0);

    Some((major, minor, patch))
}

/// Returns true if `latest` is strictly newer than `current`
pub fn is_newer_version(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some((l_maj, l_min, l_patch)), Some((c_maj, c_min, c_patch))) => {
            (l_maj, l_min, l_patch) > (c_maj, c_min, c_patch)
        }
        _ => false,
    }
}

/// Expected asset name for current OS platform
pub fn get_target_asset_name() -> &'static str {
    if cfg!(windows) {
        "vimium-desktop-windows-x86_64.exe"
    } else {
        "vimium-desktop-linux-x86_64"
    }
}

/// Checks GitHub Releases API for a newer version
pub fn check_update() -> Result<Option<UpdateInfo>> {
    let current_version = env!("CARGO_PKG_VERSION");
    let api_url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        REPO_OWNER, REPO_NAME
    );

    let output = Command::new("curl")
        .args([
            "-s",
            "-L",
            "-f",
            "-H",
            "User-Agent: vimium-desktop",
            "-H",
            "Accept: application/vnd.github.v3+json",
            &api_url,
        ])
        .output()
        .context("Failed to execute curl to check for updates")?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("GitHub API request failed: {}", err_msg);
    }

    let release: GithubRelease = serde_json::from_slice(&output.stdout)
        .context("Failed to parse GitHub release JSON response")?;

    if !is_newer_version(&release.tag_name, current_version) {
        return Ok(None);
    }

    let target_name = get_target_asset_name();
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == target_name)
        .with_context(|| {
            format!(
                "Release {} found, but does not contain asset for this platform: {}",
                release.tag_name, target_name
            )
        })?;

    Ok(Some(UpdateInfo {
        current_version: current_version.to_string(),
        latest_version: release.tag_name,
        download_url: asset.browser_download_url.clone(),
        asset_name: asset.name.clone(),
        asset_size: asset.size,
        notes: release.body.unwrap_or_default(),
    }))
}

/// Downloads new binary to target path using curl
fn download_binary(url: &str, dest_path: &Path) -> Result<()> {
    let status = Command::new("curl")
        .args(["-L", "-f", "-o"])
        .arg(dest_path)
        .arg(url)
        .status()
        .context("Failed to execute curl during download")?;

    if !status.success() {
        anyhow::bail!("Download failed with status: {}", status);
    }

    let metadata =
        std::fs::metadata(dest_path).context("Failed to read downloaded file metadata")?;

    if metadata.len() == 0 {
        let _ = std::fs::remove_file(dest_path);
        anyhow::bail!("Downloaded update file is empty");
    }

    Ok(())
}

/// Downloads and replaces the current executable
pub fn apply_update(info: &UpdateInfo) -> Result<PathBuf> {
    let current_exe =
        std::env::current_exe().context("Failed to determine path of running executable")?;

    let parent_dir = current_exe
        .parent()
        .context("Failed to get parent directory of current executable")?;

    let temp_download = parent_dir.join("vimium-desktop-update.tmp");
    #[cfg(windows)]
    let old_backup = parent_dir.join("vimium-desktop.exe.old");

    // 1. Download new binary
    download_binary(&info.download_url, &temp_download)?;

    // 2. Platform-specific binary replacement
    #[cfg(windows)]
    {
        // On Windows, running files can be renamed but not overwritten
        if old_backup.exists() {
            let _ = std::fs::remove_file(&old_backup);
        }

        std::fs::rename(&current_exe, &old_backup)
            .context("Failed to rename current executable to backup")?;

        if let Err(e) = std::fs::rename(&temp_download, &current_exe) {
            // Rollback if renaming new file failed
            let _ = std::fs::rename(&old_backup, &current_exe);
            return Err(e).context("Failed to place new executable in position; rolled back");
        }

        // Clean up backup if possible
        let _ = std::fs::remove_file(&old_backup);
    }

    #[cfg(not(windows))]
    {
        // On Linux/Unix, set executable bits and atomically replace
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&temp_download)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&temp_download, perms)?;

        std::fs::rename(&temp_download, &current_exe)
            .context("Failed to replace current executable with update")?;
    }

    Ok(current_exe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!(parse_version("0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_version("v0.1.9"), Some((0, 1, 9)));
        assert_eq!(parse_version("V1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("invalid"), None);
    }

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("v0.1.1", "0.1.0"));
        assert!(is_newer_version("1.0.0", "0.9.9"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(!is_newer_version("v0.0.9", "0.1.0"));
    }
}
