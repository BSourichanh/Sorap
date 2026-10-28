#![allow(dead_code)]

use crate::pet::Pet;
use crate::session::{SessionPhase, SessionState};

pub fn build_session_card(session: &SessionState, focus_timer: Option<f32>) -> String {
    build_session_card_full(session, focus_timer, None, None, None, None, crate::i18n::detect_lang(""))
}

pub fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{} s", seconds)
    } else if seconds < 3600 {
        format!("{} min", seconds / 60)
    } else {
        format!("{} h {:02}", seconds / 3600, (seconds % 3600) / 60)
    }
}

pub fn build_session_card_full(
    session: &SessionState,
    focus_timer: Option<f32>,
    break_timer: Option<f32>,
    sage_elapsed: Option<u64>,
    awaited_peer: Option<(&str, u64)>,
    compact_elapsed: Option<u64>,
    lang: crate::i18n::Lang,
) -> String {
    let mut lines = Vec::new();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    let short_cwd = session.cwd.replace(&home, "~");
    let display_cwd = if short_cwd.chars().count() > 38 {
        let suffix: String = short_cwd.chars().rev().take(37).collect();
        format!("…{}", suffix.chars().rev().collect::<String>())
    } else {
        short_cwd
    };
    lines.push(display_cwd);

    let elapsed_secs = session.start_time.elapsed().as_secs();
    let dur_str = format_duration(elapsed_secs);

    match session.phase {
        SessionPhase::Working => {
            lines.push(crate::i18n::tr(lang, "Working for %s").replace("%s", &dur_str));
            if let Some(secs) = sage_elapsed {
                lines.push(crate::i18n::tr(lang, "Asks the sage for %s").replace("%s", &format_duration(secs)));
            } else if !session.activity.is_empty() {
                if !session.detail.is_empty() {
                    lines.push(format!("{} · {}", session.activity, session.detail));
                } else {
                    lines.push(session.activity.clone());
                }
            }
        }
        SessionPhase::Waiting => {
            lines.push(crate::i18n::tr(lang, "Waiting for you for %s").replace("%s", &dur_str));
            if !session.detail.is_empty() {
                lines.push(session.detail.clone());
            }
        }
        SessionPhase::Idle => {
            if let Some((peer_name, secs)) = awaited_peer {
                lines.push(
                    crate::i18n::tr(lang, "Waiting for the answer of %s for %s")
                        .replacen("%s", peer_name, 1)
                        .replacen("%s", &format_duration(secs), 1),
                );
            } else if session.has_background {
                lines.push(crate::i18n::tr(lang, "Waiting for a background task for %s").replace("%s", &dur_str));
            } else {
                lines.push(crate::i18n::tr(lang, "At rest for %s").replace("%s", &dur_str));
            }
        }
    }

    if let Some(secs) = compact_elapsed {
        lines.push(crate::i18n::tr(lang, "Compacting the context for %s").replace("%s", &format_duration(secs)));
    } else if session.compacting {
        lines.push(crate::i18n::tr(lang, "Compacting the context").to_string());
    }

    if !session.subagents.is_empty() {
        lines.push(crate::i18n::tr(lang, "Subagents running: %d").replace("%d", &session.subagents.len().to_string()));
        for sub in &session.subagents {
            let act = if !sub.detail.is_empty() {
                format!("{} · {}", sub.tool, sub.detail)
            } else {
                sub.tool.clone()
            };
            if !sub.name.is_empty() && !act.is_empty() {
                lines.push(format!("  {}: {}", sub.name, act));
            } else if !sub.name.is_empty() {
                lines.push(format!("  {}", sub.name));
            } else if !act.is_empty() {
                lines.push(format!("  {}", act));
            }
        }
    } else if session.subagents_count > 0 {
        lines.push(crate::i18n::tr(lang, "Subagents running: %d").replace("%d", &session.subagents_count.to_string()));
    }

    // Serveurs si disponibles (pour session_servers)
    if session.servers > 0 {
        lines.push(crate::i18n::tr(lang, "Servers running: %d").replace("%d", &session.servers.to_string()));
    }

    if session.mail > 0 {
        lines.push(crate::i18n::tr(lang, "Letters to read: %d").replace("%d", &session.mail.to_string()));
    }

    if session.tokens_k > 0 {
        let pct = (session.context_percent * 100.0).round() as usize;
        let formatted = crate::i18n::tr(lang, "Context: %d k tokens (%d %%)")
            .replacen("%d", &session.tokens_k.to_string(), 1)
            .replacen("%d", &pct.to_string(), 1);
        lines.push(formatted);
    } else if session.context_percent > 0.0 {
        let pct = (session.context_percent * 100.0).round() as usize;
        lines.push(format!("{} {} %", crate::i18n::tr(lang, "Context:"), pct));
    }

    if session.git_conflict {
        lines.push(crate::i18n::tr(lang, "Merge or rebase to finish").to_string());
    }
    if session.dirty_lines > 0 {
        lines.push(crate::i18n::tr(lang, "Uncommitted: %d lines").replace("%d", &session.dirty_lines.to_string()));
    }
    if session.commits_behind > 0 {
        lines.push(crate::i18n::tr(lang, "%d commits behind origin").replace("%d", &session.commits_behind.to_string()));
    }

    if !session.summary.is_empty() {
        let s = if session.summary.chars().count() > 220 {
            let p: String = session.summary.chars().take(219).collect();
            format!("{}…", p)
        } else {
            session.summary.clone()
        };
        lines.push(s);
    } else if !session.last_prompt.is_empty() {
        let clean = session.last_prompt.replace('\n', " ");
        let s = if clean.chars().count() > 70 {
            let p: String = clean.chars().take(69).collect();
            format!("{}…", p)
        } else {
            clean
        };
        lines.push(crate::i18n::tr(lang, "“%s”").replace("%s", &s));
    }

    if let Some(secs) = focus_timer {
        let mins = (secs / 60.0).ceil() as u32;
        lines.push(crate::i18n::tr(lang, "Focus: %d min left").replace("%d", &mins.to_string()));
    } else if let Some(secs) = break_timer {
        let mins = (secs / 60.0).ceil() as u32;
        lines.push(crate::i18n::tr(lang, "Break: %d min left").replace("%d", &mins.to_string()));
    }

    lines.join("\n")
}

pub fn build_small_pet_card(small: &Pet) -> String {
    let act = &small.caption;
    if !small.label.is_empty() && !act.is_empty() {
        format!("{}: {}", small.label, act)
    } else if !small.label.is_empty() {
        small.label.clone()
    } else {
        act.clone()
    }
}
