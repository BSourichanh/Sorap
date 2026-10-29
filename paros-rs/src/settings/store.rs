use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use crate::settings::Settings;

pub fn data_dir() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
            PathBuf::from(home).join(".local/share")
        })
}

pub fn config_dir() -> PathBuf {
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
    data_dir().join("paros/settings.cfg")
}

pub fn godot_legacy_path() -> PathBuf {
    data_dir().join("godot/app_userdata/Paros/settings.cfg")
}

pub fn fallback_path() -> PathBuf {
    config_dir().join("paros/settings.cfg")
}

pub fn find_settings_file() -> PathBuf {
    let primary = primary_path();
    if primary.exists() {
        return primary;
    }
    let godot_legacy = godot_legacy_path();
    if godot_legacy.exists() {
        return godot_legacy;
    }
    let fallback = fallback_path();
    if fallback.exists() {
        return fallback;
    }
    primary
}

pub fn resolve_path() -> PathBuf {
    find_settings_file()
}

pub fn autostart_desktop_path() -> PathBuf {
    config_dir().join("autostart/paros.desktop")
}

pub fn sync_autostart(enabled: bool) {
    let desktop_path = autostart_desktop_path();
    if !enabled {
        if desktop_path.exists() {
            let _ = fs::remove_file(desktop_path);
        }
        return;
    }

    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
        .unwrap_or_else(|| "sorap".to_string());

    let content = format!(
        "[Desktop Entry]\nType=Application\nName=Sorap\nExec=env XMODIFIERS=@im=none \"{}\"\n",
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
        let path = resolve_path();
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

        let current_path = resolve_path();
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

pub fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}
