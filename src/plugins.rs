//! Plugin system for loading external tools.
//!
//! Plugins are TOML files in ~/.serana/plugins/ that define tool name,
//! description, parameters, and a bash command to execute.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

use serana_core::{Result, Tool};

/// Plugin definition loaded from a TOML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDef {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub parameters: Option<Value>,
    /// The command to execute. Supports {{param}} template variables.
    pub command: String,
    /// Timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_timeout() -> u64 {
    30
}

/// A tool backed by a plugin definition.
pub struct PluginTool {
    def: PluginDef,
}

impl PluginTool {
    pub fn new(def: PluginDef) -> Self {
        Self { def }
    }
}

#[async_trait]
impl Tool for PluginTool {
    fn name(&self) -> &'static str {
        // Leak the name string since Tool requires 'static.
        // Plugins are loaded once at startup so this is bounded.
        Box::leak(self.def.name.clone().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        Box::leak(self.def.description.clone().into_boxed_str())
    }


    fn parameters(&self) -> Value {
        self.def.parameters.clone().unwrap_or_else(|| {
            json!({"type": "object", "properties": {}})
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        // Template substitution: replace {{key}} with input.key values
        let mut cmd_str = self.def.command.clone();
        if let Some(obj) = input.as_object() {
            for (key, value) in obj {
                let placeholder = format!("{{{{{}}}}}", key);
                let replacement = match value {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                cmd_str = cmd_str.replace(&placeholder, &replacement);
            }
        }

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.def.timeout),
            Command::new("sh")
                .arg("-c")
                .arg(&cmd_str)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Plugin '{}' timed out after {}s", self.def.name, self.def.timeout))?
        .map_err(|e| anyhow::anyhow!("Plugin '{}' failed: {}", self.def.name, e))?;

        Ok(json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code().unwrap_or(-1),
        }))
    }
}

/// Load plugin definitions from ~/.serana/plugins/*.toml and .serana/plugins/*.toml.
pub async fn load_plugins() -> Vec<PluginDef> {
    let mut plugins = Vec::new();

    let home_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".serana")
        .join("plugins");

    let project_dir = PathBuf::from(".serana").join("plugins");

    for dir in &[home_dir, project_dir] {
        if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        if let Ok(def) = toml::from_str::<PluginDef>(&content) {
                            plugins.push(def);
                        }
                    }
                }
            }
        }
    }

    plugins
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plugin_definition() {
        let toml_str = r#"
name = "lint_check"
description = "Run linter on the project"
command = "cargo clippy {{args}}"
timeout = 60
"#;
        let def: PluginDef = toml::from_str(toml_str).unwrap();
        assert_eq!(def.name, "lint_check");
        assert_eq!(def.command, "cargo clippy {{args}}");
        assert_eq!(def.timeout, 60);
    }

    #[test]
    fn plugin_default_timeout() {
        let toml_str = r#"
name = "test"
description = "test"
command = "echo hello"
"#;
        let def: PluginDef = toml::from_str(toml_str).unwrap();
        assert_eq!(def.timeout, 30);
    }
}
