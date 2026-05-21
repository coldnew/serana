//! Subagent delegation for parallel task execution.
//!
//! Allows parent agents to spawn child agents with reduced iteration budgets.

use crate::agent::{Agent, AgentCallbacks, AgentOutput, IterationBudget};
use crate::llm::{LlmClient, Message};
use crate::tools::ToolRegistry;
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Configuration for subagent spawning.
#[derive(Debug, Clone)]
pub struct SubagentConfig {
    /// Iteration budget for the subagent
    pub budget: IterationBudget,
    /// Callbacks for progress tracking
    pub callbacks: AgentCallbacks,
}

impl Default for SubagentConfig {
    fn default() -> Self {
        Self {
            budget: IterationBudget::default_subagent(),
            callbacks: AgentCallbacks::new(),
        }
    }
}

/// Subagent task specification.
#[derive(Debug, Clone)]
pub struct SubagentTask {
    /// Unique task identifier
    pub id: String,
    /// Task instruction
    pub instruction: String,
    /// Optional configuration override
    pub config: Option<SubagentConfig>,
}

impl SubagentTask {
    pub fn new(id: impl Into<String>, instruction: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            instruction: instruction.into(),
            config: None,
        }
    }

    pub fn with_config(mut self, config: SubagentConfig) -> Self {
        self.config = Some(config);
        self
    }
}

/// Result from a subagent execution.
#[derive(Debug)]
pub struct SubagentResult {
    pub task_id: String,
    pub output: Result<AgentOutput>,
}

/// Wrapper to convert Arc<dyn LlmClient> to Box<dyn LlmClient>
struct SubagentLlm {
    inner: Arc<dyn LlmClient>,
}

#[async_trait]
impl LlmClient for SubagentLlm {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        self.inner.chat(messages).await
    }

    async fn chat_with_tools(
        &self,
        messages: &[Message],
        tools: &[crate::llm::ToolDefinition],
    ) -> Result<Message> {
        self.inner.chat_with_tools(messages, tools).await
    }
}

/// Subagent spawner for parallel task delegation.
pub struct SubagentSpawner {
    llm: Arc<dyn LlmClient>,
    default_config: SubagentConfig,
}

impl SubagentSpawner {
    pub fn new(llm: Arc<dyn LlmClient>) -> Self {
        Self {
            llm,
            default_config: SubagentConfig::default(),
        }
    }

    pub fn with_default_config(mut self, config: SubagentConfig) -> Self {
        self.default_config = config;
        self
    }

    /// Spawn a single subagent task.
    pub fn spawn_task(&self, task: SubagentTask) -> JoinHandle<SubagentResult> {
        let llm = self.llm.clone();
        let config = task
            .config
            .clone()
            .unwrap_or_else(|| self.default_config.clone());
        let task_id = task.id.clone();
        let instruction = task.instruction.clone();

        tokio::spawn(async move {
            let agent = crate::agent::coding::CodingAgent::new(
                Box::new(SubagentLlm { inner: llm }),
                ToolRegistry::new(),
            )
            .with_budget(config.budget)
            .with_callbacks(config.callbacks);

            let output = agent.execute(&instruction).await;

            SubagentResult { task_id, output }
        })
    }

    /// Spawn multiple subagent tasks in parallel.
    pub fn spawn_tasks(&self, tasks: Vec<SubagentTask>) -> Vec<JoinHandle<SubagentResult>> {
        tasks
            .into_iter()
            .map(|task| self.spawn_task(task))
            .collect()
    }

    /// Spawn tasks and wait for all to complete.
    pub async fn execute_tasks(&self, tasks: Vec<SubagentTask>) -> Vec<SubagentResult> {
        let handles = self.spawn_tasks(tasks);
        let mut results = Vec::with_capacity(handles.len());

        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(SubagentResult {
                        task_id: "unknown".to_string(),
                        output: Err(anyhow::anyhow!("Task join error: {}", e)),
                    });
                }
            }
        }

        results
    }
}

/// Delegate a task to a subagent (convenience function).
pub async fn delegate_task(
    llm: Arc<dyn LlmClient>,
    task_id: impl Into<String>,
    instruction: impl Into<String>,
) -> Result<AgentOutput> {
    let spawner = SubagentSpawner::new(llm);
    let task = SubagentTask::new(task_id, instruction);
    let handle = spawner.spawn_task(task);
    let result = handle
        .await
        .map_err(|e| anyhow::anyhow!("Join error: {}", e))?;
    result.output
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockLlm;

    #[async_trait]
    impl LlmClient for MockLlm {
        async fn chat(&self, _messages: &[Message]) -> Result<String> {
            Ok("Mock response".to_string())
        }

        async fn chat_with_tools(
            &self,
            _messages: &[Message],
            _tools: &[crate::llm::ToolDefinition],
        ) -> Result<Message> {
            Ok(Message::assistant("Mock response".to_string()))
        }
    }

    #[tokio::test]
    async fn spawns_subagent_task() {
        let llm = Arc::new(MockLlm) as Arc<dyn LlmClient>;
        let spawner = SubagentSpawner::new(llm);

        let task = SubagentTask::new("task1", "Test instruction");
        let handle = spawner.spawn_task(task);
        let result = handle.await.unwrap();

        assert_eq!(result.task_id, "task1");
        assert!(result.output.is_ok());
    }

    #[tokio::test]
    async fn executes_multiple_tasks() {
        let llm = Arc::new(MockLlm) as Arc<dyn LlmClient>;
        let spawner = SubagentSpawner::new(llm);

        let tasks = vec![
            SubagentTask::new("task1", "Instruction 1"),
            SubagentTask::new("task2", "Instruction 2"),
        ];

        let results = spawner.execute_tasks(tasks).await;
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.output.is_ok()));
    }

    #[tokio::test]
    async fn delegate_task_convenience() {
        let llm = Arc::new(MockLlm) as Arc<dyn LlmClient>;

        let output = delegate_task(llm, "task1", "Test instruction").await;
        assert!(output.is_ok());
    }
}
