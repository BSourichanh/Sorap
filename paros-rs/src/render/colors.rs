use crate::pet::{MascotKind, Pet};

pub const EYE: (u8, u8, u8, u8) = (31, 30, 29, 255);
pub const PAPER: (u8, u8, u8, u8) = (255, 253, 248, 255);
pub const HEART: (u8, u8, u8, u8) = (255, 93, 115, 255);
pub const GOLD: (u8, u8, u8, u8) = (242, 194, 48, 255);
pub const SWEAT: (u8, u8, u8, u8) = (127, 208, 255, 255);
pub const SMOKE: (u8, u8, u8, u8) = (168, 168, 168, 255);
pub const WOOD: (u8, u8, u8, u8) = (138, 90, 51, 255);
pub const MANILA: (u8, u8, u8, u8) = (232, 201, 138, 255);
pub const MANILA_DARK: (u8, u8, u8, u8) = (187, 161, 110, 255);
pub const FLAME: (u8, u8, u8, u8) = (255, 138, 42, 255);
pub const LIGHT: (u8, u8, u8, u8) = (255, 237, 115, 76);
pub const CLAUDE: (u8, u8, u8, u8) = (217, 119, 87, 255);
pub const AGY: (u8, u8, u8, u8) = (74, 129, 240, 255);
pub const SAGE_PURPLE: (u8, u8, u8, u8) = (75, 58, 140, 255);
pub const SAGE_DARK: (u8, u8, u8, u8) = (61, 47, 122, 255);
pub const MAILBOX_BLUE: (u8, u8, u8, u8) = (59, 111, 212, 255);
pub const MAILBOX_DARK: (u8, u8, u8, u8) = (43, 79, 154, 255);
pub const STRAW: (u8, u8, u8, u8) = (217, 178, 74, 255);
pub const SIGNAL: (u8, u8, u8, u8) = (93, 255, 138, 255);

pub fn lighten_color((r, g, b, a): (u8, u8, u8, u8), amount: f32) -> (u8, u8, u8, u8) {
    let lr = (r as f32 + (255.0 - r as f32) * amount).round().clamp(0.0, 255.0) as u8;
    let lg = (g as f32 + (255.0 - g as f32) * amount).round().clamp(0.0, 255.0) as u8;
    let lb = (b as f32 + (255.0 - b as f32) * amount).round().clamp(0.0, 255.0) as u8;
    (lr, lg, lb, a)
}


pub fn darken_color((r, g, b, a): (u8, u8, u8, u8), amount: f32) -> (u8, u8, u8, u8) {
    let dr = (r as f32 * (1.0 - amount)).round().clamp(0.0, 255.0) as u8;
    let dg = (g as f32 * (1.0 - amount)).round().clamp(0.0, 255.0) as u8;
    let db = (b as f32 * (1.0 - amount)).round().clamp(0.0, 255.0) as u8;
    (dr, dg, db, a)
}


pub fn lerp_color(c1: (u8, u8, u8, u8), c2: (u8, u8, u8, u8), t: f32) -> (u8, u8, u8, u8) {
    let clamped_t = t.clamp(0.0, 1.0);
    let r = (c1.0 as f32 + (c2.0 as f32 - c1.0 as f32) * clamped_t).round() as u8;
    let g = (c1.1 as f32 + (c2.1 as f32 - c1.1 as f32) * clamped_t).round() as u8;
    let b = (c1.2 as f32 + (c2.2 as f32 - c1.2 as f32) * clamped_t).round() as u8;
    let a = (c1.3 as f32 + (c2.3 as f32 - c1.3 as f32) * clamped_t).round() as u8;
    (r, g, b, a)
}


pub fn get_pet_body_color(pet: &Pet) -> (u8, u8, u8, u8) {
    let base = if let Some((r, g, b)) = pet.custom_color {
        (r, g, b, 255)
    } else {
        match pet.kind {
            MascotKind::Claude => CLAUDE,
            MascotKind::Agy => AGY,
        }
    };
    if pet.is_small {
        lighten_color(base, 0.15)
    } else {
        base
    }
}


pub fn get_hat_room(pet: &Pet) -> f32 {
    if pet.kind == MascotKind::Agy {
        0.0
    } else if pet.is_sage {
        7.5
    } else if pet.hard_hat {
        3.5
    } else {
        match pet.accessory {
            1 => 4.0, // TopHat
            2 => 3.5, // Crown
            3 => 2.0, // Cap
            4 => 2.0, // Bow
            5 => 0.0, // Glasses
            6 => 3.5, // Flower
            _ => 0.0,
        }
    }
}

