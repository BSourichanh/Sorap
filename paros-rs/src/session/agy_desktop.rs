use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use serde_json::Value;

use super::proc::extract_pbtxt_title_raw;
use super::types::SessionPhase;
use super::{AgyConvDiscovery, SessionWatcher};

#[derive(Debug, Clone)]
pub struct ActiveServer {
    pub pid: i32,
    pub data_dir: PathBuf,
}

pub fn find_active_language_servers(candidate_dirs: &[PathBuf], home_dir: &Path) -> Vec<ActiveServer> {
    let mut servers = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return servers;
    };

    for entry in entries.flatten() {
        let pid_str = entry.file_name().to_string_lossy().to_string();
        let Ok(pid) = pid_str.parse::<i32>() else { continue };

        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
        let trimmed = comm.trim();
        if !matches!(trimmed, "language_server" | "antigravity" | "antigravity-ide") {
            continue;
        }

        let mut matched_dir: Option<PathBuf> = None;

        // 1. Détection via arguments en ligne de commande (/proc/<pid>/cmdline)
        if let Ok(cmdline_bytes) = std::fs::read(format!("/proc/{pid}/cmdline")) {
            let args: Vec<String> = cmdline_bytes
                .split(|&b| b == 0)
                .map(|s| String::from_utf8_lossy(s).to_string())
                .collect();
            for (i, arg) in args.iter().enumerate() {
                let val_opt = if let Some(v) = arg.strip_prefix("--app_data_dir=") {
                    Some(v)
                } else if arg == "--app_data_dir" && i + 1 < args.len() {
                    Some(args[i + 1].as_str())
                } else {
                    None
                };

                if let Some(val) = val_opt {
                    match val {
                        "antigravity" => matched_dir = Some(home_dir.join(".gemini/antigravity")),
                        "antigravity-ide" => matched_dir = Some(home_dir.join(".gemini/antigravity-ide")),
                        "antigravity-cli" => matched_dir = Some(home_dir.join(".gemini/antigravity-cli")),
                        other => {
                            let p = PathBuf::from(other);
                            if p.is_absolute() {
                                matched_dir = Some(p);
                            } else {
                                matched_dir = Some(home_dir.join(format!(".gemini/{other}")));
                            }
                        }
                    }
                    break;
                }
            }
        }

        // 2. Détection via état relay agent-bridge (~/.agent-bridge/state/host-<pid>.json)
        if matched_dir.is_none() {
            let host_json = home_dir.join(".agent-bridge/state").join(format!("host-{pid}.json"));
            if let Ok(content) = std::fs::read_to_string(&host_json)
                && let Ok(v) = serde_json::from_str::<Value>(&content)
                    && let Some(dir_str) = v.get("data_dir").and_then(|d| d.as_str()) {
                        matched_dir = Some(PathBuf::from(dir_str));
                    }
        }

        // 3. Détection via descripteurs ouverts dans /proc/<pid>/fd
        if matched_dir.is_none()
            && let Ok(fds) = std::fs::read_dir(format!("/proc/{pid}/fd")) {
                for fd in fds.flatten() {
                    if let Ok(target) = std::fs::read_link(fd.path()) {
                        for cand in candidate_dirs {
                            if target.starts_with(cand) {
                                matched_dir = Some(cand.clone());
                                break;
                            }
                        }
                        if matched_dir.is_some() {
                            break;
                        }
                    }
                }
            }

        // 4. Repli par défaut sur ~/.gemini/antigravity
        let final_dir = matched_dir.unwrap_or_else(|| home_dir.join(".gemini/antigravity"));
        servers.push(ActiveServer { pid, data_dir: final_dir });
    }

    servers
}

