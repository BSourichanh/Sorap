use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use serde_json::Value;

use super::types::ClaudeTranscriptData;
use super::SessionWatcher;

pub fn parse_bridge_send(cmd: &str) -> Option<String> {
    let bridge_idx = cmd.find("bridge.py")?;
    let rem = &cmd[bridge_idx + 9..];
    let send_idx = rem.find("send")?;
    let after_send = rem[send_idx + 4..].trim_start();
    if after_send.is_empty() {
        return None;
    }
    if let Some(stripped) = after_send.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(stripped[..end].to_string())
    } else {
        let recipient = after_send.split_whitespace().next()?;
        Some(recipient.trim_matches('"').trim_matches('\'').to_string())
    }
}

pub fn extract_between<'a>(s: &'a str, start_pat: &str, end_pat: &str) -> Option<&'a str> {
    let start = s.find(start_pat)? + start_pat.len();
    let rem = &s[start..];
    let end = rem.find(end_pat)?;
    Some(&rem[..end])
}

pub fn extract_json_str_field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("\"{key}\":\"");
    if let Some(idx) = line.find(&pattern) {
        let after = &line[idx + pattern.len()..];
        let end = after.find('"')?;
        return Some(&after[..end]);
    }
    let pattern_sp = format!("\"{key}\": \"");
    if let Some(idx) = line.find(&pattern_sp) {
        let after = &line[idx + pattern_sp.len()..];
        let end = after.find('"')?;
        return Some(&after[..end]);
    }
    None
}

pub fn extract_exit_code(content: &str) -> Option<i32> {
    let marker = "The command exited with code ";
    let idx = content.find(marker)?;
    let after = &content[idx + marker.len()..];
    let num_str: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    num_str.parse::<i32>().ok()
}

pub fn extract_task_started_id(content: &str) -> Option<String> {
    let marker = "running as a background task with task id: ";
    let idx = content.find(marker)?;
    let after = &content[idx + marker.len()..];
    let id: String = after.chars().take_while(|c| !c.is_whitespace()).collect();
    if id.is_empty() { None } else { Some(id) }
}

pub fn extract_message_sender(content: &str) -> Option<String> {
    let idx = content.find("[Message]")?;
    let line = content[idx..].lines().next()?;
    let s_idx = line.find(" sender=")?;
    let after = &line[s_idx + 8..];
    let sender: String = after.chars().take_while(|c| !c.is_whitespace()).collect();
    if sender.is_empty() { None } else { Some(sender) }
}

