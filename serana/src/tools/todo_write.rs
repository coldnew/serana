use async_trait::async_trait;
use serde_json::{json, Value};

use serana_core::{Result, Tool};

pub struct TodoWriteTool;

#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &'static str {
        "todo_write"
    }

    fn description(&self) -> &'static str {
        "Create or update a structured task list. Input: {\"todos\": [{\"content\": \"Task description\", \"status\": \"pending\", \"priority\": \"high\"}]}"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "description": "Array of todo items",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": {"type": "string", "description": "Task description"},
                            "status": {"type": "string", "enum": ["pending", "in_progress", "completed", "cancelled"], "description": "Task status"},
                            "priority": {"type": "string", "enum": ["high", "medium", "low"], "description": "Task priority"}
                        },
                        "required": ["content", "status", "priority"]
                    }
                }
            },
            "required": ["todos"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let todos = input
            .get("todos")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing 'todos' field"))?;

        let mut output = String::new();
        for todo in todos {
            let content = todo.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let status = todo.get("status").and_then(|v| v.as_str()).unwrap_or("pending");
            let priority = todo.get("priority").and_then(|v| v.as_str()).unwrap_or("medium");

            let checkbox = match status {
                "completed" => "[x]",
                "in_progress" => "[~]",
                "cancelled" => "[-]",
                _ => "[ ]",
            };
            let prio = match priority {
                "high" => "!",
                "low" => "-",
                _ => " ",
            };
            output.push_str(&format!("{} {} {}\n", checkbox, prio, content));
        }

        Ok(json!({
            "todos": todos,
            "formatted": output.trim(),
            "count": todos.len(),
            "completed": todos.iter().filter(|t| t.get("status").and_then(|v| v.as_str()) == Some("completed")).count(),
            "pending": todos.iter().filter(|t| t.get("status").and_then(|v| v.as_str()) == Some("pending")).count(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn formats_todo_list() {
        let input = json!({
            "todos": [
                {"content": "Fix bug", "status": "completed", "priority": "high"},
                {"content": "Add tests", "status": "pending", "priority": "medium"},
            ]
        });
        let result = TodoWriteTool.execute(input).await.unwrap();
        assert_eq!(result["count"], 2);
        assert_eq!(result["completed"], 1);
        assert_eq!(result["pending"], 1);
    }
}
