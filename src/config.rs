use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    pub workspace: PathBuf,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    #[serde(default)]
    pub interactive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_max_tokens() -> usize { 4096 }
fn default_temperature() -> f32 { 0.7 }

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LlmConfig {
                api_url: "https://api.openai.com/v1".to_string(),
                api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
                model: "gpt-4".to_string(),
                temperature: 0.7,
            },
            workspace: PathBuf::from("."),
            max_tokens: 4096,
            interactive: false,
        }
    }
}
