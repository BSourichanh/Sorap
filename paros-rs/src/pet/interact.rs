#![allow(dead_code)]

use super::{Pet, PetState, PetWish};

pub trait IntoHurry {
    fn into_hurry(self) -> f32;
}

impl IntoHurry for bool {
    fn into_hurry(self) -> f32 {
        if self {
            Pet::HURRY
        } else {
            1.0
        }
    }
}

impl IntoHurry for f32 {
    fn into_hurry(self) -> f32 {
        self
    }
}

impl Pet {
    pub fn throw_letter(&mut self, side: f32) {
        if !self.is_airborne() && self.state != PetState::Climb {
            self.facing = if side >= 0.0 { 1.0 } else { -1.0 };
            self.state = PetState::Throw;
            self.state_time = 0.0;
            self.vx = 0.0;
        }
    }

    pub fn read_letter(&mut self) {
        if !self.is_airborne() && self.state != PetState::Climb {
            self.state = PetState::Read;
            self.state_time = 0.0;
            self.vx = 0.0;
        }
    }

    pub fn mailbox_side(&self, _screen_w: f32) -> f32 {
        let ground_left = if self.is_perched {
            self.current_perch.map_or(0.0, |[px, _, _, _]| px)
        } else {
            0.0
        };
        if self.x < ground_left + 150.0 * self.scale {
            1.0
        } else {
            -1.0
        }
    }

    pub fn say(&mut self, text: String, duration: f32) {
        if self.discreet || text.is_empty() {
            self.say_text = None;
            self.say_timer = 0.0;
        } else {
            self.say_text = Some(text);
            self.say_timer = duration;
        }
    }

    pub fn walk_to(&mut self, target_x: f32, hurry: impl IntoHurry) {
        self.walk_to_then(target_x, PetState::Idle, hurry);
    }

    pub fn knock_at(&mut self, edge_x: f32, side: f32) {
        self.knock_side = side;
        if self.is_perched {
            self.state = PetState::Knock;
            self.facing = side;
            self.state_time = 0.0;
            self.vx = 0.0;
        } else {
            self.walk_to_then(edge_x, PetState::Knock, Self::KNOCK_HURRY);
        }
    }

    pub fn walk_to_then(&mut self, target_x: f32, then: PetState, hurry: impl IntoHurry) {
        if self.is_airborne() || self.is_perched || self.state == PetState::Climb {
            return;
        }
        self.target_x = Some(target_x);
        self.then_state = Some(then);
        self.state = PetState::Walk;
        self.state_time = 0.0;
        self.walk_duration = fastrand::f32() * 5.0 + 3.0;
        self.facing = if target_x > self.x { 1.0 } else { -1.0 };
        self.target_hurry = hurry.into_hurry();
        self.vx = self.facing * self.walk_speed() * self.target_hurry;
    }

    pub fn walk_off(&mut self, total_w: f32, hurry: f32) {
        if self.is_airborne() || self.state == PetState::Climb {
            return;
        }
        self.is_perched = false;
        self.current_perch = None;
        let exit_left = self.x < total_w / 2.0;
        self.facing = if exit_left { -1.0 } else { 1.0 };
        let target = if exit_left {
            -120.0 * self.scale
        } else {
            total_w + 120.0 * self.scale
        };
        self.walk_to_then(target, PetState::Idle, hurry);
        self.dismissing = true;
    }

    pub fn is_off(&self, total_w: f32) -> bool {
        let margin = 72.0 * self.stature * self.scale;
        self.dismissing && (self.x < -margin || self.x > total_w + margin)
    }

    pub fn cheer(&mut self) {
        if !self.is_airborne() && self.state != PetState::Climb {
            self.state = PetState::Cheer;
            self.state_time = 0.0;
            self.vx = 0.0;
        }
        self.cheer_timer = 1.6;
    }

    pub fn worry(&mut self) {
        if !self.is_airborne() && self.state != PetState::Climb {
            self.state = PetState::Worry;
            self.state_time = 0.0;
            self.vx = 0.0;
        }
    }

