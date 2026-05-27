//! MCP (Model Context Protocol) client for external tool servers.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex};

use serana_core::{Result, Tool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

type PendingMap = HashMap<i64, oneshot::Sender<Value>>;

pub struct McpConnection {
    child: Mutex<Child>,
    writer: Mutex<tokio::process::ChildStdin>,
    pending: Mutex<PendingMap>,
    notifications: Mutex<Vec<Value>>,
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
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("No stdout"))?;

        let pending: PendingMap = HashMap::new();
        let pending_ref = std::sync::Arc::new(tokio::sync::Mutex::new(pending));
        let notifications = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));

        // Spawn reader task
        let pending_clone = pending_ref.clone();
        let notif_clone = notifications.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_jsonrpc_message(&mut reader).await {
                    Ok(msg) => {
                        if let Some(id) = msg.get("id").and_then(|v| v.as_i64()) {
                            // Response to a request
                            let mut p = pending_clone.lock().await;
                            if let Some(tx) = p.remove(&id) {
                                let _ = tx.send(msg);
                            }
                        } else {
                            // Server notification
                            let mut n = notif_clone.lock().await;
                            n.push(msg);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            child: Mutex::new(child),
            writer: Mutex::new(stdin),
            pending: Mutex::new(HashMap::new()),
            notifications: Mutex::new(Vec::new()),
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

        // Register pending response handler
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(seq, tx);
        }

        // Send request
        {
            let mut writer = self.writer.lock().await;
            writer.write_all(header.as_bytes()).await?;
            writer.write_all(body.as_bytes()).await?;
            writer.flush().await?;
        }

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => {
                // Channel closed — remove from pending
                self.pending.lock().await.remove(&seq);
                Err(anyhow::anyhow!("Response channel closed"))
            }
            Err(_) => {
                // Timeout — remove from pending
                self.pending.lock().await.remove(&seq);
                Err(anyhow::anyhow!("MCP request timed out (30s)"))
            }
        }
    }

    pub async fn take_notifications(&self) -> Vec<Value> {
        let mut n = self.notifications.lock().await;
        std::mem::take(&mut *n)
    }

    pub async fn close(&self) -> Result<()> {
        let mut child = self.child.lock().await;
        child.kill().await.ok();
        Ok(())
    }
}

async fn read_jsonrpc_message(
    reader: &mut BufReader<tokio::process::ChildStdout>,
) -> Result<Value> {
    // Read headers
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        if let Some(val) = line.strip_prefix("Content-Length: ") {
            content_length = val.parse().ok();
        }
    }

    let len = content_length.ok_or_else(|| anyhow::anyhow!("Missing Content-Length header"))?;
    let mut buf = vec![0u8; len];
    tokio::io::AsyncReadExt::read_exact(reader, &mut buf).await?;
    let msg: Value = serde_json::from_slice(&buf)?;
    Ok(msg)
}

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
        "Connect to and call MCP servers. Actions: connect, list, call, disconnect, notifications."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["connect", "list", "call", "disconnect", "notifications"],
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
                    "description": "Server name (for call/disconnect/notifications)"
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
                    env: HashMap::new(),
                };

                let conn = McpConnection::connect(&config).await?;

                // Send initialize handshake
                let init_result = conn
                    .send_request(
                        "initialize",
                        Some(json!({
                            "protocolVersion": "2024-11-05",
                            "capabilities": {},
                            "clientInfo": { "name": "serana", "version": "0.1.0" }
                        })),
                    )
                    .await?;

                let mut connections = self.connections.lock().await;
                connections.push((name.to_string(), conn));

                Ok(json!({
                    "status": "connected",
                    "name": name,
                    "server_info": init_result.get("result").and_then(|r| r.get("serverInfo")),
                }))
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
            "notifications" => {
                let server = input
                    .get("server")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'server'"))?;

                let connections = self.connections.lock().await;
                let (_, conn) = connections
                    .iter()
                    .find(|(n, _)| n == server)
                    .ok_or_else(|| anyhow::anyhow!("MCP server '{}' not connected", server))?;

                let notifs = conn.take_notifications().await;
                Ok(json!({ "notifications": notifs }))
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
