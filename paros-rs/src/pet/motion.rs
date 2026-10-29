#![allow(dead_code)]

use super::{safe_clamp, Pet, PetState};

impl Pet {
    /// Bornes horizontales des pieds sur un perchoir : [px + margin, px + pw - margin]
    /// Si pw < 2.0 * margin, centré à px + pw / 2.0
    #[inline]
    pub fn perch_x_bounds(&self, px: f32, pw: f32) -> (f32, f32) {
        let margin = self.side_margin();
        if pw < 2.0 * margin {
            let center = px + pw / 2.0;
            (center, center)
        } else {
            (px + margin, px + pw - margin)
        }
    }

    /// Vrai si un perchoir valide existe pour l escalade (pet.gd:600-605)
    pub fn can_climb(&self, floor: f32) -> bool {
        if self.is_perched {
            return false;
        }
        if let Some([_px, py, pw, _]) = self.current_perch {
            pw >= 200.0 * self.scale * self.stature
                && py < floor - 100.0 * (self.scale * self.stature)
                && py < self.y - 40.0
        } else {
            false
        }
    }

    /// Tente de monter sur le perchoir actuel
    pub fn try_climb_perch(&mut self, screen_w: f32, margin: f32) -> bool {
        if let Some([px, py, pw, _]) = self.current_perch {
            let (min_p, max_p) = self.perch_x_bounds(px, pw);
            let min_p = min_p.max(margin);
            let max_p = max_p.min(screen_w - margin);
            let target_x = if max_p > min_p {
                min_p + fastrand::f32() * (max_p - min_p)
            } else {
                min_p
            };
            self.climb_onto(target_x, py);
            true
        } else {
            false
        }
    }

    pub fn climb_onto(&mut self, target_x: f32, target_y: f32) {
        self.climb_from = (self.x, self.y);
        self.climb_to = (target_x, target_y);
        self.facing = if target_x >= self.x { 1.0 } else { -1.0 };
        self.is_perched = false;
        self.state = PetState::Climb;
        self.state_time = 0.0;
        self.vx = 0.0;
        self.vy = 0.0;
    }

    pub fn step_aside(&mut self, from_x: f32, gap: f32, bounds: (f32, f32)) {
        if self.rooted || self.is_airborne() || self.state == PetState::Climb {
            return;
        }
        let margin = self.side_margin();
        let (min_x, max_x) = if self.is_perched {
            if let Some([px, _, pw, _]) = self.current_perch {
                let (min_p, max_p) = self.perch_x_bounds(px, pw);
                let min_p = min_p.max(bounds.0 + margin);
                let max_p = max_p.min(bounds.1 - margin);
                if max_p >= min_p {
                    (min_p, max_p)
                } else {
                    let center = px + pw / 2.0;
                    (center, center)
                }
            } else {
                (bounds.0 + margin, bounds.1 - margin)
            }
        } else {
            (bounds.0 + margin, bounds.1 - margin)
        };

        let side = if self.x >= from_x { 1.0 } else { -1.0 };
        let mut target = from_x + side * gap;
        if target < min_x || target > max_x {
            target = from_x - side * gap;
        }
        let target = safe_clamp(target, min_x, max_x);

        self.target_x = Some(target);
        self.then_state = Some(PetState::Idle);
        self.state = PetState::Walk;
        self.state_time = 0.0;
        self.walk_duration = fastrand::f32() * 5.0 + 3.0;
        self.facing = if target >= self.x { 1.0 } else { -1.0 };
        self.target_hurry = 1.0;
        self.vx = self.facing * self.walk_speed();
    }

    pub fn set_perch(&mut self, perch: Option<[f32; 4]>) {
        if self.current_perch == perch {
            return;
        }

        let old_perch = self.current_perch;
        self.current_perch = perch;

        if let Some([px, py, pw, _]) = perch {
            // Si on était déjà perché et que le perchoir a bougé
            if self.is_perched {
                let (min_p, max_p) = self.perch_x_bounds(px, pw);
                if let Some([old_px, old_py, _, _]) = old_perch {
                    let dx = px - old_px;
                    self.x += dx;
                    // Si la fenêtre est descendue brutalement (> 80px) ou que la mascotte déborde
                    if (py - old_py) > 80.0 || self.x < min_p - 25.0 || self.x > max_p + 25.0 {
                        self.is_perched = false;
                        self.state = PetState::Fall;
                    } else {
                        self.x = safe_clamp(self.x, min_p, max_p);
                        self.y = py;
                    }
                } else if self.x < min_p || self.x > max_p {
                    self.is_perched = false;
                    self.state = PetState::Fall;
                } else {
                    self.y = py;
                }
            }
        } else if self.is_perched {
            // Plus de perchoir -> tombe
            self.is_perched = false;
            self.state = PetState::Fall;
        }
    }

