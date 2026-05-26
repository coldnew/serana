use std::process::Command;

const SERANA_ROOT: &str = env!("CARGO_MANIFEST_DIR");

#[test]
fn can_build_self() {
    let output = Command::new("cargo")
        .current_dir(SERANA_ROOT)
        .args(["build"])
        .output()
        .expect("Failed to run cargo build");
    assert!(output.status.success());
}

#[test]
fn git_status_works() {
    let output = Command::new("git")
        .current_dir(SERANA_ROOT)
        .args(["status", "--short"])
        .output()
        .expect("Failed to run git status");
    assert!(output.status.success());
}

#[test]
fn can_read_own_cargo_toml() {
    let content = std::fs::read_to_string(format!("{}/Cargo.toml", SERANA_ROOT)).unwrap();
    assert!(content.contains("[workspace]"));
}
