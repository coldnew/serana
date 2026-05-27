pub mod codegen;
pub mod compiler;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("read error: {0}")]
    Read(String),
    #[error("codegen error: {0}")]
    Codegen(String),
    #[error("compilation error: {0}")]
    Compilation(String),
    #[error("{0}")]
    Custom(String),
}

/// Target type for compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileTarget {
    /// Compile to a standalone binary
    Binary,
    /// Compile to a shared library (.so/.dylib/.dll)
    SharedLib,
}

/// Compilation options
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Target type (binary or shared library)
    pub target: CompileTarget,
    /// Output path (optional, defaults to input name with appropriate extension)
    pub output: Option<String>,
    /// Optimization level (0-3)
    pub opt_level: u8,
    /// Enable debug info
    pub debug: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            target: CompileTarget::Binary,
            output: None,
            opt_level: 2,
            debug: false,
        }
    }
}

/// Compile a .mora file to binary or shared library
pub fn compile_file(input: &str, options: &CompileOptions) -> Result<String, CompileError> {
    let code = std::fs::read_to_string(input)
        .map_err(|e| CompileError::Io(e))?;

    let output_path = options.output.clone().unwrap_or_else(|| {
        let stem = std::path::Path::new(input)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        match options.target {
            CompileTarget::Binary => stem,
            CompileTarget::SharedLib => {
                #[cfg(target_os = "macos")]
                { format!("lib{}.dylib", stem) }
                #[cfg(target_os = "linux")]
                { format!("lib{}.so", stem) }
                #[cfg(target_os = "windows")]
                { format!("{}.dll", stem) }
            }
        }
    });

    compiler::compile(&code, input, &output_path, options)
}
