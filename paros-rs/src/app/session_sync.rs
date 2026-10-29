use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::app::ParosApp;
use crate::letter::FlyingLetter;
use crate::pet::{MascotKind, Pet, PetState, PetWish};
use crate::session::{SessionPhase, SessionState};
use crate::settings::Settings;
use crate::system::SystemUpdate;

pub struct PetAnchorInfo {
    pub x: f32,
    pub ground: f32,
    pub scale: f32,
    pub kind: MascotKind,
    pub color: Option<(u8, u8, u8)>,
    pub headlamp: bool,
    pub grooving: bool,
}

impl ParosApp {
    /// Prise en charge du cycle de vie des mascottes Desktop Antigravity (création, mise à jour, timeout d'inactivité et sortie d'écran).
    pub fn handle_desktop_mascots(
        &mut self,
        total_w: f32,
        default_ground: f32,
        settings: &Settings,
        now: Instant,
    ) {
        let desktop_enabled = settings.agy.desktop;
        let desktop_minutes = settings.agy.desktop_minutes;
        let idle_timeout = Duration::from_secs_f32(desktop_minutes * 60.0);

        // Si désactivé dans les paramètres, toutes les mascottes desktop partent
        if !desktop_enabled {
            for (id, pet) in &mut self.pets {
                let is_desktop = pet.kind == MascotKind::Agy && self.session_watcher.desktop_meta.contains_key(id);
                if is_desktop && !pet.dismissing {
                    pet.walk_off(total_w, 1.0);
                }
            }
            return;
        }

        // Détection et synchronisation des conversations desktop depuis desktop_meta
        for (cid, meta) in &self.session_watcher.desktop_meta {
            if !meta.parent_id.is_empty() {
                continue;
            }

            if let Some(pet) = self.pets.get_mut(cid) {
                // Mascotte déjà présente : mise à jour
                if meta.not_idle {
                    // La session est active
                    if pet.dismissing {
                        pet.dismissing = false;
                        pet.state = PetState::Idle;
                    }
                    self.idle_since.remove(cid);
                } else {
                    // La session est inactive
                    let idle_start = *self.idle_since.entry(cid.clone()).or_insert(now);
                    let elapsed = now.duration_since(idle_start);
                    if elapsed >= idle_timeout && !pet.dismissing {
                        pet.walk_off(total_w, 1.0);
                    }
                }
            } else if meta.not_idle {
                // Nouvelle session desktop active : création immédiate
                let start_x = (total_w / 2.0).clamp(80.0, total_w - 80.0);
                let mut p = Pet::new(MascotKind::Agy, start_x, default_ground);
                p.id = cid.clone();
                p.label = if settings.show_name { meta.title.clone() } else { String::new() };
                p.scale = settings.pet_size;
                p.walk_speed_base = settings.walk_speed;
                p.wish = PetWish::Roam;
                self.pets.insert(cid.clone(), p);
                self.idle_since.remove(cid);
            }
        }

        // Pour les mascottes Agy existantes n'ayant plus d'activité
        for (id, pet) in &mut self.pets {
            let is_desktop = pet.kind == MascotKind::Agy && self.session_watcher.desktop_meta.contains_key(id);
            if is_desktop && !pet.dismissing {
                let is_working = self.active_sessions.get(id).is_some_and(|s| s.phase == SessionPhase::Working);
                let is_db_active = self.session_watcher.desktop_meta.get(id).is_some_and(|m| m.not_idle);
                if !is_working && !is_db_active {
                    let idle_start = *self.idle_since.entry(id.clone()).or_insert(now);
                    if now.duration_since(idle_start) >= idle_timeout {
                        pet.walk_off(total_w, 1.0);
                    }
                }
            }
        }
    }

