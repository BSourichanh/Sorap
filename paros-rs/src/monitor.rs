#![allow(dead_code)]

use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    shell::{
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface},
        WaylandSurface,
    },
    shm::{slot::SlotPool, Shm},
};
use wayland_client::{protocol::wl_output, QueueHandle};

use crate::hyprland::HyprlandClient;
use crate::ParosApp;

#[derive(Clone, Copy)]
pub struct BoundingBox {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub struct MonitorSurface {
    pub output: wl_output::WlOutput,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub layer: LayerSurface,
    pub pool: SlotPool,
    pub configured: bool,
    pub initial_mapped: bool,
    pub has_content: bool,
    pub last_damage: Option<BoundingBox>,
}

impl MonitorSurface {
    pub fn new(
        qh: &QueueHandle<ParosApp>,
        output: wl_output::WlOutput,
        compositor: &CompositorState,
        layer_shell: &LayerShell,
        shm: &Shm,
    ) -> Option<Self> {
        let surface = compositor.create_surface(qh);
        let layer = layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Overlay,
            Some("sorap"),
            Some(&output),
        );

        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.commit();

        let width = 1920u32;
        let height = 1080u32;
        let pool = match SlotPool::new((width * height * 4) as usize, shm) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Erreur de création de SlotPool : {e}");
                return None;
            }
        };

        Some(Self {
            output,
            name: String::new(),
            x: 0,
            y: 0,
            width,
            height,
            layer,
            pool,
            configured: false,
            initial_mapped: false,
            has_content: false,
            last_damage: None,
        })
    }
}

pub fn update_monitors_from_hyprland(
    monitors: &mut [MonitorSurface],
    output_state: &OutputState,
    hyprland: &HyprlandClient,
) {
    let hypr_monitors = hyprland.monitors();
    if hypr_monitors.is_empty() {
        return;
    }

    for mon in monitors.iter_mut() {
        let output_name = output_state.info(&mon.output).and_then(|info| info.name);
        if let Some(name) = output_name
            && let Some(h) = hypr_monitors.iter().find(|h| h.name == name)
        {
            mon.name = h.name.clone();
            mon.x = h.x;
            mon.y = h.y;
            mon.width = h.width as u32;
            mon.height = h.height as u32;
            continue;
        }

        let output_pos = output_state.info(&mon.output).and_then(|info| info.logical_position);
        if let Some((lx, ly)) = output_pos
            && let Some(h) = hypr_monitors.iter().find(|h| h.x == lx && h.y == ly)
        {
            mon.name = h.name.clone();
            mon.x = h.x;
            mon.y = h.y;
            mon.width = h.width as u32;
            mon.height = h.height as u32;
            continue;
        }
    }
}

pub fn total_desktop_width(monitors: &[MonitorSurface]) -> f32 {
    let max_x = monitors.iter().map(|m| m.x + m.width as i32).max().unwrap_or(1920);
    let min_x = monitors.iter().map(|m| m.x).min().unwrap_or(0);
    ((max_x - min_x).max(1920)) as f32
}

pub fn total_desktop_height(monitors: &[MonitorSurface]) -> f32 {
    let max_y = monitors.iter().map(|m| m.y + m.height as i32).max().unwrap_or(1080);
    let min_y = monitors.iter().map(|m| m.y).min().unwrap_or(0);
    ((max_y - min_y).max(1080)) as f32
}
