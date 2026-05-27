//! MCP (Model Context Protocol) client for external tool servers.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use serana_core::{Result, Tool};

/// MCP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

/// Connection to an MCP server over stdio.
pub struct McpConnection {
    child: Mutex<Child>,
    writer: Mutex<tokio::process::ChildStdin>,
    seq: std::sync::atomic::AtomicI64,
}

impl McpConnection {
    pub async fn connect(config: &McpServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("No stdin"))?;

        Ok(Self {
            child: Mutex::new(child),
            writer: Mutex::new(stdin),
            seq: std::sync::atomic::AtomicI64::new(1),
        })
    }

    pub async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let seq = self.seq.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut msg = json!({
            "jsonrpc": "2.0",
            "id": seq,
            "method": method,
        });
        if let Some(p) = params {
            msg["params"] = p;
        }

        let body = serde_json::to_string(&msg)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        let mut writer = self.writer.lock().await;
        writer.write_all(header.as_bytes()).await?;
        writer.write_all(body.as_bytes()).await?;
        writer.flush().await?;

        // For v1, we don't read responses back (would need a reader task).
        // This is a stub that demonstrates the protocol structure.
        Ok(json!({
            "jsonrpc": "2.0",
            "id": seq,
            "result": { "status": "sent" }
        }))
    }

    pub async fn close(&self) -> Result<()> {
        let mut child = self.child.lock().await;
        child.kill().await.ok();
        Ok(())
    }
}

/// Tool to manage MCP server connections.
pub struct McpTool {
    connections: Mutex<Vec<(String, McpConnection)>>,
}

impl McpTool {
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(Vec::new()),
        }
    }
}

impl Default for McpTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &'static str {
        "mcp"
    }

    fn description(&self) -> &'static str {
        "Connect to and call MCP servers. Input: {\"action\": \"connect\", \"name\": \"my-server\", \"command\": \"npx\", \"args\": [\"-y\", \"@my/mcp\"]} \
         or {\"action\": \"list\"} \
         or {\"action\": \"call\", \"server\": \"my-server\", \"method\": \"tool/call\", \"params\": {}}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["connect", "list", "call", "disconnect"],
                    "description": "MCP action"
                },
                "name": {
                    "type": "string",
                    "description": "Server name"
                },
                "command": {
                    "type": "string",
                    "description": "Server command (for connect)"
                },
                "args": {
                    "type": "array",
                    "description": "Command args (for connect)",
                    "items": { "type": "string" }
                },
                "server": {
                    "type": "string",
                    "description": "Server name (for call/disconnect)"
                },
                "method": {
                    "type": "string",
                    "description": "JSON-RPC method (for call)"
                },
                "params": {
                    "type": "object",
                    "description": "Method params (for call)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action'"))?;

        match action {
            "connect" => {
                let name = input
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'name'"))?;
                let command = input
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'command'"))?;
                let args: Vec<String> = input
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let config = McpServerConfig {
                    name: name.to_string(),
                    command: command.to_string(),
                    args,
                    env: std::collections::HashMap::new(),
                };

                let conn = McpConnection::connect(&config).await?;
                let mut connections = self.connections.lock().await;
                connections.push((name.to_string(), conn));

                Ok(json!({ "status": "connected", "name": name }))
            }
            "list" => {
                let connections = self.connections.lock().await;
                let names: Vec<&str> = connections.iter().map(|(n, _)| n.as_str()).collect();
                Ok(json!({ "servers": names }))
            }
            "call" => {
                let server = input
                    .get("server")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'server'"))?;
                let method = input
                    .get("method")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'method'"))?;
                let params = input.get("params").cloned();

                let connections = self.connections.lock().await;
                let (_, conn) = connections
                    .iter()
                    .find(|(n, _)| n == server)
                    .ok_or_else(|| anyhow::anyhow!("MCP server '{}' not connected", server))?;

                let result = conn.send_request(method, params).await?;
                Ok(result)
            }
            "disconnect" => {
                let server = input
                    .get("server")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'server'"))?;

                let mut connections = self.connections.lock().await;
                if let Some(pos) = connections.iter().position(|(n, _)| n == server) {
                    let (_, conn) = connections.remove(pos);
                    conn.close().await?;
                    Ok(json!({ "status": "disconnected", "name": server }))
                } else {
                    Err(anyhow::anyhow!("MCP server '{}' not connected", server))
                }
            }
            _ => Err(anyhow::anyhow!("Unknown MCP action: '{}'", action)),
        }
    }
}
