use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("nebula").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("A distributed P2P file sharing system"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("nebula").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn test_start_command_default() {
    let temp_dir = TempDir::new().unwrap();
    
    let mut cmd = Command::cargo_bin("nebula").unwrap();
    cmd.arg("start")
       .arg("--daemon")  // Use daemon mode to avoid interactive waiting
       .arg("--storage")
       .arg(temp_dir.path());
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Starting node in daemon mode"));
}

#[test]
fn test_start_command_custom_port() {
    let temp_dir = TempDir::new().unwrap();
    
    let mut cmd = Command::cargo_bin("nebula").unwrap();
    cmd.arg("start")
       .arg("--daemon")  // Use daemon mode to avoid interactive waiting
       .arg("--port").arg("8080")
       .arg("--storage").arg(temp_dir.path());
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Starting node in daemon mode"));
}

#[test]
fn test_config_show() {
    let temp_dir = TempDir::new().unwrap();
    
    let mut cmd = Command::cargo_bin("nebula").unwrap();
    cmd.arg("config")
       .arg("--show")
       .arg("--storage")
       .arg(temp_dir.path());
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Current configuration"));
}

#[test]
fn test_verbose_logging() {
    let temp_dir = TempDir::new().unwrap();
    
    let mut cmd = Command::cargo_bin("nebula").unwrap();
    cmd.arg("--verbose")
       .arg("config")
       .arg("--show")
       .arg("--storage")
       .arg(temp_dir.path());
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Current configuration"));
}
