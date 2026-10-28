#![allow(dead_code)]

use std::cell::Cell;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct HyprWorkspace {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(non_snake_case)]
pub struct HyprWindow {
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub at: [i32; 2],
    #[serde(default)]
    pub size: [i32; 2],
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub pid: i32,
    #[serde(default)]
    pub mapped: bool,
    #[serde(default)]
    pub fullscreen: i32,
    #[serde(default)]
    pub fullscreenClient: i32,
    #[serde(default)]
    pub workspace: Option<HyprWorkspace>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct HyprCursor {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(non_snake_case)]
pub struct HyprMonitor {
    pub id: i32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    #[serde(default)]
    pub activeWorkspace: Option<HyprWorkspace>,
    #[serde(default)]
    pub dpmsStatus: Option<bool>,
}

pub struct HyprlandClient {
    socket_path: Option<PathBuf>,
    pub last_lock_check: Cell<Instant>,
    pub cached_locked: Cell<bool>,
    pub last_cursor_check: Cell<Instant>,
    pub cached_cursor: Cell<Option<HyprCursor>>,
}

impl Default for HyprlandClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HyprlandClient {
    pub fn new() -> Self {
        let socket_path = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").and_then(|sig| {
            let xdg = std::env::var_os("XDG_RUNTIME_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/tmp"));
            let path = xdg.join("hypr").join(sig).join(".socket.sock");
            if path.exists() {
                Some(path)
            } else {
                None
            }
        });

        let now = Instant::now();
        let past = now.checked_sub(Duration::from_secs(2)).unwrap_or(now);

        Self {
            socket_path,
            last_lock_check: Cell::new(past),
            cached_locked: Cell::new(false),
            last_cursor_check: Cell::new(past),
            cached_cursor: Cell::new(None),
        }
    }

    pub fn is_available(&self) -> bool {
        self.socket_path.is_some() || std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
    }

    pub fn query(&self, cmd: &str) -> Option<String> {
        if let Some(path) = self.socket_path.as_ref()
            && let Ok(mut stream) = UnixStream::connect(path)
        {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(15)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(15)));

            if stream.write_all(cmd.as_bytes()).is_ok() {
                let mut output = Vec::new();
                if stream.read_to_end(&mut output).is_ok()
                    && let Ok(s) = String::from_utf8(output)
                {
                    return Some(s);
                }
            }
        }

        // Repli CLI hyprctl
        let mut command = Command::new("hyprctl");
        if let Some(sub) = cmd.strip_prefix("j/") {
            command.arg(sub).arg("-j");
        } else if let Some(disp) = cmd.strip_prefix("dispatch ") {
            command.arg("dispatch");
            for part in disp.split_whitespace() {
                command.arg(part);
            }
        } else {
            for part in cmd.split_whitespace() {
                command.arg(part);
            }
        }
        let output = command.output().ok()?;
        String::from_utf8(output.stdout).ok()
    }

    pub fn active_window(&self) -> Option<HyprWindow> {
        let raw = self.query("j/activewindow")?;
        if raw.trim().is_empty() || raw.trim() == "{}" {
            return None;
        }
        let win: HyprWindow = serde_json::from_str(&raw).ok()?;
        if win.mapped && win.class != "Paros" && win.class != "paros-rs" {
            Some(win)
        } else {
            None
        }
    }

    pub fn cursor_pos(&self) -> Option<HyprCursor> {
        let now = Instant::now();
        let last = self.last_cursor_check.get();
        if now.duration_since(last) < Duration::from_millis(15) {
            return self.cached_cursor.get();
        }
        self.last_cursor_check.set(now);
        let raw = self.query("j/cursorpos")?;
        let pos: Option<HyprCursor> = serde_json::from_str(&raw).ok();
        self.cached_cursor.set(pos);
        pos
    }

    pub fn monitors(&self) -> Vec<HyprMonitor> {
        let raw = match self.query("j/monitors") {
            Some(r) => r,
            None => return Vec::new(),
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn clients(&self) -> Vec<HyprWindow> {
        let raw = match self.query("j/clients") {
            Some(r) => r,
            None => return Vec::new(),
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    /// Détecte les moniteurs occupés par une fenêtre en plein écran [x, y, w, h]
    pub fn covered_screens(&self) -> Vec<[i32; 4]> {
        let monitors = self.monitors();
        let clients = self.clients();
        let mut covered = Vec::new();

        for c in clients {
            if c.fullscreen != 0 || c.fullscreenClient != 0 {
                let ws_id = c.workspace.as_ref().map(|w| w.id).unwrap_or(-1);
                for m in &monitors {
                    if let Some(ref m_ws) = m.activeWorkspace
                        && m_ws.id == ws_id
                    {
                        let rect = [m.x, m.y, m.width, m.height];
                        if !covered.contains(&rect) {
                            covered.push(rect);
                        }
                    }
                }
            }
        }
        covered
    }

    /// Détecte si l'écran est verrouillé (hyprlock, swaylock, waylock).
    /// Utilise un cache de 1,5 s calqué sur `paros-hyprland.py:256-258` pour éviter de scanner `/proc` en boucle.
    pub fn is_locked(&self) -> bool {
        let now = Instant::now();
        let last = self.last_lock_check.get();
        let cached = self.cached_locked.get();

        if now.duration_since(last) >= Duration::from_millis(1500) {
            self.last_lock_check.set(now);

            let lockers = ["hyprlock", "swaylock", "waylock"];
            let mut locked = false;
            if let Ok(entries) = std::fs::read_dir("/proc") {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let pid_str = file_name.to_string_lossy();
                    if pid_str.chars().all(|c| c.is_ascii_digit()) {
                        let comm_path = format!("/proc/{pid_str}/comm");
                        if let Ok(comm) = std::fs::read_to_string(comm_path)
                            && lockers.contains(&comm.trim())
                        {
                            locked = true;
                            break;
                        }
                    }
                }
            }
            self.cached_locked.set(locked);
            locked
        } else {
            cached
        }
    }

    /// Remonte la lignée des PID (/proc/<pid>/status PPid) pour trouver la fenêtre terminal correspondante
    pub fn focus_terminal(&self, pid: i32, title_hint: &str) -> bool {
        if pid <= 0 {
            return false;
        }

        let mut pids = HashSet::new();
        pids.insert(pid);

        let mut cur = pid;
        for _ in 0..12 {
            let status_path = format!("/proc/{cur}/status");
            if let Ok(content) = std::fs::read_to_string(&status_path) {
                let mut found_ppid = None;
                for line in content.lines() {
                    if line.starts_with("PPid:") {
                        if let Some(ppid_str) = line.split_whitespace().nth(1)
                            && let Ok(ppid) = ppid_str.parse::<i32>()
                            && ppid > 1
                        {
                            found_ppid = Some(ppid);
                        }
                        break;
                    }
                }
                if let Some(ppid) = found_ppid {
                    pids.insert(ppid);
                    cur = ppid;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let clients = self.clients();
        let mut target_addr = None;

        // 1. Recherche exacte par PID ancêtre et titre
        for c in &clients {
            if pids.contains(&c.pid) {
                if !title_hint.is_empty() && c.title.contains(title_hint) {
                    target_addr = Some(c.address.clone());
                    break;
                }
                if target_addr.is_none() {
                    target_addr = Some(c.address.clone());
                }
            }
        }

        // 2. Si non trouvé, recherche par correspondance de classe d'émulateur terminal
        if target_addr.is_none() {
            let term_classes = ["kitty", "alacritty", "foot", "wezterm", "gnome-terminal", "konsole", "xterm"];
            for c in &clients {
                let c_class = c.class.to_lowercase();
                if term_classes.iter().any(|t| c_class.contains(t))
                    && !title_hint.is_empty()
                    && c.title.contains(title_hint)
                {
                    target_addr = Some(c.address.clone());
                    break;
                }
            }
        }

        if let Some(addr) = target_addr {
            let cmd = format!("dispatch focuswindow address:{addr}");
            let _ = self.query(&cmd);
            true
        } else {
            // Fallback générique
            let cmd = format!("dispatch focuswindow pid:{pid}");
            let _ = self.query(&cmd);
            false
        }
    }

    /// Fait sonner le terminal en injectant \a (ASCII 0x07) dans son descripteur stdin
    /// (aligné sur `desktop.gd:254-256`, sans effet sonore additionnel)
    pub fn ring_terminal(&self, pid: i32) {
        if pid <= 0 {
            return;
        }

        let tty_path = format!("/proc/{pid}/fd/0");
        if Path::new(&tty_path).exists()
            && let Ok(mut f) = std::fs::OpenOptions::new().write(true).open(&tty_path).or_else(|_| File::create(&tty_path))
        {
            let _ = f.write_all(b"\x07");
            return;
        }

        // Si échec sur le processus direct, essayer sur le parent
        let status_path = format!("/proc/{pid}/status");
        if let Ok(content) = std::fs::read_to_string(&status_path) {
            for line in content.lines() {
                if line.starts_with("PPid:") {
                    if let Some(ppid_str) = line.split_whitespace().nth(1)
                        && let Ok(ppid) = ppid_str.parse::<i32>()
                    {
                        let parent_tty = format!("/proc/{ppid}/fd/0");
                        if let Ok(mut f) = std::fs::OpenOptions::new().write(true).open(&parent_tty).or_else(|_| File::create(&parent_tty)) {
                            let _ = f.write_all(b"\x07");
                        }
                    }
                    break;
                }
            }
        }
    }
}
