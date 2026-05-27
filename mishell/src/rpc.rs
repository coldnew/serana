use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run_rpc_loop() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = io::BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    while let Ok(msg) = read_message(&mut reader) {
        if let Some(response) = handle_message(&msg) {
            let _ = send_message(&mut writer, &response);
        }
    }
}

fn read_message(reader: &mut impl BufRead) -> io::Result<String> {
    // Read Content-Length header
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        if let Some(val) = line.strip_prefix("Content-Length: ") {
            content_length = val.trim().parse().ok();
        }
    }

    let len = content_length
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"))?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn send_message(writer: &mut impl Write, msg: &str) -> io::Result<()> {
    write!(writer, "Content-Length: {}\r\n\r\n{}", msg.len(), msg)?;
    writer.flush()
}

fn handle_message(msg: &str) -> Option<String> {
    let v: Value = match serde_json::from_str(msg) {
        Ok(v) => v,
        Err(_) => return Some(error_response(Value::Null, -32700, "Parse error")),
    };

    let method = v.get("method")?.as_str()?;
    let id = v.get("id").cloned().unwrap_or(Value::Null);
    let params = v.get("params").cloned().unwrap_or(json!({}));

    match method {
        "initialize" => Some(initialize_response(&id, &params)),
        "notifications/initialized" => None,
        "tools/list" => Some(tools_list_response(&id)),
        "tools/call" => Some(tools_call_response(&id, &params)),
        _ => Some(error_response(
            id,
            -32601,
            &format!("Unknown method: {}", method),
        )),
    }
}

fn initialize_response(id: &Value, params: &Value) -> String {
    let client_version = params
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("2024-11-05");

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": client_version,
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "mishell",
                "version": VERSION
            }
        }
    })
    .to_string()
}

fn tools_list_response(id: &Value) -> String {
    let tools = json!([
        {
            "name": "bash",
            "description": "Execute a shell command via mishell. Returns stdout and stderr.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command to execute" },
                    "timeout": { "type": "number", "description": "Timeout in seconds (optional)" }
                },
                "required": ["command"]
            },
            "annotations": {
                "title": "Bash",
                "readOnlyHint": false,
                "destructiveHint": true
            }
        },
        {
            "name": "read",
            "description": "Read the contents of a file. Use offset/limit for large text files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the file to read" },
                    "offset": { "type": "number", "description": "Line number to start from (1-indexed)" },
                    "limit": { "type": "number", "description": "Maximum lines to read" }
                },
                "required": ["path"]
            }
        },
        {
            "name": "edit",
            "description": "Search-and-replace in a file. Matches exactly once.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "old_text": { "type": "string", "description": "Text to find" },
                    "new_text": { "type": "string", "description": "Replacement text" }
                },
                "required": ["path", "old_text", "new_text"]
            }
        },
        {
            "name": "file",
            "description": "Detect file type via magic bytes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" }
                },
                "required": ["path"]
            }
        },
        {
            "name": "head",
            "description": "Read first N lines of a file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "lines": { "type": "number", "description": "Number of lines (default 10)" }
                },
                "required": ["path"]
            }
        },
        {
            "name": "tail",
            "description": "Read last N lines of a file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "lines": { "type": "number", "description": "Number of lines (default 10)" }
                },
                "required": ["path"]
            }
        }
    ]);

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": { "tools": tools }
    })
    .to_string()
}

fn tools_call_response(id: &Value, params: &Value) -> String {
    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => return error_response(id.clone(), -32602, "Missing tool name"),
    };

    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = match name {
        "bash" => tool_bash(&args),
        "read" => tool_read(&args),
        "edit" => tool_edit(&args),
        "file" => tool_file(&args),
        "head" => tool_head(&args),
        "tail" => tool_tail(&args),
        _ => return error_response(id.clone(), -32602, &format!("Unknown tool: {}", name)),
    };

    match result {
        Ok(content) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{ "type": "text", "text": content }],
                "isError": false
            }
        })
        .to_string(),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{ "type": "text", "text": e }],
                "isError": true
            }
        })
        .to_string(),
    }
}

fn tool_bash(args: &Value) -> Result<String, String> {
    let command = args
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or("missing command")?;

    let _timeout = args.get("timeout").and_then(|v| v.as_u64());

    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(command);

    let start = std::time::Instant::now();
    let output = cmd
        .output()
        .map_err(|e| format!("failed to execute: {}", e))?;
    let elapsed = start.elapsed().as_millis() as u64;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    let result = json!({
        "exit_code": exit_code,
        "stdout": stdout,
        "stderr": stderr,
        "duration_ms": elapsed
    });

    Ok(serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()))
}

fn tool_read(args: &Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("missing path")?;
    let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(2000) as usize;

    let content =
        std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path, e))?;
    let total_lines = content.lines().count();
    let lines: Vec<&str> = content.lines().collect();

    let start = offset.saturating_sub(1);
    let end = (start + limit).min(lines.len());
    let slice = if start < lines.len() {
        &lines[start..end]
    } else {
        &[]
    };

    let result = json!({
        "content": slice.join("\n"),
        "encoding": "text",
        "line_count": slice.len(),
        "truncated": end < lines.len(),
        "total_lines": total_lines
    });

    Ok(serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()))
}

