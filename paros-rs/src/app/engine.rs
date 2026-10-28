use std::time::{Duration, Instant};

use smithay_client_toolkit::compositor::FrameCallbackData;
use wayland_client::protocol::wl_surface;
use wayland_client::QueueHandle;

use crate::app::types::FocusPhase;
use crate::app::ParosApp;
use crate::card::{build_session_card_full, build_small_pet_card};
use crate::i18n::{detect_lang, tr};
use crate::pet::{PetState, PetWish};
use crate::sound::{play_sound_with_settings, SoundEffect};

impl ParosApp {
    pub fn step_frame(&mut self, qh: &QueueHandle<Self>, surface: &wl_surface::WlSurface) {
        let now = Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32().min(0.05);

        if dt >= 0.014 {
            self.last_frame_time = now;
            let total_w = self.total_desktop_width();
            let total_h = self.total_desktop_height();

            // 1. Rechargement des réglages (SettingsWatcher toutes les 2s)
            if now - self.last_settings_check > Duration::from_secs(2) {
                self.last_settings_check = now;
                self.settings_watcher.poll();
            }
            let settings = self.current_settings().clone();
            let lang = detect_lang(&settings.language);

            // 2. Capteurs système (SystemSense toutes les 1s)
            let sys = self
                .system_sense
                .poll(settings.night_start_hour, settings.night_end_hour);
            self.current_system = sys;

            if sys.cpu_hot && settings.system_alerts {
                for pet in self.pets.values_mut() {
                    pet.roast();
                }
            }

            if let Some(pct) = sys.battery_low
                && settings.system_alerts
            {
                let msg = tr(lang, "Low battery: %d %%").replace("%d", &pct.to_string());
                self.tell_all(&msg, false);
            }

            // Inactivité utilisateur et mouvement du pointeur (conforme à paros-hyprland.py:248)
            let active_win = self.hyprland.active_window();
            let active_pid = active_win.as_ref().map(|w| w.pid).unwrap_or(-1);
            if active_pid != self.last_active_pid {
                self.last_active_pid = active_pid;
                self.last_pointer_move = now;
                if self.user_idle {
                    self.user_idle = false;
                }
            }

            let cursor_global = self.hyprland.cursor_pos().map(|c| (c.x as f32, c.y as f32));
            if let Some(cpos) = cursor_global {
                let moved = ((cpos.0 - self.last_pointer_pos.0).powi(2)
                    + (cpos.1 - self.last_pointer_pos.1).powi(2))
                .sqrt()
                    > 5.0;
                if moved {
                    self.last_pointer_move = now;
                    self.last_pointer_pos = cpos;
                    if self.user_idle {
                        self.user_idle = false;
                    }
                    if let Some(cuddler_id) = self.cuddler_id.take()
                        && let Some(p) = self.pets.get_mut(&cuddler_id)
                    {
                        p.target_x = None;
                        p.then_state = None;
                        if p.state == PetState::Sleep {
                            p.state = PetState::Stretch;
                            p.state_time = 0.0;
                        }
                    }
                }
            }
            if !self.user_idle
                && now - self.last_pointer_move
                    >= Duration::from_secs_f32(settings.idle_minutes * 60.0)
            {
                self.user_idle = true;
            }

            // 3. Mise à jour Hyprland (écrans couverts, verrouillage, perchoir) toutes les 250ms
            if now - self.last_hypr_check > Duration::from_millis(250) {
                self.last_hypr_check = now;
                self.update_hyprland_state();
            }

            // 4. Minuteur Pomodoro global
            match self.focus_phase {
                FocusPhase::Focus(ref mut rem) => {
                    *rem -= dt;
                    if *rem <= 0.0 {
                        self.focus_phase =
                            FocusPhase::Break(settings.focus_break_minutes * 60.0);
                        let msg = tr(lang, "Break! %d min").replace(
                            "%d",
                            &(settings.focus_break_minutes as u32).to_string(),
                        );
                        self.tell_all(&msg, true);
                    }
                }
                FocusPhase::Break(ref mut rem) => {
                    *rem -= dt;
                    if *rem <= 0.0 {
                        self.focus_phase = FocusPhase::Off;
                        let msg = tr(lang, "Break is over. Back to it?").to_string();
                        self.tell_all(&msg, false);
                    }
                }
                FocusPhase::Off => {}
            }

            // 5. Polling des sessions (Claude Code & Antigravity) toutes les 500ms
            if now - self.last_session_check > Duration::from_millis(500) {
                self.last_session_check = now;
                self.poll_sessions(now);
            }

            // 6-10. Interactions sociales, sages, cuddler, tower, pyramides
            self.update_social(now, total_w, cursor_global);

            // 11. Cartes de survol
            if let Some(cursor) = self.hyprland.cursor_pos() {
                let cx = cursor.x as f32;
                let cy = cursor.y as f32;

                for (id, pet) in &mut self.pets {
                    let is_near = !self.is_pointer_down
                        && !self.has_dragged
                        && !pet.is_airborne()
                        && pet.hit_test(cx, cy);
                    pet.hovered = is_near;

                    if is_near {
                        if let Some(session) = self.active_sessions.get(id) {
                            let (f_timer, b_timer) = match self.focus_phase {
                                FocusPhase::Focus(s) => (Some(s), None),
                                FocusPhase::Break(s) => (None, Some(s)),
                                FocusPhase::Off => (None, None),
                            };
                            let sage_elapsed = self
                                .sage_trackers
                                .get(id)
                                .map(|t| now.duration_since(t.since).as_secs());
                            let awaited_peer = session.message_sent.as_ref().map(|dest| {
                                let elapsed = self
                                    .last_message_times
                                    .get(id)
                                    .map_or(0, |t| now.duration_since(*t).as_secs());
                                (dest.as_str(), elapsed)
                            });
                            let compact_elapsed = self
                                .compacting_trackers
                                .get(id)
                                .map(|t| now.duration_since(t.since).as_secs());
                            pet.hover_card = Some(build_session_card_full(
                                session,
                                f_timer,
                                b_timer,
                                sage_elapsed,
                                awaited_peer,
                                compact_elapsed,
                                lang,
                            ));
                        } else {
                            pet.hover_card = None;
                        }
                    } else {
                        pet.hover_card = None;
                    }
                }

                for small in &mut self.small_pets {
                    let small_near = !self.is_pointer_down
                        && !self.has_dragged
                        && !small.is_airborne()
                        && small.hit_test(cx, cy);
                    small.hovered = small_near;
                    if small_near && !small.dismissing {
                        small.hover_card = Some(build_small_pet_card(small));
                    } else {
                        small.hover_card = None;
                    }
                }

                for sage in self.sages.values_mut() {
                    let sage_near = !self.is_pointer_down
                        && !self.has_dragged
                        && !sage.is_airborne()
                        && sage.hit_test(cx, cy);
                    sage.hovered = sage_near;
                }
            }

            // 12. Répulsion mutuelle au repos conforme à pets.gd:108-135 (_spread)
            self.apply_spread((0.0, total_w));

            // 13. Glisser-déposer (Drag & Drop)
            if self.is_pointer_down
                && let Some(cursor) = self.hyprland.cursor_pos()
            {
                let cx = cursor.x as f32;
                let cy = cursor.y as f32;
                if let Some((sx, sy)) = self.drag_start {
                    let dist = ((cx - sx).powi(2) + (cy - sy).powi(2)).sqrt();
                    if dist > 6.0 && !self.has_dragged {
                        self.has_dragged = true;
                        if let Some(ref target_id) = self.drag_target_id {
                            if let Some(pet) = self.pets.get_mut(target_id) {
                                pet.grab(cx, cy);
                            } else if let Some(small) = self
                                .small_pets
                                .iter_mut()
                                .find(|s| &format!("{}/{}", s.parent_id, s.id) == target_id)
                            {
                                small.grab(cx, cy);
                            } else if let Some(sage) = self.sages.get_mut(target_id) {
                                sage.grab(cx, cy);
                            }
                        }
                    }
                }
                if self.has_dragged
                    && let Some(ref target_id) = self.drag_target_id
                {
                    if let Some(pet) = self.pets.get_mut(target_id) {
                        pet.drag_to(cx, cy, dt, total_w, total_h);
                    } else if let Some(small) = self
                        .small_pets
                        .iter_mut()
                        .find(|s| &format!("{}/{}", s.parent_id, s.id) == target_id)
                    {
                        small.drag_to(cx, cy, dt, total_w, total_h);
                    } else if let Some(sage) = self.sages.get_mut(target_id) {
                        sage.drag_to(cx, cy, dt, total_w, total_h);
                    }
                }
            }

            // 14. Mise à jour physique
            let mut play_knock = false;
            let mut play_thud = false;

            for pet in self.pets.values_mut() {
                pet.update(dt, total_w, total_h);
                if pet.knocked_this_frame {
                    play_knock = true;
                }
                if pet.just_landed_hard {
                    pet.just_landed_hard = false;
                    play_thud = true;
                }
            }

            for small in &mut self.small_pets {
                let parent_opt = self.pets.get(&small.parent_id);
                if let Some(parent) = parent_opt {
                    small.home_x = parent.x;
                    small.current_ground = parent.current_ground;
                }
                if small.dismissing {
                    let target_x = parent_opt.map_or(small.home_x, |p| p.x);
                    if !small.dismiss_going
                        && small.state != PetState::Cheer
                        && small.cheer_timer <= 0.0
                        && !small.is_airborne()
                    {
                        small.walk_to(target_x, true);
                        small.dismiss_going = true;
                    }
                }
                small.update(dt, total_w, total_h);
                if small.just_landed_hard {
                    small.just_landed_hard = false;
                    play_thud = true;
                }
            }
            self.small_pets.retain(|s| {
                if s.dismissing {
                    if s.dismiss_timer <= 0.0 {
                        return false;
                    }
                    if s.dismiss_going && (s.state != PetState::Walk || (s.x - s.home_x).abs() <= 15.0) {
                        return false;
                    }
                    true
                } else {
                    true
                }
            });

            for sage in self.sages.values_mut() {
                sage.update(dt, total_w, total_h);
                if sage.just_landed_hard {
                    sage.just_landed_hard = false;
                    play_thud = true;
                }
            }

            if play_knock {
                play_sound_with_settings(
                    SoundEffect::Knock,
                    settings.sound_enabled,
                    settings.sound_volume,
                );
            }
            if play_thud {
                play_sound_with_settings(
                    SoundEffect::Thud,
                    settings.sound_enabled,
                    settings.sound_volume,
                );
            }

            // 15. Lettres volantes
            for letter in &mut self.flying_letters {
                let target_pos = if letter.target_is_sage {
                    self.sages
                        .get(&letter.target_id)
                        .filter(|s| !s.hidden)
                        .map(|s| (s.x, s.y, s.scale * s.stature))
                } else {
                    self.pets
                        .get(&letter.target_id)
                        .filter(|p| !p.hidden)
                        .map(|p| (p.x, p.y, p.scale * p.stature))
                };

                let (tx, ty, ts) = match target_pos {
                    Some(pos) => pos,
                    None => {
                        letter.stop();
                        continue;
                    }
                };

                let landed = letter.update(dt, tx, ty, ts);
                if landed {
                    if letter.target_is_sage {
                        if let Some(sage) = self.sages.get_mut(&letter.target_id) {
                            sage.wish = PetWish::Read;
                            sage.wish_read = true;
                            sage.read_letter();
                        }
                    } else if let Some(pet) = self.pets.get_mut(&letter.target_id)
                        && pet.mail == 0
                        && pet.state == PetState::Idle
                    {
                        pet.read_letter();
                    }
                }
            }
            self.flying_letters.retain(|l| !l.finished);

            self.draw_all(qh);
        }

        surface.frame(qh, FrameCallbackData(surface.clone()));
        surface.commit();
    }
}
