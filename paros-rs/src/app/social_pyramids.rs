use std::time::{Duration, Instant};

use crate::app::types::{
    SubagentPyramid, PYRAMID_BUILD_SECS, PYRAMID_CHANCE, PYRAMID_HEAD, PYRAMID_HOLD_SECS,
    PYRAMID_LEVEL, PYRAMID_STEP, TOWER_AFTER_SECS, TOWER_LEVEL, TOWER_WIDTH,
};
use crate::app::ParosApp;
use crate::pet::PetState;

impl ParosApp {
    /// Gestion de la tour de repos (Tower) pour les mascottes au repos depuis 90s
    pub fn update_tower(&mut self, now: Instant, standing_allowed: bool) {
        let mut tower_valid = standing_allowed && !self.tower.is_empty();

        if tower_valid {
            for (lvl, id) in self.tower.iter().enumerate() {
                let is_valid = if let Some(p) = self.pets.get(id) {
                    let is_resting = self.idle_since.get(id).is_some_and(|since| {
                        now.duration_since(*since) >= Duration::from_secs_f32(TOWER_AFTER_SECS)
                    });
                    let state_ok = if lvl == 0 {
                        p.state == PetState::Sit || p.state == PetState::Idle
                    } else {
                        p.state == PetState::Sit
                            || p.state == PetState::Idle
                            || p.state == PetState::Walk
                            || p.state == PetState::Climb
                    };
                    is_resting && !p.is_airborne() && state_ok
                } else {
                    false
                };

                if !is_valid {
                    tower_valid = false;
                    break;
                }
            }
        }

        if !tower_valid && !self.tower.is_empty() {
            for (level, id) in self.tower.iter().enumerate() {
                if let Some(p) = self.pets.get_mut(id) {
                    p.rooted = false;
                    p.set_perch(None);
                    p.is_perched = false;
                    if level > 0 {
                        p.state = PetState::Fall;
                        p.vy = 0.0;
                    } else if p.state == PetState::Sit {
                        p.state = PetState::Idle;
                    }
                }
            }
            self.tower.clear();
        }

        if standing_allowed {
            if self.tower.is_empty() {
                let mut resting_adults: Vec<String> = self
                    .pets
                    .iter()
                    .filter(|(id, p)| {
                        let is_resting = self.idle_since.get(*id).is_some_and(|since| {
                            now.duration_since(*since)
                                >= Duration::from_secs_f32(TOWER_AFTER_SECS)
                        });
                        is_resting
                            && !p.is_airborne()
                            && !p.is_perched
                            && !p.rooted
                            && (p.state == PetState::Idle || p.state == PetState::Sit)
                    })
                    .map(|(id, _)| id.clone())
                    .collect();

                if resting_adults.len() >= 2 {
                    let base_id = resting_adults.remove(0);
                    if let Some(base) = self.pets.get_mut(&base_id) {
                        base.rooted = true;
                        base.state = PetState::Sit;
                    }
                    self.tower.push(base_id);
                    for id in resting_adults {
                        self.tower.push(id);
                    }
                }
            }

            if !self.tower.is_empty() {
                let base_x = self.pets.get(&self.tower[0]).map(|p| p.x).unwrap_or(0.0);
                let base_y = self.pets.get(&self.tower[0]).map(|p| p.y).unwrap_or(0.0);

                for level in 1..self.tower.len() {
                    let id = &self.tower[level];
                    let pet_is_perched = self.pets.get(id).is_some_and(|p| p.is_perched);
                    if pet_is_perched
                        && let Some(p) = self.pets.get_mut(id) {
                            p.rooted = true;
                        }

                    let pet_rooted = self.pets.get(id).is_some_and(|p| p.rooted);
                    if pet_rooted {
                        continue;
                    }

                    let prev_id = &self.tower[level - 1];
                    let prev_ready = self
                        .pets
                        .get(prev_id)
                        .is_some_and(|p| p.rooted || p.is_perched);
                    if !prev_ready {
                        break;
                    }

                    if let Some(p) = self.pets.get_mut(id) {
                        let target_level_y =
                            base_y - (level as f32) * TOWER_LEVEL * p.scale;
                        if (p.x - base_x).abs() > 12.0 * p.scale {
                            if !p.is_airborne() && p.state != PetState::Climb {
                                p.walk_to(base_x, false);
                            }
                        } else if !p.is_airborne() && p.state != PetState::Climb {
                            p.rooted = true;
                            let head_w = TOWER_WIDTH * p.scale;
                            p.climb_onto(base_x, target_level_y);
                            p.current_perch = Some([base_x - head_w / 2.0, target_level_y, head_w, 0.0]);
                        }
                    }
                    break;
                }
            }
        }
    }

