#![allow(dead_code)]

use super::{safe_clamp, Pet, PetState, PetWish};

impl Pet {
    pub fn update(&mut self, dt: f32, screen_w: f32, screen_h: f32) {
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt;
        let prev_state_time = self.state_time;
        self.state_time += dt;
        self.knocked_this_frame = false;

        // Décompte des lunettes de soleil (60s après commit)
        if self.cool_timer > 0.0 { self.cool_timer = (self.cool_timer - dt).max(0.0); }

        // Décompte de la bulle de dialogue temporaire
        if self.discreet {
            self.say_text = None;
            self.say_timer = 0.0;
        } else if self.say_timer > 0.0 {
            self.say_timer = (self.say_timer - dt).max(0.0);
            if self.say_timer <= 0.0 { self.say_text = None; }
        }

        // Décompte du minuteur de focus Pomodoro
        if let Some(ref mut ft) = self.focus_timer {
            *ft -= dt;
            if *ft <= 0.0 { self.focus_timer = None; self.jump(); }
        }

        // Décompte des célébrations (cœurs)
        if self.cheer_timer > 0.0 { self.cheer_timer = (self.cheer_timer - dt).max(0.0); }

        // Décompte pour le clignement des yeux (toutes les 2 à 5 s pendant 0.12 s)
        self.blink_timer -= dt;
        if self.blink_timer < -0.12 { self.blink_timer = fastrand::f32() * 3.0 + 2.0; }

        let gravity = 2600.0f32 * self.scale;
        let walk_speed = self.walk_speed();
        let margin = self.side_margin();

        // Détermination du sol actuel (sol écran ou haut du perchoir)
        let floor = screen_h - 26.0;
        let effective_ground = if self.is_perched {
            self.current_perch.map_or(floor, |[_, py, _, _]| py)
        } else {
            floor
        };
        self.current_ground = effective_ground;

        // Perte de sol / descente brutale : chute immédiate
        if !self.is_airborne() && self.state != PetState::Climb && self.y < effective_ground - 1.0 {
            self.enter_fall();
            self.vx = 0.0;
            self.vy = 0.0;
        }

        match self.state {
            PetState::Idle => {
                self.stop_in_place();
                if !self.is_off(screen_w) {
                    self.x = safe_clamp(self.x, margin, screen_w - margin);
                }

                if self.rooted && self.wish == PetWish::Roam {
                    self.state = PetState::Sit;
                    self.sit_timer = 4.0 + fastrand::f32() * 5.0;
                    self.state_time = 0.0;
                    return;
                }

                if !self.rooted && self.strays() {
                    self.enter_walk();
                    self.facing = if self.home_x > self.x { 1.0 } else { -1.0 };
                    self.target_hurry = Self::STRAY_HURRY;
                    self.vx = self.facing * walk_speed * Self::STRAY_HURRY;
                    return;
                }

                // Escalade dès l'entrée en Idle (frame 0) ou passage en Think
                if prev_state_time == 0.0 || self.wish == PetWish::Think {
                    if self.is_perched {
                        if self.wish == PetWish::Roam && fastrand::f32() < 0.10 {
                            self.enter_fall();
                            self.vx = 0.0;
                            self.vy = 0.0;
                            return;
                        }
                    } else if self.can_climb(floor) {
                        let perch_chance = if self.wish == PetWish::Think {
                            0.60
                        } else if self.wish == PetWish::Roam {
                            0.20
                        } else {
                            0.0
                        };
                        if perch_chance > 0.0 && fastrand::f32() < perch_chance {
                            self.try_climb_perch(screen_w, margin);
                            return;
                        }
                    }
                }

                if let Some(ws) = Self::wish_target_state(self.wish) {
                    self.state = ws;
                    self.state_time = 0.0;
                    return;
                }

                // Décision aléatoire après un temps d'inactivité
                if self.is_small && self.dismissing {
                    self.dismiss_timer -= dt;
                    let target_dist = self.home_x - self.x;
                    if target_dist.abs() > 15.0 {
                        self.enter_walk();
                        self.facing = if target_dist > 0.0 { 1.0 } else { -1.0 };
                        self.vx = self.facing * walk_speed * Self::HURRY;
                    } else {
                        self.vx = 0.0;
                    }
                } else if self.state_time > self.idle_duration {
                    self.state_time = 0.0;
                    self.idle_duration = fastrand::f32() * 4.0 + 2.0;
                    if fastrand::f32() < 0.30 {
                        self.state = PetState::Sit;
                        self.sit_timer = 4.0 + fastrand::f32() * 5.0;
                    } else {
                        self.enter_walk();
                        self.facing = if fastrand::bool() { 1.0 } else { -1.0 };
                        self.vx = self.facing * walk_speed;
                    }
                }
            }

            PetState::Sit => {
                self.stop_in_place();
                if !self.rooted && self.strays() {
                    self.enter_walk();
                    self.facing = if self.home_x > self.x { 1.0 } else { -1.0 };
                    self.target_hurry = Self::STRAY_HURRY;
                    self.vx = self.facing * walk_speed * Self::STRAY_HURRY;
                    return;
                }
                self.sit_timer -= dt;
                if self.wish != PetWish::Roam || (self.sit_timer <= 0.0 && !self.rooted) { self.enter_idle(); }
            }
            PetState::Sleep => {
                self.stop_in_place();
                if self.wish != PetWish::Sleep { self.state = PetState::Stretch; self.state_time = 0.0; }
            }
            PetState::Stretch => {
                self.stop_in_place();
                if self.state_time >= 1.2 { self.enter_idle(); }
            }
            PetState::Think => {
                self.stop_in_place();
                if self.wish != PetWish::Think && !self.thinking { self.enter_idle(); }
            }
            PetState::Alert => {
                self.stop_in_place();
                if self.wish != PetWish::Alert { self.enter_idle(); }
            }
            PetState::Wait => {
                self.stop_in_place();
                if self.wish != PetWish::Wait && !self.waiting { self.enter_idle(); }
            }
            PetState::Compact => {
                self.stop_in_place();
                if self.wish != PetWish::Compact && !self.compacting { self.enter_idle(); }
            }
            PetState::Cheer => {
                self.stop_in_place();
                if self.state_time >= 1.6 { self.enter_idle(); }
            }
            PetState::Greet => {
                self.stop_in_place();
                if self.state_time >= 1.4 {
                    self.facing = -self.facing;
                    self.enter_walk();
                    self.vx = self.facing * walk_speed;
                }
            }
            PetState::Glare => {
                self.stop_in_place();
                if self.state_time >= 1.8 { self.enter_idle(); }
            }
            PetState::HighFive => {
                self.stop_in_place();
                if self.state_time >= 1.3 { self.enter_idle(); }
            }
            PetState::Worry => {
                self.stop_in_place();
                if self.state_time >= 1.5 { self.enter_idle(); }
            }
            PetState::Roast => {
                self.stop_in_place();
                if self.state_time >= 7.0 { self.enter_idle(); }
            }
            PetState::Knock => {
                self.stop_in_place();
                if prev_state_time == 0.0 {
                    self.knocked_this_frame = true;
                }
                if self.state_time >= 1.2 { self.enter_idle(); }
            }
            PetState::Sweep => {
                self.stop_in_place();
                if self.state_time >= 2.2 { self.enter_idle(); }
            }
            PetState::Nod => {
                self.stop_in_place();
                if self.state_time >= 0.8 { self.enter_idle(); }
            }
            PetState::Shake => {
                self.stop_in_place();
                if self.state_time >= 1.0 { self.enter_idle(); }
            }
            PetState::Throw => {
                self.stop_in_place();
                if self.state_time >= 0.5 { self.enter_idle(); }
            }
            PetState::Read => {
                self.stop_in_place();
                if !self.wish_read && self.wish != PetWish::Read && self.state_time >= 1.6 {
                    self.enter_idle();
                }
            }

            PetState::Walk => {
                // Si une cible précise en X est définie (course de naissance, visite, knock, etc.)
                if let Some(tx) = self.target_x {
                    let dx = tx - self.x;
                    if dx.abs() <= 8.0 {
                        self.x = tx;
                        self.target_x = None;
                        self.target_hurry = 1.0;
                        self.sheet = false;
                        let next = self.then_state.take().unwrap_or(PetState::Idle);
                        if next == PetState::Idle {
                            self.enter_idle();
                        } else {
                            if next == PetState::Knock {
                                self.facing = self.knock_side;
                            }
                            self.state = next;
                            self.state_time = 0.0;
                        }
                        self.vx = 0.0;
                    } else {
                        self.facing = if dx > 0.0 { 1.0 } else { -1.0 };
                        self.vx = self.facing * walk_speed * self.target_hurry;
                        self.x += self.vx * dt;
                    }
                    return;
                }

                if self.wish != PetWish::Roam {
                    self.enter_idle();
                    self.vx = 0.0;
                    return;
                }

                if self.strays() {
                    self.facing = if self.home_x > self.x { 1.0 } else { -1.0 };
                    self.target_hurry = Self::STRAY_HURRY;
                } else if self.target_hurry == Self::STRAY_HURRY {
                    self.target_hurry = 1.0;
                }

                self.vx = self.facing * walk_speed * self.target_hurry;
                self.x += self.vx * dt;

                // Limites de déplacement
                if self.is_small {
                    if self.dismissing {
                        self.dismiss_timer -= dt;
                        let target_dist = self.home_x - self.x;
                        if target_dist.abs() <= 15.0 {
                            self.enter_idle();
                            self.vx = 0.0;
                        } else {
                            self.facing = if target_dist > 0.0 { 1.0 } else { -1.0 };
                            self.vx = self.facing * walk_speed * Self::HURRY;
                        }
                    } else {
                        // Contraint les petits personnages dans la zone autour du parent
                        let min_x = (self.home_x - self.home_reach).max(margin);
                        let max_x = (self.home_x + self.home_reach).min(screen_w - margin);
                        if min_x >= max_x {
                            self.x = (min_x + max_x) * 0.5;
                            self.vx = 0.0;
                        } else if self.x < min_x {
                            self.x = min_x;
                            self.facing = 1.0;
                            self.vx = walk_speed;
                        } else if self.x > max_x {
                            self.x = max_x;
                            self.facing = -1.0;
                            self.vx = -walk_speed;
                        }
                    }
                } else if self.is_perched {
                    if let Some([px, _, pw, _]) = self.current_perch {
                        let (min_p, max_p) = self.perch_x_bounds(px, pw);
                        if self.x < min_p {
                            self.x = min_p;
                            self.facing = 1.0;
                            self.vx = walk_speed;
                        } else if self.x > max_p {
                            self.x = max_p;
                            self.facing = -1.0;
                            self.vx = -walk_speed;
                        }
                    }
                } else if !self.strays() && !self.home_x.is_nan() && self.home_reach > 0.0 {
                    let min_x = (self.home_x - self.home_reach).max(margin);
                    let max_x = (self.home_x + self.home_reach).min(screen_w - margin);
                    if min_x >= max_x {
                        self.x = (min_x + max_x) * 0.5;
                        self.vx = 0.0;
                    } else if self.x <= min_x {
                        self.x = min_x;
                        self.facing = 1.0;
                        self.vx = walk_speed;
                    } else if self.x >= max_x {
                        self.x = max_x;
                        self.facing = -1.0;
                        self.vx = -walk_speed;
                    }
                } else if margin >= screen_w - margin {
                    self.x = screen_w * 0.5;
                    self.vx = 0.0;
                } else if self.x < margin {
                    self.x = margin;
                    self.facing = 1.0;
                    self.vx = walk_speed;
                } else if self.x > screen_w - margin {
                    self.x = screen_w - margin;
                    self.facing = -1.0;
                    self.vx = -walk_speed;
                }

                // Arrêt après la durée aléatoire de marche
                if self.state_time > self.walk_duration && !self.strays() {
                    self.enter_idle();
                    self.walk_duration = fastrand::f32() * 5.0 + 3.0;
                }
            }

            PetState::Climb => {
                let progress = (self.state_time / Self::CLIMB_DURATION).min(1.0);
                let base_x = self.climb_from.0 + (self.climb_to.0 - self.climb_from.0) * progress;
                let base_y = self.climb_from.1 + (self.climb_to.1 - self.climb_from.1) * progress;
                let arc = (progress * std::f32::consts::PI).sin() * Self::CLIMB_ARC * self.scale * self.stature;
                self.x = base_x;
                self.y = base_y - arc;
                self.vx = 0.0;
                self.vy = 0.0;

                if progress >= 1.0 {
                    self.x = self.climb_to.0;
                    self.y = self.climb_to.1;
                    self.current_ground = self.climb_to.1;
                    self.is_perched = true;
                    self.enter_idle();
                }
            }

            PetState::Fall => {
                let to_ground = effective_ground - self.y;
                if to_ground > Self::UMBRELLA_HEIGHT * self.scale && self.vy > 0.0 {
                    self.umbrella = true;
                }

                self.vy += gravity * dt;
                if self.umbrella {
                    self.vy = self.vy.min(Self::UMBRELLA_SPEED * self.scale);
                }
                self.x += self.vx * dt;
                self.y += self.vy * dt;

                if self.x < margin {
                    self.x = margin;
                    self.vx = self.vx.abs() * 0.5;
                } else if self.x > screen_w - margin {
                    self.x = screen_w - margin;
                    self.vx = -self.vx.abs() * 0.5;
                }

                if self.vx.abs() > 1.0 {
                    self.facing = if self.vx > 0.0 { 1.0 } else { -1.0 };
                }

                let landing_ground = effective_ground;

                if self.y >= landing_ground {
                    let landing_vy = self.vy;
                    self.y = landing_ground;
                    self.umbrella = false;

                    if landing_vy > Self::THUD_SPEED * self.scale {
                        self.just_landed_hard = true;
                    }

                    if landing_vy > Self::MIN_BOUNCE_SPEED * self.scale {
                        self.vy = -landing_vy * 0.5;
                        self.vx *= 0.5;
                        self.fall_start_y = self.y;
                        self.state = PetState::Fall;
                    } else {
                        self.vy = 0.0;
                        self.vx = 0.0;
                        self.enter_idle();
                    }
                }
            }

            PetState::Carried => {
                self.vx = 0.0;
                self.vy = 0.0;
            }
        }
    }
}
