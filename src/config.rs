//! Configuration management for serana
//!
//! Loads configuration from ~/.serana/config.toml with environment variable overrides.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use anyhow::Context;

/// Main configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub workspace: PathBuf,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    #[serde(default)]
    pub interactive: bool,
}

/// Provider configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name: "openai", "anthropic", "ollama", "openrouter", "custom"
    #[serde(default = "default_provider")]
    pub name: String,
    /// Custom URL (required when name = "custom")
    #[serde(default)]
    pub url: Option<String>,
}

fn default_provider() -> String {
    "openai".to_string()
}

fn default_max_tokens() -> usize { 4096 }
fn default_temperature() -> f32 { 0.7 }

/// LLM-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// API key - prefer environment variable SERANA_API_KEY
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_model() -> String {
    "gpt-4".to_string()
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            temperature: default_temperature(),
            api_key: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: ProviderConfig::default(),
            llm: LlmConfig::default(),
            workspace: PathBuf::from("."),
            max_tokens: 4096,
            interactive: false,
        }
    }
}

impl Config {
    /// Load configuration from ~/.serana/config.toml
    /// Falls back to defaults if file doesn't exist.
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_path();
        
        if config_path.exists() {
            Self::load_from_path(&config_path)
        } else {
            tracing::info!("No config file found at {:?}, using defaults", config_path);
            Ok(Self::default())
        }
    }
    
    /// Load configuration from a specific path
    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {:?}", path))?;
        
        let mut config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config from {:?}", path))?;
        
        // Apply environment variable overrides
        config.apply_env_overrides();
        
        Ok(config)
    }
    
    /// Get the configuration file path: ~/.serana/config.toml
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("serana")
            .join("config.toml")
    }
    
    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        // Provider override
        if let Ok(provider) = std::env::var("SERANA_PROVIDER") {
            self.provider.name = provider;
        }
        
        // API key override (highest priority)
        if let Ok(api_key) = std::env::var("SERANA_API_KEY") {
            self.llm.api_key = Some(api_key);
        }
        
        // Also check provider-specific env vars as fallback
        if self.llm.api_key.is_none() {
            self.llm.api_key = std::env::var("OPENAI_API_KEY").ok();
        }
        
        // Model override
        if let Ok(model) = std::env::var("SERANA_MODEL") {
            self.llm.model = model;
        }
    }
    
    /// Get the resolved API URL based on provider
    pub fn api_url(&self) -> String {
        match self.provider.name.as_str() {
            "openai" => "https://api.openai.com/v1".to_string(),
            "anthropic" => "https://api.anthropic.com/v1".to_string(),
            "ollama" => "http://localhost:11434/v1".to_string(),
            "openrouter" => "https://openrouter.ai/api/v1".to_string(),
            "custom" => self.provider.url.clone().unwrap_or_default(),
            _ => self.provider.url.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        }
    }
    
    /// Get the resolved API key
    pub fn api_key(&self) -> Option<String> {
        self.llm.api_key.clone()
    }
    
    /// Get the model name
    pub fn model(&self) -> &str {
        &self.llm.model
    }
    
    /// Get the temperature
    pub fn temperature(&self) -> f32 {
        self.llm.temperature
    }
}

/// Generate a sample configuration file
pub fn generate_sample_config() -> String {
    let config = Config {
        provider: ProviderConfig {
            name: "openai".to_string(),
            url: None,
        },
        llm: LlmConfig {
            model: "gpt-4".to_string(),
            temperature: 0.7,
            api_key: None,
        },
        workspace: PathBuf::from("."),
        max_tokens: 4096,
        interactive: false,
    };
    
    toml::to_string_pretty(&config).unwrap_or_default()
}
