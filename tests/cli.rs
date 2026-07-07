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

fn init(dir: &TempDir) {
    bb(dir).arg("init").assert().success();
}

#[test]
fn init_creates_private_project_store() {
    let dir = repo();

    bb(&dir).arg("init").assert().success().stdout(
        predicate::str::contains("Initialized Backburner")
            .and(predicate::str::contains(".backburner/backburner.db")),
    );

    assert!(dir.path().join(".backburner/backburner.db").exists());
    assert!(dir.path().join(".backburner/settings.json").exists());
    let exclude = std::fs::read_to_string(dir.path().join(".git/info/exclude")).unwrap();
    assert!(exclude.lines().any(|line| line == ".backburner/"));
}

#[test]
fn init_json_reports_project_paths() {
    let dir = repo();

    let value = json_output(bb(&dir).args(["init", "--json"]).assert());
    let root = dir.path().canonicalize().expect("canonical root");

    assert_eq!(value["root"], root.to_string_lossy().as_ref());
    assert!(
        value["database"]
            .as_str()
            .unwrap()
            .ends_with(".backburner/backburner.db")
    );
}

#[test]
fn add_defaults_to_today_and_backburner_flag_defers() {
    let dir = repo();
    init(&dir);

    bb(&dir).args(["add", "Do this now"]).assert().success();
    bb(&dir)
        .args(["add", "Remember this", "--backburner"])
        .assert()
        .success();

    bb(&dir)
        .arg("backburner")
        .assert()
        .success()
        .stdout(predicate::str::contains("#2 Remember this"));
    bb(&dir)
        .arg("today")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 Do this now"));
}

