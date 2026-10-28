use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use serde_json::Value;

use crate::pet::MascotKind;
use super::proc::{
    count_inbox_messages, extract_between, extract_exit_code, extract_message_sender,
    extract_task_started_id, parse_bridge_send, read_tail_lines,
};
use super::types::{AgyParsedTranscript, SessionPhase, SessionState};
use super::SessionWatcher;

impl SessionWatcher {
    pub(crate) fn transcript_len_for_agy(&self, conv_id: &str) -> u64 {
        for cand in &self.agy_candidate_dirs {
            let logs = cand.join("brain").join(conv_id).join(".system_generated/logs");
            if let Ok(m) = std::fs::metadata(logs.join("transcript.jsonl")).or_else(|_| std::fs::metadata(logs.join("transcript_full.jsonl"))) {
                return m.len();
            }
        }
        0
    }

    pub(crate) fn poll_agy_hooks(&mut self) {
        let log_path = if self.runtime_dir.join("agy-events.log").exists() {
            self.runtime_dir.join("agy-events.log")
        } else {
            PathBuf::from("/tmp/paros/agy-events.log")
        };
        let Ok(file) = File::open(&log_path) else { return };
        let Ok(meta) = file.metadata() else { return };
        let len = meta.len();

        if len < self.agy_log_offset { self.agy_log_offset = 0; }
        if len == self.agy_log_offset { return; }

        let mut reader = BufReader::new(file);
        if reader.seek(SeekFrom::Start(self.agy_log_offset)).is_err() { return; }

        let now = Instant::now();
        for line in reader.lines().map_while(Result::ok) {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.is_empty() { continue; }
            let event = fields[0];
            let conv_id = fields.get(1).unwrap_or(&"").trim();
            if conv_id.is_empty() { continue; }

            self.agy_last_heard.insert(conv_id.to_string(), now);

            if fields.len() >= 7 {
                let notif = fields[2].trim();
                let tool = fields[3].trim();
                let detail = fields[4].trim();
                let kind = fields[5].trim();
                let reason = fields.get(7).unwrap_or(&"").trim();

                if event == "Stop" && reason != "NO_TOOL_CALL" {
                    let tlen = self.transcript_len_for_agy(conv_id);
                    self.agy_stopped_at_len.insert(conv_id.to_string(), tlen);
                } else if event == "PreToolUse" {
                    if let Some((st, _)) = self.agy_sessions.get_mut(conv_id) {
                        st.phase = SessionPhase::Working;
                        st.activity = tool.to_string();
                        st.detail = detail.to_string();
                    }
                } else if event == "PostToolUse"
                    && let Some((st, _)) = self.agy_sessions.get_mut(conv_id) {
                        if kind == "test" && notif.is_empty() {
                            st.tests_passed = true;
                        } else if !notif.is_empty() {
                            st.tool_failed = Some((Some(tool.to_string()), kind == "test"));
                        }
                    }
            } else {
                let reason = if fields.len() >= 3 { fields[2].trim() } else { "" };
                if event == "Stop" && reason != "NO_TOOL_CALL" {
                    let tlen = self.transcript_len_for_agy(conv_id);
                    self.agy_stopped_at_len.insert(conv_id.to_string(), tlen);
                }
            }
        }
        self.agy_log_offset = len;
    }