    /// Gestion des pyramides acrobatiques des sous-agents
    pub fn update_pyramids(&mut self, now: Instant, subagents_pyramid: bool) {
        if subagents_pyramid {
            let pyr_sessions: Vec<String> = self.pyramids.keys().cloned().collect();
            for session_id in pyr_sessions {
                let mut collapse = false;
                if let Some(pyr) = self.pyramids.get(&session_id) {
                    if now >= pyr.deadline {
                        collapse = true;
                    } else if let Some(until) = pyr.until
                        && now >= until {
                            collapse = true;
                        }
                    if !collapse {
                        for member_id in &pyr.members {
                            let member_pet = self
                                .small_pets
                                .iter()
                                .find(|s| s.parent_id == session_id && &s.id == member_id);
                            if let Some(pet) = member_pet {
                                if pet.dismissing || (pet.rooted && pet.is_airborne()) {
                                    collapse = true;
                                    break;
                                }
                            } else {
                                collapse = true;
                                break;
                            }
                        }
                    }
                }

                if collapse {
                    if let Some(pyr) = self.pyramids.remove(&session_id) {
                        for (idx, member_id) in pyr.members.iter().enumerate() {
                            let row = pyr.spots.get(idx).map_or(0, |s| s.1);
                            if let Some(pet) = self
                                .small_pets
                                .iter_mut()
                                .find(|s| s.parent_id == session_id && &s.id == member_id)
                            {
                                pet.rooted = false;
                                pet.is_perched = false;
                                pet.current_perch = None;
                                if row > 0 {
                                    pet.state = PetState::Fall;
                                    pet.vy = 0.0;
                                } else if pet.state == PetState::Sit {
                                    pet.state = PetState::Idle;
                                }
                            }
                        }
                    }
                    continue;
                }

                let (members, spots, middle, floor) = {
                    let pyr = &self.pyramids[&session_id];
                    (
                        pyr.members.clone(),
                        pyr.spots.clone(),
                        pyr.middle,
                        pyr.floor,
                    )
                };

                let mut settled = true;
                for i in 0..members.len() {
                    let member_id = &members[i];
                    let (spot_x_offset, row) = spots[i];

                    if row > 0 && !settled {
                        break;
                    }

                    let pet_info = self
                        .small_pets
                        .iter()
                        .find(|s| s.parent_id == session_id && &s.id == member_id)
                        .map(|s| (s.rooted, s.is_perched, s.state, s.scale * s.stature, s.x));

                    let (pet_rooted, pet_perched, pet_state, scale, pet_x) = match pet_info {
                        Some(info) => info,
                        None => continue,
                    };

                    if pet_rooted {
                        settled = settled && (row == 0 || pet_perched || pet_state != PetState::Climb);
                        continue;
                    }

                    settled = false;
                    let total_w = self.total_desktop_width();
                    let margin = scale * 30.0;
                    let target_x = (middle + spot_x_offset * PYRAMID_STEP * scale).clamp(margin, total_w - margin);
                    if (pet_x - target_x).abs() > 3.0 * scale {
                        if let Some(pet) = self
                            .small_pets
                            .iter_mut()
                            .find(|s| s.parent_id == session_id && &s.id == member_id)
                            && !pet.is_airborne() && pet.state != PetState::Climb {
                                pet.walk_to(target_x, true);
                            }
                    } else {
                        if let Some(pet) = self
                            .small_pets
                            .iter_mut()
                            .find(|s| s.parent_id == session_id && &s.id == member_id)
                            && !pet.is_airborne() {
                                pet.rooted = true;
                                if row == 0 {
                                    pet.state = PetState::Sit;
                                } else {
                                    let target_y =
                                        floor - (row as f32) * PYRAMID_LEVEL * scale;
                                    pet.climb_onto(target_x, target_y);
                                    let head_w = PYRAMID_HEAD * scale;
                                    pet.current_perch = Some([target_x - head_w / 2.0, target_y, head_w, 0.0]);
                                }
                            }
                    }
                    if row > 0 {
                        break;
                    }
                }

                if settled
                    && let Some(pyr) = self.pyramids.get_mut(&session_id)
                        && pyr.until.is_none() {
                            pyr.until = Some(now + Duration::from_secs_f32(PYRAMID_HOLD_SECS));
                            if let Some(top_id) = pyr.members.last()
                                && let Some(top_pet) = self
                                    .small_pets
                                    .iter_mut()
                                    .find(|s| s.parent_id == session_id && &s.id == top_id)
                                {
                                    top_pet.cheer();
                                }
                        }
            }

            // Démarrage de nouvelles pyramides
            let session_ids: Vec<String> = self.active_sessions.keys().cloned().collect();
            for session_id in session_ids {
                if !self.pyramids.contains_key(&session_id) {
                    let free_sub_ids: Vec<String> = self
                        .small_pets
                        .iter()
                        .filter(|s| {
                            s.parent_id == session_id
                                && s.is_free()
                                && !s.is_perched
                                && !s.is_airborne()
                        })
                        .map(|s| s.id.clone())
                        .collect();

                    if free_sub_ids.len() >= 2 && fastrand::f32() < PYRAMID_CHANCE {
                        let mut rows = 1;
                        while (rows + 1) * (rows + 2) / 2 <= free_sub_ids.len() {
                            rows += 1;
                        }

                        let mut spots: Vec<(f32, usize)> = Vec::new();
                        for r in 0..rows {
                            let count_in_row = rows - r;
                            for i in 0..count_in_row {
                                let offset =
                                    (i as f32) - ((count_in_row - 1) as f32) / 2.0;
                                spots.push((offset, r));
                            }
                        }
                        if rows == 1 {
                            spots.push((0.0, 1));
                        }

                        let member_count = spots.len().min(free_sub_ids.len());
                        let mut chosen_ids: Vec<String> =
                            free_sub_ids[..member_count].to_vec();

                        let bottom_count = rows.min(chosen_ids.len());
                        let mut bottom_pairs: Vec<(String, f32)> = chosen_ids[..bottom_count]
                            .iter()
                            .map(|id| {
                                let x = self
                                    .small_pets
                                    .iter()
                                    .find(|s| s.parent_id == session_id && &s.id == id)
                                    .map(|s| s.x)
                                    .unwrap_or(0.0);
                                (id.clone(), x)
                            })
                            .collect();
                        bottom_pairs.sort_by(|a, b| {
                            a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        for i in 0..bottom_count {
                            chosen_ids[i] = bottom_pairs[i].0.clone();
                        }

                        let first_pet = self
                            .small_pets
                            .iter()
                            .find(|s| s.parent_id == session_id && s.id == chosen_ids[0]);
                        if let Some(first) = first_pet {
                            let total_w = self.total_desktop_width();
                            let scale = first.scale * first.stature;
                            let margin = first.side_margin();
                            let half = ((rows - 1) as f32) / 2.0 * PYRAMID_STEP * scale;
                            let reach = (first.home_reach - half).max(0.0);
                            let min_middle = margin + half;
                            let max_middle = (total_w - margin - half).max(min_middle);
                            let middle_raw = bottom_pairs[bottom_count / 2].1;
                            let middle = middle_raw
                                .clamp(first.home_x - reach, first.home_x + reach)
                                .clamp(min_middle, max_middle);
                            let floor = first.y;

                            self.pyramids.insert(
                                session_id.clone(),
                                SubagentPyramid {
                                    members: chosen_ids,
                                    spots,
                                    middle,
                                    floor,
                                    deadline: now
                                        + Duration::from_secs_f32(PYRAMID_BUILD_SECS),
                                    until: None,
                                },
                            );
                        }
                    }
                }
            }
        } else if !self.pyramids.is_empty() {
            for (session_id, pyr) in self.pyramids.drain() {
                for (idx, member_id) in pyr.members.iter().enumerate() {
                    let row = pyr.spots.get(idx).map_or(0, |s| s.1);
                    if let Some(pet) = self
                        .small_pets
                        .iter_mut()
                        .find(|s| s.parent_id == session_id && &s.id == member_id)
                    {
                        pet.rooted = false;
                        pet.is_perched = false;
                        if row > 0 {
                            pet.state = PetState::Fall;
                            pet.vy = 0.0;
                        } else if pet.state == PetState::Sit {
                            pet.state = PetState::Idle;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pyramid_constants() {
        assert_eq!(PYRAMID_STEP, 160.0);
        assert_eq!(PYRAMID_LEVEL, 72.0);
        assert_eq!(PYRAMID_HEAD, 200.0);
        assert_eq!(PYRAMID_BUILD_SECS, 25.0);
        assert_eq!(PYRAMID_HOLD_SECS, 8.0);
        assert_eq!(TOWER_AFTER_SECS, 90.0);
    }

    #[test]
    fn test_pyramid_screen_bounds_clamping() {
        let total_w = 1920.0;
        let margin = 30.0;
        let rows = 3;
        let scale = 0.5;
        let half = ((rows - 1) as f32) / 2.0 * PYRAMID_STEP * scale; // 1.0 * 160.0 * 0.5 = 80.0
        let min_middle = margin + half; // 110.0
        let max_middle = total_w - margin - half; // 1810.0

        // Test proche du bord gauche (middle_raw = 20.0)
        let middle_raw = 20.0f32;
        let clamped = middle_raw.clamp(min_middle, max_middle);
        assert_eq!(clamped, 110.0);
        let left_spot = clamped - half;
        assert!(left_spot >= margin);

        // Test proche du bord droit (middle_raw = 1910.0)
        let middle_raw_right = 1910.0f32;
        let clamped_right = middle_raw_right.clamp(min_middle, max_middle);
        assert_eq!(clamped_right, 1810.0);
        let right_spot = clamped_right + half;
        assert!(right_spot <= total_w - margin);
    }
}
