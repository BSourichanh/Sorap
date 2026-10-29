#![allow(clippy::too_many_arguments)]

use crate::letter::FlyingLetter;
use super::colors::*;
use super::Renderer;

impl Renderer {
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

    pub(crate) fn draw_hover_card(
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
            if !cur_line.is_empty() {
                lines.push(cur_line);
            }
        }

        if lines.is_empty() {
            return;
        }

        let line_height = font_size + 4.5f32;
        let mut max_line_w = 0.0f32;
        for line in &lines {
            let mut w = 0.0f32;
            for ch in line.chars() {
                w += font.metrics(ch, font_size).advance_width;
            }
            if w > max_line_w {
                max_line_w = w;
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

        if let Some(border_r) = tiny_skia::Rect::from_xywh(card_x - border, card_y - border, card_w + border * 2.0, card_h + border * 2.0) {
            pixmap.fill_rect(border_r, &ink_paint, tiny_skia::Transform::identity(), None);
        }

        let mut paper_paint = tiny_skia::Paint::default();
        paper_paint.set_color_rgba8(PAPER.0, PAPER.1, PAPER.2, 255);
        if let Some(r) = tiny_skia::Rect::from_xywh(card_x, card_y, card_w, card_h) {
            pixmap.fill_rect(r, &paper_paint, tiny_skia::Transform::identity(), None);
        }

        for (i, line) in lines.iter().enumerate() {
            let start_x = card_x + padding_h;
            let baseline_y = card_y + padding_v + (i + 1) as f32 * line_height - 3.0;
            self.draw_text_line(pixmap, line, start_x, baseline_y, font_size, EYE);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_context_menu<S: AsRef<str>>(
        &self,
        pixmap: &mut tiny_skia::PixmapMut,
        menu_x: f32,
        menu_y: f32,
        menu_w: f32,
        menu_h: f32,
        items: &[S],
        hovered_idx: Option<usize>,
        separator_after: Option<usize>,
    ) {
        let mut bg_paint = tiny_skia::Paint::default();
        bg_paint.set_color_rgba8(EYE.0, EYE.1, EYE.2, 248);

        if let Some(r) = tiny_skia::Rect::from_xywh(menu_x, menu_y, menu_w, menu_h) {
            pixmap.fill_rect(r, &bg_paint, tiny_skia::Transform::identity(), None);
        }

        let mut border_paint = tiny_skia::Paint::default();
        border_paint.set_color_rgba8(CLAUDE.0, CLAUDE.1, CLAUDE.2, 255);
        let stroke = tiny_skia::Stroke {
            width: 1.5,
            ..Default::default()
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
                if let Some(hr) = tiny_skia::Rect::from_xywh(menu_x + 3.0, item_y, menu_w - 6.0, item_height) {
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
