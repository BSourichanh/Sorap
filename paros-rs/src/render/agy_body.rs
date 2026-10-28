#![allow(clippy::too_many_arguments)]

use crate::pet::{Pet, PetState};
use super::colors::*;
use super::Renderer;

const AGY_MATRIX: [Option<(u8, u8, u8)>; 120] = [
    None, None, None, None, None, Some((242, 146, 46)), Some((240, 114, 54)), None, None, None, None, None,
    None, None, None, None, Some((219, 177, 49)), Some((246, 145, 46)), Some((243, 115, 55)), Some((240, 88, 59)), None, None, None, None,
    None, None, None, Some((158, 195, 69)), Some((181, 180, 62)), Some((226, 153, 61)), Some((246, 122, 52)), Some((248, 106, 53)), Some((239, 84, 66)), None, None, None,
    None, None, None, Some((134, 198, 78)), Some((117, 180, 94)), Some((204, 149, 77)), Some((239, 121, 71)), Some((225, 102, 82)), Some((225, 79, 89)), None, None, None,
    None, None, Some((124, 194, 81)), Some((113, 194, 92)), Some((92, 169, 143)), Some((92, 145, 179)), Some((131, 115, 176)), Some((116, 111, 195)), Some((153, 93, 168)), Some((156, 91, 151)), None, None,
    None, None, Some((128, 198, 84)), Some((84, 184, 129)), Some((64, 151, 222)), None, None, Some((74, 126, 228)), Some((112, 110, 206)), Some((143, 100, 180)), None, None,
    None, None, Some((97, 195, 125)), Some((67, 174, 171)), None, None, None, None, Some((74, 128, 234)), Some((108, 115, 216)), None, None,
    None, Some((109, 198, 148)), Some((98, 186, 213)), Some((71, 168, 220)), None, None, None, None, Some((61, 137, 251)), Some((74, 129, 240)), Some((101, 121, 225)), None,
    None, Some((107, 199, 163)), Some((100, 182, 246)), None, None, None, None, None, None, Some((56, 134, 251)), Some((72, 129, 244)), None,
    Some((103, 185, 244)), Some((100, 182, 246)), None, None, None, None, None, None, None, None, Some((56, 131, 249)), Some((61, 133, 252)),
];

