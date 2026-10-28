use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::pet::MascotKind;

pub mod types;
pub mod mpris;
pub mod git;
pub mod proc;
pub mod claude;
pub mod agy;
pub mod agy_history;

pub use types::{DesktopMetaEntry, GitCacheEntry, SessionPhase, SessionState, SubagentInfo};

pub(crate) type SubagentHistoryCacheEntry = (u64, Vec<(String, String)>, HashSet<String>);

pub struct SessionWatcher {
    pub(crate) agy_candidate_dirs: Vec<PathBuf>,
    pub(crate) claude_dir: PathBuf,
    pub(crate) runtime_dir: PathBuf,
    pub(crate) home_dir: PathBuf,

    pub(crate) cached_agy_convs: Vec<(String, Option<i32>, PathBuf)>,
    pub(crate) last_proc_scan: Instant,

    pub(crate) claude_log_offset: u64,
    pub(crate) claude_sessions: HashMap<String, SessionState>,
    pub(crate) claude_last_times: HashMap<String, Instant>,
    pub(crate) claude_last_heard: HashMap<String, Instant>,
    pub(crate) claude_transcript_paths: HashMap<String, PathBuf>,
    pub(crate) claude_transcript_lens: HashMap<String, u64>,
    pub(crate) claude_transcript_offsets: HashMap<String, u64>,
    pub(crate) transcript_offsets: HashMap<String, u64>,
    pub(crate) claude_transcript_states: HashMap<String, claude::ClaudeTranscriptState>,
    pub(crate) claude_asking: HashSet<String>,
    pub(crate) claude_advising: bool,
    pub(crate) claude_message_sent: Option<String>,

    pub(crate) agy_log_offset: u64,
    pub(crate) agy_stopped_at_len: HashMap<String, u64>,
    pub(crate) agy_last_heard: HashMap<String, Instant>,
    pub(crate) agy_transcript_lens: HashMap<String, u64>,
    pub(crate) agy_subagent_lens: HashMap<String, u64>,
    pub(crate) agy_sessions: HashMap<String, (SessionState, PathBuf)>,
    pub(crate) agy_tasks: HashMap<String, HashMap<String, bool>>,

    pub(crate) session_phase_times: HashMap<String, (SessionPhase, Instant)>,
    pub(crate) git_cache: HashMap<String, GitCacheEntry>,

    pub(crate) cached_mpris: bool,
    pub(crate) last_mpris_scan: Instant,

    pub(crate) claude_subagents: HashMap<(String, String), SubagentInfo>,
    pub(crate) claude_launching: HashMap<String, Vec<String>>,
    pub(crate) claude_names: HashMap<String, String>,
    pub(crate) compact_echo_until: HashMap<String, Instant>,

    pub(crate) recent_file_writes: HashMap<String, HashMap<String, (String, Instant)>>,
    pub(crate) seen_collisions: HashSet<(String, String)>,

    pub(crate) agy_known_subagents: HashMap<String, Vec<(String, String)>>,
    pub(crate) agy_parsed_cache: HashMap<String, types::AgyParsedTranscript>,
    pub(crate) agy_subagent_history_cache: HashMap<String, SubagentHistoryCacheEntry>,
    pub(crate) desktop_meta: HashMap<String, DesktopMetaEntry>,
    pub(crate) last_desktop_db_mtime: Option<std::time::SystemTime>,
}

impl SessionWatcher {
    pub fn new() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/home/user"));

        let candidate_paths = vec![
            home.join(".gemini/antigravity-cli"),
            home.join(".gemini/antigravity"),
            home.join("snap/antigravity-cli/common/.gemini/antigravity-cli"),
        ];
        let mut agy_candidate_dirs = Vec::new();
        for p in candidate_paths {
            if !agy_candidate_dirs.contains(&p) {
                agy_candidate_dirs.push(p);
            }
        }