    /// Synchronise les sous-agents d'une session.
    #[allow(clippy::too_many_arguments)]
    pub fn sync_subagents_for_session(
        &mut self,
        session: &SessionState,
        anchor: PetAnchorInfo,
        settings: &Settings,
    ) {
        if settings.subagents_show {
            let max_subs = settings.subagents_max;
            let count = session.subagents.len().min(max_subs).max(1) as f32;
            let reach = (140.0 + 30.0 * count) * anchor.scale;
            let active_sub_ids: HashSet<String> = session
                .subagents
                .iter()
                .take(max_subs)
                .map(|s| format!("{}/{}", session.id, s.id))
                .collect();

            for sub in session.subagents.iter().take(max_subs) {
                let caption = if settings.subagents_show_activity {
                    if !sub.detail.is_empty() {
                        format!("{} · {}", sub.tool, sub.detail)
                    } else {
                        sub.tool.clone()
                    }
                } else {
                    String::new()
                };

                let sub_key = format!("{}/{}", session.id, sub.id);
                if let Some(small) = self
                    .small_pets
                    .iter_mut()
                    .find(|s| format!("{}/{}", s.parent_id, s.id) == sub_key)
                {
                    small.label = sub.name.clone();
                    small.caption = caption;
                    small.home_x = anchor.x;
                    small.home_reach = reach;
                    small.kind = anchor.kind;
                    small.custom_color = anchor.color;
                    small.current_ground = anchor.ground;
                    small.stature = settings.subagents_size;
                    small.headlamp = anchor.headlamp;
                    small.grooving = anchor.grooving;
                    small.dismissing = false;
                } else {
                    let mut small = Pet::new_small(
                        session.id.clone(),
                        sub.id.clone(),
                        sub.name.clone(),
                        anchor.kind,
                        anchor.x,
                        anchor.ground,
                        reach,
                        anchor.scale,
                    );
                    small.custom_color = anchor.color;
                    small.stature = settings.subagents_size;
                    small.caption = caption;
                    small.current_ground = anchor.ground;
                    small.headlamp = anchor.headlamp;
                    small.grooving = anchor.grooving;
                    self.small_pets.push(small);
                }
            }

            for small in &mut self.small_pets {
                let sub_key = format!("{}/{}", small.parent_id, small.id);
                if small.parent_id == session.id
                    && !active_sub_ids.contains(&sub_key)
                    && !small.dismissing
                {
                    small.caption.clear();
                    small.sheet = false;
                    small.dismissing = true;
                    small.dismiss_going = false;
                    small.dismiss_timer = 6.0;
                    small.cheer_timer = 1.6;
                    small.cheer();
                }
            }
        } else {
            if let Some(pyr) = self.pyramids.remove(&session.id) {
                for (idx, member_id) in pyr.members.iter().enumerate() {
                    let row = pyr.spots.get(idx).map_or(0, |s| s.1);
                    if let Some(pet) = self
                        .small_pets
                        .iter_mut()
                        .find(|s| s.parent_id == session.id && &s.id == member_id)
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
            for small in &mut self.small_pets {
                if small.parent_id == session.id && !small.dismissing {
                    small.caption.clear();
                    small.sheet = false;
                    small.dismissing = true;
                    small.dismiss_going = false;
                    small.dismiss_timer = 6.0;
                    small.cheer_timer = 1.6;
                    small.cheer();
                }
            }
        }
    }

    /// Nettoie les sessions terminées et gère la sortie d'écran des mascottes.
    pub fn cleanup_finished_sessions(
        &mut self,
        active_ids: &HashSet<String>,
        total_w: f32,
        default_ground: f32,
        sys: &SystemUpdate,
        settings: &Settings,
        now: Instant,
    ) {
        let mut to_remove = Vec::new();

        for (id, pet) in &mut self.pets {
            if id == "none" {
                continue;
            }

            // Une mascotte qui a franchi les bords d'écran est prête à être purgée
            if pet.dismissing && pet.is_off(total_w) {
                to_remove.push(id.clone());
                continue;
            }

            // Si la session n'est plus active et n'est pas une conversation desktop préservée
            let is_desktop = pet.kind == MascotKind::Agy && self.session_watcher.desktop_meta.contains_key(id);
            if !active_ids.contains(id) && !is_desktop && !pet.dismissing {
                pet.walk_off(total_w, 1.0);
            }
        }

        for id in to_remove {
            self.pets.remove(&id);
            self.last_phases.remove(&id);
            self.idle_since.remove(&id);
            self.compacting_trackers.remove(&id);
            self.unfocused_since.remove(&id);
            self.succeeded_times.remove(&id);
            self.nagged_times.remove(&id);
            self.knocking_cooldowns.remove(&id);
            self.last_message_times.remove(&id);
            self.active_sessions.remove(&id);
            self.asked_peers.remove(&id);
            self.pyramids.remove(&id);
            if self.tower.contains(&id) {
                for (lvl, tid) in self.tower.iter().enumerate() {
                    if tid != &id
                        && let Some(p) = self.pets.get_mut(tid) {
                            p.rooted = false;
                            p.set_perch(None);
                            p.is_perched = false;
                            if lvl > 0 {
                                p.state = PetState::Fall;
                                p.vy = 0.0;
                            } else if p.state == PetState::Sit {
                                p.state = PetState::Idle;
                            }
                        }
                }
                self.tower.clear();
            }
            for letter in &mut self.flying_letters {
                if letter.target_id == id {
                    letter.stop();
                }
            }
            if let Some(tracker) = self.sage_trackers.get_mut(&id) {
                tracker.done = Some(now);
            }
            self.sages.remove(&id);
            self.small_pets.retain(|sp| sp.parent_id != id);
        }

        self.last_messages_sent.retain(|k, _| active_ids.contains(k));
        self.asked_peers.retain(|sender, target| {
            active_ids.contains(sender)
                && active_ids.contains(target)
                && self.active_sessions.get(target).is_some_and(|s| s.phase == SessionPhase::Working)
        });
        self.met_cooldowns.retain(|(p1, p2), t| {
            now.duration_since(*t).as_secs_f32() < crate::app::types::MEET_COOLDOWN
                && (self.pets.contains_key(p1) || self.small_pets.iter().any(|s| &s.id == p1))
                && (self.pets.contains_key(p2) || self.small_pets.iter().any(|s| &s.id == p2))
        });

        let has_any_pets = self.pets.iter().any(|(k, p)| k != "none" && !p.dismissing);
        if active_ids.is_empty() && !has_any_pets {
            let cx = total_w / 2.0;
            let feet_y = default_ground;
            let p = self.pets.entry("none".to_string()).or_insert_with(|| {
                let mut def = Pet::new(MascotKind::Claude, cx, feet_y);
                def.id = "none".to_string();
                def.custom_color = Some((217, 119, 87));
                def.label = String::new();
                def.accessory = 0;
                def.home_x = f32::NAN;
                def
            });
            p.scale = settings.pet_size;
            p.walk_speed_base = settings.walk_speed;
            p.pace = 0.7 + 0.9 * sys.load.clamp(0.0, 1.0);
            p.headlamp = sys.is_night && settings.headlamp;
            let wish = if self.cuddler_id.as_deref() == Some("none") || sys.is_night || (self.user_idle && !self.hyprland.is_locked()) {
                PetWish::Sleep
            } else {
                PetWish::Roam
            };
            p.wish = wish;
        }
    }

    /// Traite l'envoi effectif des lettres entre sessions.
    pub fn dispatch_letters(
        &mut self,
        letters_to_send: Vec<(String, String)>,
        now: Instant,
    ) {
        for (sender_id, dest_spec) in letters_to_send {
            if let Some(target_id) = self.resolve_session_named(&dest_spec, &sender_id) {
                let sender_pos = self
                    .pets
                    .get(&sender_id)
                    .map(|p| (p.x, p.current_ground, p.scale * p.stature, p.kind));
                let target_pos = self
                    .pets
                    .get(&target_id)
                    .map(|p| (p.x, p.current_ground, p.scale * p.stature));

                if let (Some((sx, sy, ss, _)), Some((tx, ty, ts))) = (sender_pos, target_pos) {
                    let side = if tx >= sx { 1.0 } else { -1.0 };
                    let seal_col = self
                        .pets
                        .get(&sender_id)
                        .map(FlyingLetter::seal_color_from_pet)
                        .unwrap_or((217, 119, 87, 255));
                    if let Some(p) = self.pets.get_mut(&sender_id) {
                        p.throw_letter(side);
                    }
                    self.last_message_times.insert(sender_id.clone(), now);
                    self.asked_peers.insert(sender_id.clone(), target_id.clone());
                    let letter = FlyingLetter::new_with_target(
                        sx, sy, ss, tx, ty, ts, seal_col, false, target_id,
                    );
                    self.flying_letters.push(letter);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_mascot_inactivity_lifecycle() {
        let mut pet = Pet::new(MascotKind::Agy, 960.0, 1000.0);
        let total_w = 1920.0;
        let desktop_minutes = 10.0f32;
        let idle_timeout = Duration::from_secs_f32(desktop_minutes * 60.0);

        let t0 = Instant::now();

        // 1. Inactif depuis 5 minutes (< 10 min) -> ne sort pas
        let t_5min = t0 + Duration::from_secs(300);
        let elapsed = t_5min.duration_since(t0);
        if elapsed >= idle_timeout && !pet.dismissing {
            pet.walk_off(total_w, 1.0);
        }
        assert!(!pet.dismissing);

        // 2. Inactif depuis 10 minutes -> déclenchement de la sortie
        let t_10min = t0 + Duration::from_secs(600);
        let elapsed = t_10min.duration_since(t0);
        if elapsed >= idle_timeout && !pet.dismissing {
            pet.walk_off(total_w, 1.0);
        }
        assert!(pet.dismissing);
        assert!(!pet.is_off(total_w), "Mascotte encore sur l'écran");

        // 3. Déplacement hors de l'écran -> is_off validé
        pet.x = total_w + 150.0;
        assert!(pet.is_off(total_w));

        // 4. Reprise d'activité avant sortie complète -> annulation du départ
        pet.x = 960.0;
        pet.dismissing = false;
        pet.state = PetState::Idle;
        assert!(!pet.dismissing);
        assert_eq!(pet.state, PetState::Idle);
    }
}
