use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::Result;

/// Result of a verification run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub success: bool,
    pub tests_passed: u32,
    pub tests_failed: u32,
    pub build_success: bool,
    pub output: String,
    pub errors: Vec<String>,
}

/// Snapshot of Serana's state before modification for rollback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub timestamp: String,
    pub git_head: String,
    pub modified_files: Vec<String>,
    pub stashed: bool,
}

/// Verification system for safe self-modification.
pub struct VerificationSystem {
    workspace: PathBuf,
}

impl VerificationSystem {
    pub fn new() -> Self {
        Self {
            workspace: find_workspace_root(),
        }
    }

    pub fn verify(&self) -> Result<VerificationResult> {
        let build_output = Command::new("cargo")
            .current_dir(&self.workspace)
            .args(["build"])
            .output()?;

        let build_success = build_output.status.success();
        let mut errors = Vec::new();
        let mut output = String::new();

        if !build_success {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            errors.push(format!(
                "Build failed: {}",
                stderr.lines().take(5).collect::<Vec<_>>().join("\n")
            ));
            return Ok(VerificationResult {
                success: false,
                tests_passed: 0,
                tests_failed: 0,
                build_success: false,
                output: stderr.to_string(),
                errors,
            });
        }

        let test_output = Command::new("cargo")
            .current_dir(&self.workspace)
            .args(["test", "--quiet"])
            .output()?;

        let stdout = String::from_utf8_lossy(&test_output.stdout);
        let stderr = String::from_utf8_lossy(&test_output.stderr);
        output = format!("{}\n{}", stdout, stderr);

        let tests_passed = parse_test_count(&output, "passed");
        let tests_failed = parse_test_count(&output, "failed");
        let success = test_output.status.success() && tests_failed == 0;

        if !success {
            errors.push(format!("{} tests failed", tests_failed));
        }

        Ok(VerificationResult {
            success,
            tests_passed,
            tests_failed,
            build_success: true,
            output,
            errors,
        })
    }

    pub fn create_snapshot(&self) -> Result<StateSnapshot> {
        let head_output = Command::new("git")
            .current_dir(&self.workspace)
            .args(["rev-parse", "HEAD"])
            .output()?;

        let git_head = String::from_utf8_lossy(&head_output.stdout)
            .trim()
            .to_string();

        let status_output = Command::new("git")
            .current_dir(&self.workspace)
            .args(["status", "--porcelain"])
            .output()?;

        let status = String::from_utf8_lossy(&status_output.stdout);
        let modified_files: Vec<String> =
            status.lines().map(|line| line[3..].to_string()).collect();

        let stashed = if !modified_files.is_empty() {
            let _ = Command::new("git")
                .current_dir(&self.workspace)
                .args(["stash", "-u"])
                .output();
            true
        } else {
            false
        };

        Ok(StateSnapshot {
            timestamp: chrono_lite_timestamp(),
            git_head,
            modified_files,
            stashed,
        })
    }

    pub fn rollback(&self, snapshot: &StateSnapshot) -> Result<()> {
        let _ = Command::new("git")
            .current_dir(&self.workspace)
            .args(["reset", "--hard", &snapshot.git_head])
            .output();

        if snapshot.stashed {
            let _ = Command::new("git")
                .current_dir(&self.workspace)
                .args(["stash", "pop"])
                .output();
        }

        Ok(())
    }
}

impl Default for VerificationSystem {
    fn default() -> Self {
        Self::new()
    }
}

fn find_workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dir = Some(manifest_dir.as_path());
    while let Some(d) = dir {
        let cargo_toml = d.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return d.to_path_buf();
                }
            }
        }
        dir = d.parent();
    }
    manifest_dir
}

fn parse_test_count(output: &str, kind: &str) -> u32 {
    for line in output.lines() {
        if line.contains(kind) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for i in 0..parts.len().saturating_sub(1) {
                if parts[i + 1] == kind || parts[i + 1].starts_with(&format!("{};", kind)) {
                    if let Ok(count) = parts[i].parse::<u32>() {
                        return count;
                    }
                }
            }
        }
    }
    0
}

fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let secs = duration.as_secs();
    let datetime = time_offset::from_unix_timestamp(secs as i64);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        datetime.year,
        datetime.month,
        datetime.day,
        datetime.hour,
        datetime.minute,
        datetime.second
    )
}

mod time_offset {
    pub struct DateTime {
        pub year: i32,
        pub month: u8,
        pub day: u8,
        pub hour: u8,
        pub minute: u8,
        pub second: u8,
    }

    pub fn from_unix_timestamp(ts: i64) -> DateTime {
        let days = ts / 86400;
        let secs = ts % 86400;
        let (year, month, day) = days_to_ymd(days as i32);
        DateTime {
            year,
            month,
            day,
            hour: (secs / 3600) as u8,
            minute: ((secs % 3600) / 60) as u8,
            second: (secs % 60) as u8,
        }
    }

    fn days_to_ymd(mut days: i32) -> (i32, u8, u8) {
        days += 719163;
        let era = (if days >= 0 { days } else { days - 146096 }) / 146097;
        let doe = days - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = mp + if mp < 10 { 3 } else { -9 };
        (y + if m <= 2 { 1 } else { 0 }, m as u8, d as u8)
    }
}
