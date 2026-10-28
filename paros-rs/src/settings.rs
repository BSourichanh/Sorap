#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone, PartialEq)]
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
    fn data_dir() -> PathBuf {
        std::env::var("XDG_DATA_HOME")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
                PathBuf::from(home).join(".local/share")
            })
    }

    fn config_dir() -> PathBuf {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
                PathBuf::from(home).join(".config")
            })
    }

    pub fn primary_path() -> PathBuf {
        Self::data_dir().join("paros/settings.cfg")
    }

    pub fn godot_legacy_path() -> PathBuf {
        Self::data_dir().join("godot/app_userdata/Paros/settings.cfg")
    }

    pub fn fallback_path() -> PathBuf {
        Self::config_dir().join("paros/settings.cfg")
    }

    pub fn find_settings_file() -> PathBuf {
        let primary = Self::primary_path();
        if primary.exists() {
            return primary;
        }
        let godot_legacy = Self::godot_legacy_path();
        if godot_legacy.exists() {
            return godot_legacy;
        }
        let fallback = Self::fallback_path();
        if fallback.exists() {
            return fallback;
        }
        primary
    }

    pub fn resolve_path() -> PathBuf {
        Self::find_settings_file()
    }

    pub fn autostart_desktop_path() -> PathBuf {
        Self::config_dir().join("autostart/paros.desktop")
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
        let mut settings = Self::default();
        let mut section = String::new();

        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                section = line[1..line.len() - 1].trim().to_ascii_lowercase();
                continue;
            }
            let Some((raw_key, raw_val)) = line.split_once('=') else {
                continue;
            };
            let key = raw_key.trim().to_ascii_lowercase();
            let val = unquote(raw_val.trim());

            match (section.as_str(), key.as_str()) {
                ("interface", "language") | ("", "language") => {
                    if !val.is_empty() {
                        settings.language = val.to_string();
                    }
                }
                ("pet", "size") | ("", "pet_size") => {
                    if let Some(v) = parse_f32(val) {
                        settings.pet_size = v.clamp(0.5, 3.0);
                    }
                }
                ("pet", "walk_speed") | ("", "walk_speed") => {
                    if let Some(v) = parse_f32(val) {
                        settings.walk_speed = v.clamp(10.0, 300.0);
                    }
                }
                ("pet", "show_name") | ("", "show_name") => {
                    if let Some(v) = parse_bool(val) {
                        settings.show_name = v;
                    }
                }
                ("pet", "accessories") | ("", "accessories") => {
                    if let Some(v) = parse_bool(val) {
                        settings.accessories = v;
                    }
                }
                ("pet", "headlamp") | ("", "headlamp") => {
                    if let Some(v) = parse_bool(val) {
                        settings.headlamp = v;
                    }
                }
                ("pet", "groove") | ("", "groove") => {
                    if let Some(v) = parse_bool(val) {
                        settings.groove = v;
                    }
                }
                ("pet", "greetings") | ("", "greetings") => {
                    if let Some(v) = parse_bool(val) {
                        settings.greetings = v;
                    }
                }
                ("pet", "tower") | ("", "tower") => {
                    if let Some(v) = parse_bool(val) {
                        settings.tower = v;
                    }
                }
                ("sound", "enabled") | ("", "sound_enabled") => {
                    if let Some(v) = parse_bool(val) {
                        settings.sound_enabled = v;
                    }
                }
                ("sound", "volume") | ("", "sound_volume") => {
                    if let Some(v) = parse_i32(val) {
                        settings.sound_volume = v.clamp(0, 100);
                    }
                }
                ("sleep", "night_start_hour") | ("", "night_start_hour") => {
                    if let Some(v) = parse_u32(val) {
                        settings.night_start_hour = v.min(23);
                    }
                }
                ("sleep", "night_end_hour") | ("", "night_end_hour") => {
                    if let Some(v) = parse_u32(val) {
                        settings.night_end_hour = v.min(23);
                    }
                }
                ("sleep", "idle_minutes") | ("", "idle_minutes") => {
                    if let Some(v) = parse_f32(val) {
                        settings.idle_minutes = v.clamp(1.0, 120.0);
                    }
                }
                ("bubble", "enabled") | ("", "bubble_enabled") => {
                    if let Some(v) = parse_bool(val) {
                        settings.bubble_enabled = v;
                    }
                }
                ("bubble", "seconds") | ("", "bubble_seconds") => {
                    if let Some(v) = parse_f32(val) {
                        settings.bubble_seconds = v.clamp(1.0, 30.0);
                    }
                }
                ("claude", "show_activity") | ("", "claude_show_activity") => {
                    if let Some(v) = parse_bool(val) {
                        settings.claude_show_activity = v;
                    }
                }
                ("claude", "knock") | ("", "claude_knock") => {
                    if let Some(v) = parse_bool(val) {
                        settings.claude_knock = v;
                    }
                }
                ("claude", "nag_minutes") | ("", "claude_nag_minutes") => {
                    if let Some(v) = parse_f32(val) {
                        settings.claude_nag_minutes = v.clamp(0.5, 60.0);
                    }
                }
                ("claude", "sage") | ("", "claude_sage") => {
                    if let Some(v) = parse_bool(val) {
                        settings.claude_sage = v;
                    }
                }
                ("claude", "sage_live") | ("", "claude_sage_live") => {
                    if let Some(v) = parse_bool(val) {
                        settings.claude_sage_live = v;
                    }
                }
                ("claude", "context_window_k") | ("", "claude_context_window_k") => {
                    if let Some(v) = parse_usize(val) {
                        settings.claude_context_window_k = v.clamp(100, 2000);
                    }
                }
                ("subagents", "show") | ("", "subagents_show") => {
                    if let Some(v) = parse_bool(val) {
                        settings.subagents_show = v;
                    }
                }
                ("subagents", "max") | ("", "subagents_max") => {
                    if let Some(v) = parse_usize(val) {
                        settings.subagents_max = v.clamp(1, 16);
                    }
                }
                ("subagents", "size") | ("", "subagents_size") => {
                    if let Some(v) = parse_f32(val) {
                        settings.subagents_size = v.clamp(0.2, 0.8);
                    }
                }
                ("subagents", "show_activity") | ("", "subagents_show_activity") => {
                    if let Some(v) = parse_bool(val) {
                        settings.subagents_show_activity = v;
                    }
                }
                ("subagents", "games") | ("", "subagents_games") => {
                    if let Some(v) = parse_bool(val) {
                        settings.subagents_games = v;
                    }
                }
                ("subagents", "pyramid") | ("", "subagents_pyramid") => {
                    if let Some(v) = parse_bool(val) {
                        settings.subagents_pyramid = v;
                    }
                }
                ("git", "enabled") | ("", "git_enabled") => {
                    if let Some(v) = parse_bool(val) {
                        settings.git_enabled = v;
                    }
                }
                ("desktop", "perch") | ("", "desktop_perch") => {
                    if let Some(v) = parse_bool(val) {
                        settings.desktop_perch = v;
                    }
                }
                ("desktop", "cuddle") | ("", "desktop_cuddle") => {
                    if let Some(v) = parse_bool(val) {
                        settings.desktop_cuddle = v;
                    }
                }
                ("desktop", "leave_fullscreen") | ("", "desktop_leave_fullscreen") => {
                    if let Some(v) = parse_bool(val) {
                        settings.desktop_leave_fullscreen = v;
                    }
                }
                ("desktop", "lock_screen_minutes") | ("", "desktop_lock_screen_minutes") => {
                    if let Some(v) = parse_f32(val) {
                        settings.desktop_lock_screen_minutes = v.clamp(0.0, 240.0);
                    }
                }
                ("focus", "minutes") | ("", "focus_minutes") => {
                    if let Some(v) = parse_f32(val) {
                        settings.focus_minutes = v.clamp(1.0, 120.0);
                    }
                }
                ("focus", "break_minutes") | ("", "focus_break_minutes") => {
                    if let Some(v) = parse_f32(val) {
                        settings.focus_break_minutes = v.clamp(1.0, 60.0);
                    }
                }
                ("system", "alerts") | ("", "system_alerts") => {
                    if let Some(v) = parse_bool(val) {
                        settings.system_alerts = v;
                    }
                }
                ("system", "autostart")
                | ("autostart", "enabled")
                | ("interface", "autostart")
                | ("", "autostart") => {
                    if let Some(v) = parse_bool(val) {
                        settings.autostart = v;
                    }
                }
                _ => {}
            }
        }

        settings
    }

    pub fn to_ini(&self) -> String {
        format!(
            "[interface]\n\n\
             language=\"{}\"\n\n\
             [pet]\n\n\
             size={:.2}\n\
             walk_speed={:.1}\n\
             show_name={}\n\
             accessories={}\n\
             greetings={}\n\
             tower={}\n\
             headlamp={}\n\
             groove={}\n\n\
             [sound]\n\n\
             enabled={}\n\
             volume={}\n\n\
             [sleep]\n\n\
             night_start_hour={}\n\
             night_end_hour={}\n\
             idle_minutes={:.1}\n\n\
             [bubble]\n\n\
             enabled={}\n\
             seconds={:.1}\n\n\
             [claude]\n\n\
             show_activity={}\n\
             nag_minutes={:.1}\n\
             context_window_k={}\n\
             knock={}\n\
             sage={}\n\
             sage_live={}\n\n\
             [subagents]\n\n\
             show={}\n\
             max={}\n\
             size={:.2}\n\
             show_activity={}\n\
             games={}\n\
             pyramid={}\n\n\
             [git]\n\n\
             enabled={}\n\n\
             [desktop]\n\n\
             perch={}\n\
             cuddle={}\n\
             leave_fullscreen={}\n\
             lock_screen_minutes={:.1}\n\n\
             [focus]\n\n\
             minutes={:.1}\n\
             break_minutes={:.1}\n\n\
             [system]\n\n\
             alerts={}\n\
             autostart={}\n",
            self.language,
            self.pet_size,
            self.walk_speed,
            self.show_name,
            self.accessories,
            self.greetings,
            self.tower,
            self.headlamp,
            self.groove,
            self.sound_enabled,
            self.sound_volume,
            self.night_start_hour,
            self.night_end_hour,
            self.idle_minutes,
            self.bubble_enabled,
            self.bubble_seconds,
            self.claude_show_activity,
            self.claude_nag_minutes,
            self.claude_context_window_k,
            self.claude_knock,
            self.claude_sage,
            self.claude_sage_live,
            self.subagents_show,
            self.subagents_max,
            self.subagents_size,
            self.subagents_show_activity,
            self.subagents_games,
            self.subagents_pyramid,
            self.git_enabled,
            self.desktop_perch,
            self.desktop_cuddle,
            self.desktop_leave_fullscreen,
            self.desktop_lock_screen_minutes,
            self.focus_minutes,
            self.focus_break_minutes,
            self.system_alerts,
            self.autostart,
        )
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
}

