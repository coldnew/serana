use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmExtensionDef {
    pub name: String,
    pub version: String,
    pub description: String,
    pub wasm_file: String,
    pub commands: Vec<WasmCommandDef>,
    pub hooks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmCommandDef {
    pub name: String,
    pub description: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EditorHook {
    BeforeSave,
    AfterSave,
    AfterOpen,
    BeforeQuit,
    OnIdle,
}

impl std::str::FromStr for EditorHook {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "before_save" => Ok(Self::BeforeSave),
            "after_save" => Ok(Self::AfterSave),
            "after_open" => Ok(Self::AfterOpen),
            "before_quit" => Ok(Self::BeforeQuit),
            "on_idle" => Ok(Self::OnIdle),
            _ => Err(format!("Unknown hook: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub hooks: Vec<String>,
    #[serde(default)]
    pub keybindings: Vec<ExtensionKeybind>,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionKeybind {
    pub keys: String,
    pub command: String,
    pub mode: Option<String>,
}

pub struct WasmExtension {
    pub manifest: ExtensionManifest,
    pub wasm_bytes: Vec<u8>,
    pub wasm_path: PathBuf,
}

pub struct WasmExtensionHost {
    extensions: Vec<WasmExtension>,
    extension_dirs: Vec<PathBuf>,
    hook_map: HashMap<EditorHook, Vec<usize>>,
}

impl WasmExtensionHost {
    pub fn new() -> Self {
        let mut dirs = Vec::new();
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".mora").join("extensions"));
        }
        dirs.push(PathBuf::from(".mora").join("extensions"));

        Self {
            extensions: Vec::new(),
            extension_dirs: dirs,
            hook_map: HashMap::new(),
        }
    }

    pub fn with_dirs(dirs: Vec<PathBuf>) -> Self {
        Self {
            extensions: Vec::new(),
            extension_dirs: dirs,
            hook_map: HashMap::new(),
        }
    }

    pub fn discover(&mut self) {
        for dir in &self.extension_dirs {
            if !dir.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("wasm") {
                        if let Ok(wasm_bytes) = std::fs::read(&path) {
                            let name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let manifest = ExtensionManifest {
                                name,
                                version: "0.1.0".to_string(),
                                description: String::new(),
                                commands: Vec::new(),
                                hooks: Vec::new(),
                                keybindings: Vec::new(),
                                dependencies: HashMap::new(),
                            };
                            self.extensions.push(WasmExtension {
                                manifest,
                                wasm_bytes,
                                wasm_path: path,
                            });
                        }
                    }
                }
            }
        }
        self.rebuild_hook_map();
    }

    fn rebuild_hook_map(&mut self) {
        self.hook_map.clear();
        for (idx, ext) in self.extensions.iter().enumerate() {
            for hook_str in &ext.manifest.hooks {
                if let Ok(hook) = hook_str.parse::<EditorHook>() {
                    self.hook_map.entry(hook).or_default().push(idx);
                }
            }
        }
    }

    pub fn extensions(&self) -> &[WasmExtension] {
        &self.extensions
    }

    pub fn count(&self) -> usize {
        self.extensions.len()
    }
}

impl WasmExtension {
    pub fn load_manifest(&mut self, manifest: ExtensionManifest) {
        self.manifest = manifest;
    }
}
