use super::SessionWatcher;

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
}
