//! Eval tool: persistent Python/JS kernels.
//!
//! Writes a wrapper script to a temp file, spawns the interpreter,
//! and communicates via stdin/sentinel markers for persistent state.

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use serana_core::{Result, Tool};

const TIMEOUT_SECS: u64 = 30;

struct Kernel {
    child: Child,
    writer: tokio::process::ChildStdin,
}

impl Kernel {
    async fn spawn(lang: &str) -> Result<Self> {
        let (cmd, args, wrapper) = match lang {
            "py" => {
                let wrapper = r#"import sys, io, traceback
while True:
    try:
        line = input()
    except EOFError:
        break
    code_lines = [line]
    while True:
        try:
            compile('\n'.join(code_lines), '<eval>', 'exec')
            break
        except SyntaxError as e:
            if 'unexpected EOF' in str(e) or 'expected an indented block' in str(e):
                try:
                    code_lines.append(input())
                except EOFError:
                    break
            else:
                break
    code = '\n'.join(code_lines)
    old_stdout, old_stderr = sys.stdout, sys.stderr
    sys.stdout, sys.stderr = io.StringIO(), io.StringIO()
    try:
        try:
            result = eval(code)
            if result is not None:
                print(repr(result))
        except SyntaxError:
            exec(code)
        out = sys.stdout.getvalue()
        err = sys.stderr.getvalue()
        sys.stdout, sys.stderr = old_stdout, old_stderr
        if out:
            sys.stdout.write(out)
        if err:
            sys.stderr.write(err)
        sys.stdout.write('__EVAL_OK__\n')
    except Exception:
        sys.stdout, sys.stderr = old_stdout, old_stderr
        traceback.print_exc()
        sys.stdout.write('__EVAL_ERR__\n')
    sys.stdout.flush()
"#;
                let path = format!("/tmp/serana_eval_kernel_{}.py", std::process::id());
                std::fs::write(&path, wrapper)?;
                ("python3", vec![path.clone()], path)
            }
            "js" => {
                let wrapper = r#"const vm = require('vm');
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, terminal: false });
let codeLines = [];
rl.on('line', (line) => {
    codeLines.push(line);
    const code = codeLines.join('\n');
    try { new Function(code); } catch (e) {
        if (e instanceof SyntaxError && !e.message.includes('Unexpected end of input')) {
            codeLines = [];
            process.stderr.write(e.message + '\n');
            process.stdout.write('__EVAL_ERR__\n');
            process.stdout.flush();
            return;
        }
        return;
    }
    codeLines = [];
    try {
        const result = vm.runInThisContext(code, { timeout: 30000 });
        if (result !== undefined) {
            process.stdout.write(typeof result === 'string' ? result : JSON.stringify(result, null, 2) + '\n');
        }
        process.stdout.write('__EVAL_OK__\n');
    } catch (e) {
        process.stderr.write((e.stack || e.message || String(e)) + '\n');
        process.stdout.write('__EVAL_ERR__\n');
    }
    process.stdout.flush();
});
rl.on('close', () => process.exit(0));
"#;
                let path = format!("/tmp/serana_eval_kernel_{}.js", std::process::id());
                std::fs::write(&path, wrapper)?;
                ("node", vec![path.clone()], path)
            }
            other => anyhow::bail!("Unsupported language: '{}'. Use \"py\" or \"js\"", other),
        };

        let mut child = Command::new(cmd)
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Clean up temp file after spawn
        let _ = std::fs::remove_file(&wrapper);

        let writer = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("No stdin"))?;

        // Give the wrapper time to initialize
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Ok(Self { child, writer })
    }

    async fn execute_code(&mut self, code: &str) -> Result<Value> {
        let sentinel_ok = "__EVAL_OK__";
        let sentinel_err = "__EVAL_ERR__";

        // Send code
        self.writer.write_all(code.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;

        // Read until sentinel
        let deadline =
            tokio::time::Instant::now() + tokio::time::Duration::from_secs(TIMEOUT_SECS);
        let stdout = self
            .child
            .stdout
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No stdout"))?;
        let stderr = self
            .child
            .stderr
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No stderr"))?;

        let mut out_buf = Vec::new();
        let mut err_buf = Vec::new();
        let mut temp = [0u8; 4096];
        let mut err_temp = [0u8; 4096];

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Ok(json!({
                    "stdout": String::from_utf8_lossy(&out_buf).to_string(),
                    "stderr": String::from_utf8_lossy(&err_buf).to_string(),
                    "timed_out": true,
                }));
            }

            tokio::select! {
                result = tokio::time::timeout(remaining, stdout.read(&mut temp)) => {
                    match result {
                        Ok(Ok(0)) => break,
                        Ok(Ok(n)) => {
                            out_buf.extend_from_slice(&temp[..n]);
                            let text = String::from_utf8_lossy(&out_buf);
                            if text.contains(sentinel_ok) {
                                let clean = text.replace(sentinel_ok, "").trim_end().to_string();
                                return Ok(json!({
                                    "stdout": clean,
                                    "stderr": String::from_utf8_lossy(&err_buf).trim().to_string(),
                                    "success": true,
                                }));
                            }
                            if text.contains(sentinel_err) {
                                let clean = text.replace(sentinel_err, "").trim_end().to_string();
                                return Ok(json!({
                                    "stdout": clean,
                                    "stderr": String::from_utf8_lossy(&err_buf).trim().to_string(),
                                    "success": false,
                                }));
                            }
                        }
                        Ok(Err(e)) => return Err(anyhow::anyhow!("Read error: {}", e)),
                        Err(_) => {
                            return Ok(json!({
                                "stdout": String::from_utf8_lossy(&out_buf).to_string(),
                                "stderr": String::from_utf8_lossy(&err_buf).to_string(),
                                "timed_out": true,
                            }));
                        }
                    }
                }
                result = stderr.read(&mut err_temp) => {
                    if let Ok(n) = result {
                        if n > 0 {
                            err_buf.extend_from_slice(&err_temp[..n]);
                        }
                    }
                }
            }
        }

        Ok(json!({
            "stdout": String::from_utf8_lossy(&out_buf).trim().to_string(),
            "stderr": String::from_utf8_lossy(&err_buf).trim().to_string(),
            "success": true,
        }))
    }
}

