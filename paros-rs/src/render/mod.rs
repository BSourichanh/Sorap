use std::cell::RefCell;
use std::collections::HashMap;

use crate::letter::FlyingLetter;
use crate::pet::{MascotKind, Pet, PetState};

pub mod accessories;
pub mod agy_body;
pub mod claude_body;
pub mod colors;
pub mod postures;
pub mod text;

#[allow(unused_imports)]
pub use accessories::*;
#[allow(unused_imports)]
pub use agy_body::*;
#[allow(unused_imports)]
pub use claude_body::*;
pub use colors::*;
#[allow(unused_imports)]
pub use postures::*;
#[allow(unused_imports)]
pub use text::*;

pub const LABEL_MAX_LENGTH: usize = 28;
pub const CAPTION_MAX_LENGTH: usize = 110;

#[derive(Clone)]
pub struct CachedGlyph {
    pub metrics: fontdue::Metrics,
    pub bitmap: Vec<u8>,
}

pub struct Renderer {
    pub(crate) font: Option<fontdue::Font>,
    pub(crate) glyph_cache: RefCell<HashMap<(char, u32), CachedGlyph>>,
}

impl Renderer {
    pub fn new() -> Self {
        let font_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf",
            "/run/current-system/sw/share/X11/fonts/TTF/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        ];
        let font = font_paths.iter().find_map(|path| {
            std::fs::read(path).ok().and_then(|data| {
                fontdue::Font::from_bytes(data, fontdue::FontSettings::default()).ok()
            })
        });

        let renderer = Self {
            font,
            glyph_cache: RefCell::new(HashMap::new()),
        };

        if let Some(ref f) = renderer.font {
            let mut cache = renderer.glyph_cache.borrow_mut();
            for &size in &[11.0f32, 12.0f32] {
                let size_key = (size * 10.0) as u32;
                for ch in 32u8..=126u8 {
                    let c = ch as char;
                    let (metrics, bitmap) = f.rasterize(c, size);
                    cache.insert((c, size_key), CachedGlyph { metrics, bitmap });
                }
            }
        }

        renderer
    }