pub fn is_desktop_subagent(
    conv_id: &str,
    data_dir: &Path,
    home_dir: &Path,
    known_subs: &HashSet<String>,
) -> bool {
    if known_subs.contains(conv_id) {
        return true;
    }
    // Présence de annotations/<id>.pbtxt -> conversation principale
    if data_dir.join("annotations").join(format!("{conv_id}.pbtxt")).exists() {
        return false;
    }
    // Présence de peers/agy-<id>.json -> agent principal déclaré
    if home_dir.join(".agent-bridge/peers").join(format!("agy-{conv_id}.json")).exists() {
        return false;
    }

    // Inspection de la première ligne du transcript pour exclure formellement les sous-agents
    let logs_dir = data_dir.join("brain").join(conv_id).join(".system_generated/logs");
    let full = logs_dir.join("transcript_full.jsonl");
    let path = if full.exists() { full } else { logs_dir.join("transcript.jsonl") };

    if let Ok(file) = File::open(&path) {
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

    // Par défaut, sans annotations ni fichier peer, c'est un sous-agent
    true
}

pub fn scan_presence_locks(
    candidate_dirs: &[PathBuf],
    known_subs: &HashSet<String>,
    home_dir: &Path,
) -> HashMap<String, (Option<i32>, PathBuf)> {
    let mut locks = HashMap::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return locks;
    };

    for entry in entries.flatten() {
        let pid_str = entry.file_name().to_string_lossy().to_string();
        let Ok(pid) = pid_str.parse::<i32>() else { continue };

        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
        let trimmed = comm.trim();
        if !matches!(trimmed, "language_server" | "agy" | "antigravity" | "antigravity-ide") {
            continue;
        }

        let Ok(fds) = std::fs::read_dir(format!("/proc/{pid}/fd")) else { continue };
        for fd in fds.flatten() {
            if let Ok(target) = std::fs::read_link(fd.path())
                && target.extension().and_then(|e| e.to_str()) == Some("lock")
            {
                let parent_opt = target.parent();
                if let Some(parent) = parent_opt
                    && parent.file_name().and_then(|n| n.to_str()) == Some("presence")
                {
                    let matched_cand = candidate_dirs
                        .iter()
                        .find(|cand| parent == cand.join("presence").as_path())
                        .cloned()
                        .or_else(|| parent.parent().map(|p| p.to_path_buf()));

                    if let Some(data_dir) = matched_cand
                        && let Some(stem) = target.file_stem().and_then(|s| s.to_str())
                        && !is_desktop_subagent(stem, &data_dir, home_dir, known_subs)
                    {
                        locks.insert(stem.to_string(), (Some(pid), data_dir));
                    }
                }
            }
        }
    }

    locks
}

pub struct DetectedDesktopConv {
    pub conv_id: String,
    pub pid: Option<i32>,
    pub data_dir: PathBuf,
    pub cwd: String,
    pub title: String,
}

pub fn scan_desktop_recent_conversations(
    candidate_dirs: &[PathBuf],
    servers: &[ActiveServer],
    desktop_minutes: f32,
    known_subs: &HashSet<String>,
    home_dir: &Path,
) -> Vec<DetectedDesktopConv> {
    let mut results = Vec::new();
    let max_duration = Duration::from_secs_f32(desktop_minutes.max(1.0) * 60.0);

    // Déterminer les dossiers de données actifs
    let mut active_dirs: Vec<PathBuf> = servers.iter().map(|s| s.data_dir.clone()).collect();
    for cand in candidate_dirs {
        if !active_dirs.contains(cand) && cand.exists() {
            active_dirs.push(cand.clone());
        }
    }

    let default_server_pid = servers.first().map(|s| s.pid);

    for data_dir in active_dirs {
        let brain_dir = data_dir.join("brain");
        let Ok(entries) = std::fs::read_dir(&brain_dir) else { continue };

        let srv_pid = servers
            .iter()
            .find(|s| s.data_dir == data_dir)
            .map(|s| s.pid)
            .or(default_server_pid);

        for entry in entries.flatten() {
            let conv_id = entry.file_name().to_string_lossy().to_string();
            if conv_id.is_empty() || known_subs.contains(&conv_id) {
                continue;
            }

            let logs_dir = entry.path().join(".system_generated/logs");
            let full_transcript = logs_dir.join("transcript_full.jsonl");
            let compact_transcript = logs_dir.join("transcript.jsonl");
            let transcript_to_check = if full_transcript.exists() {
                full_transcript
            } else if compact_transcript.exists() {
                compact_transcript
            } else {
                continue;
            };

            let Ok(meta) = std::fs::metadata(&transcript_to_check) else { continue };
            let elapsed = meta
                .modified()
                .ok()
                .and_then(|m| m.elapsed().ok())
                .unwrap_or(Duration::from_secs(999999));

            if elapsed > max_duration {
                continue;
            }

            // Vérifier annotations/<id>.pbtxt ou peers/agy-<id>.json
            let annot_path = data_dir.join("annotations").join(format!("{conv_id}.pbtxt"));
            let peer_path = home_dir.join(".agent-bridge/peers").join(format!("agy-{conv_id}.json"));
            let has_annot = annot_path.exists();
            let has_peer = peer_path.exists();

            // Exclure formellement les sous-agents (pas d'annotations ni de fichier peer)
            if !has_annot && !has_peer {
                continue;
            }

            // Extraire cwd via workspace du peer bridge si disponible, sinon chaîne vide
            let mut peer_cwd = String::new();
            let mut peer_title: Option<String> = None;
            if has_peer
                && let Ok(content) = std::fs::read_to_string(&peer_path)
                    && let Ok(v) = serde_json::from_str::<Value>(&content)
            {
                if let Some(ws) = v.get("workspace").and_then(|w| w.as_str()) {
                    peer_cwd = ws.to_string();
                }
                if let Some(t) = v.get("title").and_then(|w| w.as_str())
                    && !t.is_empty()
                {
                    peer_title = Some(t.to_string());
                }
            }

            // Extraire le titre depuis annotations/<id>.pbtxt ou cwd.file_name() ou 'Antigravity'
            let title = if let Some(t) = peer_title {
                t
            } else if has_annot
                && let Ok(annot_content) = std::fs::read_to_string(&annot_path)
                && let Some(t) = extract_pbtxt_title_raw(&annot_content)
                && !t.is_empty() && t != "New Conversation"
            {
                t
            } else if !peer_cwd.is_empty() {
                Path::new(&peer_cwd)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "Antigravity".to_string())
            } else {
                "Antigravity".to_string()
            };

            results.push(DetectedDesktopConv {
                conv_id,
                pid: srv_pid,
                data_dir: data_dir.clone(),
                cwd: peer_cwd,
                title,
            });
        }
    }

    results
}

