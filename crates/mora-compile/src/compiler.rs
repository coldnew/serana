use std::path::PathBuf;
use std::process::Command;

use crate::{CompileError, CompileOptions, CompileTarget};
use crate::codegen;

/// Compile mora-lisp code to binary or shared library
pub fn compile(
    code: &str,
    source_name: &str,
    output_path: &str,
    options: &CompileOptions,
) -> Result<String, CompileError> {
    // Generate Rust source code
    let rust_code = codegen::generate(code, source_name, options.target);

    // Create temporary cargo project
    let temp_dir = create_temp_project(&rust_code, options)?;

    // Build the project
    let artifact_path = build_project(&temp_dir, output_path, options)?;

    // Clean up temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    Ok(artifact_path)
}

fn create_temp_project(rust_code: &str, options: &CompileOptions) -> Result<PathBuf, CompileError> {
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("mora-compile-{}-{}", std::process::id(), unique_id));
    std::fs::create_dir_all(&temp_dir)?;

    // Create Cargo.toml
    let lib_section = match options.target {
        CompileTarget::Binary => String::new(),
        CompileTarget::SharedLib => {
            r#"
[lib]
crate-type = ["cdylib"]
"#.to_string()
        }
    };
    let cargo_toml = format!(
        r#"[package]
name = "mora-compiled"
version = "0.1.0"
edition = "2021"

[dependencies]
mora-lisp = {{ path = "{}" }}
{}
[profile.release]
opt-level = {}
debug = {}
"#,
        get_mora_lisp_path(),
        lib_section,
        options.opt_level,
        options.debug
    );
    std::fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;

    // Create src directory and main.rs
    let src_dir = temp_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;

    match options.target {
        CompileTarget::Binary => {
            std::fs::write(src_dir.join("main.rs"), rust_code)?;
        }
        CompileTarget::SharedLib => {
            std::fs::write(src_dir.join("lib.rs"), rust_code)?;
        }
    }

    Ok(temp_dir)
}

fn get_mora_lisp_path() -> String {
    // Try to find the mora-lisp crate relative to the current directory
    let candidates = vec![
        "../mora-lisp",
        "../../crates/mora-lisp",
        "crates/mora-lisp",
    ];

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.join("Cargo.toml").exists() {
            return path.canonicalize()
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
        }
    }

    // Fallback: use workspace-relative path
    "../../crates/mora-lisp".to_string()
}

fn build_project(
    project_dir: &PathBuf,
    output_path: &str,
    options: &CompileOptions,
) -> Result<String, CompileError> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build");

    if options.opt_level > 0 {
        cmd.arg("--release");
    }

    match options.target {
        CompileTarget::Binary => {}
        CompileTarget::SharedLib => {
            cmd.arg("--lib");
        }
    }

    cmd.current_dir(project_dir);

    let output = cmd.output().map_err(|e| {
        CompileError::Compilation(format!("Failed to run cargo: {}", e))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CompileError::Compilation(format!(
            "Cargo build failed:\n{}",
            stderr
        )));
    }

    // Find the built artifact
    let artifact_name = match options.target {
        CompileTarget::Binary => {
            #[cfg(target_os = "windows")]
            { "mora-compiled.exe" }
            #[cfg(not(target_os = "windows"))]
            { "mora-compiled" }
        }
        CompileTarget::SharedLib => {
            #[cfg(target_os = "macos")]
            { "libmora_compiled.dylib" }
            #[cfg(target_os = "linux")]
            { "libmora_compiled.so" }
            #[cfg(target_os = "windows")]
            { "mora_compiled.dll" }
        }
    };

    let profile = if options.opt_level > 0 { "release" } else { "debug" };
    let artifact_path = project_dir
        .join("target")
        .join(profile)
        .join(artifact_name);

    if !artifact_path.exists() {
        return Err(CompileError::Compilation(format!(
            "Built artifact not found at {}",
            artifact_path.display()
        )));
    }

    // Copy to output path
    let output = PathBuf::from(output_path);
    std::fs::copy(&artifact_path, &output).map_err(|e| {
        CompileError::Io(e)
    })?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&output)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&output, perms)?;
    }

    Ok(output.to_string_lossy().to_string())
}
