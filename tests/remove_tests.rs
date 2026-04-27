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
        .args(["remove", "feature-auth", "--yes"])
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
fn remove_named_workspace_forgets_workspace_and_deletes_directory() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");

    command("navi")
        .current_dir(repo.path())
        .args(["remove", "feature-auth", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("forgot workspace 'feature-auth'"))
        .stdout(predicate::str::contains("deleted workspace directory"));

    assert!(!feature_path.exists());
    assert!(!repo.run(&["workspace", "list"]).contains("feature-auth"));
}

#[test]
fn remove_with_short_yes_deletes_directory_without_prompt() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");

    command("navi")
        .current_dir(repo.path())
        .args(["remove", "feature-auth", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted workspace directory"))
        .stdout(predicate::str::contains("Type 'yes'").not());

    assert!(!feature_path.exists());
    assert!(!repo.run(&["workspace", "list"]).contains("feature-auth"));
}

#[test]
fn remove_without_yes_requires_confirmation_and_deletes_on_yes() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");
    let feature_path_display = feature_path.display().to_string();

    command("navi")
        .current_dir(repo.path())
        .args(["remove", "feature-auth"])
        .write_stdin("yes\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "permanently remove workspace 'feature-auth'",
        ))
        .stdout(predicate::str::contains(feature_path_display))
        .stdout(predicate::str::contains("Type 'yes' to continue:"))
        .stdout(predicate::str::contains("deleted workspace directory"));

    assert!(!feature_path.exists());
    assert!(!repo.run(&["workspace", "list"]).contains("feature-auth"));
}

#[test]
fn remove_without_yes_cancels_when_confirmation_is_not_yes() {
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
        .current_dir(repo.path())
        .args(["remove", "feature-auth"])
        .write_stdin("n\n")
        .assert()
        .failure()
        .stdout(predicate::str::contains("Directory to delete:"))
        .stderr(predicate::str::contains("error: remove cancelled"));

    assert!(feature_path.is_dir());
    assert!(repo.run(&["workspace", "list"]).contains("feature-auth"));
    let metadata = std::fs::read_to_string(repo.navi_metadata_path()).expect("read navi metadata");
    assert!(metadata.contains("feature-auth"));
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
        .args(["remove", "feature-auth", "--yes"])
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
fn remove_refuses_workspace_that_owns_shared_repo_storage() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");

    command("navi")
        .current_dir(&feature_path)
        .args(["remove", "default", "--yes"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("contains shared jj repo storage"));

    assert!(repo.path().is_dir());
    assert!(repo.path().join(".jj").join("repo").is_dir());
    TempJjRepo::run_at(&feature_path, &["status"]);
}

#[test]
fn remove_refuses_workspace_that_owns_shared_repo_storage_with_fallback_path() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");
    repo.clear_workspace_store_index();

    command("navi")
        .current_dir(&feature_path)
        .args(["remove", "default", "--yes"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("contains shared jj repo storage"));

    assert!(repo.path().is_dir());
    assert!(repo.path().join(".jj").join("repo").is_dir());
    TempJjRepo::run_at(&feature_path, &["status"]);
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
fn remove_refuses_workspace_when_directory_cannot_be_validated() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");
    std::fs::remove_dir_all(&feature_path).expect("remove workspace dir");

    command("navi")
        .current_dir(repo.path())
        .args(["remove", "feature-auth", "--yes"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "workspace 'feature-auth' exists, but its directory could not be resolved",
        ));

    assert!(repo.run(&["workspace", "list"]).contains("feature-auth"));
}