pub fn resolve_transcript_path(brain_dir: &Path, conv_id: &str) -> Option<PathBuf> {
    let logs_dir = brain_dir.join(conv_id).join(".system_generated/logs");
    let full = logs_dir.join("transcript_full.jsonl");
    if full.exists() {
        return Some(full);
    }
    let compact = logs_dir.join("transcript.jsonl");
    if compact.exists() {
        return Some(compact);
    }
    None
}

impl SessionWatcher {
    pub(crate) fn transcript_len_for_agy(&self, conv_id: &str) -> u64 {
        for cand in &self.agy_candidate_dirs {
            let brain_dir = cand.join("brain");
            if let Some(path) = resolve_transcript_path(&brain_dir, conv_id)
                && let Ok(m) = std::fs::metadata(&path)
            {
                return m.len();
            }
        }
        0
    }

    pub(crate) fn find_all_antigravity_convs(&mut self) -> Vec<AgyConvDiscovery> {
        if self.last_proc_scan.elapsed() < Duration::from_secs(2) && !self.cached_agy_convs.is_empty() {
            return self.cached_agy_convs.clone();
        }
        self.last_proc_scan = Instant::now();

        type ConvMapValue = (Option<i32>, PathBuf, Option<String>, Option<String>);
        let mut convs: HashMap<String, ConvMapValue> = HashMap::new();

        let mut known_subs: HashSet<String> = HashSet::new();
        for subs in self.agy_known_subagents.values() {
            for (cid, _) in subs {
                known_subs.insert(cid.clone());
            }
        }

        // 1. Détection des serveurs language_server actifs liés aux data_dirs
        let servers = find_active_language_servers(&self.agy_candidate_dirs, &self.home_dir);

        // 2. Détection des verrous presence/<id>.lock
        let locks = scan_presence_locks(&self.agy_candidate_dirs, &known_subs, &self.home_dir);
        for (stem, (pid, data_dir)) in locks {
            convs.insert(stem, (pid, data_dir, None, None));
        }

        // 3. Détection des conversations Desktop récentes selon seuil desktop_minutes
        let recent_desktop = scan_desktop_recent_conversations(
            &self.agy_candidate_dirs,
            &servers,
            self.desktop_minutes,
            &known_subs,
            &self.home_dir,
        );
        for d in recent_desktop {
            if let std::collections::hash_map::Entry::Vacant(e) = convs.entry(d.conv_id) {
                let hint_cwd = (!d.cwd.is_empty()).then_some(d.cwd);
                let hint_title = (!d.title.is_empty()).then_some(d.title);
                e.insert((d.pid, d.data_dir, hint_cwd, hint_title));
            }
        }

        // 4. Conversations depuis desktop_meta (SQLite) si serveur actif et non idle
        if let Some(srv) = servers.first() {
            let ide_dir = srv.data_dir.clone();
            for (cid, entry) in &self.desktop_meta {
                if entry.not_idle && entry.parent_id.is_empty() && !convs.contains_key(cid) {
                    let hint_cwd = (!entry.cwd.is_empty()).then_some(entry.cwd.clone());
                    let hint_title = (!entry.title.is_empty()).then_some(entry.title.clone());
                    convs.insert(cid.clone(), (Some(srv.pid), ide_dir.clone(), hint_cwd, hint_title));
                }
            }
        }

        // 5. Conserver les sessions working / avec sous-agents vivants
        for (id, (st, dir)) in &self.agy_sessions {
            if convs.contains_key(id) { continue; }
            let is_proc_alive = st.terminal_pid.is_some_and(|pid| Path::new(&format!("/proc/{pid}")).exists());
            if (st.phase == SessionPhase::Working || !st.subagents.is_empty()) && is_proc_alive {
                convs.insert(id.clone(), (st.terminal_pid, dir.clone(), Some(st.cwd.clone()), Some(st.name.clone())));
            }
        }

        let res: Vec<_> = convs.into_iter().map(|(id, (pid, dir, cwd, title))| (id, pid, dir, cwd, title)).collect();
        self.cached_agy_convs = res.clone();
        res
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
