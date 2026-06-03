use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::core::{LlmClient, Message, Result, ToolDefinition};

pub fn login_device_auth() -> anyhow::Result<()> {
    let mut child = std::process::Command::new("codex")
        .args(["login", "--device-auth"])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "Codex CLI was not found in PATH. Install it, then run `serana login codex` again."
                )
            } else {
                anyhow::anyhow!("failed to start Codex CLI: {}", error)
            }
        })?;

    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("Codex device sign-in exited with {}", status);
    }

    Ok(())
}

pub struct CodexClient {
    model: String,
    workspace: Option<PathBuf>,
}

impl CodexClient {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            workspace: None,
        }
    }

    pub fn with_workspace(mut self, workspace: PathBuf) -> Self {
        self.workspace = Some(workspace);
        self
    }

    fn prompt_from_messages(messages: &[Message]) -> String {
        messages
            .iter()
            .map(|message| match message {
                Message::Text { role, content } => format!("{}:\n{}", role, content),
                Message::ToolCall {
                    role,
                    content,
                    tool_calls,
                } => format!(
                    "{}:\n{}\n[{} tool call(s) omitted]",
                    role,
                    content.as_deref().unwrap_or(""),
                    tool_calls.len()
                ),
                Message::ToolResult {
                    role,
                    tool_call_id,
                    content,
                } => format!("{} [{}]:\n{}", role, tool_call_id, content),
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn exec_args(&self, output_path: &Path) -> Vec<String> {
        let mut args = vec![
            "exec".to_string(),
            "--model".to_string(),
            self.model.clone(),
            "--sandbox".to_string(),
            "read-only".to_string(),
            "--skip-git-repo-check".to_string(),
            "--ephemeral".to_string(),
            "--output-last-message".to_string(),
            output_path.display().to_string(),
            "--color".to_string(),
            "never".to_string(),
        ];
        if let Some(workspace) = &self.workspace {
            args.push("-C".to_string());
            args.push(workspace.display().to_string());
        }
        args.push("-".to_string());
        args
    }
}

#[async_trait]
impl LlmClient for CodexClient {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let prompt = Self::prompt_from_messages(messages);
        let output_path =
            std::env::temp_dir().join(format!("serana-codex-{}.txt", uuid::Uuid::new_v4()));
        let mut child = Command::new("codex")
            .args(self.exec_args(&output_path))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    anyhow::anyhow!(
                        "Codex CLI was not found in PATH. Install it, then run `serana login codex`."
                    )
                } else {
                    anyhow::anyhow!("failed to start Codex CLI: {}", error)
                }
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes()).await?;
        }

        let output = child.wait_with_output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Codex CLI exited with {}: {}", output.status, stderr.trim());
        }

        if let Ok(message) = tokio::fs::read_to_string(&output_path).await {
            let _ = tokio::fs::remove_file(&output_path).await;
            let message = message.trim();
            if !message.is_empty() {
                return Ok(message.to_string());
            }
        }

        let stdout = String::from_utf8(output.stdout)?;
        if stdout.trim().is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(stderr.trim().to_string());
        }
        Ok(stdout.trim().to_string())
    }

    async fn chat_with_tools(
        &self,
        messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<Message> {
        Ok(Message::assistant(self.chat(messages).await?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_prompt_includes_roles() {
        let prompt = CodexClient::prompt_from_messages(&[
            Message::system("Be brief".to_string()),
            Message::user("Hello".to_string()),
        ]);

        assert!(prompt.contains("system:\nBe brief"));
        assert!(prompt.contains("user:\nHello"));
    }

    #[test]
    fn codex_exec_args_select_model() {
        let args = CodexClient::new("gpt-5.5").exec_args(Path::new("/tmp/out.txt"));

        assert!(args
            .windows(2)
            .any(|pair| pair[0] == "--model" && pair[1] == "gpt-5.5"));
        assert!(args
            .windows(2)
            .any(|pair| pair[0] == "--sandbox" && pair[1] == "read-only"));
        assert!(args
            .windows(2)
            .any(|pair| pair[0] == "--output-last-message" && pair[1] == "/tmp/out.txt"));
        assert!(!args.iter().any(|arg| arg == "--ask-for-approval"));
    }
}
