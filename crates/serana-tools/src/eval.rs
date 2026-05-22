//! Eval tool: execute code in a subprocess kernel.

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use serana_core::{Result, Tool};

/// Execute code in a persistent kernel. Supports Python and JavaScript.
pub struct EvalTool;

const TIMEOUT_SECS: u64 = 30;

#[async_trait]
impl Tool for EvalTool {
    fn name(&self) -> &'static str {
        "eval"
    }

    fn description(&self) -> &'static str {
        "Execute code in a persistent kernel. Input: {\"language\": \"py\"|\"js\", \"code\": \"...\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "language": {
                    "type": "string",
                    "enum": ["py", "js"],
                    "description": "Runtime: \"py\" for Python 3, \"js\" for Node.js"
                },
                "code": {
                    "type": "string",
                    "description": "Code to execute"
                }
            },
            "required": ["language", "code"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let language = input
            .get("language")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'language' field"))?;

        let code = input
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'code' field"))?;

        let (cmd, args): (&str, &[&str]) = match language {
            "py" => ("python3", &[]),
            "js" => ("node", &[]),
            other => anyhow::bail!("Unsupported language: '{}'. Use \"py\" or \"js\"", other),
        };

        let mut child = Command::new(cmd)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Write code to stdin, then close it so the process can finish.
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(code.as_bytes()).await?;
            // stdin is dropped here, closing the pipe
        } else {
            anyhow::bail!("Failed to open stdin for subprocess");
        }

        let output = match tokio::time::timeout(
            std::time::Duration::from_secs(TIMEOUT_SECS),
            child.wait_with_output(),
        )
        .await
        {
            Ok(result) => result?,
            Err(_) => anyhow::bail!("Execution timed out after {} seconds", TIMEOUT_SECS),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
        }))
    }
}
