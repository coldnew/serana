use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

/// Minimal LSP client supporting the core features needed by mora.
///
/// Implements JSON-RPC 2.0 over stdio for communicating with
/// language servers (clangd, rust-analyzer, pyright, etc.).
///
/// Supported requests:
/// - initialize / initialized
/// - textDocument/definition
/// - textDocument/references
/// - textDocument/hover
/// - textDocument/completion
/// - textDocument/didOpen / didSave / didChange
/// - shutdown / exit
pub struct LspClient {
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    request_id: u64,
    /// Server capabilities from initialize response
    pub server_name: String,
    pub root_uri: String,
    initialized: bool,
    open_documents: HashMap<String, String>,
}

/// LSP position (0-indexed line, 0-indexed character).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

/// LSP range (start..end).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP location (uri + range).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LspLocation {
    pub uri: String,
    pub range: LspRange,
}

/// LSP completion item.
#[derive(Debug, Clone)]
pub struct LspCompletionItem {
    pub label: String,
    pub kind: Option<u32>,
    pub detail: Option<String>,
    pub documentation: Option<String>,
}

/// LSP hover result.
#[derive(Debug, Clone)]
pub struct LspHoverResult {
    pub contents: String,
    pub range: Option<LspRange>,
}

impl LspClient {
    pub fn new() -> Self {
        Self {
            process: None,
            stdin: None,
            request_id: 0,
            server_name: String::new(),
            root_uri: String::new(),
            initialized: false,
            open_documents: HashMap::new(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.process.is_some() && self.initialized
    }

    /// Start a language server process.
    pub fn start(&mut self, command: &str, args: &[&str], root_uri: &str) -> Result<(), String> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("Failed to start LSP server: {e}"))?;
        let stdin = child.stdin.take().ok_or("No stdin")?;
        self.process = Some(child);
        self.stdin = Some(stdin);
        self.root_uri = root_uri.to_string();

        // Send initialize request
        let init_params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "completion": { "completionItem": { "snippetSupport": false } },
                    "hover": { "contentFormat": ["plaintext"] },
                    "definition": {},
                    "references": {},
                    "publishDiagnostics": {}
                }
            }
        });
        let response = self.send_request("initialize", init_params)?;

        if let Some(name) = response.get("result").and_then(|r| r.get("serverInfo")).and_then(|s| s.get("name")) {
            self.server_name = name.as_str().unwrap_or("unknown").to_string();
        }

        // Send initialized notification
        self.send_notification("initialized", serde_json::json!({}))?;
        self.initialized = true;

        Ok(())
    }

    /// Notify the server that a document was opened.
    pub fn did_open(&mut self, uri: &str, language_id: &str, version: i32, text: &str) -> Result<(), String> {
        self.open_documents.insert(uri.to_string(), text.to_string());
        self.send_notification("textDocument/didOpen", serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": version,
                "text": text
            }
        }))
    }

    /// Notify the server that a document was saved.
    pub fn did_save(&mut self, uri: &str) -> Result<(), String> {
        self.send_notification("textDocument/didSave", serde_json::json!({
            "textDocument": { "uri": uri }
        }))
    }

    /// Request go-to-definition.
    pub fn definition(&mut self, uri: &str, line: u32, character: u32) -> Result<Vec<LspLocation>, String> {
        let response = self.send_request("textDocument/definition", serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }))?;
        self.parse_locations(&response)
    }

    /// Request find-references.
    pub fn references(&mut self, uri: &str, line: u32, character: u32) -> Result<Vec<LspLocation>, String> {
        let response = self.send_request("textDocument/references", serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": true }
        }))?;
        self.parse_locations(&response)
    }

    /// Request hover documentation.
    pub fn hover(&mut self, uri: &str, line: u32, character: u32) -> Result<Option<LspHoverResult>, String> {
        let response = self.send_request("textDocument/hover", serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }))?;

        let result = match response.get("result") {
            Some(r) if !r.is_null() => r,
            _ => return Ok(None),
        };

        let contents = result.get("contents")
            .and_then(|c| {
                if let Some(s) = c.as_str() {
                    Some(s.to_string())
                } else if let Some(arr) = c.as_array() {
                    Some(arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n"))
                } else if let Some(s) = c.get("value").and_then(|v| v.as_str()) {
                    Some(s.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let range = result.get("range").and_then(|r| serde_json::from_value(r.clone()).ok());

        Ok(Some(LspHoverResult { contents, range }))
    }

    /// Request completion.
    pub fn completion(&mut self, uri: &str, line: u32, character: u32) -> Result<Vec<LspCompletionItem>, String> {
        let response = self.send_request("textDocument/completion", serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }))?;

        let result = match response.get("result") {
            Some(r) if !r.is_null() => r,
            _ => return Ok(vec![]),
        };

        let items = if let Some(arr) = result.get("items").and_then(|v| v.as_array()) {
            arr
        } else if let Some(arr) = result.as_array() {
            arr
        } else {
            return Ok(vec![]);
        };

        let completions: Vec<LspCompletionItem> = items.iter().map(|item| {
            LspCompletionItem {
                label: item.get("label").and_then(|l| l.as_str()).unwrap_or("").to_string(),
                kind: item.get("kind").and_then(|k| k.as_u64()).map(|k| k as u32),
                detail: item.get("detail").and_then(|d| d.as_str()).map(|s| s.to_string()),
                documentation: item.get("documentation").and_then(|d| d.as_str()).map(|s| s.to_string()),
            }
        }).collect();

        Ok(completions)
    }

    /// Shutdown the language server.
    pub fn shutdown(&mut self) {
        if self.initialized {
            let _ = self.send_request("shutdown", serde_json::json!(null));
            let _ = self.send_notification("exit", serde_json::json!(null));
            self.initialized = false;
        }
        if let Some(mut proc) = self.process.take() {
            let _ = proc.kill();
            let _ = proc.wait();
        }
        self.stdin = None;
    }

    // ── JSON-RPC 2.0 protocol ──

    fn send_request(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
        self.request_id += 1;
        let id = self.request_id;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        self.write_message(&request)?;

        // Read response - read from stdout directly
        if let Some(ref mut proc) = self.process {
            if let Some(ref mut stdout) = proc.stdout {
                return Self::read_response_impl(stdout, id);
            }
        }

        Err("No stdout available".to_string())
    }

    fn send_notification(&mut self, method: &str, params: serde_json::Value) -> Result<(), String> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.write_message(&notification)
    }

    fn write_message(&mut self, msg: &serde_json::Value) -> Result<(), String> {
        let body = serde_json::to_string(msg).map_err(|e| e.to_string())?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        if let Some(ref mut stdin) = self.stdin {
            stdin.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
            stdin.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
            stdin.flush().map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("No stdin".to_string())
        }
    }

    fn read_response_impl<R: Read>(reader: R, expected_id: u64) -> Result<serde_json::Value, String> {
        let mut buf_reader = BufReader::new(reader);

        loop {
            // Read headers
            let mut content_length = 0usize;
            let mut header_line = String::new();

            loop {
                header_line.clear();
                match buf_reader.read_line(&mut header_line) {
                    Ok(0) => return Err("Server closed connection".to_string()),
                    Ok(_) => {}
                    Err(e) => return Err(format!("Read error: {e}")),
                }

                let trimmed = header_line.trim();
                if trimmed.is_empty() {
                    break;
                }

                if let Some(len_str) = trimmed.strip_prefix("Content-Length: ") {
                    content_length = len_str.trim().parse().unwrap_or(0);
                }
            }

            if content_length == 0 {
                return Err("No Content-Length header".to_string());
            }

            // Read body
            let mut body = vec![0u8; content_length];
            buf_reader.read_exact(&mut body).map_err(|e| e.to_string())?;

            let response: serde_json::Value = serde_json::from_slice(&body)
                .map_err(|e| format!("JSON parse error: {e}"))?;

            // Check if this is a response (has id) or notification (no id)
            if let Some(id) = response.get("id").and_then(|v| v.as_u64()) {
                if id == expected_id {
                    return Ok(response);
                }
            }
            // Skip notifications (like textDocument/publishDiagnostics)
        }
    }

    fn parse_locations(&self, response: &serde_json::Value) -> Result<Vec<LspLocation>, String> {
        let result = match response.get("result") {
            Some(r) if !r.is_null() => r,
            _ => return Ok(vec![]),
        };

        let locations: Vec<LspLocation> = if let Some(arr) = result.as_array() {
            arr.iter().filter_map(|loc| serde_json::from_value(loc.clone()).ok()).collect()
        } else if result.get("uri").is_some() {
            // Single location
            serde_json::from_value(result.clone()).ok().into_iter().collect()
        } else {
            vec![]
        };

        Ok(locations)
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Detect the language server command for a given file extension.
pub fn detect_language_server(ext: &str) -> Option<(&str, Vec<&str>, &str)> {
    match ext {
        "rs" => Some(("rust-analyzer", vec![], "rust")),
        "py" => Some(("pyright-langserver", vec!["--stdio"], "python")),
        "ts" | "tsx" | "js" | "jsx" => Some(("typescript-language-server", vec!["--stdio"], "typescript")),
        "go" => Some(("gopls", vec![], "go")),
        "c" | "h" | "cpp" | "hpp" => Some(("clangd", vec![], "cpp")),
        "lua" => Some(("lua-language-server", vec![], "lua")),
        "rb" => Some(("solargraph", vec!["stdio"], "ruby")),
        _ => None,
    }
}
