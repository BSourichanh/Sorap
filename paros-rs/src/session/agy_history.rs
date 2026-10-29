use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use serde_json::Value;

use super::proc::{
    extract_exit_code, extract_message_sender, extract_task_started_id, read_tail_lines,
};
use super::types::{DesktopMetaEntry, SubagentInfo};
use super::SessionWatcher;

/// Extrait l'historique des invocations de sous-agents (`invoke_subagent`) et annulations depuis un log.
pub fn parse_agy_subagent_history(path: &Path) -> (Vec<(String, String)>, HashSet<String>) {
    let mut subagent_pairs = Vec::new();
    let mut canceled_cids = HashSet::new();
    let Ok(full_file) = File::open(path) else {
        return (subagent_pairs, canceled_cids);
    };
    let full_reader = BufReader::new(full_file);
    let mut pending_roles = Vec::new();

    for line in full_reader.lines().map_while(Result::ok) {
        if line.contains("has been explicitly canceled")
            && let Some(p1) = line.find("Subagent \"") {
                let rem = &line[p1 + 10..];
                if let Some(p2) = rem.find('"') {
                    canceled_cids.insert(rem[..p2].to_string());
                }
            }
        if (line.contains("invoke_subagent") || !pending_roles.is_empty())
            && let Ok(step) = serde_json::from_str::<Value>(&line) {
                let step_type = step.get("type").and_then(|t| t.as_str()).unwrap_or("");
                let source = step.get("source").and_then(|s| s.as_str()).unwrap_or("");
                if step_type == "PLANNER_RESPONSE" {
                    if let Some(calls) = step.get("tool_calls").and_then(|c| c.as_array()) {
                        for call in calls {
                            if call.get("name").and_then(|n| n.as_str()) == Some("invoke_subagent") {
                                let args = call.get("args");
                                if let Some(sub_list) = args
                                    .and_then(|a| a.get("Subagents"))
                                    .and_then(|s| s.as_array())
                                {
                                    for sub_item in sub_list {
                                        let r = sub_item
                                            .get("Role")
                                            .or_else(|| sub_item.get("TypeName"))
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("Sous-agent");
                                        pending_roles
                                            .push(r.trim_matches('"').trim_matches('\'').to_string());
                                    }
                                } else {
                                    let role = args
                                        .and_then(|a| a.get("Role").or_else(|| a.get("TypeName")))
                                        .and_then(|r| r.as_str())
                                        .unwrap_or("Sous-agent")
                                        .trim_matches('"')
                                        .trim_matches('\'')
                                        .to_string();
                                    pending_roles.push(role);
                                }
                            }
                        }
                    }
                } else if source == "MODEL" && !pending_roles.is_empty()
                    && let Some(content) = step.get("content").and_then(|c| c.as_str()) {
                        let mut start = 0;
                        while let Some(idx) = content[start..].find("\"conversationId\"") {
                            let after = &content[start + idx + 16..];
                            if let Some(q1) = after.find('"') {
                                let after_q1 = &after[q1 + 1..];
                                if let Some(q2) = after_q1.find('"') {
                                    let cid = after_q1[..q2].to_string();
                                    let role = if !pending_roles.is_empty() {
                                        pending_roles.remove(0)
                                    } else {
                                        "Sous-agent".to_string()
                                    };
                                    if !subagent_pairs.iter().any(|(existing_id, _)| existing_id == &cid) {
                                        subagent_pairs.push((cid, role));
                                    }
                                    start += idx + 16 + q1 + 1 + q2 + 1;
                                    continue;
                                }
                            }
                            break;
                        }
                    }
            }
    }
    (subagent_pairs, canceled_cids)
}

