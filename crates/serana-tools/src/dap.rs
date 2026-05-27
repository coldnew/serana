//! DAP (Debug Adapter Protocol) debugger tool.
//!
//! Drives debug adapters over the DAP protocol via stdio.
//! Supports launch/attach, breakpoints, stepping, inspection.

use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex};

use serana_core::{Result, Tool};

/// A DAP session managing a debug adapter subprocess.
pub struct DapSession {
    child: Mutex<Child>,
    writer: Mutex<tokio::process::ChildStdin>,
    reader_rx: Mutex<mpsc::Receiver<String>>,
    _reader_handle: tokio::task::JoinHandle<()>,
    seq: std::sync::atomic::AtomicI64,
}

impl DapSession {
    /// Launch a debug adapter and start a session.
    pub async fn launch(adapter: &str, program: &str, args: &[&str]) -> Result<Self> {
        let mut child = Command::new(adapter)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to launch adapter '{}': {}", adapter, e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("No stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("No stdout"))?;

        let (tx, rx) = mpsc::channel(64);
        let reader_handle = tokio::spawn(async move {
            Self::read_messages(stdout, tx).await;
        });

        let session = Self {
            child: Mutex::new(child),
            writer: Mutex::new(stdin),
            reader_rx: Mutex::new(rx),
            _reader_handle: reader_handle,
            seq: std::sync::atomic::AtomicI64::new(1),
        };

        // Initialize the session
        session
            .send_request(
                "initialize",
                Some(json!({
                    "clientID": "serana",
                    "adapterID": adapter,
                    "pathFormat": "path",
                    "linesStartAt1": true,
                    "columnsStartAt1": true,
                })),
            )
            .await?;

        // Wait for initialized event
        session.wait_for_event("initialized", 5000).await?;

        // Launch the program
        session
            .send_request(
                "launch",
                Some(json!({
                    "program": program,
                    "stopOnEntry": false,
                })),
            )
            .await?;