    pub(crate) fn find_all_antigravity_convs(&mut self) -> Vec<(String, Option<i32>, PathBuf)> {
        if self.last_proc_scan.elapsed() < Duration::from_secs(2) && !self.cached_agy_convs.is_empty() {
            return self.cached_agy_convs.clone();
        }
        self.last_proc_scan = Instant::now();

        let mut convs: HashMap<String, (Option<i32>, PathBuf)> = HashMap::new();
        let mut ide_pid: Option<i32> = None;

        let mut known_subs: HashSet<String> = HashSet::new();
        for subs in self.agy_known_subagents.values() {
            for (cid, _) in subs {
                known_subs.insert(cid.clone());
            }
        }

        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let pid_str = entry.file_name().to_string_lossy().to_string();
                let Ok(pid) = pid_str.parse::<i32>() else { continue };

                let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
                let trimmed = comm.trim();
                if matches!(trimmed, "language_server" | "agy" | "antigravity" | "antigravity-ide") {
                    if matches!(trimmed, "language_server" | "antigravity" | "antigravity-ide") {
                        ide_pid = Some(pid);
                    }
                    if let Ok(fds) = std::fs::read_dir(format!("/proc/{pid}/fd")) {
                        for fd in fds.flatten() {
                            if let Ok(target) = std::fs::read_link(fd.path())
                                && target.extension().and_then(|e| e.to_str()) == Some("lock") {
                                    let parent_opt = target.parent();
                                    let matched_cand = self.agy_candidate_dirs.iter()
                                        .find(|cand| Some(cand.join("presence").as_path()) == parent_opt)
                                        .cloned()
                                        .or_else(|| parent_opt.filter(|p| p.ends_with("presence")).and_then(|p| p.parent()).map(|p| p.to_path_buf()));

                                    if let Some(data_dir) = matched_cand
                                        && let Some(stem) = target.file_stem().and_then(|s| s.to_str()) {
                                            if self.is_subagent_conv(stem, &data_dir, &known_subs) {
                                                continue;
                                            }
                                            convs.insert(stem.to_string(), (Some(pid), data_dir));
                                        }
                                }
                        }
                    }
                }
            }
        }

        if let Some(ipid) = ide_pid {
            let ide_dir = self.home_dir.join(".gemini/antigravity");
            for (cid, entry) in &self.desktop_meta {
                if entry.not_idle && entry.parent_id.is_empty() && !convs.contains_key(cid) {
                    convs.insert(cid.clone(), (Some(ipid), ide_dir.clone()));
                }
            }
        }

        for (id, (st, dir)) in &self.agy_sessions {
            if convs.contains_key(id) { continue; }
            let is_proc_alive = st.terminal_pid.is_some_and(|pid| Path::new(&format!("/proc/{pid}")).exists());
            if (st.phase == SessionPhase::Working || !st.subagents.is_empty()) && is_proc_alive {
                convs.insert(id.clone(), (st.terminal_pid, dir.clone()));
            }
        }

        let res: Vec<_> = convs.into_iter().map(|(id, (pid, dir))| (id, pid, dir)).collect();
        self.cached_agy_convs = res.clone();
        res
    }

    pub(crate) fn check_all_antigravity(
        &mut self,
        git_enabled: bool,
        groove_enabled: bool,
    ) -> Vec<SessionState> {
        self.sync_desktop_meta();
        let convs = self.find_all_antigravity_convs();
        let mut results = Vec::new();
        let home_str = self.home_dir.to_string_lossy().to_string();
        let home_trimmed = home_str.trim_end_matches('/');

        let mut known_subs: HashSet<String> = HashSet::new();
        for subs in self.agy_known_subagents.values() {
            for (cid, _) in subs {
                known_subs.insert(cid.clone());
            }
        }

        for (conv_id, pid, data_dir) in convs {
            if known_subs.contains(&conv_id) || self.is_subagent_conv(&conv_id, &data_dir, &known_subs) {
                continue;
            }
            let brain_dir = data_dir.join("brain");
            let Some(parsed) = self.parse_agy_transcript(&brain_dir, &conv_id) else { continue };

            let mut phase = parsed.phase;
            let current_tlen = self.agy_transcript_lens.get(&conv_id).copied().unwrap_or(0);
            if let Some(&stopped_len) = self.agy_stopped_at_len.get(&conv_id) {
                if current_tlen > stopped_len {
                    self.agy_stopped_at_len.remove(&conv_id);
                } else if phase == SessionPhase::Working {
                    phase = SessionPhase::Idle;
                }
            }

            let proc_cwd = pid.and_then(|p| std::fs::read_link(format!("/proc/{p}/cwd")).ok()).map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            let proc_cwd_trimmed = proc_cwd.trim_end_matches('/');
            let mut cwd = if proc_cwd_trimmed.is_empty() || proc_cwd_trimmed == home_trimmed {
                parsed.workspace_cwd.clone().unwrap_or_else(|| if !proc_cwd.is_empty() { proc_cwd } else { std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| ".".to_string()) })
            } else {
                proc_cwd
            };
            if let Some(meta) = self.desktop_meta.get(&conv_id)
                && !meta.cwd.is_empty() {
                    cwd = meta.cwd.clone();
                }

            let title = self.read_antigravity_title(&data_dir, &conv_id, &cwd);
            let (branch, conflict, dirty, behind, repo_cleaned) = self.check_git(&cwd, git_enabled);
            let music = if groove_enabled { self.check_mpris() } else { false };
            let rounded = ((parsed.tokens as f32 / 10000.0).round() as usize) * 10000;

            let agy_inbox = self.home_dir.join(format!(".agent-bridge/inbox/agy-{conv_id}"));
            let mail_count = if agy_inbox.exists() { count_inbox_messages(&agy_inbox) } else { count_inbox_messages(&self.home_dir.join(".agent-bridge/inbox")) };

            let start_time = self.phase_start_time(&conv_id, phase);
            let silent_for = self.agy_last_heard.get(&conv_id).map(|t| t.elapsed()).unwrap_or(Duration::ZERO);
            let quiet_level = Self::compute_quiet_level(phase, silent_for);
            let lineage = Self::build_lineage(pid);

            let mut st = SessionState::new(conv_id.clone(), MascotKind::Agy, title, cwd, pid);
            st.branch = branch;
            st.phase = phase;
            st.activity = if phase == SessionPhase::Idle { String::new() } else { parsed.activity };
            st.detail = if phase == SessionPhase::Idle { String::new() } else { parsed.detail };
            st.git_conflict = conflict;
            st.dirty_lines = dirty;
            st.commits_behind = behind;
            st.tokens_k = rounded / 1000;
            st.context_percent = (rounded as f32 / 1_000_000.0).clamp(0.0, 1.0);
            st.subagents_count = parsed.subagents_count;
            st.subagents = parsed.subagents;
            st.last_prompt = parsed.last_prompt;
            st.summary = parsed.summary;
            st.has_background = parsed.has_bg;
            st.compacting = parsed.is_compacting;
            st.is_music_playing = music;
            st.start_time = start_time;
            st.mail = mail_count;
            st.message_sent = if phase == SessionPhase::Idle { None } else { parsed.message_sent };
            st.servers = parsed.servers;
            st.tool_failed = parsed.tool_failed;
            st.tests_passed = parsed.tests_passed;
            st.repo_cleaned = repo_cleaned;
            st.quiet_level = quiet_level;
            st.turn_origin_peer = parsed.turn_origin_peer;
            st.lineage = lineage;

            self.agy_sessions.insert(conv_id.clone(), (st.clone(), data_dir.clone()));
            results.push(st);
        }
        self.agy_sessions.retain(|k, _| results.iter().any(|s| &s.id == k));
        self.agy_tasks.retain(|k, _| results.iter().any(|s| &s.id == k));
        self.agy_stopped_at_len.retain(|k, _| results.iter().any(|s| &s.id == k));
        self.agy_last_heard.retain(|k, _| results.iter().any(|s| &s.id == k));
        self.agy_transcript_lens.retain(|k, _| results.iter().any(|s| &s.id == k));
        self.agy_known_subagents.retain(|k, _| results.iter().any(|s| &s.id == k));
        self.agy_parsed_cache.retain(|k, _| results.iter().any(|s| &s.id == k));
        self.agy_subagent_history_cache.retain(|k, _| results.iter().any(|s| &s.id == k));

        let all_active_subs: HashSet<String> = self.agy_known_subagents.values().flat_map(|subs| subs.iter().map(|(id, _)| id.clone())).collect();
        self.agy_subagent_lens.retain(|k, _| all_active_subs.contains(k));

        results
    }

    #[allow(dead_code)]
    pub(crate) fn poll_agy_transcripts(&mut self, brain_dir: &Path, conv_id: &str) -> Option<AgyParsedTranscript> {
        self.parse_agy_transcript(brain_dir, conv_id)
    }

    pub(crate) fn parse_agy_transcript(&mut self, brain_dir: &Path, conv_id: &str) -> Option<AgyParsedTranscript> {
        let conv_dir = brain_dir.join(conv_id);
        let logs_dir = conv_dir.join(".system_generated/logs");
        let compact_path = logs_dir.join("transcript.jsonl");
        let path = if compact_path.exists() { compact_path } else { logs_dir.join("transcript_full.jsonl") };

        let file = File::open(&path).ok()?;
        let metadata = file.metadata().ok()?;
        let file_len = metadata.len();
        if file_len == 0 { return None; }

        let mtime_elapsed = metadata.modified().ok().and_then(|m| m.elapsed().ok()).unwrap_or(Duration::ZERO);
        let prev_len = self.agy_transcript_lens.insert(conv_id.to_string(), file_len);
        let is_first_sight = prev_len.is_none();
        if let Some(pl) = prev_len {
            if file_len > pl {
                self.agy_last_heard.insert(conv_id.to_string(), Instant::now());
            } else if file_len == pl {
                // Court-circuit : taille inchangée, réutilisation immédiate du cache tail/JSON
                if let Some(mut cached) = self.agy_parsed_cache.get(conv_id).cloned() {
                    let mut tool_failed = None;
                    let subagents = self.parse_subagents_for_transcript(brain_dir, &path, conv_id, &mut tool_failed);
                    cached.subagents_count = subagents.len();
                    cached.subagents = subagents;
                    if tool_failed.is_some() {
                        cached.tool_failed = tool_failed;
                    }
                    return Some(cached);
                }
            }
        } else {
            let initial_heard = Instant::now().checked_sub(mtime_elapsed).unwrap_or_else(Instant::now);
            self.agy_last_heard.insert(conv_id.to_string(), initial_heard);
        }

        let lines = read_tail_lines(&file, file_len, 131072)?;
        let mut phase = SessionPhase::Idle;
        let mut activity = String::new();
        let mut detail = String::new();
        let mut tokens = 0usize;
        let mut is_compacting = false;
        let mut schedule_bg = false;
        let mut last_prompt = String::new();
        let mut summary = String::new();
        let mut message_sent: Option<String> = None;
        let mut workspace_cwd: Option<String> = None;
        let mut turn_origin_peer = false;
        let mut tool_failed: Option<(Option<String>, bool)> = None;
        let mut tests_passed = false;
        let mut pending_calls: Vec<(String, Value)> = Vec::new();
        let mut tasks: HashMap<String, bool> = self.agy_tasks.remove(conv_id).unwrap_or_default();

        for (line_offset, line) in lines {
            let is_new_line = prev_len.is_some_and(|pl| line_offset >= pl);
            let Ok(step) = serde_json::from_str::<Value>(&line) else { continue };
            let step_type = step.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let source = step.get("source").and_then(|s| s.as_str()).unwrap_or("");
            let content = step.get("content").and_then(|c| c.as_str()).unwrap_or("");

            match step_type {
                "USER_INPUT" => {
                    phase = SessionPhase::Working;
                    activity = "Réflexion...".to_string();
                    detail.clear();
                    is_compacting = false;
                    pending_calls.clear();
                    message_sent = None;

                    if content.contains("<CONTEXT_SUMMARY>")
                        && let Some(s_text) = extract_between(content, "<summary>", "</summary>") {
                            summary = s_text.trim().replace('\n', " ");
                        }

                    if workspace_cwd.is_none() && content.contains("Active Workspaces:")
                        && let Some(idx) = content.find("Active Workspaces:") {
                            for wline in content[idx + 18..].lines().take(5) {
                                if let Some(rest) = wline.trim().strip_prefix("- ") {
                                    let ws = rest.trim();
                                    if !ws.is_empty() { workspace_cwd = Some(ws.to_string()); break; }
                                }
                            }
                        }

                    if content.contains("<bridge-message") || content.contains("<cross-session-message") {
                        turn_origin_peer = true;
                    } else {
                        turn_origin_peer = false;
                        if let Some(req) = extract_between(content, "<USER_REQUEST>", "</USER_REQUEST>") {
                            let cleaned = req.trim().replace('\n', " ");
                            if !cleaned.is_empty() { last_prompt = cleaned; }
                        } else {
                            let first_line = content.lines().next().unwrap_or("").trim();
                            if !first_line.is_empty() { last_prompt = first_line.to_string(); }
                        }
                    }
                }
                "PLANNER_RESPONSE" => {
                    let inp = step.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    let cache = step.get("cache_read_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    if inp + cache > 0 { tokens = (inp + cache) as usize; }

                    if let Some(calls) = step.get("tool_calls").and_then(|c| c.as_array()) {
                        if !calls.is_empty() {
                            is_compacting = false;
                            pending_calls.clear();
                            for call in calls {
                                let cname = call.get("name").and_then(|n| n.as_str()).unwrap_or("Outil");
                                let cargs = call.get("args").cloned().unwrap_or(Value::Null);
                                pending_calls.push((cname.to_string(), cargs.clone()));

                                if cname == "manage_task" && cargs.get("Action").and_then(|v| v.as_str()) == Some("kill")
                                    && let Some(tid) = cargs.get("TaskId").and_then(|v| v.as_str()) { tasks.remove(tid); }
                                if cname == "run_command"
                                    && let Some(cmd) = cargs.get("CommandLine").and_then(|v| v.as_str())
                                        && let Some(target) = parse_bridge_send(cmd) { message_sent = Some(target); }
                                if cname == "schedule" { schedule_bg = true; }
                            }

                            if let Some(ask_call) = calls.iter().find(|c| c.get("name").and_then(|n| n.as_str()) == Some("ask_question")) {
                                let q_text = ask_call.get("args")
                                    .and_then(|a| a.get("questions")).and_then(|q| q.as_array())
                                    .and_then(|qa| qa.first()).and_then(|q0| q0.get("question")).and_then(|q| q.as_str()).unwrap_or("Question");
                                phase = SessionPhase::Waiting;
                                activity = "Question".to_string();
                                detail = q_text.to_string();
                            } else {
                                let first_call = &calls[0];
                                let tool_name = first_call.get("name").and_then(|n| n.as_str()).unwrap_or("Outil");
                                phase = SessionPhase::Working;
                                activity = tool_name.to_string();
                                detail = Self::extract_agy_tool_detail(first_call.get("args"));
                            }
                        } else {
                            phase = SessionPhase::Idle;
                            activity.clear();
                            detail.clear();
                            pending_calls.clear();
                        }
                    } else {
                        phase = SessionPhase::Idle;
                        activity.clear();
                        detail.clear();
                        pending_calls.clear();
                    }
                }
                "CHECKPOINT" => {
                    tokens = 0;
                    is_compacting = true;
                }
                "SYSTEM_MESSAGE" => {
                    if source == "SYSTEM"
                        && let Some(sender) = extract_message_sender(content) {
                            tasks.remove(&sender);
                            phase = SessionPhase::Working;
                            activity = "Réflexion...".to_string();
                            detail.clear();
                        }
                }
                _ => {
                    if source == "MODEL" {
                        let popped = (!pending_calls.is_empty()).then(|| pending_calls.remove(0));
                        let status = step.get("status").and_then(|v| v.as_str()).unwrap_or("");
                        let (call_name, call_args) = popped.as_ref().map(|(n, a)| (n.as_str(), Some(a))).unwrap_or(("", None));
                        let cmd = call_args.and_then(|a| a.get("CommandLine")).and_then(|v| v.as_str()).unwrap_or("");
                        let is_daemon = call_args.and_then(|a| a.get("IsDaemon")).and_then(|v| v.as_bool()).unwrap_or(false);

                        if status == "RUNNING" {
                            if let Some(tid) = extract_task_started_id(content) {
                                let is_srv = call_name == "run_command" && (Self::is_server(cmd) || is_daemon);
                                tasks.insert(tid, is_srv);
                            }
                        } else if !is_first_sight && is_new_line {
                            let is_test = Self::is_test_command(cmd);
                            if status == "ERROR" {
                                tool_failed = Some((None, is_test));
                            } else if (call_name == "run_command" || content.contains("The command exited with code "))
                                && let Some(code) = extract_exit_code(content) {
                                    if code != 0 {
                                        tool_failed = Some((None, is_test));
                                    } else if is_test {
                                        tests_passed = true;
                                    }
                                }
                        }
                    }
                }
            }
        }

        if workspace_cwd.is_none() && file_len > 131072
            && let Ok(mut start_file) = File::open(&path) {
                use std::io::Read;
                let mut head_buf = vec![0u8; 32768.min(file_len as usize)];
                if start_file.read_exact(&mut head_buf).is_ok() {
                    let head_str = String::from_utf8_lossy(&head_buf);
                    if let Some(idx) = head_str.find("Active Workspaces:") {
                        for wline in head_str[idx + 18..].lines().take(5) {
                            if let Some(rest) = wline.trim().strip_prefix("- ") {
                                let ws = rest.trim();
                                if !ws.is_empty() { workspace_cwd = Some(ws.to_string()); break; }
                            }
                        }
                    }
                }
            }

        const STALE_SECONDS: u64 = 600;
        let is_known = self.agy_sessions.contains_key(conv_id);
        if !is_known && phase == SessionPhase::Working && mtime_elapsed > Duration::from_secs(STALE_SECONDS) {
            phase = SessionPhase::Idle;
            activity.clear();
            detail.clear();
        }

        let servers = tasks.values().filter(|&&is_srv| is_srv).count();
        let bg_tasks = tasks.values().filter(|&&is_srv| !is_srv).count();
        let has_bg = bg_tasks > 0 || schedule_bg;
        self.agy_tasks.insert(conv_id.to_string(), tasks);

        let subagents = self.parse_subagents_for_transcript(brain_dir, &path, conv_id, &mut tool_failed);

        let subagents_count = subagents.len();
        let result = AgyParsedTranscript {
            phase,
            activity,
            detail,
            tokens,
            is_compacting,
            has_bg,
            servers,
            last_prompt,
            summary,
            subagents_count,
            subagents,
            message_sent,
            workspace_cwd,
            turn_origin_peer,
            tool_failed,
            tests_passed,
        };
        self.agy_parsed_cache.insert(conv_id.to_string(), result.clone());
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_agy_checkpoint_sets_compacting() {
        let temp_dir = std::env::temp_dir().join(format!("paros_test_agy_{}", fastrand::u64(..)));
        let brain_dir = temp_dir.join("brain");
        let conv_id = "test_conv_checkpoint";
        let logs_dir = brain_dir.join(conv_id).join(".system_generated/logs");
        std::fs::create_dir_all(&logs_dir).unwrap();
        let transcript = logs_dir.join("transcript.jsonl");

        {
            let mut f = File::create(&transcript).unwrap();
            writeln!(f, "{{\"type\":\"USER_INPUT\",\"source\":\"USER\",\"content\":\"hello\"}}").unwrap();
            writeln!(f, "{{\"type\":\"CHECKPOINT\",\"source\":\"SYSTEM\",\"content\":\"checkpoint\"}}").unwrap();
            writeln!(f, "{{\"type\":\"GENERIC\",\"source\":\"MODEL\",\"content\":\"step after checkpoint\"}}").unwrap();
        }

        let mut watcher = SessionWatcher::new();
        let parsed = watcher.parse_agy_transcript(&brain_dir, conv_id).unwrap();
        assert!(parsed.is_compacting);
        assert_eq!(parsed.tokens, 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_agy_task_persistence_across_polls() {
        let temp_dir = std::env::temp_dir().join(format!("paros_test_agy_task_{}", fastrand::u64(..)));
        let brain_dir = temp_dir.join("brain");
        let conv_id = "test_conv_tasks";
        let logs_dir = brain_dir.join(conv_id).join(".system_generated/logs");
        std::fs::create_dir_all(&logs_dir).unwrap();
        let transcript = logs_dir.join("transcript.jsonl");

        let mut watcher = SessionWatcher::new();
        watcher.agy_tasks.entry(conv_id.to_string()).or_default().insert("task_123".into(), false);

        {
            let mut f = File::create(&transcript).unwrap();
            writeln!(f, "{{\"type\":\"USER_INPUT\",\"source\":\"USER\",\"content\":\"hello\"}}").unwrap();
        }

        let parsed = watcher.parse_agy_transcript(&brain_dir, conv_id).unwrap();
        assert!(parsed.has_bg);
        assert_eq!(watcher.agy_tasks.get(conv_id).unwrap().len(), 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

