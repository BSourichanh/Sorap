use std::collections::{HashMap, HashSet};
use std::time::Instant;

use smithay_client_toolkit::{
    compositor::CompositorState,
    data_device_manager::{data_device::DataDevice, DataDeviceManagerState},
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::wlr_layer::LayerShell,
    shm::Shm,
};
use wayland_client::{
    protocol::{wl_output, wl_surface},
    QueueHandle,
};

use crate::app::state_resolve::resolve_session_from_map;
use crate::app::types::{CompactingTracker, FocusPhase, SageTracker, SubagentPyramid};
use crate::hyprland::HyprlandClient;
use crate::i18n::{detect_lang, Lang};
use crate::letter::FlyingLetter;
use crate::menu::ContextMenu;
use crate::monitor::{total_desktop_height, total_desktop_width, update_monitors_from_hyprland, MonitorSurface};
use crate::pet::Pet;
use crate::render::Renderer;
use crate::session::{SessionPhase, SessionState, SessionWatcher};
use crate::settings::{Settings, SettingsWatcher};
use crate::system::{SystemSense, SystemUpdate};

pub struct ParosApp {
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor: CompositorState,
    pub layer_shell: LayerShell,
    pub shm: Shm,

    pub monitors: Vec<MonitorSurface>,
    pub exit: bool,

    pub pets: HashMap<String, Pet>,
    pub small_pets: Vec<Pet>,
    pub sages: HashMap<String, Pet>,
    pub sage_trackers: HashMap<String, SageTracker>,
    pub flying_letters: Vec<FlyingLetter>,

    pub hyprland: HyprlandClient,
    pub session_watcher: SessionWatcher,
    pub settings_watcher: SettingsWatcher,
    pub system_sense: SystemSense,
    pub renderer: Renderer,

    pub last_frame_time: Instant,
    pub last_hypr_check: Instant,
    pub last_session_check: Instant,
    pub last_subagent_tick: Instant,
    pub last_settings_check: Instant,
    pub last_pointer_move: Instant,
    pub last_pointer_pos: (f32, f32),
    pub last_active_pid: i32,
    pub user_idle: bool,

    pub active_sessions: HashMap<String, SessionState>,
    pub last_phases: HashMap<String, SessionPhase>,
    pub last_messages_sent: HashMap<String, String>,
    pub last_message_times: HashMap<String, Instant>,
    pub idle_since: HashMap<String, Instant>,
    pub compacting_trackers: HashMap<String, CompactingTracker>,
    pub met_cooldowns: HashMap<(String, String), Instant>,
    pub succeeded_times: HashMap<String, Instant>,
    pub knocking_cooldowns: HashMap<String, Instant>,
    pub unfocused_since: HashMap<String, Instant>,
    pub nagged_times: HashMap<String, Instant>,
    pub crowded_since: HashMap<(String, String), Instant>,
    pub asked_peers: HashMap<String, String>,

    pub tower: Vec<String>,
    pub cuddler_id: Option<String>,
    pub pyramids: HashMap<String, SubagentPyramid>,
    pub owed_celebrations: HashSet<String>,
    pub screen_locked: bool,
    pub lock_started_at: Option<Instant>,
    pub lock_start_time: Option<Instant>,
    pub dpms_off_sent: bool,
    pub dpms_off_needed: bool,

    pub focus_phase: FocusPhase,

    // Gestion du glisser-déposer
    pub is_pointer_down: bool,
    pub drag_start: Option<(f32, f32)>,
    pub has_dragged: bool,
    pub drag_target_id: Option<String>,

    // Menu contextuel
    pub context_menu: Option<ContextMenu>,
    pub context_menu_pet_id: Option<String>,

    // Double-clic
    pub last_left_click: Option<Instant>,
    pub last_left_pos: (f32, f32),

    // Glisser-déposer de fichiers Wayland (files_dropped)
    pub data_device_manager: DataDeviceManagerState,
    pub data_device: Option<DataDevice>,
    pub dnd_pos: (f32, f32),
    pub dnd_surface: Option<wl_surface::WlSurface>,

    pub current_system: SystemUpdate,
}

impl ParosApp {
    pub fn add_monitor(&mut self, qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        if self.monitors.iter().any(|m| m.output == output) {
            return;
        }

        if let Some(mon) = MonitorSurface::new(qh, output, &self.compositor, &self.layer_shell, &self.shm) {
            self.monitors.push(mon);
            self.update_monitors_from_hyprland();
        }
    }

    pub fn update_monitors_from_hyprland(&mut self) {
        update_monitors_from_hyprland(&mut self.monitors, &self.output_state, &self.hyprland);
    }

