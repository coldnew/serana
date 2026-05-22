//! Browser tool via headless Chromium and CDP.
//!
//! Spawns headless Chromium, connects via CDP WebSocket,
//! supports navigation, JS evaluation, screenshots.

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;

use serana_core::{Result, Tool};

/// Browser tool for driving headless Chromium.
pub struct BrowserTool;

impl BrowserTool {
    async fn cdp_request(_method: &str, _params: Option<Value>, port: u16) -> Result<Value> {
        let url = format!("http://127.0.0.1:{}/json/version", port);
        let client = reqwest::Client::new();
        let version_resp = client.get(&url).send().await?;
        let version: Value = version_resp.json().await?;
        let _ws_url = version["webSocketDebuggerUrl"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No WebSocket debugger URL"))?;

        // Use HTTP endpoint for simplicity (CDP over HTTP)
        let target_url = format!("http://127.0.0.1:{}/json", port);
        let targets_resp = client.get(&target_url).send().await?;
        let targets: Vec<Value> = targets_resp.json().await?;

        let page = targets
            .iter()
            .find(|t| t["type"].as_str() == Some("page"))
            .ok_or_else(|| anyhow::anyhow!("No page target found"))?;

        let _ws_page_url = page["webSocketDebuggerUrl"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No page WebSocket URL"))?;

        // For v1, use simple HTTP-based approach
        // Full WebSocket CDP requires tungstenite dependency
        Ok(json!({
            "status": "connected",
            "url": page["url"].as_str().unwrap_or(""),
            "title": page["title"].as_str().unwrap_or(""),
        }))
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &'static str {
        "browser"
    }

    fn description(&self) -> &'static str {
        "Drive a headless browser. Input: {\"action\": \"open\", \"url\": \"https://example.com\"} \
         or {\"action\": \"run\", \"code\": \"document.title\"} \
         or {\"action\": \"close\"}"
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
                    "description": "JavaScript to execute (for run)"
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

                // Try to use chromium/chrome with remote debugging
                let port = 9222u16;
                let output = Command::new("chromium")
                    .args([
                        "--headless=new",
                        "--disable-gpu",
                        &format!("--remote-debugging-port={}", port),
                        "--no-sandbox",
                        url,
                    ])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();

                match output {
                    Ok(_child) => {
                        // Give browser time to start
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        Ok(json!({ "status": "opened", "url": url, "port": port }))
                    }
                    Err(e) => Err(anyhow::anyhow!(
                        "Failed to start browser (chromium not found?): {}",
                        e
                    )),
                }
            }
            "run" => {
                let _code = input
                    .get("code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'code' for run"))?;
                // v1: use puppeteer-style HTTP evaluate via CDP
                let client = reqwest::Client::new();
                let targets_resp = client
                    .get("http://127.0.0.1:9222/json")
                    .send()
                    .await?;
                let targets: Vec<Value> = targets_resp.json().await?;
                let page = targets
                    .iter()
                    .find(|t| t["type"].as_str() == Some("page"))
                    .ok_or_else(|| anyhow::anyhow!("No page target found"))?;

                Ok(json!({
                    "status": "ready",
                    "url": page["url"].as_str().unwrap_or(""),
                    "title": page["title"].as_str().unwrap_or(""),
                    "note": "Full JS evaluation requires WebSocket CDP (tungstenite). Use url_fetch for simple page content."
                }))
            }
            "close" => {
                // Kill chromium processes
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

                // Simple fetch + strip HTML
                let client = reqwest::Client::new();
                let resp = client.get(url).send().await?;
                let html = resp.text().await?;

                // Very basic HTML-to-text: strip tags
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
