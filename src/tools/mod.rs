use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;
use crate::Result;

pub mod fs;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn execute(&self, input: Value) -> Result<Value>;
}

pub struct ToolRegistry {
    tools: HashMap<&'static str, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|b| &**b)
    }

    pub fn describe_all(&self) -> String {
        self.tools
            .values()
            .map(|t| format!("- {}: {}", t.name(), t.description()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
