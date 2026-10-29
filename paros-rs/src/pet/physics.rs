#![allow(dead_code)]

use super::{Pet, PetState};

pub const MAX_THROW_SPEED: f32 = 3000.0;
pub const BAGGAGE_DRAG: f32 = 0.2;
pub const CLIMB_DURATION: f32 = 0.7;
pub const CLIMB_ARC: f32 = 120.0;
pub const MIN_BOUNCE_SPEED: f32 = 350.0;
pub const THUD_SPEED: f32 = 500.0;
pub const UMBRELLA_HEIGHT: f32 = 350.0;
pub const UMBRELLA_SPEED: f32 = 220.0;
pub const JUMP_SPEED: f32 = 430.0;
pub const SIDE_MARGIN: f32 = 78.0;
pub const HURRY: f32 = 2.2;
pub const STRAY_HURRY: f32 = 2.2;
pub const KNOCK_HURRY: f32 = 2.4;

#[inline]
pub fn safe_clamp(val: f32, min: f32, max: f32) -> f32 {
    if min <= max {
        val.clamp(min, max)
    } else {
        (min + max) * 0.5
    }
}

impl Pet {
    pub const MAX_THROW_SPEED: f32 = MAX_THROW_SPEED;
    pub const BAGGAGE_DRAG: f32 = BAGGAGE_DRAG;
    pub const CLIMB_DURATION: f32 = CLIMB_DURATION;
    pub const CLIMB_ARC: f32 = CLIMB_ARC;
    pub const MIN_BOUNCE_SPEED: f32 = MIN_BOUNCE_SPEED;
    pub const THUD_SPEED: f32 = THUD_SPEED;
    pub const UMBRELLA_HEIGHT: f32 = UMBRELLA_HEIGHT;
    pub const UMBRELLA_SPEED: f32 = UMBRELLA_SPEED;
    pub const JUMP_SPEED: f32 = JUMP_SPEED;
    pub const SIDE_MARGIN: f32 = SIDE_MARGIN;
    pub const HURRY: f32 = HURRY;
    pub const STRAY_HURRY: f32 = STRAY_HURRY;
    pub const KNOCK_HURRY: f32 = KNOCK_HURRY;

    /// Marge latérale adaptative aux bords d écran : (150 - (150 - 78) * stature) * scale
    #[inline]
    pub fn side_margin(&self) -> f32 {
        (150.0 - (150.0 - Self::SIDE_MARGIN) * self.stature) * self.scale
    }

    /// Vitesse de marche : walk_speed_base * pace * drag * scale
    #[inline]
    pub fn walk_speed(&self) -> f32 {
        let drag = 1.0 - Self::BAGGAGE_DRAG * (self.baggage.min(3) as f32);
        self.walk_speed_base * self.pace * drag * self.scale
    }

    pub const fn is_airborne(&self) -> bool {
        matches!(self.state, PetState::Fall | PetState::Carried)
    }

    pub const fn stacked(&self) -> bool {
        self.rooted && self.is_perched
    }