impl SessionWatcher {
    /// Détermine si une conversation appartient à un sous-agent plutôt qu'à une session racine.
    pub(crate) fn is_subagent_conv(
        &self,
        conv_id: &str,
        data_dir: &Path,
        known_subagents: &HashSet<String>,
    ) -> bool {
        if let Some(entry) = self.desktop_meta.get(conv_id) {
            return !entry.parent_id.is_empty();
        }
        if known_subagents.contains(conv_id) {
            return true;
        }
        // Les conversations de premier niveau disposent d'un fichier d'annotation .pbtxt ou d'un peer bridge.
        if data_dir.join("annotations").join(format!("{conv_id}.pbtxt")).exists() {
            return false;
        }
        if self.home_dir.join(".agent-bridge/peers").join(format!("agy-{conv_id}.json")).exists() {
            return false;
        }
        for cand in &self.agy_candidate_dirs {
            if cand.join("annotations").join(format!("{conv_id}.pbtxt")).exists() {
                return false;
            }
        }
        // Vérification de la première ligne : les sous-agents démarrent avec un message système délégué.
        for cand in &self.agy_candidate_dirs {
            let brain_dir = cand.join("brain");
            let logs = brain_dir.join(conv_id).join(".system_generated/logs");
            let tpath = if logs.join("transcript.jsonl").exists() {
                logs.join("transcript.jsonl")
            } else {
                logs.join("transcript_full.jsonl")
            };
            if let Ok(file) = File::open(&tpath) {
                let mut reader = BufReader::new(file);
                let mut first_line = String::new();
                if reader.read_line(&mut first_line).is_ok() {
                    if (first_line.contains("[Message]") && first_line.contains("sender="))
                        || (first_line.contains("\"source\":\"SYSTEM\"") && !first_line.contains("<USER_REQUEST>"))
                    {
                        return true;
                    }
                    if first_line.contains("<USER_REQUEST>") || first_line.contains("\"type\":\"USER_INPUT\"") {
                        return false;
                    }
                }
            }
        }
        false
    }