    /// Évite les écrans occupés par une fenêtre en plein écran en se déplaçant vers un moniteur libre
    pub fn evade_screens(&mut self, covered: &[[i32; 4]], free_screens: &[[i32; 4]]) {
        if covered.is_empty() || free_screens.is_empty() {
            return;
        }
        for cov in covered {
            let cx1 = cov[0] as f32;
            let cx2 = (cov[0] + cov[2]) as f32;
            if self.x >= cx1 && self.x <= cx2 {
                if self.is_perched {
                    self.is_perched = false;
                    self.state = PetState::Fall;
                }
                let mut closest_x = self.x;
                let mut min_dist = f32::MAX;
                for free in free_screens {
                    let center_x = free[0] as f32 + (free[2] as f32) / 2.0;
                    let d = (center_x - self.x).abs();
                    if d < min_dist {
                        min_dist = d;
                        closest_x = center_x;
                    }
                }
                self.target_x = Some(closest_x);
                self.target_hurry = Self::HURRY;
                self.then_state = None;
                self.state = PetState::Walk;
                self.facing = if closest_x > self.x { 1.0 } else { -1.0 };
                self.vx = self.facing * self.walk_speed() * Self::HURRY;
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pet::types::{MascotKind, PetState, PetWish};

    #[test]
    fn test_climb_does_not_abort_due_to_loss_of_ground() {
        let mut pet = Pet::new(MascotKind::Claude, 500.0, 500.0);
        pet.climb_onto(500.0, 200.0);
        assert_eq!(pet.state, PetState::Climb);
        pet.update(0.01, 1000.0, 500.0 + 26.0);
        assert_eq!(pet.state, PetState::Climb, "Climb ne doit pas être interrompu par la perte de sol");
    }


    #[test]
    fn test_perch_bounce() {
        let mut pet = Pet::new(MascotKind::Claude, 200.0, 300.0);
        pet.scale = 1.0;
        pet.is_perched = true;
        pet.current_perch = Some([100.0, 300.0, 200.0, 20.0]);
        pet.state = PetState::Fall;
        pet.y = 299.0;
        pet.vy = 400.0; // > 350.0 * scale
        pet.update(0.01, 1000.0, 800.0);

        assert_eq!(pet.state, PetState::Fall, "Mascotte doit rebondir sur le perchoir en état Fall");
        assert!(pet.is_perched, "Mascotte reste perchee lors du rebond");
        // vy apres gravite (400.0 + 2600.0 * 0.01 = 426.0) rebondit avec -0.5 -> -213.0
        assert_eq!(pet.vy, -213.0, "vy doit etre -landing_vy * 0.5");
    }


    #[test]
    fn test_perch_min_width_stature() {
        // Sous-agent (stature = 0.52) : seuil = 200 * 1.0 * 0.52 = 104.0 px
        let small_pet = Pet::new_small(
            "p1".to_string(),
            "s1".to_string(),
            "sub".to_string(),
            MascotKind::Claude,
            500.0,
            600.0,
            50.0,
            1.0,
        );
        let min_w = 200.0 * small_pet.scale * small_pet.stature;
        assert!((min_w - 104.0).abs() < 1e-3);
    }


    #[test]
    fn test_perch_preservation_on_fall() {
        let mut pet = Pet::new(MascotKind::Claude, 200.0, 300.0);
        pet.is_perched = true;
        let perch = Some([100.0, 300.0, 300.0, 20.0]);
        pet.current_perch = perch;
        pet.enter_fall();
        assert_eq!(pet.state, PetState::Fall);
        assert!(!pet.is_perched);
        assert_eq!(pet.current_perch, perch, "current_perch doit rester préservé");
    }


    #[test]
    fn test_perch_x_bounds() {
        let pet = Pet::new(MascotKind::Claude, 500.0, 500.0);
        let margin = pet.side_margin(); // 78.0
        // Perchoir normal (pw = 400.0 >= 2.0 * margin = 156.0)
        let (min_p, max_p) = pet.perch_x_bounds(100.0, 400.0);
        assert_eq!(min_p, 100.0 + margin);
        assert_eq!(max_p, 100.0 + 400.0 - margin);

        // Perchoir étroit (pw = 100.0 < 2.0 * margin = 156.0) -> centré
        let (center_min, center_max) = pet.perch_x_bounds(100.0, 100.0);
        assert_eq!(center_min, 150.0);
        assert_eq!(center_max, 150.0);
    }


    #[test]
    fn test_roam_perch_climb_and_descent() {
        let mut pet = Pet::new(MascotKind::Claude, 500.0, 500.0);
        pet.wish = PetWish::Roam;
        pet.current_perch = Some([400.0, 200.0, 300.0, 20.0]);
        let mut climbed = false;
        for _ in 0..50 {
            let mut p = pet.clone();
            p.update(0.01, 1000.0, 500.0 + 26.0);
            if p.state == PetState::Climb {
                climbed = true;
                break;
            }
        }
        assert!(climbed, "Pet with wish Roam must attempt climb on frame 0 with 20% chance");

        let mut perched_pet = Pet::new(MascotKind::Claude, 500.0, 200.0);
        perched_pet.wish = PetWish::Roam;
        perched_pet.is_perched = true;
        perched_pet.current_perch = Some([400.0, 200.0, 300.0, 20.0]);
        let mut descended = false;
        for _ in 0..100 {
            let mut p = perched_pet.clone();
            p.update(0.01, 1000.0, 500.0 + 26.0);
            if p.state == PetState::Fall {
                assert_eq!(p.vx, 0.0);
                assert_eq!(p.vy, 0.0);
                descended = true;
                break;
            }
        }
        assert!(descended, "Perched pet with wish Roam must descend with 10% chance on frame 0");
    }


    #[test]
    fn test_step_aside_reversal_and_perch() {
        let mut pet = Pet::new(MascotKind::Claude, 1850.0, 600.0);
        let _margin = pet.side_margin(); // 78.0
        let bounds = (0.0, 1920.0);
        // Autre mascotte à 1800.0, pet est à 1850.0 (side = +1)
        // target initial = 1800 + 170 = 1970 > max_x (1920 - 78 = 1842)
        // Doit inverser la direction : 1800 - 170 = 1630.0
        pet.step_aside(1800.0, 170.0, bounds);
        assert_eq!(pet.state, PetState::Walk);
        assert_eq!(pet.target_x, Some(1630.0));
        assert_eq!(pet.facing, -1.0);

        // Step aside sur perchoir
        let mut perched_pet = Pet::new(MascotKind::Claude, 680.0, 300.0);
        perched_pet.is_perched = true;
        perched_pet.current_perch = Some([500.0, 300.0, 250.0, 20.0]);
        // min_p = (500 + 78).max(78) = 578.0
        // max_p = (500 + 250 - 78).min(1842) = 672.0
        // Autre mascotte à 660.0, perched_pet à 680.0 (side = +1)
        // target initial = 660 + 50 = 710 > 672.0
        // Inversion : 660 - 50 = 610.0 (borné au perchoir)
        perched_pet.step_aside(660.0, 50.0, bounds);
        assert_eq!(perched_pet.state, PetState::Walk);
        assert_eq!(perched_pet.target_x, Some(610.0));
        assert!(perched_pet.is_perched);

        // Si rooted, ne bouge pas
        let mut rooted_pet = Pet::new(MascotKind::Claude, 500.0, 600.0);
        rooted_pet.rooted = true;
        rooted_pet.step_aside(480.0, 100.0, bounds);
        assert_eq!(rooted_pet.state, PetState::Idle);
        assert_eq!(rooted_pet.target_x, None);
    }


    #[test]
    fn test_think_perch_climb() {
        let mut pet = Pet::new(MascotKind::Claude, 500.0, 500.0);
        pet.wish = PetWish::Think;
        pet.current_perch = Some([400.0, 200.0, 300.0, 20.0]);
        // Tente plusieurs fois : avec 60% de chance, au moins une doit tenter l'escalade
        let mut climbed = false;
        for _ in 0..50 {
            let mut p = pet.clone();
            p.update(0.01, 1000.0, 500.0 + 26.0);
            if p.state == PetState::Climb {
                climbed = true;
                break;
            }
        }
        assert!(climbed, "Pet with wish Think must attempt to climb valid perch with 60% probability");

        // Sans perchoir valide (trop étroit) : reste au sol en Think
        let mut pet_no_perch = Pet::new(MascotKind::Claude, 500.0, 500.0);
        pet_no_perch.wish = PetWish::Think;
        pet_no_perch.current_perch = Some([400.0, 200.0, 50.0, 20.0]); // 50 < 200
        pet_no_perch.update(0.01, 1000.0, 500.0 + 26.0);
        assert_eq!(pet_no_perch.state, PetState::Think);
    }

}
