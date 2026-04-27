mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use predicates::prelude::*;
use serde_json::Value;

use common::{TempJjRepo, command, command_output};

fn parse_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("parse list json")
}

fn workspace_by_name<'a>(json: &'a Value, name: &str) -> &'a Value {
    json["workspaces"]
        .as_array()
        .expect("workspaces array")
        .iter()
        .find(|workspace| workspace["name"] == name)
        .expect("workspace entry")
}

#[test]
fn list_reports_large_real_diff_stats_instead_of_unknown() {
    let repo = TempJjRepo::new();

    for index in 0..6000 {
        fs::write(repo.path().join(format!("file-{index}.txt")), "x\n").expect("write file");
    }

    let output = command_output("navi", repo.path(), &["list", "--json"]);
    assert!(output.status.success());

    let json = parse_json(&output);
    let workspace = workspace_by_name(&json, "default");

    assert_eq!(workspace["diff"]["status"], "available");
    assert_eq!(workspace["diff"]["files_changed"], 6000);
    assert_eq!(workspace["diff"]["insertions"], 6000);
    assert_eq!(workspace["diff"]["deletions"], 0);
}

#[test]
fn list_handles_current_workspace_snapshot_failure_without_failing_discovery() {
    let repo = TempJjRepo::new();
    let unreadable = repo.path().join("unreadable.txt");
    fs::write(&unreadable, "secret\n").expect("write unreadable file");
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000))
        .expect("make file unreadable");

    let output = command_output("navi", repo.path(), &["list"]);

    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o644))
        .expect("restore unreadable file permissions");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("not-current"));
    assert!(!stdout.contains("[ ok ] [ not-current ]"));
}

#[test]
fn list_makes_secondary_workspace_current_before_rendering() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("agent-a");
    std::fs::write(feature_path.join("agent.txt"), "agent work\n").expect("write agent file");

    command("navi")
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("agent-a"))
        .stdout(predicate::str::contains("1f +1 -0"))
        .stdout(predicate::str::contains("not-current").not());
}

#[test]
fn list_json_includes_currentness_diff_and_age() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "agent-a"])
        .assert()
        .success();

    let feature_path = repo
        .path()
        .with_file_name(format!("{}.agent-a", repo.repo_name()));
    std::fs::write(feature_path.join("agent.txt"), "agent work\n").expect("write agent file");

    let output = command_output("navi", repo.path(), &["list", "--json"]);
    assert!(output.status.success());

    let json = parse_json(&output);
    let workspace = workspace_by_name(&json, "agent-a");

    assert_eq!(workspace["freshness"]["status"], "current");
    assert_eq!(workspace["diff"]["status"], "available");
    assert_eq!(workspace["diff"]["files_changed"], 1);
    assert_eq!(workspace["diff"]["insertions"], 1);
    assert_eq!(workspace["diff"]["deletions"], 0);
    assert!(workspace["age"]["created_at"].is_string());
    assert!(workspace["age"]["display"].is_string());
}

#[test]
fn list_json_uses_null_age_for_jj_only_workspace() {
    let repo = TempJjRepo::new();
    repo.create_workspace("agent-a");

    let output = command_output("navi", repo.path(), &["list", "--json"]);
    assert!(output.status.success());

    let json = parse_json(&output);
    let workspace = workspace_by_name(&json, "agent-a");

    assert!(workspace["age"]["created_at"].is_null());
    assert!(workspace["age"]["display"].is_null());
}

#[test]
fn nv_list_uses_fresh_list_output() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("agent-a");
    std::fs::write(feature_path.join("agent.txt"), "agent work\n").expect("write agent file");

    command("nv")
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("agent-a"))
        .stdout(predicate::str::contains("1f +1 -0"));
}
