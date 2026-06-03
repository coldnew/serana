use crate::core::{AgentCallbacks, LlmClient, Message, Result, ToolDefinition};
use futures::StreamExt;

use super::stream_rules::{ContextMode, StreamRuleEngine, StreamRuleMatch};

/// Outcome from a turn that may have been interrupted by TTSR.
pub enum TurnOutcome {
    /// Normal completion.
    Complete(Message),
    /// Stream was interrupted by a TTSR rule. Contains injection text and context mode.
    Interrupted {
        name: String,
        injection: String,
        context: ContextMode,
        partial_content: String,
    },
}

pub struct TurnRunner<'a> {
    llm: &'a dyn LlmClient,
    callbacks: &'a AgentCallbacks,
}

impl<'a> TurnRunner<'a> {
    pub fn new(llm: &'a dyn LlmClient, callbacks: &'a AgentCallbacks) -> Self {
        Self { llm, callbacks }
    }

    pub async fn run(&self, messages: &[Message], tools: &[ToolDefinition]) -> Result<Message> {
        let outcome = self.run_with_ttsr(messages, tools, None).await?;
        match outcome {
            TurnOutcome::Complete(msg) => Ok(msg),
            TurnOutcome::Interrupted { .. } => {
                // Should not happen without TTSR engine
                anyhow::bail!("Unexpected TTSR interrupt without engine")
            }
        }
    }

    pub async fn run_with_ttsr(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        stream_rules: Option<&mut StreamRuleEngine>,
    ) -> Result<TurnOutcome> {
        let mut stream = self.llm.chat_with_tools_stream(messages, tools);
        let mut final_message: Option<Message> = None;
        let mut accumulated_content = String::new();

        while let Some(item) = stream.next().await {
            match item {
                Ok(Message::Text { content, .. }) => {
                    self.callbacks.fire_stream_delta(&content);
                    accumulated_content.push_str(&content);

                    // Check TTSR rules against accumulated text output
                    if let Some(ref engine) = stream_rules {
                        match engine.check(&accumulated_content) {
                            StreamRuleMatch::Interrupt {
                                name,
                                injection,
                                context,
                            } => {
                                return Ok(TurnOutcome::Interrupted {
                                    name,
                                    injection,
                                    context,
                                    partial_content: accumulated_content,
                                });
                            }
                            StreamRuleMatch::Deferred { .. } => {
                                // Continue streaming; deferred injection handled after turn
                            }
                            StreamRuleMatch::None => {}
                        }
                    }
                }
                Ok(msg) => {
                    final_message = Some(msg);
                    break;
                }
                Err(e) => return Err(e),
            }
        }

        if final_message.is_none() && !accumulated_content.is_empty() {
            final_message = Some(Message::assistant(accumulated_content));
        }

        final_message
            .map(TurnOutcome::Complete)
            .ok_or_else(|| anyhow::anyhow!("No message received from stream"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures::stream;
    use std::sync::{Arc, Mutex};

    struct StreamingTextLlm;

    #[async_trait]
    impl LlmClient for StreamingTextLlm {
        async fn chat(&self, _messages: &[Message]) -> Result<String> {
            Ok("unused".to_string())
        }

        async fn chat_with_tools(
            &self,
            _messages: &[Message],
            _tools: &[ToolDefinition],
        ) -> Result<Message> {
            Ok(Message::assistant("unused".to_string()))
        }

        fn chat_with_tools_stream<'a>(
            &'a self,
            _messages: &'a [Message],
            _tools: &'a [ToolDefinition],
        ) -> std::pin::Pin<Box<dyn futures::Stream<Item = Result<Message>> + Send + 'a>> {
            Box::pin(stream::iter(vec![
                Ok(Message::assistant("hello ".to_string())),
                Ok(Message::assistant("world".to_string())),
            ]))
        }
    }

    #[tokio::test]
    async fn accumulates_streamed_text_messages() {
        let deltas = Arc::new(Mutex::new(String::new()));
        let deltas_for_cb = deltas.clone();
        let callbacks = AgentCallbacks::new().with_stream_delta(Arc::new(move |delta| {
            deltas_for_cb.lock().unwrap().push_str(delta);
        }));
        let runner = TurnRunner::new(&StreamingTextLlm, &callbacks);

        let message = runner.run(&[], &[]).await.unwrap();

        match message {
            Message::Text { content, .. } => assert_eq!(content, "hello world"),
            _ => panic!("expected text message"),
        }
        assert_eq!(&*deltas.lock().unwrap(), "hello world");
    }
}
