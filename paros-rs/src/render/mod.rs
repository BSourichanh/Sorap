use std::cell::RefCell;
use std::collections::HashMap;

pub mod accessories;
pub mod agy_body;
pub mod claude_body;
pub mod colors;
pub mod draw_pet;
pub mod overlays;
pub mod particles;
pub mod postures;
pub mod text;

#[allow(unused_imports)]
pub use accessories::*;
#[allow(unused_imports)]
pub use agy_body::*;
#[allow(unused_imports)]
pub use claude_body::*;
#[allow(unused_imports)]
pub use colors::*;
#[allow(unused_imports)]
pub use draw_pet::*;
#[allow(unused_imports)]
pub use overlays::*;
#[allow(unused_imports)]
pub use particles::*;
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
}
