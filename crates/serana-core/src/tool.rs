use async_trait::async_trait;
use serde_json::Value;

use crate::Result;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    /// JSON Schema for tool parameters (optional, defaults to empty object)
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }
    async fn execute(&self, input: Value) -> Result<Value>;
}
