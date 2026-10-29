#![allow(dead_code, unused_imports)]

pub mod behavior;
pub mod interact;
pub mod motion;
pub mod physics;
pub mod types;

use std::time::Instant;

pub use behavior::*;
pub use interact::*;
pub use motion::*;
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
    pub(crate) fn stop_in_place(&mut self) {
        self.vx = 0.0;
        self.vy = 0.0;
    }

    pub const fn wish_target_state(wish: PetWish) -> Option<PetState> {
        wish.target_state()
    }

}