pub struct EvalTool {
    kernels: Mutex<Vec<(String, Kernel)>>,
}

impl EvalTool {
    pub fn new() -> Self {
        Self {
            kernels: Mutex::new(Vec::new()),
        }
    }
}

impl Default for EvalTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EvalTool {
    fn name(&self) -> &'static str {
        "eval"
    }

    fn description(&self) -> &'static str {
        "Execute code in a persistent kernel (Python/JS). State persists between calls. \
         Input: {\"language\": \"py\"|\"js\", \"code\": \"...\", \"reset\": false}"
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
                },
                "reset": {
                    "type": "boolean",
                    "description": "Reset the kernel (clear state) before executing"
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

        let reset = input
            .get("reset")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut kernels = self.kernels.lock().await;

        if reset {
            if let Some(pos) = kernels.iter().position(|(l, _)| l == language) {
                kernels.remove(pos);
            }
        }

        let kernel_idx = if let Some(idx) = kernels.iter().position(|(l, _)| l == language) {
            idx
        } else {
            let kernel = Kernel::spawn(language).await?;
            kernels.push((language.to_string(), kernel));
            kernels.len() - 1
        };

        let result = kernels[kernel_idx].1.execute_code(code).await?;

        // If the process died, remove it
        if kernels[kernel_idx]
            .1
            .child
            .try_wait()
            .ok()
            .flatten()
            .is_some()
        {
            kernels.remove(kernel_idx);
        }

        Ok(result)
    }
}
