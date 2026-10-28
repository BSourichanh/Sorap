use crate::pet::MascotKind;
use super::colors::*;
use super::Renderer;

impl Renderer {
    pub fn draw_tag(&self, pixmap: &mut tiny_skia::PixmapMut, label: &str, cx: f32, top_y: f32, edge_color: (u8, u8, u8, u8)) {
        let Some(font) = self.font.as_ref() else {
            return;
        };

        let truncated: String;
        let label = if label.chars().count() > super::LABEL_MAX_LENGTH {
            truncated = label.chars().take(super::LABEL_MAX_LENGTH).collect();
            &truncated
        } else {
            label
        };

        let font_size = 11.0f32;
        let mut text_width = 0.0f32;
        for ch in label.chars() {
            text_width += font.metrics(ch, font_size).advance_width;
        }

        let padding_h = 5.0f32;
        let box_w = text_width + padding_h * 2.0;
        let box_h = font_size + 4.0;

        let bx = cx - box_w / 2.0;
        let by = top_y;

        let mut bg_paint = tiny_skia::Paint::default();
        bg_paint.set_color_rgba8(EYE.0, EYE.1, EYE.2, 255);
        if let Some(r) = tiny_skia::Rect::from_xywh(bx, by, box_w, box_h) {
            pixmap.fill_rect(r, &bg_paint, tiny_skia::Transform::identity(), None);
        }

        let mut edge_paint = tiny_skia::Paint::default();
        edge_paint.set_color_rgba8(edge_color.0, edge_color.1, edge_color.2, 255);
        if let Some(r) = tiny_skia::Rect::from_xywh(bx, by, 4.0, box_h) {
            pixmap.fill_rect(r, &edge_paint, tiny_skia::Transform::identity(), None);
        }

        let cur_x = bx + 7.0;
        let text_y = by + font_size;
        self.draw_text_line(pixmap, label, cur_x, text_y, font_size, PAPER);
    }


