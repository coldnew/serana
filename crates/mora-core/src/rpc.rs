use serde::{Deserialize, Serialize};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use serana::core::{Agent, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default)]
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcEvent {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

pub struct RpcServer;

impl RpcServer {
    pub async fn run<A: Agent>(agent: &A) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let request: RpcRequest = match serde_json::from_str(trimmed) {
                Ok(r) => r,
                Err(e) => {
                    let resp = RpcResponse {
                        id: "unknown".to_string(),
                        kind: "error".to_string(),
                        response: None,
                        error: Some(format!("Invalid request: {}", e)),
                        success: false,
                    };
                    Self::write_response(&mut stdout, &resp).await?;
                    continue;
                }
            };

            match request.kind.as_str() {
                "prompt" => {
                    let message = request.message.as_deref().unwrap_or("");
                    match agent.execute(message).await {
                        Ok(output) => {
                            let resp = RpcResponse {
                                id: request.id,
                                kind: "response".to_string(),
                                response: Some(output.response),
                                error: None,
                                success: output.success,
                            };
                            Self::write_response(&mut stdout, &resp).await?;
                        }
                        Err(e) => {
                            let resp = RpcResponse {
                                id: request.id,
                                kind: "response".to_string(),
                                response: None,
                                error: Some(e.to_string()),
                                success: false,
                            };
                            Self::write_response(&mut stdout, &resp).await?;
                        }
                    }
                }
                "abort" => {
                    let resp = RpcResponse {
                        id: request.id,
                        kind: "ack".to_string(),
                        response: Some("abort_requested".to_string()),
                        error: None,
                        success: true,
                    };
                    Self::write_response(&mut stdout, &resp).await?;
                }
                "set_model" => {
                    let resp = RpcResponse {
                        id: request.id,
                        kind: "ack".to_string(),
                        response: Some("model_set".to_string()),
                        error: None,
                        success: true,
                    };
                    Self::write_response(&mut stdout, &resp).await?;
                }
                "ping" => {
                    let resp = RpcResponse {
                        id: request.id,
                        kind: "pong".to_string(),
                        response: None,
                        error: None,
                        success: true,
                    };
                    Self::write_response(&mut stdout, &resp).await?;
                }
                _ => {
                    let resp = RpcResponse {
                        id: request.id,
                        kind: "error".to_string(),
                        response: None,
                        error: Some(format!("Unknown command: {}", request.kind)),
                        success: false,
                    };
                    Self::write_response(&mut stdout, &resp).await?;
                }
            }
        }

        Ok(())
    }

    async fn write_response(stdout: &mut tokio::io::Stdout, resp: &RpcResponse) -> Result<()> {
        let json = serde_json::to_string(resp)?;
        stdout.write_all(json.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rpc_request() {
        let json = r#"{"id":"r1","type":"prompt","message":"hello"}"#;
        let req: RpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.id, "r1");
        assert_eq!(req.kind, "prompt");
        assert_eq!(req.message.as_deref(), Some("hello"));
    }

    #[test]
    fn serializes_rpc_response() {
        let resp = RpcResponse {
            id: "r1".to_string(),
            kind: "response".to_string(),
            response: Some("hello".to_string()),
            error: None,
            success: true,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"id\":\"r1\""));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn parses_set_model_request() {
        let json = r#"{"id":"r2","type":"set_model","provider":"anthropic","model_id":"claude-3"}"#;
        let req: RpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.provider.as_deref(), Some("anthropic"));
        assert_eq!(req.model_id.as_deref(), Some("claude-3"));
    }
}