fn tool_edit(args: &Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("missing path")?;
    let old_text = args
        .get("old_text")
        .and_then(|v| v.as_str())
        .ok_or("missing old_text")?;
    let new_text = args
        .get("new_text")
        .and_then(|v| v.as_str())
        .ok_or("missing new_text")?;

    let content =
        std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path, e))?;

    // Try exact match
    if let Some(pos) = content.find(old_text) {
        // Check uniqueness
        if content[pos + old_text.len()..].find(old_text).is_some() {
            return Err(format!("old_text is not unique in {}", path));
        }

        let mut new_content = String::with_capacity(content.len() + new_text.len());
        new_content.push_str(&content[..pos]);
        new_content.push_str(new_text);
        new_content.push_str(&content[pos + old_text.len()..]);

        std::fs::write(path, &new_content).map_err(|e| format!("cannot write {}: {}", path, e))?;

        let first_line = content[..pos].lines().count() + 1;
        let diff = format!("-{}\n+{}", old_text, new_text);

        let result = json!({
            "success": true,
            "first_changed_line": first_line,
            "diff": diff
        });

        return Ok(serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()));
    }

    // Fuzzy match: strip trailing whitespace
    let strip = |s: &str| -> String {
        s.lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    };

    let stripped_content = strip(&content);
    let stripped_old = strip(old_text);

    if let Some(pos) = stripped_content.find(&stripped_old) {
        if stripped_content[pos + stripped_old.len()..]
            .find(&stripped_old)
            .is_some()
        {
            return Err(format!("old_text is not unique in {}", path));
        }

        // Map position back to original
        let mut orig_pos = 0;
        let mut stripped_pos = 0;
        let content_bytes = content.as_bytes();
        let stripped_content_bytes = stripped_content.as_bytes();

        while stripped_pos < pos && orig_pos < content.len() {
            if stripped_pos < stripped_content.len()
                && stripped_content_bytes[stripped_pos] == content_bytes[orig_pos]
            {
                stripped_pos += 1;
                orig_pos += 1;
            } else {
                while orig_pos < content.len()
                    && (content_bytes[orig_pos] == b' ' || content_bytes[orig_pos] == b'\t')
                {
                    orig_pos += 1;
                }
            }
        }

        let mut match_end_orig = orig_pos;
        let mut match_end_stripped = pos + stripped_old.len();
        while match_end_stripped < stripped_content.len() && match_end_orig < content.len() {
            if stripped_content_bytes[match_end_stripped] == content_bytes[match_end_orig] {
                match_end_stripped += 1;
                match_end_orig += 1;
            } else {
                while match_end_orig < content.len()
                    && (content_bytes[match_end_orig] == b' '
                        || content_bytes[match_end_orig] == b'\t')
                {
                    match_end_orig += 1;
                }
            }
        }

        let mut new_content = String::with_capacity(content.len() + new_text.len());
        new_content.push_str(&content[..orig_pos]);
        new_content.push_str(new_text);
        new_content.push_str(&content[match_end_orig..]);

        std::fs::write(path, &new_content).map_err(|e| format!("cannot write {}: {}", path, e))?;

        let first_line = content[..orig_pos].lines().count() + 1;
        let diff = format!("-{}\n+{}", &content[orig_pos..match_end_orig], new_text);

        let result = json!({
            "success": true,
            "first_changed_line": first_line,
            "diff": diff,
            "fuzzy_matched": true
        });

        return Ok(serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()));
    }

    Err(format!("old_text not found in {}", path))
}

fn tool_file(args: &Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("missing path")?;
    let p = std::path::Path::new(path);

    if !p.exists() {
        return Err(format!("{}: not found", path));
    }

    if p.is_dir() {
        return Ok(json!({ "path": path, "type": "directory" }).to_string());
    }

    let mut buf = [0u8; 512];
    let (file_type, size) = match std::fs::File::open(p) {
        Ok(mut f) => {
            use std::io::Read;
            let size = f.metadata().map(|m| m.len()).unwrap_or(0);
            let n = f.read(&mut buf).unwrap_or(0);
            (crate::shell::detect_file_type(&buf[..n], path), size)
        }
        Err(e) => return Err(format!("{}: cannot read: {}", path, e)),
    };

    let result = json!({
        "path": path,
        "type": file_type.unwrap_or_else(|| "unknown".to_string()),
        "size": size
    });

    Ok(serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()))
}

fn tool_head(args: &Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("missing path")?;
    let n = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    let content = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))?;
    let lines: Vec<&str> = content.lines().take(n).collect();

    let result = json!({
        "content": lines.join("\n"),
        "line_count": lines.len()
    });

    Ok(serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()))
}

fn tool_tail(args: &Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("missing path")?;
    let n = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    let content = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))?;
    let all_lines: Vec<&str> = content.lines().collect();
    let start = if all_lines.len() > n {
        all_lines.len() - n
    } else {
        0
    };
    let lines = &all_lines[start..];

    let result = json!({
        "content": lines.join("\n"),
        "line_count": lines.len()
    });

    Ok(serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()))
}

fn error_response(id: Value, code: i32, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
    .to_string()
}
