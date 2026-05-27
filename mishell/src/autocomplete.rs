use crate::shell::Shell;
use std::collections::HashMap;

pub struct AutoCompleter {
    commands: Vec<String>,
    subcommands: HashMap<String, Vec<String>>,
}

impl Default for AutoCompleter {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoCompleter {
    pub fn new() -> Self {
        Self {
            commands: Self::load_commands(),
            subcommands: Self::load_subcommands(),
        }
    }

    fn load_subcommands() -> HashMap<String, Vec<String>> {
        let mut map = HashMap::new();
        
        map.insert("git".into(), vec![
            "add", "am", "archive", "bisect", "branch", "bundle", "checkout",
            "cherry-pick", "clean", "clone", "commit", "config", "describe",
            "diff", "fetch", "format-patch", "gc", "gitk", "grep", "gui",
            "help", "init", "instaweb", "log", "merge", "mv", "notes",
            "pull", "push", "rebase", "reflog", "remote", "replace",
            "reset", "restore", "revert", "rm", "shortlog", "show",
            "stash", "status", "submodule", "switch", "tag", "worktree",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("cargo".into(), vec![
            "build", "b", "check", "c", "clean", "doc", "fetch",
            "fix", "generate-lockfile", "init", "install", "locate-project",
            "login", "metadata", "new", "owner", "package", "pkgid",
            "publish", "read-manifest", "remove", "report", "run", "r",
            "rustc", "rustdoc", "search", "test", "t", "tree", "uninstall",
            "update", "vendor", "verify-project", "version", "yank",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("docker".into(), vec![
            "attach", "build", "commit", "cp", "create", "diff",
            "events", "exec", "export", "history", "images", "import",
            "info", "inspect", "kill", "load", "login", "logout",
            "logs", "pause", "port", "ps", "pull", "push", "rename",
            "restart", "rm", "rmi", "run", "save", "search",
            "start", "stats", "stop", "tag", "top", "unpause",
            "update", "version", "wait",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("npm".into(), vec![
            "access", "adduser", "audit", "bin", "bugs", "cache",
            "ci", "completion", "config", "dedupe", "deprecate",
            "diff", "dist-tag", "docs", "doctor", "edit", "exec",
            "explain", "explore", "fund", "help", "hook", "init",
            "install", "link", "ll", "login", "ls", "org", "outdated",
            "owner", "pack", "ping", "pkg", "prefix", "profile",
            "prune", "publish", "query", "rebuild", "repo", "restart",
            "root", "run-script", "search", "set", "shrinkwrap",
            "star", "stars", "start", "stop", "team", "test",
            "token", "uninstall", "unpublish", "unstar", "update",
            "version", "view", "whoami",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("pip".into(), vec![
            "install", "download", "uninstall", "freeze", "list",
            "show", "check", "config", "search", "cache", "index",
            "wheel", "hash", "completion", "debug", "help",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("systemctl".into(), vec![
            "list-units", "list-sockets", "list-timers", "start",
            "stop", "reload", "restart", "try-restart", "reload-or-restart",
            "try-reload-or-restart", "isolate", "kill", "clean",
            "freeze", "thaw", "is-active", "is-failed", "status",
            "show", "cat", "set-property", "help", "reset-failed",
            "list-dependencies", "list-unit-files", "enable", "disable",
            "reenable", "preset", "preset-all", "is-enabled", "mask",
            "unmask", "link", "revert", "add-wants", "add-requires",
            "edit", "get-default", "set-default", "list-machines",
            "list-jobs", "cancel", "daemon-reload", "daemon-reexec",
            "show-environment", "set-environment", "unset-environment",
            "import-environment", "is-system-running", "default", "rescue",
            "emergency", "halt", "poweroff", "reboot", "kexec",
            "exit", "switch-root", "suspend", "hibernate", "hybrid-sleep",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("make".into(), vec![
            "--always-make", "--directory", "--dry-run", "--environment-overrides",
            "--file", "--ignore-errors", "--include-dir", "--jobs",
            "--just-print", "--keep-going", "--load-average", "--max-load",
            "--no-builtin-rules", "--no-print-directory", "--old-file",
            "--output-sync", "--print-data-base", "--question", "--quiet",
            "--recon", "--silent", "--touch", "--version", "--warn-undefined-variables",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("brew".into(), vec![
            "install", "uninstall", "reinstall", "list", "search",
            "info", "home", "update", "upgrade", "cleanup", "doctor",
            "deps", "edit", "fetch", "pin", "unpin", "tap", "untap",
            "services", "bundle", "cask", "formulae", "outdated",
            "link", "unlink", "missing", "desc", "cat", "options",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("apt".into(), vec![
            "install", "remove", "purge", "update", "upgrade",
            "full-upgrade", "autoremove", "autoclean", "clean",
            "search", "show", "list", "edit-sources", "satisfy",
            "depends", "rdepends", "policy", "madison", "download",
            "changelog", "source", "build-dep", "dist-upgrade",
            "dselect-upgrade", "indextargets", "add-repository",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("ssh".into(), vec![
            "-p", "-i", "-L", "-R", "-D", "-N", "-f", "-C",
            "-o", "-J", "-A", "-X", "-Y", "-v", "-q",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("rsync".into(), vec![
            "-a", "-v", "-z", "-P", "-r", "-t", "-o", "-g",
            "--delete", "--exclude", "--include", "--progress",
            "--dry-run", "--verbose", "-e", "--compress",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("tar".into(), vec![
            "-c", "-x", "-t", "-f", "-v", "-z", "-j", "-J",
            "--create", "--extract", "--list", "--file", "--verbose",
            "--gzip", "--bzip2", "--xz", "--delete", "--append",
            "--update", "--diff", "--compare",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("grep".into(), vec![
            "-i", "-v", "-n", "-l", "-c", "-r", "-R", "-w",
            "-x", "-e", "-f", "--ignore-case", "--invert-match",
            "--line-number", "--files-with-matches", "--count",
            "--recursive", "--word-regexp", "--line-regexp",
            "--only-matching", "--quiet", "--silent", "-o", "-q",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("find".into(), vec![
            "-name", "-type", "-mtime", "-mmin", "-size", "-exec",
            "-delete", "-print", "-ls", "-empty", "-newer", "-perm",
            "-user", "-group", "-maxdepth", "-mindepth", "-iname",
            "-regex", "-iregex", "-not", "-and", "-or",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("curl".into(), vec![
            "-X", "-H", "-d", "--data", "-o", "-O", "-s", "-S",
            "-L", "--location", "-k", "--insecure", "-u", "--user",
            "-b", "--cookie", "-c", "--cookie-jar", "-A", "--user-agent",
            "-e", "--referer", "--compressed", "-I", "--head",
            "-v", "--verbose", "-f", "--fail", "-w", "--write-out",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("wget".into(), vec![
            "-O", "-P", "-c", "-r", "-np", "-nd", "-l", "-A",
            "-R", "--reject", "--accept", "-q", "--quiet", "-v",
            "--verbose", "-b", "--background", "-o", "--output-file",
            "--limit-rate", "--tries", "--wait", "--random-wait",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("kubectl".into(), vec![
            "get", "describe", "create", "apply", "delete", "edit",
            "logs", "exec", "port-forward", "proxy", "run", "expose",
            "scale", "autoscale", "rollout", "set", "label", "annotate",
            "taint", "patch", "replace", "convert", "cluster-info",
            "top", "cordon", "uncordon", "drain", "taint", "api-resources",
            "api-versions", "config", "plugin", "wait", "attach",
            "auth", "certificates", "completion", "debug", "diff",
            "events", "explain", "kustomize", "options", "version",
        ].iter().map(|s| s.to_string()).collect());
        
        map.insert("gh".into(), vec![
            "api", "auth", "browse", "codespace", "config", "extension",
            "gist", "gpg-key", "issue", "label", "org", "pr", "release",
            "repo", "ruleset", "run", "secret", "ssh-key", "status",
            "variable", "web", "workflow",
        ].iter().map(|s| s.to_string()).collect());
        
        map
    }

    fn load_commands() -> Vec<String> {
        let mut commands = Vec::new();

        // Load from PATH
        if let Ok(path) = std::env::var("PATH") {
            for dir in path.split(':') {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            commands.push(name.to_string());
                        }
                    }
                }
            }
        }

        // Add builtins
        commands.extend([
            "cd", "export", "alias", "abbr", "set", "exit",
            "pushd", "popd", "dirs", "history", "echo", "printf",
            "read", "test", "[", "true", "false", "pwd", "type",
            "hash", "help", "source", ".", "eval", "exec",
        ].iter().map(|s| s.to_string()));

        commands.sort();
        commands.dedup();
        commands
    }

    pub fn complete(&self, input: &str, cursor_pos: usize, shell: &Shell) -> Vec<String> {
        let before_cursor = &input[..cursor_pos];
        let parts: Vec<&str> = before_cursor.split_whitespace().collect();

        if parts.is_empty() || (parts.len() == 1 && !before_cursor.ends_with(' ')) {
            let prefix = parts.first().unwrap_or(&"");
            return self.complete_command(prefix);
        }

        // Check for context-aware subcommand completion
        let cmd_name = parts[0];
        if parts.len() == 2 && !before_cursor.ends_with(' ') {
            // Completing subcommand (e.g., "git ch" -> "checkout", "cherry-pick")
            let prefix = parts[1];
            if let Some(subs) = self.subcommands.get(cmd_name) {
                let completions: Vec<String> = subs.iter()
                    .filter(|s| s.starts_with(prefix))
                    .cloned()
                    .collect();
                if !completions.is_empty() {
                    return completions;
                }
            }
        } else if parts.len() >= 2 && before_cursor.ends_with(' ') {
            // After subcommand, complete based on context
            let subcmd = parts.get(1).unwrap_or(&"");
            return self.complete_context_args(cmd_name, subcmd, "", shell);
        } else if parts.len() >= 3 {
            let subcmd = parts[1];
            let last_word = parts.last().unwrap_or(&"");
            return self.complete_context_args(cmd_name, subcmd, last_word, shell);
        }

        // Default: file path completion
        let last_word = parts.last().unwrap_or(&"");
        self.complete_argument(last_word, shell)
    }

    fn complete_context_args(&self, cmd: &str, subcmd: &str, prefix: &str, shell: &Shell) -> Vec<String> {
        let mut completions = Vec::new();
        
        match cmd {
            "git" => {
                match subcmd {
                    "checkout" | "switch" | "co" => {
                        // Complete branches
                        if let Ok(output) = std::process::Command::new("git")
                            .args(["branch", "--format=%(refname:short)"])
                            .output()
                        {
                            let branches = String::from_utf8_lossy(&output.stdout);
                            for branch in branches.lines() {
                                if branch.starts_with(prefix) {
                                    completions.push(branch.to_string());
                                }
                            }
                        }
                        // Also suggest remote branches with --track
                        if prefix.starts_with("--") {
                            completions.extend(["--track", "--orphan", "-b", "--detach"]
                                .iter().filter(|s| s.starts_with(prefix)).map(|s| s.to_string()));
                        }
                    }
                    "add" | "diff" | "log" | "show" | "status" | "restore" | "rm" => {
                        // Complete files
                        completions.extend(self.complete_file_paths(prefix));
                        // Common flags
                        if prefix.starts_with('-') {
                            let flags = match subcmd {
                                "add" => vec!["-p", "--patch", "-n", "--dry-run", "-u", "--update", "-A", "--all", "-i", "--interactive"],
                                "diff" => vec!["--cached", "--staged", "--stat", "--name-only", "--name-status", "-w", "--word-diff"],
                                "log" => vec!["--oneline", "--graph", "--all", "--stat", "-n", "--since", "--until", "--author", "--grep"],
                                "status" => vec!["-s", "--short", "-b", "--branch", "--porcelain", "-u"],
                                "restore" => vec!["--staged", "--source", "--worktree"],
                                "rm" => vec!["-f", "--force", "-r", "--cached", "-n", "--dry-run"],
                                _ => vec![],
                            };
                            completions.extend(flags.iter().filter(|s| s.starts_with(prefix)).map(|s| s.to_string()));
                        }
                    }
                    "commit" => {
                        if prefix.starts_with('-') {
                            completions.extend(["-m", "--message", "-a", "--all", "--amend", "-p", "--patch",
                                "--no-edit", "--allow-empty", "-S", "--gpg-sign", "--signoff", "-s"]
                                .iter().filter(|s| s.starts_with(prefix)).map(|s| s.to_string()));
                        }
                    }
                    "push" | "pull" | "fetch" => {
                        if prefix.starts_with('-') {
                            let flags = match subcmd {
                                "push" => vec!["-u", "--set-upstream", "--force", "-f", "--tags", "--all", "--dry-run", "-n", "--delete"],
                                "pull" => vec!["--rebase", "--no-rebase", "--ff-only", "--no-ff", "-v", "--verbose"],
                                "fetch" => vec!["--all", "--tags", "--prune", "--dry-run", "-v", "--verbose", "--depth"],
                                _ => vec![],
                            };
                            completions.extend(flags.iter().filter(|s| s.starts_with(prefix)).map(|s| s.to_string()));
                        } else {
                            // Complete remotes
                            if let Ok(output) = std::process::Command::new("git")
                                .args(["remote"])
                                .output()
                            {
                                let remotes = String::from_utf8_lossy(&output.stdout);
                                for remote in remotes.lines() {
                                    if remote.starts_with(prefix) {
                                        completions.push(remote.to_string());
                                    }
                                }
                            }
                        }
                    }
                    "stash" => {
                        completions.extend(["push", "pop", "list", "show", "drop", "clear", "apply", "branch", "create"]
                            .iter().filter(|s| s.starts_with(prefix)).map(|s| s.to_string()));
                    }
                    "remote" => {
                        completions.extend(["add", "remove", "rename", "set-url", "show", "prune", "update"]
                            .iter().filter(|s| s.starts_with(prefix)).map(|s| s.to_string()));
                    }
                    "branch" if prefix.starts_with('-') => {
                        completions.extend(["-d", "--delete", "-D", "-a", "--all", "-r", "--remotes",
                            "-v", "--verbose", "--merged", "--no-merged", "--contains", "--sort"]
                            .iter().filter(|s| s.starts_with(prefix)).map(|s| s.to_string()));
                    }
                    "branch" => {
                        completions.extend(self.complete_file_paths(prefix));
                    }
                    _ => {
                        completions.extend(self.complete_file_paths(prefix));
                    }
                }
            }
            "cargo" => {
                match subcmd {
                    "build" | "b" | "check" | "c" | "test" | "t" | "run" | "r" | "doc" => {
                        if prefix.starts_with('-') {
                            let flags = match subcmd {
                                "build" | "b" | "check" | "c" => vec!["--release", "-r", "--lib", "--bin", "--bins",
                                    "--example", "--examples", "--test", "--tests", "--bench", "--benches",
                                    "--all-targets", "-p", "--package", "--features", "--all-features",
                                    "--no-default-features", "--target", "-j", "--jobs", "--message-format",
                                    "--manifest-path", "--locked", "--frozen", "--offline", "-v", "--verbose"],
                                "test" | "t" => vec!["--release", "-r", "--lib", "--bin", "--test", "--bench",
                                    "-p", "--package", "--features", "--all-features", "--no-default-features",
                                    "--target", "--no-run", "--nocapture", "--exact", "-j", "--jobs",
                                    "--manifest-path", "--locked", "--frozen", "--offline", "-v", "--verbose",
                                    "--doc", "--show-output", "--ignored", "--include-ignored"],
                                "run" | "r" => vec!["--release", "-r", "--bin", "--example", "-p", "--package",
                                    "--features", "--target", "-j", "--jobs", "--manifest-path",
                                    "--locked", "--frozen", "--offline", "-v", "--verbose"],
                                "doc" => vec!["--open", "--document-private-items", "--no-deps",
                                    "-p", "--package", "--features", "--target", "--manifest-path",
                                    "--locked", "--frozen", "--offline", "-v", "--verbose"],
                                _ => vec![],
                            };
                            completions.extend(flags.iter().filter(|s| s.starts_with(prefix)).map(|s| s.to_string()));
                        } else {
                            // Complete binary/example names from Cargo.toml
                            completions.extend(self.complete_cargo_targets(prefix));
                        }
                    }
                    "install" if prefix.starts_with('-') => {
                        completions.extend(["--force", "-f", "--git", "--branch", "--tag", "--rev",
                            "--path", "--root", "--registry", "--version", "--features",
                            "--no-default-features", "--profile", "--debug", "--locked"]
                            .iter().filter(|s| s.starts_with(prefix)).map(|s| s.to_string()));
                    }
                    _ => {}
                }
            }
            "docker" => {
                match subcmd {
                    "run" | "create" | "exec" if prefix.starts_with('-') => {
                        let flags = match subcmd {
                            "run" | "create" => vec!["-d", "--detach", "-it", "-p", "--publish", "-v", "--volume",
                                "-e", "--env", "--name", "--network", "--rm", "--restart",
                                "-w", "--workdir", "-u", "--user", "--memory", "--cpus",
                                "--gpus", "--platform", "--privileged", "--cap-add"],
                            "exec" => vec!["-it", "-d", "-e", "--env", "-u", "--user", "-w", "--workdir"],
                            _ => vec![],
                        };
                        completions.extend(flags.iter().filter(|s| s.starts_with(prefix)).map(|s| s.to_string()));
                    }
                    "rm" | "stop" | "start" | "restart" | "logs" | "inspect" | "stats" => {
                        // Complete container names/IDs
                        completions.extend(self.complete_docker_containers(prefix));
                    }
                    "images" | "rmi" | "pull" | "push" | "tag" => {
                        // Complete image names
                        completions.extend(self.complete_docker_images(prefix));
                    }
                    _ => {}
                }
            }
            "make" => {
                // Complete targets from Makefile
                completions.extend(self.complete_make_targets(prefix));
            }
            _ => {
                completions.extend(self.complete_file_paths(prefix));
            }
        }
        
        // Variable completion
        if let Some(var_prefix) = prefix.strip_prefix('$') {
            completions.clear();
            for key in shell.vars().keys() {
                if key.starts_with(var_prefix) {
                    completions.push(format!("${}", key));
                }
            }
        }
        
        completions.sort();
        completions.dedup();
        completions.truncate(20);
        completions
    }

    fn complete_file_paths(&self, prefix: &str) -> Vec<String> {
        let mut completions = Vec::new();
        let (dir, file_prefix) = if let Some(pos) = prefix.rfind('/') {
            let dir = &prefix[..pos];
            let file = &prefix[pos + 1..];
            (if dir.is_empty() { "/" } else { dir }, file)
        } else {
            (".", prefix)
        };

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(file_prefix) && !name.starts_with('.') {
                        let path = format!("{}/{}", dir, name);
                        let path = path.replace("//", "/");
                        if entry.path().is_dir() {
                            completions.push(format!("{}/", path));
                        } else {
                            completions.push(path);
                        }
                    }
                }
            }
        }
        completions
    }

    fn complete_cargo_targets(&self, prefix: &str) -> Vec<String> {
        let mut completions = Vec::new();
        if let Ok(content) = std::fs::read_to_string("Cargo.toml") {
            // Parse [[bin]] sections and [lib]
            let mut in_bin = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed == "[[bin]]" || trimmed.starts_with("[[bin]]") {
                    in_bin = true;
                } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    in_bin = false;
                }
                if in_bin {
                    if let Some(name) = trimmed.strip_prefix("name") {
                        if let Some(eq_pos) = name.find('=') {
                            let name = name[eq_pos + 1..].trim().trim_matches('"');
                            if name.starts_with(prefix) {
                                completions.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        completions
    }

    fn complete_docker_containers(&self, prefix: &str) -> Vec<String> {
        let mut completions = Vec::new();
        if let Ok(output) = std::process::Command::new("docker")
            .args(["ps", "-a", "--format", "{{.Names}}"])
            .output()
        {
            let names = String::from_utf8_lossy(&output.stdout);
            for name in names.lines() {
                if name.starts_with(prefix) {
                    completions.push(name.to_string());
                }
            }
        }
        completions
    }

    fn complete_docker_images(&self, prefix: &str) -> Vec<String> {
        let mut completions = Vec::new();
        if let Ok(output) = std::process::Command::new("docker")
            .args(["images", "--format", "{{.Repository}}:{{.Tag}}"])
            .output()
        {
            let images = String::from_utf8_lossy(&output.stdout);
            for img in images.lines() {
                if img.starts_with(prefix) {
                    completions.push(img.to_string());
                }
            }
        }
        completions
    }

    fn complete_make_targets(&self, prefix: &str) -> Vec<String> {
        let mut completions = Vec::new();
        let makefiles = ["Makefile", "makefile", "GNUmakefile"];
        for mf in &makefiles {
            if let Ok(content) = std::fs::read_to_string(mf) {
                for line in content.lines() {
                    // Match lines like "target: deps"
                    if let Some(colon_pos) = line.find(':') {
                        let target = line[..colon_pos].trim();
                        if !target.is_empty() && !target.starts_with('#') && !target.starts_with('\t') {
                            // Handle multiple targets on one line
                            for t in target.split_whitespace() {
                                if t.starts_with(prefix) {
                                    completions.push(t.to_string());
                                }
                            }
                        }
                    }
                }
                break;
            }
        }
        completions
    }

    fn complete_command(&self, prefix: &str) -> Vec<String> {
        self.commands
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .take(20)
            .cloned()
            .collect()
    }

    fn complete_argument(&self, prefix: &str, shell: &Shell) -> Vec<String> {
        let mut completions = Vec::new();

        // Variable completion
        if let Some(var_prefix) = prefix.strip_prefix('$') {
            for key in shell.vars().keys() {
                if key.starts_with(var_prefix) {
                    completions.push(format!("${}", key));
                }
            }
            return completions;
        }

        // Alias completion
        if let Some(alias_prefix) = prefix.strip_prefix('!') {
            for name in shell.aliases().keys() {
                if name.starts_with(alias_prefix) {
                    completions.push(format!("!{}", name));
                }
            }
            return completions;
        }

        // File path completion
        let (dir, file_prefix) = if let Some(pos) = prefix.rfind('/') {
            let dir = &prefix[..pos];
            let file = &prefix[pos + 1..];
            (if dir.is_empty() { "/" } else { dir }, file)
        } else {
            (".", prefix)
        };

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(file_prefix) && !name.starts_with('.') {
                        let path = format!("{}/{}", dir, name);
                        let path = path.replace("//", "/");
                        if entry.path().is_dir() {
                            completions.push(format!("{}/", path));
                        } else {
                            completions.push(path);
                        }
                    }
                }
            }
        }

        completions.sort();
        completions.truncate(20);
        completions
    }
}
