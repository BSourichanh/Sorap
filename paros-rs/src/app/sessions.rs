use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::app::compacting::CompactingTracker;
use crate::app::types::{hash_str, SAGE_GAP};
use crate::app::ParosApp;
use crate::i18n::{detect_lang, tr};
use crate::letter::FlyingLetter;
use crate::pet::{MascotKind, Pet, PetState, PetWish};
use crate::session::SessionPhase;
use crate::sound::{play_sound_with_settings, SoundEffect};

impl ParosApp {
    pub fn poll_sessions(&mut self, now: Instant) {
        let settings = self.current_settings().clone();
        let lang = detect_lang(&settings.language);
        let sys = self.current_system;
        let total_w = self.total_desktop_width();
        let total_h = self.total_desktop_height();
        let default_ground = total_h - 30.0;

        let sessions = self.session_watcher.poll_all_with_settings(
            settings.git_enabled,
            settings.claude_context_window_k,
            settings.groove,
        );
        let active_ids: HashSet<String> = sessions.iter().map(|s| s.id.clone()).collect();
        if !active_ids.is_empty() {
            self.pets.remove("none");
        }
        let active_window = self.hyprland.active_window();
        let active_win_pid = active_window.as_ref().map(|w| w.pid).unwrap_or(-1);

        let mut letters_to_send: Vec<(String, String)> = Vec::new();
        let mut sages_to_call: Vec<String> = Vec::new();
        let mut celebrations_to_trigger: Vec<String> = Vec::new();
        let mut collisions_to_handle: Vec<(String, String, String)> = Vec::new();

        for (idx, session) in sessions.iter().enumerate() {
            let prev_phase = self.last_phases.get(&session.id).copied().unwrap_or(SessionPhase::Idle);

            // A. Transitions de phase
            if prev_phase == SessionPhase::Working && session.phase == SessionPhase::Idle {
                let for_user = !session.turn_origin_peer;
                let has_bg = session.has_background;
                if has_bg {
                    if for_user {
                        self.owed_celebrations.insert(session.id.clone());
                    }
                } else if for_user {
                    play_sound_with_settings(SoundEffect::Success, settings.sound_enabled, settings.sound_volume);
                    if let Some(pet) = self.pets.get_mut(&session.id) {
                        pet.cheer();
                        pet.say(tr(lang, "Task done!").to_string(), settings.bubble_seconds);
                    }
                    celebrations_to_trigger.push(session.id.clone());
                } else if let Some(pet) = self.pets.get_mut(&session.id) {
                    pet.nod();
                }
            }
            self.last_phases.insert(session.id.clone(), session.phase);

            // B. Création ou récupération de la mascotte adulte
            let pet = self.pets.entry(session.id.clone()).or_insert_with(|| {
                let spacing = 220.0;
                let start_x = 450.0 + (idx as f32 * spacing);
                let initial_x = start_x.clamp(80.0, total_w - 80.0);
                let mut p = Pet::new(session.kind, initial_x, default_ground);
                p.id = session.id.clone();
                p
            });

            // Notification d'attente immédiate (session_needs_you)
            if session.phase == SessionPhase::Waiting && prev_phase != SessionPhase::Waiting
                && pet.say_text.is_none() {
                    let template = tr(lang, "%s is waiting for you");
                    let msg = if !session.detail.is_empty() {
                        session.detail.clone()
                    } else {
                        let name = if !pet.label.is_empty() {
                            &pet.label
                        } else if session.kind == MascotKind::Agy {
                            "Antigravity"
                        } else {
                            "Claude"
                        };
                        template.replace("%s", name)
                    };
                    pet.say(msg, settings.bubble_seconds);
                }

            // Célébration différée lorsque la tâche de fond est terminée
            if self.owed_celebrations.contains(&session.id)
                && !session.has_background && session.message_sent.is_none() {
                    self.owed_celebrations.remove(&session.id);
                    pet.cheer();
                    pet.say(tr(lang, "Task done!").to_string(), settings.bubble_seconds);
                    play_sound_with_settings(SoundEffect::Success, settings.sound_enabled, settings.sound_volume);
                    celebrations_to_trigger.push(session.id.clone());
                }

            pet.kind = session.kind;
            pet.id = session.id.clone();
            pet.custom_color = session.custom_color;
            pet.scale = settings.pet_size;
            pet.walk_speed_base = settings.walk_speed;
            pet.pace = 0.7 + 0.9 * sys.load.clamp(0.0, 1.0);

            pet.label = if !settings.show_name {
                String::new()
            } else if session.branch.is_empty() {
                session.name.clone()
            } else {
                format!("{} · {}", session.name, session.branch)
            };

            pet.accessory = if !settings.accessories || session.name.is_empty() {
                0
            } else {
                hash_str(&session.name) % 7
            };

            if session.phase == SessionPhase::Working {
                self.asked_peers.remove(&session.id);
            }
            let awaits_peer = session.phase == SessionPhase::Idle
                && self.asked_peers.get(&session.id).is_some_and(|target_name| {
                    sessions.iter().any(|s| {
                        (&s.id == target_name || &s.name == target_name)
                            && s.phase == SessionPhase::Working
                    })
                });

            pet.hard_hat = session.git_conflict;
            pet.baggage = if session.dirty_lines >= 300 { 3 } else if session.dirty_lines >= 50 { 2 } else if session.dirty_lines >= 1 { 1 } else { 0 };
            pet.lost = session.commits_behind > 0;
            pet.serving = session.servers > 0;
            pet.tapping = session.quiet_level == 1;
            pet.meditating = session.quiet_level >= 2;
            pet.fullness = session.context_percent;
            pet.smoking = pet.fullness >= 0.75;
            pet.hourglass = session.has_background || awaits_peer;
            pet.headlamp = sys.is_night && settings.headlamp;
            pet.grooving = session.is_music_playing && settings.groove;

            if session.phase == SessionPhase::Working && settings.claude_show_activity {
                if session.advising {
                    pet.caption = tr(lang, "Asks the sage").to_string();
                } else if !session.activity.is_empty() {
                    pet.caption = if !session.detail.is_empty() {
                        format!("{} · {}", session.activity, session.detail)
                    } else {
                        session.activity.clone()
                    };
                } else {
                    pet.caption.clear();
                }
            } else {
                pet.caption.clear();
            }
            let inbound = self.flying_letters.iter().filter(|l| l.target_id == session.id && !l.target_is_sage).count();
            let effective_mail = session.mail.saturating_sub(inbound);
            if effective_mail < pet.mail && inbound == 0 && !pet.is_airborne() && pet.state != PetState::Climb {
                pet.read_letter();
            }
            pet.mail = effective_mail;

            // C. Suivi du repos continu & Wish
            CompactingTracker::update_rest_and_wish(
                &mut self.idle_since,
                &self.compacting_trackers,
                self.cuddler_id.as_deref(),
                self.user_idle,
                self.hyprland.is_locked(),
                session,
                pet,
                awaits_peer,
                sys.is_night,
                now,
            );

            // D. Événements spéciaux
            if session.repo_cleaned {
                pet.sweep();
                pet.cool_timer = 60.0;
            }

            if session.tests_passed {
                pet.cheer();
                pet.say(tr(lang, "Green tests!").to_string(), settings.bubble_seconds);
                play_sound_with_settings(
                    SoundEffect::Success,
                    settings.sound_enabled,
                    settings.sound_volume,
                );
                celebrations_to_trigger.push(session.id.clone());
            }

            if let Some((sub_agent_opt, is_test)) = &session.tool_failed {
                if let Some(sub_id) = sub_agent_opt {
                    let sub_key = format!("{}/{}", session.id, sub_id);
                    if let Some(small) = self
                        .small_pets
                        .iter_mut()
                        .find(|s| format!("{}/{}", s.parent_id, s.id) == sub_key)
                    {
                        small.worry();
                    }
                } else {
                    pet.worry();
                    if *is_test {
                        pet.say(tr(lang, "Red tests").to_string(), settings.bubble_seconds);
                    }
                }
                if sub_agent_opt.is_none() {
                    play_sound_with_settings(SoundEffect::Failure, settings.sound_enabled, settings.sound_volume);
                }
            }

            if let Some((other_name, filename)) = &session.collision {
                collisions_to_handle.push((session.id.clone(), other_name.clone(), filename.clone()));
            }

            // E. Compactage du contexte (sécurité max 900s conforme à brain.gd:75)
            CompactingTracker::handle_session_compacting(
                &mut self.compacting_trackers,
                session,
                pet,
                lang,
                settings.bubble_seconds,
                now,
            );

            // F. Toquement au bord de l'écran en attente
            CompactingTracker::handle_waiting_and_knocking(
                &mut self.unfocused_since,
                &mut self.knocking_cooldowns,
                &mut self.nagged_times,
                &self.monitors,
                self.last_pointer_pos,
                session,
                pet,
                active_win_pid,
                total_w,
                lang,
                &settings,
                now,
            );

            // G. Envoi de lettres inter-sessions (mémorisation pour envoi)
            if let Some(ref dest) = session.message_sent {
                let last = self.last_messages_sent.get(&session.id);
                if last != Some(dest) {
                    self.last_messages_sent
                        .insert(session.id.clone(), dest.clone());
                    letters_to_send.push((session.id.clone(), dest.clone()));
                }
            } else {
                self.last_messages_sent.remove(&session.id);
            }

            // H. Conseiller Sage
            if session.advising && settings.claude_sage {
                if let Some(tracker) = self.sage_trackers.get_mut(&session.id) {
                    if tracker.leaving {
                        tracker.leaving = false;
                        tracker.done = None;
                        tracker.gone_time = None;
                        if let Some(sage) = self.sages.get_mut(&session.id) {
                            let target_x = pet.x + tracker.side * SAGE_GAP * pet.scale_factor();
                            sage.target_x = Some(target_x);
                            sage.state = PetState::Walk;
                            sage.pace = crate::app::types::SAGE_HURRY;
                        }
                    }
                } else if !self.sages.contains_key(&session.id) {
                    sages_to_call.push(session.id.clone());
                }
            } else if !session.advising
                && let Some(tracker) = self.sage_trackers.get_mut(&session.id)
                    && tracker.done.is_none() {
                        let min_over = tracker.since + Duration::from_secs_f32(crate::app::types::SAGE_MIN_SECS);
                        tracker.done = Some(now.max(min_over));
                    }

            if let Some(sage) = self.sages.get_mut(&session.id) {
                sage.headlamp = pet.headlamp;
                pet.look_toward(sage.x);
                sage.look_toward(pet.x);
                if !self.sage_trackers.get(&session.id).is_some_and(|t| t.leaving) {
                    sage.wish = PetWish::Read;
                }
            }

            // I. Synchronisation des sous-agents
            if settings.subagents_show {
                let max_subs = settings.subagents_max;
                let count = session.subagents.len().min(max_subs).max(1) as f32;
                let reach = (140.0 + 30.0 * count) * pet.scale;
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
                        small.home_x = pet.x;
                        small.home_reach = reach;
                        small.kind = pet.kind;
                        small.custom_color = pet.custom_color;
                        small.current_ground = pet.current_ground;
                        small.stature = settings.subagents_size;
                        small.headlamp = pet.headlamp;
                        small.grooving = pet.grooving;
                        small.dismissing = false;
                    } else {
                        let mut small = Pet::new_small(
                            session.id.clone(),
                            sub.id.clone(),
                            sub.name.clone(),
                            pet.kind,
                            pet.x,
                            pet.current_ground,
                            reach,
                            pet.scale,
                        );
                        small.custom_color = pet.custom_color;
                        small.stature = settings.subagents_size;
                        small.caption = caption;
                        small.current_ground = pet.current_ground;
                        small.headlamp = pet.headlamp;
                        small.grooving = pet.grooving;
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

        // Nettoyage des sessions terminées
        let to_remove: Vec<String> = self
            .pets
            .keys()
            .filter(|id| id.as_str() != "none" && !active_ids.contains(*id))
            .cloned()
            .collect();
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

        if active_ids.is_empty() {
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

        for p in self.pets.values_mut() {
            if let Some((_, t)) = p.dispute
                && now.duration_since(t).as_secs() >= 600 {
                    p.dispute = None;
                }
        }

        self.active_sessions = sessions.into_iter().map(|s| (s.id.clone(), s)).collect();

        // Envoi effectif des lettres
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

        for sid in celebrations_to_trigger {
            self.celebrate(&sid, now);
        }

        self.handle_session_collisions(collisions_to_handle, lang, &settings, now);

        // Regard orienté vers la session attendue (awaited_peer)
        for (id, session) in &self.active_sessions {
            if let Some(ref dest) = session.message_sent
                && let Some(target_id) = self.resolve_session_named(dest, id)
                    && let Some(target_x) = self.pets.get(&target_id).map(|p| p.x)
                        && let Some(p) = self.pets.get_mut(id) {
                            p.look_toward(target_x);
                        }
        }

        self.dispatch_sage_calls(sages_to_call, total_w, now);
    }
}
