//! Multi-provider registry with role-based model routing.
//!
//! Maps roles (default, smol, slow, plan) to LLM clients with fallback chains.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use serana_core::{Config, LlmClient, Message, Result, ToolDefinition};

use crate::{FallbackChain, OpenAiClient, OpenRouterClient};

/// Model roles route work by intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelRole {
    /// Default for normal turns.
    Default,
    /// Cheap model for subagent fan-out.
    Smol,
    /// Deep reasoning model.
    Slow,
    /// Plan mode model.
    Plan,
    /// Commit / changelog model.
    Commit,
}

impl ModelRole {
    pub fn all() -> &'static [ModelRole] {
        &[
            ModelRole::Default,
            ModelRole::Smol,
            ModelRole::Slow,
            ModelRole::Plan,
            ModelRole::Commit,
        ]
    }
}

impl std::fmt::Display for ModelRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => write!(f, "default"),
            Self::Smol => write!(f, "smol"),
            Self::Slow => write!(f, "slow"),
            Self::Plan => write!(f, "plan"),
            Self::Commit => write!(f, "commit"),
        }
    }
}

impl std::str::FromStr for ModelRole {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "default" => Ok(Self::Default),
            "smol" => Ok(Self::Smol),
            "slow" => Ok(Self::Slow),
            "plan" => Ok(Self::Plan),
            "commit" => Ok(Self::Commit),
            _ => Err(anyhow::anyhow!("Unknown model role: '{}'", s)),
        }
    }
}

/// Registry mapping model roles to LLM clients with fallback chains.
pub struct ProviderRegistry {
    /// Per-role fallback chains, Arc-wrapped so RoutingClient can clone and drop the lock.
    roles: HashMap<ModelRole, Arc<FallbackChain>>,
    /// All known clients by name.
    providers: HashMap<String, Arc<dyn LlmClient>>,
    /// Active role.
    active_role: ModelRole,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
            providers: HashMap::new(),
            active_role: ModelRole::Default,
        }
    }

    /// Build a registry from a config with multiple providers.
    pub fn from_config(config: &Config) -> Result<Self> {
        let mut registry = Self::new();

        let primary: Arc<dyn LlmClient> = match config.provider.name.as_str() {
            "openrouter" => Arc::new(OpenRouterClient::new(config.clone())),
            _ => Arc::new(OpenAiClient::new(config.clone())),
        };
        registry.register_provider(&config.provider.name, primary.clone());

        let mut default_chain = FallbackChain::with_defaults();
        default_chain.add_provider(&config.provider.name, primary.clone());
        registry
            .roles
            .insert(ModelRole::Default, Arc::new(default_chain));

        for role in &[
            ModelRole::Smol,
            ModelRole::Slow,
            ModelRole::Plan,
            ModelRole::Commit,
        ] {
            let mut chain = FallbackChain::with_defaults();
            chain.add_provider(&config.provider.name, primary.clone());
            registry.roles.insert(*role, Arc::new(chain));
        }

        Ok(registry)
    }

    /// Register a named provider.
    pub fn register_provider(&mut self, name: &str, client: Arc<dyn LlmClient>) {
        self.providers.insert(name.to_string(), client);
    }

    /// Add a provider to a specific role's fallback chain.
    pub fn add_to_role(
        &mut self,
        role: ModelRole,
        provider_name: &str,
        client: Arc<dyn LlmClient>,
    ) {
        self.providers
            .insert(provider_name.to_string(), client.clone());
        let chain = self
            .roles
            .entry(role)
            .or_insert_with(|| Arc::new(FallbackChain::with_defaults()));
        // FallbackChain doesn't support mutation through Arc, so we rebuild.
        // For v1 this is acceptable since roles are configured at startup.
        let mut new_chain = FallbackChain::with_defaults();
        new_chain.add_provider(provider_name, client);
        *chain = Arc::new(new_chain);
    }

    /// Set the active role.
    pub fn set_active_role(&mut self, role: ModelRole) {
        self.active_role = role;
    }

    /// Get the active role.
    pub fn active_role(&self) -> ModelRole {
        self.active_role
    }

    /// Cycle to the next role.
    pub fn cycle_role(&mut self) -> ModelRole {
        let all = ModelRole::all();
        let idx = all.iter().position(|r| *r == self.active_role).unwrap_or(0);
        let next = all[(idx + 1) % all.len()];
        self.active_role = next;
        next
    }

    /// Get the Arc'd fallback chain for the active role.
    fn active_chain(&self) -> Option<Arc<FallbackChain>> {
        self.roles.get(&self.active_role).cloned()
    }

    /// Get provider names for the active role.
    pub fn active_providers(&self) -> Vec<String> {
        self.roles
            .get(&self.active_role)
            .map(|chain| chain.provider_count())
            .map(|count| (0..count).map(|i| format!("provider_{}", i)).collect())
            .unwrap_or_default()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A routing LLM client that delegates to the active role in a ProviderRegistry.
pub struct RoutingClient {
    registry: parking_lot::RwLock<ProviderRegistry>,
}

impl RoutingClient {
    pub fn new(registry: ProviderRegistry) -> Self {
        Self {
            registry: parking_lot::RwLock::new(registry),
        }
    }

    pub fn set_role(&self, role: ModelRole) {
        self.registry.write().set_active_role(role);
    }

    pub fn active_role(&self) -> ModelRole {
        self.registry.read().active_role()
    }

    pub fn cycle_role(&self) -> ModelRole {
        self.registry.write().cycle_role()
    }

    /// Clone the Arc chain for the active role, then drop the lock.
    fn get_chain(&self) -> Result<Arc<FallbackChain>> {
        self.registry
            .read()
            .active_chain()
            .ok_or_else(|| anyhow::anyhow!("No client for active role"))
    }
}

#[async_trait]
impl LlmClient for RoutingClient {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let chain = self.get_chain()?;
        // Arc is cloned, lock is dropped before .await.
        chain.chat(messages).await
    }

    async fn chat_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message> {
        let chain = self.get_chain()?;
        chain.chat_with_tools(messages, tools).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_role_from_str() {
        assert_eq!("default".parse::<ModelRole>().unwrap(), ModelRole::Default);
        assert_eq!("smol".parse::<ModelRole>().unwrap(), ModelRole::Smol);
        assert_eq!("slow".parse::<ModelRole>().unwrap(), ModelRole::Slow);
        assert!("unknown".parse::<ModelRole>().is_err());
    }

    #[test]
    fn model_role_display() {
        assert_eq!(ModelRole::Default.to_string(), "default");
        assert_eq!(ModelRole::Smol.to_string(), "smol");
    }

    #[test]
    fn registry_from_config() {
        let config = Config::default();
        let registry = ProviderRegistry::from_config(&config).unwrap();
        let chain = registry.active_chain();
        assert!(chain.is_some());
    }

    #[test]
    fn role_cycling() {
        let config = Config::default();
        let mut registry = ProviderRegistry::from_config(&config).unwrap();
        assert_eq!(registry.active_role(), ModelRole::Default);
        assert_eq!(registry.cycle_role(), ModelRole::Smol);
        assert_eq!(registry.cycle_role(), ModelRole::Slow);
    }
}