    pub fn with_glyph<F, R>(&self, ch: char, font_size: f32, f: F) -> Option<R>
    where
        F: FnOnce(&fontdue::Metrics, &[u8]) -> R,
    {
        let size_key = (font_size * 10.0) as u32;
        let key = (ch, size_key);

        if let Some(glyph) = self.glyph_cache.borrow().get(&key) {
            return Some(f(&glyph.metrics, &glyph.bitmap));
        }

        let font = self.font.as_ref()?;
        let (metrics, bitmap) = font.rasterize(ch, font_size);
        let cached = CachedGlyph { metrics, bitmap };
        let res = f(&cached.metrics, &cached.bitmap);
        self.glyph_cache.borrow_mut().insert(key, cached);
        Some(res)
    }

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
        if pet.meditating  { hop = 5.0 + (time * 1.5).sin() * 2.0;
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
        if pet.meditating  { self.draw_mantra(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, time, false);
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
        if pet.baggage > 0  { Self::draw_baggage(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing, pet.baggage, pet.kind);
        }

        // 8. Mascotte (Claude ou Agy)
        match pet.kind {
            MascotKind::Claude => {
                Self::draw_claude(pixmap, cx, feet_y, unit, pet, body_offset_x, body_offset_y, hop, arm_raise,
                    is_airborne, look_x, look_y, body_color, );
            }
            MascotKind::Agy => {
                Self::draw_agy(pixmap, cx, feet_y, unit, pet, body_offset_x, body_offset_y, hop, arm_raise,
                    is_airborne, look_x, look_y, body_color, );
            }
        }

        // 9. Casque de chantier (dessiné après le corps pour que le rebord repose sur le front sans être masqué)
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

        // 9d. Boîtier et sangle de la lampe frontale (dessinés après le corps)
        if pet.headlamp && pet.state != PetState::Sleep {
            Self::draw_headlamp(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // Parapluie lors des chutes ralenties (immédiatement après le chapeau et le corps)
        if pet.umbrella {
            Self::draw_umbrella(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // 10. Balai (Sweep)
        if pet.state == PetState::Sweep  { Self::draw_broom(pixmap, cx, feet_y, unit, pet.facing, pet.state_time);
        }

        // 11. Symboles mantra de méditation (premier plan)
        if pet.meditating  { self.draw_mantra(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, time, true);
        }

        // 12. Feu de camp (au repos complet ou Roast)
        if (pet.campfire || pet.state == PetState::Roast) && !is_airborne  { Self::draw_campfire(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing, time, pet.state_time);
        }

        // 13. Sablier (tâche de fond en cours ou Wait)
        if (pet.hourglass || pet.state == PetState::Wait) && !is_airborne  { Self::draw_hourglass(pixmap, cx, feet_y, unit, pet.facing, time);
        }

        // 14. Carte dépliée en main si lost
        if pet.lost && !is_airborne && matches!(pet.state, PetState::Idle | PetState::Sit)  { Self::draw_lost_map(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // 15. Compactage du contexte (Compact)
        if pet.state == PetState::Compact {
            Self::draw_compaction(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing,
                pet.state_time, time, pet.strained, );
        }


        // 17. Fumée au-dessus de la tête (contexte plein >= 75%, masquée si Sleep)
        if pet.smoking && pet.state != PetState::Sleep {
            Self::draw_smoke(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, time);
        }

        // 18. Particules d'expression (ronflement, alerte, réflexion, cœurs)
        if pet.state == PetState::Sleep  { self.draw_snore(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, time, body_color);
        } else if pet.cheer_timer > 0.0 || pet.state == PetState::Cheer {
            Self::draw_hearts(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, time);
        } else if pet.state == PetState::Alert {
            Self::draw_alert(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, time);
        } else if pet.thinking || pet.state == PetState::Think {
            Self::draw_think(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, time, body_color);
        }

        // Croix de colère rouge pour Glare
        if pet.state == PetState::Glare  { Self::draw_glare_cross(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, pet.facing);
        }

        // Étincelles dorées pour HighFive
        if pet.state == PetState::HighFive  { Self::draw_high_five_sparks(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing, time);
        }

        // Goutte de sueur bleue pour Worry
        if pet.state == PetState::Worry  { Self::draw_worry_sweat(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // Étincelles dorées pour Shake
        if pet.state == PetState::Shake  { Self::draw_shake_sparks(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, time);
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
        if is_grooving  { Self::draw_notes(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y + hat_offset_y, time, body_color);
        }

        // 21. Feuille de papier tenue (sous-agent en course de naissance)
        if pet.sheet  { Self::draw_sheet(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // 21b. Feuille de lecture tenue (session ou Sage qui lit une lettre)
        if pet.state == PetState::Read  { Self::draw_read_sheet(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, pet.facing);
        }

        // Mode discret : masque étiquette, légendes, bulles et cartes de survol
        if pet.discreet  { return;
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

        // 24. Bulle de dialogue (Say Bubble) prioritaire sur Carte au survol (Hover Card) conforme à bubble.gd:57
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

    #[allow(dead_code)]
    pub fn draw_flying_letter(&self, pixmap: &mut tiny_skia::PixmapMut, letter: &FlyingLetter) {
        self.draw_flying_letter_at(pixmap, letter, letter.current_x, letter.current_y);
    }

    pub fn draw_flying_letter_at(
        &self,
        pixmap: &mut tiny_skia::PixmapMut,
        letter: &FlyingLetter,
        cx: f32,
        cy: f32,
    ) {
        if letter.finished {
            return;
        }

        let transform = tiny_skia::Transform::from_translate(cx, cy)
            .pre_rotate(letter.angle.to_degrees());

        let mut paper_paint = tiny_skia::Paint::default();
        paper_paint.set_color_rgba8(PAPER.0, PAPER.1, PAPER.2, PAPER.3);

        let mut ink_paint = tiny_skia::Paint::default();
        ink_paint.set_color_rgba8(EYE.0, EYE.1, EYE.2, EYE.3);

        let mut seal_paint = tiny_skia::Paint::default();
        seal_paint.set_color_rgba8(
            letter.seal_color.0,
            letter.seal_color.1,
            letter.seal_color.2,
            letter.seal_color.3,
        );

        let half_w = 18.0 * letter.scale;
        let half_h = 12.0 * letter.scale;
        let w = 36.0 * letter.scale;
        let h = 24.0 * letter.scale;

        if let Some(r) = tiny_skia::Rect::from_xywh(-half_w, -half_h, w, h) {
            pixmap.fill_rect(r, &paper_paint, transform, None);
        }

        let stroke = tiny_skia::Stroke {
            width: 2.0 * letter.scale,
            ..Default::default()
        };
        if let Some(r) = tiny_skia::Rect::from_xywh(-half_w, -half_h, w, h) {
            let path = tiny_skia::PathBuilder::from_rect(r);
            pixmap.stroke_path(&path, &ink_paint, &stroke, transform, None);
        }

        let mut pb = tiny_skia::PathBuilder::new();
        pb.move_to(-half_w, -half_h);
        pb.line_to(0.0, 2.0 * letter.scale);
        pb.line_to(half_w, -half_h);
        if let Some(p) = pb.finish() {
            pixmap.stroke_path(&p, &ink_paint, &stroke, transform, None);
        }

        let seal_size = 6.0f32 * letter.scale;
        if let Some(r) = tiny_skia::Rect::from_xywh(-3.0 * letter.scale, -letter.scale, seal_size, seal_size) {
            pixmap.fill_rect(r, &seal_paint, transform, None);
        }
    }

    fn draw_hover_card(
        &self,
        pixmap: &mut tiny_skia::PixmapMut,
        card_text: &str,
        cx: f32,
        feet_y: f32,
        head_offset: f32,
    ) {
        let Some(font) = self.font.as_ref() else {
            return;
        };

        let font_size = 12.0f32;
        let max_text_width = 264.0f32;

        let mut lines = Vec::new();
        for raw_line in card_text.lines() {
            if raw_line.is_empty() {
                lines.push(String::new());
                continue;
            }
            let mut cur_line = String::new();
            let mut cur_w = 0.0f32;
            for word in raw_line.split(' ') {
                let word_w: f32 = word.chars().map(|c| font.metrics(c, font_size).advance_width).sum();
                let space_w = font.metrics(' ', font_size).advance_width;
                if cur_line.is_empty() {
                    cur_line = word.to_string();
                    cur_w = word_w;
                } else if cur_w + space_w + word_w <= max_text_width {
                    cur_line.push(' ');
                    cur_line.push_str(word);
                    cur_w += space_w + word_w;
                } else {
                    lines.push(cur_line);
                    cur_line = word.to_string();
                    cur_w = word_w;
                }
            }
            if !cur_line.is_empty()  { lines.push(cur_line);
            }
        }

        if lines.is_empty()  { return;
        }

        let line_height = font_size + 4.5f32;
        let mut max_line_w = 0.0f32;
        for line in &lines {
            let mut w = 0.0f32;
            for ch in line.chars() {
                w += font.metrics(ch, font_size).advance_width;
            }
            if w > max_line_w  { max_line_w = w;
            }
        }

        let padding_h = 10.0f32;
        let padding_v = 7.0f32;
        let border = 3.0f32;
        let tail = 8.0f32;

        let card_w = max_line_w + padding_h * 2.0;
        let card_h = lines.len() as f32 * line_height + padding_v * 2.0;

        let card_x = cx - card_w / 2.0;
        let raw_y = feet_y - head_offset - card_h - border - tail;
        let card_y = raw_y.max(4.0);

        let mut ink_paint = tiny_skia::Paint::default();
        ink_paint.set_color_rgba8(EYE.0, EYE.1, EYE.2, 255);

        if card_y + card_h < feet_y - head_offset + 10.0
            && let Some(tail_r) = tiny_skia::Rect::from_xywh(cx - tail / 2.0, card_y + card_h, tail, border + tail)
        {
            pixmap.fill_rect(tail_r, &ink_paint, tiny_skia::Transform::identity(), None);
        }

        if let Some(border_r) = tiny_skia::Rect::from_xywh(card_x - border, card_y - border, card_w + border * 2.0, card_h + border * 2.0)  { pixmap.fill_rect(border_r, &ink_paint, tiny_skia::Transform::identity(), None);
        }

        let mut paper_paint = tiny_skia::Paint::default();
        paper_paint.set_color_rgba8(PAPER.0, PAPER.1, PAPER.2, 255);
        if let Some(r) = tiny_skia::Rect::from_xywh(card_x, card_y, card_w, card_h)  { pixmap.fill_rect(r, &paper_paint, tiny_skia::Transform::identity(), None);
        }

        for (i, line) in lines.iter().enumerate() {
            let start_x = card_x + padding_h;
            let baseline_y = card_y + padding_v + (i + 1) as f32 * line_height - 3.0;
            self.draw_text_line(pixmap, line, start_x, baseline_y, font_size, EYE);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_context_menu<S: AsRef<str>>(&self, pixmap: &mut tiny_skia::PixmapMut, menu_x: f32,
        menu_y: f32, menu_w: f32, menu_h: f32, items: &[S], hovered_idx: Option<usize>,
        separator_after: Option<usize>,
    ) {
        let mut bg_paint = tiny_skia::Paint::default();
        bg_paint.set_color_rgba8(EYE.0, EYE.1, EYE.2, 248);

        if let Some(r) = tiny_skia::Rect::from_xywh(menu_x, menu_y, menu_w, menu_h)  { pixmap.fill_rect(r, &bg_paint, tiny_skia::Transform::identity(), None);
        }

        let mut border_paint = tiny_skia::Paint::default();
        border_paint.set_color_rgba8(CLAUDE.0, CLAUDE.1, CLAUDE.2, 255);
        let stroke = tiny_skia::Stroke {
            width: 1.5, ..Default::default()
        };
        if let Some(r) = tiny_skia::Rect::from_xywh(menu_x, menu_y, menu_w, menu_h) {
            let path = tiny_skia::PathBuilder::from_rect(r);
            pixmap.stroke_path(&path, &border_paint, &stroke, tiny_skia::Transform::identity(), None);
        }

        let item_height = 26.0f32;
        let padding_top = 5.0f32;
        let padding_left = 12.0f32;
        let font_size = 11.0f32;

        for (idx, item) in items.iter().enumerate() {
            let item_text = item.as_ref();
            let item_y = menu_y + padding_top + idx as f32 * item_height;

            if hovered_idx == Some(idx) {
                let mut hover_paint = tiny_skia::Paint::default();
                hover_paint.set_color_rgba8(CLAUDE.0, CLAUDE.1, CLAUDE.2, 220);
                if let Some(hr) = tiny_skia::Rect::from_xywh(menu_x + 3.0, item_y, menu_w - 6.0, item_height,
                ) {
                    pixmap.fill_rect(hr, &hover_paint, tiny_skia::Transform::identity(), None);
                }
            }

            let cur_x = menu_x + padding_left;
            let text_y = item_y + 17.0;
            self.draw_text_line(pixmap, item_text, cur_x, text_y, font_size, PAPER);

            if separator_after == Some(idx) {
                let sep_y = item_y + item_height;
                if let Some(sep_rect) = tiny_skia::Rect::from_xywh(menu_x + 6.0, sep_y, menu_w - 12.0, 1.0) {
                    let mut sep_paint = tiny_skia::Paint::default();
                    sep_paint.set_color_rgba8(CLAUDE.0, CLAUDE.1, CLAUDE.2, 120);
                    pixmap.fill_rect(sep_rect, &sep_paint, tiny_skia::Transform::identity(), None);
                }
            }
        }
    }
}
