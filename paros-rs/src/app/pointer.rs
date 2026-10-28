use std::time::{Duration, Instant};
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind, PointerHandler};
use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::{protocol::wl_pointer, Connection, QueueHandle};

use crate::app::types::FocusPhase;
use crate::app::ParosApp;
use crate::i18n::{detect_lang, tr};
use crate::menu::ContextMenuAction;

impl PointerHandler for ParosApp {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            let mon_info = self
                .monitors
                .iter()
                .find(|m| m.layer.wl_surface() == &event.surface)
                .map(|m| (m.name.clone(), m.width as f32, m.height as f32));

            let (local_x, local_y) = (event.position.0 as f32, event.position.1 as f32);

            match event.kind {
                PointerEventKind::Motion { .. } => {
                    if let Some(menu) = &mut self.context_menu
                        && mon_info.as_ref().is_some_and(|(name, _, _)| name == &menu.mon_name)
                        && menu.update_hover(local_x, local_y)
                    {
                        self.draw_all(qh);
                    }
                }
                PointerEventKind::Press { button, .. } => {
                    let now = Instant::now();
                    let cur_pos = if let Some(c) = self.hyprland.cursor_pos() {
                        (c.x as f32, c.y as f32)
                    } else if let Some(mon) = self.monitors.iter().find(|m| m.layer.wl_surface() == &event.surface) {
                        (local_x + mon.x as f32, local_y + mon.y as f32)
                    } else {
                        (local_x, local_y)
                    };

                    let mut clicked_pet_id: Option<String> = None;
                    for (id, pet) in &self.pets {
                        if pet.hit_test(cur_pos.0, cur_pos.1) {
                            clicked_pet_id = Some(id.clone());
                            break;
                        }
                    }
                    if clicked_pet_id.is_none() {
                        for small in &self.small_pets {
                            if small.hit_test(cur_pos.0, cur_pos.1) {
                                clicked_pet_id = Some(format!("{}/{}", small.parent_id, small.id));
                                break;
                            }
                        }
                    }
                    if clicked_pet_id.is_none() {
                        for (id, sage) in &self.sages {
                            if sage.hit_test(cur_pos.0, cur_pos.1) {
                                clicked_pet_id = Some(id.clone());
                                break;
                            }
                        }
                    }

                    if button == 0x110 {
                        // Clic gauche
                        let is_double_click = self.last_left_click.is_some_and(|last| {
                            now.duration_since(last) < Duration::from_millis(300)
                                && ((cur_pos.0 - self.last_left_pos.0).powi(2) + (cur_pos.1 - self.last_left_pos.1).powi(2)).sqrt() < 30.0
                        });

                        self.last_left_click = Some(now);
                        self.last_left_pos = cur_pos;

                        if is_double_click {
                            self.last_left_click = None;
                            if let Some(ref target_id) = clicked_pet_id {
                                let session_key = target_id.split('/').next().unwrap_or(target_id);
                                let terminal_pid = self.active_sessions.get(session_key).and_then(|s| s.terminal_pid);
                                let session_name = self.active_sessions.get(session_key).map(|s| s.name.clone()).unwrap_or_default();
                                if let Some(pid) = terminal_pid {
                                    self.hyprland.focus_terminal(pid, &session_name);
                                    self.hyprland.ring_terminal(pid);
                                }
                                return;
                            }
                        }

                        if let Some(menu) = &self.context_menu {
                            let in_menu = mon_info.as_ref().is_some_and(|(name, _, _)| menu.contains(name, local_x, local_y));
                            if in_menu {
                                let action = menu.get_action_at(local_y);
                                self.close_context_menu();

                                if let Some(act) = action {
                                    match act {
                                        ContextMenuAction::FocusTerminal => {
                                            if let Some(ref pet_id) = self.context_menu_pet_id {
                                                let session_key = pet_id.split('/').next().unwrap_or(pet_id);
                                                let terminal_pid = self.active_sessions.get(session_key).and_then(|s| s.terminal_pid);
                                                let session_name = self.active_sessions.get(session_key).map(|s| s.name.clone()).unwrap_or_default();
                                                if let Some(pid) = terminal_pid {
                                                    self.hyprland.focus_terminal(pid, &session_name);
                                                    self.hyprland.ring_terminal(pid);
                                                }
                                            }
                                        }
                                        ContextMenuAction::RingTerminal => {
                                            if let Some(ref pet_id) = self.context_menu_pet_id {
                                                let session_key = pet_id.split('/').next().unwrap_or(pet_id);
                                                let terminal_pid = self.active_sessions.get(session_key).and_then(|s| s.terminal_pid);
                                                if let Some(pid) = terminal_pid {
                                                    self.hyprland.ring_terminal(pid);
                                                }
                                            }
                                        }
                                        ContextMenuAction::ToggleFocusTimer => {
                                            let s = self.current_settings().clone();
                                            let lang = detect_lang(&s.language);
                                            match self.focus_phase {
                                                FocusPhase::Off => {
                                                    self.focus_phase = FocusPhase::Focus(s.focus_minutes * 60.0);
                                                    let msg = tr(lang, "Focus: %d min").replace("%d", &(s.focus_minutes as u32).to_string());
                                                    self.tell_all(&msg, false);
                                                }
                                                FocusPhase::Focus(_) | FocusPhase::Break(_) => {
                                                    self.focus_phase = FocusPhase::Off;
                                                }
                                            }
                                        }
                                        ContextMenuAction::OpenSettings => {
                                            let current_exe = std::env::current_exe().unwrap_or_default();
                                            let project_dir = current_exe
                                                .ancestors()
                                                .find(|p| p.join("project.godot").exists())
                                                .map(|p| p.to_path_buf())
                                                .unwrap_or_else(|| std::path::PathBuf::from("/home/user/Documents/Paros"));

                                            crate::sound::reap_zombies();
                                            if let Ok(mut child) = std::process::Command::new("godot")
                                                .env("XMODIFIERS", "@im=none")
                                                .arg("--path")
                                                .arg(project_dir)
                                                .arg("--")
                                                .arg("--settings")
                                                .spawn()
                                            {
                                                let _ = child.try_wait();
                                            }
                                        }
                                        ContextMenuAction::Quit => {
                                            let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
                                                .unwrap_or_else(|_| "/tmp".to_string());
                                            let pid_file = format!("{runtime_dir}/paros/paros.pid");
                                            if let Ok(content) = std::fs::read_to_string(&pid_file)
                                                && content.trim() == std::process::id().to_string()
                                            {
                                                let _ = std::fs::remove_file(&pid_file);
                                            }
                                            self.exit = true;
                                        }
                                    }
                                }
                                self.draw_all(qh);
                                return;
                            } else {
                                self.close_context_menu();
                                self.draw_all(qh);
                            }
                        }

                        if let Some(target_id) = clicked_pet_id {
                            self.is_pointer_down = true;
                            self.drag_target_id = Some(target_id);
                            self.drag_start = Some(cur_pos);
                            self.has_dragged = false;
                            self.draw_all(qh);
                        }
                    } else if button == 0x111 {
                        // Clic droit
                        self.context_menu_pet_id = clicked_pet_id;
                        if let Some((name, w, h)) = mon_info {
                            self.open_context_menu(name, local_x, local_y, w, h);
                            self.draw_all(qh);
                        }
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    self.is_pointer_down = false;
                    if button == 0x110 {
                        if let Some(ref target_id) = self.drag_target_id {
                            if let Some(pet) = self.pets.get_mut(target_id) {
                                if self.has_dragged {
                                    pet.release();
                                } else {
                                    pet.cheer();
                                }
                            } else if let Some(small) = self.small_pets.iter_mut().find(|s| &format!("{}/{}", s.parent_id, s.id) == target_id) {
                                if self.has_dragged {
                                    small.release();
                                } else {
                                    small.cheer();
                                }
                            } else if let Some(sage) = self.sages.get_mut(target_id) {
                                if self.has_dragged {
                                    sage.release();
                                } else {
                                    sage.cheer();
                                }
                            }
                        }
                        self.drag_target_id = None;
                        self.drag_start = None;
                        self.has_dragged = false;
                        self.draw_all(qh);
                    }
                }
                PointerEventKind::Leave { .. } if !self.has_dragged && self.is_pointer_down => {
                    // Un drag actif traverse les moniteurs sans être interrompu par la sortie de surface.
                    // Seul PointerEventKind::Release finalise le lâcher de la mascotte.
                    self.is_pointer_down = false;
                    self.drag_target_id = None;
                    self.drag_start = None;
                    self.draw_all(qh);
                }
                _ => {}
            }
        }
    }
}
