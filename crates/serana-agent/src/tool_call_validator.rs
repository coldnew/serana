use std::collections::HashSet;

use serana_core::{Result, ToolCallData, ToolDefinition};

pub struct ToolCallValidator<'a> {
    tool_names: HashSet<&'a str>,
}

impl<'a> ToolCallValidator<'a> {
    pub fn new(tools: &'a [ToolDefinition]) -> Self {
        Self {
            tool_names: tools
                .iter()
                .map(|tool| tool.function.name.as_str())
                .collect(),
        }
    }

    pub fn validate(&self, tool_calls: &[ToolCallData]) -> Result<()> {
        let mut ids = HashSet::new();

        for tool_call in tool_calls {
            if tool_call.id.trim().is_empty() {
                anyhow::bail!("Tool call id cannot be empty");
            }
            if !ids.insert(tool_call.id.as_str()) {
                anyhow::bail!("Duplicate tool call id: {}", tool_call.id);
            }
            if !self.tool_names.contains(tool_call.function.name.as_str()) {
                anyhow::bail!(
                    "Unknown tool requested by model: {}",
                    tool_call.function.name
                );
            }
            serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments).map_err(
                |err| {
                    anyhow::anyhow!(
                        "Invalid JSON arguments for tool {}: {}",
                        tool_call.function.name,
                        err
                    )
                },
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serana_core::{FunctionCall, FunctionDefinition, ToolCallData};

    fn tool_definition(name: &str) -> ToolDefinition {
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: name.to_string(),
                description: String::new(),
                parameters: serde_json::json!({}),
            },
        }
    }

    fn tool_call(id: &str, name: &str, arguments: &str) -> ToolCallData {
        ToolCallData {
            id: id.to_string(),
            r#type: "function".to_string(),
            function: FunctionCall {
                name: name.to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    #[test]
    fn accepts_known_tool_with_json_arguments() {
        let tools = [tool_definition("read_file")];
        let validator = ToolCallValidator::new(&tools);

        validator
            .validate(&[tool_call("call_1", "read_file", r#"{"path":"Cargo.toml"}"#)])
            .unwrap();
    }

    #[test]
    fn rejects_unknown_tools() {
        let tools = [tool_definition("read_file")];
        let validator = ToolCallValidator::new(&tools);

        let err = validator
            .validate(&[tool_call("call_1", "unknown", "{}")])
            .unwrap_err();

        assert!(err.to_string().contains("Unknown tool"));
    }

    #[test]
    fn rejects_invalid_json_arguments() {
        let tools = [tool_definition("read_file")];
        let validator = ToolCallValidator::new(&tools);

        let err = validator
            .validate(&[tool_call("call_1", "read_file", "{bad json")])
            .unwrap_err();

        assert!(err.to_string().contains("Invalid JSON arguments"));
    }

    #[test]
    fn rejects_duplicate_ids() {
        let tools = [tool_definition("read_file")];
        let validator = ToolCallValidator::new(&tools);

        let err = validator
            .validate(&[
                tool_call("call_1", "read_file", "{}"),
                tool_call("call_1", "read_file", "{}"),
            ])
            .unwrap_err();

        assert!(err.to_string().contains("Duplicate tool call id"));
    }
}
