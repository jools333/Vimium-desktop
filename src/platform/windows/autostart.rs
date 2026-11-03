use anyhow::Result;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ, RegCloseKey, RegDeleteValueW,
    RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::core::w;

const RUN_KEY: &windows::core::PCWSTR = &w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const APP_NAME: &windows::core::PCWSTR = &w!("VimiumDesktop");

pub fn is_autostart_enabled() -> bool {
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, *RUN_KEY, 0, KEY_READ, &mut hkey).is_err() {
            return false;
        }

        let mut data_type = REG_SZ;
        let mut buf_size: u32 = 0;
        let res = RegQueryValueExW(
            hkey,
            *APP_NAME,
            None,
            Some(&mut data_type),
            None,
            Some(&mut buf_size),
        );

        let _ = RegCloseKey(hkey);
        res.is_ok() && buf_size > 0
    }
}

pub fn set_autostart(enable: bool) -> Result<()> {
    unsafe {
        let mut hkey = HKEY::default();
        RegOpenKeyExW(HKEY_CURRENT_USER, *RUN_KEY, 0, KEY_WRITE, &mut hkey).ok()?;

        if enable {
            let exe_path = std::env::current_exe()?;
            let exe_str = format!("\"{}\"", exe_path.to_string_lossy());
            let wide_val: Vec<u16> = exe_str.encode_utf16().chain(std::iter::once(0)).collect();
            let byte_slice = std::slice::from_raw_parts(
                wide_val.as_ptr() as *const u8,
                wide_val.len() * std::mem::size_of::<u16>(),
            );

            let res = RegSetValueExW(hkey, *APP_NAME, 0, REG_SZ, Some(byte_slice));
            let _ = RegCloseKey(hkey);
            res.ok()?;
        } else {
            let _ = RegDeleteValueW(hkey, *APP_NAME);
            let _ = RegCloseKey(hkey);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autostart_toggle() {
        let initial = is_autostart_enabled();
        // Set true
        set_autostart(true).expect("failed to enable autostart");
        assert!(is_autostart_enabled());
        // Set false
        set_autostart(false).expect("failed to disable autostart");
        assert!(!is_autostart_enabled());
        // Restore initial
        if initial {
            let _ = set_autostart(true);
        }
    }
}
