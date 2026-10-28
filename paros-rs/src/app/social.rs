use std::time::{Duration, Instant};

use crate::app::types::{
    CUDDLE_GAP, CUDDLE_IDLE_SECS, HIGH_FIVE_DISTANCE, HIGH_FIVE_SECS, MEET_COOLDOWN, MEET_DISTANCE,
    SAGE_HURRY, SAGE_LEAVE_SECS, VISIT_GAP,
};
use crate::app::ParosApp;
use crate::letter::FlyingLetter;
use crate::pet::{PetState, PetWish};

impl ParosApp {
    pub fn update_social(
        &mut self,
        now: Instant,
        total_w: f32,
        cursor_global: Option<(f32, f32)>,
    ) {
        let settings = self.current_settings().clone();
        let sys = self.current_system;

        // 6. Gestion du départ des Sages
        let mut sages_to_remove = Vec::new();
        for (id, tracker) in &mut self.sage_trackers {
            if tracker.done.is_none()
                && now.duration_since(tracker.spawn_time) >= Duration::from_secs(900)
            {
                tracker.done = Some(now);
            }

            if let Some(done_time) = tracker.done
                && now >= done_time && !tracker.leaving {
                    tracker.leaving = true;
                    tracker.gone_time = Some(now + Duration::from_secs_f32(SAGE_LEAVE_SECS));
                    if let Some(sage) = self.sages.get_mut(id) {
                        if let Some(pet) = self.pets.get(id) {
                            let side = if pet.x > sage.x { 1.0 } else { -1.0 };
                            sage.throw_letter(side);
                            let seal_col = FlyingLetter::seal_color_from_pet(sage);
                            let letter = FlyingLetter::new_with_target(
                                sage.x,
                                sage.y,
                                sage.scale * sage.stature,
                                pet.x,
                                pet.y,
                                pet.scale * pet.stature,
                                seal_col,
                                false,
                                id.clone(),
                            );
                            self.flying_letters.push(letter);
                        }
                        let (mon_left, mon_right) = self
                            .monitors
                            .iter()
                            .find(|m| sage.x >= m.x as f32 && sage.x < (m.x + m.width as i32) as f32)
                            .map(|m| (m.x as f32, (m.x + m.width as i32) as f32))
                            .unwrap_or((0.0, total_w));
                        let exit_x = if (sage.x - mon_left) < (mon_right - sage.x) {
                            mon_left - 300.0
                        } else {
                            mon_right + 300.0
                        };
                        sage.target_x = Some(exit_x);
                        sage.state = PetState::Walk;
                        sage.pace = SAGE_HURRY;
                    }
                }

            if tracker.leaving
                && let Some(sage) = self.sages.get(id) {
                    let (mon_left, mon_right) = self
                        .monitors
                        .iter()
                        .find(|m| sage.x >= (m.x as f32 - 150.0) && sage.x <= ((m.x + m.width as i32) as f32 + 150.0))
                        .map(|m| (m.x as f32, (m.x + m.width as i32) as f32))
                        .unwrap_or((0.0, total_w));
                    let is_off = sage.x < mon_left - 150.0 || sage.x > mon_right + 150.0;
                    let expired = tracker.gone_time.is_some_and(|gt| now >= gt);
                    if is_off || expired {
                        sages_to_remove.push(id.clone());
                    }
                }
        }
        for id in sages_to_remove {
            self.sages.remove(&id);
            self.sage_trackers.remove(&id);
            if let Some(p) = self.pets.get_mut(&id) {
                p.activity_text.clear();
            }
        }

        // 7. Salutations ou rivalité entre mascottes adultes qui se croisent (Meet < 150 px, cooldown 45s)
        if settings.greetings {
            let pet_keys: Vec<String> = self.pets.keys().cloned().collect();
            for i in 0..pet_keys.len() {
                for j in (i + 1)..pet_keys.len() {
                    let k1 = &pet_keys[i];
                    let k2 = &pet_keys[j];

                    let pair_key = if k1 < k2 {
                        (k1.clone(), k2.clone())
                    } else {
                        (k2.clone(), k1.clone())
                    };

                    let can_greet = self.met_cooldowns.get(&pair_key).is_none_or(|t| {
                        now.duration_since(*t) >= Duration::from_secs_f32(MEET_COOLDOWN)
                    });
                    if can_greet {
                        let action = {
                            let (Some(p1), Some(p2)) = (self.pets.get(k1), self.pets.get(k2)) else {
                                continue;
                            };
                            if (p1.x - p2.x).abs() > MEET_DISTANCE * p1.scale {
                                None
                            } else {
                                let p1_free = !p1.is_airborne() && !p1.is_perched && p1.wish == PetWish::Roam && p1.is_free();
                                let p2_free = !p2.is_airborne() && !p2.is_perched && p2.wish == PetWish::Roam && p2.is_free();
                                if p1_free && p2_free {
                                    let dx = p2.x - p1.x;
                                    let dy = p2.y - p1.y;
                                    let dist = (dx * dx + dy * dy).sqrt();
                                    if dist <= MEET_DISTANCE * p1.scale {
                                        let side = if dx >= 0.0 { 1.0 } else { -1.0 };
                                        let is_rival = match (&p1.dispute, &p2.dispute) {
                                            (Some((f1, t1)), Some((f2, t2))) => {
                                                f1 == f2
                                                    && now.duration_since(*t1).as_secs_f32() < 600.0
                                                    && now.duration_since(*t2).as_secs_f32() < 600.0
                                            }
                                            _ => false,
                                        };
                                        Some((side, is_rival))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            }
                        };

                        if let Some((side, is_rival)) = action {
                            self.met_cooldowns.insert(pair_key, now);
                            if is_rival {
                                if let Some(p) = self.pets.get_mut(k1) {
                                    p.glare(side);
                                }
                                if let Some(p) = self.pets.get_mut(k2) {
                                    p.glare(-side);
                                }
                            } else {
                                if let Some(p) = self.pets.get_mut(k1) {
                                    p.greet(side);
                                }
                                if let Some(p) = self.pets.get_mut(k2) {
                                    p.greet(-side);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 8. Envoi d'un câlineur (Cuddler) vers le pointeur immobile (60s seul)
        let pointer_still =
            now.duration_since(self.last_pointer_move) >= Duration::from_secs_f32(CUDDLE_IDLE_SECS);
        if settings.desktop_cuddle && pointer_still && self.cuddler_id.is_none()
            && let Some(cpos) = cursor_global {
                let mut closest_id = None;
                let mut min_d = f32::MAX;
                for (id, p) in &self.pets {
                    if !p.is_airborne()
                        && !p.is_perched
                        && !p.rooted
                        && !p.dismissing
                        && p.target_x.is_none()
                        && p.wish == PetWish::Roam
                        && p.is_free()
                    {
                        let d = (p.x - cpos.0).abs();
                        if d < min_d {
                            min_d = d;
                            closest_id = Some(id.clone());
                        }
                    }
                }
                if let Some(id) = closest_id {
                    self.cuddler_id = Some(id.clone());
                    if let Some(p) = self.pets.get_mut(&id) {
                        let side = if p.x >= cpos.0 { 1.0 } else { -1.0 };
                        let target_x =
                            (cpos.0 + side * CUDDLE_GAP * p.scale).clamp(60.0, total_w - 60.0);
                        p.walk_to_then(target_x, PetState::Sleep, false);
                    }
                }
            }

        // 9. Tour de repos (Tower) pour les mascottes au repos depuis 90s
        let standing_allowed = settings.tower && !sys.is_night && !self.user_idle;
        self.update_tower(now, standing_allowed);

        // 10. Pyramides et jeux des sous-agents
        if now - self.last_subagent_tick >= Duration::from_millis(250) {
            self.last_subagent_tick = now;

            // A. Pyramides acrobatiques
            self.update_pyramids(now, settings.subagents_pyramid);

            // B. Jeux des sous-agents (visites amicales et sauts)
            if settings.subagents_games {
                let parent_ids: Vec<String> = self.active_sessions.keys().cloned().collect();
                for pid in parent_ids {
                    let group_free: Vec<usize> = self
                        .small_pets
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| {
                            s.parent_id == pid
                                && s.is_free()
                                && !s.is_perched
                                && !s.is_airborne()
                        })
                        .map(|(i, _)| i)
                        .collect();

                    for &idx in &group_free {
                        let roll = fastrand::f32();
                        if roll < 0.04 {
                            self.small_pets[idx].jump();
                        } else if roll < 0.04 + 0.015 && group_free.len() >= 2 {
                            let others: Vec<usize> =
                                group_free.iter().copied().filter(|&i| i != idx).collect();
                            if !others.is_empty() {
                                let idx_b = others[fastrand::usize(..others.len())];
                                let b_x = self.small_pets[idx_b].x;
                                let a = &mut self.small_pets[idx];
                                let scale = a.scale * a.stature;
                                let side = if a.x < b_x { -1.0 } else { 1.0 };
                                let target = (b_x + side * VISIT_GAP * scale)
                                    .clamp(a.home_x - a.home_reach, a.home_x + a.home_reach);
                                a.walk_to_then(target, PetState::Greet, true);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn celebrate(&mut self, pet_id: &str, now: Instant) {
        let (pet_x, pet_y, pet_scale) = match self.pets.get(pet_id) {
            Some(p) => (p.x, p.y, p.scale),
            None => return,
        };

        let mut partner = None;
        for (other_id, succ_time) in &self.succeeded_times {
            if other_id != pet_id
                && now.duration_since(*succ_time) <= Duration::from_secs_f32(HIGH_FIVE_SECS)
                && let Some(other) = self.pets.get(other_id) {
                    // Vérifier que le partenaire peut célébrer (non aéroporté, non grimpant)
                    if other.is_airborne() || other.state == PetState::Climb {
                        continue;
                    }
                    let dx = other.x - pet_x;
                    let dy = other.y - pet_y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist <= HIGH_FIVE_DISTANCE * pet_scale {
                        partner = Some((other_id.clone(), dx));
                        break;
                    }
                }
        }

        if let Some((other_id, dx)) = partner {
            let side = if dx >= 0.0 { 1.0 } else { -1.0 };
            let p_can = self.pets.get(pet_id).is_some_and(|p| !p.is_airborne() && p.state != PetState::Climb);
            let op_can = self.pets.get(&other_id).is_some_and(|op| !op.is_airborne() && op.state != PetState::Climb);
            if p_can && op_can {
                if let Some(p) = self.pets.get_mut(pet_id) {
                    p.high_five(side);
                }
                if let Some(op) = self.pets.get_mut(&other_id) {
                    op.high_five(-side);
                }
                self.succeeded_times.remove(&other_id);
            } else {
                self.succeeded_times.insert(pet_id.to_string(), now);
            }
        } else {
            self.succeeded_times.insert(pet_id.to_string(), now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pet::{MascotKind, Pet};

    #[test]
    fn test_subagent_eligibility_for_pyramids_and_games() {
        let mut p_idle = Pet::new_small("sess1".into(), "sub1".into(), "bash".into(), MascotKind::Claude, 100.0, 500.0, 100.0, 1.0);
        p_idle.target_x = None;
        p_idle.state = PetState::Idle;

        let mut p_sit = Pet::new_small("sess1".into(), "sub2".into(), "bash".into(), MascotKind::Claude, 110.0, 500.0, 100.0, 1.0);
        p_sit.target_x = None;
        p_sit.state = PetState::Sit;

        let mut p_walk_free = Pet::new_small("sess1".into(), "sub3".into(), "bash".into(), MascotKind::Claude, 120.0, 500.0, 100.0, 1.0);
        p_walk_free.target_x = None;
        p_walk_free.state = PetState::Walk;

        let mut p_walk_directed = Pet::new_small("sess1".into(), "sub4".into(), "bash".into(), MascotKind::Claude, 130.0, 500.0, 100.0, 1.0);
        p_walk_directed.target_x = Some(200.0);
        p_walk_directed.state = PetState::Walk;

        let mut p_rooted = Pet::new_small("sess1".into(), "sub5".into(), "bash".into(), MascotKind::Claude, 140.0, 500.0, 100.0, 1.0);
        p_rooted.target_x = None;
        p_rooted.rooted = true;

        let mut p_perched = Pet::new_small("sess1".into(), "sub6".into(), "bash".into(), MascotKind::Claude, 150.0, 500.0, 100.0, 1.0);
        p_perched.target_x = None;
        p_perched.is_perched = true;

        let pets = vec![p_idle, p_sit, p_walk_free, p_walk_directed, p_rooted, p_perched];
        let free_ids: Vec<String> = pets
            .iter()
            .filter(|s| s.parent_id == "sess1" && s.is_free() && !s.is_perched && !s.is_airborne())
            .map(|s| s.id.clone())
            .collect();

        assert_eq!(free_ids, vec!["sub1", "sub2", "sub3"]);
    }

    #[test]
    fn test_high_five_reciprocity_and_spatial_pruning() {
        let p1 = Pet::new(MascotKind::Claude, 100.0, 500.0);
        let mut p2 = Pet::new(MascotKind::Claude, 150.0, 500.0);
        p2.state = PetState::Fall;

        let p1_can = !p1.is_airborne() && p1.state != PetState::Climb;
        let p2_can = !p2.is_airborne() && p2.state != PetState::Climb;
        assert!(p1_can);
        assert!(!p2_can, "Un pet en Fall ne doit pas valider le High-Five");

        // Élagage spatial 1D
        let near_dx = (p1.x - p2.x).abs();
        assert!(near_dx <= MEET_DISTANCE * p1.scale);
        let far_x = 400.0;
        assert!((p1.x - far_x).abs() > MEET_DISTANCE * p1.scale);
    }
}

