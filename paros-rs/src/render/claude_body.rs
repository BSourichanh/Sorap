#![allow(clippy::too_many_arguments)]

use crate::pet::{Pet, PetState};
use super::colors::*;
use super::Renderer;

impl Renderer {
    pub fn draw_block(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        offset_x: f32,
        offset_y: f32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: (u8, u8, u8, u8),
        sided: bool,
        facing: f32,
    ) {
        let rx = if sided && facing < 0.0 { -(x + w) } else { x };
        let px = cx + rx * unit + offset_x;
        let py = feet_y + y * unit + offset_y;
        let pw = w * unit;
        let ph = h * unit;

        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(color.0, color.1, color.2, color.3);
        if let Some(r) = tiny_skia::Rect::from_xywh(px, py, pw, ph) {
            pixmap.fill_rect(r, &paint, tiny_skia::Transform::identity(), None);
        }
    }


    pub fn draw_claude(
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
        body_color: (u8, u8, u8, u8),
    ) {
        let mut terracotta = tiny_skia::Paint::default();
        terracotta.set_color_rgba8(body_color.0, body_color.1, body_color.2, body_color.3);

        let time = pet.time;
        let pace = pet.pace;

        let seated = pet.meditating
            || matches!(
                pet.state,
                PetState::Sit | PetState::Sleep | PetState::Roast | PetState::Wait | PetState::Compact
            );

        let front_leg = if pet.facing > 0.0 { 3 } else { 0 };

        // Pattes (masquées si assis / méditation / endormi / compact / roast)
        if !seated {
            let leg_cols = [-5.0f32, -3.0, 2.0, 4.0];
            let leg_top = feet_y - 2.0 * unit + body_offset_y;

            for (i, &col) in leg_cols.iter().enumerate() {
                let mut lx = cx + col * unit + body_offset_x;
                let mut bottom = feet_y - hop;

                if is_airborne {
                    lx += (time * 6.0 + i as f32).sin() * 2.0;
                } else if pet.state == PetState::Walk {
                    let step = (time * 9.0 * pace + (i % 2) as f32 * std::f32::consts::PI).sin();
                    bottom -= step.max(0.0) * unit * 0.8;
                } else if pet.state == PetState::Think && pet.tapping && i == front_leg {
                    bottom -= (time * 12.0).sin().max(0.0) * unit * 0.7;
                }

                let h = (bottom - leg_top).max(unit * 0.2);
                if let Some(r) = tiny_skia::Rect::from_xywh(lx, leg_top, unit, h) {
                    pixmap.fill_rect(r, &terracotta, tiny_skia::Transform::identity(), None);
                }
            }
        }

        // Corps
        if let Some(r) = tiny_skia::Rect::from_xywh(
            cx - 6.0 * unit + body_offset_x,
            feet_y - 10.0 * unit + body_offset_y,
            12.0 * unit,
            8.0 * unit,
        ) {
            pixmap.fill_rect(r, &terracotta, tiny_skia::Transform::identity(), None);
        }

        // Bras
        if let Some(r) = tiny_skia::Rect::from_xywh(
            cx - 8.0 * unit + body_offset_x,
            feet_y - 6.0 * unit + body_offset_y - arm_raise[0] * unit,
            2.0 * unit,
            2.0 * unit,
        ) {
            pixmap.fill_rect(r, &terracotta, tiny_skia::Transform::identity(), None);
        }
        if let Some(r) = tiny_skia::Rect::from_xywh(
            cx + 6.0 * unit + body_offset_x,
            feet_y - 6.0 * unit + body_offset_y - arm_raise[1] * unit,
            2.0 * unit,
            2.0 * unit,
        ) {
            pixmap.fill_rect(r, &terracotta, tiny_skia::Transform::identity(), None);
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
            eye_paint.set_color_rgba8(eye_color.0, eye_color.1, eye_color.2, eye_color.3);

            let eyes = [
                (-4.0f32, -8.0f32, 1.0f32, 2.0f32),
                (3.0f32, -8.0f32, 1.0f32, 2.0f32),
            ];

            for &(ex, ey, ew, eh) in &eyes {
                let (px, py, pw, ph) = if closed {
                    let center_y = ey + eh / 2.0;
                    (
                        cx + (ex + look_x - 0.25) * unit + body_offset_x,
                        feet_y + (center_y - 0.2) * unit + body_offset_y,
                        1.5 * unit,
                        0.4 * unit,
                    )
                } else if pet.state == PetState::Glare {
                    (
                        cx + (ex + look_x - 0.2) * unit + body_offset_x,
                        feet_y + (ey + look_y + 1.0) * unit + body_offset_y,
                        1.4 * unit,
                        1.0 * unit,
                    )
                } else if wide {
                    (
                        cx + (ex + look_x - 0.3) * unit + body_offset_x,
                        feet_y + (ey + look_y - 0.3) * unit + body_offset_y,
                        (ew + 0.6) * unit,
                        (eh + 0.6) * unit,
                    )
                } else {
                    (
                        cx + (ex + look_x) * unit + body_offset_x,
                        feet_y + (ey + look_y) * unit + body_offset_y,
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
