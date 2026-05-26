use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use serana_core::{Agent, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AcpError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

pub struct AcpServer;

impl AcpServer {
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

            let request: AcpRequest = match serde_json::from_str(trimmed) {
                Ok(r) => r,
                Err(e) => {
                    let resp = AcpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(AcpError {
                            code: -32700,
                            message: format!("Parse error: {}", e),
                            data: None,
                        }),
                    };
                    Self::write_response(&mut stdout, &resp).await?;
                    continue;
                }
            };

            if request.id.is_none() {
                continue;
            }

            match request.method.as_str() {
                "initialize" => {
                    let resp = AcpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(json!({
                            "serverInfo": {
                                "name": "serana",
                                "version": "0.1.0"
                            },
                            "capabilities": {
                                "tools": true,
                                "fs": { "readTextFile": true, "writeTextFile": true },
                                "terminal": { "create": true, "output": true }
                            }
                        })),
                        error: None,
                    };
                    Self::write_response(&mut stdout, &resp).await?;
                }
                "session/prompt" => {
                    let message = request.params
                        .as_ref()
                        .and_then(|p| p.get("message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    match agent.execute(message).await {
                        Ok(output) => {
                            let resp = AcpResponse {
                                jsonrpc: "2.0".to_string(),
                                id: request.id,
                                result: Some(json!({
                                    "content": output.response,
                                    "success": output.success
                                })),
                                error: None,
                            };
                            Self::write_response(&mut stdout, &resp).await?;
                        }
                        Err(e) => {
                            let resp = AcpResponse {
                                jsonrpc: "2.0".to_string(),
                                id: request.id,
                                result: None,
                                error: Some(AcpError {
                                    code: -32000,
                                    message: e.to_string(),
                                    data: None,
                                }),
                            };
                            Self::write_response(&mut stdout, &resp).await?;
                        }
                    }
                }
                "session/request_permission" => {
                    let resp = AcpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(json!({ "approved": true })),
                        error: None,
                    };
                    Self::write_response(&mut stdout, &resp).await?;
                }
                _ => {
                    let resp = AcpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(AcpError {
                            code: -32601,
                            message: format!("Method not found: {}", request.method),
                            data: None,
                        }),
                    };
                    Self::write_response(&mut stdout, &resp).await?;
                }
            }
        }

        Ok(())
    }

    async fn write_response(stdout: &mut tokio::io::Stdout, resp: &AcpResponse) -> Result<()> {
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
    fn parses_acp_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let req: AcpRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "initialize");
        assert_eq!(req.id, Some(json!(1)));
    }

    #[test]
    fn serializes_acp_response() {
        let resp = AcpResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            result: Some(json!({"capabilities": {}})),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("capabilities"));
    }

    #[test]
    fn acp_error_response() {
        let resp = AcpResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            result: None,
            error: Some(AcpError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            }),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Method not found"));
    }
}