    pub fn greet(&mut self, side: f32) {
        if !self.is_airborne() && self.state != PetState::Climb {
            self.facing = if side >= 0.0 { 1.0 } else { -1.0 };
            self.state = PetState::Greet;
            self.state_time = 0.0;
            self.vx = 0.0;
        }
    }

    pub fn glare(&mut self, side: f32) {
        if !self.is_airborne() && self.state != PetState::Climb {
            self.facing = if side >= 0.0 { 1.0 } else { -1.0 };
            self.state = PetState::Glare;
            self.state_time = 0.0;
            self.vx = 0.0;
        }
    }

    pub fn high_five(&mut self, side: f32) {
        if !self.is_airborne() && self.state != PetState::Climb {
            self.facing = if side >= 0.0 { 1.0 } else { -1.0 };
            self.state = PetState::HighFive;
            self.state_time = 0.0;
            self.vx = 0.0;
        }
    }

    pub fn roast(&mut self) {
        if !self.is_airborne() && self.state != PetState::Climb {
            self.state = PetState::Roast;
            self.state_time = 0.0;
            self.vx = 0.0;
        }
    }

    pub fn sweep(&mut self) {
        if !self.is_airborne() && self.state != PetState::Climb {
            self.state = PetState::Sweep;
            self.state_time = 0.0;
            self.vx = 0.0;
        }
    }

    pub fn nod(&mut self) {
        if !self.is_airborne() && self.state != PetState::Climb {
            self.state = PetState::Nod;
            self.state_time = 0.0;
            self.vx = 0.0;
        }
    }

    pub fn shake_off(&mut self) {
        if !self.is_airborne() && self.state != PetState::Climb {
            self.state = PetState::Shake;
            self.state_time = 0.0;
            self.vx = 0.0;
        }
    }

    pub fn look_toward(&mut self, target_x: f32) {
        if matches!(
            self.state,
            PetState::Idle | PetState::Sit | PetState::Sleep | PetState::Think | PetState::Wait
        ) && (target_x - self.x).abs() > 1.0
        {
            self.facing = if target_x > self.x { 1.0 } else { -1.0 };
        }
    }

    pub fn jump(&mut self) {
        if self.is_airborne() || self.rooted || self.state == PetState::Climb {
            return;
        }
        self.state = PetState::Fall;
        self.fall_start_y = self.y;
        self.vy = -Self::JUMP_SPEED * self.scale;
        self.vx = 0.0;
        self.state_time = 0.0;
    }

    pub fn grab(&mut self, mouse_x: f32, mouse_y: f32) {
        self.state = PetState::Carried;
        self.is_perched = false;
        self.grab_offset = (self.x - mouse_x, self.y - mouse_y);
        self.vx = 0.0;
        self.vy = 0.0;
        self.throw_vx = 0.0;
        self.throw_vy = 0.0;
        self.fall_start_y = self.y;
        self.umbrella = false;
        self.state_time = 0.0;
    }

    pub fn drag_to(&mut self, mouse_x: f32, mouse_y: f32, dt: f32, _max_w: f32, _max_h: f32) {
        if self.state == PetState::Carried {
            let target_x = mouse_x + self.grab_offset.0;
            let target_y = mouse_y + self.grab_offset.1;
            let inst_vx = (target_x - self.x) / dt.max(0.001);
            let inst_vy = (target_y - self.y) / dt.max(0.001);
            self.throw_vx = self.throw_vx * 0.65 + inst_vx * 0.35;
            self.throw_vy = self.throw_vy * 0.65 + inst_vy * 0.35;

            self.x = target_x;
            self.y = target_y;
            self.fall_start_y = target_y;
        }
    }

    pub fn release(&mut self) {
        self.release_with_velocity(self.throw_vx, self.throw_vy);
    }

    pub fn release_with_velocity(&mut self, vx: f32, vy: f32) {
        if self.state == PetState::Carried {
            self.state = PetState::Fall;
            self.vx = vx;
            self.vy = vy;
            let speed = self.vx.hypot(self.vy);
            if speed > Self::MAX_THROW_SPEED {
                let factor = Self::MAX_THROW_SPEED / speed;
                self.vx *= factor;
                self.vy *= factor;
            }
            self.fall_start_y = self.y;
            self.umbrella = false;
            self.state_time = 0.0;
        }
    }

