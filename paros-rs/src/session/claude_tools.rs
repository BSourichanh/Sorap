use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::Instant;
use serde_json::Value;

use super::proc::{
    extract_between, extract_json_str_field, parse_bridge_send,
};
use super::types::ClaudeTranscriptData;
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

#[allow(clippy::too_many_arguments)]
pub fn parse_claude_transcript_line(
    trimmed: &str,
    data: &mut ClaudeTranscriptData,
    queue: &mut Vec<bool>,
    launches: &mut HashMap<String, bool>,
    tasks: &mut HashMap<String, bool>,
    writes_by_call: &mut HashMap<String, String>,
    last_write_path: &mut Option<String>,
    mut on_write: impl FnMut(&str),
) {
    if trimmed.starts_with("{\"type\":\"agent-color\"") {
        if let Some(col_name) = extract_json_str_field(trimmed, "agentColor").or_else(|| extract_json_str_field(trimmed, "color"))
            && let Some(rgb) = SessionWatcher::parse_claude_color(col_name) {
                data.custom_color = Some(rgb);
            }
    } else if trimmed.starts_with("{\"type\":\"last-prompt\"")
        && let Some(lp) = extract_json_str_field(trimmed, "lastPrompt") {
            data.last_prompt = Some(lp.to_string());
        }

    if trimmed.contains("\"subtype\":\"compact_boundary\"") {
        data.context_tokens = 0;
        data.compacting_reset = true;
    }

    if trimmed.contains("\"subtype\":\"away_summary\"")
        && let Ok(val) = serde_json::from_str::<Value>(trimmed)
            && let Some(c) = val.get("content").and_then(|v| v.as_str()) {
                data.summary = Some(c.trim_end_matches(" (disable recaps in /config)").to_string());
            }

    if trimmed.starts_with("{\"type\":\"queue-operation\"") {
        let is_letter = trimmed.contains("<cross-session-message") || trimmed.contains("<bridge-message");
        if let Some(op) = extract_json_str_field(trimmed, "operation") {
            match op {
                "enqueue" => {
                    queue.push(is_letter);
                    if let Some(tid) = extract_between(trimmed, "<task-id>", "</task-id>") {
                        tasks.remove(tid.trim());
                    }
                }
                "dequeue" => {
                    if !queue.is_empty() {
                        queue.remove(0);
                    }
                }
                "remove" => {
                    if let Some(pos) = queue.iter().position(|&b| b == is_letter) {
                        queue.remove(pos);
                    }
                }
                "popAll" => queue.clear(),
                _ => {}
            }
        }
    }

    if let Some(tid) = extract_between(trimmed, "<task-id>", "</task-id>") {
        tasks.remove(tid.trim());
    }

    if trimmed.contains("\"type\":\"advisor_tool_result\"") {
        data.advising = false;
    } else if trimmed.contains("\"type\":\"server_tool_use\"") && trimmed.contains("\"name\":\"advisor\"") {
        data.advising = true;
    }

    if trimmed.contains("\"backgroundTaskId\":\"") || trimmed.contains("\"status\":\"async_launched\"") {
        let tid = extract_json_str_field(trimmed, "backgroundTaskId").or_else(|| extract_json_str_field(trimmed, "agentId"));
        let call_id = extract_json_str_field(trimmed, "tool_use_id").unwrap_or("");
        if let Some(task_id) = tid {
            let is_srv = launches.remove(call_id).unwrap_or(false);
            tasks.insert(task_id.to_string(), is_srv);
        }
    }

    if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
        if data.cwd.is_none()
            && let Some(c) = val.get("cwd").and_then(|v| v.as_str())
                && !c.is_empty() {
                    data.cwd = Some(c.to_string());
                }

        if let Some(origin_kind) = val.get("origin").and_then(|o| o.get("kind")).and_then(|k| k.as_str()) {
            data.turn_origin_peer = origin_kind == "peer";
            data.summary = Some(String::new());
        }

        let msg_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if msg_type == "user" || msg_type == "human" {
            let is_tool_result = val.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_array())
                .is_some_and(|arr| arr.iter().any(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_result")));
            if !is_tool_result {
                if trimmed.contains("<cross-session-message") || trimmed.contains("<bridge-message") {
                    data.turn_origin_peer = true;
                } else if !trimmed.contains("\"origin\":") {
                    data.turn_origin_peer = false;
                }
                if let Some(color_arg) = extract_between(trimmed, "<command-name>/color</command-name>", "</command-args>") {
                    if let Some(idx) = color_arg.rfind("<command-args>") {
                        let cname = &color_arg[idx + 14..];
                        if let Some(rgb) = SessionWatcher::parse_claude_color(cname) {
                            data.custom_color = Some(rgb);
                        }
                    }
                } else if let Some(c_str) = val.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str())
                    && let Some(rest) = c_str.trim().strip_prefix("/color ")
                        && let Some(rgb) = SessionWatcher::parse_claude_color(rest) {
                            data.custom_color = Some(rgb);
                        }
            }
        }

        if let Some(usage) = val.get("message").and_then(|m| m.get("usage")).or_else(|| val.get("usage")) {
            let inp = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            let cc = usage.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            let cr = usage.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            let total = (inp + cc + cr) as usize;
            if total > 0 {
                data.context_tokens = total;
            }
        }

        if let Some(content_arr) = val.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_array()) {
            for block in content_arr {
                if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                    data.advising = false;
                    let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let input = block.get("input");

                    if name == "Bash" {
                        let run_bg = input.and_then(|i| i.get("run_in_background")).and_then(|v| v.as_bool()).unwrap_or(false);
                        let cmd = input.and_then(|i| i.get("command")).and_then(|v| v.as_str()).unwrap_or("");
                        if run_bg {
                            launches.insert(id.to_string(), SessionWatcher::is_server(cmd));
                        }
                        if let Some(target) = parse_bridge_send(cmd) {
                            data.message_sent = Some(target);
                        }
                    } else if name == "TaskStop" {
                        if let Some(tid) = input.and_then(|i| i.get("task_id").or_else(|| i.get("shell_id"))).and_then(|v| v.as_str()) {
                            tasks.remove(tid);
                        }
                    } else if matches!(name, "Edit" | "Write" | "MultiEdit" | "NotebookEdit") {
                        if let Some(p) = input.and_then(|i| i.get("file_path").or_else(|| i.get("notebook_path"))).and_then(|v| v.as_str())
                            && !p.is_empty() {
                                writes_by_call.insert(id.to_string(), p.to_string());
                                *last_write_path = Some(p.to_string());
                                on_write(p);
                            }
                    } else if (name == "SendMessage" || name == "mcp__agent-bridge__send")
                        && let Some(to) = input.and_then(|i| i.get("to").or_else(|| i.get("recipient"))).and_then(|v| v.as_str())
                            && !to.is_empty() {
                                data.message_sent = Some(to.to_string());
                            }
                }
            }
        }
    }
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
            }
            line_buf.clear();
        }

        state.offset = last_valid_offset;
        state.data.queue_mail = state.queue.iter().filter(|&&is_let| is_let).count();
        state.data.servers_count = state.tasks.values().filter(|&&srv| srv).count();
        state.data.background_count = state.tasks.len() - state.data.servers_count;

        let ret = state.data.clone();
        self.claude_transcript_offsets.insert(session_id.to_string(), last_valid_offset);
        self.transcript_offsets.insert(session_id.to_string(), last_valid_offset);
        self.claude_transcript_states.insert(session_id.to_string(), state);
        Some(ret)
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

        let session_dir = projects_dir.join("proj_1");
        std::fs::create_dir_all(&session_dir).unwrap();
        let session_id = "test_session_1";
        let transcript_file = session_dir.join(format!("{session_id}.jsonl"));

        {
            let mut f = File::create(&transcript_file).unwrap();
            writeln!(f, "{{\"type\":\"agent-color\",\"color\":\"red\"}}").unwrap();
            writeln!(f, "{{\"type\":\"last-prompt\",\"lastPrompt\":\"hello world\"}}").unwrap();
        }

        let data1 = watcher.poll_claude_transcript(session_id, "test").unwrap();
        assert_eq!(data1.custom_color, Some((0xe5, 0x48, 0x4d)));
        assert_eq!(data1.last_prompt.as_deref(), Some("hello world"));

        // Seconde lecture sans modification : doit réutiliser l'état sans re-parser
        let data2 = watcher.poll_claude_transcript(session_id, "test").unwrap();
        assert_eq!(data2.last_prompt.as_deref(), Some("hello world"));

        // Écriture incrémentale
        {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&transcript_file)
                .unwrap();
            writeln!(f, "{{\"type\":\"last-prompt\",\"lastPrompt\":\"updated prompt\"}}").unwrap();
        }

        let data3 = watcher.poll_claude_transcript(session_id, "test").unwrap();
        assert_eq!(data3.last_prompt.as_deref(), Some("updated prompt"));

        // Tronquage / rotation du fichier
        {
            let mut f = File::create(&transcript_file).unwrap();
            writeln!(f, "{{\"type\":\"agent-color\",\"color\":\"blue\"}}").unwrap();
            writeln!(f, "{{\"type\":\"last-prompt\",\"lastPrompt\":\"reset prompt\"}}").unwrap();
        }

        let data4 = watcher.poll_claude_transcript(session_id, "test").unwrap();
        assert_eq!(data4.custom_color, Some((0x4a, 0x90, 0xe2)));
        assert_eq!(data4.last_prompt.as_deref(), Some("reset prompt"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
