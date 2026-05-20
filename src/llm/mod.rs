use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::Result;

pub mod openai;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn system(content: String) -> Self {
        Self { role: "system".to_string(), content }
    }
    pub fn user(content: String) -> Self {
        Self { role: "user".to_string(), content }
    }
    pub fn assistant(content: String) -> Self {
        Self { role: "assistant".to_string(), content }
    }
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, messages: &[Message]) -> Result<String>;
}
