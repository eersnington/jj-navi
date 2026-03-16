mod common;

use predicates::prelude::*;

use common::{TempJjRepo, command};

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
    assert!(repo.navi_config_path().is_file());
    assert!(
        std::fs::read_to_string(repo.navi_config_path())
            .expect("read navi config")
            .contains("workspace_template = \"../{repo}.{workspace}\"")
    );
    assert!(repo.navi_metadata_path().is_file());
    let metadata = std::fs::read_to_string(repo.navi_metadata_path()).expect("read navi metadata");
    assert!(metadata.contains("name = \"feature-auth\""));
    assert!(metadata.contains("path = \""));
    assert!(metadata.contains("created_by_navi = true"));
    assert!(metadata.contains("template = \"../{repo}.{workspace}\""));
    assert!(metadata.contains("revision = \"\""));
    assert!(metadata.contains("created_at = \""));
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
fn switch_create_uses_configured_workspace_template() {
    let repo = TempJjRepo::new();
    repo.write_navi_config("workspace_template = \"../{repo}-{workspace}\"\n");
    let expected_path = repo
        .path()
        .with_file_name(format!("{}-feature-auth", repo.repo_name()));

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success()
        .stdout(predicate::eq(format!(
            "../{}-feature-auth\n",
            repo.repo_name()
        )));

    assert!(expected_path.is_dir());
}

#[test]
fn switch_create_preserves_literal_placeholder_text_in_repo_name() {
    let repo = TempJjRepo::new_with_prefix("repo{workspace}.");
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
fn switch_fails_for_forgotten_workspace_even_if_directory_remains() {
    let repo = TempJjRepo::new();
    let workspace_path = repo.create_workspace("feature-auth");

    command("navi")
        .current_dir(repo.path())
        .args(["remove", "feature-auth"])
        .assert()
        .success();

    assert!(workspace_path.is_dir());

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "feature-auth"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error: workspace does not exist"));
}

#[test]
fn switch_uses_actual_jj_workspace_path_for_existing_workspace() {
    let repo = TempJjRepo::new();
    let custom_path = repo
        .path()
        .with_file_name(format!("{}-custom-feature-auth", repo.repo_name()));
    repo.create_workspace_at("feature-auth", &custom_path);

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "feature-auth"])
        .assert()
        .success()
        .stdout(predicate::eq(format!(
            "../{}\n",
            custom_path
                .file_name()
                .expect("custom workspace dir")
                .to_string_lossy()
        )));
}

#[test]
fn switch_uses_metadata_fallback_when_jj_workspace_path_is_missing() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success();

    repo.clear_workspace_store_index();

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "feature-auth"])
        .assert()
        .success()
        .stdout(predicate::eq(format!(
            "../{}.feature-auth\n",
            repo.repo_name()
        )))
        .stderr(predicate::str::is_empty());
}

#[test]
fn switch_uses_template_fallback_with_warning_when_metadata_is_absent() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");
    repo.clear_workspace_store_index();

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "feature-auth"])
        .assert()
        .success()
        .stdout(predicate::eq(format!(
            "../{}.feature-auth\n",
            repo.repo_name()
        )))
        .stderr(predicate::str::contains(
            "warning: jj could not resolve this workspace path; using navi fallback",
        ));
}

#[test]
fn switch_uses_repo_primary_root_for_default_when_jj_path_is_missing() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success();

    let feature_path = repo
        .path()
        .with_file_name(format!("{}.feature-auth", repo.repo_name()));
    repo.clear_workspace_store_index();

    command("navi")
        .current_dir(&feature_path)
        .args(["switch", "default"])
        .assert()
        .success()
        .stdout(predicate::eq(format!("../{}\n", repo.repo_name())))
        .stderr(predicate::str::is_empty());
}

#[test]
fn switch_uses_repo_primary_root_for_default_from_nested_secondary_directory() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success();

    let feature_path = repo
        .path()
        .with_file_name(format!("{}.feature-auth", repo.repo_name()));
    let nested_path = feature_path.join("nested").join("dir");
    std::fs::create_dir_all(&nested_path).expect("create nested path");
    repo.clear_workspace_store_index();

    command("navi")
        .current_dir(&nested_path)
        .args(["switch", "default"])
        .assert()
        .success()
        .stdout(predicate::eq(format!("../../../{}\n", repo.repo_name())))
        .stderr(predicate::str::is_empty());
}

#[test]
fn switch_fails_with_last_known_path_when_fallback_directory_is_missing() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success();

    repo.clear_workspace_store_index();

    let feature_path =
        repo.path()
            .with_file_name(format!("{}.{}", repo.repo_name(), "feature-auth"));
    std::fs::remove_dir_all(&feature_path).expect("remove workspace dir");

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "feature-auth"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "error: workspace 'feature-auth' exists, but its directory could not be resolved",
        ))
        .stderr(predicate::str::contains(format!(
            "hint: last known path: ../{}.feature-auth",
            repo.repo_name()
        )));
}

#[test]
fn malformed_repo_config_fails_config_dependent_command() {
    let repo = TempJjRepo::new();
    repo.write_navi_config("workspace_template = \"../{repo\"\n");

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            repo.navi_config_path().display().to_string(),
        ));
}

#[test]
fn switch_writes_cd_directive_when_shell_integration_is_active() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");
    let directive_dir = tempfile::TempDir::new().expect("temp directive dir");
    let directive_file = directive_dir.path().join("navi-directives.sh");

    command("navi")
        .current_dir(repo.path())
        .env("NAVI_DIRECTIVE_FILE", &directive_file)
        .args(["switch", "feature-auth"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let contents = std::fs::read_to_string(directive_file).expect("read directive file");
    assert_eq!(
        contents,
        format!("cd -- '../{}.feature-auth'\n", repo.repo_name())
    );
}

#[test]
fn switch_writes_shell_escaped_directive_for_special_paths() {
    let repo = TempJjRepo::new();
    repo.write_navi_config("workspace_template = \"../{repo}.space {workspace}'s\"\n");

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success();

    let directive_dir = tempfile::TempDir::new().expect("temp directive dir");
    let directive_file = directive_dir.path().join("navi-directives.sh");
    command("navi")
        .current_dir(repo.path())
        .env("NAVI_DIRECTIVE_FILE", &directive_file)
        .args(["switch", "feature-auth"])
        .assert()
        .success();

    let contents = std::fs::read_to_string(directive_file).expect("read directive file");
    assert_eq!(
        contents,
        format!("cd -- '../{}.space feature-auth'\\''s'\n", repo.repo_name())
    );
}