pub fn sync_autostart(enabled: bool) {
    let desktop_path = Settings::autostart_desktop_path();
    if !enabled {
        if desktop_path.exists() {
            let _ = fs::remove_file(desktop_path);
        }
        return;
    }

    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
        .unwrap_or_else(|| "paros-rs".to_string());

    let content = format!(
        "[Desktop Entry]\nType=Application\nName=Paros\nExec=env XMODIFIERS=@im=none \"{}\"\n",
        exe
    );

    if let Ok(existing) = fs::read_to_string(&desktop_path)
        && existing == content
    {
        return;
    }

    if let Some(parent) = desktop_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(desktop_path, content);
}

pub struct SettingsWatcher {
    settings: Settings,
    path: PathBuf,
    last_mtime: Option<SystemTime>,
    last_check: Instant,
}

impl Default for SettingsWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsWatcher {
    pub fn new() -> Self {
        let path = Settings::resolve_path();
        let settings = if path.exists() {
            Settings::load_from_path(&path)
        } else {
            let default_settings = Settings::default();
            default_settings.save_to_path(&path);
            default_settings
        };
        sync_autostart(settings.autostart);
        let last_mtime = file_mtime(&path);

        Self {
            settings,
            path,
            last_mtime,
            last_check: Instant::now(),
        }
    }