    pub fn push(&mut self, amount: f32) {
        if self.rooted {
            return;
        }
        self.vx = amount;
        self.facing = if amount >= 0.0 { 1.0 } else { -1.0 };
        if self.state == PetState::Idle {
            self.enter_walk();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pet::types::{MascotKind, PetState, PetWish};

    #[test]
    fn test_fall_thresholds_independent_of_stature() {
        let mut small_pet = Pet::new_small(
            "p1".to_string(),
            "s1".to_string(),
            "sub".to_string(),
            MascotKind::Claude,
            200.0,
            600.0,
            100.0,
            1.0,
        );
        small_pet.scale = 1.0;
        assert_eq!(small_pet.stature, 0.52);

        // Seuil parachute : 350.0 * scale (sans stature)
        small_pet.state = PetState::Fall;
        small_pet.y = 100.0;
        small_pet.vy = 100.0;
        // distance au sol (600.0) = 500.0 > 350.0
        small_pet.update(0.01, 1000.0, 600.0 + 26.0);
        assert!(small_pet.umbrella, "Umbrella must open when fall distance > 350 * scale");

        // Seuil thud et rebond sans stature
        let mut pet_land = Pet::new_small(
            "p1".to_string(),
            "s2".to_string(),
            "sub".to_string(),
            MascotKind::Claude,
            200.0,
            600.0,
            100.0,
            1.0,
        );
        pet_land.state = PetState::Fall;
        pet_land.y = 599.0;
        pet_land.vy = 400.0; // > 350.0 mais < 500.0
        pet_land.update(0.02, 1000.0, 600.0 + 26.0);
        assert!(!pet_land.just_landed_hard, "400 < 500: pas d'impact lourd");
        assert_eq!(pet_land.state, PetState::Fall, "400 > 350: doit rebondir en état Fall");
        assert!(pet_land.vy < 0.0, "vy inversée lors du rebond");
    }


    #[test]
    fn test_is_airborne() {
        let mut pet = Pet::new(MascotKind::Claude, 100.0, 500.0);
        assert!(!pet.is_airborne());

        pet.state = PetState::Walk;
        assert!(!pet.is_airborne());

        pet.state = PetState::Sit;
        assert!(!pet.is_airborne());

        pet.state = PetState::Climb;
        assert!(!pet.is_airborne(), "Climb n'est pas aéroporté (conforme à pet.gd:367)");

        pet.state = PetState::Fall;
        assert!(pet.is_airborne(), "Fall est aéroporté");

        pet.state = PetState::Carried;
        assert!(pet.is_airborne(), "Carried est aéroporté");
    }


    #[test]
    fn test_knock_at_and_in_air_orientation() {
        let mut pet = Pet::new(MascotKind::Claude, 500.0, 600.0);
        // Knock non perché -> court avec KNOCK_HURRY = 2.4
        pet.knock_at(100.0, -1.0);
        assert_eq!(pet.state, PetState::Walk);
        assert_eq!(pet.target_x, Some(100.0));
        assert_eq!(pet.target_hurry, Pet::KNOCK_HURRY);
        assert_eq!(pet.knock_side, -1.0);

        // Knock perché -> bascule immédiatement en Knock orienté
        pet.is_perched = true;
        pet.knock_at(100.0, 1.0);
        assert_eq!(pet.state, PetState::Knock);
        assert_eq!(pet.facing, 1.0);

        // Orientation en l'air (Fall)
        let mut fall_pet = Pet::new(MascotKind::Claude, 500.0, 300.0);
        fall_pet.state = PetState::Fall;
        fall_pet.vx = -50.0;
        fall_pet.facing = 1.0;
        fall_pet.update(0.01, 1000.0, 600.0);
        assert_eq!(fall_pet.facing, -1.0, "Facing orienté selon le signe de vx");
    }


    #[test]
    fn test_loss_of_ground_fall() {
        let mut pet = Pet::new(MascotKind::Claude, 100.0, 300.0);
        pet.vx = 50.0;
        pet.update(0.01, 1000.0, 800.0);
        assert_eq!(pet.state, PetState::Fall, "Perte de sol doit basculer immédiatement en Fall");
        assert_eq!(pet.vx, 0.0, "vx doit être remise à 0.0 lors de la chute");
        // Gravité appliquée lors de la chute réelle : 2600 * 1.0 * 0.01 = 26.0
        assert_eq!(pet.vy, 26.0, "vy accélérée par gravité");
    }


    #[test]
    fn test_new_small_parent_scale() {
        let small_1 = Pet::new_small(
            "p1".to_string(),
            "s1".to_string(),
            "sub".to_string(),
            MascotKind::Claude,
            500.0,
            600.0,
            50.0,
            2.0,
        );
        let dist_1 = (small_1.target_x.unwrap() - 500.0).abs();
        // BIRTH_RUN = (60..130) * 2.0 = 120..260
        assert!(dist_1 >= 119.0 && dist_1 <= 261.0);
    }


    #[test]
    fn test_rooted_protection_in_sit() {
        let mut pet = Pet::new(MascotKind::Claude, 100.0, 600.0);
        pet.home_x = 500.0;
        pet.home_reach = 100.0;
        pet.rooted = true;
        pet.state = PetState::Sit;
        pet.sit_timer = 5.0;
        // x = 100.0 est hors de 500 +- 100 -> pet.strays() est vrai
        assert!(pet.strays());
        // Lors de l'update, rooted empêche le déclenchement de Walk
        pet.update(0.1, 1000.0, 600.0);
        assert_eq!(pet.state, PetState::Sit);
        assert_eq!(pet.vx, 0.0);
    }


    #[test]
    fn test_sage_speed_and_jump_reset_vx() {
        let sage = Pet::new_sage(100.0, 500.0, 500.0);
        assert_eq!(sage.pace, 1.0, "Le Sage doit avoir pace = 1.0");
        assert_eq!(sage.target_hurry, 2.0);
        assert_eq!(sage.vx, sage.walk_speed());

        let mut pet = Pet::new(MascotKind::Claude, 100.0, 500.0);
        pet.vx = 150.0;
        pet.jump();
        assert_eq!(pet.state, PetState::Fall);
        assert_eq!(pet.vx, 0.0, "jump() doit remettre vx à 0.0");
    }


    #[test]
    fn test_scale_factor_and_sage_target() {
        let mut pet = Pet::new(MascotKind::Claude, 500.0, 500.0);
        pet.scale = 1.5;
        pet.stature = 1.0;
        assert_eq!(pet.scale_factor(), 1.5);

        let small = Pet::new_small("parent".into(), "small".into(), "agent".into(), MascotKind::Claude, 500.0, 500.0, 100.0, 1.0);
        assert_eq!(small.scale_factor(), 1.0 * 0.52);

        // Sage recall distance with scale_factor
        let side = 1.0;
        let sage_gap = 230.0;
        let target_x = pet.x + side * sage_gap * pet.scale_factor();
        assert_eq!(target_x, 500.0 + 230.0 * 1.5);
    }

    #[test]
    fn test_side_margin_adaptive() {
        let mut pet = Pet::new(MascotKind::Claude, 100.0, 500.0);
        pet.scale = 1.0;
        pet.stature = 1.0;
        assert!((pet.side_margin() - 78.0).abs() < 1e-4);

        pet.stature = 0.52;
        let expected_margin = (150.0 - (150.0 - 78.0) * 0.52) * 1.0;
        assert!((pet.side_margin() - expected_margin).abs() < 1e-4);
        assert!((pet.side_margin() - 112.56).abs() < 1e-2);
    }


    #[test]
    fn test_strays_detection() {
        let mut pet = Pet::new(MascotKind::Claude, 500.0, 600.0);
        // Par défaut pour un adulte : home_x = NAN, home_reach = 0.0 -> errance libre
        assert!(pet.home_x.is_nan());
        assert_eq!(pet.home_reach, 0.0);
        assert!(!pet.strays());

        pet.home_x = 500.0;
        pet.home_reach = 100.0;
        pet.x = 500.0;
        assert!(!pet.strays());

        // Limite exacte (500 - 100 - 1.0 = 399.0)
        pet.x = 399.5;
        assert!(!pet.strays());
        pet.x = 398.9;
        assert!(pet.strays());

        // Limite supérieure (500 + 100 + 1.0 = 601.0)
        pet.x = 600.5;
        assert!(!pet.strays());
        pet.x = 601.5;
        assert!(pet.strays());

        // Si perché, ne s'égare jamais
        pet.is_perched = true;
        assert!(!pet.strays());

        // Si home_x == 0.0, ne s'égare pas
        pet.is_perched = false;
        pet.home_x = 0.0;
        assert!(!pet.strays());
    }


    #[test]
    fn test_subagent_cadence_and_boundary_bounce() {
        let mut small_pet = Pet::new_small(
            "p1".to_string(),
            "s1".to_string(),
            "sub".to_string(),
            MascotKind::Claude,
            500.0,
            600.0,
            50.0,
            1.0,
        );
        // Au départ (naissance) : target_x défini, course dirigée avec HURRY = 2.2
        assert!(small_pet.target_x.is_some());
        assert_eq!(small_pet.target_hurry, Pet::HURRY);

        // Une fois libre en marche :
        small_pet.target_x = None;
        small_pet.target_hurry = 1.0;
        small_pet.state = PetState::Walk;
        small_pet.home_x = 500.0;
        small_pet.home_reach = 50.0;
        small_pet.x = 440.0; // En-dessous de min_x (500 - 50 = 450)
        let ws = small_pet.walk_speed();
        small_pet.update(0.01, 1000.0, 600.0 + 26.0);
        // Le rebond ne doit PAS avoir le multiplicateur artificiel 1.5x
        assert_eq!(small_pet.vx, ws, "Rebond de bordure a vitesse de marche normale (sans 1.5x)");
    }


    #[test]
    fn test_walk_does_not_stop_if_strays() {
        let mut pet = Pet::new(MascotKind::Claude, 100.0, 600.0);
        pet.home_x = 500.0;
        pet.home_reach = 100.0;
        pet.state = PetState::Walk;
        pet.walk_duration = 1.0;
        pet.state_time = 2.0;
        assert!(pet.strays());
        pet.update(0.01, 1000.0, 600.0 + 26.0);
        assert_eq!(pet.state, PetState::Walk, "Ne doit pas quitter Walk tant qu'elle s'égare");
    }


    #[test]
    fn test_walk_off_idle_no_clamp() {
        let mut pet = Pet::new(MascotKind::Claude, 1000.0, 600.0);
        let screen_w = 1000.0;
        let margin = pet.side_margin();
        pet.walk_off(screen_w, 1.0);
        assert!(pet.dismissing);
        pet.x = screen_w + 80.0;
        assert!(pet.is_off(screen_w));
        pet.state = PetState::Idle;
        pet.update(0.01, screen_w, 600.0 + 26.0);
        assert_eq!(pet.x, screen_w + 80.0, "La mascotte sortie d'écran ne doit pas être ramenée à margin");

        // Mascotte sur l'écran (non sortie) : doit être clampée
        let mut pet_on = Pet::new(MascotKind::Claude, screen_w + 10.0, 600.0);
        pet_on.state = PetState::Idle;
        pet_on.update(0.01, screen_w, 600.0 + 26.0);
        assert_eq!(pet_on.x, screen_w - margin);
    }


    #[test]
    fn test_walk_speed_scale() {
        let mut pet = Pet::new(MascotKind::Claude, 100.0, 500.0);
        pet.walk_speed_base = 70.0;
        pet.pace = 1.0;
        pet.scale = 1.5;
        pet.baggage = 0;
        assert_eq!(pet.walk_speed(), 70.0 * 1.5);

        pet.baggage = 1; // drag = 0.8
        assert_eq!(pet.walk_speed(), 70.0 * 0.8 * 1.5);
    }

}
