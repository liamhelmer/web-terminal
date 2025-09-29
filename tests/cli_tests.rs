// CLI integration tests
// Per spec-kit/005-cli-spec.md and spec-kit/008-testing-spec.md

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("web-terminal").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("web-terminal"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("web-terminal").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("web-terminal"));
}

#[test]
fn test_start_command_help() {
    let mut cmd = Command::cargo_bin("web-terminal").unwrap();
    cmd.args(&["start", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Start the web-terminal server"));
}

#[test]
fn test_config_show() {
    let mut cmd = Command::cargo_bin("web-terminal").unwrap();
    cmd.args(&["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration"));
}

#[test]
fn test_sessions_list() {
    let mut cmd = Command::cargo_bin("web-terminal").unwrap();
    cmd.args(&["sessions", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Active Sessions"));
}

#[test]
fn test_health_check() {
    let mut cmd = Command::cargo_bin("web-terminal").unwrap();
    cmd.args(&["health"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Health Check"));
}

#[test]
fn test_metrics_display() {
    let mut cmd = Command::cargo_bin("web-terminal").unwrap();
    cmd.args(&["metrics"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Server Metrics"));
}

#[test]
fn test_status_json_output() {
    let mut cmd = Command::cargo_bin("web-terminal").unwrap();
    cmd.args(&["status", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"running\""));
}

#[test]
fn test_completions_bash() {
    let mut cmd = Command::cargo_bin("web-terminal").unwrap();
    cmd.args(&["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_web-terminal"));
}

#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("web-terminal").unwrap();
    cmd.arg("invalid-command")
        .assert()
        .failure();
}

#[test]
fn test_global_verbose_flag() {
    let mut cmd = Command::cargo_bin("web-terminal").unwrap();
    cmd.args(&["--verbose", "status"])
        .assert()
        .success();
}

#[test]
fn test_global_quiet_flag() {
    let mut cmd = Command::cargo_bin("web-terminal").unwrap();
    cmd.args(&["--quiet", "status"])
        .assert()
        .success();
}

#[test]
fn test_config_env_override() {
    let mut cmd = Command::cargo_bin("web-terminal").unwrap();
    cmd.env("WEB_TERMINAL_CONFIG", "custom-config.toml")
        .args(&["config", "show"])
        .assert()
        .success();
}