    pub fn poll(&mut self) -> bool {
        if self.last_check.elapsed() < Duration::from_secs(2) {
            return false;
        }
        self.last_check = Instant::now();

        let current_path = Settings::resolve_path();
        let current_mtime = file_mtime(&current_path);

        if current_path != self.path || current_mtime != self.last_mtime {
            self.path = current_path;
            self.last_mtime = current_mtime;
            if self.path.exists() {
                let new_settings = Settings::load_from_path(&self.path);
                let changed = new_settings != self.settings;
                if self.settings.autostart != new_settings.autostart {
                    sync_autostart(new_settings.autostart);
                }
                self.settings = new_settings;
                return changed;
            }
        }
        false
    }

    pub fn get(&self) -> &Settings {
        &self.settings
    }

    pub fn get_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    pub fn save(&mut self) {
        self.settings.save_to_path(&self.path);
        sync_autostart(self.settings.autostart);
        self.last_mtime = file_mtime(&self.path);
    }
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_f32(s: &str) -> Option<f32> {
    s.trim().parse::<f32>().ok()
}

fn parse_i32(s: &str) -> Option<i32> {
    s.trim()
        .parse::<i32>()
        .ok()
        .or_else(|| s.trim().parse::<f32>().ok().map(|f| f as i32))
}

fn parse_u32(s: &str) -> Option<u32> {
    s.trim()
        .parse::<u32>()
        .ok()
        .or_else(|| s.trim().parse::<f32>().ok().map(|f| f.max(0.0) as u32))
}

fn parse_usize(s: &str) -> Option<usize> {
    s.trim()
        .parse::<usize>()
        .ok()
        .or_else(|| s.trim().parse::<f32>().ok().map(|f| f.max(0.0) as usize))
}