    pub fn draw_activity_caption(
        &self,
        pixmap: &mut tiny_skia::PixmapMut,
        caption: &str,
        cx: f32,
        bottom_y: f32,
        edge_color: (u8, u8, u8, u8),
    ) {
        let Some(font) = self.font.as_ref() else {
            return;
        };
        let truncated: String;
        let caption = if caption.chars().count() > super::CAPTION_MAX_LENGTH {
            truncated = caption.chars().take(super::CAPTION_MAX_LENGTH).collect();
            &truncated
        } else {
            caption
        };
        let font_size = 11.0f32;
        let max_w = 280.0f32;

        let mut lines = Vec::new();
        for raw_line in caption.lines() {
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
                } else if cur_w + space_w + word_w <= max_w {
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

        let line_height = font_size + 3.0;
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

        let padding_h = 5.0f32;
        let box_w = (max_line_w + padding_h * 2.0).min(max_w + 10.0);
        let box_h = lines.len() as f32 * line_height + 4.0;

        let bx = cx - box_w / 2.0;
        let by = (bottom_y - box_h).max(4.0);

        let mut bg_paint = tiny_skia::Paint::default();
        bg_paint.set_color_rgba8(EYE.0, EYE.1, EYE.2, 255);
        if let Some(r) = tiny_skia::Rect::from_xywh(bx, by, box_w, box_h) {
            pixmap.fill_rect(r, &bg_paint, tiny_skia::Transform::identity(), None);
        }

        let mut edge_paint = tiny_skia::Paint::default();
        edge_paint.set_color_rgba8(edge_color.0, edge_color.1, edge_color.2, 255);
        if let Some(r) = tiny_skia::Rect::from_xywh(bx, by, 4.0, box_h) {
            pixmap.fill_rect(r, &edge_paint, tiny_skia::Transform::identity(), None);
        }

        for (i, line) in lines.iter().enumerate() {
            let start_x = bx + 7.0;
            let baseline_y = by + (i + 1) as f32 * line_height;
            self.draw_text_line(pixmap, line, start_x, baseline_y, font_size, PAPER);
        }
    }


    pub fn draw_bubble(&self, pixmap: &mut tiny_skia::PixmapMut, text: &str, cx: f32, top_y: f32) {
        let Some(font) = self.font.as_ref() else {
            return;
        };

        let font_size = 14.0f32;
        let max_text_width = 264.0f32;

        let mut lines = Vec::new();
        for raw_line in text.lines() {
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
        let line_height = font_size + 4.0f32;

        let bubble_w = max_line_w + padding_h * 2.0;
        let bubble_h = font_size + (lines.len() - 1) as f32 * line_height + padding_v * 2.0;

        let bx = (cx - bubble_w / 2.0).round();
        let by = (top_y - bubble_h - border - tail).round().max(border);

        let mut ink_paint = tiny_skia::Paint::default();
        ink_paint.set_color_rgba8(EYE.0, EYE.1, EYE.2, 255);

        // Queue (tail)
        if let Some(tail_r) = tiny_skia::Rect::from_xywh(cx - tail / 2.0, by + bubble_h, tail, border + tail) {
            pixmap.fill_rect(tail_r, &ink_paint, tiny_skia::Transform::identity(), None);
        }

        // Bordure (border) : grow(BORDER)
        if let Some(border_r) = tiny_skia::Rect::from_xywh(bx - border, by - border, bubble_w + border * 2.0, bubble_h + border * 2.0) {
            pixmap.fill_rect(border_r, &ink_paint, tiny_skia::Transform::identity(), None);
        }

        // Fond papier (paper background)
        let mut paper_paint = tiny_skia::Paint::default();
        paper_paint.set_color_rgba8(PAPER.0, PAPER.1, PAPER.2, 255);
        if let Some(rect) = tiny_skia::Rect::from_xywh(bx, by, bubble_w, bubble_h) {
            pixmap.fill_rect(rect, &paper_paint, tiny_skia::Transform::identity(), None);
        }

        // Lignes de texte
        for (i, line) in lines.iter().enumerate() {
            let cur_x = bx + padding_h;
            let text_y = by + padding_v + font_size - 1.0 + i as f32 * line_height;
            self.draw_text_line(pixmap, line, cur_x, text_y, font_size, EYE);
        }
    }


    pub fn draw_small_caption(
        &self,
        pixmap: &mut tiny_skia::PixmapMut,
        caption: &str,
        cx: f32,
        top_y: f32,
        kind: MascotKind,
    ) {
        let Some(font) = self.font.as_ref() else {
            return;
        };

        let font_size = 11.0f32;
        let max_caption_width = 110.0f32;

        let mut text = caption.to_string();
        let mut text_width = 0.0f32;
        for ch in text.chars() {
            text_width += font.metrics(ch, font_size).advance_width;
        }

        while text_width > max_caption_width && text.len() > 1 {
            text.pop();
            let candidate = format!("{}…", text.trim_end());
            let mut w = 0.0f32;
            for ch in candidate.chars() {
                w += font.metrics(ch, font_size).advance_width;
            }
            if w <= max_caption_width || text.is_empty() {
                text = candidate;
                text_width = w;
                break;
            }
        }

        let padding_h = 5.0f32;
        let box_w = text_width + padding_h * 2.0;
        let box_h = font_size + 4.0;

        let bx = cx - box_w / 2.0;
        let by = (top_y - box_h).max(4.0);

        let mut bg_paint = tiny_skia::Paint::default();
        bg_paint.set_color_rgba8(EYE.0, EYE.1, EYE.2, 255);
        if let Some(r) = tiny_skia::Rect::from_xywh(bx, by, box_w, box_h) {
            pixmap.fill_rect(r, &bg_paint, tiny_skia::Transform::identity(), None);
        }

        let edge_color = match kind {
            MascotKind::Claude => CLAUDE,
            MascotKind::Agy => AGY,
        };
        let mut edge_paint = tiny_skia::Paint::default();
        edge_paint.set_color_rgba8(edge_color.0, edge_color.1, edge_color.2, 255);
        if let Some(r) = tiny_skia::Rect::from_xywh(bx, by, 3.5, box_h) {
            pixmap.fill_rect(r, &edge_paint, tiny_skia::Transform::identity(), None);
        }

        let cur_x = bx + 6.0;
        let text_y = by + font_size;
        self.draw_text_line(pixmap, &text, cur_x, text_y, font_size, PAPER);
    }


    #[inline]
    fn blend_pixel(dst: &mut tiny_skia::PremultipliedColorU8, color: (u8, u8, u8, u8), coverage: u8) {
        let src_a = (coverage as u32 * color.3 as u32) / 255;
        if src_a == 0 {
            return;
        }
        let src_r = (color.0 as u32 * src_a) / 255;
        let src_g = (color.1 as u32 * src_a) / 255;
        let src_b = (color.2 as u32 * src_a) / 255;

        let inv_a = 255 - src_a;
        let out_r = (src_r + (dst.red() as u32 * inv_a + 127) / 255).min(255) as u8;
        let out_g = (src_g + (dst.green() as u32 * inv_a + 127) / 255).min(255) as u8;
        let out_b = (src_b + (dst.blue() as u32 * inv_a + 127) / 255).min(255) as u8;
        let out_a = (src_a + (dst.alpha() as u32 * inv_a + 127) / 255).min(255) as u8;

        if let Some(c) = tiny_skia::PremultipliedColorU8::from_rgba(out_r, out_g, out_b, out_a) {
            *dst = c;
        }
    }

    pub fn draw_text_line(
        &self,
        pixmap: &mut tiny_skia::PixmapMut,
        text: &str,
        start_x: f32,
        baseline_y: f32,
        font_size: f32,
        color: (u8, u8, u8, u8),
    ) {
        let p_width = pixmap.width() as i32;
        let p_height = pixmap.height() as i32;
        let pixels = pixmap.pixels_mut();

        let mut cur_x = start_x;
        for ch in text.chars() {
            let adv = self.with_glyph(ch, font_size, |metrics, bitmap| {
                let gx = (cur_x + metrics.xmin as f32).round() as i32;
                let gy = (baseline_y - metrics.height as f32 - metrics.ymin as f32).round() as i32;

                for row in 0..metrics.height {
                    let py = gy + row as i32;
                    if py < 0 || py >= p_height {
                        continue;
                    }
                    let row_offset = py as usize * p_width as usize;
                    let bitmap_row_offset = row * metrics.width;

                    for col in 0..metrics.width {
                        let alpha = bitmap[bitmap_row_offset + col];
                        if alpha == 0 {
                            continue;
                        }
                        let px = gx + col as i32;
                        if px >= 0 && px < p_width {
                            Self::blend_pixel(&mut pixels[row_offset + px as usize], color, alpha);
                        }
                    }
                }
                metrics.advance_width
            });

            if let Some(w) = adv {
                cur_x += w;
            } else if let Some(font) = self.font.as_ref() {
                cur_x += font.metrics(ch, font_size).advance_width;
            }
        }
    }

}