pub fn decode_pbtxt_escaped(raw: &str) -> String {
    let bytes_in = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes_in.len());
    let mut i = 0;
    while i < bytes_in.len() {
        if bytes_in[i] == b'\\' && i + 1 < bytes_in.len() {
            if i + 3 < bytes_in.len()
                && (b'0'..=b'7').contains(&bytes_in[i + 1])
                && (b'0'..=b'7').contains(&bytes_in[i + 2])
                && (b'0'..=b'7').contains(&bytes_in[i + 3])
            {
                let val = (bytes_in[i + 1] - b'0') * 64
                    + (bytes_in[i + 2] - b'0') * 8
                    + (bytes_in[i + 3] - b'0');
                out.push(val);
                i += 4;
                continue;
            }
            let next = bytes_in[i + 1];
            match next {
                b'n' => out.push(b'\n'),
                b't' => out.push(b'\t'),
                b'r' => out.push(b'\r'),
                b'"' => out.push(b'"'),
                b'\\' => out.push(b'\\'),
                other => out.push(other),
            }
            i += 2;
            continue;
        }
        out.push(bytes_in[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

pub fn extract_pbtxt_title_raw(content: &str) -> Option<String> {
    let idx = content.find("title:")?;
    let after_colon = content[idx + 6..].trim_start();
    let after_quote = after_colon.strip_prefix('"')?;
    let bytes = after_quote.as_bytes();
    let mut end = 0;
    while end < bytes.len() {
        if bytes[end] == b'\\' {
            end += 2;
            continue;
        }
        if bytes[end] == b'"' {
            return Some(decode_pbtxt_escaped(&after_quote[..end]));
        }
        end += 1;
    }
    None
}

pub fn count_inbox_messages(inbox_dir: &Path) -> usize {
    if let Ok(entries) = std::fs::read_dir(inbox_dir) {
        entries
            .flatten()
            .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
            .count()
    } else {
        0
    }
}

pub fn read_tail_lines(mut file: &File, file_len: u64, max_bytes: u64) -> Option<Vec<(u64, String)>> {
    let seek_pos = file_len.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(seek_pos)).ok()?;

    let mut reader = BufReader::new(file);
    let mut lines = Vec::new();
    let mut cursor = seek_pos;
    let mut buf = String::new();
    while let Ok(bytes_read) = reader.read_line(&mut buf) {
        if bytes_read == 0 {
            break;
        }
        let line_start = cursor;
        cursor += bytes_read as u64;
        let trimmed = buf.trim_end();
        if !trimmed.trim().is_empty() {
            lines.push((line_start, trimmed.to_string()));
        }
        buf.clear();
    }
    if lines.is_empty() {
        return None;
    }
    if seek_pos > 0 && lines.len() > 1 {
        Some(lines[1..].to_vec())
    } else {
        Some(lines)
    }
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
    pub fn is_server(command: &str) -> bool {
        let cmd = command.trim();
        if cmd.is_empty() {
            return false;
        }
        let server_bins = [
            "vite", "nodemon", "webpack-dev-server", "http-server", "live-server", "browser-sync", "uvicorn", "gunicorn",
        ];
        for token in cmd.split(|c: char| c.is_whitespace() || matches!(c, ';' | '&' | '|' | '(' | ')' | '/')) {
            if server_bins.contains(&token) {
                return true;
            }
        }
        let pkg_mgrs = ["npm", "pnpm", "yarn", "bun"];
        let pkg_cmds = ["dev", "start", "serve", "watch", "preview"];
        let words: Vec<&str> = cmd.split_whitespace().collect();
        for i in 0..words.len() {
            let w = words[i].trim_matches(|c: char| matches!(c, ';' | '&' | '|' | '(' | ')'));
            if pkg_mgrs.contains(&w)
                && let Some(&next) = words.get(i + 1) {
                    let next_clean = next.trim_matches(|c: char| matches!(c, ';' | '&' | '|' | '(' | ')'));
                    if pkg_cmds.contains(&next_clean) {
                        return true;
                    }
                    if next_clean == "run"
                        && let Some(&third) = words.get(i + 2) {
                            let third_clean = third.trim_matches(|c: char| matches!(c, ';' | '&' | '|' | '(' | ')'));
                            if pkg_cmds.contains(&third_clean) {
                                return true;
                            }
                        }
                }
            if matches!(w, "next" | "astro" | "nuxt")
                && words.get(i + 1).map(|s| s.trim_matches(|c: char| matches!(c, ';' | '&' | '|' | '(' | ')'))) == Some("dev") {
                    return true;
                }
            if matches!(w, "ng" | "jekyll" | "hugo")
                && words.get(i + 1).map(|s| s.trim_matches(|c: char| matches!(c, ';' | '&' | '|' | '(' | ')'))) == Some("serve") {
                    return true;
                }
            if w == "runserver" || w == "--watch" {
                return true;
            }
        }
        cmd.contains("-m http.server")
            || cmd.contains("flask run")
            || cmd.contains("php -S ")
            || cmd.contains("docker compose up")
            || cmd.contains("tail -f")
    }

    pub fn is_test_command(command: &str) -> bool {
        let cmd = command.trim();
        if cmd.is_empty() {
            return false;
        }
        if cmd.contains("./test.sh") || cmd.ends_with("test.sh") {
            return true;
        }
        let test_bins = ["pytest", "jest", "vitest", "phpunit", "rspec", "ctest"];
        for token in cmd.split(|c: char| c.is_whitespace() || matches!(c, '/' | ';' | '&' | '|')) {
            if test_bins.contains(&token) {
                return true;
            }
        }
        let runners = [
            "npm", "pnpm", "yarn", "bun", "cargo", "go", "dotnet", "mvn", "gradle", "make", "composer",
        ];
        let words: Vec<&str> = cmd.split_whitespace().collect();
        for i in 0..words.len() {
            let w = words[i].trim_matches(|c: char| matches!(c, ';' | '&' | '|' | '(' | ')'));
            if runners.contains(&w)
                && let Some(&next) = words.get(i + 1) {
                    let next_clean = next.trim_matches(|c: char| matches!(c, ';' | '&' | '|' | '(' | ')'));
                    if next_clean == "test" {
                        return true;
                    }
                    if next_clean == "run"
                        && let Some(&third) = words.get(i + 2) {
                            let third_clean = third.trim_matches(|c: char| matches!(c, ';' | '&' | '|' | '(' | ')'));
                            if third_clean == "test" {
                                return true;
                            }
                        }
                }
        }
        false
    }

    pub fn parse_claude_color(s: &str) -> Option<(u8, u8, u8)> {
        match s.trim().to_lowercase().as_str() {
            "red" => Some((0xe5, 0x48, 0x4d)),
            "blue" => Some((0x4a, 0x90, 0xe2)),
            "green" => Some((0x46, 0xa7, 0x58)),
            "yellow" => Some((0xe2, 0xb9, 0x3b)),
            "purple" => Some((0x8e, 0x6f, 0xd8)),
            "orange" => Some((0xf0, 0x8c, 0x3a)),
            "pink" => Some((0xe8, 0x6a, 0xa6)),
            "cyan" => Some((0x3b, 0xb8, 0xc4)),
            hex if hex.starts_with('#') && hex.len() == 7 => {
                let r = u8::from_str_radix(&hex[1..3], 16).ok()?;
                let g = u8::from_str_radix(&hex[3..5], 16).ok()?;
                let b = u8::from_str_radix(&hex[5..7], 16).ok()?;
                Some((r, g, b))
            }
            _ => None,
        }
    }

    pub fn build_lineage(pid: Option<i32>) -> Vec<i32> {
        let Some(mut cur) = pid else { return Vec::new() };
        let mut pids = Vec::new();
        while cur > 1 && pids.len() < 12 {
            pids.push(cur);
            let status_path = format!("/proc/{cur}/status");
            let Ok(content) = std::fs::read_to_string(&status_path) else { break };
            let mut next_pid = 0;
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("PPid:") {
                    next_pid = rest.trim().parse::<i32>().unwrap_or(0);
                    break;
                }
            }
            if next_pid <= 1 || next_pid == cur {
                break;
            }
            cur = next_pid;
        }
        pids
    }

    pub fn extract_agy_tool_detail(args: Option<&serde_json::Value>) -> String {
        let Some(a) = args else { return String::new() };
        let path_keys = ["Path", "AbsolutePath", "TargetFile", "File", "DirectoryPath", "SearchPath", "SearchDirectory"];
        for k in path_keys {
            if let Some(s) = a.get(k).and_then(|v| v.as_str()) {
                let raw = s.trim_matches(|c| c == '"' || c == '\'');
                return Path::new(raw).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| raw.to_string());
            }
        }
        let text_keys = ["Url", "query", "Query", "Pattern", "toolSummary", "CommandLine"];
        for k in text_keys {
            if let Some(s) = a.get(k).and_then(|v| v.as_str()) {
                return s.trim_matches(|c| c == '"' || c == '\'').to_string();
            }
        }
        String::new()
    }

    pub(crate) fn read_antigravity_title(&self, data_dir: &Path, conv_id: &str, cwd: &str) -> String {
        if let Some(entry) = self.desktop_meta.get(conv_id)
            && !entry.title.is_empty() && entry.title != "New Conversation" {
                return entry.title.clone();
            }
        let annot_path = data_dir.join("annotations").join(format!("{conv_id}.pbtxt"));
        if let Ok(content) = std::fs::read_to_string(&annot_path)
            && let Some(title) = extract_pbtxt_title_raw(&content)
                && !title.is_empty() && title != "New Conversation" {
                    return title;
                }
        for cand in &self.agy_candidate_dirs {
            let p = cand.join("annotations").join(format!("{conv_id}.pbtxt"));
            if p != annot_path
                && let Ok(content) = std::fs::read_to_string(&p)
                    && let Some(title) = extract_pbtxt_title_raw(&content)
                        && !title.is_empty() && title != "New Conversation" {
                            return title;
                        }
        }
        Path::new(cwd).file_name().map(|f| f.to_string_lossy().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "Antigravity".to_string())
    }
}
