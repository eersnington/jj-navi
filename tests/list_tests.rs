mod common;

use std::path::Path;

use predicates::prelude::*;
use serde_json::Value;

use common::{TempJjRepo, command, command_output};

fn parse_list_json(output: &std::process::Output) -> Value {
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

fn relative_name(path: &Path) -> String {
    path.file_name()
        .expect("path file name")
        .to_string_lossy()
        .to_string()
}

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
        .args(["ls"])
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
fn list_json_is_pretty_by_default() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");
    let repo_root = std::fs::canonicalize(repo.path()).expect("canonical repo root");
    let feature_root = std::fs::canonicalize(&feature_path).expect("canonical feature root");

    let output = command_output("navi", repo.path(), &["list", "--json"]);

    assert!(output.status.success(), "json list failed");
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("\n  \"workspaces\": [\n"));

    let json = parse_list_json(&output);
    let default = workspace_by_name(&json, "default");
    let feature = workspace_by_name(&json, "feature-auth");
    assert_eq!(default["is_current"], true);
    assert_eq!(default["path"]["display"], ".");
    assert_eq!(default["path"]["absolute"], repo_root.display().to_string());
    assert_eq!(default["path"]["state"], "confirmed");
    assert_eq!(default["path"]["source"], "current_workspace");
    assert_eq!(default["health"]["statuses"][0], "ok");
    assert_eq!(default["health"]["metadata_status"], "missing_record");
    assert_eq!(feature["is_current"], false);
    assert_eq!(
        feature["path"]["display"],
        format!("../{}", relative_name(&feature_path))
    );
    assert_eq!(
        feature["path"]["absolute"],
        feature_root.display().to_string()
    );
    assert_eq!(feature["path"]["state"], "confirmed");
    assert_eq!(feature["path"]["source"], "jj_recorded");
    assert_eq!(feature["health"]["metadata_status"], "missing_record");
}

#[test]
fn list_json_compact_renders_single_line() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");

    let output = command_output("navi", repo.path(), &["ls", "-j", "-c"]);

    assert!(output.status.success(), "compact json list failed");
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf8 stdout");
    assert_eq!(stdout.matches('\n').count(), 1);
    assert!(stdout.ends_with('\n'));

    let json = parse_list_json(&output);
    assert!(json["workspaces"].is_array());
}

#[test]
fn list_json_reports_present_with_path_metadata_status() {
    let repo = TempJjRepo::new();

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success();

    let output = command_output("navi", repo.path(), &["list", "--json"]);

    assert!(output.status.success(), "json list failed");
    let json = parse_list_json(&output);
    let feature = workspace_by_name(&json, "feature-auth");
    assert_eq!(feature["health"]["metadata_status"], "present_with_path");
}

#[test]
fn list_json_reports_present_without_path_metadata_status() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");
    repo.write_navi_metadata(
        "[[workspace]]\nname = \"feature-auth\"\ncreated_by_navi = true\ncreated_at = \"2026-03-11T00:00:00Z\"\ntemplate = \"../{repo}.{workspace}\"\nrevision = \"\"\n",
    );

    let output = command_output("navi", repo.path(), &["list", "--json"]);

    assert!(output.status.success(), "json list failed");
    let json = parse_list_json(&output);
    let feature = workspace_by_name(&json, "feature-auth");
    assert_eq!(feature["health"]["metadata_status"], "present_without_path");
    assert_eq!(feature["path"]["source"], "jj_recorded");
}

#[test]
fn list_json_reports_template_fallback() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");
    repo.clear_workspace_store_index();

    let output = command_output("navi", repo.path(), &["list", "--json"]);

    assert!(output.status.success(), "json list failed");
    let json = parse_list_json(&output);
    let feature = workspace_by_name(&json, "feature-auth");
    assert_eq!(
        feature["path"]["display"],
        format!("../{}.feature-auth", repo.repo_name())
    );
    assert_eq!(feature["path"]["state"], "inferred");
    assert_eq!(feature["path"]["source"], "template");
    assert_eq!(feature["health"]["statuses"][0], "inferred");
}

#[test]
fn list_json_reports_inferred_missing_workspace() {
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

    let output = command_output("navi", repo.path(), &["list", "--json"]);

    assert!(output.status.success(), "json list failed");
    let json = parse_list_json(&output);
    let feature = workspace_by_name(&json, "feature-auth");
    assert_eq!(
        feature["path"]["display"],
        format!("../{}", relative_name(&feature_path))
    );
    assert_eq!(feature["path"]["state"], "missing");
    assert_eq!(feature["path"]["source"], "navi_metadata");
    assert_eq!(feature["health"]["statuses"][0], "inferred");
    assert_eq!(feature["health"]["statuses"][1], "missing");
}

#[test]
fn list_json_reports_stale_workspace() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");
    let feature_name = feature_path.file_name().expect("workspace dir name");
    let mut moved_name = feature_name.to_os_string();
    moved_name.push(".moved");
    let moved_path = feature_path.with_file_name(moved_name);

    std::fs::rename(&feature_path, &moved_path).expect("move workspace dir");
    std::fs::create_dir(&feature_path).expect("create stale workspace dir");

    let output = command_output("navi", repo.path(), &["list", "--json"]);

    assert!(output.status.success(), "json list failed");
    let json = parse_list_json(&output);
    let feature = workspace_by_name(&json, "feature-auth");
    assert_eq!(feature["path"]["state"], "stale");
    assert_eq!(feature["path"]["source"], "jj_recorded");
    assert_eq!(feature["health"]["statuses"][0], "stale");
}

#[test]
fn list_json_reports_jj_only_workspace() {
    let repo = TempJjRepo::new();
    let custom_path = repo
        .path()
        .with_file_name(format!("{}-custom-feature-auth", repo.repo_name()));
    let custom_root = std::fs::canonicalize(repo.create_workspace_at("feature-auth", &custom_path))
        .expect("canonical custom workspace");

    let output = command_output("navi", repo.path(), &["list", "--json"]);

    assert!(output.status.success(), "json list failed");
    let json = parse_list_json(&output);
    let feature = workspace_by_name(&json, "feature-auth");
    assert_eq!(
        feature["path"]["absolute"],
        custom_root.display().to_string()
    );
    assert_eq!(feature["path"]["source"], "jj_recorded");
    assert_eq!(feature["health"]["statuses"][0], "jj-only");
    assert_eq!(feature["health"]["metadata_status"], "missing_record");
}

#[test]
fn list_json_uses_repo_primary_root_for_default_when_jj_path_is_missing() {
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

    let output = command_output("navi", &feature_path, &["list", "--json"]);

    assert!(output.status.success(), "json list failed");
    let json = parse_list_json(&output);
    let default = workspace_by_name(&json, "default");
    let repo_root = std::fs::canonicalize(repo.path()).expect("canonical repo root");
    assert_eq!(
        default["path"]["display"],
        format!("../{}", repo.repo_name())
    );
    assert_eq!(default["path"]["absolute"], repo_root.display().to_string());
    assert_eq!(default["path"]["state"], "confirmed");
    assert_eq!(default["path"]["source"], "repo_primary");
    assert_eq!(default["health"]["statuses"][0], "ok");
    assert!(
        default["health"]["statuses"]
            .as_array()
            .is_some_and(|s| s.len() == 1)
    );
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