    pub fn total_desktop_width(&self) -> f32 {
        total_desktop_width(&self.monitors)
    }

    pub fn total_desktop_height(&self) -> f32 {
        total_desktop_height(&self.monitors)
    }

    pub fn current_settings(&self) -> &Settings {
        self.settings_watcher.get()
    }

    pub fn current_lang(&self) -> Lang {
        detect_lang(&self.current_settings().language)
    }

    pub fn tell_all(&mut self, text: &str, happy: bool) {
        let bubble_secs = self.current_settings().bubble_seconds;
        for pet in self.pets.values_mut() {
            pet.say(text.to_string(), bubble_secs);
            if happy {
                pet.cheer();
            }
        }
    }

    pub fn check_dpms_timeout(&mut self, now: Instant) {
        let lock_screen_mins = self.current_settings().desktop_lock_screen_minutes;
        let should_dispatch = update_lock_dpms_state(
            self.screen_locked,
            &mut self.lock_started_at,
            &mut self.dpms_off_sent,
            &mut self.dpms_off_needed,
            lock_screen_mins,
            now,
        );
        self.lock_start_time = self.lock_started_at;
        if should_dispatch {
            let _ = self.hyprland.query("dispatch dpms off");
        }
    }

    pub fn update_hyprland_state(&mut self) {
        if !self.hyprland.is_available() {
            return;
        }

        self.screen_locked = self.hyprland.is_locked();
        self.check_dpms_timeout(Instant::now());

        let leave_fullscreen = self.current_settings().desktop_leave_fullscreen;
        let perch_enabled = self.current_settings().desktop_perch;

        if self.screen_locked {
            for pet in self.pets.values_mut() {
                pet.set_perch(None);
                pet.discreet = true;
            }
            for small in &mut self.small_pets {
                small.discreet = true;
            }
            for sage in self.sages.values_mut() {
                sage.discreet = true;
            }
            return;
        }

        for pet in self.pets.values_mut() {
            pet.discreet = false;
        }
        for small in &mut self.small_pets {
            small.discreet = false;
        }
        for sage in self.sages.values_mut() {
            sage.discreet = false;
        }

        if leave_fullscreen {
            let covered = self.hyprland.covered_screens();
            if !covered.is_empty() {
                let free_screens: Vec<[i32; 4]> = self
                    .monitors
                    .iter()
                    .filter(|m| !covered.iter().any(|c| c[0] == m.x && c[1] == m.y))
                    .map(|m| [m.x, m.y, m.width as i32, m.height as i32])
                    .collect();

                let all_covered = free_screens.is_empty();
                for pet in self.pets.values_mut() {
                    if all_covered {
                        pet.hidden = true;
                    } else {
                        pet.hidden = false;
                        pet.evade_screens(&covered, &free_screens);
                    }
                }
                for small in &mut self.small_pets {
                    small.hidden = all_covered;
                }
                for sage in self.sages.values_mut() {
                    sage.hidden = all_covered;
                }
            } else {
                for pet in self.pets.values_mut() {
                    pet.hidden = false;
                }
                for small in &mut self.small_pets {
                    small.hidden = false;
                }
                for sage in self.sages.values_mut() {
                    sage.hidden = false;
                }
            }
        }

        if perch_enabled {
            if let Some(win) = self.hyprland.active_window() {
                if win.fullscreen != 0 || win.fullscreenClient != 0 {
                    for (id, pet) in &mut self.pets {
                        if !self.tower.contains(id) && !pet.rooted {
                            pet.set_perch(None);
                        }
                    }
                } else {
                    let wx = win.at[0] as f32;
                    let wy = win.at[1] as f32;
                    let ww = win.size[0] as f32;
                    let wh = win.size[1] as f32;
                    for (id, pet) in &mut self.pets {
                        if !self.tower.contains(id) && !pet.rooted {
                            pet.set_perch(Some([wx, wy, ww, wh]));
                        }
                    }
                }
            } else {
                for (id, pet) in &mut self.pets {
                    if !self.tower.contains(id) && !pet.rooted {
                        pet.set_perch(None);
                    }
                }
            }
        } else {
            for (id, pet) in &mut self.pets {
                if !self.tower.contains(id) && !pet.rooted {
                    pet.set_perch(None);
                }
            }
        }
    }

