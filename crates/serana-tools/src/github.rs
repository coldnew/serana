use async_trait::async_trait;
use serde_json::{json, Value};

use serana_core::{Result, Tool};

fn github_token() -> Result<String> {
    std::env::var("GITHUB_TOKEN").map_err(|_| anyhow::anyhow!("GITHUB_TOKEN env var not set"))
}

fn github_client() -> Result<reqwest::Client> {
    let token = github_token()?;
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("serana"),
    );
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|e| anyhow::anyhow!("Invalid GITHUB_TOKEN: {e}"))?,
    );
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {e}"))
}

fn parse_repo(input: &Value) -> Result<(&str, &str)> {
    let repo = input
        .get("repo")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'repo' field"))?;
    let (owner, name) = repo
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("Invalid repo format, expected 'owner/repo'"))?;
    if owner.is_empty() || name.is_empty() {
        return Err(anyhow::anyhow!("Invalid repo format, expected 'owner/repo'"));
    }
    Ok((owner, name))
}

fn parse_number(input: &Value) -> Result<u64> {
    input
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'number' field"))
}

fn extract_user(user: &Value) -> Value {
    json!({
        "login": user.get("login").and_then(|v| v.as_str()).unwrap_or(""),
        "id": user.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
    })
}

pub struct GitHubPrViewTool;
pub struct GitHubIssueViewTool;
pub struct GitHubPrDiffTool;

#[async_trait]
impl Tool for GitHubPrViewTool {
    fn name(&self) -> &'static str {
        "github_pr_view"
    }

    fn description(&self) -> &'static str {
        "View a GitHub PR. Input: {\"repo\": \"owner/repo\", \"number\": 123}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repo": {
                    "type": "string",
                    "description": "Repository in owner/repo format"
                },
                "number": {
                    "type": "integer",
                    "description": "PR number"
                }
            },
            "required": ["repo", "number"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let (owner, name) = parse_repo(&input)?;
        let number = parse_number(&input)?;

        let client = github_client()?;
        let url = format!("https://api.github.com/repos/{owner}/{name}/pulls/{number}");
        let resp = client.get(&url).send().await?;
        let status = resp.status();
        let body: Value = resp.json().await?;

        if !status.is_success() {
            let msg = body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(anyhow::anyhow!("GitHub API error ({status}): {msg}"));
        }

        Ok(json!({
            "number": number,
            "title": body.get("title").and_then(|v| v.as_str()).unwrap_or(""),
            "body": body.get("body").and_then(|v| v.as_str()).unwrap_or(""),
            "state": body.get("state").and_then(|v| v.as_str()).unwrap_or(""),
            "user": extract_user(body.get("user").unwrap_or(&Value::Null)),
            "html_url": body.get("html_url").and_then(|v| v.as_str()).unwrap_or(""),
            "created_at": body.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
            "updated_at": body.get("updated_at").and_then(|v| v.as_str()).unwrap_or(""),
            "merged": body.get("merged").and_then(|v| v.as_bool()).unwrap_or(false),
            "mergeable": body.get("mergeable"),
            "additions": body.get("additions").and_then(|v| v.as_u64()).unwrap_or(0),
            "deletions": body.get("deletions").and_then(|v| v.as_u64()).unwrap_or(0),
            "changed_files": body.get("changed_files").and_then(|v| v.as_u64()).unwrap_or(0),
            "base": body.get("base").and_then(|b| b.get("ref")).and_then(|v| v.as_str()).unwrap_or(""),
            "head": body.get("head").and_then(|b| b.get("ref")).and_then(|v| v.as_str()).unwrap_or(""),
        }))
    }
}

#[async_trait]
impl Tool for GitHubIssueViewTool {
    fn name(&self) -> &'static str {
        "github_issue_view"
    }

    fn description(&self) -> &'static str {
        "View a GitHub issue. Input: {\"repo\": \"owner/repo\", \"number\": 123}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repo": {
                    "type": "string",
                    "description": "Repository in owner/repo format"
                },
                "number": {
                    "type": "integer",
                    "description": "Issue number"
                }
            },
            "required": ["repo", "number"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let (owner, name) = parse_repo(&input)?;
        let number = parse_number(&input)?;

        let client = github_client()?;
        let url = format!("https://api.github.com/repos/{owner}/{name}/issues/{number}");
        let resp = client.get(&url).send().await?;
        let status = resp.status();
        let body: Value = resp.json().await?;

        if !status.is_success() {
            let msg = body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(anyhow::anyhow!("GitHub API error ({status}): {msg}"));
        }

        let labels: Vec<String> = body
            .get("labels")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|l| l.get("name").and_then(|v| v.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(json!({
            "number": number,
            "title": body.get("title").and_then(|v| v.as_str()).unwrap_or(""),
            "body": body.get("body").and_then(|v| v.as_str()).unwrap_or(""),
            "state": body.get("state").and_then(|v| v.as_str()).unwrap_or(""),
            "user": extract_user(body.get("user").unwrap_or(&Value::Null)),
            "html_url": body.get("html_url").and_then(|v| v.as_str()).unwrap_or(""),
            "created_at": body.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
            "updated_at": body.get("updated_at").and_then(|v| v.as_str()).unwrap_or(""),
            "labels": labels,
            "assignee": body.get("assignee").map(extract_user),
            "comments": body.get("comments").and_then(|v| v.as_u64()).unwrap_or(0),
        }))
    }
}

#[async_trait]
impl Tool for GitHubPrDiffTool {
    fn name(&self) -> &'static str {
        "github_pr_diff"
    }

    fn description(&self) -> &'static str {
        "View the diff of a GitHub PR. Input: {\"repo\": \"owner/repo\", \"number\": 123}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repo": {
                    "type": "string",
                    "description": "Repository in owner/repo format"
                },
                "number": {
                    "type": "integer",
                    "description": "PR number"
                }
            },
            "required": ["repo", "number"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let (owner, name) = parse_repo(&input)?;
        let number = parse_number(&input)?;

        let client = github_client()?;
        let url = format!("https://api.github.com/repos/{owner}/{name}/pulls/{number}");
        let resp = client
            .get(&url)
            .header(reqwest::header::ACCEPT, "application/vnd.github.v3.diff")
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body: Value = resp.json().await.unwrap_or(Value::Null);
            let msg = body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(anyhow::anyhow!("GitHub API error ({status}): {msg}"));
        }

        let diff = resp.text().await?;

        Ok(json!({
            "repo": format!("{owner}/{name}"),
            "number": number,
            "diff": diff,
        }))
    }
}