    /// Synchronise les métadonnées de sessions Desktop Antigravity depuis la base SQLite.
    pub(crate) fn sync_desktop_meta(&mut self) {
        let ide_dir = self.home_dir.join(".gemini/antigravity");
        let db_path = ide_dir.join("conversation_summaries.db");
        let wal_path = ide_dir.join("conversation_summaries.db-wal");
        if !db_path.exists() {
            return;
        }

        let mtime = std::fs::metadata(&db_path).and_then(|m| m.modified()).ok();
        let wal_mtime = if wal_path.exists() {
            std::fs::metadata(&wal_path).and_then(|m| m.modified()).ok()
        } else {
            None
        };
        let latest = match (mtime, wal_mtime) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, None) => a,
            (None, b) => b,
        };
        if let Some(t) = latest {
            if Some(t) == self.last_desktop_db_mtime {
                return;
            }
            self.last_desktop_db_mtime = Some(t);
        }

        let script = PathBuf::from("/home/user/Documents/Paros/linux/paros-antigravity-db.py");
        if !script.exists() {
            return;
        }

        if let Ok(output) = std::process::Command::new("python3")
            .arg("-I")
            .arg(&script)
            .arg("--db")
            .arg(&db_path)
            .output()
            && output.status.success()
                && let Ok(parsed) = serde_json::from_slice::<HashMap<String, DesktopMetaEntry>>(&output.stdout) {
                    self.desktop_meta = parsed;
                }
    }

    /// Extrait différentiellement l'historique des sous-agents en évitant de relire le fichier depuis l'octet 0.
    pub(crate) fn parse_agy_subagent_history_cached(
        &mut self,
        conv_id: &str,
        path: &Path,
    ) -> (Vec<(String, String)>, HashSet<String>) {
        let Ok(file) = File::open(path) else {
            return (Vec::new(), HashSet::new());
        };
        let Ok(meta) = file.metadata() else {
            return (Vec::new(), HashSet::new());
        };
        let file_len = meta.len();

        if let Some((cached_len, pairs, canceled)) = self.agy_subagent_history_cache.get(conv_id) {
            if file_len == *cached_len {
                return (pairs.clone(), canceled.clone());
            }
            if file_len > *cached_len {
                let mut reader = BufReader::new(&file);
                if reader.seek(SeekFrom::Start(*cached_len)).is_ok() {
                    let mut new_pairs = pairs.clone();
                    let mut new_canceled = canceled.clone();
                    let mut pending_roles = Vec::new();

                    for line in reader.lines().map_while(Result::ok) {
                        if line.contains("has been explicitly canceled")
                            && let Some(p1) = line.find("Subagent \"") {
                                let rem = &line[p1 + 10..];
                                if let Some(p2) = rem.find('"') {
                                    new_canceled.insert(rem[..p2].to_string());
                                }
                            }
                        if (line.contains("invoke_subagent") || !pending_roles.is_empty())
                            && let Ok(step) = serde_json::from_str::<Value>(&line) {
                                let step_type = step.get("type").and_then(|t| t.as_str()).unwrap_or("");
                                let source = step.get("source").and_then(|s| s.as_str()).unwrap_or("");
                                if step_type == "PLANNER_RESPONSE" {
                                    if let Some(calls) = step.get("tool_calls").and_then(|c| c.as_array()) {
                                        for call in calls {
                                            if call.get("name").and_then(|n| n.as_str()) == Some("invoke_subagent") {
                                                let args = call.get("args");
                                                if let Some(sub_list) = args
                                                    .and_then(|a| a.get("Subagents"))
                                                    .and_then(|s| s.as_array())
                                                {
                                                    for sub_item in sub_list {
                                                        let r = sub_item
                                                            .get("Role")
                                                            .or_else(|| sub_item.get("TypeName"))
                                                            .and_then(|v| v.as_str())
                                                            .unwrap_or("Sous-agent");
                                                        pending_roles
                                                            .push(r.trim_matches('"').trim_matches('\'').to_string());
                                                    }
                                                } else {
                                                    let role = args
                                                        .and_then(|a| a.get("Role").or_else(|| a.get("TypeName")))
                                                        .and_then(|r| r.as_str())
                                                        .unwrap_or("Sous-agent")
                                                        .trim_matches('"')
                                                        .trim_matches('\'')
                                                        .to_string();
                                                    pending_roles.push(role);
                                                }
                                            }
                                        }
                                    }
                                } else if source == "MODEL" && !pending_roles.is_empty()
                                    && let Some(content) = step.get("content").and_then(|c| c.as_str()) {
                                        let mut start = 0;
                                        while let Some(idx) = content[start..].find("\"conversationId\"") {
                                            let after = &content[start + idx + 16..];
                                            if let Some(q1) = after.find('"') {
                                                let after_q1 = &after[q1 + 1..];
                                                if let Some(q2) = after_q1.find('"') {
                                                    let cid = after_q1[..q2].to_string();
                                                    let role = if !pending_roles.is_empty() {
                                                        pending_roles.remove(0)
                                                    } else {
                                                        "Sous-agent".to_string()
                                                    };
                                                    if !new_pairs.iter().any(|(existing_id, _)| existing_id == &cid) {
                                                        new_pairs.push((cid, role));
                                                    }
                                                    start += idx + 16 + q1 + 1 + q2 + 1;
                                                    continue;
                                                }
                                            }
                                            break;
                                        }
                                    }
                            }
                    }
                    self.agy_subagent_history_cache.insert(
                        conv_id.to_string(),
                        (file_len, new_pairs.clone(), new_canceled.clone()),
                    );
                    return (new_pairs, new_canceled);
                }
            }
        }

        let (pairs, canceled) = parse_agy_subagent_history(path);
        self.agy_subagent_history_cache.insert(
            conv_id.to_string(),
            (file_len, pairs.clone(), canceled.clone()),
        );
        (pairs, canceled)
    }

    /// Parse les sous-agents d'un transcript et suit leurs états d'exécution et outils.
    pub(crate) fn parse_subagents_for_transcript(
        &mut self,
        brain_dir: &Path,
        transcript_path: &Path,
        conv_id: &str,
        tool_failed: &mut Option<(Option<String>, bool)>,
    ) -> Vec<SubagentInfo> {
        let (subagent_pairs, canceled_cids) =
            self.parse_agy_subagent_history_cached(conv_id, transcript_path);
        self.agy_known_subagents
            .insert(conv_id.to_string(), subagent_pairs.clone());
        let mut subagents = Vec::new();

        for (cid, role) in subagent_pairs {
            if canceled_cids.contains(&cid) {
                continue;
            }
            let sub_compact = brain_dir.join(&cid).join(".system_generated/logs/transcript.jsonl");
            let sub_path = if sub_compact.exists() {
                sub_compact
            } else {
                brain_dir.join(&cid).join(".system_generated/logs/transcript_full.jsonl")
            };

            let Ok(sub_file) = File::open(&sub_path) else {
                continue;
            };
            let Ok(meta) = sub_file.metadata() else {
                continue;
            };
            let len = meta.len();
            let sub_prev_len = self.agy_subagent_lens.insert(cid.clone(), len);
            if meta
                .modified()
                .map(|m| m.elapsed().map(|d| d.as_secs() > 60).unwrap_or(false))
                .unwrap_or(false)
                || len == 0
            {
                continue;
            }
            if let Some(sub_lines) = read_tail_lines(&sub_file, len, 131072) {
                let mut is_running = false;
                let mut tool_name = String::new();
                let mut tool_detail = String::new();
                let mut sub_tasks: HashSet<String> = HashSet::new();

                for (soff, sline) in sub_lines {
                    if let Ok(sstep) = serde_json::from_str::<Value>(&sline) {
                        let stype = sstep.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        let ssource = sstep.get("source").and_then(|s| s.as_str()).unwrap_or("");
                        let scontent = sstep.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        let sstatus = sstep.get("status").and_then(|v| v.as_str()).unwrap_or("");

                        if stype == "PLANNER_RESPONSE" {
                            if let Some(scalls) = sstep.get("tool_calls").and_then(|c| c.as_array()) {
                                if !scalls.is_empty() {
                                    is_running = true;
                                    let first = &scalls[0];
                                    tool_name = first
                                        .get("name")
                                        .and_then(|n| n.as_str())
                                        .unwrap_or("Outil")
                                        .to_string();
                                    tool_detail = Self::extract_agy_tool_detail(first.get("args"));
                                } else {
                                    is_running = !sub_tasks.is_empty();
                                }
                            } else {
                                is_running = !sub_tasks.is_empty();
                            }
                        } else if stype == "USER_INPUT" {
                            is_running = true;
                            tool_name = "Réflexion...".to_string();
                            tool_detail.clear();
                        } else if stype == "SYSTEM_MESSAGE" {
                            if let Some(sender) = extract_message_sender(scontent) {
                                sub_tasks.remove(&sender);
                            }
                            is_running = true;
                        } else if ssource == "MODEL" {
                            if sstatus == "RUNNING" || scontent.contains("background task") {
                                if let Some(tid) = extract_task_started_id(scontent) {
                                    sub_tasks.insert(tid);
                                } else {
                                    sub_tasks.insert(cid.clone());
                                }
                            }
                            if let Some(spl) = sub_prev_len
                                && soff >= spl {
                                    let code_failed = extract_exit_code(scontent).is_some_and(|c| c != 0);
                                    if sstatus == "ERROR" || code_failed {
                                        *tool_failed = Some((Some(cid.clone()), false));
                                    }
                                }
                        }
                    }
                }

                if is_running {
                    subagents.push(SubagentInfo {
                        id: cid,
                        name: role,
                        tool: tool_name,
                        detail: tool_detail,
                        running: true,
                    });
                }
            }
        }

        subagents
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_agy_subagent_history_extraction() {
        let temp_dir = std::env::temp_dir().join(format!("paros_test_agy_hist_{}", fastrand::u64(..)));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let transcript = temp_dir.join("transcript.jsonl");

        {
            let mut f = File::create(&transcript).unwrap();
            writeln!(f, "{{\"type\":\"PLANNER_RESPONSE\",\"source\":\"MODEL\",\"tool_calls\":[{{\"name\":\"invoke_subagent\",\"args\":{{\"Role\":\"Code Reviewer\"}}}}]}}").unwrap();
            writeln!(f, "{{\"type\":\"GENERIC\",\"source\":\"MODEL\",\"content\":\"Started \\\"conversationId\\\": \\\"sub_conv_456\\\"\"}}").unwrap();
            writeln!(f, "Subagent \"sub_canceled_789\" has been explicitly canceled").unwrap();
        }

        let (pairs, canceled) = parse_agy_subagent_history(&transcript);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "sub_conv_456");
        assert_eq!(pairs[0].1, "Code Reviewer");
        assert!(canceled.contains("sub_canceled_789"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
