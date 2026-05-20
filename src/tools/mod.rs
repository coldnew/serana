use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::Result;

pub mod fs;
pub mod hashline;

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
        let mut registry = Self {
            tools: HashMap::new(),
        };
        registry.register(Box::new(fs::ReadFileTool));
        registry.register(Box::new(fs::WriteFileTool));
        registry.register(Box::new(fs::EditFileTool));
        registry
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn list(&self) -> Vec<&'static str> {
        self.tools.keys().copied().collect()
    }

    pub fn describe_all(&self) -> String {
        let mut descriptions: Vec<&str> = self.tools.values().map(|t| t.description()).collect();
        descriptions.sort();
        descriptions.join("\n")
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
