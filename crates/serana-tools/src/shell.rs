//! Embedded shell with persistent sessions.
//!
//! Maintains a long-running bash process that can receive commands
//! via stdin, avoiding fork/exec overhead for each command.
//! Output is automatically minimized to reduce token usage.

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use serana_core::{Result, Tool};

use crate::minimizer;

/// A persistent shell session.
pub struct ShellSession {
    child: Mutex<Child>,
    writer: Mutex<tokio::process::ChildStdin>,
}

impl ShellSession {
    /// Start a new persistent bash session.
    pub async fn new() -> Result<Self> {
        // Try to use bash
        let shell = if Command::new("bash").arg("--version").output().await.is_ok() {
            "bash"
        } else {
            "sh"
        };

        let mut child = Command::new(shell)
            .arg("--norc")
            .arg("--noprofile")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("No stdin"))?;

        Ok(Self {
            child: Mutex::new(child),
            writer: Mutex::new(stdin),
        })
    }

    /// Execute a command in the shell session.
    pub async fn execute(&self, command: &str, timeout_secs: u64) -> Result<Value> {
        // Use a sentinel to detect command completion
        let sentinel = format!(
            "__SERANA_DONE_{}__",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );

        let wrapped = format!("{}\necho \"{}\"", command, sentinel);

        let mut writer = self.writer.lock().await;
        writer.write_all(wrapped.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        drop(writer);

        // Read output until sentinel
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout_secs);
        let mut child = self.child.lock().await;
        let stdout = child
            .stdout
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No stdout"))?;

        let mut buf = Vec::new();
        let mut temp = [0u8; 4096];

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Ok(json!({
                    "stdout": String::from_utf8_lossy(&buf),
                    "stderr": "",
                    "exit_code": -1,
                    "timed_out": true,
                }));
            }

            match tokio::time::timeout(remaining, stdout.read(&mut temp)).await {
                Ok(Ok(0)) => break,
                Ok(Ok(n)) => {
                    buf.extend_from_slice(&temp[..n]);
                    let text = String::from_utf8_lossy(&buf);
                    if text.contains(&sentinel) {
                        // Remove sentinel from output
                        let clean = text.replace(&sentinel, "").to_string();
                        return Ok(json!({
                            "stdout": clean,
                            "stderr": "",
                            "exit_code": 0,
                        }));
                    }
                }
                Ok(Err(e)) => {
                    return Err(anyhow::anyhow!("Shell read error: {}", e));
                }
                Err(_) => {
                    return Ok(json!({
                        "stdout": String::from_utf8_lossy(&buf),
                        "stderr": "",
                        "exit_code": -1,
                        "timed_out": true,
                    }));
                }
            }
        }

        Ok(json!({
            "stdout": String::from_utf8_lossy(&buf),
            "stderr": "",
            "exit_code": 0,
        }))
    }

    pub async fn close(&self) -> Result<()> {
        let mut child = self.child.lock().await;
        child.kill().await.ok();
        Ok(())
    }
}

/// Persistent shell tool.
pub struct ShellTool {
    session: Mutex<Option<ShellSession>>,
}

impl ShellTool {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
        }
    }
}

impl Default for ShellTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &'static str {
        "shell"
    }

    fn description(&self) -> &'static str {
        "Run a command in a persistent shell session (no fork/exec per command). Input: {\"command\": \"ls -la\", \"timeout\": 30}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 30)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' field"))?;

        let timeout = input.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30);

        let mut guard = self.session.lock().await;

        // Lazily create session
        if guard.is_none() {
            let session = ShellSession::new().await?;
            *guard = Some(session);
        }

        let session = guard.as_ref().unwrap();
        let result = session.execute(command, timeout).await?;

        // Apply minimizer to reduce output tokens
        if let (Some(stdout), Some(exit_code)) = (
            result.get("stdout").and_then(|v| v.as_str()),
            result.get("exit_code").and_then(|v| v.as_i64()),
        ) {
            let minimized = minimizer::minimize(command, stdout, exit_code as i32);
            if minimized.filter.is_some() {
                return Ok(json!({
                    "stdout": minimized.text,
                    "stderr": result.get("stderr").and_then(|v| v.as_str()).unwrap_or(""),
                    "exit_code": exit_code,
                    "minimized": true,
                    "filter": minimized.filter.unwrap_or(""),
                    "original_bytes": minimized.original_text.len(),
                }));
            }
        }

        Ok(result)
    }
}
