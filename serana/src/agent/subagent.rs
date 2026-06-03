use super::{AgentFactory, AgentRuntimeConfig};
use crate::core::{
    Agent, AgentCallbacks, AgentOutput, IterationBudget, LlmClient, Message, Result,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct SubagentConfig {
    pub budget: IterationBudget,
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

#[derive(Debug, Clone)]
pub struct SubagentTask {
    pub id: String,
    pub instruction: String,
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

#[derive(Debug)]
pub struct SubagentResult {
    pub task_id: String,
    pub output: Result<AgentOutput>,
}

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
        tools: &[crate::core::ToolDefinition],
    ) -> Result<Message> {
        self.inner.chat_with_tools(messages, tools).await
    }
}

pub struct SubagentSpawner {
    llm: Arc<dyn LlmClient>,
    default_config: SubagentConfig,
    factory: AgentFactory,
}

impl SubagentSpawner {
    pub fn new(llm: Arc<dyn LlmClient>) -> Self {
        Self {
            llm,
            default_config: SubagentConfig::default(),
            factory: AgentFactory::hermes(AgentRuntimeConfig::default()),
        }
    }

    pub fn with_default_config(mut self, config: SubagentConfig) -> Self {
        self.default_config = config;
        self
    }

    pub fn with_runtime_config(mut self, config: AgentRuntimeConfig) -> Self {
        self.factory = AgentFactory::hermes(config);
        self
    }

    pub fn with_factory(mut self, factory: AgentFactory) -> Self {
        self.factory = factory;
        self
    }

    pub fn spawn_task(&self, task: SubagentTask) -> JoinHandle<SubagentResult> {
        let llm = self.llm.clone();
        let config = task
            .config
            .clone()
            .unwrap_or_else(|| self.default_config.clone());
        let task_id = task.id.clone();
        let instruction = task.instruction.clone();
        let factory = self.factory.clone();

        tokio::spawn(async move {
            let agent = factory
                .build(Box::new(SubagentLlm { inner: llm }))
                .with_budget(config.budget)
                .with_callbacks(config.callbacks);

            let output = agent.execute(&instruction).await;

            SubagentResult { task_id, output }
        })
    }

    pub fn spawn_tasks(&self, tasks: Vec<SubagentTask>) -> Vec<JoinHandle<SubagentResult>> {
        tasks
            .into_iter()
            .map(|task| self.spawn_task(task))
            .collect()
    }

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
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockLlm;

    #[async_trait]
    impl LlmClient for MockLlm {
        async fn chat(&self, _messages: &[Message]) -> Result<String> {
            Ok("Mock response".to_string())
        }

        async fn chat_with_tools(
            &self,
            _messages: &[Message],
            _tools: &[crate::core::ToolDefinition],
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

    struct RuntimeAwareLlm {
        saw_skill: Arc<AtomicBool>,
    }

    #[async_trait]
    impl LlmClient for RuntimeAwareLlm {
        async fn chat(&self, _messages: &[Message]) -> Result<String> {
            Ok("Mock response".to_string())
        }

        async fn chat_with_tools(
            &self,
            messages: &[Message],
            _tools: &[crate::core::ToolDefinition],
        ) -> Result<Message> {
            let has_skill = messages.iter().any(|message| match message {
                Message::Text { role, content } if role == "system" => {
                    content.contains("[research] Inspect code before editing")
                }
                _ => false,
            });
            self.saw_skill.store(has_skill, Ordering::SeqCst);
            Ok(Message::assistant("Mock response".to_string()))
        }
    }

    #[tokio::test]
    async fn subagents_inherit_runtime_config() {
        let saw_skill = Arc::new(AtomicBool::new(false));
        let llm = Arc::new(RuntimeAwareLlm {
            saw_skill: saw_skill.clone(),
        }) as Arc<dyn LlmClient>;
        let runtime = AgentRuntimeConfig::default()
            .with_skills(vec!["[research] Inspect code before editing".to_string()]);
        let spawner = SubagentSpawner::new(llm).with_runtime_config(runtime);

        let results = spawner
            .execute_tasks(vec![SubagentTask::new("task1", "Test instruction")])
            .await;

        assert_eq!(results.len(), 1);
        assert!(results[0].output.is_ok());
        assert!(saw_skill.load(Ordering::SeqCst));
    }

    #[test]
    fn runtime_config_uses_hermes_factory_defaults() {
        let llm = Arc::new(MockLlm) as Arc<dyn LlmClient>;
        let spawner = SubagentSpawner::new(llm).with_runtime_config(AgentRuntimeConfig::default());
        let tools = spawner.factory.build_tools();

        assert!(tools.get("read_self").is_some());
        assert!(tools.get("lsp_definition").is_some());
        assert!(tools.get("skill_create").is_some());
    }
}
