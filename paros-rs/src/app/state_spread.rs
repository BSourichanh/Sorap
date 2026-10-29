use std::collections::HashMap;
use std::time::Instant;

use crate::app::ParosApp;
use crate::pet::{Pet, PetState, PetWish};

impl ParosApp {
    pub fn apply_spread(&mut self, desktop_bounds: (f32, f32)) {
        let now = Instant::now();
        let mut crowded = HashMap::new();

        let mut ids: Vec<String> = self.pets.keys().cloned().collect();
        for s in self.sages.keys() {
            ids.push(format!("sage:{s}"));
        }
        for sp in &self.small_pets {
            ids.push(format!("{}/{}", sp.parent_id, sp.id));
        }

        let settled_states = [
            PetState::Idle,
            PetState::Sit,
            PetState::Sleep,
            PetState::Think,
            PetState::Alert,
            PetState::Wait,
        ];

        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let id_a = &ids[i];
                let id_b = &ids[j];

                let (feet_a, rooted_a, state_a, stat_a, is_small_a, wish_a) = match self.get_pet_info(id_a) {
                    Some(info) => info,
                    None => continue,
                };
                let (feet_b, rooted_b, state_b, stat_b, is_small_b, wish_b) = match self.get_pet_info(id_b) {
                    Some(info) => info,
                    None => continue,
                };

                if rooted_a || rooted_b {
                    continue;
                }
                if !settled_states.contains(&state_a) || !settled_states.contains(&state_b) {
                    continue;
                }

                let apart_x = (feet_b.0 - feet_a.0).abs();
                let apart_y = (feet_b.1 - feet_a.1).abs();
                let room_for = |scale_factor: f32, is_small: bool| -> f32 {
                    let mut room = scale_factor;
                    if is_small {
                        room *= 1.6;
                    }
                    room
                };
                let size = (room_for(stat_a, is_small_a) + room_for(stat_b, is_small_b)) / 2.0;

                if apart_x >= 120.0 * size || apart_y >= 120.0 * size / 2.0 {
                    continue;
                }

                let pair_key = if id_a < id_b {
                    (id_a.clone(), id_b.clone())
                } else {
                    (id_b.clone(), id_a.clone())
                };

                let since = self.crowded_since.get(&pair_key).copied().unwrap_or(now);
                crowded.insert(pair_key, since);

                let is_small = is_small_a || is_small_b;
                let thresh = if is_small { 0.6 } else { 2.0 };

                if (now - since).as_secs_f32() >= thresh {
                    let b_moves = if stat_a != stat_b {
                        stat_b < stat_a
                    } else {
                        wish_priority(wish_b) < wish_priority(wish_a)
                    };

                    let gap = 170.0 * size;
                    if b_moves {
                        if let Some(pb) = self.get_pet_mut(id_b) {
                            pb.step_aside(feet_a.0, gap, desktop_bounds);
                        }
                    } else {
                        if let Some(pa) = self.get_pet_mut(id_a) {
                            pa.step_aside(feet_b.0, gap, desktop_bounds);
                        }
                    }
                }
            }
        }

        self.crowded_since = crowded;
    }

    #[allow(clippy::type_complexity)]
    pub fn get_pet_info(&self, id: &str) -> Option<((f32, f32), bool, PetState, f32, bool, PetWish)> {
        if let Some(stripped) = id.strip_prefix("sage:") {
            return self.sages.get(stripped).map(|p| ((p.x, p.y), p.rooted, p.state, p.scale * p.stature, p.is_small || p.stature < 1.0, p.wish));
        }
        if let Some(p) = self.pets.get(id) {
            return Some(((p.x, p.y), p.rooted, p.state, p.scale * p.stature, p.is_small || p.stature < 1.0, p.wish));
        }
        if let Some(p) = self.sages.get(id) {
            return Some(((p.x, p.y), p.rooted, p.state, p.scale * p.stature, p.is_small || p.stature < 1.0, p.wish));
        }
        if let Some(p) = self.small_pets.iter().find(|p| format!("{}/{}", p.parent_id, p.id) == id || p.id == id) {
            return Some(((p.x, p.y), p.rooted, p.state, p.scale * p.stature, p.is_small || p.stature < 1.0, p.wish));
        }
        None
    }

    pub fn get_pet_mut(&mut self, id: &str) -> Option<&mut Pet> {
        if let Some(stripped) = id.strip_prefix("sage:") {
            return self.sages.get_mut(stripped);
        }
        if let Some(p) = self.pets.get_mut(id) {
            return Some(p);
        }
        if let Some(p) = self.sages.get_mut(id) {
            return Some(p);
        }
        if let Some(p) = self.small_pets.iter_mut().find(|p| format!("{}/{}", p.parent_id, p.id) == id || p.id == id) {
            return Some(p);
        }
        None
    }
}

pub fn wish_priority(w: PetWish) -> u8 {
    match w {
        PetWish::Roam => 0,
        PetWish::Wait => 1,
        PetWish::Think => 2,
        PetWish::Alert => 3,
        PetWish::Sleep => 4,
        PetWish::Compact => 5,
        PetWish::Read => 6,
    }
}
