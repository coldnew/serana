//! Shell output minimizer.
//!
//! Post-execution output compression for verbose shell commands.
//! Detects the running program and applies per-tool filters to reduce
//! token usage while preserving important information.

use regex::Regex;

/// Result of minimization.
pub struct MinimizedOutput {
    /// The minimized text (or original if no minimization applied).
    pub text: String,
    /// The original raw text before minimization.
    pub original_text: String,
    /// Name of the filter that was applied, if any.
    pub filter: Option<&'static str>,
}

/// Minimize shell output by detecting the program and applying appropriate filters.
pub fn minimize(command: &str, output: &str, exit_code: i32) -> MinimizedOutput {
    let original = output.to_string();

    // Only minimize single commands (no pipes, &&, ||, ;)
    if is_compound_command(command) {
        return MinimizedOutput {
            text: original.clone(),
            original_text: original,
            filter: None,
        };
    }

    let Some(identity) = detect_command(command) else {
        return MinimizedOutput {
            text: original.clone(),
            original_text: original,
            filter: None,
        };
    };

    let minimized = match identity.program.as_str() {
        "git" => minimize_git(&identity.subcommand, output, exit_code),
        "cargo" => minimize_cargo(&identity.subcommand, output, exit_code),
        "ls" | "tree" | "find" | "du" | "df" => Some(minimize_listing(output)),
        "cat" | "head" | "tail" | "wc" => Some(minimize_listing(output)),
        _ => None,
    };

    match minimized {
        Some(text) if !text.is_empty() => {
            let text = ensure_success_visible(text, exit_code);
            MinimizedOutput {
                text,
                original_text: original,
                filter: Some(identity.program.leak()),
            }
        }
        _ => MinimizedOutput {
            text: original.clone(),
            original_text: original,
            filter: None,
        },
    }
}

/// Detected command identity.
struct CommandIdentity {
    program: String,
    subcommand: String,
}

/// Check if a command is compound (pipes, chains, etc.).
fn is_compound_command(command: &str) -> bool {
    // Quick check for shell operators
    let mut in_single = false;
    let mut in_double = false;
    for c in command.chars() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '|' | '&' | ';' if !in_single && !in_double => return true,
            _ => {}
        }
    }
    false
}

/// Detect program and subcommand from a command string.
fn detect_command(command: &str) -> Option<CommandIdentity> {
    let tokens = tokenize(command);
    if tokens.is_empty() {
        return None;
    }

    // Strip launch prefixes
    let mut idx = 0;
    while idx < tokens.len() {
        let t = &tokens[idx];
        // Skip env assignments
        if t.contains('=') && !t.starts_with('-') {
            idx += 1;
            continue;
        }
        // Skip common prefixes
        match t.as_str() {
            "sudo" | "env" | "command" | "builtin" | "noglob" | "exec" | "time" | "nice"
            | "nohup" => {
                idx += 1;
                // sudo may have flags
                if t == "sudo" {
                    while idx < tokens.len() && tokens[idx].starts_with('-') {
                        idx += 1;
                    }
                }
                continue;
            }
            _ => break,
        }
    }

    if idx >= tokens.len() {
        return None;
    }

    let program = extract_basename(&tokens[idx]);
    let subcommand = detect_subcommand(&program, &tokens[idx + 1..]);

    Some(CommandIdentity {
        program,
        subcommand,
    })
}

/// Extract the basename of a program path.
fn extract_basename(path: &str) -> String {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .to_lowercase()
        .to_string()
}

/// Detect subcommand for known programs.
fn detect_subcommand(program: &str, args: &[String]) -> String {
    match program {
        "git" => detect_git_subcommand(args),
        "cargo" => detect_cargo_subcommand(args),
        "gh" => detect_gh_subcommand(args),
        _ => {
            // Default: first non-flag argument
            args.iter()
                .find(|a| !a.starts_with('-'))
                .cloned()
                .unwrap_or_default()
        }
    }
}

fn detect_git_subcommand(args: &[String]) -> String {
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "-C" | "-c" | "--git-dir" | "--work-tree" => {
                i += 2; // skip flag + value
                continue;
            }
            "--bare" | "--no-pager" | "--no-optional-locks" | "--literal-pathspecs" => {
                i += 1;
                continue;
            }
            _ if a.starts_with("--git-dir=")
                || a.starts_with("--work-tree=")
                || a.starts_with("-C")
                || a.starts_with("-c") =>
            {
                i += 1;
                continue;
            }
            _ if a.starts_with('-') => {
                i += 1;
                continue;
            }
            _ => return a.clone(),
        }
    }
    String::new()
}

