#![allow(clippy::too_many_arguments)]

use super::colors::*;
use super::Renderer;

impl Renderer {
    pub fn draw_glare_cross(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 5.0, -13.0, 2.4, 0.6, HEART, true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 5.9, -13.9, 0.6, 2.4, HEART, true, facing);
    }

    pub fn draw_high_five_sparks(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
        time: f32,
    ) {
        if (time % 0.2) < 0.12 {
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 8.5, -15.0, 1.0, 1.0, GOLD, true, facing);
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 10.0, -13.5, 0.8, 0.8, GOLD, true, facing);
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 9.6, -16.6, 0.8, 0.8, GOLD, true, facing);
        }
    }

    pub fn draw_worry_sweat(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        _facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 6.6, -10.5, 0.8, 1.6, SWEAT, false, 1.0);
    }

    pub fn draw_shake_sparks(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        time: f32,
    ) {
        if (time % 0.3) < 0.2 {
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -7.5, -11.5, 0.8, 0.8, GOLD, false, 1.0);
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 6.8, -12.2, 0.8, 0.8, GOLD, false, 1.0);
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -0.4, -13.2, 0.8, 0.8, GOLD, false, 1.0);
        }
    }

    pub fn draw_mantra(
        &self,
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        time: f32,
        in_front: bool,
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

    pub fn draw_smoke(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        time: f32,
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
    pub fn draw_focus_badge(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        _secs_left: f32,
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

    pub fn draw_headlamp_beam(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
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

    pub fn draw_notes(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        time: f32,
        body_color: (u8, u8, u8, u8),
    ) {
        const NOTE_ROWS: &[&str] = &[
            "..XXX", "..X..", "..X..", "XXX..", "XXX..",
        ];

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
    pub fn draw_snore(
        &self,
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        time: f32,
        body_color: (u8, u8, u8, u8),
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

    pub fn draw_alert(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        time: f32,
    ) {
        if (time % 0.6) < 0.4 {
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -0.5, -15.5, 1.0, 2.5, HEART, false, 1.0);
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -0.5, -12.5, 1.0, 1.0, HEART, false, 1.0);
        }
    }

    pub fn draw_think(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        time: f32,
        body_color: (u8, u8, u8, u8),
    ) {
        let count = ((time * 2.5) as usize) % 4;
        for i in 0..count {
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -2.4 + i as f32 * 2.0, -12.5, 0.8, 0.8, body_color, false, 1.0);
        }
    }

    pub fn draw_hearts(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        time: f32,
    ) {
        const HEART_ROWS: &[&str] = &[
            ".XX.XX.", "XXXXXXX", "XXXXXXX", ".XXXXX.", "..XXX..", "...X...",
        ];

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
