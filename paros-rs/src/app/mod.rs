pub mod compacting;
pub mod dnd;
pub mod engine;
pub mod pointer;
pub mod render_pass;
pub mod sessions;
pub mod session_sync;
pub mod social;
pub mod social_pyramids;
pub mod state;
pub mod state_resolve;
pub mod state_spread;
pub mod types;

use std::num::NonZeroU32;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, FrameCallbackData},
    delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState},
    shell::wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    shell::WaylandSurface,
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    protocol::{wl_output, wl_seat, wl_surface},
    Connection, QueueHandle,
};

pub use state::*;
pub use types::*;

impl CompositorHandler for ParosApp {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.step_frame(qh, surface);
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        // En cas de drag entre écrans, ne pas annuler le drag sur leave.
        // Seul le relâchement (Release) finalise le drag.
        if !self.has_dragged && self.is_pointer_down {
            self.is_pointer_down = false;
            self.drag_target_id = None;
            self.drag_start = None;
            self.draw_all(qh);
        }
    }
}

impl OutputHandler for ParosApp {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.add_monitor(qh, output);
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        self.update_monitors_from_hyprland();
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.monitors.retain(|m| m.output != output);
    }
}

impl LayerShellHandler for ParosApp {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        self.monitors
            .retain(|m| m.layer.wl_surface() != layer.wl_surface());
        if self.monitors.is_empty() {
            self.exit = true;
        }
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (w, h) = configure.new_size;

        for mon in &mut self.monitors {
            if mon.layer.wl_surface() == layer.wl_surface() {
                mon.width = NonZeroU32::new(w).map_or(mon.width, NonZeroU32::get);
                mon.height = NonZeroU32::new(h).map_or(mon.height, NonZeroU32::get);
                mon.configured = true;
                break;
            }
        }

        self.update_monitors_from_hyprland();
        self.draw_all(qh);

        for mon in &self.monitors {
            if mon.configured {
                mon.layer
                    .wl_surface()
                    .frame(qh, FrameCallbackData(mon.layer.wl_surface().clone()));
                mon.layer.commit();
            }
        }
    }
}

impl SeatHandler for ParosApp {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        if self.data_device.is_none() {
            self.data_device = Some(self.data_device_manager.get_data_device(qh, &seat));
        }
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            self.seat_state.get_pointer(qh, &seat).ok();
            if self.data_device.is_none() {
                self.data_device = Some(self.data_device_manager.get_data_device(qh, &seat));
            }
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}
}

impl ShmHandler for ParosApp {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_registry!(ParosApp);

impl ProvidesRegistryState for ParosApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

smithay_client_toolkit::delegate_dispatch2!(ParosApp);
