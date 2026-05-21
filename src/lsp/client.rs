//! LSP client implementation

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::lsp::types::{Location, Position};
use crate::Result;

/// LSP client for a single language server.
pub struct LspClient {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    workspace_root: PathBuf,
    next_id: AtomicU64,
}

impl LspClient {
    /// Spawn and initialize a new language server process.
    pub async fn spawn(command: &str, workspace_root: &Path) -> Result<Self> {
        let mut process = Command::new(command)
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to capture LSP stdin"))?;
        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to capture LSP stdout"))?;

        let mut client = Self {
            process,
            stdin,
            stdout: BufReader::new(stdout),
            workspace_root: workspace_root.to_path_buf(),
            next_id: AtomicU64::new(1),
        };

        client.initialize().await?;
        Ok(client)
    }

    /// Find definition locations for a symbol at a file position.
    pub async fn definition(&mut self, path: &Path, position: Position) -> Result<Vec<Location>> {
        let result = self
            .request(
                "textDocument/definition",
                json!({
                    "textDocument": { "uri": file_uri(path)? },
                    "position": position,
                }),
            )
            .await?;
        parse_locations(result)
    }

    /// Find references for a symbol at a file position.
    pub async fn references(&mut self, path: &Path, position: Position) -> Result<Vec<Location>> {
        let result = self
            .request(
                "textDocument/references",
                json!({
                    "textDocument": { "uri": file_uri(path)? },
                    "position": position,
                    "context": { "includeDeclaration": true },
                }),
            )
            .await?;
        parse_locations(result)
    }

    /// Get hover text for a symbol at a file position.
    pub async fn hover(&mut self, path: &Path, position: Position) -> Result<Option<String>> {
        let result = self
            .request(
                "textDocument/hover",
                json!({
                    "textDocument": { "uri": file_uri(path)? },
                    "position": position,
                }),
            )
            .await?;
        Ok(parse_hover(result))
    }

    /// Open or update a document in the language server.
    pub async fn did_open(&mut self, path: &Path, language_id: &str, text: &str) -> Result<()> {
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": file_uri(path)?,
                    "languageId": language_id,
                    "version": 1,
                    "text": text,
                }
            }),
        )
        .await
    }

    /// Shutdown the language server.
    pub async fn shutdown(mut self) -> Result<()> {
        let _ = self.request("shutdown", Value::Null).await;
        let _ = self.notify("exit", Value::Null).await;
        self.process.kill().await?;
        Ok(())
    }

    async fn initialize(&mut self) -> Result<()> {
        let root_uri = file_uri(&self.workspace_root)?;
        self.request(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": {},
            }),
        )
        .await?;
        self.notify("initialized", json!({})).await
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.send(&message).await?;

        loop {
            let response = self.receive().await?;
            if response.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = response.get("error") {
                anyhow::bail!("LSP request {} failed: {}", method, error);
            }
            return Ok(response.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        let message = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.send(&message).await
    }

    async fn send(&mut self, message: &Value) -> Result<()> {
        let body = serde_json::to_string(message)?;
        self.stdin
            .write_all(format!("Content-Length: {}\r\n\r\n{}", body.len(), body).as_bytes())
            .await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Value> {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).await?;
            let line = line.trim();
            if line.is_empty() {
                break;
            }
            if let Some(length) = line.strip_prefix("Content-Length:") {
                content_length = Some(length.trim().parse::<usize>()?);
            }
        }

        let length = content_length.ok_or_else(|| anyhow::anyhow!("missing LSP Content-Length"))?;
        let mut buffer = vec![0; length];
        self.stdout.read_exact(&mut buffer).await?;
        Ok(serde_json::from_slice(&buffer)?)
    }
}

fn parse_locations(result: Value) -> Result<Vec<Location>> {
    if result.is_null() {
        return Ok(Vec::new());
    }
    if result.is_array() {
        Ok(serde_json::from_value(result)?)
    } else {
        Ok(vec![serde_json::from_value(result)?])
    }
}

fn parse_hover(result: Value) -> Option<String> {
    let contents = result.get("contents")?;
    if let Some(value) = contents.as_str() {
        return Some(value.to_string());
    }
    if let Some(value) = contents.get("value").and_then(Value::as_str) {
        return Some(value.to_string());
    }
    if let Some(array) = contents.as_array() {
        let values: Vec<&str> = array
            .iter()
            .filter_map(|item| {
                item.as_str()
                    .or_else(|| item.get("value").and_then(Value::as_str))
            })
            .collect();
        if !values.is_empty() {
            return Some(values.join("\n"));
        }
    }
    None
}

fn file_uri(path: &Path) -> Result<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(format!("file://{}", absolute.canonicalize()?.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_location_response() {
        let location = json!({
            "uri": "file:///tmp/main.rs",
            "range": {
                "start": { "line": 1, "character": 2 },
                "end": { "line": 1, "character": 5 }
            }
        });
        let parsed = parse_locations(location).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].range.start.line, 1);
    }

    #[test]
    fn parses_hover_markup_response() {
        let hover = json!({ "contents": { "kind": "markdown", "value": "fn main()" } });
        assert_eq!(parse_hover(hover).as_deref(), Some("fn main()"));
    }
}
