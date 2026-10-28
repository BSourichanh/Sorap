use std::path::Path;
use std::time::{Duration, Instant};
use super::types::GitCacheEntry;
use super::SessionWatcher;

impl SessionWatcher {
    pub(crate) fn check_git(&mut self, cwd: &str, git_enabled: bool) -> (String, bool, usize, usize, bool) {
        if !git_enabled {
            return (String::new(), false, 0, 0, false);
        }

        if let Some(entry) = self.git_cache.get(cwd)
            && entry.last_scan.elapsed() < Duration::from_secs(5) {
                return (
                    entry.branch.clone(),
                    entry.conflict,
                    entry.dirty,
                    entry.behind,
                    false,
                );
            }

        let now = Instant::now();
        let cwd_path = Path::new(cwd);
        let Some(repo_root) = cwd_path.ancestors().find(|p| p.join(".git").exists()) else {
            self.git_cache.insert(
                cwd.to_string(),
                GitCacheEntry {
                    branch: String::new(),
                    head_oid: String::new(),
                    conflict: false,
                    dirty: 0,
                    behind: 0,
                    was_dirty: false,
                    last_scan: now,
                },
            );
            return (String::new(), false, 0, 0, false);
        };

        let git_dir = repo_root.join(".git");
        let real_git_dir = if git_dir.is_file() {
            if let Ok(content) = std::fs::read_to_string(&git_dir) {
                if let Some(rest) = content.trim().strip_prefix("gitdir:") {
                    let rel_or_abs = rest.trim();
                    let path = Path::new(rel_or_abs);
                    if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        repo_root.join(path)
                    }
                } else {
                    git_dir.clone()
                }
            } else {
                git_dir.clone()
            }
        } else {
            git_dir.clone()
        };

        let repo_root_str = repo_root.to_string_lossy().to_string();

        let mut branch = String::new();
        let mut head_oid = String::new();
        let mut conflict = real_git_dir.join("MERGE_HEAD").exists()
            || real_git_dir.join("rebase-merge").exists()
            || real_git_dir.join("rebase-apply").exists()
            || real_git_dir.join("CHERRY_PICK_HEAD").exists();
        let mut dirty = 0usize;
        let mut behind = 0usize;
        let mut has_porcelain_changes = false;

        if let Ok(output) = std::process::Command::new("git")
            .args([
                "--no-optional-locks",
                "-C",
                &repo_root_str,
                "status",
                "--porcelain=v2",
                "--branch",
            ])
            .output()
        {
            let s = String::from_utf8_lossy(&output.stdout);
            for line in s.lines() {
                if let Some(rest) = line.strip_prefix("# branch.oid ") {
                    let oid = rest.trim();
                    if oid != "(initial)" {
                        head_oid = oid.to_string();
                    }
                } else if let Some(rest) = line.strip_prefix("# branch.head ") {
                    branch = rest.replace("(detached)", "").trim().to_string();
                } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
                    if let Some(idx) = rest.find(" -") {
                        behind = rest[idx + 2..]
                            .split_whitespace()
                            .next()
                            .and_then(|v| v.parse::<usize>().ok())
                            .unwrap_or(0);
                    }
                } else if !line.trim().is_empty() && !line.starts_with('#') {
                    has_porcelain_changes = true;
                    if line.starts_with("u ") {
                        conflict = true;
                    }
                }
            }
        }

        if let Ok(diff_output) = std::process::Command::new("git")
            .args(["--no-optional-locks", "-C", &repo_root_str, "diff", "--shortstat", "HEAD"])
            .output()
        {
            let diff_s = String::from_utf8_lossy(&diff_output.stdout);
            dirty = Self::count_changed_lines(&diff_s);
        }

        let prev = self.git_cache.get(cwd);
        let prev_was_dirty = prev.is_some_and(|e| e.was_dirty);
        let prev_head_oid = prev.map(|e| e.head_oid.as_str()).unwrap_or("");

        let repo_cleaned = prev_was_dirty
            && dirty == 0
            && !has_porcelain_changes
            && !prev_head_oid.is_empty()
            && !head_oid.is_empty()
            && head_oid != prev_head_oid;

        let was_dirty = dirty > 0 || has_porcelain_changes;

        self.git_cache.insert(
            cwd.to_string(),
            GitCacheEntry {
                branch: branch.clone(),
                head_oid,
                conflict,
                dirty,
                behind,
                was_dirty,
                last_scan: now,
            },
        );

        (branch, conflict, dirty, behind, repo_cleaned)
    }

    pub(crate) fn count_changed_lines(output: &str) -> usize {
        let mut total = 0usize;
        for part in output.split(',') {
            let trimmed = part.trim();
            if (trimmed.contains("insertion") || trimmed.contains("deletion"))
                && let Some(num_str) = trimmed.split_whitespace().next()
                    && let Ok(n) = num_str.parse::<usize>() {
                        total += n;
                    }
        }
        total
    }
}
