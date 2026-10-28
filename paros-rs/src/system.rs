#![allow(dead_code)]

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const HOT_MILLICELSIUS: i64 = 92_000;
const BUSY_LOAD: f32 = 0.9;
const LOW_BATTERY_PERCENT: u32 = 15;
const ALERT_COOLDOWN: Duration = Duration::from_secs(600);
const HOUR_CACHE_TTL: Duration = Duration::from_secs(30);

const THERMAL_PATH: &str = "/sys/class/thermal";
const POWER_PATH: &str = "/sys/class/power_supply";
const POLL_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemSenseState {
    pub load: f32,
    pub cpu_hot: bool,
    pub battery_low: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemUpdate {
    pub load: f32,
    pub cpu_hot: bool,
    pub battery_low: Option<u32>,
    pub is_night: bool,
    pub hour: u32,
}

#[repr(C)]
struct LibcTm {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
    tm_gmtoff: i64,
    tm_zone: *const std::ffi::c_char,
}

unsafe extern "C" {
    fn localtime_r(timep: *const i64, result: *mut LibcTm) -> *mut LibcTm;
}

pub struct SystemSense {
    last_cpu_hot_alert: Option<Instant>,
    last_battery_alert: Option<Instant>,
    last_poll: Option<Instant>,
    cached_state: Option<SystemSenseState>,
    cached_hour: u32,
    last_hour_check: Option<Instant>,
    cores: f32,
}

impl Default for SystemSense {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemSense {
    pub fn new() -> Self {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get() as f32)
            .unwrap_or(1.0)
            .max(1.0);
        let hour = query_local_hour().unwrap_or(12);
        Self {
            last_cpu_hot_alert: None,
            last_battery_alert: None,
            last_poll: None,
            cached_state: None,
            cached_hour: hour,
            last_hour_check: Some(Instant::now()),
            cores,
        }
    }

    pub fn poll(&mut self, night_start_hour: u32, night_end_hour: u32) -> SystemUpdate {
        let now = Instant::now();
        let (load, cpu_hot, battery_low) = if let (Some(last), Some(state)) = (self.last_poll, self.cached_state) {
            if now.duration_since(last) < POLL_INTERVAL {
                (state.load, false, None)
            } else {
                self.poll_sensors(now)
            }
        } else {
            self.poll_sensors(now)
        };

        let hour = self.local_hour();
        let is_night = Self::is_night(hour, night_start_hour, night_end_hour);

        SystemUpdate {
            load,
            cpu_hot,
            battery_low,
            is_night,
            hour,
        }
    }

    fn poll_sensors(&mut self, now: Instant) -> (f32, bool, Option<u32>) {
        let load = self.read_load();
        let hottest_mc = read_max_thermal_millicelsius();

        let is_hot_condition = hottest_mc >= HOT_MILLICELSIUS || load >= BUSY_LOAD;
        let cpu_hot = if is_hot_condition {
            let can_fire = self
                .last_cpu_hot_alert
                .is_none_or(|last| now.duration_since(last) >= ALERT_COOLDOWN);
            if can_fire {
                self.last_cpu_hot_alert = Some(now);
                true
            } else {
                false
            }
        } else {
            false
        };

        let battery_low = if let Some(pct) = read_discharging_low_battery() {
            let can_fire = self
                .last_battery_alert
                .is_none_or(|last| now.duration_since(last) >= ALERT_COOLDOWN);
            if can_fire {
                self.last_battery_alert = Some(now);
                Some(pct)
            } else {
                None
            }
        } else {
            None
        };

        let state = SystemSenseState {
            load,
            cpu_hot,
            battery_low,
        };
        self.last_poll = Some(now);
        self.cached_state = Some(state);

        (load, cpu_hot, battery_low)
    }

    pub fn local_hour(&mut self) -> u32 {
        let now = Instant::now();
        let needs_refresh = self
            .last_hour_check
            .is_none_or(|last| now.duration_since(last) >= HOUR_CACHE_TTL);
        if needs_refresh {
            self.last_hour_check = Some(now);
            if let Some(h) = query_local_hour_libc().or_else(query_local_hour_cmd) {
                self.cached_hour = h;
            }
        }
        self.cached_hour
    }

    pub fn is_night(hour: u32, night_start_hour: u32, night_end_hour: u32) -> bool {
        let start = night_start_hour.min(23);
        let end = night_end_hour.min(23);
        if start > end {
            hour >= start || hour < end
        } else {
            hour >= start && hour < end
        }
    }

    fn read_load(&self) -> f32 {
        let Ok(content) = fs::read_to_string("/proc/loadavg") else {
            return 0.0;
        };
        let Some(first) = content.split_whitespace().next() else {
            return 0.0;
        };
        let Ok(load_1m) = first.parse::<f32>() else {
            return 0.0;
        };
        let raw = load_1m / self.cores;
        (raw * 10.0).round() / 10.0
    }
}

fn read_max_thermal_millicelsius() -> i64 {
    let Ok(entries) = fs::read_dir(THERMAL_PATH) else {
        return 0;
    };
    let mut hottest = 0i64;
    for entry in entries.flatten() {
        let temp_path = entry.path().join("temp");
        if let Some(val) = read_trimmed(&temp_path).and_then(|s| s.parse::<i64>().ok())
            && val > hottest
        {
            hottest = val;
        }
    }
    hottest
}

fn read_discharging_low_battery() -> Option<u32> {
    let entries = fs::read_dir(POWER_PATH).ok()?;
    for entry in entries.flatten() {
        let dir = entry.path();
        let kind = read_trimmed(&dir.join("type")).unwrap_or_default();
        if kind != "Battery" {
            continue;
        }
        let status = read_trimmed(&dir.join("status")).unwrap_or_default();
        if status != "Discharging" {
            continue;
        }
        if let Some(pct) = read_trimmed(&dir.join("capacity")).and_then(|s| s.parse::<u32>().ok())
            && pct <= LOW_BATTERY_PERCENT
        {
            return Some(pct);
        }
    }
    None
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn query_local_hour() -> Option<u32> {
    query_local_hour_libc().or_else(query_local_hour_cmd)
}

fn query_local_hour_libc() -> Option<u32> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    let mut tm = LibcTm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null(),
    };
    // SAFETY: `secs` and `tm` are valid stack pointers; `localtime_r` is thread-safe POSIX API.
    let ptr = unsafe { localtime_r(&secs, &mut tm) };
    if ptr.is_null() || !(0..=23).contains(&tm.tm_hour) {
        None
    } else {
        Some(tm.tm_hour as u32)
    }
}

fn query_local_hour_cmd() -> Option<u32> {
    Command::new("date")
        .arg("+%H")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|&h| h <= 23)
}
