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

#[allow(dead_code)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserMode {
    TabsAndToolbar, // 0: Tabs, Address bar, Navigation toolbar (Web page excluded) - DEFAULT
    SkipAll,        // 1: Skip browser completely
    Full,           // 2: Scan everything including webpage DOM
}

#[allow(dead_code)]
impl BrowserMode {
    pub fn name(&self) -> &'static str {
        match self {
            BrowserMode::TabsAndToolbar => "Вкладки и панель",
            BrowserMode::SkipAll => "Пропуск",
            BrowserMode::Full => "Полный",
        }
    }

    pub fn from_id(id: usize) -> Self {
        match id {
            1 => BrowserMode::SkipAll,
            2 => BrowserMode::Full,
            _ => BrowserMode::TabsAndToolbar,
        }
    }

    pub fn to_id(self) -> usize {
        match self {
            BrowserMode::TabsAndToolbar => 0,
            BrowserMode::SkipAll => 1,
            BrowserMode::Full => 2,
        }
    }
}

#[allow(dead_code)]
pub struct AppConfig {
    pub preset: HotkeyPreset,
    pub browser_mode: BrowserMode,
    pub autostart: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            preset: HotkeyPreset::AltF,
            browser_mode: BrowserMode::TabsAndToolbar,
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
            if let Some(line) = lines.get(1) {
                let trimmed = line.trim();
                if let Ok(id) = trimmed.parse::<usize>() {
                    cfg.browser_mode = BrowserMode::from_id(id);
                } else if let Ok(skip) = trimmed.parse::<bool>() {
                    // Backward compatibility with older bool format:
                    // If true was "skip browsers", map to TabsAndToolbar (smart skip of DOM)
                    cfg.browser_mode = if skip {
                        BrowserMode::TabsAndToolbar
                    } else {
                        BrowserMode::Full
                    };
                }
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
            self.browser_mode.to_id(),
            self.autostart
        );
        let _ = fs::write(path, content);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_mode_ids() {
        assert_eq!(BrowserMode::TabsAndToolbar.to_id(), 0);
        assert_eq!(BrowserMode::SkipAll.to_id(), 1);
        assert_eq!(BrowserMode::Full.to_id(), 2);

        assert_eq!(BrowserMode::from_id(0), BrowserMode::TabsAndToolbar);
        assert_eq!(BrowserMode::from_id(1), BrowserMode::SkipAll);
        assert_eq!(BrowserMode::from_id(2), BrowserMode::Full);
        assert_eq!(BrowserMode::from_id(99), BrowserMode::TabsAndToolbar);
    }
}
