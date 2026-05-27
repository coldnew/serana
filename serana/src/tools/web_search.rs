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
        "Search the web. Supports Brave (default), DuckDuckGo, and Google. Input: {\"query\": \"search terms\", \"provider\": \"brave\"}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "provider": {
                    "type": "string",
                    "enum": ["brave", "duckduckgo", "google"],
                    "description": "Search provider (default: brave, or first available)"
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

        let provider = input
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("auto");

        match provider {
            "brave" => search_brave(query).await,
            "duckduckgo" | "ddg" => search_duckduckgo(query).await,
            "google" => search_google(query).await,
            "auto" | _ => {
                // Try providers in order: Brave (if key set), DuckDuckGo, Google
                if std::env::var("BRAVE_API_KEY").is_ok() {
                    search_brave(query).await
                } else if std::env::var("GOOGLE_API_KEY").is_ok()
                    && std::env::var("GOOGLE_CX").is_ok()
                {
                    search_google(query).await
                } else {
                    search_duckduckgo(query).await
                }
            }
        }
    }
}

async fn search_brave(query: &str) -> Result<Value> {
    let api_key = std::env::var("BRAVE_API_KEY").map_err(|_| {
        anyhow::anyhow!("BRAVE_API_KEY environment variable not set.")
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

    Ok(json!({
        "provider": "brave",
        "results": results,
    }))
}

async fn search_duckduckgo(query: &str) -> Result<Value> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://html.duckduckgo.com/html/")
        .header("User-Agent", "Mozilla/5.0 (compatible; serana/1.0)")
        .query(&[("q", query)])
        .send()
        .await?;

    let html = resp.text().await?;
    let results = parse_ddg_html(&html);

    Ok(json!({
        "provider": "duckduckgo",
        "results": results,
    }))
}

fn parse_ddg_html(html: &str) -> Vec<Value> {
    let mut results = Vec::new();

    // Simple HTML parsing for DuckDuckGo results
    // Look for result blocks with class="result__a" (title/link) and class="result__snippet" (description)
    let lines: Vec<&str> = html.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        // Find result links: <a rel="nofollow" class="result__a" href="URL">TITLE</a>
        if line.contains("class=\"result__a\"") {
            if let Some(url_start) = line.find("href=\"") {
                let url_begin = url_start + 6;
                if let Some(url_end) = line[url_begin..].find('"') {
                    let url = &line[url_begin..url_begin + url_end];

                    // Extract title text between > and </a>
                    let title = if let Some(gt) = line.find('>') {
                        let after_gt = &line[gt + 1..];
                        if let Some(close) = after_gt.find("</a>") {
                            strip_html_tags(&after_gt[..close])
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    // Look for snippet in nearby lines
                    let mut description = String::new();
                    for j in (i + 1)..(i + 10).min(lines.len()) {
                        if lines[j].contains("class=\"result__snippet\"") {
                            if let Some(gt) = lines[j].find('>') {
                                let after_gt = &lines[j][gt + 1..];
                                if let Some(close) = after_gt.find("</") {
                                    description = strip_html_tags(&after_gt[..close]);
                                }
                            }
                            break;
                        }
                    }

                    if !url.starts_with("javascript:") && !title.is_empty() {
                        results.push(json!({
                            "title": title,
                            "url": url,
                            "description": description,
                        }));
                    }
                }
            }
        }
        i += 1;
        if results.len() >= 10 {
            break;
        }
    }

    results
}

fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result.trim().to_string()
}

async fn search_google(query: &str) -> Result<Value> {
    let api_key = std::env::var("GOOGLE_API_KEY").map_err(|_| {
        anyhow::anyhow!("GOOGLE_API_KEY environment variable not set.")
    })?;
    let cx = std::env::var("GOOGLE_CX").map_err(|_| {
        anyhow::anyhow!("GOOGLE_CX environment variable not set.")
    })?;

    let client = reqwest::Client::new();
    let resp = client
        .get("https://www.googleapis.com/customsearch/v1")
        .query(&[
            ("key", api_key.as_str()),
            ("cx", cx.as_str()),
            ("q", query),
            ("num", "10"),
        ])
        .send()
        .await?;

    let body: Value = resp.json().await?;

    let results: Vec<Value> = body
        .get("items")
        .and_then(|items| items.as_array())
        .map(|arr| {
            arr.iter()
                .take(10)
                .map(|r| {
                    json!({
                        "title": r.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                        "url": r.get("link").and_then(|v| v.as_str()).unwrap_or(""),
                        "description": r.get("snippet").and_then(|v| v.as_str()).unwrap_or("")
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(json!({
        "provider": "google",
        "results": results,
    }))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_tags_removes_html() {
        assert_eq!(strip_html_tags("<b>hello</b> world"), "hello world");
        assert_eq!(strip_html_tags("no tags"), "no tags");
    }

    #[test]
    fn parse_ddg_finds_results() {
        let html = r#"
<a rel="nofollow" class="result__a" href="https://example.com">Example Title</a>
<span class="result__snippet">This is a description</span>
"#;
        let results = parse_ddg_html(html);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["title"], "Example Title");
        assert_eq!(results[0]["url"], "https://example.com");
    }
}
