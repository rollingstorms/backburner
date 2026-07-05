use std::process::Command as StdCommand;

use assert_cmd::Command;
use chrono::{Days, Local};
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn repo() -> TempDir {
    let dir = tempfile::tempdir().expect("temp repo");
    let status = StdCommand::new("git")
        .arg("init")
        .current_dir(dir.path())
        .status()
        .expect("git init");
    assert!(status.success());
    dir
}

fn bb(dir: &TempDir) -> Command {
    let mut command = Command::cargo_bin("bb").expect("bb binary");
    command.current_dir(dir.path());
    command
}

fn json_output(assert: assert_cmd::assert::Assert) -> Value {
    let output = assert.success().get_output().stdout.clone();
    serde_json::from_slice(&output).expect("json output")
}

#[test]
fn init_creates_private_project_store() {
    let dir = repo();

    bb(&dir).arg("init").assert().success().stdout(
        predicate::str::contains("Initialized Backburner")
            .and(predicate::str::contains(".backburner/backburner.db")),
    );

    assert!(dir.path().join(".backburner/backburner.db").exists());
    let exclude = std::fs::read_to_string(dir.path().join(".git/info/exclude")).unwrap();
    assert!(exclude.lines().any(|line| line == ".backburner/"));
}

#[test]
fn add_defaults_to_backburner_and_today_flag_uses_today() {
    let dir = repo();
    bb(&dir).arg("init").assert().success();

    bb(&dir).args(["add", "Remember this"]).assert().success();
    bb(&dir)
        .args(["add", "Do this now", "--today"])
        .assert()
        .success();

    bb(&dir)
        .arg("backburner")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 Remember this"));
    bb(&dir)
        .arg("today")
        .assert()
        .success()
        .stdout(predicate::str::contains("#2 Do this now"));
}

#[test]
fn done_and_finish_day_preserve_backburner_rules() {
    let dir = repo();
    bb(&dir).arg("init").assert().success();
    bb(&dir)
        .args(["add", "Complete me", "--today"])
        .assert()
        .success();
    bb(&dir)
        .args(["add", "Carry me over", "--today"])
        .assert()
        .success();

    bb(&dir).args(["done", "1"]).assert().success();
    bb(&dir).arg("today").assert().success().stdout(
        predicate::str::contains("[x] #1 Complete me")
            .and(predicate::str::contains("[ ] #2 Carry me over")),
    );

    let result = json_output(bb(&dir).args(["finish-day", "--json"]).assert());
    assert_eq!(result["archived"], 1);
    assert_eq!(result["backburnered"], 1);

    bb(&dir)
        .arg("archive")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 Complete me"));
    bb(&dir)
        .arg("backburner")
        .assert()
        .success()
        .stdout(predicate::str::contains("#2 Carry me over"));
}

#[test]
fn planned_backburner_tasks_promote_when_reading_today() {
    let dir = repo();
    bb(&dir).arg("init").assert().success();
    let tomorrow = (Local::now().date_naive() + Days::new(1))
        .format("%Y-%m-%d")
        .to_string();

    bb(&dir)
        .args(["add", "Due now", "--plan", "today"])
        .assert()
        .success();
    bb(&dir)
        .args(["add", "Due later", "--plan", &tomorrow])
        .assert()
        .success();

    bb(&dir)
        .arg("today")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 Due now"))
        .stdout(predicate::str::contains("Due later").not());
    bb(&dir)
        .arg("backburner")
        .assert()
        .success()
        .stdout(predicate::str::contains("#2 Due later"));
}

#[test]
fn show_json_includes_evidence() {
    let dir = repo();
    bb(&dir).arg("init").assert().success();
    let value = json_output(
        bb(&dir)
            .args([
                "add",
                "Fix auth redirect",
                "--file",
                "src/auth.rs:42",
                "--cmd",
                "cargo test auth",
                "--note",
                "Fails after token expiry.",
                "--source",
                "agent",
                "--json",
            ])
            .assert(),
    );
    assert_eq!(value["task"]["id"], 1);
    assert_eq!(value["task"]["source"], "agent");
    assert_eq!(value["notes"][0]["body"], "Fails after token expiry.");
    assert_eq!(value["files"][0]["path"], "src/auth.rs");
    assert_eq!(value["files"][0]["line"], 42);
    assert_eq!(value["commands"][0]["command"], "cargo test auth");

    let shown = json_output(bb(&dir).args(["show", "1", "--json"]).assert());
    assert_eq!(shown["files"][0]["path"], "src/auth.rs");
}

#[test]
fn context_json_promotes_due_items_and_includes_backburner() {
    let dir = repo();
    bb(&dir).arg("init").assert().success();
    bb(&dir)
        .args(["add", "Active", "--today"])
        .assert()
        .success();
    bb(&dir)
        .args(["add", "Due memory", "--plan", "today"])
        .assert()
        .success();
    bb(&dir).args(["add", "Later memory"]).assert().success();

    let value = json_output(bb(&dir).args(["context", "--json"]).assert());
    assert_eq!(value["promoted"], 1);
    assert_eq!(value["today"].as_array().unwrap().len(), 2);
    assert_eq!(value["backburner"].as_array().unwrap().len(), 1);
}

#[test]
fn prompt_prints_bundled_agent_prompt() {
    let dir = repo();

    bb(&dir)
        .args(["prompt", "session-start"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bb context --json"))
        .stdout(predicate::str::contains("Do not expand scope"));
}

#[test]
fn commands_fail_before_init() {
    let dir = repo();

    bb(&dir)
        .arg("today")
        .assert()
        .failure()
        .stderr(predicate::str::contains("run `bb init` first"));
}
