use std::fs;
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyPreset {
    AltF,
    AltSpace,
    AltSemicolon,
    CtrlShiftF,
}

impl HotkeyPreset {
    pub fn name(&self) -> &'static str {
        match self {
            HotkeyPreset::AltF => "Alt + F",
            HotkeyPreset::AltSpace => "Alt + Пробел",
            HotkeyPreset::AltSemicolon => "Alt + ;",
            HotkeyPreset::CtrlShiftF => "Ctrl + Shift + F",
        }
    }

    pub fn to_win32_modifiers_and_vk(self) -> (u32, u32) {
        // Win32 modifiers: MOD_ALT = 0x0001, MOD_CONTROL = 0x0002, MOD_SHIFT = 0x0004, MOD_NOREPEAT = 0x4000
        match self {
            HotkeyPreset::AltF => (0x0001 | 0x4000, 0x46), // Alt + 'F'
            HotkeyPreset::AltSpace => (0x0001 | 0x4000, 0x20), // Alt + VK_SPACE
            HotkeyPreset::AltSemicolon => (0x0001 | 0x4000, 0xBA), // Alt + VK_OEM_1 (;)
            HotkeyPreset::CtrlShiftF => (0x0002 | 0x0004 | 0x4000, 0x46), // Ctrl + Shift + 'F'
        }
    }

    pub fn from_id(id: usize) -> Self {
        match id {
            1 => HotkeyPreset::AltSpace,
            2 => HotkeyPreset::AltSemicolon,
            3 => HotkeyPreset::CtrlShiftF,
            _ => HotkeyPreset::AltF,
        }
    }

    pub fn to_id(self) -> usize {
        match self {
            HotkeyPreset::AltF => 0,
            HotkeyPreset::AltSpace => 1,
            HotkeyPreset::AltSemicolon => 2,
            HotkeyPreset::CtrlShiftF => 3,
        }
    }
}

#[allow(dead_code)]
pub struct AppConfig {
    pub preset: HotkeyPreset,
    pub skip_browsers: bool,
    pub autostart: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            preset: HotkeyPreset::AltF,
            skip_browsers: true,
            autostart: false,
        }
    }
}

impl AppConfig {
    fn config_path() -> PathBuf {
        let mut path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        path.pop();
        path.push("vimium_config.txt");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(content) = fs::read_to_string(&path) {
            let lines: Vec<&str> = content.lines().collect();
            let mut cfg = Self::default();
            if let Some(Ok(id)) = lines.first().map(|s| s.trim().parse::<usize>()) {
                cfg.preset = HotkeyPreset::from_id(id);
            }
            if let Some(Ok(skip)) = lines.get(1).map(|s| s.trim().parse::<bool>()) {
                cfg.skip_browsers = skip;
            }
            if let Some(Ok(auto)) = lines.get(2).map(|s| s.trim().parse::<bool>()) {
                cfg.autostart = auto;
            }
            return cfg;
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        let content = format!(
            "{}\n{}\n{}",
            self.preset.to_id(),
            self.skip_browsers,
            self.autostart
        );
        let _ = fs::write(path, content);
    }
}
