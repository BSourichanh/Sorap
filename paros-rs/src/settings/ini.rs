use crate::settings::Settings;

pub fn parse_ini(content: &str) -> Settings {
    let mut settings = Settings::default();
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
            ("agy", "desktop") | ("", "agy_desktop") => {
                if let Some(v) = parse_bool(val) {
                    settings.agy.desktop = v;
                }
            }
            ("agy", "desktop_minutes") | ("", "agy_desktop_minutes") => {
                if let Some(v) = parse_f32(val) {
                    settings.agy.desktop_minutes = v.clamp(0.5, 120.0);
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

pub fn to_ini(settings: &Settings) -> String {
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
         [agy]\n\n\
         desktop={}\n\
         desktop_minutes={:.1}\n\n\
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
        settings.language,
        settings.pet_size,
        settings.walk_speed,
        settings.show_name,
        settings.accessories,
        settings.greetings,
        settings.tower,
        settings.headlamp,
        settings.groove,
        settings.sound_enabled,
        settings.sound_volume,
        settings.night_start_hour,
        settings.night_end_hour,
        settings.idle_minutes,
        settings.bubble_enabled,
        settings.bubble_seconds,
        settings.claude_show_activity,
        settings.claude_nag_minutes,
        settings.claude_context_window_k,
        settings.claude_knock,
        settings.claude_sage,
        settings.claude_sage_live,
        settings.agy.desktop,
        settings.agy.desktop_minutes,
        settings.subagents_show,
        settings.subagents_max,
        settings.subagents_size,
        settings.subagents_show_activity,
        settings.subagents_games,
        settings.subagents_pyramid,
        settings.git_enabled,
        settings.desktop_perch,
        settings.desktop_cuddle,
        settings.desktop_leave_fullscreen,
        settings.desktop_lock_screen_minutes,
        settings.focus_minutes,
        settings.focus_break_minutes,
        settings.system_alerts,
        settings.autostart,
    )
}

pub fn unquote(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

pub fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

pub fn parse_f32(s: &str) -> Option<f32> {
    s.trim().parse::<f32>().ok()
}

pub fn parse_i32(s: &str) -> Option<i32> {
    s.trim()
        .parse::<i32>()
        .ok()
        .or_else(|| s.trim().parse::<f32>().ok().map(|f| f as i32))
}

pub fn parse_u32(s: &str) -> Option<u32> {
    s.trim()
        .parse::<u32>()
        .ok()
        .or_else(|| s.trim().parse::<f32>().ok().map(|f| f.max(0.0) as u32))
}

pub fn parse_usize(s: &str) -> Option<usize> {
    s.trim()
        .parse::<usize>()
        .ok()
        .or_else(|| s.trim().parse::<f32>().ok().map(|f| f.max(0.0) as usize))
}
