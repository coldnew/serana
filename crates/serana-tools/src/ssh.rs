//! SSH remote command execution tool.

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use serana_core::{Result, Tool};

/// Run a command on a remote host via SSH.
pub struct SshTool;

#[async_trait]
impl Tool for SshTool {
    fn name(&self) -> &'static str {
        "ssh"
    }

    fn description(&self) -> &'static str {
        "Run a command on a remote host via SSH. Input: {\"host\": \"user@host\", \"command\": \"ls -la\"}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "host": {
                    "type": "string",
                    "description": "Remote host in user@host format"
                },
                "command": {
                    "type": "string",
                    "description": "Command to execute on the remote host"
                }
            },
            "required": ["host", "command"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let host = input
            .get("host")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'host' field"))?;

        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' field"))?;

        let result = timeout(
            Duration::from_secs(30),
            Command::new("ssh")
                .arg(host)
                .arg(command)
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => Ok(json!({
                "stdout": String::from_utf8_lossy(&output.stdout),
                "stderr": String::from_utf8_lossy(&output.stderr),
                "exit_code": output.status.code().unwrap_or(-1),
            })),
            Ok(Err(e)) => Err(anyhow::anyhow!("SSH command failed: {}", e)),
            Err(_) => Ok(json!({
                "stdout": "",
                "stderr": "Command timed out after 30 seconds",
                "exit_code": -1,
            })),
        }
    }
}
