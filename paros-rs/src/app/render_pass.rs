use smithay_client_toolkit::compositor::Region;
use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::protocol::wl_shm;
use wayland_client::QueueHandle;

use crate::app::ParosApp;

impl ParosApp {
    pub fn draw_all(&mut self, _qh: &QueueHandle<Self>) {
        if self.monitors.is_empty() {
            return;
        }

        let cursor_global = self.hyprland.cursor_pos().map(|c| (c.x as f32, c.y as f32));
        let bubble_enabled = self.current_settings().bubble_enabled;

        for mon in &mut self.monitors {
            if !mon.configured || mon.width == 0 || mon.height == 0 {
                continue;
            }

            let local_cursor = cursor_global.map(|(gx, gy)| (gx - mon.x as f32, gy - mon.y as f32));

            let any_intersect = self.pets.values().any(|pet| {
                if pet.hidden {
                    return false;
                }
                let box_w = if pet.hovered || pet.campfire || pet.hourglass || pet.say_text.is_some() {
                    380.0f32
                } else {
                    260.0f32
                };

                let local_cx = pet.x - mon.x as f32;
                let local_feet_y = pet.y - mon.y as f32;
                let local_ground_y = pet.current_ground - mon.y as f32;

                let p_min_x = local_cx - (box_w / 2.0);
                let p_max_x = local_cx + (box_w / 2.0);
                let p_min_y = (if pet.hovered {
                    local_feet_y - 380.0
                } else {
                    local_feet_y - 250.0
                }).min(local_ground_y - 10.0);
                let p_max_y = local_feet_y.max(local_ground_y) + 40.0;

                p_max_x >= 0.0
                    && p_min_x <= mon.width as f32
                    && p_max_y >= 0.0
                    && p_min_y <= mon.height as f32
            });

            let is_menu_here = self
                .context_menu
                .as_ref()
                .is_some_and(|m| m.mon_name == mon.name);

            let small_intersects = self.small_pets.iter().any(|s| {
                if s.hidden {
                    return false;
                }
                let scx = s.x - mon.x as f32;
                scx >= -60.0 && scx <= mon.width as f32 + 60.0
            });
            let sage_intersects = self.sages.values().any(|s| {
                if s.hidden {
                    return false;
                }
                let scx = s.x - mon.x as f32;
                scx >= -100.0 && scx <= mon.width as f32 + 100.0
            });
            let letter_intersects = self.flying_letters.iter().any(|l| {
                let lx = l.current_x - mon.x as f32;
                let ly = l.current_y - mon.y as f32;
                lx >= -60.0 && lx <= mon.width as f32 + 60.0 && ly >= -60.0 && ly <= mon.height as f32 + 60.0
            });

            let intersects = any_intersect || is_menu_here || small_intersects || sage_intersects || letter_intersects;

            if !intersects {
                if mon.has_content || !mon.initial_mapped {
                    mon.has_content = false;
                    mon.initial_mapped = true;

                    let stride = mon.width as i32 * 4;
                    if let Ok((buffer, canvas)) = mon.pool.create_buffer(
                        mon.width as i32,
                        mon.height as i32,
                        stride,
                        wl_shm::Format::Argb8888,
                    ) && let Some(mut pixmap) =
                        tiny_skia::PixmapMut::from_bytes(canvas, mon.width, mon.height)
                    {
                        pixmap.fill(tiny_skia::Color::TRANSPARENT);
                        mon.layer
                            .wl_surface()
                            .damage_buffer(0, 0, mon.width as i32, mon.height as i32);
                        buffer.attach_to(mon.layer.wl_surface()).ok();
                    }
                    let region = Region::new(&self.compositor).ok();
                    if let Some(ref r) = region {
                        mon.layer
                            .wl_surface()
                            .set_input_region(Some(r.wl_region()));
                    }
                    mon.layer.commit();
                    drop(region);
                }
                continue;
            }

            mon.initial_mapped = true;
            mon.has_content = true;
            let stride = mon.width as i32 * 4;
            let (buffer, canvas) = match mon.pool.create_buffer(
                mon.width as i32,
                mon.height as i32,
                stride,
                wl_shm::Format::Argb8888,
            ) {
                Ok(b) => b,
                Err(_) => continue,
            };

            let Some(mut pixmap) =
                tiny_skia::PixmapMut::from_bytes(canvas, mon.width, mon.height)
            else {
                continue;
            };

            pixmap.fill(tiny_skia::Color::TRANSPARENT);

            // Mascottes adultes
            for pet in self.pets.values() {
                if pet.hidden {
                    continue;
                }
                let local_cx = pet.x - mon.x as f32;
                let local_feet_y = pet.y - mon.y as f32;

                if local_cx >= -150.0 && local_cx <= mon.width as f32 + 150.0 {
                    self.renderer.draw_pet_at(
                        &mut pixmap,
                        pet,
                        local_cx,
                        local_feet_y,
                        local_cursor,
                        None,
                        bubble_enabled,
                    );
                }
            }

            // Sous-agents
            for small in &self.small_pets {
                if small.hidden {
                    continue;
                }
                let s_local_cx = small.x - mon.x as f32;
                let s_local_feet_y = small.y - mon.y as f32;
                if s_local_cx >= -60.0 && s_local_cx <= mon.width as f32 + 60.0 {
                    self.renderer.draw_pet_at(
                        &mut pixmap,
                        small,
                        s_local_cx,
                        s_local_feet_y,
                        None,
                        None,
                        bubble_enabled,
                    );
                }
            }

            // Conseillers Sages
            for sage in self.sages.values() {
                if sage.hidden {
                    continue;
                }
                let sage_cx = sage.x - mon.x as f32;
                let sage_feet_y = sage.y - mon.y as f32;
                if sage_cx >= -100.0 && sage_cx <= mon.width as f32 + 100.0 {
                    self.renderer.draw_pet_at(
                        &mut pixmap,
                        sage,
                        sage_cx,
                        sage_feet_y,
                        None,
                        None,
                        bubble_enabled,
                    );
                }
            }

            // Lettres volantes
            for letter in &self.flying_letters {
                let lx = letter.current_x - mon.x as f32;
                let ly = letter.current_y - mon.y as f32;
                if lx >= -60.0 && lx <= mon.width as f32 + 60.0 && ly >= -60.0 && ly <= mon.height as f32 + 60.0 {
                    self.renderer.draw_flying_letter_at(&mut pixmap, letter, lx, ly);
                }
            }

            // Menu contextuel
            if let Some(ref menu) = self.context_menu
                && mon.name == menu.mon_name
            {
                let labels: Vec<&str> = menu.items_data.iter().map(|(_, l)| l.as_str()).collect();
                self.renderer.draw_context_menu(
                    &mut pixmap,
                    menu.x,
                    menu.y,
                    menu.width,
                    menu.height,
                    &labels,
                    menu.hovered,
                    menu.separator_after,
                );
            }

            mon.layer
                .wl_surface()
                .damage_buffer(0, 0, mon.width as i32, mon.height as i32);
            mon.last_damage = None;

            // Région d'entrée pour clics et interactions
            let region = Region::new(&self.compositor).ok();
            if let Some(ref r) = region {
                for pet in self.pets.values() {
                    if pet.hidden {
                        continue;
                    }
                    let factor = pet.scale * pet.stature;
                    let local_cx = pet.x - mon.x as f32;
                    let local_feet_y = pet.y - mon.y as f32;
                    let pet_min_x = local_cx - 70.0 * factor;
                    let pet_max_x = local_cx + 70.0 * factor;
                    let pet_min_y = local_feet_y - 120.0 * factor;
                    let pet_max_y = local_feet_y + 35.0 * factor;

                    let x0 = pet_min_x.max(0.0).min(mon.width as f32) as i32;
                    let y0 = pet_min_y.max(0.0).min(mon.height as f32) as i32;
                    let x1 = pet_max_x.max(0.0).min(mon.width as f32) as i32;
                    let y1 = pet_max_y.max(0.0).min(mon.height as f32) as i32;
                    let in_w = x1 - x0;
                    let in_h = y1 - y0;
                    if in_w > 0 && in_h > 0 {
                        r.add(x0, y0, in_w, in_h);
                    }
                }

                for small in &self.small_pets {
                    if small.hidden {
                        continue;
                    }
                    let factor = small.scale * small.stature;
                    let s_cx = small.x - mon.x as f32;
                    let s_feet_y = small.y - mon.y as f32;
                    let s_min_x = s_cx - 40.0 * factor;
                    let s_max_x = s_cx + 40.0 * factor;
                    let s_min_y = s_feet_y - 85.0 * factor;
                    let s_max_y = s_feet_y + 20.0 * factor;
                    let x0 = s_min_x.max(0.0).min(mon.width as f32) as i32;
                    let y0 = s_min_y.max(0.0).min(mon.height as f32) as i32;
                    let x1 = s_max_x.max(0.0).min(mon.width as f32) as i32;
                    let y1 = s_max_y.max(0.0).min(mon.height as f32) as i32;
                    let in_w = x1 - x0;
                    let in_h = y1 - y0;
                    if in_w > 0 && in_h > 0 {
                        r.add(x0, y0, in_w, in_h);
                    }
                }

                for sage in self.sages.values() {
                    if sage.hidden {
                        continue;
                    }
                    let factor = sage.scale * sage.stature;
                    let s_cx = sage.x - mon.x as f32;
                    let s_feet_y = sage.y - mon.y as f32;
                    let s_min_x = s_cx - 60.0 * factor;
                    let s_max_x = s_cx + 60.0 * factor;
                    let s_min_y = s_feet_y - 140.0 * factor;
                    let s_max_y = s_feet_y + 30.0 * factor;
                    let x0 = s_min_x.max(0.0).min(mon.width as f32) as i32;
                    let y0 = s_min_y.max(0.0).min(mon.height as f32) as i32;
                    let x1 = s_max_x.max(0.0).min(mon.width as f32) as i32;
                    let y1 = s_max_y.max(0.0).min(mon.height as f32) as i32;
                    let in_w = x1 - x0;
                    let in_h = y1 - y0;
                    if in_w > 0 && in_h > 0 {
                        r.add(x0, y0, in_w, in_h);
                    }
                }

                if let Some(ref menu) = self.context_menu
                    && mon.name == menu.mon_name
                {
                    r.add(0, 0, mon.width as i32, mon.height as i32);
                }

                mon.layer
                    .wl_surface()
                    .set_input_region(Some(r.wl_region()));
            }

            buffer.attach_to(mon.layer.wl_surface()).ok();
            mon.layer.commit();
            drop(region);
        }
    }
}
