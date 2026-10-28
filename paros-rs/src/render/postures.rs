#![allow(clippy::too_many_arguments)]

use super::colors::*;
use super::Renderer;

impl Renderer {
    pub fn draw_campfire(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32,
        body_offset_x: f32, body_offset_y: f32, facing: f32, time: f32, state_time: f32,
    ) {
        const CAMPFIRE_X: f32 = 10.0;
        // Bûches de bois
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, CAMPFIRE_X - 1.5, -0.8, 3.6, 0.8, WOOD, true, facing);

        // Flammes animées
        for i in 0..3 {
            let height = 1.4 + (time * 7.0 + i as f32 * 2.1).sin().abs() * 1.6;
            let flame_col = if i == 1 { GOLD } else { FLAME };
            Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, CAMPFIRE_X - 1.1 + i as f32, -0.8 - height, 1.0, height, flame_col, true, facing);
        }

        // Bâton et chamallow qui brunit progressivement (PAPER -> WOOD)
        let done = (state_time / 7.0).clamp(0.0, 1.0);
        let marshmallow_col = lerp_color(PAPER, WOOD, done * 0.8);

        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 7.5, -6.2, CAMPFIRE_X - 7.2, 0.3, WOOD, true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, CAMPFIRE_X - 0.1, -6.8, 1.3, 1.3, marshmallow_col, true, facing);
    }


    pub fn draw_hourglass(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32, facing: f32,
        time: f32,
    ) {
        const HOURGLASS_X: f32 = 10.5;
        const HOURGLASS_HALF: f32 = 2.6;
        const HOURGLASS_ROWS: [f32; 4] = [0.3, 0.7, 1.1, 1.4];
        const HOURGLASS_RUN: f32 = 5.0;
        const HOURGLASS_TURN: f32 = 0.7;

        let moment = time % (HOURGLASS_RUN + HOURGLASS_TURN);
        let run = (moment / HOURGLASS_RUN).min(1.0);
        let turn = ((moment - HOURGLASS_RUN).max(0.0) / HOURGLASS_TURN).min(1.0);

        let center_x = cx + HOURGLASS_X * facing * unit;
        let center_y = feet_y + (-HOURGLASS_HALF - 0.5) * unit;

        let transform = tiny_skia::Transform::from_translate(center_x, center_y)
            .pre_rotate(turn * 180.0);

        let mut wood_paint = tiny_skia::Paint::default();
        wood_paint.set_color_rgba8(WOOD.0, WOOD.1, WOOD.2, WOOD.3);

        let mut paper_paint = tiny_skia::Paint::default();
        paper_paint.set_color_rgba8(PAPER.0, PAPER.1, PAPER.2, 217);

        let mut gold_paint = tiny_skia::Paint::default();
        gold_paint.set_color_rgba8(GOLD.0, GOLD.1, GOLD.2, GOLD.3);

        let row_h = HOURGLASS_HALF / HOURGLASS_ROWS.len() as f32;

        for &side in &[-1.0f32, 1.0f32] {
            // Socle en bois
            if let Some(r) = tiny_skia::Rect::from_xywh(-1.8 * unit,
                (side * (HOURGLASS_HALF + 0.25) - 0.25) * unit, 3.6 * unit, 0.5 * unit,
            ) {
                pixmap.fill_rect(r, &wood_paint, transform, None);
            }

            let (sand_x, sand_y) = if side < 0.0 {
                (0.0, (1.0 - run) * HOURGLASS_HALF)
            } else {
                ((1.0 - run) * HOURGLASS_HALF, HOURGLASS_HALF)
            };

            for (i, &half_w) in HOURGLASS_ROWS.iter().enumerate() {
                let from = i as f32 * row_h;
                let to = from + row_h;

                let y_top = if side > 0.0 { from } else { -to };
                let h = to - from;
                if let Some(r) = tiny_skia::Rect::from_xywh(-half_w * unit, y_top * unit, half_w * 2.0 * unit, h * unit) {
                    pixmap.fill_rect(r, &paper_paint, transform, None);
                }

                let s_from = from.max(sand_x);
                let s_to = to.min(sand_y);
                if s_to > s_from {
                    let s_top = if side > 0.0 { s_from } else { -s_to };
                    let s_h = s_to - s_from;
                    if let Some(r) = tiny_skia::Rect::from_xywh(-half_w * unit, s_top * unit, half_w * 2.0 * unit, s_h * unit) {
                        pixmap.fill_rect(r, &gold_paint, transform, None);
                    }
                }
            }
        }

        // Filet de sable qui tombe
        if run > 0.0 && run < 1.0
            && let Some(r) = tiny_skia::Rect::from_xywh(-0.1 * unit, 0.0, 0.2 * unit, (1.0 - run) * HOURGLASS_HALF * unit)
        {
            pixmap.fill_rect(r, &gold_paint, transform, None);
        }
    }


    pub fn draw_broom(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32, facing: f32,
        state_time: f32,
    ) {
        let stroke = (state_time / 2.2 * std::f32::consts::TAU * 3.0).sin();
        let x = 8.6 + stroke * 1.4;
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, x, -7.5, 0.4, 6.5, WOOD, true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, x - 0.9, -1.3, 2.2, 1.3, STRAW, true, facing);

        let sign = if stroke >= 0.0 { 1.0 } else { -1.0 };
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, x + 2.0 * sign, -1.0 - stroke.abs(), 0.6, 0.6, SMOKE, true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, x + 3.0 * sign, -0.6 - stroke.abs() * 1.8, 0.5, 0.5, SMOKE, true, facing);
    }


    pub fn draw_compaction(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32,
        body_offset_x: f32, body_offset_y: f32, facing: f32, state_time: f32, time: f32, strained: bool,
    ) {
        const BIN_X: f32 = 11.0;
        const BALL: f32 = 1.2;
        const COMPACT_CYCLE: f32 = 2.4;
        const COMPACT_PULL: f32 = 0.3;
        const COMPACT_CRUMPLE: f32 = 0.55;
        const PAPER_BALLS: [(f32, f32); 3] = [
            (BIN_X - 1.3, -4.6), (BIN_X, -4.8), (BIN_X - 0.6, -5.6), ];

        let thrown = ((state_time / COMPACT_CYCLE) as usize).min(PAPER_BALLS.len());
        for &(bx, by) in &PAPER_BALLS[..thrown] {
            Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, bx, by, BALL, BALL, PAPER, true, facing);
        }

        // Corbeille grillagée BIN
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, BIN_X - 1.5, -3.2, 3.0, 3.2, (140, 140, 140, 255), true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, BIN_X - 1.9, -3.6, 3.8, 0.5, (94, 94, 94, 255), true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, BIN_X - 0.8, -3.1, 0.3, 3.0, (110, 110, 110, 255), true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, BIN_X + 0.5, -3.1, 0.3, 3.0, (110, 110, 110, 255), true, facing);

        let moment = (state_time % COMPACT_CYCLE) / COMPACT_CYCLE;
        if moment < COMPACT_PULL {
            let out = moment / COMPACT_PULL * 3.2;
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 3.6, -10.0 - out, 2.4, out, PAPER, true, facing);
            for line in [0.8f32, 1.6f32] {
                if out > line + 0.4 {
                    Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 4.0, -10.0 - out + line, 1.6, 0.3, SMOKE, true, facing);
                }
            }
        } else if moment < COMPACT_CRUMPLE {
            let t = (moment - COMPACT_PULL) / (COMPACT_CRUMPLE - COMPACT_PULL);
            let size = 2.6 + (BALL - 2.6) * t;
            let hx = 8.5;
            let hy = -6.5;
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, hx - size / 2.0, hy - size / 2.0, size, size, PAPER, true, facing);
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, hx - 0.4, hy - 0.15, 0.6, 0.3, SMOKE, true, facing);
        } else {
            let flight = (moment - COMPACT_CRUMPLE) / (1.0 - COMPACT_CRUMPLE);
            let hand_x = 8.5;
            let hand_y = -6.5;
            let bin_top_x = BIN_X;
            let bin_top_y = -3.6;
            let at_x = hand_x + (bin_top_x - hand_x) * flight;
            let at_y = hand_y + (bin_top_y - hand_y) * flight - (flight * std::f32::consts::PI).sin() * 4.0;
            let off_x = body_offset_x * (1.0 - flight);
            let off_y = body_offset_y * (1.0 - flight);
            Self::draw_block(pixmap, cx, feet_y, unit, off_x, off_y, at_x - BALL / 2.0, at_y - BALL / 2.0, BALL, BALL, PAPER, true, facing);
        }

        if strained {
            let drop_y = -10.5 + (time % 1.0) * 1.5;
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -7.4, drop_y, 0.8, 1.6, SWEAT, true, facing);
        }
    }


    pub fn draw_lost_map(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32,
        body_offset_x: f32, body_offset_y: f32, facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 6.5, -8.5, 3.0, 2.4, PAPER, true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 7.0, -7.8, 2.0, 0.4, (70, 167, 88, 255), true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 7.6, -7.1, 1.2, 0.4, HEART, true, facing);
    }


    pub fn draw_lost_question_mark(&self, pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32,
        body_offset_x: f32, body_offset_y: f32, color: (u8, u8, u8, u8),
    ) {
        let qx = cx + body_offset_x - 5.0;
        let qy = feet_y - 10.0 * unit + body_offset_y - 8.0;
        self.draw_text_line(pixmap, "?", qx, qy, 20.0 * (unit / 9.0), color);
    }


    pub fn draw_glare_cross(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32,
        body_offset_x: f32, body_offset_y: f32, facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 5.0, -13.0, 2.4, 0.6, HEART, true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 5.9, -13.9, 0.6, 2.4, HEART, true, facing);
    }


    pub fn draw_high_five_sparks(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32,
        body_offset_x: f32, body_offset_y: f32, facing: f32, time: f32,
    ) {
        if (time % 0.2) < 0.12 {
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 8.5, -15.0, 1.0, 1.0, GOLD, true, facing);
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 10.0, -13.5, 0.8, 0.8, GOLD, true, facing);
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 9.6, -16.6, 0.8, 0.8, GOLD, true, facing);
        }
    }


    pub fn draw_worry_sweat(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32,
        body_offset_x: f32, body_offset_y: f32, _facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 6.6, -10.5, 0.8, 1.6, SWEAT, false, 1.0);
    }


    pub fn draw_shake_sparks(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32,
        body_offset_x: f32, body_offset_y: f32, time: f32,
    ) {
        if (time % 0.3) < 0.2 {
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -7.5, -11.5, 0.8, 0.8, GOLD, false, 1.0);
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 6.8, -12.2, 0.8, 0.8, GOLD, false, 1.0);
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -0.4, -13.2, 0.8, 0.8, GOLD, false, 1.0);
        }
    }


    pub fn draw_mantra(&self, pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32,
        body_offset_x: f32, body_offset_y: f32, time: f32, in_front: bool,
    ) {
        let mantra = ["π", "∑", "√", "∞"];
        let center_x = cx + body_offset_x;
        let center_y = feet_y - 6.0 * unit + body_offset_y;

        for (i, &sym) in mantra.iter().enumerate() {
            let angle = time * 1.1 + std::f32::consts::TAU * i as f32 / 4.0;
            let sin_val = angle.sin();
            if (sin_val >= 0.0) != in_front {
                continue;
            }
            let at_x = center_x + angle.cos() * 9.5 * unit;
            let at_y = center_y + sin_val * 2.2 * unit;
            let depth = sin_val * 0.5 + 0.5;
            let font_size = (11.0 + depth * 7.0).max(9.0) * (unit / 9.0);
            let alpha = ((0.45 + depth * 0.55) * 255.0) as u8;
            let color = (GOLD.0, GOLD.1, GOLD.2, alpha);
            self.draw_text_line(pixmap, sym, at_x, at_y, font_size, color);
        }
    }


    pub fn draw_smoke(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32, body_offset_x: f32,
        body_offset_y: f32, time: f32,
    ) {
        for i in 0..4 {
            let phase = (time * 0.5 + i as f32 * 0.25) % 1.0;
            let size = (0.7 + phase * 0.9) * unit;
            let x = cx + body_offset_x + (i as f32 - 1.5) * 16.0 + (time * 2.0 + i as f32 * 1.3).sin() * 5.0;
            let y = feet_y - 10.0 * unit + body_offset_y - 4.0 - phase * 34.0;

            let alpha = ((phase * std::f32::consts::PI).sin() * 0.8 * 255.0) as u8;
            let mut paint = tiny_skia::Paint::default();
            paint.set_color_rgba8(SMOKE.0, SMOKE.1, SMOKE.2, alpha);

            if let Some(r) = tiny_skia::Rect::from_xywh(x - size / 2.0, y - size / 2.0, size, size) {
                pixmap.fill_rect(r, &paint, tiny_skia::Transform::identity(), None);
            }
        }
    }


    #[allow(dead_code)]
    pub fn draw_focus_badge(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32,
        body_offset_x: f32, body_offset_y: f32, _secs_left: f32,
    ) {
        let badge_x = cx + body_offset_x;
        let badge_y = feet_y - 15.0 * unit + body_offset_y;

        let mut apple_paint = tiny_skia::Paint::default();
        apple_paint.set_color_rgba8(235, 60, 60, 255);
        if let Some(r) = tiny_skia::Rect::from_xywh(badge_x - 1.2 * unit, badge_y, 2.4 * unit, 2.2 * unit) {
            pixmap.fill_rect(r, &apple_paint, tiny_skia::Transform::identity(), None);
        }

        let mut stem_paint = tiny_skia::Paint::default();
        stem_paint.set_color_rgba8(WOOD.0, WOOD.1, WOOD.2, 255);
        if let Some(r) = tiny_skia::Rect::from_xywh(badge_x - 0.2 * unit, badge_y - 0.6 * unit, 0.4 * unit, 0.6 * unit) {
            pixmap.fill_rect(r, &stem_paint, tiny_skia::Transform::identity(), None);
        }

        let mut leaf_paint = tiny_skia::Paint::default();
        leaf_paint.set_color_rgba8(70, 180, 80, 255);
        if let Some(r) = tiny_skia::Rect::from_xywh(badge_x + 0.2 * unit, badge_y - 0.5 * unit, 0.7 * unit, 0.4 * unit) {
            pixmap.fill_rect(r, &leaf_paint, tiny_skia::Transform::identity(), None);
        }
    }


    pub fn draw_headlamp_beam(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32,
        body_offset_x: f32, body_offset_y: f32, facing: f32,
    ) {
        let mut pb = tiny_skia::PathBuilder::new();
        let lamp_x = cx + 5.8 * facing * unit + body_offset_x;
        let lamp_y = feet_y - 9.2 * unit + body_offset_y;
        let far_x = cx + 16.0 * facing * unit;
        let far_y = feet_y + 0.4 * unit;
        let near_x = cx + 8.5 * facing * unit;
        let near_y = feet_y + 0.4 * unit;

        pb.move_to(lamp_x, lamp_y);
        pb.line_to(far_x, far_y);
        pb.line_to(near_x, near_y);
        pb.close();

        if let Some(path) = pb.finish() {
            let mut paint = tiny_skia::Paint::default();
            paint.set_color_rgba8(LIGHT.0, LIGHT.1, LIGHT.2, LIGHT.3);
            pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, tiny_skia::Transform::identity(), None);
        }
    }


    pub fn draw_notes(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32, body_offset_x: f32,
        body_offset_y: f32, time: f32, body_color: (u8, u8, u8, u8),
    ) {
        const NOTE_ROWS: &[&str] = &[
            "..XXX", "..X..", "..X..", "XXX..", "XXX..", ];

        let phase = (time * 0.5) % 1.0;
        let side = if ((time * 0.5) as usize).is_multiple_of(2) { 1.0 } else { -1.0 };
        let nx = cx + body_offset_x + side * (6.0 + 2.5) * unit + (time * 3.0).sin() * 3.0;
        let ny = feet_y - 10.0 * unit + body_offset_y - phase * 26.0;

        let alpha = ((phase * std::f32::consts::PI).sin() * 255.0) as u8;
        let note_col = darken_color(body_color, 0.2);
        let mut note_paint = tiny_skia::Paint::default();
        note_paint.set_color_rgba8(note_col.0, note_col.1, note_col.2, alpha);

        let pixel = 3.0f32;
        for (r_idx, row) in NOTE_ROWS.iter().enumerate() {
            for (c_idx, ch) in row.chars().enumerate() {
                if ch == 'X' {
                    let px = nx + (c_idx as f32 - 2.5) * pixel;
                    let py = ny + (r_idx as f32 - 2.5) * pixel;
                    if let Some(rect) = tiny_skia::Rect::from_xywh(px, py, pixel, pixel) {
                        pixmap.fill_rect(rect, &note_paint, tiny_skia::Transform::identity(), None);
                    }
                }
            }
        }
    }


    #[allow(clippy::too_many_arguments)]
    pub fn draw_snore(&self, pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32, body_offset_x: f32,
        body_offset_y: f32, time: f32, body_color: (u8, u8, u8, u8),
    ) {
        for i in 0..3 {
            let phase = (time * 0.4 + i as f32 / 3.0) % 1.0;
            let zx = cx + body_offset_x + 6.0 * unit - 10.0 + phase * 20.0;
            let zy = feet_y - 10.0 * unit + body_offset_y - 6.0 - phase * 45.0;

            let alpha = ((phase * std::f32::consts::PI).sin() * 255.0) as u8;
            let snore_col = darken_color(body_color, 0.2);
            let font_size = 12.0 + phase * 12.0;
            let color = (snore_col.0, snore_col.1, snore_col.2, alpha);

            self.draw_text_line(pixmap, "z", zx, zy, font_size, color);
        }
    }


    pub fn draw_alert(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32, body_offset_x: f32,
        body_offset_y: f32, time: f32,
    ) {
        if (time % 0.6) < 0.4 {
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -0.5, -15.5, 1.0, 2.5,
                HEART, false, 1.0, );
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -0.5, -12.5, 1.0, 1.0,
                HEART, false, 1.0, );
        }
    }


    pub fn draw_think(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32, body_offset_x: f32,
        body_offset_y: f32, time: f32, body_color: (u8, u8, u8, u8),
    ) {
        let count = ((time * 2.5) as usize) % 4;
        for i in 0..count {
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -2.4 + i as f32 * 2.0,
                -12.5, 0.8, 0.8, body_color, false, 1.0, );
        }
    }


    pub fn draw_hearts(pixmap: &mut tiny_skia::PixmapMut, cx: f32, feet_y: f32, unit: f32, body_offset_x: f32,
        body_offset_y: f32, time: f32,
    ) {
        const HEART_ROWS: &[&str] = &[
            ".XX.XX.", "XXXXXXX", "XXXXXXX", ".XXXXX.", "..XXX..", "...X...", ];

        for i in 0..3 {
            let phase = (time * 0.9 + i as f32 / 3.0) % 1.0;
            let hx = cx + body_offset_x + (i as f32 - 1.0) * 40.0 + (time * 4.0 + i as f32).sin() * 4.0;
            let hy = feet_y - 10.0 * unit + body_offset_y - 16.0 - phase * 36.0;

            let alpha = ((phase * std::f32::consts::PI).sin() * 255.0) as u8;
            let mut paint = tiny_skia::Paint::default();
            paint.set_color_rgba8(HEART.0, HEART.1, HEART.2, alpha);

            let pixel = 3.0f32;
            for (r_idx, row) in HEART_ROWS.iter().enumerate() {
                for (c_idx, ch) in row.chars().enumerate() {
                    if ch == 'X' {
                        let px = hx + (c_idx as f32 - 3.5) * pixel;
                        let py = hy + (r_idx as f32 - 3.0) * pixel;
                        if let Some(rect) = tiny_skia::Rect::from_xywh(px, py, pixel, pixel) {
                            pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                        }
                    }
                }
            }
        }
    }
}