        Ok(session)
    }

    async fn read_messages(stdout: tokio::process::ChildStdout, tx: mpsc::Sender<String>) {
        let mut reader = BufReader::new(stdout);
        let mut content_length = 0usize;

        loop {
            // Read headers
            let mut header_line = String::new();
            match reader.read_line(&mut header_line).await {
                Ok(0) | Err(_) => break,
                _ => {}
            }

            if header_line.trim().is_empty() {
                // Empty line signals end of headers for this message
                if content_length == 0 {
                    continue;
                }
            } else if let Some(len) = header_line.strip_prefix("Content-Length:") {
                content_length = len.trim().parse().unwrap_or(0);
                // Read the blank line
                let mut blank = String::new();
                let _ = reader.read_line(&mut blank).await;
            } else {
                continue;
            }

            if content_length == 0 {
                continue;
            }

            // Read body
            let mut buf = vec![0u8; content_length];
            if reader.read_exact(&mut buf).await.is_err() {
                break;
            }

            if let Ok(msg) = String::from_utf8(buf) {
                if tx.send(msg).await.is_err() {
                    break;
                }
            }

            content_length = 0;
        }
    }

    async fn send_request(&self, command: &str, args: Option<Value>) -> Result<i64> {
        let seq = self.seq.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let mut msg = json!({
            "seq": seq,
            "type": "request",
            "command": command,
        });
        if let Some(a) = args {
            msg["arguments"] = a;
        }

        let body = serde_json::to_string(&msg)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        let mut writer = self.writer.lock().await;
        writer.write_all(header.as_bytes()).await?;
        writer.write_all(body.as_bytes()).await?;
        writer.flush().await?;

        Ok(seq)
    }

    async fn wait_for_event(&self, event_name: &str, timeout_ms: u64) -> Result<Value> {
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
        let mut rx = self.reader_rx.lock().await;

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                anyhow::bail!("Timeout waiting for DAP event '{}'", event_name);
            }

            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Some(msg_str)) => {
                    if let Ok(msg) = serde_json::from_str::<Value>(&msg_str) {
                        if msg["type"].as_str() == Some("event")
                            && msg["event"].as_str() == Some(event_name)
                        {
                            return Ok(msg["body"].clone());
                        }
                    }
                }
                _ => anyhow::bail!("Timeout or channel closed waiting for '{}'", event_name),
            }
        }
    }

    /// Set a breakpoint at a file:line.
    pub async fn set_breakpoint(&self, file: &str, line: u64) -> Result<Value> {
        self.send_request(
            "setBreakpoints",
            Some(json!({
                "source": { "path": file },
                "breakpoints": [{ "line": line }],
            })),
        )
        .await?;
        // Read response
        self.read_response(3000).await
    }

    /// Continue execution.
    pub async fn cont(&self, thread_id: u64) -> Result<()> {
        self.send_request("continue", Some(json!({ "threadId": thread_id })))
            .await?;
        Ok(())
    }

    /// Step over.
    pub async fn step_over(&self, thread_id: u64) -> Result<()> {
        self.send_request("next", Some(json!({ "threadId": thread_id })))
            .await?;
        Ok(())
    }

    /// Step into.
    pub async fn step_in(&self, thread_id: u64) -> Result<()> {
        self.send_request("stepIn", Some(json!({ "threadId": thread_id })))
            .await?;
        Ok(())
    }

    /// Step out.
    pub async fn step_out(&self, thread_id: u64) -> Result<()> {
        self.send_request("stepOut", Some(json!({ "threadId": thread_id })))
            .await?;
        Ok(())
    }

    /// Get threads.
    pub async fn threads(&self) -> Result<Value> {
        self.send_request("threads", None).await?;
        self.read_response(3000).await
    }

    /// Get stack trace.
    pub async fn stack_trace(&self, thread_id: u64) -> Result<Value> {
        self.send_request("stackTrace", Some(json!({ "threadId": thread_id })))
            .await?;
        self.read_response(3000).await
    }

    /// Evaluate an expression.
    pub async fn evaluate(&self, expression: &str, frame_id: Option<u64>) -> Result<Value> {
        let mut args = json!({ "expression": expression, "context": "repl" });
        if let Some(fid) = frame_id {
            args["frameId"] = json!(fid);
        }
        self.send_request("evaluate", Some(args)).await?;
        self.read_response(3000).await
    }

    /// Terminate the session.
    pub async fn terminate(&self) -> Result<()> {
        self.send_request("terminate", None).await?;
        Ok(())
    }

    /// Disconnect the session.
    pub async fn disconnect(&self) -> Result<()> {
        self.send_request("disconnect", Some(json!({ "terminateDebuggee": true })))
            .await?;
        Ok(())
    }

    async fn read_response(&self, timeout_ms: u64) -> Result<Value> {
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
        let mut rx = self.reader_rx.lock().await;

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                anyhow::bail!("Timeout waiting for DAP response");
            }

            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Some(msg_str)) => {
                    if let Ok(msg) = serde_json::from_str::<Value>(&msg_str) {
                        if msg["type"].as_str() == Some("response") {
                            if msg["success"].as_bool() == Some(false) {
                                let err = msg["message"].as_str().unwrap_or("unknown error");
                                anyhow::bail!("DAP error: {}", err);
                            }
                            return Ok(msg["body"].clone());
                        }
                        // Skip events while waiting for response
                    }
                }
                _ => anyhow::bail!("Timeout or channel closed waiting for response"),
            }
        }
    }
}

/// DAP debug tool that manages a debug session.
pub struct DebugTool {
    session: Mutex<Option<DapSession>>,
}

impl DebugTool {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
        }
    }
}

