use std::time::Instant;
use crate::pet::MascotKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Working,
    Waiting,
    Idle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubagentInfo {
    pub id: String,
    pub name: String,
    pub tool: String,
    pub detail: String,
    pub running: bool,
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub id: String,
    pub kind: MascotKind,
    pub name: String,
    pub branch: String,
    pub cwd: String,
    pub phase: SessionPhase,
    pub activity: String,
    pub detail: String,
    pub git_conflict: bool,
    pub dirty_lines: usize,
    pub commits_behind: usize,
    pub tokens_k: usize,
    pub context_percent: f32,
    pub subagents_count: usize,
    pub subagents: Vec<SubagentInfo>,
    pub last_prompt: String,
    pub summary: String,
    pub has_background: bool,
    pub compacting: bool,
    pub is_music_playing: bool,
    pub terminal_pid: Option<i32>,
    pub start_time: Instant,
    pub mail: usize,
    pub advising: bool,
    pub message_sent: Option<String>,
    pub servers: usize,
    pub tool_failed: Option<(Option<String>, bool)>,
    pub tests_passed: bool,
    pub repo_cleaned: bool,
    pub quiet_level: u8,
    pub custom_color: Option<(u8, u8, u8)>,
    pub collision: Option<(String, String)>,
    pub turn_origin_peer: bool,
    pub lineage: Vec<i32>,
}

impl SessionState {
    pub fn new(id: String, kind: MascotKind, name: String, cwd: String, pid: Option<i32>) -> Self {
        Self {
            id,
            kind,
            name,
            branch: String::new(),
            cwd,
            phase: SessionPhase::Idle,
            activity: String::new(),
            detail: String::new(),
            git_conflict: false,
            dirty_lines: 0,
            commits_behind: 0,
            tokens_k: 0,
            context_percent: 0.0,
            subagents_count: 0,
            subagents: Vec::new(),
            last_prompt: String::new(),
            summary: String::new(),
            has_background: false,
            compacting: false,
            is_music_playing: false,
            terminal_pid: pid,
            start_time: Instant::now(),
            mail: 0,
            advising: false,
            message_sent: None,
            servers: 0,
            tool_failed: None,
            tests_passed: false,
            repo_cleaned: false,
            quiet_level: 0,
            custom_color: None,
            collision: None,
            turn_origin_peer: false,
            lineage: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitCacheEntry {
    pub branch: String,
    pub head_oid: String,
    pub conflict: bool,
    pub dirty: usize,
    pub behind: usize,
    pub was_dirty: bool,
    pub last_scan: Instant,
}

#[derive(Default, Debug, Clone)]
pub struct ClaudeTranscriptData {
    pub context_tokens: usize,
    pub queue_mail: usize,
    pub servers_count: usize,
    pub background_count: usize,
    pub last_prompt: Option<String>,
    pub summary: Option<String>,
    pub cwd: Option<String>,
    pub compacting_reset: bool,
    pub advising: bool,
    pub message_sent: Option<String>,
    pub custom_color: Option<(u8, u8, u8)>,
    pub turn_origin_peer: bool,
    pub collision: Option<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct AgyParsedTranscript {
    pub phase: SessionPhase,
    pub activity: String,
    pub detail: String,
    pub tokens: usize,
    pub is_compacting: bool,
    pub has_bg: bool,
    pub servers: usize,
    pub last_prompt: String,
    pub summary: String,
    pub subagents_count: usize,
    pub subagents: Vec<SubagentInfo>,
    pub message_sent: Option<String>,
    pub workspace_cwd: Option<String>,
    pub turn_origin_peer: bool,
    pub tool_failed: Option<(Option<String>, bool)>,
    pub tests_passed: bool,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct DesktopMetaEntry {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub status: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub parent_id: String,
    #[serde(default)]
    pub not_idle: bool,
}
