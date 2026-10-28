#![allow(clippy::too_many_arguments)]

#[derive(Debug, Clone)]
pub struct FlyingLetter {
    pub from_x: f32,
    pub from_y: f32,
    pub current_x: f32,
    pub current_y: f32,
    pub angle: f32,
    pub time: f32,
    pub duration: f32,
    pub arc: f32,
    pub seal_color: (u8, u8, u8, u8),
    pub scale: f32,
    pub target_is_sage: bool,
    pub target_id: String,
    pub finished: bool,
}

impl FlyingLetter {
    pub const SPEED: f32 = 700.0;
    pub const SECONDS_MIN: f32 = 0.7;
    pub const SECONDS_MAX: f32 = 1.8;
    pub const ARC_SHARE: f32 = 0.25;
    pub const ARC_MAX: f32 = 220.0;
    pub const HAND_HEIGHT: f32 = 70.0;
    pub const ROCK: f32 = 0.18;

    pub fn seal_color_from_pet(pet: &crate::pet::Pet) -> (u8, u8, u8, u8) {
        crate::render::colors::get_pet_body_color(pet)
    }

    pub fn stop(&mut self) {
        self.finished = true;
    }

    #[allow(dead_code)]
    pub fn new(
        from_x: f32,
        from_feet_y: f32,
        from_scale: f32,
        to_x: f32,
        to_feet_y: f32,
        to_scale: f32,
        seal_color: (u8, u8, u8, u8),
        target_is_sage: bool,
    ) -> Self {
        Self::new_with_target(
            from_x,
            from_feet_y,
            from_scale,
            to_x,
            to_feet_y,
            to_scale,
            seal_color,
            target_is_sage,
            String::new(),
        )
    }

    pub fn new_with_target(
        from_x: f32,
        from_feet_y: f32,
        from_scale: f32,
        to_x: f32,
        to_feet_y: f32,
        to_scale: f32,
        seal_color: (u8, u8, u8, u8),
        target_is_sage: bool,
        target_id: String,
    ) -> Self {
        let start_y = from_feet_y - Self::HAND_HEIGHT * from_scale;
        let end_y = to_feet_y - Self::HAND_HEIGHT * to_scale;

        let dx = to_x - from_x;
        let dy = end_y - start_y;
        let dist = (dx * dx + dy * dy).sqrt();

        let duration =
            (dist / (Self::SPEED * to_scale)).clamp(Self::SECONDS_MIN, Self::SECONDS_MAX);
        let arc = (dist * Self::ARC_SHARE).min(Self::ARC_MAX * to_scale);

        Self {
            from_x,
            from_y: start_y,
            current_x: from_x,
            current_y: start_y,
            angle: 0.0,
            time: 0.0,
            duration,
            arc,
            seal_color,
            scale: to_scale,
            target_is_sage,
            target_id,
            finished: false,
        }
    }

    pub fn update(&mut self, dt: f32, to_x: f32, to_feet_y: f32, to_scale: f32) -> bool {
        if self.finished {
            return false;
        }

        self.time += dt;
        self.scale = to_scale;
        let progress = (self.time / self.duration).min(1.0);

        let target_y = to_feet_y - Self::HAND_HEIGHT * to_scale;
        let dx = to_x - self.from_x;
        let dy = target_y - self.from_y;
        let distance = (dx * dx + dy * dy).sqrt();
        self.arc = (distance * Self::ARC_SHARE).min(Self::ARC_MAX * to_scale);

        // Interpolation en arc de cercle parabolique
        let base_x = self.from_x + (to_x - self.from_x) * progress;
        let base_y = self.from_y + (target_y - self.from_y) * progress;
        let arc_offset = (progress * std::f32::consts::PI).sin() * self.arc;

        self.current_x = base_x;
        self.current_y = base_y - arc_offset;

        // Oscillation angulaire de l'enveloppe
        self.angle = (self.time * 9.0).sin() * Self::ROCK;

        if progress >= 1.0 {
            self.finished = true;
            return true; // Atterrissage
        }

        false
    }
}
