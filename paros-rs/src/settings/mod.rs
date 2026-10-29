#![allow(dead_code)]

pub mod ini;
pub mod store;

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

pub use store::{sync_autostart, SettingsWatcher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgySettings {
    pub desktop: bool,
    pub desktop_minutes: f32,
}

impl Default for AgySettings {
    fn default() -> Self {
        Self {
            desktop: true,
            desktop_minutes: 10.0,
        }
    }
}

impl AgySettings {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub language: String,
    pub pet_size: f32,
    pub walk_speed: f32,
    pub show_name: bool,
    pub accessories: bool,
    pub headlamp: bool,
    pub groove: bool,
    pub greetings: bool,
    pub tower: bool,
    pub sound_enabled: bool,
    pub sound_volume: i32,
    pub night_start_hour: u32,
    pub night_end_hour: u32,
    pub idle_minutes: f32,
    pub bubble_enabled: bool,
    pub bubble_seconds: f32,
    pub claude_show_activity: bool,
    pub claude_knock: bool,
    pub claude_nag_minutes: f32,
    pub claude_sage: bool,
    pub claude_sage_live: bool,
    pub claude_context_window_k: usize,
    pub agy: AgySettings,
    pub subagents_show: bool,
    pub subagents_max: usize,
    pub subagents_size: f32,
    pub subagents_show_activity: bool,
    pub subagents_games: bool,
    pub subagents_pyramid: bool,
    pub git_enabled: bool,
    pub desktop_perch: bool,
    pub desktop_cuddle: bool,
    pub desktop_leave_fullscreen: bool,
    pub desktop_lock_screen_minutes: f32,
    pub focus_minutes: f32,
    pub focus_break_minutes: f32,
    pub system_alerts: bool,
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let autostart = {
            let p1 = Self::autostart_desktop_path();
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
            let p2 = PathBuf::from(home).join(".config/autostart/paros.desktop");
            p1.exists() || p2.exists()
        };
        Self {
            language: "auto".to_string(),
            pet_size: 1.0,
            walk_speed: 70.0,
            show_name: true,
            accessories: true,
            headlamp: true,
            groove: true,
            greetings: true,
            tower: true,
            sound_enabled: true,
            sound_volume: 50,
            night_start_hour: 23,
            night_end_hour: 7,
            idle_minutes: 5.0,
            bubble_enabled: true,
            bubble_seconds: 4.0,
            claude_show_activity: true,
            claude_knock: true,
            claude_nag_minutes: 2.0,
            claude_sage: true,
            claude_sage_live: true,
            claude_context_window_k: 1000,
            agy: AgySettings::default(),
            subagents_show: true,
            subagents_max: 16,
            subagents_size: 0.4,
            subagents_show_activity: true,
            subagents_games: true,
            subagents_pyramid: true,
            git_enabled: true,
            desktop_perch: true,
            desktop_cuddle: true,
            desktop_leave_fullscreen: true,
            desktop_lock_screen_minutes: 10.0,
            focus_minutes: 25.0,
            focus_break_minutes: 5.0,
            system_alerts: true,
            autostart,
        }
    }
}

impl Settings {
    pub fn primary_path() -> PathBuf {
        store::primary_path()
    }

    pub fn godot_legacy_path() -> PathBuf {
        store::godot_legacy_path()
    }

    pub fn fallback_path() -> PathBuf {
        store::fallback_path()
    }

    pub fn find_settings_file() -> PathBuf {
        store::find_settings_file()
    }

    pub fn resolve_path() -> PathBuf {
        store::resolve_path()
    }

    pub fn autostart_desktop_path() -> PathBuf {
        store::autostart_desktop_path()
    }

    pub fn load() -> Self {
        let path = Self::resolve_path();
        if path.exists() {
            Self::load_from_path(&path)
        } else {
            let settings = Self::default();
            settings.save_to_path(&path);
            settings
        }
    }

    pub fn load_from_path(path: &Path) -> Self {
        let Ok(content) = fs::read_to_string(path) else {
            return Self::default();
        };
        Self::parse_ini(&content)
    }

    pub fn parse_ini(content: &str) -> Self {
        ini::parse_ini(content)
    }

    pub fn to_ini(&self) -> String {
        ini::to_ini(self)
    }

    pub fn save(&self) {
        let path = Self::resolve_path();
        self.save_to_path(&path);
        sync_autostart(self.autostart);
    }

