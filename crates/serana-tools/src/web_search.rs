use async_trait::async_trait;
use serde_json::{json, Value};

use serana_core::{Result, Tool};

pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn description(&self) -> &'static str {
        "Search the web. Input: {\"query\": \"search terms\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' field"))?;

        let api_key = std::env::var("BRAVE_API_KEY").map_err(|_| {
            anyhow::anyhow!("BRAVE_API_KEY environment variable not set. Please set it to use web search.")
        })?;

        let client = reqwest::Client::new();
        let resp = client
            .get("https://api.search.brave.com/res/v1/web/search")
            .header("Accept", "application/json")
            .header("X-Subscription-Token", &api_key)
            .query(&[("q", query), ("count", "10")])
            .send()
            .await?;

        let body: Value = resp.json().await?;

        let results: Vec<Value> = body
            .get("web")
            .and_then(|w| w.get("results"))
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .take(10)
                    .map(|r| {
                        json!({
                            "title": r.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                            "url": r.get("url").and_then(|v| v.as_str()).unwrap_or(""),
                            "description": r.get("description").and_then(|v| v.as_str()).unwrap_or("")
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(json!(results))
    }
}

pub struct UrlFetchTool;

#[async_trait]
impl Tool for UrlFetchTool {
    fn name(&self) -> &'static str {
        "url_fetch"
    }

    fn description(&self) -> &'static str {
        "Fetch a URL and return its content as text. Input: {\"url\": \"https://...\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' field"))?;

        let client = reqwest::Client::new();
        let resp = client.get(url).send().await?;

        let mut text = resp.text().await?;

        // Truncate to 50KB
        const MAX_BYTES: usize = 50 * 1024;
        if text.len() > MAX_BYTES {
            text.truncate(MAX_BYTES);
        }

        Ok(json!({
            "url": url,
            "content": text
        }))
    }
}
