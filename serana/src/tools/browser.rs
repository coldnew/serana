//! Browser tool via headless Chromium and CDP.
//!
//! Spawns headless Chromium, connects via CDP WebSocket,
//! supports navigation, JS evaluation, screenshots.

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::core::{Result, Tool};

static CDP_PORT: u16 = 9222;
static BROWSER_STATE: Mutex<Option<BrowserState>> = Mutex::const_new(None);

struct BrowserState {
    ws_url: String,
}

async fn get_page_ws_url(port: u16) -> Result<String> {
    let client = reqwest::Client::new();
    let targets_resp = client
        .get(format!("http://127.0.0.1:{}/json", port))
        .send()
        .await?;
    let targets: Vec<Value> = targets_resp.json().await?;
    let page = targets
        .iter()
        .find(|t| t["type"].as_str() == Some("page"))
        .ok_or_else(|| anyhow::anyhow!("No page target found"))?;
    page["webSocketDebuggerUrl"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No page WebSocket URL"))
}

async fn cdp_evaluate(ws_url: &str, expression: &str) -> Result<Value> {
    let (mut ws, _) = connect_async(ws_url).await?;

    let cmd = json!({
        "id": 1,
        "method": "Runtime.evaluate",
        "params": {
            "expression": expression,
            "returnByValue": true,
            "awaitPromise": true,
        }
    });
    ws.send(Message::Text(cmd.to_string())).await?;

    // Wait for response with matching id
    loop {
        match ws.next().await {
            Some(Ok(Message::Text(text))) => {
                let msg: Value = serde_json::from_str(&text)?;
                if msg["id"].as_i64() == Some(1) {
                    if let Some(err) = msg.get("error") {
                        return Ok(json!({
                            "error": err["message"].as_str().unwrap_or("CDP error"),
                        }));
                    }
                    let result = &msg["result"]["result"];
                    if result["type"].as_str() == Some("undefined") {
                        return Ok(json!({ "type": "undefined" }));
                    }
                    return Ok(result.clone());
                }
            }
            Some(Ok(Message::Close(_))) | None => break,
            Some(Err(e)) => return Err(anyhow::anyhow!("WebSocket error: {}", e)),
            _ => continue,
        }
    }
    Err(anyhow::anyhow!("WebSocket closed before response"))
}

async fn cdp_screenshot(ws_url: &str) -> Result<Value> {
    let (mut ws, _) = connect_async(ws_url).await?;

    let cmd = json!({
        "id": 1,
        "method": "Page.captureScreenshot",
        "params": { "format": "png" }
    });
    ws.send(Message::Text(cmd.to_string())).await?;

    loop {
        match ws.next().await {
            Some(Ok(Message::Text(text))) => {
                let msg: Value = serde_json::from_str(&text)?;
                if msg["id"].as_i64() == Some(1) {
                    if let Some(err) = msg.get("error") {
                        return Ok(json!({
                            "error": err["message"].as_str().unwrap_or("CDP error"),
                        }));
                    }
                    let data = msg["result"]["data"].as_str().unwrap_or("");
                    return Ok(json!({
                        "format": "png",
                        "data": data,
                        "size_bytes": data.len(),
                    }));
                }
            }
            Some(Ok(Message::Close(_))) | None => break,
            Some(Err(e)) => return Err(anyhow::anyhow!("WebSocket error: {}", e)),
            _ => continue,
        }
    }
    Err(anyhow::anyhow!("WebSocket closed before response"))
}

/// Browser tool for driving headless Chromium.
pub struct BrowserTool;

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &'static str {
        "browser"
    }

    fn description(&self) -> &'static str {
        "Drive a headless browser via CDP. Actions: open (spawn + navigate), run (evaluate JS), \
         screenshot (capture page as base64 PNG), extract (fetch + strip HTML), close (kill browser)."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["open", "close", "run", "screenshot", "extract"],
                    "description": "Browser action"
                },
                "url": {
                    "type": "string",
                    "description": "URL to navigate to (for open)"
                },
                "code": {
                    "type": "string",
                    "description": "JavaScript to evaluate (for run)"
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
            "open" => {
                let url = input
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'url' for open"))?;

                let output = Command::new("chromium")
                    .args([
                        "--headless=new",
                        "--disable-gpu",
                        &format!("--remote-debugging-port={}", CDP_PORT),
                        "--no-sandbox",
                        "--disable-dev-shm-usage",
                        url,
                    ])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();

                match output {
                    Ok(_child) => {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        let ws_url = get_page_ws_url(CDP_PORT).await?;
                        let mut state = BROWSER_STATE.lock().await;
                        *state = Some(BrowserState {
                            ws_url: ws_url.clone(),
                        });
                        Ok(json!({ "status": "opened", "url": url, "port": CDP_PORT }))
                    }
                    Err(e) => Err(anyhow::anyhow!(
                        "Failed to start browser (chromium not found?): {}",
                        e
                    )),
                }
            }
            "run" => {
                let code = input
                    .get("code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'code' for run"))?;

                let state = BROWSER_STATE.lock().await;
                let ws_url = match &*state {
                    Some(s) => s.ws_url.clone(),
                    None => {
                        // Try auto-detecting a running browser
                        match get_page_ws_url(CDP_PORT).await {
                            Ok(url) => url,
                            Err(_) => {
                                return Ok(json!({
                                    "error": "No browser session. Use action=open first."
                                }));
                            }
                        }
                    }
                };
                drop(state);

                let result = cdp_evaluate(&ws_url, code).await?;
                Ok(result)
            }
            "screenshot" => {
                let state = BROWSER_STATE.lock().await;
                let ws_url = match &*state {
                    Some(s) => s.ws_url.clone(),
                    None => get_page_ws_url(CDP_PORT).await?,
                };
                drop(state);

                cdp_screenshot(&ws_url).await
            }
            "close" => {
                let mut state = BROWSER_STATE.lock().await;
                *state = None;
                let _ = Command::new("pkill")
                    .args(["-f", "remote-debugging-port=9222"])
                    .output()
                    .await;
                Ok(json!({ "status": "closed" }))
            }
            "extract" => {
                let url = input
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'url' for extract"))?;

                let client = reqwest::Client::new();
                let resp = client.get(url).send().await?;
                let html = resp.text().await?;

                let text: String = html
                    .lines()
                    .map(|line| {
                        let mut in_tag = false;
                        let mut result = String::new();
                        for ch in line.chars() {
                            match ch {
                                '<' => in_tag = true,
                                '>' => in_tag = false,
                                _ if !in_tag => result.push(ch),
                                _ => {}
                            }
                        }
                        result
                    })
                    .filter(|line| !line.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");

                let truncated = if text.len() > 50_000 {
                    &text[..50_000]
                } else {
                    &text
                };

                Ok(json!({
                    "url": url,
                    "content": truncated,
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown browser action: '{}'", action)),
        }
    }
}
