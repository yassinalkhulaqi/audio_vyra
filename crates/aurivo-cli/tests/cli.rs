//! End-to-end tests for the diagnostic CLI modules.

use std::fs;
use std::process::Command;

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("aurivo-cli-test-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn help_command_succeeds() {
    let exe = env!("CARGO_BIN_EXE_aurivo-cli");
    let output = Command::new(exe).arg("help").output().unwrap();
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("aurivo"));
}

#[test]
fn effects_command_lists_twelve_stages() {
    let exe = env!("CARGO_BIN_EXE_aurivo-cli");
    let output = Command::new(exe).arg("effects").output().unwrap();
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("limiter"));
    assert_eq!(text.lines().count(), 12);
}

#[test]
fn preset_validate_reports_invalid_file() {
    let dir = temp_dir("preset");
    let path = dir.join("bad.json");
    fs::write(&path, r#"{"name":"","params":{"gain_db":null}}"#).unwrap();
    let exe = env!("CARGO_BIN_EXE_aurivo-cli");
    let output = Command::new(exe).arg("preset").arg("validate").arg(&path).output().unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("problem"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn state_diff_reports_changes() {
    let dir = temp_dir("state");
    let a = dir.join("a.json");
    let b = dir.join("b.json");
    fs::write(&a, r#"{"input_gain_db":0.0}"#).unwrap();
    fs::write(&b, r#"{"input_gain_db":2.0}"#).unwrap();
    let exe = env!("CARGO_BIN_EXE_aurivo-cli");
    let output = Command::new(exe).arg("state").arg("diff").arg(&a).arg(&b).output().unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("input_gain_db"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unknown_command_fails() {
    let exe = env!("CARGO_BIN_EXE_aurivo-cli");
    let output = Command::new(exe).arg("nope").output().unwrap();
    assert!(!output.status.success());
}
