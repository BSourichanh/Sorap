mod app;
mod card;
mod hyprland;
mod i18n;
mod letter;
mod menu;
mod monitor;
mod pet;
mod render;
mod session;
mod settings;
mod sound;
mod system;

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use smithay_client_toolkit::{
    compositor::CompositorState,
    data_device_manager::DataDeviceManagerState,
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::wlr_layer::LayerShell,
    shm::Shm,
};
use wayland_client::{globals::registry_queue_init, protocol::wl_output, Connection};

use app::{FocusPhase, ParosApp};
use hyprland::HyprlandClient;
use pet::{MascotKind, Pet};
use render::Renderer;
use session::SessionWatcher;
use settings::SettingsWatcher;
use system::SystemSense;

struct PidGuard(String);

impl Drop for PidGuard {
    fn drop(&mut self) {
        if let Ok(content) = std::fs::read_to_string(&self.0)
            && content.trim() == std::process::id().to_string()
        {
            let _ = std::fs::remove_file(&self.0);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let pid_dir = format!("{runtime_dir}/paros");
    let pid_file = format!("{pid_dir}/paros.pid");

    if let Ok(content) = std::fs::read_to_string(&pid_file)
        && let Ok(other_pid) = content.trim().parse::<i32>()
    {
        let my_pid = std::process::id() as i32;
        if other_pid != my_pid && std::path::Path::new(&format!("/proc/{}", other_pid)).exists() {
            println!(
                "Une autre instance de Sorap est déjà active (PID {}). Arrêt.",
                other_pid
            );
            return Ok(());
        }
    }
    let _ = std::fs::create_dir_all(&pid_dir);
    let _ = std::fs::write(&pid_file, format!("{}\n", std::process::id()));
    let _pid_guard = PidGuard(pid_file);

    println!("Démarrage de Sorap (multi-sessions, multi-écrans Hyprland & parité complète)...");
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh)?;
    let layer_shell = LayerShell::bind(&globals, &qh)?;
    let shm = Shm::bind(&globals, &qh)?;
    let output_state = OutputState::new(&globals, &qh);
    let data_device_manager = DataDeviceManagerState::bind(&globals, &qh)?;
    let seat_state = SeatState::new(&globals, &qh);
    let mut initial_data_device = None;
    for seat in seat_state.seats() {
        if initial_data_device.is_none() {
            initial_data_device = Some(data_device_manager.get_data_device(&qh, &seat));
        }
    }

    let hyprland = HyprlandClient::new();
    let mut pets = HashMap::new();
    let mut initial_pet = Pet::new(MascotKind::Claude, 500.0, 1080.0 - 30.0);
    initial_pet.id = "none".to_string();
    initial_pet.custom_color = Some((217, 119, 87));
    initial_pet.label = String::new();
    initial_pet.accessory = 0;
    initial_pet.home_x = f32::NAN;
    pets.insert("none".to_string(), initial_pet);

    let session_watcher = SessionWatcher::new();
    let mut settings_watcher = SettingsWatcher::new();
    settings_watcher.poll();
    let mut system_sense = SystemSense::new();
    let current_system = system_sense.poll(23, 7);
    let renderer = Renderer::new();

    let mut app = ParosApp {
        registry_state: RegistryState::new(&globals),
        seat_state,
        data_device_manager,
        data_device: initial_data_device,
        dnd_pos: (0.0, 0.0),
        dnd_surface: None,
        output_state,
        compositor,
        layer_shell,
        shm,
        monitors: Vec::new(),
        exit: false,
        pets,
        small_pets: Vec::new(),
        sages: HashMap::new(),
        sage_trackers: HashMap::new(),
        flying_letters: Vec::new(),
        hyprland,
        session_watcher,
        settings_watcher,
        system_sense,
        renderer,
        last_frame_time: Instant::now(),
        last_hypr_check: Instant::now(),
        last_session_check: Instant::now(),
        last_subagent_tick: Instant::now(),
        last_settings_check: Instant::now(),
        last_pointer_move: Instant::now(),
        last_pointer_pos: (0.0, 0.0),
        last_active_pid: -1,
        user_idle: false,
        active_sessions: HashMap::new(),
        last_phases: HashMap::new(),
        last_messages_sent: HashMap::new(),
        last_message_times: HashMap::new(),
        idle_since: HashMap::new(),
        compacting_trackers: HashMap::new(),
        met_cooldowns: HashMap::new(),
        succeeded_times: HashMap::new(),
        knocking_cooldowns: HashMap::new(),
        unfocused_since: HashMap::new(),
        nagged_times: HashMap::new(),
        crowded_since: HashMap::new(),
        asked_peers: HashMap::new(),
        tower: Vec::new(),
        cuddler_id: None,
        pyramids: HashMap::new(),
        owed_celebrations: HashSet::new(),
        screen_locked: false,
        lock_started_at: None,
        lock_start_time: None,
        dpms_off_sent: false,
        dpms_off_needed: false,
        focus_phase: FocusPhase::Off,
        is_pointer_down: false,
        drag_start: None,
        has_dragged: false,
        drag_target_id: None,
        context_menu: None,
        context_menu_pet_id: None,
        last_left_click: None,
        last_left_pos: (0.0, 0.0),
        current_system,
    };

    let outputs: Vec<wl_output::WlOutput> = app.output_state.outputs().collect();
    for out in outputs {
        app.add_monitor(&qh, out);
    }

    event_queue.roundtrip(&mut app)?;
    app.update_monitors_from_hyprland();
    app.draw_all(&qh);

    println!("Moteur actif : surveillance multi-sessions, audio RIFF/WAVE et intégration Hyprland opérationnelles.");
    loop {
        event_queue.blocking_dispatch(&mut app)?;
        if app.exit {
            break;
        }
    }

    Ok(())
}