#[test]
fn done_and_finish_session_preserves_backburner_rules() {
    let dir = repo();
    init(&dir);
    bb(&dir).args(["add", "Complete me"]).assert().success();
    bb(&dir).args(["add", "Carry me over"]).assert().success();

    bb(&dir).args(["done", "1"]).assert().success();
    bb(&dir).arg("today").assert().success().stdout(
        predicate::str::contains("[x] #1 Complete me")
            .and(predicate::str::contains("[ ] #2 Carry me over")),
    );

    let result = json_output(bb(&dir).args(["finish-session", "--json"]).assert());
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
    init(&dir);
    let tomorrow = (Local::now().date_naive() + Days::new(1))
        .format("%Y-%m-%d")
        .to_string();

    bb(&dir)
        .args(["add", "Due now", "--backburner", "--plan", "today"])
        .assert()
        .success();
    bb(&dir)
        .args(["add", "Due later", "--backburner", "--plan", &tomorrow])
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
fn promoted_unfinished_tasks_stay_backburnered_after_finish_session() {
    let dir = repo();
    init(&dir);

    bb(&dir)
        .args([
            "add",
            "Due but unfinished",
            "--backburner",
            "--plan",
            "today",
        ])
        .assert()
        .success();
    bb(&dir)
        .arg("today")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 Due but unfinished"));

    let result = json_output(bb(&dir).args(["finish-session", "--json"]).assert());
    assert_eq!(result["archived"], 0);
    assert_eq!(result["backburnered"], 1);

    bb(&dir)
        .arg("today")
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing here."));
    bb(&dir)
        .arg("backburner")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 Due but unfinished"));
}

#[test]
fn finish_day_remains_supported_as_alias() {
    let dir = repo();
    init(&dir);

    bb(&dir).args(["add", "Alias complete"]).assert().success();
    bb(&dir).args(["done", "1"]).assert().success();

    let result = json_output(bb(&dir).args(["finish-day", "--json"]).assert());
    assert_eq!(result["archived"], 1);
    assert_eq!(result["backburnered"], 0);
}

#[test]
fn stale_settings_roll_today_tasks_into_archive_and_backburner() {
    let dir = repo();
    init(&dir);
    bb(&dir)
        .args(["add", "Archive tomorrow"])
        .assert()
        .success();
    bb(&dir).args(["add", "Defer tomorrow"]).assert().success();
    bb(&dir).args(["done", "1"]).assert().success();

    let yesterday = Local::now()
        .date_naive()
        .checked_sub_days(Days::new(1))
        .expect("yesterday")
        .format("%Y-%m-%d")
        .to_string();
    std::fs::write(
        dir.path().join(".backburner/settings.json"),
        format!("{{\"lastRolloverDate\":\"{yesterday}\"}}\n"),
    )
    .expect("write settings");

    bb(&dir)
        .arg("today")
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing here."));

    bb(&dir)
        .arg("archive")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 Archive tomorrow"));
    bb(&dir)
        .arg("backburner")
        .assert()
        .success()
        .stdout(predicate::str::contains("#2 Defer tomorrow"));

    let settings = std::fs::read_to_string(dir.path().join(".backburner/settings.json")).unwrap();
    assert!(settings.contains(&format!(
        "\"lastRolloverDate\": \"{}\"",
        Local::now().format("%Y-%m-%d")
    )));
}

#[test]
fn show_json_includes_evidence() {
    let dir = repo();
    init(&dir);
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
fn list_commands_support_json_output() {
    let dir = repo();
    init(&dir);
    bb(&dir).args(["add", "Today item"]).assert().success();
    bb(&dir)
        .args(["add", "Backburner item", "--backburner"])
        .assert()
        .success();
    bb(&dir).args(["add", "Archived item"]).assert().success();
    bb(&dir).args(["move", "3", "archive"]).assert().success();

    let today = json_output(bb(&dir).args(["today", "--json"]).assert());
    assert_eq!(today.as_array().unwrap().len(), 1);
    assert_eq!(today[0]["task"]["title"], "Today item");

    let backburner = json_output(bb(&dir).args(["backburner", "--json"]).assert());
    assert_eq!(backburner.as_array().unwrap().len(), 1);
    assert_eq!(backburner[0]["task"]["title"], "Backburner item");

    let archive = json_output(bb(&dir).args(["archive", "--json"]).assert());
    assert_eq!(archive.as_array().unwrap().len(), 1);
    assert_eq!(archive[0]["task"]["title"], "Archived item");
}

#[test]
fn show_human_output_includes_task_details() {
    let dir = repo();
    init(&dir);
    bb(&dir)
        .args([
            "add",
            "Inspect output",
            "--file",
            "src/main.rs:10",
            "--cmd",
            "cargo test",
            "--note",
            "Remember the edge case.",
        ])
        .assert()
        .success();

    bb(&dir)
        .args(["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 Inspect output"))
        .stdout(predicate::str::contains("Status: today"))
        .stdout(predicate::str::contains("Remember the edge case."))
        .stdout(predicate::str::contains("src/main.rs:10"))
        .stdout(predicate::str::contains("cargo test"));
}

#[test]
fn note_plan_move_undone_and_delete_commands_mutate_tasks() {
    let dir = repo();
    init(&dir);
    bb(&dir).args(["add", "Mutable task"]).assert().success();

    bb(&dir)
        .args(["note", "1", "Added after creation.", "--source", "agent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added note to #1."));
    bb(&dir)
        .args(["plan", "1", "tomorrow"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated plan for #1."));
    bb(&dir)
        .args(["move", "1", "backburner"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Moved #1 to backburner."));
    bb(&dir).args(["done", "1"]).assert().success();
    bb(&dir)
        .args(["undone", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Marked #1 undone in backburner."));

    let value = json_output(bb(&dir).args(["show", "1", "--json"]).assert());
    let tomorrow = (Local::now().date_naive() + Days::new(1))
        .format("%Y-%m-%d")
        .to_string();
    assert_eq!(value["task"]["status"], "backburner");
    assert_eq!(value["task"]["plannedDateKey"], tomorrow);
    assert_eq!(value["task"]["completedAt"], Value::Null);
    assert_eq!(value["notes"][0]["body"], "Added after creation.");
    assert_eq!(value["notes"][0]["source"], "agent");

    bb(&dir).args(["plan", "1", "none"]).assert().success();
    let unplanned = json_output(bb(&dir).args(["show", "1", "--json"]).assert());
    assert_eq!(unplanned["task"]["plannedDateKey"], Value::Null);

    bb(&dir)
        .args(["delete", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted #1."));
    bb(&dir)
        .args(["show", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("task #1 not found"));
}

#[test]
fn undone_restores_archived_tasks_to_backburner() {
    let dir = repo();
    init(&dir);
    bb(&dir)
        .args(["add", "Revive this later"])
        .assert()
        .success();
    bb(&dir).args(["done", "1"]).assert().success();
    bb(&dir).arg("finish-session").assert().success();

    bb(&dir)
        .args(["undone", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Marked #1 undone in backburner."));

    bb(&dir)
        .arg("archive")
        .assert()
        .success()
        .stdout(predicate::str::contains("Revive this later").not());

    bb(&dir)
        .arg("backburner")
        .assert()
        .success()
        .stdout(predicate::str::contains("[ ] #1 Revive this later"));
}

#[test]
fn move_supports_archived_alias() {
    let dir = repo();
    init(&dir);
    bb(&dir).args(["add", "Alias task"]).assert().success();

    bb(&dir)
        .args(["move", "1", "archived"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Moved #1 to archived."));

    bb(&dir)
        .arg("archive")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 Alias task"));
}

#[test]
fn emoji_commands_normalize_to_existing_commands() {
    let dir = repo();
    init(&dir);

    bb(&dir)
        .args(["➕", "Today by emoji", "☀️"])
        .assert()
        .success();
    bb(&dir)
        .args(["add", "Deferred by emoji", "🔥"])
        .assert()
        .success();

    bb(&dir)
        .arg("☀️")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 Today by emoji"))
        .stdout(predicate::str::contains("Deferred by emoji").not());
    bb(&dir)
        .arg("🔥")
        .assert()
        .success()
        .stdout(predicate::str::contains("#2 Deferred by emoji"));

    bb(&dir)
        .args(["🚚", "1", "🔥"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Moved #1 to backburner."));
    bb(&dir)
        .args(["📅", "2", "☀️"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated plan for #2."));

    bb(&dir)
        .arg("📋")
        .assert()
        .success()
        .stdout(predicate::str::contains("Today"))
        .stdout(predicate::str::contains("Backburner"));
    bb(&dir)
        .args(["👁️", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 Today by emoji"));
    bb(&dir)
        .args(["📝", "1", "Emoji note"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added note to #1."));
    bb(&dir)
        .args(["✅", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Marked #1 done."));
    bb(&dir)
        .args(["🟩", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Marked #1 undone in backburner."));
    bb(&dir)
        .args(["🚚", "1", "🗄️"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Moved #1 to archived."));
    bb(&dir)
        .arg("🗄️")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 Today by emoji"));
    bb(&dir).args(["+", "Finish by emoji"]).assert().success();
    bb(&dir).args(["✅", "3"]).assert().success();
    bb(&dir)
        .arg("🌇")
        .assert()
        .success()
        .stdout(predicate::str::contains("Archived 1 completed task(s)."));
}

#[test]
fn context_json_promotes_due_items_and_includes_backburner() {
    let dir = repo();
    init(&dir);
    bb(&dir).args(["add", "Active"]).assert().success();
    bb(&dir)
        .args(["add", "Due memory", "--backburner", "--plan", "today"])
        .assert()
        .success();
    bb(&dir)
        .args(["add", "Later memory", "--backburner"])
        .assert()
        .success();

    let value = json_output(bb(&dir).args(["context", "--json"]).assert());
    assert_eq!(value["promoted"], 1);
    assert_eq!(value["today"].as_array().unwrap().len(), 2);
    assert_eq!(value["backburner"].as_array().unwrap().len(), 1);
}

#[test]
fn context_human_output_includes_promotions_and_backburner_sample() {
    let dir = repo();
    init(&dir);
    bb(&dir)
        .args(["add", "Due context", "--backburner", "--plan", "today"])
        .assert()
        .success();
    bb(&dir)
        .args(["add", "Deferred context", "--backburner"])
        .assert()
        .success();

    bb(&dir)
        .arg("context")
        .assert()
        .success()
        .stdout(predicate::str::contains("Promoted 1 planned task(s)."))
        .stdout(predicate::str::contains("Today"))
        .stdout(predicate::str::contains("#1 Due context"))
        .stdout(predicate::str::contains("Backburner"))
        .stdout(predicate::str::contains("#2 Deferred context"));
}

#[test]
fn help_describes_commands_and_options() {
    let dir = repo();

    bb(&dir)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "add             Add a task to Today",
        ))
        .stdout(predicate::str::contains(
            "finish-session  Archive completed Today tasks and defer unfinished ones",
        ))
        .stdout(predicate::str::contains(
            "undone          Mark a task incomplete and revive archived tasks",
        ))
        .stdout(predicate::str::contains(
            "context         Print context: Today and Backburner tasks",
        ))
        .stdout(predicate::str::contains("prompt          Print").not())
        .stdout(predicate::str::contains(
            "help            Print command help or advanced usage examples",
        ))
        .stdout(predicate::str::contains(
            "Run `bb help --usage` for multiline workflow examples.",
        ));

    bb(&dir)
        .args(["add", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "--backburner       Put the new task in Backburner instead of Today",
        ))
        .stdout(predicate::str::contains(
            "--cmd <COMMANDS>   Command evidence for restarting or verifying work",
        ));

    let plain_dir = tempfile::tempdir().expect("temp dir");
    bb(&plain_dir)
        .args(["help", "--usage"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Backburner advanced usage"))
        .stdout(predicate::str::contains(
            "bb add \"Fix auth redirect regression\" \\",
        ))
        .stdout(predicate::str::contains("bb context --json"));

    bb(&plain_dir)
        .arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: bb <COMMAND>"));
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

#[test]
fn commands_fail_outside_git_repository() {
    let dir = tempfile::tempdir().expect("temp dir");

    bb(&dir)
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not inside a git repository"));
}

#[test]
fn invalid_inputs_return_useful_errors() {
    let dir = repo();
    init(&dir);
    bb(&dir).args(["add", "Valid task"]).assert().success();

    bb(&dir)
        .args(["add", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("title cannot be empty"));
    bb(&dir)
        .args(["note", "1", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("note cannot be empty"));
    bb(&dir)
        .args(["plan", "1", "next-week"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "plan must be today, tomorrow, none, or YYYY-MM-DD",
        ));
    bb(&dir)
        .args(["show", "404"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("task #404 not found"));
}

#[test]
fn clap_rejects_invalid_enums() {
    let dir = repo();
    init(&dir);
    bb(&dir).args(["add", "Valid task"]).assert().success();

    bb(&dir)
        .args(["move", "1", "later"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'later'"));
    bb(&dir)
        .args(["note", "1", "Hello", "--source", "robot"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'robot'"));
}