    pub fn hit_test(&self, mx: f32, my: f32) -> bool {
        let stat = self.stature * self.scale;
        let rx = 72.0 * stat;
        let ry = 70.0 * stat;
        let cx = self.x;
        let cy = self.y - 54.0 * stat;
        let dx = mx - cx;
        let dy = my - cy;
        (dx / rx).powi(2) + (dy / ry).powi(2) <= 1.0
    }

    /// Vrai lorsque la mascotte est libre d'interagir ou de jouer (small_pets.gd:178, 199 / pet.gd:375)
    #[inline]
    pub fn is_free(&self) -> bool {
        self.wish == PetWish::Roam
            && matches!(self.state, PetState::Idle | PetState::Sit | PetState::Walk)
            && self.target_x.is_none()
            && !self.dismissing
            && !self.rooted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pet::{MascotKind, PetWish};

    #[test]
    fn test_is_free_states_and_conditions() {
        let mut pet = Pet::new(MascotKind::Claude, 100.0, 500.0);
        assert!(pet.is_free(), "Idle pet sans cible doit être libre");

        pet.wish = PetWish::Think;
        assert!(
            !pet.is_free(),
            "Pet avec wish non-Roam ne doit pas être libre"
        );
        pet.wish = PetWish::Roam;

        pet.state = PetState::Sit;
        assert!(pet.is_free(), "Pet assis sans cible doit être libre");

        pet.state = PetState::Walk;
        assert!(pet.is_free(), "Pet marchant sans cible doit être libre");

        pet.target_x = Some(200.0);
        assert!(
            !pet.is_free(),
            "Pet avec cible dirigée ne doit pas être libre"
        );
        pet.target_x = None;

        pet.rooted = true;
        assert!(!pet.is_free(), "Pet rooted ne doit pas être libre");
        pet.rooted = false;

        pet.dismissing = true;
        assert!(
            !pet.is_free(),
            "Pet en renvoi (dismissing) ne doit pas être libre"
        );
        pet.dismissing = false;

        pet.state = PetState::Fall;
        assert!(!pet.is_free(), "Pet en chute ne doit pas être libre");

        pet.state = PetState::Greet;
        assert!(!pet.is_free(), "Pet saluant ne doit pas être libre");
    }

    #[test]
    fn test_hit_test_zero_scale_no_div_by_zero() {
        let mut pet = Pet::new(MascotKind::Claude, 100.0, 500.0);
        pet.scale = 0.0;
        pet.stature = 0.0;
        assert!(!pet.hit_test(100.0, 500.0));
    }

    #[test]
    fn test_safe_clamp_inverted_and_dt_catapult_protection() {
        use crate::pet::physics::safe_clamp;
        assert_eq!(safe_clamp(75.0, 100.0, 50.0), 75.0);
        assert_eq!(safe_clamp(20.0, 10.0, 50.0), 20.0);

        let mut pet = Pet::new(MascotKind::Claude, 100.0, 100.0);
        pet.state = PetState::Fall;
        pet.vy = 0.0;
        pet.update(10.0, 1000.0, 600.0 + 26.0);
        assert!(pet.vy <= 261.0, "vy doit être bornée par dt.clamp(0.0, 0.1)");
    }

    #[test]
    fn test_territory_confinement_no_jitter() {
        let mut pet = Pet::new(MascotKind::Claude, 20.0, 600.0);
        pet.home_x = 20.0;
        pet.home_reach = 10.0;
        pet.state = PetState::Walk;
        assert!(!pet.strays());
        pet.update(0.01, 1000.0, 600.0 + 26.0);
        assert_eq!(pet.vx, 0.0, "La mascotte doit s immobiliser sans osciller/jitter");
        assert_eq!(pet.x, 54.0, "La mascotte se cale au point médian (78 + 30) / 2 = 54");
    }
}
