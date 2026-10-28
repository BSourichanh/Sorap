#![allow(clippy::too_many_arguments)]

use crate::pet::MascotKind;
use super::colors::*;
use super::Renderer;

impl Renderer {
    pub fn draw_accessories(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
        accessory: usize,
    ) {
        match accessory {
            1 => {
                // Haut-de-forme (Top Hat)
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -4.5, -11.0, 9.0, 1.0, EYE, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -3.0, -14.0, 6.0, 3.0, EYE, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -3.0, -12.0, 6.0, 1.0, HEART, false, facing);
            }
            2 => {
                // Couronne (Crown)
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -4.0, -11.5, 8.0, 1.5, GOLD, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -4.0, -13.0, 1.5, 1.5, GOLD, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -0.75, -13.5, 1.5, 2.0, GOLD, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 2.5, -13.0, 1.5, 1.5, GOLD, false, facing);
            }
            3 => {
                // Casquette (Cap)
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -4.0, -12.0, 8.0, 2.0, (59, 111, 212, 255), false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 3.0, -11.0, 4.5, 1.0, (43, 79, 154, 255), false, facing);
            }
            4 => {
                // Nœud papillon (Bow)
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 1.5, -12.0, 1.5, 2.0, HEART, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 4.0, -12.0, 1.5, 2.0, HEART, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 3.0, -11.5, 1.0, 1.0, (201, 58, 85, 255), false, facing);
            }
            5 => {
                // Lunettes (Glasses)
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -5.9, -8.8, 4.2, 0.5, EYE, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -5.9, -5.7, 4.2, 0.5, EYE, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -5.9, -8.8, 0.5, 3.6, EYE, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -2.2, -8.8, 0.5, 3.6, EYE, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 1.7, -8.8, 4.2, 0.5, EYE, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 1.7, -5.7, 4.2, 0.5, EYE, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 1.7, -8.8, 0.5, 3.6, EYE, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 5.4, -8.8, 0.5, 3.6, EYE, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -1.7, -7.6, 3.4, 0.5, EYE, false, facing);
            }
            6 => {
                // Fleur (Flower)
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -0.25, -12.5, 0.5, 2.5, (70, 167, 88, 255), false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -1.25, -14.0, 2.5, 2.0, HEART, false, facing);
                Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -0.5, -13.5, 1.0, 1.0, GOLD, false, facing);
            }
            _ => {}
        }
    }


    pub fn draw_hard_hat(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -4.0, -12.5, 8.0, 2.5, GOLD, false, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -5.0, -10.5, 10.0, 0.8, (201, 154, 18, 255), false, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -0.5, -13.0, 1.0, 3.0, (255, 226, 122, 255), false, facing);
    }


    pub fn draw_construction_sign(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        _facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, 8.7, -6.5, 0.5, 6.5, (138, 138, 138, 255), false, 1.0);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, 7.4, -9.6, 3.1, 3.1, GOLD, false, 1.0);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, 8.7, -9.1, 0.5, 1.4, EYE, false, 1.0);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, 8.7, -7.4, 0.5, 0.5, EYE, false, 1.0);
    }


    #[allow(dead_code)]
    pub fn draw_hard_hat_and_sign(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
        _is_airborne: bool,
        pet_kind: MascotKind,
    ) {
        Self::draw_construction_sign(pixmap, cx, feet_y, unit, facing);
        if pet_kind != MascotKind::Agy {
            Self::draw_hard_hat(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, facing);
        }
    }


    pub fn draw_baggage(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
        baggage_count: usize,
        kind: MascotKind,
    ) {
        let x_shift = if kind == MascotKind::Agy { 2.2 } else { 0.0 };
        for i in 0..baggage_count.min(10) {
            let base_y = -7.3 - i as f32 * 1.4;
            let folder_x = -8.8 + x_shift;
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, folder_x, base_y, 2.8, 1.2, MANILA, true, facing);
            Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, folder_x, base_y - 0.3, 1.2, 0.3, MANILA_DARK, true, facing);
        }
    }


    pub fn draw_umbrella(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -0.25, -15.5, 0.5, 5.5, EYE, false, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -6.5, -16.5, 13.0, 1.2, HEART, false, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -5.0, -17.5, 10.0, 1.0, HEART, false, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -3.0, -18.3, 6.0, 0.8, HEART, false, facing);
    }


    pub fn draw_sheet(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -3.0, -15.5, 6.0, 4.0, PAPER, false, facing);
    }


    pub fn draw_read_sheet(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 6.5, -9.0, 3.0, 3.4, PAPER, true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 7.0, -8.3, 2.0, 0.3, SMOKE, true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 7.0, -7.5, 2.0, 0.3, SMOKE, true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 7.0, -6.7, 1.3, 0.3, SMOKE, true, facing);
    }


    pub fn draw_sage_beard(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -4.0, -5.6, 8.0, 1.4, PAPER, false, 1.0);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -3.2, -4.2, 6.4, 1.4, PAPER, false, 1.0);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -2.0, -2.8, 4.0, 1.4, PAPER, false, 1.0);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -1.0, -1.4, 2.0, 1.0, PAPER, false, 1.0);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -0.5, -5.6, 1.0, 0.5, SMOKE, false, 1.0);
    }


    pub fn draw_sage_staff(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, -8.7, -13.2, 0.7, 13.2, WOOD, true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, -9.3, -15.0, 1.9, 1.9, SWEAT, true, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, -8.9, -14.6, 0.6, 0.6, PAPER, true, facing);
    }


    pub fn draw_sage_hat(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -5.5, -11.0, 11.0, 1.0, SAGE_DARK, false, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -4.0, -12.5, 8.0, 1.5, SAGE_PURPLE, false, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -3.0, -14.0, 6.0, 1.5, SAGE_PURPLE, false, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -2.0, -15.5, 4.0, 1.5, SAGE_PURPLE, false, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -1.0, -17.0, 2.0, 1.5, SAGE_PURPLE, false, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -2.4, -12.2, 0.8, 0.8, GOLD, false, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 0.8, -14.9, 0.7, 0.7, GOLD, false, facing);
    }


    pub fn draw_mailbox(
        &self,
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        side: f32,
        mail_count: usize,
        discreet: bool,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, -11.9, -4.5, 0.7, 4.5, WOOD, true, -side);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, -13.6, -7.5, 4.1, 3.0, MAILBOX_BLUE, true, -side);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, -13.6, -7.9, 4.1, 0.4, MAILBOX_DARK, true, -side);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, -12.6, -7.2, 2.1, 0.7, PAPER, true, -side);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, -13.0, -6.6, 2.9, 0.4, EYE, true, -side);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, -9.5, -9.4, 0.4, 2.9, HEART, true, -side);
        Self::draw_block(pixmap, cx, feet_y, unit, 0.0, 0.0, -10.7, -9.4, 1.2, 0.9, HEART, true, -side);

        if mail_count > 1 && !discreet {
            let tag_x = cx + side * 11.5 * unit;
            let tag_y = feet_y - 12.0 * unit;
            let count_str = mail_count.to_string();
            self.draw_tag(pixmap, &count_str, tag_x, tag_y, HEART);
        }
    }


    pub fn draw_headlamp(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
        facing: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -6.0, -9.5, 12.0, 0.5, EYE, false, facing);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 4.4, -9.9, 1.4, 1.3, GOLD, true, facing);
    }


    pub fn draw_shades(
        pixmap: &mut tiny_skia::PixmapMut,
        cx: f32,
        feet_y: f32,
        unit: f32,
        body_offset_x: f32,
        body_offset_y: f32,
    ) {
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -5.6, -8.2, 4.2, 2.4, EYE, false, 1.0);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 1.4, -8.2, 4.2, 2.4, EYE, false, 1.0);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -1.4, -7.8, 2.8, 0.6, EYE, false, 1.0);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, -5.0, -7.8, 1.0, 0.5, PAPER, false, 1.0);
        Self::draw_block(pixmap, cx, feet_y, unit, body_offset_x, body_offset_y, 2.0, -7.8, 1.0, 0.5, PAPER, false, 1.0);
    }

}
