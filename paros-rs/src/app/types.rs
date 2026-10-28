use std::time::Instant;

pub const SAGE_GAP: f32 = 230.0;
pub const SAGE_ENTRY: f32 = 460.0;
pub const SAGE_HURRY: f32 = 2.0;
pub const SAGE_MIN_SECS: f32 = 5.0;
pub const SAGE_LEAVE_SECS: f32 = 30.0;
pub const COMPACT_MIN_SECS: f32 = 6.0;
pub const COMPACT_MAX_SECS: f32 = 900.0;
pub const MEET_DISTANCE: f32 = 150.0;
pub const MEET_COOLDOWN: f32 = 45.0;
pub const HIGH_FIVE_DISTANCE: f32 = 200.0;
pub const HIGH_FIVE_SECS: f32 = 20.0;
pub const KNOCK_AFTER_SECS: f32 = 20.0;
pub const KNOCK_REPEAT_SECS: f32 = 30.0;
pub const KNOCK_REACH: f32 = 72.0;
pub const TOWER_LEVEL: f32 = 72.0;
pub const TOWER_WIDTH: f32 = 200.0;
pub const TOWER_AFTER_SECS: f32 = 90.0;
pub const CUDDLE_GAP: f32 = 70.0;
pub const CUDDLE_IDLE_SECS: f32 = 60.0;
pub const PYRAMID_STEP: f32 = 160.0;
pub const PYRAMID_LEVEL: f32 = 72.0;
pub const PYRAMID_HEAD: f32 = 200.0;
pub const PYRAMID_HOLD_SECS: f32 = 8.0;
pub const PYRAMID_BUILD_SECS: f32 = 25.0;
pub const PYRAMID_CHANCE: f32 = 0.006;
pub const VISIT_GAP: f32 = 200.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusPhase {
    Off,
    Focus(f32),
    Break(f32),
}

#[derive(Debug, Clone)]
pub struct SageTracker {
    pub since: Instant,
    pub spawn_time: Instant,
    pub done: Option<Instant>,
    pub side: f32,
    pub leaving: bool,
    pub gone_time: Option<Instant>,
}

pub use super::compacting::CompactingTracker;

#[derive(Debug, Clone)]
pub struct SubagentPyramid {
    pub members: Vec<String>,
    pub spots: Vec<(f32, usize)>,
    pub middle: f32,
    pub floor: f32,
    pub deadline: Instant,
    pub until: Option<Instant>,
}

pub fn url_decode(s: &str) -> String {
    let mut bytes_out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len()
            && let Ok(val) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16) {
                bytes_out.push(val);
                i += 3;
                continue;
            }
        bytes_out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&bytes_out).into_owned()
}

pub fn hash_str(s: &str) -> usize {
    s.bytes().fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_decode_utf8() {
        let input = "file:///home/user/T%C3%A9l%C3%A9chargements/test%20file.txt";
        let expected = "file:///home/user/Téléchargements/test file.txt";
        assert_eq!(url_decode(input), expected);
    }

    #[test]
    fn test_accessory_modulo_seven_distribution() {
        assert_eq!(KNOCK_REACH, 72.0);
        let names = ["main", "dev", "fix/bug", "feat/ai", "paros", "test", "session-1", "session-2"];
        let mut accessories = std::collections::HashSet::new();
        for name in &names {
            let acc = hash_str(name) % 7;
            assert!(acc < 7);
            accessories.insert(acc);
        }
        // Accessory 0 is valid (no accessory)
        assert!(accessories.len() > 1);
    }
}
