//! Concurrent tool execution with callback support.
//!
//! Executes multiple tool calls in parallel while respecting dependencies.

use serde_json::Value;

use crate::agent::{AgentCallbacks, ToolCall};
use crate::llm::ToolCallData;
use crate::tools::ToolRegistry;
use crate::Result;

/// Execute multiple tool calls concurrently.
pub async fn execute_tools_concurrent(
    tool_calls: &[ToolCallData],
    registry: &ToolRegistry,
    callbacks: &AgentCallbacks,
) -> Vec<ToolExecutionResult> {
    let mut results = Vec::with_capacity(tool_calls.len());

    for tc in tool_calls {
        let tool_name = &tc.function.name;
        let tool_args = &tc.function.arguments;

        // Notify start
        if let Some(cb) = &callbacks.tool_progress {
            cb(tool_name, tool_args, false);
        }

        let result = execute_single_tool(tool_name, tool_args, registry).await;

        // Notify completion
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

/// Execute a single tool call.
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

/// Result of a tool execution.
#[derive(Debug)]
pub struct ToolExecutionResult {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub result: Result<Value>,
}

impl ToolExecutionResult {
    /// Convert to agent ToolCall record.
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

    /// Get result as string for message history.
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
    use crate::tools::ToolRegistry;

    #[tokio::test]
    async fn test_sequential_execution() {
        let registry = ToolRegistry::new();
        let callbacks = AgentCallbacks::new();

        let tool_calls = vec![crate::llm::ToolCallData {
            id: "call_1".to_string(),
            r#type: "function".to_string(),
            function: crate::llm::FunctionCall {
                name: "read_file".to_string(),
                arguments: r#"{"path": "test.txt"}"#.to_string(),
            },
        }];

        let results = execute_tools_concurrent(&tool_calls, &registry, &callbacks).await;
        assert_eq!(results.len(), 1);
    }
}
