use std::time::{Duration, Instant};
use super::SessionWatcher;

impl SessionWatcher {
    pub(crate) fn check_mpris(&mut self) -> bool {
        if self.last_mpris_scan.elapsed() < Duration::from_secs(4) {
            return self.cached_mpris;
        }
        self.last_mpris_scan = Instant::now();

        let playerctl_res = std::process::Command::new("playerctl")
            .args(["-a", "status"])
            .output();

        let status = match playerctl_res {
            Ok(ref o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .any(|line| line.trim() == "Playing")
            }
            _ => Self::check_mpris_gdbus(),
        };

        self.cached_mpris = status;
        status
    }

    pub(crate) fn check_mpris_gdbus() -> bool {
        let Ok(output) = std::process::Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                "org.freedesktop.DBus",
                "--object-path",
                "/org/freedesktop/DBus",
                "--method",
                "org.freedesktop.DBus.ListNames",
                "--timeout",
                "2",
            ])
            .output()
        else {
            return false;
        };

        let names = String::from_utf8_lossy(&output.stdout);
        for token in names.split(|c: char| c == '\'' || c == '"' || c == ',' || c.is_whitespace()) {
            if token.starts_with("org.mpris.MediaPlayer2.")
                && let Ok(st_out) = std::process::Command::new("gdbus")
                    .args([
                        "call",
                        "--session",
                        "--dest",
                        token,
                        "--object-path",
                        "/org/mpris/MediaPlayer2",
                        "--method",
                        "org.freedesktop.DBus.Properties.Get",
                        "org.mpris.MediaPlayer2.Player",
                        "PlaybackStatus",
                        "--timeout",
                        "2",
                    ])
                    .output()
                {
                    let st_s = String::from_utf8_lossy(&st_out.stdout);
                    if st_s.contains("'Playing'") || st_s.contains("\"Playing\"") {
                        return true;
                    }
                }
        }
        false
    }
}
