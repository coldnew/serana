use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;

use serana_core::{Result, Tool};

pub struct PtyTool;

#[async_trait]
impl Tool for PtyTool {
    fn name(&self) -> &'static str {
        "pty"
    }

    fn description(&self) -> &'static str {
        "Run a command with stdin input (for interactive prompts). Input: {\"command\": \"ssh user@host\", \"input\": \"password\\n\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Command to run"
                },
                "input": {
                    "type": "string",
                    "description": "Input to send to stdin"
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

        let stdin_input = input.get("input").and_then(|v| v.as_str()).unwrap_or("");
        let timeout = input.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30);

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        if !stdin_input.is_empty() {
            use tokio::io::AsyncWriteExt;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(stdin_input.as_bytes()).await?;
                stdin.flush().await?;
            }
        }

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout),
            child.wait_with_output(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("PTY command timed out after {}s", timeout))?
        .map_err(|e| anyhow::anyhow!("PTY command failed: {}", e))?;

        Ok(json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code().unwrap_or(-1),
        }))
    }
}

pub mod workspace_isolation {
    use std::path::{Path, PathBuf};
    use tokio::fs;

    use serana_core::Result;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum IsolationStrategy {
        Auto,
        Copy,
        OverlayFs,
    }

    pub async fn isolate_workspace(source: &Path, strategy: IsolationStrategy) -> Result<PathBuf> {
        let tmp_base = std::env::temp_dir().join("serana-workspaces");
        fs::create_dir_all(&tmp_base).await?;

        let id = uuid::Uuid::new_v4();
        let dest = tmp_base.join(format!("ws-{}", id));

        match strategy {
            IsolationStrategy::Auto => {
                if try_btrfs_reflink(source, &dest).await.is_ok() {
                    return Ok(dest);
                }
                copy_dir_recursive(source, &dest).await?;
                Ok(dest)
            }
            IsolationStrategy::Copy => {
                copy_dir_recursive(source, &dest).await?;
                Ok(dest)
            }
            IsolationStrategy::OverlayFs => {
                try_overlayfs(source, &dest).await?;
                Ok(dest)
            }
        }
    }

    pub async fn cleanup_workspace(path: &Path) -> Result<()> {
        if path.exists() {
            fs::remove_dir_all(path).await?;
        }
        Ok(())
    }

    async fn try_btrfs_reflink(source: &Path, dest: &Path) -> Result<()> {
        let output = tokio::process::Command::new("cp")
            .args(["--reflink=always", "-a"])
            .arg(source)
            .arg(dest)
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("btrfs reflink not supported"))
        }
    }

    async fn try_overlayfs(source: &Path, dest: &Path) -> Result<()> {
        let work_dir = dest.with_extension("work");
        let upper_dir = dest.with_extension("upper");

        fs::create_dir_all(&work_dir).await?;
        fs::create_dir_all(&upper_dir).await?;
        fs::create_dir_all(dest).await?;

        let output = tokio::process::Command::new("mount")
            .args([
                "-t", "overlay", "overlay",
                &format!(
                    "lowerdir={},upperdir={},workdir={}",
                    source.display(), upper_dir.display(), work_dir.display()
                ),
            ])
            .arg(dest)
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("overlayfs mount failed"))
        }
    }

    async fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<()> {
        fs::create_dir_all(dest).await?;

        let mut entries = fs::read_dir(source).await?;
        while let Some(entry) = entries.next_entry().await? {
            let src_path = entry.path();
            let dst_path = dest.join(entry.file_name());

            if src_path.is_dir() {
                Box::pin(copy_dir_recursive(&src_path, &dst_path)).await?;
            } else {
                fs::copy(&src_path, &dst_path).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn pty_tool_runs_command() {
        let tool = PtyTool;
        let result = tool.execute(json!({"command": "echo hello"})).await.unwrap();
        assert_eq!(result["stdout"].as_str().unwrap().trim(), "hello");
        assert_eq!(result["exit_code"].as_i64().unwrap(), 0);
    }

    #[tokio::test]
    async fn pty_tool_sends_input() {
        let tool = PtyTool;
        let result = tool.execute(json!({"command": "cat", "input": "test input"})).await.unwrap();
        assert_eq!(result["stdout"].as_str().unwrap().trim(), "test input");
    }

    #[tokio::test]
    async fn workspace_copy_isolation() {
        let src = tempdir().unwrap();
        tokio::fs::write(src.path().join("test.txt"), "hello").await.unwrap();

        let dest = workspace_isolation::isolate_workspace(
            src.path(),
            workspace_isolation::IsolationStrategy::Copy,
        ).await.unwrap();

        let content = tokio::fs::read_to_string(dest.join("test.txt")).await.unwrap();
        assert_eq!(content, "hello");

        workspace_isolation::cleanup_workspace(&dest).await.unwrap();
    }
}
