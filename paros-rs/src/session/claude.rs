use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use serde_json::Value;

use crate::pet::MascotKind;
use super::proc::{count_inbox_messages, extract_json_str_field};
pub(crate) use super::claude_tools::ClaudeTranscriptState;
use super::types::{SessionPhase, SessionState, SubagentInfo};
use super::SessionWatcher;

impl SessionWatcher {
    pub(crate) fn poll_claude_registry(&mut self) -> Vec<SessionState> {
        let mut results = Vec::new();
        let mut live_registry_ids: HashSet<String> = HashSet::new();

        let mut live_node_pids = Vec::new();
        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let pid_str = entry.file_name().to_string_lossy().to_string();
                let Ok(pid) = pid_str.parse::<i32>() else { continue };
                let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
                let trimmed = comm.trim();
                if trimmed == "claude" || trimmed == "node" {
                    live_node_pids.push(pid);
                }
            }
        }

        let reg_path = self.claude_dir.join("sessions.json");
        let reg_content = std::fs::read_to_string(&reg_path).unwrap_or_default();
        let val: Value = serde_json::from_str(&reg_content).unwrap_or(Value::Null);

        let sessions_map = val.get("sessions").and_then(|s| s.as_object());
        let empty_map = serde_json::Map::new();
        let entries = sessions_map.unwrap_or(&empty_map);

        for (session_id_str, val) in entries {
            let session_id = session_id_str.clone();
            live_registry_ids.insert(session_id.clone());

            let pid = val.get("pid").and_then(|v| v.as_i64()).map(|p| p as i32).unwrap_or(0);
            if pid <= 0 || !live_node_pids.contains(&pid) {
                continue;
            }

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

        results.extend(self.claude_sessions.values().cloned());
        results
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

            let notif_type = fields.get(2).unwrap_or(&"").to_string();
            let tool = fields.get(3).unwrap_or(&"").to_string();
            let detail = fields.get(4).unwrap_or(&"").to_string();
            let kind_field = fields.get(5).unwrap_or(&"").to_string();
            let agent_id = fields.get(6).unwrap_or(&"").to_string();

            if (event == "PreToolUse" || event == "PostToolUse")
                && (tool == "SendMessage" || tool == "mcp__agent-bridge__send")
                && let Some(target) = extract_json_str_field(&detail, "to").or_else(|| extract_json_str_field(&detail, "recipient"))
            {
                self.claude_message_sent = Some(target.to_string());
            }

            if event == "Stop" {
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

    #[test]
    fn test_claude_compact_echo_stop_and_launching_segregation() {
        let temp_dir = std::env::temp_dir().join(format!("paros_test_hooks_{}", fastrand::u64(..)));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut watcher = SessionWatcher::new();
        watcher.runtime_dir = temp_dir.clone();
        let log_file = temp_dir.join("claude-events.log");

        let sid = "session_echo_test";
        watcher.claude_sessions.insert(
            sid.to_string(),
            SessionState::new(sid.to_string(), MascotKind::Claude, "Test".to_string(), "/tmp".to_string(), None),
        );

        // Événement PreCompact
        std::fs::write(&log_file, format!("PreCompact\t{sid}\n")).unwrap();
        watcher.poll_claude_hooks();
        assert!(watcher.claude_sessions.get(sid).unwrap().compacting);

        // PostCompact
        std::fs::write(&log_file, format!("PreCompact\t{sid}\nPostCompact\t{sid}\n")).unwrap();
        watcher.claude_log_offset = 0;
        watcher.poll_claude_hooks();
        assert!(!watcher.claude_sessions.get(sid).unwrap().compacting);
        assert!(watcher.compact_echo_until.contains_key(sid));

        // Stop pendant l'écho de compactage
        std::fs::write(&log_file, format!("Stop\t{sid}\n")).unwrap();
        watcher.claude_log_offset = 0;
        watcher.poll_claude_hooks();
        assert_eq!(watcher.claude_sessions.get(sid).unwrap().phase, SessionPhase::Working);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
