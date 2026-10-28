use std::io::Read;
use smithay_client_toolkit::data_device_manager::{
    data_device::DataDeviceHandler,
    data_offer::{DataOfferHandler, DragOffer},
};
use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::{
    protocol::{
        wl_data_device::WlDataDevice,
        wl_data_device_manager::DndAction,
        wl_surface,
    },
    Connection, QueueHandle,
};

use crate::app::types::url_decode;
use crate::app::ParosApp;
use crate::i18n::tr;
use crate::pet::Pet;

impl DataOfferHandler for ParosApp {
    fn source_actions(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        offer: &mut DragOffer,
        _actions: DndAction,
    ) {
        offer.set_actions(DndAction::Copy, DndAction::Copy);
    }

    fn selected_action(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _offer: &mut DragOffer,
        _actions: DndAction,
    ) {
    }
}

impl DataDeviceHandler for ParosApp {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &WlDataDevice,
        x: f64,
        y: f64,
        wl_surface: &wl_surface::WlSurface,
    ) {
        self.dnd_pos = (x as f32, y as f32);
        self.dnd_surface = Some(wl_surface.clone());
        if let Some(ref dev) = self.data_device
            && let Some(offer) = dev.data().drag_offer()
        {
            if let Some(mime) = offer.with_mime_types(|mimes| {
                if mimes.iter().any(|m| m == "text/uri-list") {
                    Some("text/uri-list".to_string())
                } else {
                    mimes.first().cloned()
                }
            }) {
                offer.accept_mime_type(0, Some(mime));
            }
            offer.set_actions(DndAction::Copy, DndAction::Copy);
        }
    }

    fn leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice) {
        self.dnd_surface = None;
    }

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &WlDataDevice,
        x: f64,
        y: f64,
    ) {
        self.dnd_pos = (x as f32, y as f32);
        if let Some(ref dev) = self.data_device
            && let Some(offer) = dev.data().drag_offer()
        {
            offer.set_actions(DndAction::Copy, DndAction::Copy);
        }
    }

    fn selection(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice) {}

    fn drop_performed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _data_device: &WlDataDevice,
    ) {
        if let Some(ref dev) = self.data_device
            && let Some(offer) = dev.data().drag_offer()
        {
            let mime = "text/uri-list".to_string();
            if let Ok(mut pipe) = offer.receive(mime.clone()) {
                offer.accept_mime_type(1, Some(mime));
                offer.set_actions(DndAction::Copy, DndAction::Copy);
                offer.finish();

                let mut content = String::new();
                if pipe.read_to_string(&mut content).is_ok() {
                    self.handle_files_dropped(&content);
                }
            }
            offer.destroy();
        }
    }
}

impl ParosApp {
    pub fn handle_files_dropped(&mut self, content: &str) {
        let mut paths: Vec<String> = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let raw_path = if let Some(stripped) = trimmed.strip_prefix("file://") {
                stripped
            } else {
                trimmed
            };
            let decoded = url_decode(raw_path);
            if !decoded.is_empty() {
                paths.push(decoded);
            }
        }
        if paths.is_empty() {
            return;
        }

        let formatted = paths
            .iter()
            .map(|p| format!("'{}'", p))
            .collect::<Vec<_>>()
            .join(" ");

        if let Ok(mut child) = std::process::Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(formatted.as_bytes());
            }
            let _ = child.wait();
        }

        let settings = self.current_settings().clone();
        let lang = self.current_lang();
        let msg = tr(lang, "Path copied: paste it in the terminal").to_string();

        let (drop_x, drop_y) = if let Some(ref surf) = self.dnd_surface {
            if let Some(mon) = self.monitors.iter().find(|m| m.layer.wl_surface() == surf) {
                (mon.x as f32 + self.dnd_pos.0, mon.y as f32 + self.dnd_pos.1)
            } else {
                self.dnd_pos
            }
        } else {
            self.dnd_pos
        };

        let mut target_pet: Option<&mut Pet> = None;
        for pet in self.pets.values_mut() {
            if pet.hit_test(drop_x, drop_y) {
                target_pet = Some(pet);
                break;
            }
        }
        if let Some(pet) = target_pet {
            pet.say(msg, settings.bubble_seconds);
        } else {
            let mut closest: Option<(&mut Pet, f32)> = None;
            for pet in self.pets.values_mut() {
                let d = (pet.x - drop_x).abs();
                match closest {
                    Some((_, min_d)) if d < min_d => closest = Some((pet, d)),
                    None => closest = Some((pet, d)),
                    _ => {}
                }
            }
            if let Some((pet, _)) = closest {
                pet.say(msg, settings.bubble_seconds);
            }
        }
    }
}
