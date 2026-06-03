use serde_json::Value;

use crate::core::{AgentCallbacks, ApprovalDecision, Result, ToolApproval, ToolCall, ToolCallData};
use crate::tools::ToolRegistry;

pub async fn execute_tools_concurrent(
    tool_calls: &[ToolCallData],
    registry: &ToolRegistry,
    callbacks: &AgentCallbacks,
    approval: Option<&ToolApproval>,
) -> Vec<ToolExecutionResult> {
    let mut results = Vec::with_capacity(tool_calls.len());

    for tc in tool_calls {
        let tool_name = &tc.function.name;
        let tool_args = &tc.function.arguments;

        // Check approval before executing
        if let Some(approval) = approval {
            if approval.requires_approval(tool_name) {
                let risk = ToolApproval::classify_risk(tool_name);
                // Request user approval via callback
                let approved = if let Some(ref approve_cb) = callbacks.request_approval {
                    approve_cb(tool_name, tool_args, risk)
                } else {
                    // No callback available, deny by default in smart mode
                    false
                };

                if !approved {
                    results.push(ToolExecutionResult {
                        id: tc.id.clone(),
                        name: tool_name.clone(),
                        arguments: tool_args.clone(),
                        result: Err(anyhow::anyhow!("Tool call denied by user: {}", tool_name)),
                    });
                    continue;
                }
            }
        }

        if let Some(cb) = &callbacks.tool_progress {
            cb(tool_name, tool_args, false);
        }

        let result = execute_single_tool(tool_name, tool_args, registry).await;

        if let Some(cb) = &callbacks.tool_progress {
            cb(tool_name, tool_args, true);
        }

        results.push(ToolExecutionResult {
            id: tc.id.clone(),
            name: tc.function.name.clone(),
            arguments: tc.function.arguments.clone(),
            result,
        });
    }

    results
}

async fn execute_single_tool(
    name: &str,
    arguments: &str,
    registry: &ToolRegistry,
) -> Result<Value> {
    let tool = registry
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", name))?;

    let args: Value =
        serde_json::from_str(arguments).unwrap_or(Value::Object(serde_json::Map::new()));

    tool.execute(args).await
}

#[derive(Debug)]
pub struct ToolExecutionResult {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub result: Result<Value>,
}

impl ToolExecutionResult {
    pub fn to_tool_call(&self) -> ToolCall {
        ToolCall {
            name: self.name.clone(),
            arguments: serde_json::from_str(&self.arguments).unwrap_or(Value::Null),
            result: match &self.result {
                Ok(v) => Some(v.clone()),
                Err(e) => Some(Value::String(format!("Error: {}", e))),
            },
        }
    }

    pub fn result_string(&self) -> String {
        match &self.result {
            Ok(v) => serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sequential_execution() {
        let registry = ToolRegistry::core();
        let callbacks = AgentCallbacks::new();

        let tool_calls = vec![crate::core::ToolCallData {
            id: "call_1".to_string(),
            r#type: "function".to_string(),
            function: crate::core::FunctionCall {
                name: "read_file".to_string(),
                arguments: r#"{"path": "test.txt"}"#.to_string(),
            },
        }];

        let results = execute_tools_concurrent(&tool_calls, &registry, &callbacks, None).await;
        assert_eq!(results.len(), 1);
    }
}
