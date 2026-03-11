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
    let feature_path = repo.create_workspace("feature-auth");
    let bugfix_path = repo.create_workspace("bugfix-api");
    repo.run(&["describe", "-m", "Default workspace"]);
    TempJjRepo::run_at(&feature_path, &["describe", "-m", "Feature auth work"]);
    TempJjRepo::run_at(&bugfix_path, &["describe", "-m", "Bugfix api work"]);

    command("navi")
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("marker"))
        .stdout(predicate::str::contains("@"))
        .stdout(predicate::str::contains("default"))
        .stdout(predicate::str::contains("Feature auth work"))
        .stdout(predicate::str::contains("Bugfix api work"))
        .stdout(predicate::str::contains("commit"))
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
fn list_survives_missing_workspace_directory() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");
    let feature_name = feature_path.file_name().expect("workspace dir name");
    let mut moved_name = feature_name.to_os_string();
    moved_name.push(".moved");
    let moved_path = feature_path.with_file_name(moved_name);
    std::fs::rename(&feature_path, &moved_path).expect("move workspace dir");

    command("navi")
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("feature-auth"));
}

#[test]
fn remove_named_workspace_forgets_workspace_and_keeps_directory() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");

    command("navi")
        .current_dir(repo.path())
        .args(["remove", "feature-auth"])
        .assert()
        .success()
        .stdout(predicate::str::contains("forgot workspace 'feature-auth'"));

    assert!(feature_path.is_dir());
    assert!(!repo.run(&["workspace", "list"]).contains("feature-auth"));
}

#[test]
fn remove_without_name_forgets_current_workspace() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");

    command("navi")
        .current_dir(&feature_path)
        .args(["remove"])
        .assert()
        .success()
        .stdout(predicate::str::contains("forgot workspace 'feature-auth'"));

    assert!(feature_path.is_dir());
    assert!(!repo.run(&["workspace", "list"]).contains("feature-auth"));
}

#[test]
fn remove_missing_workspace_fails_with_useful_error() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["remove", "does-not-exist"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "error: workspace 'does-not-exist' does not exist",
        ));
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

#[test]
fn navi_help_uses_navi_name() {
    command("navi")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: navi"));
}

#[test]
fn nv_help_uses_nv_name() {
    command("nv")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: nv"));
}