    pub fn save_to_path(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, self.to_ini());
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn get_value(&self, section: &str, key: &str) -> Option<String> {
        match (section.to_ascii_lowercase().as_str(), key.to_ascii_lowercase().as_str()) {
            ("agy", "desktop") => Some(self.agy.desktop.to_string()),
            ("agy", "desktop_minutes") => Some(self.agy.desktop_minutes.to_string()),
            ("interface", "language") => Some(self.language.clone()),
            ("pet", "size") => Some(self.pet_size.to_string()),
            ("pet", "walk_speed") => Some(self.walk_speed.to_string()),
            ("pet", "show_name") => Some(self.show_name.to_string()),
            ("pet", "accessories") => Some(self.accessories.to_string()),
            ("pet", "headlamp") => Some(self.headlamp.to_string()),
            ("pet", "groove") => Some(self.groove.to_string()),
            ("pet", "greetings") => Some(self.greetings.to_string()),
            ("pet", "tower") => Some(self.tower.to_string()),
            ("sound", "enabled") => Some(self.sound_enabled.to_string()),
            ("sound", "volume") => Some(self.sound_volume.to_string()),
            ("sleep", "night_start_hour") => Some(self.night_start_hour.to_string()),
            ("sleep", "night_end_hour") => Some(self.night_end_hour.to_string()),
            ("sleep", "idle_minutes") => Some(self.idle_minutes.to_string()),
            ("bubble", "enabled") => Some(self.bubble_enabled.to_string()),
            ("bubble", "seconds") => Some(self.bubble_seconds.to_string()),
            ("claude", "show_activity") => Some(self.claude_show_activity.to_string()),
            ("claude", "knock") => Some(self.claude_knock.to_string()),
            ("claude", "nag_minutes") => Some(self.claude_nag_minutes.to_string()),
            ("claude", "sage") => Some(self.claude_sage.to_string()),
            ("claude", "sage_live") => Some(self.claude_sage_live.to_string()),
            ("claude", "context_window_k") => Some(self.claude_context_window_k.to_string()),
            ("subagents", "show") => Some(self.subagents_show.to_string()),
            ("subagents", "max") => Some(self.subagents_max.to_string()),
            ("subagents", "size") => Some(self.subagents_size.to_string()),
            ("subagents", "show_activity") => Some(self.subagents_show_activity.to_string()),
            ("subagents", "games") => Some(self.subagents_games.to_string()),
            ("subagents", "pyramid") => Some(self.subagents_pyramid.to_string()),
            ("git", "enabled") => Some(self.git_enabled.to_string()),
            ("desktop", "perch") => Some(self.desktop_perch.to_string()),
            ("desktop", "cuddle") => Some(self.desktop_cuddle.to_string()),
            ("desktop", "leave_fullscreen") => Some(self.desktop_leave_fullscreen.to_string()),
            ("desktop", "lock_screen_minutes") => Some(self.desktop_lock_screen_minutes.to_string()),
            ("focus", "minutes") => Some(self.focus_minutes.to_string()),
            ("focus", "break_minutes") => Some(self.focus_break_minutes.to_string()),
            ("system", "alerts") => Some(self.system_alerts.to_string()),
            ("system", "autostart") | ("autostart", "enabled") => Some(self.autostart.to_string()),
            _ => None,
        }
    }

    pub fn set_value(&mut self, section: &str, key: &str, value: &str) -> bool {
        match (section.to_ascii_lowercase().as_str(), key.to_ascii_lowercase().as_str()) {
            ("agy", "desktop") => {
                if let Some(v) = ini::parse_bool(value) {
                    self.agy.desktop = v;
                    return true;
                }
            }
            ("agy", "desktop_minutes") => {
                if let Some(v) = ini::parse_f32(value) {
                    self.agy.desktop_minutes = v.clamp(0.5, 120.0);
                    return true;
                }
            }
            ("interface", "language") => {
                self.language = value.trim().to_string();
                return true;
            }
            ("pet", "size") => {
                if let Some(v) = ini::parse_f32(value) {
                    self.pet_size = v.clamp(0.5, 3.0);
                    return true;
                }
            }
            ("pet", "walk_speed") => {
                if let Some(v) = ini::parse_f32(value) {
                    self.walk_speed = v.clamp(10.0, 300.0);
                    return true;
                }
            }
            ("pet", "show_name") => {
                if let Some(v) = ini::parse_bool(value) {
                    self.show_name = v;
                    return true;
                }
            }
            ("sound", "enabled") => {
                if let Some(v) = ini::parse_bool(value) {
                    self.sound_enabled = v;
                    return true;
                }
            }
            ("sound", "volume") => {
                if let Some(v) = ini::parse_i32(value) {
                    self.sound_volume = v.clamp(0, 100);
                    return true;
                }
            }
            ("git", "enabled") => {
                if let Some(v) = ini::parse_bool(value) {
                    self.git_enabled = v;
                    return true;
                }
            }
            ("system", "alerts") => {
                if let Some(v) = ini::parse_bool(value) {
                    self.system_alerts = v;
                    return true;
                }
            }
            ("system", "autostart") | ("autostart", "enabled") => {
                if let Some(v) = ini::parse_bool(value) {
                    self.autostart = v;
                    return true;
                }
            }
            _ => {}
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agy_settings_defaults_and_json() {
        let agy = AgySettings::default();
        assert!(agy.desktop);
        assert!((agy.desktop_minutes - 10.0).abs() < f32::EPSILON);

        let json = agy.to_json().expect("serialize agy json");
        assert!(json.contains("\"desktop\": true"));
        let parsed = AgySettings::from_json(&json).expect("deserialize agy json");
        assert_eq!(parsed, agy);
    }

    #[test]
    fn test_settings_ini_roundtrip_agy() {
        let mut settings = Settings::default();
        settings.agy.desktop = false;
        settings.agy.desktop_minutes = 15.5;

        let ini = settings.to_ini();
        assert!(ini.contains("[agy]"));
        assert!(ini.contains("desktop=false"));
        assert!(ini.contains("desktop_minutes=15.5"));

        let loaded = Settings::parse_ini(&ini);
        assert!(!loaded.agy.desktop);
        assert!((loaded.agy.desktop_minutes - 15.5).abs() < 0.01);
    }

    #[test]
    fn test_settings_get_set_value() {
        let mut settings = Settings::default();
        assert_eq!(settings.get_value("agy", "desktop"), Some("true".to_string()));
        assert_eq!(settings.get_value("agy", "desktop_minutes"), Some("10".to_string()));

        assert!(settings.set_value("agy", "desktop", "false"));
        assert!(!settings.agy.desktop);
        assert_eq!(settings.get_value("agy", "desktop"), Some("false".to_string()));

        assert!(settings.set_value("agy", "desktop_minutes", "25.0"));
        assert!((settings.agy.desktop_minutes - 25.0).abs() < f32::EPSILON);
        assert_eq!(settings.get_value("agy", "desktop_minutes"), Some("25".to_string()));
    }
}

