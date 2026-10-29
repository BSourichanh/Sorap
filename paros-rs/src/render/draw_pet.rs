use crate::pet::{MascotKind, Pet, PetState};
use super::colors::*;
use super::Renderer;
use super::{CAPTION_MAX_LENGTH, LABEL_MAX_LENGTH};

impl Renderer {
    #[allow(dead_code)]
    pub fn draw_pet(
        &self,
        pixmap: &mut tiny_skia::PixmapMut,
        pet: &Pet,
        cursor_pos: Option<(f32, f32)>,
        bubble_text: Option<&str>,
        bubble_enabled: bool,
    ) {
        self.draw_pet_at(pixmap, pet, pet.x, pet.y, cursor_pos, bubble_text, bubble_enabled);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_pet_at(
        &self,
        pixmap: &mut tiny_skia::PixmapMut,
        pet: &Pet,
        cx: f32,
        feet_y: f32,
        cursor_pos: Option<(f32, f32)>,
        bubble_text: Option<&str>,
        bubble_enabled: bool,
    ) {
        if pet.hidden {
            return;
        }
        let unit = 9.0f32 * pet.scale * pet.stature;
        let time = pet.time;
        let pace = pet.pace;
        let is_airborne = pet.is_airborne();
        let front = if pet.facing > 0.0 { 1 } else { 0 };

        let body_color = get_pet_body_color(pet);
        let hat_room = get_hat_room(pet);
        let hat_offset_y = -hat_room * unit;

        let mut arm_raise = [0.0f32, 0.0f32];
        let mut rise = ((time * 2.0).sin() * 0.5 + 0.5) * 2.0;
        let mut hop = 0.0f32;
        let mut shake = 0.0f32;

        match pet.state {
            PetState::Walk => rise = (time * 9.0 * pace).sin().abs() * 3.0,
            PetState::Idle => rise = ((time * 2.0).sin() * 0.5 + 0.5) * 2.0,
            PetState::Think => {
                rise = if pet.meditating {
                    ((time * 1.2).sin() * 0.5 + 0.5) * 2.0 - 2.0 * unit
                } else {
                    ((time * 2.0).sin() * 0.5 + 0.5) * 2.0
                };
            }
            PetState::Sit | PetState::Sleep | PetState::Wait => {
                rise = ((time * 1.2).sin() * 0.5 + 0.5) * 2.0 - 2.0 * unit
            }
            PetState::Roast => {
                rise = ((time * 1.2).sin() * 0.5 + 0.5) * 2.0 - 2.0 * unit;
                arm_raise[front] = 0.5;
            }
            PetState::Compact => {
                rise = ((time * 1.2).sin() * 0.5 + 0.5) * 2.0 - 2.0 * unit;
                let moment = (pet.state_time % 2.4) / 2.4;
                if moment < 0.3 {
                    arm_raise[front] = 4.0;
                } else if moment < 0.55 {
                    arm_raise[front] = 0.5;
                } else if moment < 0.65 {
                    arm_raise[front] = 2.0;
                }
            }
            PetState::Stretch => {
                rise = ((pet.state_time / 1.2).min(1.0) * std::f32::consts::PI).sin() * 12.0;
                arm_raise = [2.0, 2.0];
            }
            PetState::Cheer => {
                hop = (time * 10.0).sin().abs() * 18.0;
                arm_raise = [2.0, 2.0];
            }
            PetState::Greet => {
                hop = (time * 8.0).sin().abs() * 4.0;
                arm_raise[front] = 1.0 + if (time * 14.0).sin() > 0.0 { 1.0 } else { 0.0 };
            }
            PetState::Glare => shake = (time * 30.0).sin() * 1.0,
            PetState::HighFive => {
                hop = ((pet.state_time / 1.3).min(1.0) * std::f32::consts::PI).sin() * 12.0;
                arm_raise[front] = 3.0;
            }
            PetState::Worry => shake = (time * 45.0).sin() * 2.5,
            PetState::Knock => {
                for &t0 in &[0.25f32, 0.65f32] {
                    if (pet.state_time - t0).abs() < 0.12 {
                        arm_raise[front] = 2.0;
                    }
                    if pet.state_time >= t0 && pet.state_time < t0 + 0.12 {
                        shake += ((pet.state_time - t0) / 0.12 * std::f32::consts::PI * 2.0).sin()
                            * 2.0
                            * pet.facing;
                    }
                }
            }
            PetState::Sweep => arm_raise[front] = 0.5,
            PetState::Nod => {
                rise = -((pet.state_time / 0.8 * std::f32::consts::TAU).sin()).abs() * 5.0;
            }
            PetState::Shake => {
                shake = (time * 40.0).sin() * 3.0 * (1.0 - (pet.state_time / 1.0).clamp(0.0, 1.0))
            }
            PetState::Alert => {
                hop = (time * 8.0).sin().abs() * (if pet.urgent { 18.0 } else { 6.0 });
                arm_raise[0] = 2.0 * if (time * 10.0).sin() > 0.0 { 1.0 } else { 0.0 };
                arm_raise[1] = 2.0 * if (time * 10.0 + std::f32::consts::PI).sin() > 0.0 { 1.0 } else { 0.0 };
            }
            PetState::Throw => {
                hop = ((pet.state_time / 0.5).min(1.0) * std::f32::consts::PI).sin() * 6.0;
                arm_raise[front] = 3.0;
            }
            PetState::Read => arm_raise[front] = 0.5,
            PetState::Climb | PetState::Carried | PetState::Fall => arm_raise = [2.0, 2.0],
            #[allow(unreachable_patterns)]
            _ => {}
        }

        if is_airborne || pet.sheet {
            arm_raise = [2.0, 2.0];
        }

        // Bras arrière qui se gratte la tête si égaré
        if pet.lost && !is_airborne && matches!(pet.state, PetState::Idle | PetState::Sit) {
            arm_raise[1 - front] = 2.0 + if (time * 8.0).sin() > 0.0 { 0.5 } else { 0.0 };
        }

        // Lévitation en méditation
        if pet.meditating {
            hop = 5.0 + (time * 1.5).sin() * 2.0;
        }

        let is_grooving = pet.grooving
            && !is_airborne
            && matches!(pet.state, PetState::Idle | PetState::Sit | PetState::Wait | PetState::Think)
            && !pet.meditating;

        if is_grooving {
            rise += (time * std::f32::consts::PI * 2.0).sin().abs() * 2.5;
            shake += (time * std::f32::consts::PI * 2.0).sin() * 1.5;
        }

        let body_offset_y = -hop - rise;
        let body_offset_x = shake;

        // 1. Étiquette sous les pattes (nom de session · branche, masquée si discreet ou empilée)
        let stacked = pet.stacked();
        if !pet.discreet && !pet.label.is_empty() && !stacked {
            let label: String = pet.label.chars().take(LABEL_MAX_LENGTH).collect();
            self.draw_tag(pixmap, &label, cx, feet_y + 7.0, body_color);
        }

        // 2. Ombre au sol (SHADOW = 18% alpha, masquée si en l'air ou empilée, rétrécie par le hop)
        if !is_airborne && !stacked {
            let shadow_y = feet_y;
            let shadow_w = (13.0 * unit * (1.0 - hop * 0.015)).max(unit * 4.0);

            let mut shadow_paint = tiny_skia::Paint::default();
            shadow_paint.set_color_rgba8(0, 0, 0, 46);

            if let Some(r) = tiny_skia::Rect::from_xywh(cx - shadow_w / 2.0, shadow_y, shadow_w, 4.0) {
                pixmap.fill_rect(r, &shadow_paint, tiny_skia::Transform::identity(), None);
            }
        }

        // 2b. Boîte aux lettres au sol si messages en attente (masquée si empilée)
        if pet.mail > 0 && !is_airborne && !stacked {
            let mbox_side = pet.mailbox_side(pixmap.width() as f32);
            self.draw_mailbox(pixmap, cx, feet_y, unit, mbox_side, pet.mail, pet.discreet);
        }

        // 3. Panneau d'avertissement Git conflit au sol (avant le corps)
        if pet.hard_hat && !is_airborne {
            Self::draw_construction_sign(pixmap, cx, feet_y, unit, pet.facing);
        }

        // 4. Lampe frontale (faisceau lumineux de nuit)
        if pet.headlamp && pet.state != PetState::Sleep && !is_airborne {
            Self::draw_headlamp_beam(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // 5. Symboles mantra de méditation (arrière-plan)
        if pet.meditating {
            self.draw_mantra(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, time, false);
        }

        let eyes_closed = pet.meditating
            || pet.state == PetState::Shake
            || ((pet.blink_timer < 0.0 || matches!(pet.state, PetState::Sleep | PetState::Stretch)) && !is_airborne);

        // 6. Calcul dynamique du regard (suivi curseur prioritaire si survolé, sinon orientation par état)
        let (look_x, look_y) = if pet.hovered && !is_airborne && !eyes_closed {
            if let Some((cur_x, cur_y)) = cursor_pos {
                let head_x = cx + body_offset_x;
                let head_y = feet_y - 7.0 * unit + body_offset_y;
                let dx = cur_x - head_x;
                let dy = cur_y - head_y;
                let max_reach = unit * 3.0;
                let mut lx = dx / max_reach;
                let mut ly = dy / max_reach;
                let len = (lx * lx + ly * ly).sqrt();
                if len > 1.0 {
                    lx /= len;
                    ly /= len;
                }
                (lx, ly)
            } else {
                (if pet.facing > 0.0 { 1.0 } else { -1.0 }, 0.0)
            }
        } else if matches!(pet.state, PetState::Sleep | PetState::Stretch | PetState::Worry) {
            (0.0, 0.0)
        } else if pet.state == PetState::Sit {
            ((time * 0.9).sin().signum(), 0.0)
        } else if pet.state == PetState::Read {
            (if pet.facing > 0.0 { 1.0 } else { -1.0 }, 0.3)
        } else if pet.state == PetState::Wait {
            (if pet.facing > 0.0 { 1.0 } else { -1.0 }, 0.6)
        } else if pet.state == PetState::Think {
            if pet.tapping {
                (if pet.facing > 0.0 { 0.5 } else { -0.5 }, 0.5)
            } else {
                (0.0, -0.6)
            }
        } else if pet.state == PetState::Compact {
            let moment = (pet.state_time % 2.4) / 2.4;
            if moment < 0.3 {
                (0.0, -0.6)
            } else {
                (if pet.facing > 0.0 { 1.0 } else { -1.0 }, 0.5)
            }
        } else {
            (if pet.facing > 0.0 { 1.0 } else { -1.0 }, 0.0)
        };

        // 7. Dossiers Git modifiés (baggage)
        if pet.baggage > 0 {
            Self::draw_baggage(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing, pet.baggage, pet.kind);
        }

        // 8. Mascotte (Claude ou Agy)
        match pet.kind {
            MascotKind::Claude => {
                Self::draw_claude(pixmap, cx, feet_y, unit, pet, body_offset_x, body_offset_y, hop, arm_raise,
                    is_airborne, look_x, look_y, body_color);
            }
            MascotKind::Agy => {
                Self::draw_agy(pixmap, cx, feet_y, unit, pet, body_offset_x, body_offset_y, hop, arm_raise,
                    is_airborne, look_x, look_y, body_color);
            }
        }

        // 9. Casque de chantier
        if pet.hard_hat && pet.kind != MascotKind::Agy && !pet.is_sage {
            Self::draw_hard_hat(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // 9b. Accessoires sur la tête
        if !pet.hard_hat && pet.kind != MascotKind::Agy && pet.accessory > 0 && !pet.is_sage {
            Self::draw_accessories(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing, pet.accessory);
        }

        // 9c. Attributs du Sage
        if pet.is_sage {
            Self::draw_sage_beard(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y);
            Self::draw_sage_staff(pixmap, cx, feet_y, unit, pet.facing);
            Self::draw_sage_hat(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // 9d. Boîtier et sangle de la lampe frontale
        if pet.headlamp && pet.state != PetState::Sleep {
            Self::draw_headlamp(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // Parapluie lors des chutes ralenties
        if pet.umbrella {
            Self::draw_umbrella(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // 10. Balai (Sweep)
        if pet.state == PetState::Sweep {
            Self::draw_broom(pixmap, cx, feet_y, unit, pet.facing, pet.state_time);
        }

        // 11. Symboles mantra de méditation (premier plan)
        if pet.meditating {
            self.draw_mantra(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, time, true);
        }

        // 12. Feu de camp (au repos complet ou Roast)
        if (pet.campfire || pet.state == PetState::Roast) && !is_airborne {
            Self::draw_campfire(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing, time, pet.state_time);
        }

        // 13. Sablier (tâche de fond en cours ou Wait)
        if (pet.hourglass || pet.state == PetState::Wait) && !is_airborne {
            Self::draw_hourglass(pixmap, cx, feet_y, unit, pet.facing, time);
        }

        // 14. Carte dépliée en main si lost
        if pet.lost && !is_airborne && matches!(pet.state, PetState::Idle | PetState::Sit) {
            Self::draw_lost_map(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // 15. Compactage du contexte (Compact)
        if pet.state == PetState::Compact {
            Self::draw_compaction(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing,
                pet.state_time, time, pet.strained);
        }

        // 17. Fumée au-dessus de la tête
        if pet.smoking && pet.state != PetState::Sleep {
            Self::draw_smoke(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, time);
        }

        // 18. Particules d'expression
        if pet.state == PetState::Sleep {
            self.draw_snore(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, time, body_color);
        } else if pet.cheer_timer > 0.0 || pet.state == PetState::Cheer {
            Self::draw_hearts(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, time);
        } else if pet.state == PetState::Alert {
            Self::draw_alert(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, time);
        } else if pet.thinking || pet.state == PetState::Think {
            Self::draw_think(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, time, body_color);
        }

        // Croix de colère rouge pour Glare
        if pet.state == PetState::Glare {
            Self::draw_glare_cross(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // Étincelles dorées pour HighFive
        if pet.state == PetState::HighFive {
            Self::draw_high_five_sparks(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing, time);
        }

        // Goutte de sueur bleue pour Worry
        if pet.state == PetState::Worry {
            Self::draw_worry_sweat(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // Étincelles dorées pour Shake
        if pet.state == PetState::Shake {
            Self::draw_shake_sparks(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, time);
        }

        // Point d'interrogation clignotant au-dessus de la tête si lost
        if pet.lost
            && !is_airborne
            && matches!(pet.state, PetState::Idle | PetState::Sit)
            && (time % 1.2) < 0.8
        {
            self.draw_lost_question_mark(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, body_color);
        }

        // 19. Notes de musique (grooving)
        if is_grooving {
            Self::draw_notes(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, time, body_color);
        }

        // 21. Feuille de papier tenue (sous-agent en course de naissance)
        if pet.sheet {
            Self::draw_sheet(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // 21b. Feuille de lecture tenue (session ou Sage qui lit une lettre)
        if pet.state == PetState::Read {
            Self::draw_read_sheet(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // Mode discret : masque étiquette, légendes, bulles et cartes de survol
        if pet.discreet {
            return;
        }

        // 22. Caption d'outil pour petit personnage (sous-agent)
        if pet.is_small && (!pet.caption.is_empty() || !pet.activity_text.is_empty()) {
            let cap = if !pet.activity_text.is_empty() {
                &pet.activity_text
            } else {
                &pet.caption
            };
            self.draw_small_caption(pixmap, cap, cx, feet_y - 12.0 * unit + body_offset_y + hat_offset_y, pet.kind);
        }

        // 23. Panneau sombre d'activité pour mascotte normale quand elle travaille
        if !pet.is_small {
            let act_text = if !pet.activity_text.is_empty() {
                &pet.activity_text
            } else if pet.state == PetState::Think && !pet.caption.is_empty() {
                &pet.caption
            } else {
                ""
            };
            if !act_text.is_empty() {
                let truncated: String;
                let text_to_draw = if act_text.chars().count() > CAPTION_MAX_LENGTH {
                    truncated = act_text.chars().take(CAPTION_MAX_LENGTH).collect();
                    &truncated
                } else {
                    act_text
                };
                let bottom_y = feet_y - (14.2 + hat_room) * unit + body_offset_y;
                self.draw_activity_caption(pixmap, text_to_draw, cx, bottom_y, body_color);
            }
        }

        // 24. Bulle de dialogue prioritaire sur Carte au survol
        let say = if bubble_enabled {
            pet.say_text.as_deref().or(bubble_text)
        } else {
            None
        };
        let has_speech = if let Some(text) = say {
            !text.is_empty() && (pet.say_text.is_none() || pet.say_timer > 0.0)
        } else {
            false
        };

        if has_speech {
            if let Some(text) = say {
                self.draw_bubble(pixmap, text, cx, feet_y - 114.0 * pet.scale * pet.stature + hat_offset_y);
            }
        } else if pet.hovered
            && let Some(ref card) = pet.hover_card
        {
            let head_offset = 114.0 * pet.scale * pet.stature;
            self.draw_hover_card(pixmap, card, cx, feet_y, head_offset);
        }
    }
}