impl Default for DebugTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DebugTool {
    fn name(&self) -> &'static str {
        "debug"
    }

    fn description(&self) -> &'static str {
        "Drive a DAP debug session. Input: {\"action\": \"launch\", \"adapter\": \"debugpy\", \"program\": \"main.py\"} \
         or {\"action\": \"set_breakpoint\", \"file\": \"main.py\", \"line\": 10} \
         or {\"action\": \"continue\"|\"step_over\"|\"step_in\"|\"step_out\"|\"threads\"|\"stack_trace\"|\"terminate\", \"thread_id\": 1} \
         or {\"action\": \"evaluate\", \"expression\": \"x\", \"frame_id\": 0}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["launch", "set_breakpoint", "continue", "step_over", "step_in", "step_out", "threads", "stack_trace", "evaluate", "terminate"],
                    "description": "Debug action"
                },
                "adapter": {
                    "type": "string",
                    "description": "Adapter command (e.g. debugpy, lldb-dap)"
                },
                "program": {
                    "type": "string",
                    "description": "Program to debug"
                },
                "file": {
                    "type": "string",
                    "description": "File for breakpoint"
                },
                "line": {
                    "type": "integer",
                    "description": "Line number for breakpoint"
                },
                "thread_id": {
                    "type": "integer",
                    "description": "Thread ID for stepping/continue"
                },
                "frame_id": {
                    "type": "integer",
                    "description": "Stack frame ID for evaluate"
                },
                "expression": {
                    "type": "string",
                    "description": "Expression to evaluate"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' field"))?;

        match action {
            "launch" => {
                let adapter = input
                    .get("adapter")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'adapter' for launch"))?;
                let program = input
                    .get("program")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'program' for launch"))?;

                let session = DapSession::launch(adapter, program, &[]).await?;
                let mut guard = self.session.lock().await;
                *guard = Some(session);
                Ok(json!({ "status": "launched", "adapter": adapter, "program": program }))
            }
            "set_breakpoint" => {
                let file = input
                    .get("file")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'file'"))?;
                let line = input
                    .get("line")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'line'"))?;
                let guard = self.session.lock().await;
                let session = guard
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No active debug session"))?;
                let result = session.set_breakpoint(file, line).await?;
                Ok(
                    json!({ "status": "breakpoint_set", "file": file, "line": line, "result": result }),
                )
            }
            "continue" => {
                let thread_id = input.get("thread_id").and_then(|v| v.as_u64()).unwrap_or(1);
                let guard = self.session.lock().await;
                let session = guard
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No active debug session"))?;
                session.cont(thread_id).await?;
                Ok(json!({ "status": "continued", "thread_id": thread_id }))
            }
            "step_over" => {
                let thread_id = input.get("thread_id").and_then(|v| v.as_u64()).unwrap_or(1);
                let guard = self.session.lock().await;
                let session = guard
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No active debug session"))?;
                session.step_over(thread_id).await?;
                Ok(json!({ "status": "stepped_over", "thread_id": thread_id }))
            }
            "step_in" => {
                let thread_id = input.get("thread_id").and_then(|v| v.as_u64()).unwrap_or(1);
                let guard = self.session.lock().await;
                let session = guard
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No active debug session"))?;
                session.step_in(thread_id).await?;
                Ok(json!({ "status": "stepped_in", "thread_id": thread_id }))
            }
            "step_out" => {
                let thread_id = input.get("thread_id").and_then(|v| v.as_u64()).unwrap_or(1);
                let guard = self.session.lock().await;
                let session = guard
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No active debug session"))?;
                session.step_out(thread_id).await?;
                Ok(json!({ "status": "stepped_out", "thread_id": thread_id }))
            }
            "threads" => {
                let guard = self.session.lock().await;
                let session = guard
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No active debug session"))?;
                let result = session.threads().await?;
                Ok(json!({ "threads": result }))
            }
            "stack_trace" => {
                let thread_id = input.get("thread_id").and_then(|v| v.as_u64()).unwrap_or(1);
                let guard = self.session.lock().await;
                let session = guard
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No active debug session"))?;
                let result = session.stack_trace(thread_id).await?;
                Ok(json!({ "stack_trace": result }))
            }
            "evaluate" => {
                let expression = input
                    .get("expression")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'expression'"))?;
                let frame_id = input.get("frame_id").and_then(|v| v.as_u64());
                let guard = self.session.lock().await;
                let session = guard
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No active debug session"))?;
                let result = session.evaluate(expression, frame_id).await?;
                Ok(json!({ "result": result }))
            }
            "terminate" => {
                let mut guard = self.session.lock().await;
                if let Some(session) = guard.as_ref() {
                    session.terminate().await?;
                    session.disconnect().await?;
                }
                *guard = None;
                Ok(json!({ "status": "terminated" }))
            }
            _ => Err(anyhow::anyhow!("Unknown debug action: '{}'", action)),
        }
    }
}
