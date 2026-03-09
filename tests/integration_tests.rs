mod common;

use assert_cmd::Command;
use predicates::prelude::*;

use common::TempJjRepo;

fn command(bin: &str) -> Command {
    match bin {
        "navi" => Command::new(assert_cmd::cargo::cargo_bin!("navi")),
        "nv" => Command::new(assert_cmd::cargo::cargo_bin!("nv")),
        _ => panic!("unknown bin: {bin}"),
    }
}

#[test]
fn switch_existing_prints_relative_path() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "feature-auth"])
        .assert()
        .success()
        .stdout(predicate::eq(format!(
            "../{}.feature-auth\n",
            repo.repo_name()
        )));
}

#[test]
fn switch_create_creates_workspace() {
    let repo = TempJjRepo::new();
    let expected_path =
        repo.path()
            .with_file_name(format!("{}.{}", repo.repo_name(), "feature-auth"));

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success()
        .stdout(predicate::eq(format!(
            "../{}.feature-auth\n",
            repo.repo_name()
        )));

    assert!(expected_path.is_dir());
}

#[test]
fn switch_create_with_revision_uses_requested_parent() {
    let repo = TempJjRepo::new();
    let expected_parent = repo.rev_id("@");
    let workspace_path =
        repo.path()
            .with_file_name(format!("{}.{}", repo.repo_name(), "feature-auth"));

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth", "--revision", "@"])
        .assert()
        .success();

    let created_parent = std::process::Command::new("jj")
        .args(["log", "-r", "@-", "--no-graph", "-T", "commit_id"])
        .current_dir(&workspace_path)
        .output()
        .expect("run jj log");

    assert!(created_parent.status.success(), "jj log failed");
    assert_eq!(
        String::from_utf8_lossy(&created_parent.stdout).trim(),
        expected_parent
    );
}

#[test]
fn list_prints_workspace_table() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");
    repo.create_workspace("bugfix-api");

    command("navi")
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("workspace"))
        .stdout(predicate::str::contains("default"))
        .stdout(predicate::str::contains(format!(
            "../{}.feature-auth",
            repo.repo_name()
        )))
        .stdout(predicate::str::contains(format!(
            "../{}.bugfix-api",
            repo.repo_name()
        )));
}

#[test]
fn nested_workspace_discovery_works_from_secondary_workspace() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");
    let nested_path = feature_path.join("nested").join("dir");
    std::fs::create_dir_all(&nested_path).expect("create nested path");

    command("navi")
        .current_dir(&nested_path)
        .args(["switch", "--create", "bugfix-api"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "{}.bugfix-api",
            repo.repo_name()
        )));
}

#[test]
fn nv_binary_works() {
    let repo = TempJjRepo::new();

    command("nv")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success()
        .stdout(predicate::eq(format!(
            "../{}.feature-auth\n",
            repo.repo_name()
        )));
}
