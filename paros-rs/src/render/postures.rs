#![allow(clippy::too_many_arguments)]

use super::colors::*;
use super::Renderer;

impl Renderer {
    pub fn draw_campfire(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
        time: f32,
        state_time: f32,
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

    pub fn draw_hourglass(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        facing: f32,
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
            if let Some(r) = tiny_skia::Rect::from_xywh(
                -1.8 * unit,
                (side * (HOURGLASS_HALF + 0.25) - 0.25) * unit,
                3.6 * unit,
                0.5 * unit,
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

    pub fn draw_broom(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        facing: f32,
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

    pub fn draw_compaction(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
        state_time: f32,
        time: f32,
        strained: bool,
    ) {
        const BIN_X: f32 = 11.0;
        const BALL: f32 = 1.2;
        const COMPACT_CYCLE: f32 = 2.4;
        const COMPACT_PULL: f32 = 0.3;
        const COMPACT_CRUMPLE: f32 = 0.55;
        const PAPER_BALLS: [(f32, f32); 3] = [
            (BIN_X - 1.3, -4.6), (BIN_X, -4.8), (BIN_X - 0.6, -5.6),
        ];

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
                    Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 4.0, -10.0 - line, 1.6, 0.3, (170, 170, 170, 255), true, facing);
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

    pub fn draw_lost_map(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 6.5, -8.5, 3.0, 2.4, PAPER, true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 7.0, -7.8, 2.0, 0.4, (70, 167, 88, 255), true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 7.6, -7.1, 1.2, 0.4, HEART, true, facing);
    }

    pub fn draw_lost_question_mark(
        &self,
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        color: (u8, u8, u8, u8),
    ) {
        let qx = cx + body_offset_x - 5.0;
        let qy = feet_y - 10.0 * unit + body_offset_y - 8.0;
        self.draw_text_line(pixmap, "?", qx, qy, 20.0 * (unit / 9.0), color);
    }
}
