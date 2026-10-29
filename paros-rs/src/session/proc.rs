use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

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

impl SessionWatcher {
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
