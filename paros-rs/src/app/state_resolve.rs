use std::collections::HashMap;
use crate::pet::MascotKind;
use crate::session::types::SessionState;

pub fn resolve_session_from_map(
    sessions: &HashMap<String, SessionState>,
    target_spec: &str,
    sender_id: &str,
) -> Option<String> {
    let cleaned = target_spec.split(" [").next().unwrap_or(target_spec).trim();
    if cleaned.is_empty() {
        return None;
    }
    let (kind, is_folder, filter) = if cleaned == "claude" || cleaned == "agy" {
        (Some(cleaned), true, "")
    } else if let Some((k, f)) = cleaned.split_once(':') {
        (Some(k), false, f)
    } else if let Some((k, f)) = cleaned.split_once('@') {
        (Some(k), true, f)
    } else {
        (None, false, cleaned)
    };

    let mut matches = Vec::new();
    for (id, s) in sessions {
        if id == sender_id {
            continue;
        }
        if let Some(k) = kind {
            let match_kind = match s.kind {
                MascotKind::Claude => k == "claude",
                MascotKind::Agy => k == "agy",
            };
            if !match_kind {
                continue;
            }
        }
        if is_folder {
            if filter.is_empty() || s.cwd.contains(filter) {
                matches.push(id.clone());
            }
        } else if !filter.is_empty() {
            if s.name == filter
                || (kind == Some("agy") && id.starts_with(filter))
                || (kind.is_none() && id.starts_with(filter))
            {
                return Some(id.clone());
            }
        } else {
            matches.push(id.clone());
        }
    }

    if matches.len() == 1 {
        Some(matches.remove(0))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_session_named_syntax() {
        let mut map = HashMap::new();
        map.insert(
            "s1".into(),
            SessionState::new("s1".into(), MascotKind::Claude, "paros-35".into(), "/work/paros".into(), None),
        );
        map.insert(
            "c6bdd622-affe".into(),
            SessionState::new("c6bdd622-affe".into(), MascotKind::Agy, "General Discussion".into(), "/work/agy".into(), None),
        );

        assert_eq!(resolve_session_from_map(&map, "agy:c6bdd622", "s1"), Some("c6bdd622-affe".into()));
        assert_eq!(resolve_session_from_map(&map, "agy:General Discussion", "s1"), Some("c6bdd622-affe".into()));
        assert_eq!(resolve_session_from_map(&map, "claude:paros-35", "c6bdd622-affe"), Some("s1".into()));
        assert_eq!(resolve_session_from_map(&map, "claude:nonexistent", "s1"), None);
        assert_eq!(resolve_session_from_map(&map, "claude:paros", "c6bdd622-affe"), None);

        assert_eq!(resolve_session_from_map(&map, "agy@work/agy", "s1"), Some("c6bdd622-affe".into()));
        assert_eq!(resolve_session_from_map(&map, "claude@work/paros", "c6bdd622-affe"), Some("s1".into()));
        assert_eq!(resolve_session_from_map(&map, "claude@other", "s1"), None);

        assert_eq!(resolve_session_from_map(&map, "agy", "s1"), Some("c6bdd622-affe".into()));
        assert_eq!(resolve_session_from_map(&map, "claude", "c6bdd622-affe"), Some("s1".into()));
        assert_eq!(resolve_session_from_map(&map, "paros-35", "c6bdd622-affe"), Some("s1".into()));
        assert_eq!(resolve_session_from_map(&map, "c6bdd622", "s1"), Some("c6bdd622-affe".into()));
    }
}