fn detect_cargo_subcommand(args: &[String]) -> String {
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "-C" | "--manifest-path" | "--target-dir" | "--color" => {
                i += 2;
                continue;
            }
            "--locked" | "--frozen" | "--offline" | "--quiet" | "--verbose" | "-q" | "-v" => {
                i += 1;
                continue;
            }
            _ if a.starts_with('+') => {
                // Toolchain selector: +nightly
                i += 1;
                continue;
            }
            _ if a.starts_with("--manifest-path=")
                || a.starts_with("--target-dir=")
                || a.starts_with("--color=") =>
            {
                i += 1;
                continue;
            }
            _ if a.starts_with('-') => {
                i += 1;
                continue;
            }
            _ => return a.clone(),
        }
    }
    String::new()
}

fn detect_gh_subcommand(args: &[String]) -> String {
    args.iter()
        .find(|a| !a.starts_with('-'))
        .cloned()
        .unwrap_or_default()
}

/// Simple tokenizer that respects quotes.
fn tokenize(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;

    for c in command.chars() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            ' ' | '\t' if !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

// ── Text Primitives ──────────────────────────────────────────────────────────

/// Strip ANSI CSI escape sequences.
fn strip_ansi(text: &str) -> String {
    let re = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(text, "").to_string()
}

/// Keep first `head` and last `tail` lines, with an omission marker.
fn head_tail_lines(text: &str, head: usize, tail: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.len();
    if total <= head + tail {
        return text.to_string();
    }

    let mut result: Vec<String> = Vec::new();
    for line in lines.iter().take(head) {
        result.push(line.to_string());
    }
    result.push(format!("... {} lines omitted ...", total - head - tail));
    for line in lines.iter().skip(total - tail) {
        result.push(line.to_string());
    }
    result.join("\n")
}

/// Collapse consecutive duplicate lines.
fn dedup_consecutive(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    let mut result = Vec::new();
    let mut prev = lines[0];
    let mut count = 1;

    for line in &lines[1..] {
        if *line == prev {
            count += 1;
        } else {
            if count > 1 {
                result.push(format!("{} (x{})", prev, count));
            } else {
                result.push(prev.to_string());
            }
            prev = line;
            count = 1;
        }
    }
    if count > 1 {
        result.push(format!("{} (x{})", prev, count));
    } else {
        result.push(prev.to_string());
    }
    result.join("\n")
}

/// Truncate each line to max_chars.
fn truncate_lines(text: &str, max_chars: usize) -> String {
    text.lines()
        .map(|line| {
            if line.chars().count() > max_chars {
                let truncated: String = line.chars().take(max_chars).collect();
                let dropped = line.chars().count() - max_chars;
                format!("{}...[+{}]", truncated, dropped)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Strip lines matching any of the given patterns.
fn strip_lines_matching(text: &str, patterns: &[&str]) -> String {
    let regexes: Vec<Regex> = patterns
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect();

    text.lines()
        .filter(|line| !regexes.iter().any(|re| re.is_match(line)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Ensure that empty success output shows "OK".
fn ensure_success_visible(text: String, exit_code: i32) -> String {
    if exit_code == 0 && text.trim().is_empty() {
        "OK\n".to_string()
    } else {
        text
    }
}

// ── Git Filter ───────────────────────────────────────────────────────────────

fn minimize_git(subcommand: &str, output: &str, _exit_code: i32) -> Option<String> {
    let output = strip_ansi(output);

    match subcommand {
        "diff" => Some(condense_diff(&output)),
        "show" => {
            // show HEAD:path is file content, don't minimize
            if output.lines().count() > 100 {
                Some(head_tail_lines(&output, 80, 40))
            } else {
                Some(output)
            }
        }
        "log" => Some(condense_log(&output)),
        "branch" | "stash" | "tag" => Some(compact_listing(&output, 40)),
        "push" | "pull" | "fetch" | "merge" | "rebase" | "checkout" | "switch" | "restore"
        | "clean" | "reset" | "add" | "commit" => Some(condense_noisy_output(&output)),
        "status" => Some(condense_git_status(&output)),
        _ => None,
    }
}

fn condense_diff(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() <= 80 {
        return output.to_string();
    }

    let mut stat_lines = Vec::new();
    let mut hunks = Vec::new();
    let mut current_file = String::new();
    let mut hunk_lines = Vec::new();
    let mut files_seen = 0;
    let max_files = 20;
    let max_hunks_per_file = 8;
    let max_lines_per_hunk = 6;
    let mut hunks_in_file = 0;

    for line in &lines {
        if line.starts_with("diff --git") {
            if !hunk_lines.is_empty() {
                hunks.extend(truncate_hunk(&hunk_lines, max_lines_per_hunk));
                hunk_lines.clear();
            }
            files_seen += 1;
            if files_seen > max_files {
                hunks.push(format!("... and more files ..."));
                break;
            }
            hunks_in_file = 0;
            if let Some(name) = line.split(" b/").nth(1) {
                current_file = name.to_string();
            }
        } else if line.starts_with("@@") {
            if !hunk_lines.is_empty() {
                hunks.extend(truncate_hunk(&hunk_lines, max_lines_per_hunk));
                hunk_lines.clear();
            }
            hunks_in_file += 1;
            if hunks_in_file > max_hunks_per_file {
                hunks.push(format!("... more hunks in {} ...", current_file));
                continue;
            }
            hunk_lines.push(line.to_string());
        } else if line.starts_with("+++") || line.starts_with("---") {
            stat_lines.push(line.to_string());
        } else if line.starts_with("+") || line.starts_with("-") || line.starts_with(" ") {
            hunk_lines.push(line.to_string());
        } else {
            hunk_lines.push(line.to_string());
        }
    }
    if !hunk_lines.is_empty() {
        hunks.extend(truncate_hunk(&hunk_lines, max_lines_per_hunk));
    }

    let mut result = stat_lines.join("\n");
    if !result.is_empty() {
        result.push('\n');
    }
    result.push_str(&hunks.join("\n"));
    truncate_lines(&result, 160)
}

fn truncate_hunk(lines: &[String], max: usize) -> Vec<String> {
    if lines.len() <= max {
        lines.to_vec()
    } else {
        let mut result: Vec<String> = lines.iter().take(max).cloned().collect();
        result.push(format!("... {} more lines ...", lines.len() - max));
        result
    }
}

fn condense_log(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() <= 32 {
        return output.to_string();
    }

    let mut entries = Vec::new();
    let mut current = Vec::new();

    for line in &lines {
        if line.starts_with("commit ") {
            if !current.is_empty() {
                entries.push(current.join("\n"));
                current.clear();
            }
        }
        current.push(line.to_string());
    }
    if !current.is_empty() {
        entries.push(current.join("\n"));
    }

    // Extract short hash + subject from each entry
    let condensed: Vec<String> = entries
        .iter()
        .map(|entry| {
            let lines: Vec<&str> = entry.lines().collect();
            let hash = lines
                .first()
                .and_then(|l| l.strip_prefix("commit "))
                .map(|h| &h[..h.len().min(8)])
                .unwrap_or("?");
            let subject = lines
                .iter()
                .find(|l| !l.starts_with("commit") && !l.starts_with("Author") && !l.starts_with("Date") && !l.trim().is_empty())
                .map_or("(no subject)", |v| v);
            format!("{} {}", hash, subject)
        })
        .collect();

    head_tail_lines(&condensed.join("\n"), 16, 8)
}

fn compact_listing(output: &str, max: usize) -> String {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() <= max {
        return output.to_string();
    }
    let count = lines.len();
    let truncated = head_tail_lines(output, max / 2, max / 2);
    format!("{} ({} entries)\n{}", count, count, truncated)
}

fn condense_noisy_output(output: &str) -> String {
    let deduped = dedup_consecutive(output);
    head_tail_lines(&deduped, 80, 40)
}

fn condense_git_status(output: &str) -> String {
    let output = strip_ansi(output);
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() <= 50 {
        return output;
    }

    let mut staged = 0;
    let mut modified = 0;
    let mut untracked = 0;
    let mut deleted = 0;
    let mut other = 0;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with("M ") || trimmed.starts_with("A ") || trimmed.starts_with("R ") {
            staged += 1;
        } else if trimmed.starts_with(" M") || trimmed.starts_with("MM") {
            modified += 1;
        } else if trimmed.starts_with("??") {
            untracked += 1;
        } else if trimmed.starts_with(" D") || trimmed.starts_with("D ") {
            deleted += 1;
        } else if !trimmed.is_empty() && !trimmed.starts_with("On branch") && !trimmed.starts_with("Your branch") {
            other += 1;
        }
    }

    let mut summary = Vec::new();
    if staged > 0 {
        summary.push(format!("{} staged", staged));
    }
    if modified > 0 {
        summary.push(format!("{} modified", modified));
    }
    if untracked > 0 {
        summary.push(format!("{} untracked", untracked));
    }
    if deleted > 0 {
        summary.push(format!("{} deleted", deleted));
    }
    if other > 0 {
        summary.push(format!("{} other", other));
    }

    format!("git status: {}", summary.join(", "))
}

// ── Cargo Filter ─────────────────────────────────────────────────────────────

fn minimize_cargo(subcommand: &str, output: &str, exit_code: i32) -> Option<String> {
    let output = strip_ansi(output);

    match subcommand {
        "build" | "check" | "clippy" | "doc" | "run" => Some(condense_build(&output)),
        "test" | "bench" => Some(condense_test(&output, exit_code)),
        "fmt" => Some(condense_fmt(&output)),
        "metadata" => None, // JSON, don't touch
        "tree" | "update" | "install" | "publish" => Some(compact_general(&output)),
        _ => None,
    }
}

fn condense_build(output: &str) -> String {
    let noise = [
        r"^Compiling\s+",
        r"^Checking\s+",
        r"^Downloading\s+",
        r"^Downloaded\s+",
        r"^Locking\s+\d+\s+packages?",
        r"^Updating\s+",
        r"^Fresh\s+",
        r"^Adding\s+",
        r"^Verifying\s+",
        r"^Documenting\s+",
    ];

    let filtered = strip_lines_matching(output, &noise);
    let deduped = dedup_consecutive(&filtered);
    let result = head_tail_lines(&deduped, 120, 60);

    if result.trim().is_empty() && output.contains("Finished") {
        // Extract just the Finished line
        output
            .lines()
            .find(|l| l.starts_with("Finished"))
            .map(|l| format!("cargo: {}", l))
            .unwrap_or_else(|| "cargo: OK".to_string())
    } else {
        result
    }
}

fn condense_test(output: &str, exit_code: i32) -> String {
    if exit_code == 0 {
        // On success, summarize test results
        let result_line = output
            .lines()
            .find(|l| l.starts_with("test result:"));

        if let Some(line) = result_line {
            return format!("cargo test: {}", line);
        }

        // Try to extract summary from multiple test suites
        let suites: Vec<&str> = output
            .lines()
            .filter(|l| l.starts_with("test result:"))
            .collect();

        if !suites.is_empty() {
            return suites.join("\n");
        }

        "cargo test: OK".to_string()
    } else {
        // On failure, keep only failure-related lines
        let failure_patterns = [
            r"^failures:",
            r"^---- ",
            r"^error\[",
            r"^error:",
            r"^thread '.*panicked",
            r"^test result: FAILED",
            r"^FAILED",
        ];
        let failures = strip_lines_matching_keep(output, &failure_patterns);
        if failures.trim().is_empty() {
            // Keep last 40 lines as fallback
            head_tail_lines(output, 0, 40)
        } else {
            failures
        }
    }
}

fn condense_fmt(output: &str) -> String {
    let deduped = dedup_consecutive(output);
    head_tail_lines(&deduped, 80, 40)
}

fn compact_general(output: &str) -> String {
    let noise = [
        r"^Updating\s+",
        r"^Locking\s+",
        r"^Downloading\s+",
        r"^Downloaded\s+",
        r"^Verifying\s+",
        r"^Installing\s+",
        r"^Adding\s+",
        r"^Removed\s+",
    ];
    let filtered = strip_lines_matching(output, &noise);
    let deduped = dedup_consecutive(&filtered);
    head_tail_lines(&deduped, 80, 40)
}

// ── Listing Filter ───────────────────────────────────────────────────────────

fn minimize_listing(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() <= 80 {
        return output.to_string();
    }
    head_tail_lines(output, 40, 20)
}

// ── Keep-only filter ─────────────────────────────────────────────────────────

/// Keep only lines matching at least one pattern.
fn strip_lines_matching_keep(text: &str, patterns: &[&str]) -> String {
    let regexes: Vec<Regex> = patterns
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect();

    text.lines()
        .filter(|line| regexes.iter().any(|re| re.is_match(line)))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("git status");
        assert_eq!(tokens, vec!["git", "status"]);
    }

    #[test]
    fn test_tokenize_quotes() {
        let tokens = tokenize(r#"git commit -m "hello world""#);
        assert_eq!(tokens, vec!["git", "commit", "-m", "hello world"]);
    }

    #[test]
    fn test_is_compound() {
        assert!(!is_compound_command("git status"));
        assert!(is_compound_command("git status && echo ok"));
        assert!(is_compound_command("cat foo | grep bar"));
        assert!(is_compound_command("echo a; echo b"));
    }

    #[test]
    fn test_detect_git() {
        let id = detect_command("git diff").unwrap();
        assert_eq!(id.program, "git");
        assert_eq!(id.subcommand, "diff");
    }

    #[test]
    fn test_detect_git_with_flags() {
        let id = detect_command("git -C /tmp log --oneline").unwrap();
        assert_eq!(id.program, "git");
        assert_eq!(id.subcommand, "log");
    }

    #[test]
    fn test_detect_cargo() {
        let id = detect_command("cargo test").unwrap();
        assert_eq!(id.program, "cargo");
        assert_eq!(id.subcommand, "test");
    }

    #[test]
    fn test_detect_cargo_with_flags() {
        let id = detect_command("cargo +nightly clippy -- -W warnings").unwrap();
        assert_eq!(id.program, "cargo");
        assert_eq!(id.subcommand, "clippy");
    }

    #[test]
    fn test_detect_sudo() {
        let id = detect_command("sudo apt install foo").unwrap();
        assert_eq!(id.program, "apt");
        assert_eq!(id.subcommand, "install");
    }

    #[test]
    fn test_strip_ansi() {
        assert_eq!(strip_ansi("\x1b[31mhello\x1b[0m"), "hello");
    }

    #[test]
    fn test_head_tail() {
        let text = (1..=100)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let result = head_tail_lines(&text, 3, 2);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 6); // 3 head + marker + 2 tail
        assert!(lines[3].contains("95 lines omitted"));
    }

    #[test]
    fn test_dedup_consecutive() {
        let input = "a\na\na\nb\nb\nc";
        assert_eq!(dedup_consecutive(input), "a (x3)\nb (x2)\nc");
    }

    #[test]
    fn test_minimize_git_log() {
        let output = (0..50)
            .map(|i| format!("commit abcdef{:02}\nAuthor: Test\nDate: Today\n\n    Commit {}\n", i, i))
            .collect::<Vec<_>>()
            .join("\n");
        let result = minimize("git log", &output, 0);
        assert!(result.text.lines().count() < output.lines().count());
    }

    #[test]
    fn test_minimize_cargo_test_success() {
        let output = "running 10 tests\ntest ok1 ... ok\ntest ok2 ... ok\n\ntest result: ok. 10 passed; 0 failed; 0 ignored\n";
        let result = minimize("cargo test", output, 0);
        assert!(result.text.contains("test result: ok"));
    }

    #[test]
    fn test_minimize_cargo_test_failure() {
        let output = "running 10 tests\ntest ok1 ... ok\ntest bad1 ... FAILED\n\nfailures:\n---- bad1 stdout ----\nthread 'bad1' panicked\n\ntest result: FAILED. 9 passed; 1 failed\n";
        let result = minimize("cargo test", output, 1);
        assert!(result.text.contains("FAILED"));
        assert!(result.text.contains("failures:"));
    }

    #[test]
    fn test_minimize_cargo_build() {
        let output = "Compiling foo v0.1.0\nCompiling bar v0.2.0\nCompiling baz v0.3.0\nFinished release [optimized]\n";
        let result = minimize("cargo build", output, 0);
        assert!(result.text.contains("Finished"));
    }

    #[test]
    fn test_no_minimize_for_unknown() {
        let output = "some output\n";
        let result = minimize("unknown_cmd", output, 0);
        assert_eq!(result.text, output);
        assert!(result.filter.is_none());
    }

    #[test]
    fn test_no_minimize_for_compound() {
        let output = "output\n";
        let result = minimize("git status && echo ok", output, 0);
        assert_eq!(result.text, output);
        assert!(result.filter.is_none());
    }
}
