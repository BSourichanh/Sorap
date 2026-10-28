#![allow(dead_code, unused_imports)]

pub mod interact;
pub mod physics;
pub mod types;

use std::time::Instant;

pub use interact::*;
pub use physics::*;
pub use types::*;

#[derive(Debug, Clone)]
pub struct Pet {
    pub kind: MascotKind,
    pub state: PetState,
    pub wish: PetWish,
    pub x: f32, pub y: f32, pub vx: f32, pub vy: f32, pub facing: f32, pub ground_y: f32,
    pub state_time: f32, pub last_tick: Instant,
    pub time: f32, pub blink_timer: f32, pub pace: f32, pub walk_speed_base: f32,
    pub current_ground: f32, pub current_perch: Option<[f32; 4]>, pub is_perched: bool,
    pub climb_from: (f32, f32), pub climb_to: (f32, f32),
    pub scale: f32, pub stature: f32, pub is_small: bool,
    pub id: String, pub parent_id: String,
    pub dismissing: bool, pub dismiss_going: bool, pub dismiss_timer: f32,
    pub home_x: f32, pub home_reach: f32, pub caption: String,
    pub target_x: Option<f32>, pub then_state: Option<PetState>, pub target_hurry: f32, pub grab_offset: (f32, f32),
    pub sheet: bool, pub umbrella: bool, pub fall_start_y: f32,
    pub throw_vx: f32, pub throw_vy: f32, pub sit_timer: f32,
    pub idle_duration: f32, pub walk_duration: f32,
    pub label: String, pub dispute: Option<(String, Instant)>, pub accessory: usize,
    pub hard_hat: bool, pub baggage: usize, pub smoking: bool, pub hourglass: bool,
    pub campfire: bool, pub compacting: bool, pub headlamp: bool, pub grooving: bool,
    pub cool_timer: f32, pub lost: bool, pub serving: bool, pub tapping: bool,
    pub meditating: bool, pub strained: bool, pub discreet: bool, pub hidden: bool,
    pub urgent: bool, pub rooted: bool, pub custom_color: Option<(u8, u8, u8)>,
    pub knocked_this_frame: bool, pub say_text: Option<String>, pub say_timer: f32,
    pub activity_text: String, pub mail: usize, pub is_sage: bool, pub wish_read: bool,
    pub hovered: bool, pub hover_card: Option<String>, pub just_landed_hard: bool,
    pub focus_timer: Option<f32>, pub cheer_timer: f32, pub waiting: bool, pub thinking: bool,
    pub knock_side: f32, pub fullness: f32,
}

impl Pet {
    pub fn new(kind: MascotKind, initial_x: f32, ground_y: f32) -> Self {
        Self {
            kind, state: PetState::Idle, wish: PetWish::Roam,
            x: initial_x, y: ground_y, vx: 0.0, vy: 0.0, facing: 1.0, ground_y,
            state_time: 0.0, last_tick: Instant::now(),
            time: 0.0, blink_timer: 3.0, pace: 1.0, walk_speed_base: 70.0,
            current_ground: ground_y, current_perch: None, is_perched: false,
            climb_from: (initial_x, ground_y), climb_to: (initial_x, ground_y),
            scale: 1.0, stature: 1.0, is_small: false,
            id: String::new(), parent_id: String::new(),
            dismissing: false, dismiss_going: false, dismiss_timer: 0.0,
            home_x: f32::NAN, home_reach: 0.0, caption: String::new(),
            target_x: None, then_state: None, target_hurry: 1.0, grab_offset: (0.0, 0.0),
            sheet: false, umbrella: false, fall_start_y: ground_y,
            throw_vx: 0.0, throw_vy: 0.0, sit_timer: 0.0,
            idle_duration: fastrand::f32() * 4.0 + 2.0,
            walk_duration: fastrand::f32() * 5.0 + 3.0,
            label: String::new(), dispute: None, accessory: 0,
            hard_hat: false, baggage: 0, smoking: false, hourglass: false,
            campfire: false, compacting: false, headlamp: false, grooving: false,
            cool_timer: 0.0, lost: false, serving: false, tapping: false,
            meditating: false, strained: false, discreet: false, hidden: false,
            urgent: false, rooted: false, custom_color: None, knocked_this_frame: false,
            say_text: None, say_timer: 0.0, activity_text: String::new(),
            mail: 0, is_sage: false, wish_read: false,
            hovered: false, hover_card: None, just_landed_hard: false,
            focus_timer: None, cheer_timer: 0.0, waiting: false, thinking: false,
            knock_side: 1.0, fullness: 0.0,
        }
    }

    pub fn new_sage(spawn_x: f32, ground_y: f32, target_x: f32) -> Self {
        let mut pet = Self::new(MascotKind::Claude, spawn_x, ground_y);
        pet.stature = 1.5;
        pet.is_sage = true;
        pet.pace = 1.0;
        pet.state = PetState::Walk;
        pet.target_x = Some(target_x);
        pet.target_hurry = 2.0;
        pet.facing = if target_x > spawn_x { 1.0 } else { -1.0 };
        pet.vx = pet.facing * pet.walk_speed();
        pet
    }

    #[inline]
    pub fn scale_factor(&self) -> f32 {
        self.scale * self.stature
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_small(
        parent_id: String,
        id: String,
        role: String,
        kind: MascotKind,
        parent_x: f32,
        ground_y: f32,
        home_reach: f32,
        parent_scale: f32,
    ) -> Self {
        let side = if fastrand::bool() { 1.0 } else { -1.0 };
        let run_dist = (60.0 + fastrand::f32() * 70.0) * parent_scale; // BIRTH_RUN = 60..130 px * parent_scale
        let target = parent_x + side * run_dist;

        let mut pet = Self::new(kind, parent_x, ground_y);
        pet.parent_id = parent_id;
        pet.id = id;
        pet.label = role;
        pet.stature = 0.52;
        pet.is_small = true;
        pet.home_x = parent_x;
        pet.home_reach = home_reach;
        pet.pace = 1.0;
        pet.state = PetState::Walk;
        pet.facing = side;
        pet.target_x = Some(target);
        pet.target_hurry = Self::HURRY;
        pet.sheet = true; // Tient sa feuille de papier au départ
        pet.vx = side * (pet.walk_speed() * Self::HURRY);
        pet
    }

    /// Vrai lorsque la mascotte s'éloigne au-delà de sa zone d'errance (pet.gd:683-687)
    pub fn strays(&self) -> bool {
        if self.is_perched || self.home_x.is_nan() || self.home_x == 0.0 {
            return false;
        }
        self.x < self.home_x - self.home_reach - 1.0 || self.x > self.home_x + self.home_reach + 1.0
    }

    pub fn enter_idle(&mut self) {
        self.state = PetState::Idle;
        self.state_time = 0.0;
        self.idle_duration = fastrand::f32() * 4.0 + 2.0;
    }

    pub fn enter_walk(&mut self) {
        self.state = PetState::Walk;
        self.state_time = 0.0;
        self.walk_duration = fastrand::f32() * 5.0 + 3.0;
    }

    pub fn enter_fall(&mut self) {
        self.state = PetState::Fall;
        self.state_time = 0.0;
        self.is_perched = false;
        self.fall_start_y = self.y;
    }

    #[inline]
    fn stop_in_place(&mut self) {
        self.vx = 0.0;
        self.vy = 0.0;
    }

    pub const fn wish_target_state(wish: PetWish) -> Option<PetState> {
        wish.target_state()
    }

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
