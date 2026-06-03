use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};

use super::message::Message;
use super::Result;

/// Tool definition for LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub r#type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// LLM client trait
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Send a chat completion request
    async fn chat(&self, messages: &[Message]) -> Result<String>;

    /// Send a chat completion request with tool support
    async fn chat_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message>;

    /// Send a streaming chat completion request (yields chunks as they arrive)
    fn chat_stream<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> std::pin::Pin<Box<dyn Stream<Item = Result<String>> + Send + 'a>> {
        Box::pin(futures::stream::once(
            async move { self.chat(messages).await },
        ))
    }

    /// Send a streaming chat completion request with tool support
    fn chat_with_tools_stream<'a>(
        &'a self,
        messages: &'a [Message],
        tools: &'a [ToolDefinition],
    ) -> std::pin::Pin<Box<dyn Stream<Item = Result<Message>> + Send + 'a>> {
        Box::pin(futures::stream::once(async move {
            self.chat_with_tools(messages, tools).await
        }))
    }
}
