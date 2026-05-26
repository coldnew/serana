use std::collections::HashMap;

use crate::eval::{EvalError, Evaluator};
use crate::types::Value;
use crate::reader;

pub struct MoraHost {
    evaluator: Evaluator,
    hooks: HashMap<String, Vec<Value>>,
    commands: HashMap<String, Value>,
    keybindings: HashMap<String, Value>,
}

impl MoraHost {
    pub fn new() -> Self {
        Self {
            evaluator: Evaluator::new(),
            hooks: HashMap::new(),
            commands: HashMap::new(),
            keybindings: HashMap::new(),
        }
    }

    pub fn eval(&mut self, code: &str) -> Result<Value, EvalError> {
        let forms = reader::read_all(code)
            .map_err(|e| EvalError::Custom(format!("read error: {}", e)))?;
        let mut result = Value::Nil;
        for form in forms {
            result = self.evaluator.eval(&form)?;
        }
        Ok(result)
    }

    pub fn register_hook(&mut self, hook_name: &str, handler: Value) {
        self.hooks
            .entry(hook_name.to_string())
            .or_default()
            .push(handler);
    }

    pub fn run_hooks(&mut self, hook_name: &str, args: &[Value]) -> Result<Vec<Value>, EvalError> {
        let mut results = Vec::new();
        if let Some(handlers) = self.hooks.get(hook_name).cloned() {
            for handler in handlers {
                let mut all_args = vec![handler];
                all_args.extend(args.iter().cloned());
                results.push(self.evaluator.eval(&Value::list(all_args))?);
            }
        }
        Ok(results)
    }

    pub fn register_command(&mut self, name: &str, handler: Value) {
        self.commands.insert(name.to_string(), handler);
    }

    pub fn run_command(&mut self, name: &str, args: &[Value]) -> Result<Value, EvalError> {
        let handler = self.commands
            .get(name)
            .cloned()
            .ok_or_else(|| EvalError::Custom(format!("command not found: {}", name)))?;
        
        let mut all_args = vec![handler];
        all_args.extend(args.iter().cloned());
        self.evaluator.eval(&Value::list(all_args))
    }

    pub fn register_keybinding(&mut self, key: &str, action: Value) {
        self.keybindings.insert(key.to_string(), action);
    }

    pub fn handle_key(&mut self, key: &str) -> Result<Option<Value>, EvalError> {
        if let Some(action) = self.keybindings.get(key).cloned() {
            Ok(Some(self.evaluator.eval(&action)?))
        } else {
            Ok(None)
        }
    }

    pub fn load_extension(&mut self, code: &str) -> Result<(), EvalError> {
        self.eval(code)?;
        Ok(())
    }

    pub fn load_extension_file(&mut self, path: &str) -> Result<(), EvalError> {
        let code = std::fs::read_to_string(path)
            .map_err(|e| EvalError::Io(e.to_string()))?;
        self.load_extension(&code)
    }

    pub fn evaluator(&self) -> &Evaluator {
        &self.evaluator
    }

    pub fn evaluator_mut(&mut self) -> &mut Evaluator {
        &mut self.evaluator
    }
}

impl Default for MoraHost {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ExtensionManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub commands: Vec<String>,
    pub hooks: Vec<String>,
    pub keybindings: HashMap<String, String>,
}

impl ExtensionManifest {
    pub fn from_value(val: &Value) -> Result<Self, EvalError> {
        match val {
            Value::Map(m) => {
                let get_string = |key: &str| -> Option<String> {
                    m.iter()
                        .find(|(k, _)| matches!(k, Value::Keyword(kw) if kw.name.as_str() == key))
                        .and_then(|(_, v)| match v {
                            Value::String(s) => Some(s.to_string()),
                            _ => None,
                        })
                };

                let get_vec_string = |key: &str| -> Vec<String> {
                    m.iter()
                        .find(|(k, _)| matches!(k, Value::Keyword(kw) if kw.name.as_str() == key))
                        .and_then(|(_, v)| match v {
                            Value::List(v) | Value::Vector(v) => Some(
                                v.iter()
                                    .filter_map(|item| match item {
                                        Value::String(s) => Some(s.to_string()),
                                        _ => None,
                                    })
                                    .collect(),
                            ),
                            _ => None,
                        })
                        .unwrap_or_default()
                };

                let get_map_string = |key: &str| -> HashMap<String, String> {
                    m.iter()
                        .find(|(k, _)| matches!(k, Value::Keyword(kw) if kw.name.as_str() == key))
                        .and_then(|(_, v)| match v {
                            Value::Map(m) => Some(
                                m.iter()
                                    .filter_map(|(k, v)| match (k, v) {
                                        (Value::String(k), Value::String(v)) => {
                                            Some((k.to_string(), v.to_string()))
                                        }
                                        _ => None,
                                    })
                                    .collect(),
                            ),
                            _ => None,
                        })
                        .unwrap_or_default()
                };

                Ok(ExtensionManifest {
                    name: get_string("name").unwrap_or_default(),
                    version: get_string("version").unwrap_or_default(),
                    description: get_string("description").unwrap_or_default(),
                    author: get_string("author").unwrap_or_default(),
                    commands: get_vec_string("commands"),
                    hooks: get_vec_string("hooks"),
                    keybindings: get_map_string("keybindings"),
                })
            }
            _ => Err(EvalError::Type("extension manifest must be a map".to_string())),
        }
    }
}