impl Renderer {
    pub fn draw_agy(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        pet: &Pet,
        body_offset_x: f32,
        body_offset_y: f32,
        hop: f32,
        arm_raise: [f32; 2],
        is_airborne: bool,
        look_x: f32,
        look_y: f32,
        _body_color: (u8, u8, u8, u8),
    ) {
        let time = pet.time;
        let pace = pet.pace;

        let agy_look_x = look_x * 0.5;
        let agy_look_y = look_y * 0.5;

        let seated = pet.meditating
            || matches!(
                pet.state,
                PetState::Sit | PetState::Sleep | PetState::Roast | PetState::Wait | PetState::Compact
            );

        let front_leg = if pet.facing > 0.0 { 1 } else { 0 };

        // Pattes d'Agy
        if !seated {
            for leg_idx in 0..2 {
                let mut leg_x = body_offset_x;
                let mut lift = hop;

                if is_airborne {
                    leg_x += (time * 6.0 + leg_idx as f32).sin() * 2.0;
                } else if pet.state == PetState::Walk {
                    let step = (time * 9.0 * pace + leg_idx as f32 * std::f32::consts::PI).sin();
                    lift += step.max(0.0) * unit * 0.8;
                } else if pet.state == PetState::Think && pet.tapping && leg_idx == front_leg {
                    lift += (time * 12.0).sin().max(0.0) * unit * 0.7;
                }

                let top = feet_y - 2.0 * unit + body_offset_y;
                let foot_y = feet_y - unit - lift;

                let col_range = if leg_idx == 0 { 0..6 } else { 6..12 };

                for row_idx in 8..10 {
                    for col_idx in col_range.clone() {
                        let Some(cell_color) = AGY_MATRIX[row_idx * 12 + col_idx] else {
                            continue;
                        };

                        let mut paint = tiny_skia::Paint::default();
                        paint.set_color_rgba8(cell_color.0, cell_color.1, cell_color.2, 255);

                        let lx = cx + (col_idx as f32 - 6.0) * unit + leg_x;
                        if row_idx == 8 {
                            let h = (foot_y - top).max(unit);
                            if let Some(rect) = tiny_skia::Rect::from_xywh(lx, top, unit, h) {
                                pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                            }
                        } else if let Some(rect) = tiny_skia::Rect::from_xywh(lx, foot_y, unit, unit) {
                            pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                        }
                    }
                }
            }
        }

        // Corps d'Agy
        for row_idx in 0..8 {
            let y = feet_y - (10.0 - row_idx as f32) * unit + body_offset_y;
            for col_idx in 0..12 {
                let Some(cell_color) = AGY_MATRIX[row_idx * 12 + col_idx] else {
                    continue;
                };

                let mut paint = tiny_skia::Paint::default();
                paint.set_color_rgba8(cell_color.0, cell_color.1, cell_color.2, 255);
                let x = cx + (col_idx as f32 - 6.0) * unit + body_offset_x;
                if let Some(rect) = tiny_skia::Rect::from_xywh(x, y, unit, unit) {
                    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }
            }
        }

        // Bras d'Agy (avec affichage du bras arrière quand pet.baggage > 0)
        let front = if pet.facing > 0.0 { 1 } else { 0 };

        // Bras gauche (0)
        if arm_raise[0] > 0.0 || (front != 0 && pet.baggage > 0) {
            let arm_col = (124, 194, 81, 255);
            let mut arm_paint = tiny_skia::Paint::default();
            arm_paint.set_color_rgba8(arm_col.0, arm_col.1, arm_col.2, 255);
            if let Some(r) = tiny_skia::Rect::from_xywh(
                cx - 6.0 * unit + body_offset_x,
                feet_y - 6.0 * unit + body_offset_y - arm_raise[0] * unit,
                2.0 * unit,
                2.0 * unit,
            ) {
                pixmap.fill_rect(r, &arm_paint, tiny_skia::Transform::identity(), None);
            }
        }

        // Bras droit (1)
        if arm_raise[1] > 0.0 || (front != 1 && pet.baggage > 0) {
            let arm_col = (156, 91, 151, 255);
            let mut arm_paint = tiny_skia::Paint::default();
            arm_paint.set_color_rgba8(arm_col.0, arm_col.1, arm_col.2, 255);
            if let Some(r) = tiny_skia::Rect::from_xywh(
                cx + 4.0 * unit + body_offset_x,
                feet_y - 6.0 * unit + body_offset_y - arm_raise[1] * unit,
                2.0 * unit,
                2.0 * unit,
            ) {
                pixmap.fill_rect(r, &arm_paint, tiny_skia::Transform::identity(), None);
            }
        }

        // Yeux ou lunettes de soleil
        if pet.cool_timer > 0.0 {
            Self::draw_shades(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y);
        } else {
            let closed = pet.meditating
                || pet.state == PetState::Shake
                || ((pet.blink_timer < 0.0 || matches!(pet.state, PetState::Sleep | PetState::Stretch)) && !is_airborne);
            let wide = is_airborne || pet.state == PetState::Worry;

            let eye_color = if pet.serving {
                let t = (time * std::f32::consts::TAU / 2.4).sin() * 0.5 + 0.5;
                lerp_color(EYE, SIGNAL, t)
            } else {
                EYE
            };

            let mut eye_paint = tiny_skia::Paint::default();
            eye_paint.set_color_rgba8(eye_color.0, eye_color.1, eye_color.2, 255);

            let agy_eyes = [
                (-2.0f32, -7.0f32, 1.0f32, 2.0f32),
                (1.0f32, -7.0f32, 1.0f32, 2.0f32),
            ];

            for &(ex, ey, ew, eh) in &agy_eyes {
                let (px, py, pw, ph) = if closed {
                    let center_y = ey + eh / 2.0;
                    (
                        cx + (ex + agy_look_x - 0.25) * unit + body_offset_x,
                        feet_y + (center_y - 0.2) * unit + body_offset_y,
                        1.5 * unit,
                        0.4 * unit,
                    )
                } else if pet.state == PetState::Glare {
                    (
                        cx + (ex + agy_look_x - 0.2) * unit + body_offset_x,
                        feet_y + (ey + agy_look_y + 1.0) * unit + body_offset_y,
                        1.4 * unit,
                        1.0 * unit,
                    )
                } else if wide {
                    (
                        cx + (ex + agy_look_x - 0.3) * unit + body_offset_x,
                        feet_y + (ey + agy_look_y - 0.3) * unit + body_offset_y,
                        (ew + 0.6) * unit,
                        (eh + 0.6) * unit,
                    )
                } else {
                    (
                        cx + (ex + agy_look_x) * unit + body_offset_x,
                        feet_y + (ey + agy_look_y) * unit + body_offset_y,
                        ew * unit,
                        eh * unit,
                    )
                };

                if let Some(r) = tiny_skia::Rect::from_xywh(px, py, pw, ph) {
                    pixmap.fill_rect(r, &eye_paint, tiny_skia::Transform::identity(), None);
                }
            }
        }
    }

}
