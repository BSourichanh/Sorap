#![allow(clippy::too_many_arguments)]

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::app::types::{
    COMPACT_MAX_SECS, COMPACT_MIN_SECS, KNOCK_AFTER_SECS, KNOCK_REACH,
    KNOCK_REPEAT_SECS, SAGE_ENTRY, SAGE_GAP,
};
use crate::app::ParosApp;
use crate::card;
use crate::i18n::{tr, Lang};
use crate::letter::FlyingLetter;
use crate::monitor::MonitorSurface;
use crate::pet::{MascotKind, Pet, PetWish};
use crate::session::types::{SessionPhase, SessionState};
use crate::settings::Settings;

#[derive(Debug, Clone)]
pub struct CompactingTracker {
    pub since: Instant,
    pub done: Option<Instant>,
    pub auto: bool,
}

impl CompactingTracker {
    pub fn new(since: Instant, auto: bool) -> Self {
        Self {
            since,
            done: None,
            auto,
        }
    }

    /// Traite le compactage du contexte pour une session donnée.
    pub fn handle_session_compacting(
        trackers: &mut HashMap<String, CompactingTracker>,
        session: &SessionState,
        pet: &mut Pet,
        lang: Lang,
        bubble_seconds: f32,
        now: Instant,
    ) {
        if session.compacting {
            let is_auto = session.kind == MascotKind::Agy || session.context_percent >= 0.75;
            let tracker = trackers.entry(session.id.clone()).or_insert_with(|| {
                let text = if is_auto {
                    tr(lang, "Context full: compacting").to_string()
                } else {
                    tr(lang, "Compacting the context").to_string()
                };
                pet.say(text, bubble_seconds);
                CompactingTracker::new(now, is_auto)
            });
            tracker.done = None;
            if now.duration_since(tracker.since).as_secs_f32() >= COMPACT_MAX_SECS {
                trackers.remove(&session.id);
                pet.shake_off();
                pet.say(tr(lang, "Context compacted").to_string(), bubble_seconds);
                pet.strained = false;
            }
        } else if let Some(tracker) = trackers.get_mut(&session.id) {
            if tracker.done.is_none() {
                let min_over = tracker.since + Duration::from_secs_f32(COMPACT_MIN_SECS);
                tracker.done = Some(now.max(min_over));
            }
            let max_over = tracker.since + Duration::from_secs_f32(COMPACT_MAX_SECS);
            let done_time = tracker.done.unwrap_or(max_over).min(max_over);
            if now >= done_time {
                trackers.remove(&session.id);
                pet.shake_off();
                pet.say(tr(lang, "Context compacted").to_string(), bubble_seconds);
                pet.strained = false;
            }
        }
    }

    /// Met à jour l'état de repos continu et le souhait (Wish) de la mascotte.
    pub fn update_rest_and_wish(
        idle_since: &mut HashMap<String, Instant>,
        compacting_trackers: &HashMap<String, CompactingTracker>,
        cuddler_id: Option<&str>,
        user_idle: bool,
        is_locked: bool,
        session: &SessionState,
        pet: &mut Pet,
        awaits_peer: bool,
        is_night: bool,
        now: Instant,
    ) {
        let is_at_rest = session.phase == SessionPhase::Idle
            && !session.has_background
            && !awaits_peer
            && session.message_sent.is_none()
            && !session.compacting
            && !compacting_trackers.contains_key(&session.id);
        if is_at_rest {
            idle_since.entry(session.id.clone()).or_insert(now);
        } else {
            idle_since.remove(&session.id);
        }

        let wish = if cuddler_id == Some(&session.id) {
            PetWish::Sleep
        } else if session.phase == SessionPhase::Waiting {
            PetWish::Alert
        } else if compacting_trackers.contains_key(&session.id) {
            PetWish::Compact
        } else if session.phase == SessionPhase::Working {
            PetWish::Think
        } else if session.has_background || awaits_peer {
            PetWish::Wait
        } else if is_night || (user_idle && !is_locked) {
            PetWish::Sleep
        } else {
            PetWish::Roam
        };
        pet.wish = wish;
        pet.strained = compacting_trackers.get(&session.id).is_some_and(|t| t.auto);
    }