    pub fn open_context_menu(&mut self, mon_name: String, x: f32, y: f32, mon_w: f32, mon_h: f32) {
        let parent_id = self.context_menu_pet_id.as_ref().and_then(|id| self.resolve_parent_session(id));
        let has_session = parent_id.as_ref().is_some_and(|pid| self.active_sessions.contains_key(pid) || self.pets.contains_key(pid));
        let (focus_rem, break_rem) = match self.focus_phase {
            FocusPhase::Focus(secs) => (Some((secs / 60.0).ceil() as u32), None),
            FocusPhase::Break(secs) => (None, Some((secs / 60.0).ceil() as u32)),
            FocusPhase::Off => (None, None),
        };
        let lang = self.current_lang();
        let focus_default = self.current_settings().focus_minutes as u32;
        self.context_menu = Some(ContextMenu::new_with_mon(
            mon_name,
            x,
            y,
            mon_w,
            mon_h,
            has_session,
            focus_rem,
            break_rem,
            lang,
            focus_default,
        ));
    }

    pub fn close_context_menu(&mut self) {
        self.context_menu = None;
    }

    pub fn resolve_session_named(&self, target_spec: &str, sender_id: &str) -> Option<String> {
        resolve_session_from_map(&self.active_sessions, target_spec, sender_id)
    }

    pub fn resolve_parent_session(&self, pet_id: &str) -> Option<String> {
        if let Some(idx) = pet_id.find('/') {
            return Some(pet_id[..idx].to_string());
        }
        if pet_id.starts_with("sage_") {
            return Some(pet_id.trim_start_matches("sage_").to_string());
        }
        if self.pets.contains_key(pet_id) || self.active_sessions.contains_key(pet_id) {
            return Some(pet_id.to_string());
        }
        None
    }
}

pub fn update_lock_dpms_state(
    screen_locked: bool,
    lock_started_at: &mut Option<Instant>,
    dpms_off_sent: &mut bool,
    dpms_off_needed: &mut bool,
    lock_screen_mins: f32,
    now: Instant,
) -> bool {
    if screen_locked {
        let start = *lock_started_at.get_or_insert(now);
        if !*dpms_off_sent && lock_screen_mins > 0.0 {
            let elapsed = now.duration_since(start).as_secs_f32();
            if elapsed >= lock_screen_mins * 60.0 {
                *dpms_off_sent = true;
                *dpms_off_needed = true;
                return true;
            }
        }
        false
    } else {
        *lock_started_at = None;
        *dpms_off_sent = false;
        *dpms_off_needed = false;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_lock_dpms_timing_and_reset() {
        let mut lock_started_at = None;
        let mut dpms_off_sent = false;
        let mut dpms_off_needed = false;
        let base = Instant::now();

        // 1. Initial lock sets lock_started_at
        let dispatch = update_lock_dpms_state(
            true,
            &mut lock_started_at,
            &mut dpms_off_sent,
            &mut dpms_off_needed,
            1.0, // 1 minute
            base,
        );
        assert!(!dispatch);
        assert_eq!(lock_started_at, Some(base));
        assert!(!dpms_off_sent);

        // 2. 30s elapsed -> no DPMS yet
        let t_30s = base + Duration::from_secs(30);
        let dispatch = update_lock_dpms_state(
            true,
            &mut lock_started_at,
            &mut dpms_off_sent,
            &mut dpms_off_needed,
            1.0,
            t_30s,
        );
        assert!(!dispatch);
        assert!(!dpms_off_sent);

        // 3. 60s elapsed -> triggers DPMS off
        let t_60s = base + Duration::from_secs(60);
        let dispatch = update_lock_dpms_state(
            true,
            &mut lock_started_at,
            &mut dpms_off_sent,
            &mut dpms_off_needed,
            1.0,
            t_60s,
        );
        assert!(dispatch);
        assert!(dpms_off_sent);
        assert!(dpms_off_needed);

        // 4. Next tick while locked -> doesn't fire again
        let t_70s = base + Duration::from_secs(70);
        let dispatch = update_lock_dpms_state(
            true,
            &mut lock_started_at,
            &mut dpms_off_sent,
            &mut dpms_off_needed,
            1.0,
            t_70s,
        );
        assert!(!dispatch);
        assert!(dpms_off_sent);

        // 5. Unlock -> resets all fields
        let t_unlock = base + Duration::from_secs(80);
        let dispatch = update_lock_dpms_state(
            false,
            &mut lock_started_at,
            &mut dpms_off_sent,
            &mut dpms_off_needed,
            1.0,
            t_unlock,
        );
        assert!(!dispatch);
        assert_eq!(lock_started_at, None);
        assert!(!dpms_off_sent);
        assert!(!dpms_off_needed);
    }
}
