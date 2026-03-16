mod common;

use predicates::prelude::*;

use common::{TempJjRepo, command};

#[test]
fn remove_cleans_up_workspace_metadata() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success();

    command("navi")
        .current_dir(repo.path())
        .args(["remove", "feature-auth"])
        .assert()
        .success();

    let metadata = std::fs::read_to_string(repo.navi_metadata_path()).expect("read navi metadata");
    assert!(!metadata.contains("feature-auth"));
}

#[test]
fn malformed_workspace_metadata_fails_metadata_writing_command() {
    let repo = TempJjRepo::new();
    repo.write_navi_metadata("[[workspace]]\nname = \"feature-auth\"\ncreated_by_navi = true\ncreated_at = \"2026-03-11T00:00:00Z\"\ntemplate = \"../{repo\"\nrevision = \"\"\n");

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "bugfix-api"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            repo.navi_metadata_path().display().to_string(),
        ));

    let expected_path =
        repo.path()
            .with_file_name(format!("{}.{}", repo.repo_name(), "bugfix-api"));
    assert!(!expected_path.exists());
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
fn remove_without_name_requires_explicit_workspace_name() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["remove"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage: navi remove <WORKSPACE>"));
}

#[test]
fn remove_current_workspace_fails_and_keeps_metadata() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success();

    let feature_path =
        repo.path()
            .with_file_name(format!("{}.{}", repo.repo_name(), "feature-auth"));

    command("navi")
        .current_dir(&feature_path)
        .args(["remove", "feature-auth"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "error: cannot remove current workspace",
        ));

    assert!(feature_path.is_dir());
    assert!(repo.run(&["workspace", "list"]).contains("feature-auth"));
    let metadata = std::fs::read_to_string(repo.navi_metadata_path()).expect("read navi metadata");
    assert!(metadata.contains("feature-auth"));
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
