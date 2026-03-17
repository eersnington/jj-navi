mod common;

use predicates::prelude::*;

use common::{TempJjRepo, command, command_output};

#[test]
fn list_uses_actual_jj_path_after_config_changes() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success();

    repo.write_navi_config("workspace_template = \"../{workspace}\"\n");

    command("navi")
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains(format!(
            "../{}.feature-auth",
            repo.repo_name()
        )))
        .stdout(predicate::str::contains("[ ok ]"))
        .stdout(predicate::str::contains("feature-auth"));
}

#[test]
fn list_uses_actual_jj_workspace_path_for_non_navi_workspace() {
    let repo = TempJjRepo::new();
    let custom_path = repo
        .path()
        .with_file_name(format!("{}-custom-feature-auth", repo.repo_name()));
    repo.create_workspace_at("feature-auth", &custom_path);

    command("navi")
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            custom_path
                .file_name()
                .expect("custom workspace dir")
                .to_string_lossy()
                .to_string(),
        ))
        .stdout(predicate::str::contains("jj-only"))
        .stdout(predicate::str::contains("feature-auth"));
}

#[test]
fn list_uses_repo_primary_root_for_default_when_jj_path_is_missing() {
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
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("default"))
        .stdout(predicate::str::contains(format!("../{}", repo.repo_name())))
        .stdout(predicate::str::contains(format!("../{}.default", repo.repo_name())).not())
        .stdout(predicate::str::contains("[ ok ]"));
}

#[test]
fn list_works_when_metadata_is_absent() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");
    assert!(!repo.navi_metadata_path().exists());

    command("navi")
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("feature-auth"));
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
        .stdout(predicate::str::starts_with("cur"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("@"))
        .stdout(predicate::str::contains("default"))
        .stdout(predicate::str::contains("[ ok ]"))
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
fn list_uses_current_workspace_root_when_jj_workspace_paths_are_missing() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success();

    repo.clear_workspace_store_index();

    command("navi")
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@    default"))
        .stdout(predicate::str::contains("feature-auth"))
        .stdout(predicate::str::contains("[ inferred ]"))
        .stdout(predicate::str::contains(format!(
            "../{}.feature-auth",
            repo.repo_name()
        )));
}

#[test]
fn list_reports_missing_workspace_directory_from_inferred_path() {
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
        .stdout(predicate::str::contains("[ inferred ] [ missing ]"))
        .stdout(predicate::str::contains(format!(
            "../{}.feature-auth",
            repo.repo_name()
        )))
        .stdout(
            predicate::str::contains(format!("../{}.feature-auth [missing]", repo.repo_name()))
                .not(),
        );
}

#[test]
fn list_reports_stale_workspace_directory_from_jj_path_without_inferred_marker() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");
    let feature_name = feature_path.file_name().expect("workspace dir name");
    let mut moved_name = feature_name.to_os_string();
    moved_name.push(".moved");
    let moved_path = feature_path.with_file_name(moved_name);

    std::fs::rename(&feature_path, &moved_path).expect("move workspace dir");
    std::fs::create_dir(&feature_path).expect("create stale workspace dir");

    command("navi")
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[ stale ]"))
        .stdout(predicate::str::contains(format!(
            "../{}.feature-auth",
            repo.repo_name()
        )))
        .stdout(predicate::str::contains("feature-auth"))
        .stdout(predicate::str::contains("[ inferred ] [ stale ]").not());
}

#[test]
fn list_does_not_treat_pathless_metadata_as_jj_only() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");
    repo.write_navi_metadata(
        "[[workspace]]\nname = \"feature-auth\"\ncreated_by_navi = true\ncreated_at = \"2026-03-11T00:00:00Z\"\ntemplate = \"../{repo}.{workspace}\"\nrevision = \"\"\n",
    );

    command("navi")
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("feature-auth"))
        .stdout(predicate::str::contains("jj-only").not());
}

#[test]
fn list_does_not_treat_empty_metadata_path_as_jj_only() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");
    repo.write_navi_metadata(
        "[[workspace]]\nname = \"feature-auth\"\npath = \"\"\ncreated_by_navi = true\ncreated_at = \"2026-03-11T00:00:00Z\"\ntemplate = \"../{repo}.{workspace}\"\nrevision = \"\"\n",
    );

    command("navi")
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("feature-auth"))
        .stdout(predicate::str::contains("jj-only").not());
}

#[test]
fn list_is_stable_from_nested_directory() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");
    let nested_path = repo.path().join("nested").join("dir");
    std::fs::create_dir_all(&nested_path).expect("create nested path");

    let root_output = command_output("navi", repo.path(), &["list"]);
    let nested_output = command_output("navi", &nested_path, &["list"]);

    assert!(root_output.status.success(), "root list failed");
    assert!(nested_output.status.success(), "nested list failed");
    assert_eq!(root_output.stdout, nested_output.stdout);
}

#[test]
fn orphaned_workspace_reports_recovery_error() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");
    repo.run(&["workspace", "forget", "feature-auth"]);

    command("navi")
        .current_dir(&feature_path)
        .args(["list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "error: current directory is no longer a registered jj workspace",
        ))
        .stderr(predicate::str::contains(
            "hint: cd into another workspace or recreate this workspace with jj",
        ));
}
