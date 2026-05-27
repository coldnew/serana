//! LSP (Language Server Protocol) client implementation
//!
//! Provides built-in LSP support for code intelligence features like
//! go-to-definition, find-references, hover, rename, and diagnostics.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::types::{Location, Position};
use serana_core::Result;

/// Document version tracking for didChange notifications.
#[derive(Debug, Clone)]
struct DocumentState {
    version: i32,
    content: String,
}

/// Language identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageId {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
}

impl LanguageId {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            "ts" | "tsx" => Some(Self::TypeScript),
            "js" | "jsx" => Some(Self::JavaScript),
            "py" => Some(Self::Python),
            "go" => Some(Self::Go),
            _ => None,
        }
    }

    pub fn lsp_language_id(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Python => "python",
            Self::Go => "go",
        }
    }

    pub fn server_command(&self) -> &'static str {
        match self {
            Self::Rust => "rust-analyzer",
            Self::TypeScript | Self::JavaScript => "typescript-language-server",
            Self::Python => "pylsp",
            Self::Go => "gopls",
        }
    }
}

/// LSP manager that handles multiple language servers
pub struct LspManager {
    workspace_root: PathBuf,
    servers: HashMap<LanguageId, LspClient>,
}

impl LspManager {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            servers: HashMap::new(),
        }
    }

    pub fn detect_language_servers(&self) -> Vec<LanguageId> {
        let mut languages = Vec::new();

        if self.workspace_root.join("Cargo.toml").exists() {
            languages.push(LanguageId::Rust);
        }

        if self.workspace_root.join("package.json").exists() {
            languages.push(LanguageId::TypeScript);
        }

        if self.workspace_root.join("go.mod").exists() {
            languages.push(LanguageId::Go);
        }

        if self.workspace_root.join("requirements.txt").exists()
            || self.workspace_root.join("pyproject.toml").exists()
        {
            languages.push(LanguageId::Python);
        }

        languages
    }

    pub async fn definition(&mut self, path: &Path, position: Position) -> Result<Vec<Location>> {
        let lang = language_for_path(path)?;
        self.ensure_server(lang).await?;
        self.servers
            .get_mut(&lang)
            .ok_or_else(|| anyhow::anyhow!("LSP server not available for {:?}", lang))?
            .definition(path, position)
            .await
    }

    pub async fn references(&mut self, path: &Path, position: Position) -> Result<Vec<Location>> {
        let lang = language_for_path(path)?;
        self.ensure_server(lang).await?;
        self.servers
            .get_mut(&lang)
            .ok_or_else(|| anyhow::anyhow!("LSP server not available for {:?}", lang))?
            .references(path, position)
            .await
    }

    pub async fn hover(&mut self, path: &Path, position: Position) -> Result<Option<String>> {
        let lang = language_for_path(path)?;
        self.ensure_server(lang).await?;
        self.servers
            .get_mut(&lang)
            .ok_or_else(|| anyhow::anyhow!("LSP server not available for {:?}", lang))?
            .hover(path, position)
            .await
    }

    /// Notify the language server that a file's content has changed.
    pub async fn change_file(&mut self, path: &Path, new_text: &str) -> Result<()> {
        let lang = language_for_path(path)?;
        self.ensure_server(lang).await?;
        self.servers
            .get_mut(&lang)
            .ok_or_else(|| anyhow::anyhow!("LSP server not available for {:?}", lang))?
            .did_change(path, new_text)
            .await
    }

    async fn ensure_server(&mut self, lang: LanguageId) -> Result<()> {
        if !self.servers.contains_key(&lang) {
            self.start_server(lang).await?;
        }
        Ok(())
    }

    pub async fn start_server(&mut self, lang: LanguageId) -> Result<()> {
        let command = lang.server_command();
        let client = LspClient::spawn(command, &self.workspace_root).await?;
        self.servers.insert(lang, client);
        Ok(())
    }

    pub fn get_client(&self, lang: LanguageId) -> Option<&LspClient> {
        self.servers.get(&lang)
    }

    pub fn get_client_mut(&mut self, lang: LanguageId) -> Option<&mut LspClient> {
        self.servers.get_mut(&lang)
    }

    pub async fn shutdown_all(&mut self) -> Result<()> {
        for (_lang, client) in self.servers.drain() {
            client.shutdown().await?;
        }
        Ok(())
    }
}

fn language_for_path(path: &Path) -> Result<LanguageId> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| anyhow::anyhow!("missing file extension for {}", path.display()))?;
    LanguageId::from_extension(ext)
        .ok_or_else(|| anyhow::anyhow!("unsupported source language: {}", path.display()))
}

/// LSP client for a single language server.
pub struct LspClient {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    workspace_root: PathBuf,
    next_id: AtomicU64,
    /// Track open documents and their versions.
    documents: HashMap<String, DocumentState>,
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
            documents: HashMap::new(),
        };

        client.initialize().await?;
        Ok(client)
    }

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

    pub async fn did_open(&mut self, path: &Path, language_id: &str, text: &str) -> Result<()> {
        let uri = file_uri(path)?;
        self.documents.insert(
            uri.clone(),
            DocumentState {
                version: 1,
                content: text.to_string(),
            },
        );
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": text,
                }
            }),
        )
        .await
    }

    /// Notify the language server that a document's content has changed.
    /// Uses full document sync (sends the entire text on each change).
    pub async fn did_change(&mut self, path: &Path, new_text: &str) -> Result<()> {
        let uri = file_uri(path)?;
        let version = if let Some(state) = self.documents.get_mut(&uri) {
            state.version += 1;
            state.content = new_text.to_string();
            state.version
        } else {
            // Auto-open if not yet tracked
            self.documents.insert(
                uri.clone(),
                DocumentState {
                    version: 1,
                    content: new_text.to_string(),
                },
            );
            1
        };

        self.notify(
            "textDocument/didChange",
            json!({
                "textDocument": {
                    "uri": uri,
                    "version": version,
                },
                "contentChanges": [{
                    "text": new_text,
                }]
            }),
        )
        .await
    }

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
    fn test_language_from_extension() {
        assert_eq!(LanguageId::from_extension("rs"), Some(LanguageId::Rust));
        assert_eq!(
            LanguageId::from_extension("ts"),
            Some(LanguageId::TypeScript)
        );
        assert_eq!(LanguageId::from_extension("py"), Some(LanguageId::Python));
        assert_eq!(LanguageId::from_extension("go"), Some(LanguageId::Go));
        assert_eq!(LanguageId::from_extension("txt"), None);
    }

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
