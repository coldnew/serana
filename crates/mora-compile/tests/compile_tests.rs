use mora_compile::{CompileOptions, CompileTarget, compile_file};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_compile_to_binary() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("test.mora");
    let output_path = temp_dir.path().join("test_binary");

    // Write a simple mora script
    fs::write(&input_path, r#"
(println "Hello from test!")
(+ 1 2)
"#).unwrap();

    let options = CompileOptions {
        target: CompileTarget::Binary,
        output: Some(output_path.to_string_lossy().to_string()),
        opt_level: 0,
        debug: false,
    };

    let result = compile_file(input_path.to_str().unwrap(), &options);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    let output = result.unwrap();
    assert!(std::path::Path::new(&output).exists(), "Output file does not exist");

    // Run the compiled binary
    let output = std::process::Command::new(&output)
        .output()
        .expect("Failed to run compiled binary");

    assert!(output.status.success(), "Binary exited with error");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello from test!"), "Expected output not found: {}", stdout);
}

#[test]
fn test_compile_to_shared_lib() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("test.mora");
    let output_path = temp_dir.path().join("libtest.so");

    // Write a simple mora script
    fs::write(&input_path, r#"
(defn add [a b] (+ a b))
"#).unwrap();

    let options = CompileOptions {
        target: CompileTarget::SharedLib,
        output: Some(output_path.to_string_lossy().to_string()),
        opt_level: 0,
        debug: false,
    };

    let result = compile_file(input_path.to_str().unwrap(), &options);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    let output = result.unwrap();
    assert!(std::path::Path::new(&output).exists(), "Output file does not exist");
}

#[test]
fn test_compile_with_complex_script() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("complex.mora");
    let output_path = temp_dir.path().join("complex");

    // Write a more complex script
    fs::write(&input_path, r#"
;; Factorial function
(defn factorial [n]
  (if (<= n 1)
    1
    (* n (factorial (- n 1)))))

(println (str "5! = " (factorial 5)))
(println (str "10! = " (factorial 10)))
"#).unwrap();

    let options = CompileOptions {
        target: CompileTarget::Binary,
        output: Some(output_path.to_string_lossy().to_string()),
        opt_level: 2,
        debug: false,
    };

    let result = compile_file(input_path.to_str().unwrap(), &options);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    let output = result.unwrap();
    assert!(std::path::Path::new(&output).exists(), "Output file does not exist");

    // Run the compiled binary
    let output = std::process::Command::new(&output)
        .output()
        .expect("Failed to run compiled binary");

    assert!(output.status.success(), "Binary exited with error");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("5! = 120"), "Expected factorial(5) = 120, got: {}", stdout);
    assert!(stdout.contains("10! = 3628800"), "Expected factorial(10) = 3628800, got: {}", stdout);
}
