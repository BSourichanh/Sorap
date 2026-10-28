use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use serde_json::Value;

use crate::pet::MascotKind;
use super::proc::{
    count_inbox_messages, extract_json_str_field, parse_claude_transcript_line,
};
use super::types::{ClaudeTranscriptData, SessionPhase, SessionState, SubagentInfo};
use super::SessionWatcher;

#[derive(Default, Clone)]
pub(crate) struct ClaudeTranscriptState {
    pub(crate) offset: u64,
    pub(crate) data: ClaudeTranscriptData,
    pub(crate) queue: Vec<bool>,
    pub(crate) launches: HashMap<String, bool>,
    pub(crate) tasks: HashMap<String, bool>,
    pub(crate) writes_by_call: HashMap<String, String>,
    pub(crate) last_write_path: Option<String>,
}

impl SessionWatcher {
    pub(crate) fn find_claude_transcript(&mut self, session_id: &str) -> Option<PathBuf> {
        if let Some(cached) = self.claude_transcript_paths.get(session_id)
            && cached.exists() {
                return Some(cached.clone());
            }
        let projects_dir = self.claude_dir.join("projects");
        let target_name = format!("{session_id}.jsonl");
        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let candidate = entry.path().join(&target_name);
                if candidate.exists() {
                    self.claude_transcript_paths
                        .insert(session_id.to_string(), candidate.clone());
                    return Some(candidate);
                }
            }
        }
        None
    }

    pub(crate) fn poll_claude_transcript(
        &mut self,
        session_id: &str,
        session_name: &str,
    ) -> Option<ClaudeTranscriptData> {
        let path = self.find_claude_transcript(session_id)?;
        let file = File::open(&path).ok()?;
        let meta = file.metadata().ok()?;
        let file_len = meta.len();

        let prev_len = self.claude_transcript_lens.insert(session_id.to_string(), file_len);
        if prev_len.is_some_and(|pl| file_len > pl) {
            self.claude_last_heard.insert(session_id.to_string(), Instant::now());
        }

        if file_len == 0 {
            self.claude_transcript_offsets.insert(session_id.to_string(), 0);
            self.transcript_offsets.insert(session_id.to_string(), 0);
            return Some(ClaudeTranscriptData::default());
        }

        let is_new = !self.claude_transcript_offsets.contains_key(session_id)
            && !self.transcript_offsets.contains_key(session_id);
        let mut cur_offset = self.claude_transcript_offsets.get(session_id).copied().unwrap_or(0);

        if is_new && file_len > 131072 {
            cur_offset = file_len.saturating_sub(131072);
        } else if file_len < cur_offset {
            cur_offset = 0;
            self.claude_transcript_offsets.insert(session_id.to_string(), 0);
            self.transcript_offsets.insert(session_id.to_string(), 0);
            self.claude_transcript_states.remove(session_id);
        }

        if !is_new && cur_offset >= file_len
            && let Some(state) = self.claude_transcript_states.get(session_id) {
                return Some(state.data.clone());
            }

        let mut reader = BufReader::new(file);
        if cur_offset > 0 {
            if reader.seek(SeekFrom::Start(cur_offset)).is_err() {
                return self.claude_transcript_states.get(session_id).map(|s| s.data.clone());
            }
            if is_new {
                let mut discard = String::new();
                let _ = reader.read_line(&mut discard);
            }
        }

        let mut state = self.claude_transcript_states.remove(session_id).unwrap_or_default();
        let mut line_buf = String::new();
        let mut last_valid_offset = reader.stream_position().unwrap_or(cur_offset);

        while let Ok(bytes) = reader.read_line(&mut line_buf) {
            if bytes == 0 {
                break;
            }
            let trimmed = line_buf.trim();
            if !line_buf.ends_with('\n') && !trimmed.ends_with('}') {
                break;
            }
            last_valid_offset = reader.stream_position().unwrap_or(last_valid_offset + bytes as u64);

            if !trimmed.is_empty() {
                parse_claude_transcript_line(
                    trimmed,
                    &mut state.data,
                    &mut state.queue,
                    &mut state.launches,
                    &mut state.tasks,
                    &mut state.writes_by_call,
                    &mut state.last_write_path,
                    |p| self.record_file_write(session_id, session_name, p),
                );

                if trimmed.contains("<tool_use_error>File has been modified since read") {
                    let call_id = extract_json_str_field(trimmed, "tool_use_id").unwrap_or("unknown").to_string();
                    let collision_key = (session_id.to_string(), call_id.clone());
                    if !self.seen_collisions.contains(&collision_key) {
                        self.seen_collisions.insert(collision_key);
                        let path = state.writes_by_call.get(&call_id).cloned().or_else(|| state.last_write_path.clone()).unwrap_or_default();
                        let filename = Path::new(&path).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| path.clone());
                        if let Some(other_name) = self.find_collision_writer(session_id, &path) {
                            state.data.collision = Some((other_name, filename));
                        } else if !filename.is_empty() {
                            state.data.collision = Some(("Autre session".to_string(), filename));
                        }
                    }
                }
            }
            line_buf.clear();
        }

        state.data.queue_mail = state.queue.iter().filter(|&&b| b).count();
        let servers = state.tasks.values().filter(|&&s| s).count() + state.launches.values().filter(|&&s| s).count();
        let bg = state.tasks.values().filter(|&&s| !s).count() + state.launches.values().filter(|&&s| !s).count();
        state.data.servers_count = servers;
        state.data.background_count = bg;
        state.offset = last_valid_offset;

        self.claude_transcript_offsets.insert(session_id.to_string(), last_valid_offset);
        self.transcript_offsets.insert(session_id.to_string(), last_valid_offset);

        let ret_data = state.data.clone();
        state.data.collision = None;
        state.data.compacting_reset = false;

        self.claude_transcript_states.insert(session_id.to_string(), state);

        Some(ret_data)
    }

    #[allow(dead_code)]
    pub(crate) fn parse_claude_transcript(
        &mut self,
        session_id: &str,
        session_name: &str,
    ) -> Option<ClaudeTranscriptData> {
        self.poll_claude_transcript(session_id, session_name)
    }

    pub(crate) fn poll_claude_registry(&mut self) {
        let sessions_dir = self.claude_dir.join("sessions");
        let Ok(entries) = std::fs::read_dir(&sessions_dir) else { return };

        let mut live_registry_ids = HashSet::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else { continue };
            let Ok(val) = serde_json::from_str::<Value>(&content) else { continue };

            if val.get("kind").and_then(|v| v.as_str()).unwrap_or("interactive") != "interactive" {
                continue;
            }
            let Some(session_id) = val.get("sessionId").and_then(|v| v.as_str()).map(|s| s.to_string()) else {
                continue;
            };
            let Some(pid) = val.get("pid").and_then(|v| v.as_i64()).map(|p| p as i32) else {
                continue;
            };
            if pid <= 0 || !Path::new(&format!("/proc/{pid}")).exists() {
                continue;
            }

            live_registry_ids.insert(session_id.clone());
            self.claude_last_times.insert(session_id.clone(), Instant::now());
            self.claude_last_heard.entry(session_id.clone()).or_insert_with(Instant::now);

            let cwd = val.get("cwd").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| {
                std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| ".".to_string())
            });

            let raw_name = val.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();
            let name = if !raw_name.is_empty() {
                raw_name.to_string()
            } else {
                Path::new(&cwd).file_name().map(|f| f.to_string_lossy().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "Claude Code".to_string())
            };

            let status_opt = val.get("status").and_then(|v| v.as_str());
            if status_opt == Some("idle") {
                self.claude_asking.remove(&session_id);
            }
            let is_asking = self.claude_asking.contains(&session_id);

            let mapped_phase = match status_opt {
                Some("waiting") => Some(SessionPhase::Waiting),
                Some("busy") => Some(if is_asking { SessionPhase::Waiting } else { SessionPhase::Working }),
                Some("idle") | Some("shell") => Some(SessionPhase::Idle),
                _ => is_asking.then_some(SessionPhase::Waiting),
            };
            let is_shell = status_opt == Some("shell");

            if let Some(st) = self.claude_sessions.get_mut(&session_id) {
                st.name = name;
                st.cwd = cwd;
                st.terminal_pid = Some(pid);
                if let Some(ph) = mapped_phase {
                    if st.phase != ph {
                        self.claude_last_heard.insert(session_id.clone(), Instant::now());
                    }
                    st.phase = ph;
                }
                if is_shell {
                    st.has_background = true;
                }
            } else {
                let phase = mapped_phase.unwrap_or(SessionPhase::Idle);
                let start_time = self.phase_start_time(&session_id, phase);
                let lineage = Self::build_lineage(Some(pid));
                let mut st = SessionState::new(session_id.clone(), MascotKind::Claude, name, cwd, Some(pid));
                st.phase = phase;
                st.start_time = start_time;
                st.has_background = is_shell;
                st.lineage = lineage;
                self.claude_sessions.insert(session_id, st);
            }
        }

        self.claude_sessions.retain(|sid, st| {
            if let Some(pid) = st.terminal_pid {
                live_registry_ids.contains(sid) && Path::new(&format!("/proc/{pid}")).exists()
            } else {
                true
            }
        });
        self.claude_transcript_offsets.retain(|sid, _| self.claude_sessions.contains_key(sid));
        self.transcript_offsets.retain(|sid, _| self.claude_sessions.contains_key(sid));
        self.claude_transcript_states.retain(|sid, _| self.claude_sessions.contains_key(sid));
        self.claude_last_times.retain(|sid, _| self.claude_sessions.contains_key(sid));
        self.claude_last_heard.retain(|sid, _| self.claude_sessions.contains_key(sid));
        self.claude_transcript_paths.retain(|sid, _| self.claude_sessions.contains_key(sid));
        self.claude_transcript_lens.retain(|sid, _| self.claude_sessions.contains_key(sid));
        self.claude_asking.retain(|sid| self.claude_sessions.contains_key(sid));
        self.claude_subagents.retain(|(sid, _), _| self.claude_sessions.contains_key(sid));
        self.claude_names.retain(|aid, _| self.claude_subagents.keys().any(|(_, id)| id == aid));
    }

    pub(crate) fn poll_claude_hooks(&mut self) {
        let log_path = if self.runtime_dir.join("claude-events.log").exists() {
            self.runtime_dir.join("claude-events.log")
        } else {
            PathBuf::from("/tmp/paros/claude-events.log")
        };
        let Ok(file) = File::open(&log_path) else { return };
        let Ok(meta) = file.metadata() else { return };
        let len = meta.len();

        if len < self.claude_log_offset {
            self.claude_log_offset = 0;
        }
        if len == self.claude_log_offset {
            return;
        }

        let mut reader = BufReader::new(file);
        if reader.seek(SeekFrom::Start(self.claude_log_offset)).is_err() {
            return;
        }

        for line in reader.lines().map_while(Result::ok) {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.is_empty() {
                continue;
            }
            let event = fields[0];
            let session_id = fields.get(1).unwrap_or(&"").to_string();
            if session_id.is_empty() {
                continue;
            }
            let notif_type = fields.get(2).unwrap_or(&"").trim();
            let tool = fields.get(3).unwrap_or(&"").to_string();
            let detail = fields.get(4).unwrap_or(&"").to_string();
            let kind_field = fields.get(5).unwrap_or(&"").trim();
            let agent_id = fields.get(6).unwrap_or(&"").trim().to_string();

            if event == "PreToolUse" && agent_id.is_empty() && (tool == "Agent" || tool == "Task") {
                self.claude_launching.entry(session_id.clone()).or_default().push(detail.clone());
            }

            if event == "SubagentStart" && !agent_id.is_empty() {
                let name = self.claude_launching
                    .get_mut(&session_id)
                    .and_then(|queue| if !queue.is_empty() { Some(queue.remove(0)) } else { None })
                    .unwrap_or_else(|| {
                        self.claude_names.get(&agent_id).cloned().unwrap_or_else(|| {
                            if !detail.is_empty() { detail.clone() } else { "Sous-agent".to_string() }
                        })
                    });
                self.claude_names.insert(agent_id.clone(), name.clone());
                self.claude_subagents.insert(
                    (session_id.clone(), agent_id.clone()),
                    SubagentInfo { id: agent_id.clone(), name, tool: String::new(), detail: String::new(), running: true },
                );
            } else if event == "SubagentStop" && !agent_id.is_empty() {
                self.claude_subagents.remove(&(session_id.clone(), agent_id.clone()));
            } else if event == "PreToolUse" && !agent_id.is_empty() {
                let raw = detail.trim_matches(|c| c == '"' || c == '\'');
                let det = Path::new(raw).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| raw.to_string());
                let entry = self.claude_subagents.entry((session_id.clone(), agent_id.clone())).or_insert_with(|| {
                    let name = self.claude_names.get(&agent_id).cloned().unwrap_or_else(|| "Sous-agent".to_string());
                    SubagentInfo { id: agent_id.clone(), name, tool: tool.clone(), detail: det.clone(), running: true }
                });
                entry.tool = tool.clone();
                entry.detail = det;
                entry.running = true;
            }

            if event == "PreToolUse" && agent_id.is_empty() {
                if tool == "SendMessage" || tool == "mcp__agent-bridge__send" {
                    self.claude_message_sent = Some(detail.clone());
                } else { self.claude_advising = tool == "advisor"; }
                if matches!(tool.as_str(), "Edit" | "Write" | "MultiEdit" | "NotebookEdit") && !detail.is_empty() {
                    let sname = self.claude_sessions.get(&session_id).map(|s| s.name.clone()).unwrap_or_else(|| "Claude Code".to_string());
                    self.record_file_write(&session_id, &sname, &detail);
                }
            } else if event == "Stop" || event == "UserPromptSubmit" {
                self.claude_advising = false;
                self.claude_message_sent = None;
            }

            let now = Instant::now();
            self.claude_last_times.insert(session_id.clone(), now);
            self.claude_last_heard.insert(session_id.clone(), now);

            if event != "Notification" {
                self.claude_asking.remove(&session_id);
            }

            let is_test = kind_field == "test" || Self::is_test_command(&detail) || Self::is_test_command(&tool);
            let default_cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| ".".to_string());

            let st = self.claude_sessions.entry(session_id.clone()).or_insert_with(|| {
                let mut s = SessionState::new(session_id.clone(), MascotKind::Claude, "Claude Code".to_string(), default_cwd, None);
                s.start_time = now;
                s
            });

            match event {
                "UserPromptSubmit" => {
                    if !detail.is_empty() { st.last_prompt = detail; }
                    st.phase = SessionPhase::Working;
                    st.activity = "Réflexion...".to_string();
                    st.detail.clear();
                    st.compacting = false;
                }
                "PreToolUse" if agent_id.is_empty() => {
                    st.phase = SessionPhase::Working;
                    st.activity = tool;
                    st.detail = detail;
                    st.compacting = false;
                }
                "PostToolUse" => { if is_test { st.tests_passed = true; } }
                "PostToolUseFailure" => {
                    let sub_opt = (!agent_id.is_empty()).then_some(agent_id);
                    st.tool_failed = Some((sub_opt, is_test));
                }
                "Notification" if notif_type == "permission_prompt" || notif_type == "elicitation_dialog" => {
                    self.claude_asking.insert(session_id.clone());
                    st.phase = SessionPhase::Waiting;
                    st.activity = "Demande".to_string();
                    st.detail = detail;
                }
                "PreCompact" if agent_id.is_empty() => {
                    st.compacting = true;
                    st.phase = SessionPhase::Working;
                    st.activity = "Compactage...".to_string();
                }
                "PostCompact" if agent_id.is_empty() => {
                    st.compacting = false;
                    self.compact_echo_until.insert(session_id.clone(), Instant::now() + Duration::from_secs(10));
                }
                "Stop" if agent_id.is_empty() => {
                    if let Some(queue) = self.claude_launching.get_mut(&session_id) {
                        queue.clear();
                    }
                    let is_compact_echo = self.compact_echo_until.get(&session_id).is_some_and(|until| Instant::now() < *until);
                    if !st.compacting && !is_compact_echo {
                        st.phase = SessionPhase::Idle;
                        st.activity.clear();
                        st.detail.clear();
                    }
                }
                _ => {}
            }
        }

        self.claude_log_offset = len;

        let now = Instant::now();
        let last_times = &self.claude_last_times;
        self.claude_sessions.retain(|sid, st| {
            if st.terminal_pid.is_some() {
                true
            } else {
                last_times.get(sid).is_some_and(|t| now.duration_since(*t) < Duration::from_secs(60))
            }
        });
        self.compact_echo_until.retain(|_, until| *until > now);
        self.claude_launching.retain(|sid, _| self.claude_sessions.contains_key(sid));
    }

    pub(crate) fn check_advising_file(&self) -> bool {
        let path = if self.runtime_dir.join("advising.txt").exists() {
            self.runtime_dir.join("advising.txt")
        } else if let Some(xdg) = std::env::var_os("XDG_RUNTIME_DIR") {
            let p = PathBuf::from(xdg).join("paros/advising.txt");
            if p.exists() { p } else { PathBuf::from("/tmp/paros/advising.txt") }
        } else {
            PathBuf::from("/tmp/paros/advising.txt")
        };

        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path) {
                let lines: Vec<&str> = content.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
                if let Some(first) = lines.first()
                    && let Ok(file_time) = first.parse::<f64>() {
                        let now_unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
                        let diff = now_unix - file_time;
                        if (-6.0..6.0).contains(&diff) {
                            return lines.len() > 1;
                        }
                        return false;
                    }
            }

        std::fs::metadata("/tmp/paros/claude-advising")
            .or_else(|_| std::fs::metadata(self.runtime_dir.join("claude-advising")))
            .and_then(|m| m.modified())
            .map(|mtime| mtime.elapsed().map(|d| d.as_secs() < 8).unwrap_or(false))
            .unwrap_or(false)
    }

    pub(crate) fn refresh_claude_sessions(
        &mut self,
        git_enabled: bool,
        claude_context_k: usize,
        groove_enabled: bool,
    ) {
        let music = if groove_enabled { self.check_mpris() } else { false };
        let inbox_base = self.home_dir.join(".agent-bridge/inbox");
        let base_inbox_mail = count_inbox_messages(&inbox_base);
        let advising_file_active = self.check_advising_file();

        let session_ids: Vec<String> = self.claude_sessions.keys().cloned().collect();
        for sid in session_ids {
            let sname = self.claude_sessions.get(&sid).map(|s| s.name.clone()).unwrap_or_default();
            let tdata = self.poll_claude_transcript(&sid, &sname);

            let current_cwd = {
                let Some(st) = self.claude_sessions.get_mut(&sid) else { continue };
                if let Some(ref td) = tdata
                    && let Some(ref t_cwd) = td.cwd
                        && (st.terminal_pid.is_none() || st.cwd.is_empty() || st.cwd == ".") {
                            st.cwd = t_cwd.clone();
                            if st.name == "Claude Code"
                                && let Some(fname) = Path::new(t_cwd).file_name().map(|f| f.to_string_lossy().to_string())
                                    && !fname.is_empty() { st.name = fname; }
                        }
                st.cwd.clone()
            };

            let (branch, conflict, dirty, behind, repo_cleaned) = self.check_git(&current_cwd, git_enabled);
            let subagents_vec: Vec<SubagentInfo> = self.claude_subagents.iter().filter(|((s, _), _)| s == &sid).map(|(_, sub)| sub.clone()).collect();
            let silent_dur = self.claude_last_heard.get(&sid).map(|t| t.elapsed()).unwrap_or(Duration::ZERO);
            let phase = self.claude_sessions.get(&sid).map(|s| s.phase).unwrap_or(SessionPhase::Idle);
            let start_time = self.phase_start_time(&sid, phase);
            let quiet_level = Self::compute_quiet_level(phase, silent_dur);

            let session_inbox = inbox_base.join(format!("claude-{sid}"));
            let specific_mail = if session_inbox.exists() { count_inbox_messages(&session_inbox) } else { base_inbox_mail };

            if let Some(st) = self.claude_sessions.get_mut(&sid) {
                st.branch = branch;
                st.git_conflict = conflict;
                st.dirty_lines = dirty;
                st.commits_behind = behind;
                if repo_cleaned { st.repo_cleaned = true; }
                st.is_music_playing = music;
                st.subagents_count = subagents_vec.len();
                st.subagents = subagents_vec;
                st.start_time = start_time;
                st.quiet_level = quiet_level;
                st.lineage = Self::build_lineage(st.terminal_pid);

                if let Some(td) = tdata {
                    if td.context_tokens > 0 || td.compacting_reset {
                        let rounded = ((td.context_tokens as f32 / 10000.0).round() as usize) * 10000;
                        st.tokens_k = rounded / 1000;
                        let denom = (claude_context_k.max(1) * 1000) as f32;
                        st.context_percent = (rounded as f32 / denom).clamp(0.0, 1.0);
                    }
                    st.mail = specific_mail + td.queue_mail;
                    st.servers = td.servers_count;
                    st.has_background = td.background_count > 0;
                    st.advising = self.claude_advising || advising_file_active || td.advising;
                    if td.custom_color.is_some() { st.custom_color = td.custom_color; }
                    st.turn_origin_peer = td.turn_origin_peer;
                    if td.collision.is_some() { st.collision = td.collision; }
                    if let Some(lp) = td.last_prompt
                        && !lp.is_empty() { st.last_prompt = lp; }
                    if let Some(sum) = td.summary { st.summary = sum; }
                    if st.phase == SessionPhase::Idle {
                        st.message_sent = None;
                    } else if self.claude_message_sent.is_some() {
                        st.message_sent = self.claude_message_sent.clone();
                    } else if td.message_sent.is_some() {
                        st.message_sent = td.message_sent;
                    }
                } else {
                    st.mail = specific_mail;
                    st.advising = self.claude_advising || advising_file_active;
                    st.message_sent = if st.phase == SessionPhase::Idle { None } else { self.claude_message_sent.clone() };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_incremental_claude_transcript() {
        let temp_dir = std::env::temp_dir().join(format!("paros_test_{}", fastrand::u64(..)));
        let projects_dir = temp_dir.join("projects");
        std::fs::create_dir_all(&projects_dir).unwrap();

        let mut watcher = SessionWatcher::new();
        watcher.claude_dir = temp_dir.clone();

        let proj_dir = projects_dir.join("proj1");
        std::fs::create_dir_all(&proj_dir).unwrap();

        let session_id = "test_session_1";
        let transcript_path = proj_dir.join(format!("{session_id}.jsonl"));

        // Étape 1 : écriture initiale
        {
            let mut f = File::create(&transcript_path).unwrap();
            writeln!(f, "{{\"type\":\"user\",\"message\":\"hello\",\"usage\":{{\"input_tokens\":1000}}}}").unwrap();
        }

        let data1 = watcher.poll_claude_transcript(session_id, "test").unwrap();
        assert_eq!(data1.context_tokens, 1000);
        let offset1 = *watcher.claude_transcript_offsets.get(session_id).unwrap();
        assert!(offset1 > 0);

        // Étape 2 : relecture sans modification -> données en cache et offset inchangé
        let data2 = watcher.poll_claude_transcript(session_id, "test").unwrap();
        assert_eq!(data2.context_tokens, 1000);
        assert_eq!(*watcher.claude_transcript_offsets.get(session_id).unwrap(), offset1);

        // Étape 3 : ajout d'une nouvelle ligne -> offset progresse, tokens mis à jour
        {
            let mut f = std::fs::OpenOptions::new().append(true).open(&transcript_path).unwrap();
            writeln!(f, "{{\"type\":\"user\",\"message\":\"world\",\"usage\":{{\"input_tokens\":2500}}}}").unwrap();
        }

        let data3 = watcher.poll_claude_transcript(session_id, "test").unwrap();
        assert_eq!(data3.context_tokens, 2500);
        let offset2 = *watcher.claude_transcript_offsets.get(session_id).unwrap();
        assert!(offset2 > offset1);

        // Étape 4 : troncature du fichier -> réinitialisation à 0 et relecture
        {
            let mut f = File::create(&transcript_path).unwrap();
            writeln!(f, "{{\"type\":\"user\",\"message\":\"reset\",\"usage\":{{\"input_tokens\":500}}}}").unwrap();
        }

        let data4 = watcher.poll_claude_transcript(session_id, "test").unwrap();
        assert_eq!(data4.context_tokens, 500);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_claude_compact_echo_stop_and_launching_segregation() {
        let temp_dir = std::env::temp_dir().join(format!("paros_test_hooks_{}", fastrand::u64(..)));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let log_path = temp_dir.join("claude-events.log");

        let mut watcher = SessionWatcher::new();
        watcher.runtime_dir = temp_dir.clone();
        watcher.claude_log_offset = 0;

        // PreToolUse Agent pour s1 et s2
        {
            let mut f = File::create(&log_path).unwrap();
            writeln!(f, "PreToolUse\ts1\t\tAgent\tWorker1\t\t").unwrap();
            writeln!(f, "PreToolUse\ts2\t\tAgent\tWorker2\t\t").unwrap();
        }
        watcher.poll_claude_hooks();
        assert_eq!(watcher.claude_launching.get("s1").unwrap(), &vec!["Worker1".to_string()]);
        assert_eq!(watcher.claude_launching.get("s2").unwrap(), &vec!["Worker2".to_string()]);

        // Stop sur s1 vide uniquement la file de s1
        {
            let mut f = std::fs::OpenOptions::new().append(true).open(&log_path).unwrap();
            writeln!(f, "Stop\ts1\t\t\t\t\t").unwrap();
        }
        watcher.poll_claude_hooks();
        assert!(watcher.claude_launching.get("s1").map_or(true, |v| v.is_empty()));
        assert_eq!(watcher.claude_launching.get("s2").unwrap(), &vec!["Worker2".to_string()]);

        // PostCompact sur s2 positionne compact_echo_until
        {
            let mut f = std::fs::OpenOptions::new().append(true).open(&log_path).unwrap();
            writeln!(f, "PostCompact\ts2\t\t\t\t\t").unwrap();
        }
        watcher.poll_claude_hooks();
        assert!(watcher.compact_echo_until.contains_key("s2"));

        // Un Stop immédiat sur s2 est ignoré
        let s2_state = watcher.claude_sessions.get_mut("s2").unwrap();
        s2_state.phase = SessionPhase::Working;
        s2_state.activity = "Outil".to_string();

        {
            let mut f = std::fs::OpenOptions::new().append(true).open(&log_path).unwrap();
            writeln!(f, "Stop\ts2\t\t\t\t\t").unwrap();
        }
        watcher.poll_claude_hooks();
        let s2 = watcher.claude_sessions.get("s2").unwrap();
        assert_eq!(s2.phase, SessionPhase::Working);
        assert_eq!(s2.activity, "Outil");

        // Si compact_echo_until est expiré, le Stop est appliqué
        watcher.compact_echo_until.insert("s2".to_string(), Instant::now() - Duration::from_secs(1));
        {
            let mut f = std::fs::OpenOptions::new().append(true).open(&log_path).unwrap();
            writeln!(f, "Stop\ts2\t\t\t\t\t").unwrap();
        }
        watcher.poll_claude_hooks();
        let s2_after = watcher.claude_sessions.get("s2").unwrap();
        assert_eq!(s2_after.phase, SessionPhase::Idle);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