        let claude_dir = std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".claude"));

        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .or_else(|| std::env::var_os("TEMP"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("paros");

        let claude_log_path = if runtime.join("claude-events.log").exists() {
            runtime.join("claude-events.log")
        } else {
            PathBuf::from("/tmp/paros/claude-events.log")
        };
        let claude_log_offset = std::fs::metadata(&claude_log_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let agy_log_path = if runtime.join("agy-events.log").exists() {
            runtime.join("agy-events.log")
        } else {
            PathBuf::from("/tmp/paros/agy-events.log")
        };
        let agy_log_offset = std::fs::metadata(&agy_log_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Self {
            agy_candidate_dirs,
            claude_dir,
            runtime_dir: runtime,
            home_dir: home,

            cached_agy_convs: Vec::new(),
            last_proc_scan: Instant::now() - Duration::from_secs(10),

            claude_log_offset,
            claude_sessions: HashMap::new(),
            claude_last_times: HashMap::new(),
            claude_last_heard: HashMap::new(),
            claude_transcript_paths: HashMap::new(),
            claude_transcript_lens: HashMap::new(),
            claude_transcript_offsets: HashMap::new(),
            transcript_offsets: HashMap::new(),
            claude_transcript_states: HashMap::new(),
            claude_asking: HashSet::new(),
            claude_advising: false,
            claude_message_sent: None,

            agy_log_offset,
            agy_stopped_at_len: HashMap::new(),
            agy_last_heard: HashMap::new(),
            agy_transcript_lens: HashMap::new(),
            agy_subagent_lens: HashMap::new(),
            agy_sessions: HashMap::new(),
            agy_tasks: HashMap::new(),

            session_phase_times: HashMap::new(),
            git_cache: HashMap::new(),

            cached_mpris: false,
            last_mpris_scan: Instant::now() - Duration::from_secs(10),

            claude_subagents: HashMap::new(),
            claude_launching: HashMap::new(),
            claude_names: HashMap::new(),
            compact_echo_until: HashMap::new(),

            recent_file_writes: HashMap::new(),
            seen_collisions: HashSet::new(),

            agy_known_subagents: HashMap::new(),
            agy_parsed_cache: HashMap::new(),
            agy_subagent_history_cache: HashMap::new(),
            desktop_meta: HashMap::new(),
            last_desktop_db_mtime: None,
        }
    }

    pub(crate) fn phase_start_time(&mut self, session_id: &str, phase: SessionPhase) -> Instant {
        let now = Instant::now();
        match self.session_phase_times.get(session_id) {
            Some((prev_phase, start)) if *prev_phase == phase => *start,
            _ => {
                self.session_phase_times.insert(session_id.to_string(), (phase, now));
                now
            }
        }
    }

    pub(crate) fn compute_quiet_level(phase: SessionPhase, silent_for: Duration) -> u8 {
        if phase != SessionPhase::Working {
            return 0;
        }
        let secs = silent_for.as_secs();
        if secs >= 25 {
            2
        } else if secs >= 8 {
            1
        } else {
            0
        }
    }

    pub(crate) fn record_file_write(&mut self, session_id: &str, session_name: &str, path: &str) {
        if path.is_empty() {
            return;
        }
        if self.recent_file_writes.len() >= 400 {
            self.recent_file_writes.clear();
        }
        self.recent_file_writes
            .entry(path.to_string())
            .or_default()
            .insert(session_id.to_string(), (session_name.to_string(), Instant::now()));
    }

    pub(crate) fn find_collision_writer(&self, session_id: &str, path: &str) -> Option<String> {
        let now = Instant::now();
        if let Some(writers) = self.recent_file_writes.get(path) {
            let mut best: Option<(&str, Instant)> = None;
            for (writer_id, (writer_name, when)) in writers {
                if writer_id != session_id && now.duration_since(*when) < Duration::from_secs(900) {
                    let display_name = self
                        .claude_sessions
                        .get(writer_id)
                        .map(|s| s.name.as_str())
                        .unwrap_or(writer_name.as_str());
                    if best.is_none_or(|(_, b_when)| *when > b_when) {
                        best = Some((display_name, *when));
                    }
                }
            }
            if let Some((name, _)) = best {
                return Some(name.to_string());
            }
        }
        for (other_id, st) in &self.claude_sessions {
            if other_id != session_id {
                return Some(st.name.clone());
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn poll_all(&mut self) -> Vec<SessionState> {
        self.poll_all_with_settings(true, 1000, true)
    }

    pub fn poll_all_with_settings(
        &mut self,
        git_enabled: bool,
        claude_context_k: usize,
        groove_enabled: bool,
    ) -> Vec<SessionState> {
        for st in self.claude_sessions.values_mut() {
            st.tool_failed = None;
            st.tests_passed = false;
            st.repo_cleaned = false;
            st.collision = None;
        }

        self.poll_claude_registry();
        self.poll_claude_hooks();
        self.refresh_claude_sessions(git_enabled, claude_context_k, groove_enabled);
        self.poll_agy_hooks();

        let mut all_sessions = Vec::new();

        for st in self.claude_sessions.values() {
            all_sessions.push(st.clone());
        }

        let agy_sessions = self.check_all_antigravity(git_enabled, groove_enabled);
        for st in agy_sessions {
            all_sessions.push(st);
        }

        if all_sessions.is_empty() {
            let cwd = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string());
            let (branch, conflict, dirty, behind, repo_cleaned) = self.check_git(&cwd, git_enabled);
            let music = if groove_enabled {
                self.check_mpris()
            } else {
                false
            };
            let start_time = self.phase_start_time("none", SessionPhase::Idle);
            let mut s = SessionState::new("none".to_string(), MascotKind::Claude, "Paros".to_string(), cwd, None);
            s.branch = branch;
            s.git_conflict = conflict;
            s.dirty_lines = dirty;
            s.commits_behind = behind;
            s.repo_cleaned = repo_cleaned;
            s.is_music_playing = music;
            s.start_time = start_time;
            all_sessions.push(s);
        }

        let active_ids: HashSet<&str> = all_sessions.iter().map(|s| s.id.as_str()).collect();
        self.session_phase_times.retain(|k, _| active_ids.contains(k.as_str()) || k == "none");
        if self.seen_collisions.len() > 200 {
            self.seen_collisions.clear();
        }

        all_sessions
    }
}
