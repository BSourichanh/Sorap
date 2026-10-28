#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MascotKind {
    Claude,
    Agy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetState {
    Idle,
    Sit,
    Walk,
    Climb,
    Fall,
    Carried,
    Sleep,
    Stretch,
    Think,
    Alert,
    Cheer,
    Greet,
    Glare,
    HighFive,
    Worry,
    Roast,
    Knock,
    Sweep,
    Wait,
    Throw,
    Read,
    Nod,
    Compact,
    Shake,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetWish {
    Roam,
    Sleep,
    Think,
    Alert,
    Wait,
    Read,
    Compact,
}

impl PetWish {
    pub const fn target_state(self) -> Option<PetState> {
        match self {
            Self::Roam => None,
            Self::Sleep => Some(PetState::Sleep),
            Self::Think => Some(PetState::Think),
            Self::Alert => Some(PetState::Alert),
            Self::Wait => Some(PetState::Wait),
            Self::Read => Some(PetState::Read),
            Self::Compact => Some(PetState::Compact),
        }
    }
}
