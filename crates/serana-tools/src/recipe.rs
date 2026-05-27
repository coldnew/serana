use async_trait::async_trait;
use serde_json::{json, Value};
use std::time::Duration;

use serana_core::{Result, Tool};

/// Detect and run a task from the project's task runner.
pub struct RecipeTool;

enum Runner {
    Cargo,
    Just,
    Make,
    Npm,
}

impl Runner {
    fn name(&self) -> &'static str {
        match self {
            Runner::Cargo => "cargo",
            Runner::Just => "just",
            Runner::Make => "make",
            Runner::Npm => "npm",
        }
    }

    fn command<'a>(&self, task: &'a str) -> (&'static str, Vec<&'a str>) {
        match self {
            Runner::Cargo => ("cargo", vec![task]),
            Runner::Just => ("just", vec![task]),
            Runner::Make => ("make", vec![task]),
            Runner::Npm => ("npm", vec!["run", task]),
        }
    }
}

/// Detect which task runner is available in `cwd`, checking files in priority order.
fn detect_runner(cwd: &std::path::Path) -> Option<Runner> {
    if cwd.join("Cargo.toml").exists() {
        return Some(Runner::Cargo);
    }
    if cwd.join("Justfile").exists() || cwd.join("justfile").exists() {
        return Some(Runner::Just);
    }
    if cwd.join("Makefile").exists() {
        return Some(Runner::Make);
    }
    if cwd.join("package.json").exists() {
        return Some(Runner::Npm);
    }
    None
}

#[async_trait]
impl Tool for RecipeTool {
    fn name(&self) -> &'static str {
        "recipe"
    }

    fn description(&self) -> &'static str {
        "Run a task from the project's task runner (Makefile, Justfile, package.json, Cargo.toml). Input: {\"task\": \"build\", \"cwd\": \".\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "Task/command name to run (e.g. build, test, check)"
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory to detect task runner in (defaults to \".\")"
                }
            },
            "required": ["task"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let task = input
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'task' field"))?;

        let cwd_str = input.get("cwd").and_then(|v| v.as_str()).unwrap_or(".");
        let cwd = std::path::Path::new(cwd_str);
        let cwd = if cwd.is_relative() {
            std::env::current_dir()?.join(cwd)
        } else {
            cwd.to_path_buf()
        };

        let runner = detect_runner(&cwd).ok_or_else(|| {
            anyhow::anyhow!("No supported task runner found in {}", cwd.display())
        })?;

        let runner_name = runner.name();
        let (program, args) = runner.command(task);
        let full_command = format!("{} {}", program, args.join(" "));

        let output = tokio::time::timeout(
            Duration::from_secs(120),
            tokio::process::Command::new(program)
                .args(&args)
                .current_dir(&cwd)
                .output(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Command timed out after 120s: {}", full_command))?
        .map_err(|e| anyhow::anyhow!("Failed to run '{}': {}", full_command, e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(json!({
            "runner": runner_name,
            "command": full_command,
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
        }))
    }
}
