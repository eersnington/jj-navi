mod common;

use predicates::prelude::*;
use serde_json::Value;
use std::path::Path;

use common::{TempJjRepo, command, command_output};

fn parse_merge_preview_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("parse merge preview json")
}

fn all_commit_ids(repo: &TempJjRepo) -> String {
    repo.run(&[
        "log",
        "-r",
        "all()",
        "--no-graph",
        "-T",
        "commit_id ++ \"\\n\"",
    ])
}

fn working_copy_commit_id_without_snapshot(path: &Path) -> String {
    let output = std::process::Command::new("jj")
        .args([
            "--ignore-working-copy",
            "log",
            "-r",
            "@",
            "--no-graph",
            "-T",
            "commit_id",
        ])
        .current_dir(path)
        .output()
        .expect("run jj log");

    assert!(output.status.success(), "jj log failed");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

#[test]
fn merge_preview_defaults_target_to_current_workspace() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-a");

    command("navi")
        .current_dir(repo.path())
        .args(["merge", "--from", "feature-a"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Merge recommendation"))
        .stdout(predicate::str::contains("source: feature-a"))
        .stdout(predicate::str::contains("target: default"))
        .stdout(predicate::str::contains("change "))
        .stdout(predicate::str::contains("commit "))
        .stdout(predicate::str::contains("path: "))
        .stdout(predicate::str::contains("health: "))
        .stdout(predicate::str::contains("jj duplicate "))
        .stdout(predicate::str::contains("jj rebase -s <duplicate> -d "))
        .stdout(predicate::str::contains(
            "Use the change ID printed by `jj duplicate` for `<duplicate>`.",
        ));
}

#[test]
fn merge_preview_resolves_explicit_target_workspace() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-a");
    repo.create_workspace("integration");

    command("navi")
        .current_dir(repo.path())
        .args(["merge", "--from", "feature-a", "--into", "integration"])
        .assert()
        .success()
        .stdout(predicate::str::contains("source: feature-a"))
        .stdout(predicate::str::contains("target: integration"));
}

#[test]
fn merge_preview_json_includes_workspace_details_and_commands() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-a");

    let output = command_output(
        "navi",
        repo.path(),
        &["merge", "--from", "feature-a", "--json"],
    );

    assert!(output.status.success(), "merge preview json failed");
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("\n  \"source\": {\n"));

    let json = parse_merge_preview_json(&output);
    assert_eq!(json["source"]["name"], "feature-a");
    assert_eq!(json["target"]["name"], "default");
    assert!(
        json["source"]["commit_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
    assert!(
        json["source"]["change_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
    assert_eq!(json["source"]["path"]["state"], "confirmed");
    assert_eq!(json["source"]["freshness"]["status"], "current");
    assert!(
        json["commands"][0]
            .as_str()
            .expect("first command")
            .starts_with("jj duplicate ")
    );
    assert!(
        json["commands"][1]
            .as_str()
            .expect("second command")
            .starts_with("jj rebase -s <duplicate> -d ")
    );
}

#[test]
fn merge_preview_fails_for_missing_source_workspace() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["merge", "--from", "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "error: merge source workspace 'missing' does not exist",
        ));
}

#[test]
fn merge_preview_fails_for_missing_target_workspace() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-a");

    command("navi")
        .current_dir(repo.path())
        .args(["merge", "--from", "feature-a", "--into", "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "error: merge target workspace 'missing' does not exist",
        ));
}

#[test]
fn merge_preview_fails_for_same_source_and_target() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["merge", "--from", "default"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "error: cannot merge workspace 'default' into itself",
        ));
}

#[test]
fn merge_preview_refuses_stale_source_workspace_path() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-a");
    let moved_path = feature_path.with_file_name(format!(
        "{}.moved",
        feature_path
            .file_name()
            .expect("workspace dir name")
            .to_string_lossy()
    ));
    std::fs::rename(&feature_path, &moved_path).expect("move workspace dir");
    std::fs::create_dir(&feature_path).expect("create stale workspace dir");

    command("navi")
        .current_dir(repo.path())
        .args(["merge", "--from", "feature-a"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "error: merge source workspace 'feature-a' is not ready: workspace path is stale",
        ));
}

#[test]
fn merge_preview_refuses_stale_target_workspace_path() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-a");
    let target_path = repo.create_workspace("integration");
    let moved_path = target_path.with_file_name(format!(
        "{}.moved",
        target_path
            .file_name()
            .expect("workspace dir name")
            .to_string_lossy()
    ));
    std::fs::rename(&target_path, &moved_path).expect("move workspace dir");
    std::fs::create_dir(&target_path).expect("create stale workspace dir");

    command("navi")
        .current_dir(repo.path())
        .args(["merge", "--from", "feature-a", "--into", "integration"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "error: merge target workspace 'integration' is not ready: workspace path is stale",
        ));
}

#[test]
fn merge_preview_does_not_create_or_rebase_commits() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-a");
    let before = all_commit_ids(&repo);

    command("navi")
        .current_dir(repo.path())
        .args(["merge", "--from", "feature-a"])
        .assert()
        .success();

    let after = all_commit_ids(&repo);
    assert_eq!(after, before);
}

#[test]
fn merge_preview_does_not_snapshot_dirty_source_workspace() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-a");
    std::fs::write(feature_path.join("dirty.txt"), "unsnapshotted\n").expect("write dirty file");
    let before = working_copy_commit_id_without_snapshot(&feature_path);

    command("navi")
        .current_dir(repo.path())
        .args(["merge", "--from", "feature-a"])
        .assert()
        .success();

    let after = working_copy_commit_id_without_snapshot(&feature_path);
    assert_eq!(after, before);
}
