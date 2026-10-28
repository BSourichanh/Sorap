#![allow(dead_code, clippy::too_many_arguments)]

#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub mon_name: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub hovered: Option<usize>,
    pub focus_timer_active: bool,
    pub items_data: Vec<(ContextMenuAction, String)>,
    pub separator_after: Option<usize>,
    pub target_pet_id: Option<String>,
    pub parent_session: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuAction {
    FocusTerminal,
    RingTerminal,
    ToggleFocusTimer,
    OpenSettings,
    Quit,
}

impl ContextMenuAction {
    /// Conforme à Godot (focus.gd:29 / desktop.gd:254) : aucun son Knock lors du démarrage Pomodoro ou ring terminal
    #[inline]
    pub fn should_play_knock(&self) -> bool {
        false
    }
}

impl ContextMenu {
    pub const WIDTH: f32 = 250.0;
    pub const HEIGHT: f32 = 140.0;
    pub const ITEM_HEIGHT: f32 = 26.0;
    pub const ITEM_COUNT: usize = 5;

    /// Indique si le démarrage du minuteur Pomodoro doit émettre un son Knock (aligné sur focus.gd:29 : false)
    pub const PLAY_KNOCK_ON_POMODORO: bool = false;

    /// Indique si la sonnerie du terminal doit émettre un son Knock (aligné sur desktop.gd:254 : false)
    pub const PLAY_KNOCK_ON_RING: bool = false;

    /// Résout la session parente d'un pet_id (ex: sous-agent "parent/subagent" -> "parent", sage -> session_id).
    /// Aligné sur context_menu.gd:26 et brain.gd:178.
    pub fn resolve_parent_session(pet_id: &str) -> &str {
        pet_id.split('/').next().unwrap_or(pet_id)
    }

    /// Détermine si un identifiant de mascotte (normale, sous-agent ou sage) correspond à une session active
    pub fn has_session_for_pet<F>(pet_id: Option<&str>, mut is_session_active: F) -> bool
    where
        F: FnMut(&str) -> bool,
    {
        match pet_id {
            Some(id) if !id.is_empty() => {
                let parent = Self::resolve_parent_session(id);
                is_session_active(parent)
            }
            _ => false,
        }
    }

    pub fn new(
        x: f32,
        y: f32,
        mon_w: f32,
        mon_h: f32,
        has_session: bool,
        focus_remaining_min: Option<u32>,
        break_remaining_min: Option<u32>,
        lang: crate::i18n::Lang,
        focus_default_min: u32,
    ) -> Self {
        Self::new_with_mon(
            String::new(),
            x,
            y,
            mon_w,
            mon_h,
            has_session,
            focus_remaining_min,
            break_remaining_min,
            lang,
            focus_default_min,
        )
    }

    pub fn new_with_mon(
        mon_name: String,
        x: f32,
        y: f32,
        mon_w: f32,
        mon_h: f32,
        has_session: bool,
        focus_remaining_min: Option<u32>,
        break_remaining_min: Option<u32>,
        lang: crate::i18n::Lang,
        focus_default_min: u32,
    ) -> Self {
        Self::new_with_mon_and_pet(
            mon_name,
            x,
            y,
            mon_w,
            mon_h,
            None,
            has_session,
            focus_remaining_min,
            break_remaining_min,
            lang,
            focus_default_min,
        )
    }

    pub fn new_for_pet<F>(
        mon_name: String,
        x: f32,
        y: f32,
        mon_w: f32,
        mon_h: f32,
        pet_id: Option<&str>,
        is_session_active: F,
        focus_remaining_min: Option<u32>,
        break_remaining_min: Option<u32>,
        lang: crate::i18n::Lang,
        focus_default_min: u32,
    ) -> Self
    where
        F: FnMut(&str) -> bool,
    {
        let has_session = Self::has_session_for_pet(pet_id, is_session_active);
        Self::new_with_mon_and_pet(
            mon_name,
            x,
            y,
            mon_w,
            mon_h,
            pet_id,
            has_session,
            focus_remaining_min,
            break_remaining_min,
            lang,
            focus_default_min,
        )
    }