    /// Gère l'attente prolongée et le toquement au bord de l'écran.
    pub fn handle_waiting_and_knocking(
        unfocused_since: &mut HashMap<String, Instant>,
        knocking_cooldowns: &mut HashMap<String, Instant>,
        nagged_times: &mut HashMap<String, Instant>,
        monitors: &[MonitorSurface],
        last_pointer_pos: (f32, f32),
        session: &SessionState,
        pet: &mut Pet,
        active_win_pid: i32,
        total_w: f32,
        lang: Lang,
        settings: &Settings,
        now: Instant,
    ) {
        let is_focused = session.lineage.contains(&active_win_pid);
        if session.phase == SessionPhase::Waiting && !is_focused {
            let unfocused_t = unfocused_since
                .entry(session.id.clone())
                .or_insert(now);
            let unfocused_secs = now.duration_since(*unfocused_t).as_secs_f32();

            if settings.claude_knock && unfocused_secs >= KNOCK_AFTER_SECS {
                let last_knock = knocking_cooldowns
                    .get(&session.id)
                    .copied()
                    .unwrap_or(Instant::now() - Duration::from_secs(60));
                if now.duration_since(last_knock)
                    >= Duration::from_secs_f32(KNOCK_REPEAT_SECS)
                {
                    knocking_cooldowns.insert(session.id.clone(), now);
                    let px = if last_pointer_pos.0 > 0.0 {
                        last_pointer_pos.0
                    } else {
                        pet.x
                    };
                    let reach = KNOCK_REACH * pet.scale_factor();
                    let target_mon = monitors
                        .iter()
                        .find(|m| px >= m.x as f32 && px <= (m.x as f32 + m.width as f32))
                        .or_else(|| monitors.iter().find(|m| pet.x >= m.x as f32 && pet.x <= (m.x as f32 + m.width as f32)));

                    let (edge_x, side) = if let Some(mon) = target_mon {
                        let mon_left = mon.x as f32;
                        let mon_right = mon.x as f32 + mon.width as f32;
                        let knock_side = if px >= mon_left + (mon.width as f32) / 2.0 { 1.0 } else { -1.0 };
                        let edge_x = if knock_side > 0.0 { mon_right - reach } else { mon_left + reach };
                        (edge_x, knock_side)
                    } else if pet.x < total_w / 2.0 {
                        (reach, -1.0)
                    } else {
                        (total_w - reach, 1.0)
                    };
                    pet.knock_at(edge_x, side);
                }
            }

            let total_waited = now.duration_since(session.start_time).as_secs_f32();
            let nag_limit = settings.claude_nag_minutes * 60.0;
            if total_waited >= nag_limit {
                let last_nag = nagged_times
                    .get(&session.id)
                    .copied()
                    .unwrap_or(Instant::now() - Duration::from_secs(120));
                if now.duration_since(last_nag) >= Duration::from_secs(60) {
                    nagged_times.insert(session.id.clone(), now);
                    pet.urgent = true;
                    let dur_str = card::format_duration(total_waited as u64);
                    let msg = tr(lang, "Claude has been waiting for %s").replace("%s", &dur_str);
                    pet.say(msg, settings.bubble_seconds);
                }
            }
        } else {
            unfocused_since.remove(&session.id);
            pet.urgent = false;
        }
    }
}

impl ParosApp {
    /// Fait apparaître les conseillers Sages demandés et lance la lettre de requête.
    pub fn dispatch_sage_calls(&mut self, sages_to_call: Vec<String>, total_w: f32, now: Instant) {
        for session_id in sages_to_call {
            if !self.sages.contains_key(&session_id) {
                let pet_pos = self
                    .pets
                    .get(&session_id)
                    .map(|p| (p.x, p.current_ground, p.scale, p.stature));
                if let Some((px, p_ground, p_scale, p_stature)) = pet_pos {
                    let mon = self.monitors.iter().find(|m| {
                        px >= m.x as f32 && px < (m.x as f32 + m.width as f32)
                    });
                    let (mon_min_x, mon_max_x, mon_center_x) = if let Some(m) = mon {
                        (m.x as f32 + 60.0, m.x as f32 + m.width as f32 - 60.0, m.x as f32 + m.width as f32 / 2.0)
                    } else {
                        (60.0, total_w - 60.0, total_w / 2.0)
                    };
                    let side = if px > mon_center_x { -1.0 } else { 1.0 };
                    let size = p_scale;
                    let spawn_x = (px + side * SAGE_ENTRY * size).clamp(mon_min_x, mon_max_x);
                    let target_x = (px + side * SAGE_GAP * size).clamp(mon_min_x, mon_max_x);
                    let mut sage = Pet::new_sage(spawn_x, p_ground, target_x);
                    sage.current_ground = p_ground;
                    sage.wish = PetWish::Read;
                    self.sages.insert(session_id.clone(), sage);
                    self.sage_trackers.insert(
                        session_id.clone(),
                        crate::app::types::SageTracker {
                            since: now,
                            spawn_time: now,
                            done: None,
                            side,
                            leaving: false,
                            gone_time: None,
                        },
                    );
                    if let Some(pet) = self.pets.get_mut(&session_id) {
                        pet.throw_letter(side);
                        pet.activity_text.clear();
                    }
                    let seal_col = self
                        .pets
                        .get(&session_id)
                        .map(FlyingLetter::seal_color_from_pet)
                        .unwrap_or((217, 119, 87, 255));
                    let letter = FlyingLetter::new_with_target(
                        px,
                        p_ground,
                        p_scale * p_stature,
                        target_x,
                        p_ground,
                        1.5,
                        seal_col,
                        true,
                        session_id.clone(),
                    );
                    self.flying_letters.push(letter);
                }
            }
        }
    }

    /// Gère les disputes visuelles lors de collisions d'édition inter-sessions.
    pub fn handle_session_collisions(
        &mut self,
        collisions: Vec<(String, String, String)>,
        lang: Lang,
        settings: &Settings,
        now: Instant,
    ) {
        for (sid, other_name, filename) in collisions {
            let other_id_opt = self.resolve_session_named(&other_name, &sid);
            let side = if let Some(ref oid) = other_id_opt {
                if let Some(p) = self.pets.get(&sid) {
                    if let Some(op) = self.pets.get(oid) {
                        if op.x >= p.x { 1.0 } else { -1.0 }
                    } else {
                        1.0
                    }
                } else {
                    1.0
                }
            } else {
                1.0
            };
            if let Some(p) = self.pets.get_mut(&sid) {
                p.glare(side);
                p.dispute = Some((filename.clone(), now));
                let msg = tr(lang, "%s: also changed by %s")
                    .replacen("%s", &filename, 1)
                    .replacen("%s", &other_name, 1);
                p.say(msg, settings.bubble_seconds);
            }
            if let Some(ref oid) = other_id_opt
                && let Some(op) = self.pets.get_mut(oid) {
                    op.glare(-side);
                    op.dispute = Some((filename.clone(), now));
                }
            crate::sound::play_sound_with_settings(
                crate::sound::SoundEffect::Failure,
                settings.sound_enabled,
                settings.sound_volume,
            );
        }
    }
}