    pub fn new_with_mon_and_pet(
        mon_name: String,
        x: f32,
        y: f32,
        mon_w: f32,
        mon_h: f32,
        pet_id: Option<&str>,
        has_session: bool,
        focus_remaining_min: Option<u32>,
        break_remaining_min: Option<u32>,
        lang: crate::i18n::Lang,
        focus_default_min: u32,
    ) -> Self {
        let parent_session = pet_id.map(Self::resolve_parent_session).map(str::to_string);
        let mut items_data = Vec::new();

        if has_session {
            items_data.push((
                ContextMenuAction::FocusTerminal,
                crate::i18n::tr(lang, "Go to its terminal").to_string(),
            ));
            items_data.push((
                ContextMenuAction::RingTerminal,
                crate::i18n::tr(lang, "Ring its terminal").to_string(),
            ));
        }

        let focus_timer_active = focus_remaining_min.is_some() || break_remaining_min.is_some();
        let focus_label = if let Some(min) = break_remaining_min {
            crate::i18n::tr(lang, "Stop the break (%d min left)").replace("%d", &min.to_string())
        } else if let Some(min) = focus_remaining_min {
            crate::i18n::tr(lang, "Stop the focus (%d min left)").replace("%d", &min.to_string())
        } else {
            crate::i18n::tr(lang, "Start a focus (%d min)").replace("%d", &focus_default_min.to_string())
        };
        items_data.push((ContextMenuAction::ToggleFocusTimer, focus_label));
        let focus_idx = items_data.len() - 1;

        items_data.push((
            ContextMenuAction::OpenSettings,
            crate::i18n::tr(lang, "Settings…").to_string(),
        ));
        items_data.push((
            ContextMenuAction::Quit,
            crate::i18n::tr(lang, "Quit").to_string(),
        ));

        let height = items_data.len() as f32 * Self::ITEM_HEIGHT + 10.0;
        let clamped_x = x.clamp(10.0, (mon_w - Self::WIDTH - 10.0).max(10.0));
        let clamped_y = y.clamp(10.0, (mon_h - height - 10.0).max(10.0));

        Self {
            mon_name,
            x: clamped_x,
            y: clamped_y,
            width: Self::WIDTH,
            height,
            hovered: None,
            focus_timer_active,
            items_data,
            separator_after: Some(focus_idx),
            target_pet_id: pet_id.map(str::to_string),
            parent_session,
        }
    }

    pub fn new_full(
        mon_name: String,
        x: f32,
        y: f32,
        mon_w: f32,
        mon_h: f32,
        has_session: bool,
        focus_remaining_min: Option<u32>,
        break_remaining_min: Option<u32>,
        lang: crate::i18n::Lang,
    ) -> Self {
        Self::new_with_mon(
            mon_name,
            x,
            y,
            mon_w,
            mon_h,
            has_session,
            focus_remaining_min,
            break_remaining_min,
            lang,
            25,
        )
    }

    pub fn contains(&self, mon_name: &str, x: f32, y: f32) -> bool {
        (self.mon_name.is_empty() || self.mon_name == mon_name)
            && x >= self.x
            && x <= self.x + self.width
            && y >= self.y
            && y <= self.y + self.height
    }

    pub fn update_hover(&mut self, _x: f32, y: f32) -> bool {
        let rel_y = y - (self.y + 5.0);
        let new_hover = if rel_y >= 0.0 {
            let idx = (rel_y / Self::ITEM_HEIGHT) as usize;
            if idx < self.items_data.len() {
                Some(idx)
            } else {
                None
            }
        } else {
            None
        };

        if self.hovered != new_hover {
            self.hovered = new_hover;
            true
        } else {
            false
        }
    }

    pub fn get_action_at(&self, y: f32) -> Option<ContextMenuAction> {
        let rel_y = y - (self.y + 5.0);
        if rel_y < 0.0 {
            return None;
        }
        let idx = (rel_y / Self::ITEM_HEIGHT) as usize;
        self.items_data.get(idx).map(|(action, _)| *action)
    }

    pub fn items(&self) -> Vec<&str> {
        self.items_data.iter().map(|(_, label)| label.as_str()).collect()
    }
}
