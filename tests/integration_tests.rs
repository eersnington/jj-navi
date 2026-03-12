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
    assert!(repo.navi_config_path().is_file());
    assert!(
        std::fs::read_to_string(repo.navi_config_path())
            .expect("read navi config")
            .contains("workspace_template = \"../{repo}.{workspace}\"")
    );
    assert!(repo.navi_metadata_path().is_file());
    let metadata = std::fs::read_to_string(repo.navi_metadata_path()).expect("read navi metadata");
    assert!(metadata.contains("name = \"feature-auth\""));
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
fn list_uses_recorded_template_after_config_changes() {
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
        .stdout(predicate::str::contains(format!(
            "../{}.feature-auth",
            repo.repo_name()
        )))
        .stdout(predicate::str::contains("feature-auth"));
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
fn config_shell_init_bash_prints_wrapper() {
    command("navi")
        .args(["config", "shell", "init", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("navi()"))
        .stdout(predicate::str::contains("NAVI_DIRECTIVE_FILE"));
}

#[test]
fn config_shell_init_zsh_prints_wrapper() {
    command("navi")
        .args(["config", "shell", "init", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("navi()"))
        .stdout(predicate::str::contains("command navi \"$@\""));
}

#[test]
fn config_shell_init_rejects_unsupported_shell() {
    command("navi")
        .args(["config", "shell", "init", "fish"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error: unsupported shell 'fish'"));
}

#[test]
fn config_shell_install_creates_bashrc_managed_block() {
    let home = tempfile::TempDir::new().expect("temp home");
    let bashrc = home.path().join(".bashrc");

    command("navi")
        .env("HOME", home.path())
        .args(["config", "shell", "install", "--shell", "bash"])
        .assert()
        .success();

    let contents = std::fs::read_to_string(bashrc).expect("read bashrc");
    assert_eq!(contents.matches("# >>> jj-navi shell init >>>").count(), 1);
    assert!(contents.contains("eval \"$(command navi config shell init bash)\""));
}

#[test]
fn config_shell_install_creates_zshrc_managed_block() {
    let home = tempfile::TempDir::new().expect("temp home");
    let zshrc = home.path().join(".zshrc");

    command("navi")
        .env("HOME", home.path())
        .args(["config", "shell", "install", "--shell", "zsh"])
        .assert()
        .success();

    let contents = std::fs::read_to_string(zshrc).expect("read zshrc");
    assert_eq!(contents.matches("# >>> jj-navi shell init >>>").count(), 1);
    assert!(contents.contains("eval \"$(command navi config shell init zsh)\""));
}

#[test]
fn config_shell_install_updates_managed_block_in_place() {
    let home = tempfile::TempDir::new().expect("temp home");
    let bashrc = home.path().join(".bashrc");

    command("navi")
        .env("HOME", home.path())
        .args(["config", "shell", "install", "--shell", "bash"])
        .assert()
        .success();
    command("navi")
        .env("HOME", home.path())
        .args(["config", "shell", "install", "--shell", "bash"])
        .assert()
        .success();

    let contents = std::fs::read_to_string(bashrc).expect("read bashrc");
    assert_eq!(contents.matches("# >>> jj-navi shell init >>>").count(), 1);
}

#[test]
fn config_shell_install_reports_real_rc_path_for_invalid_managed_block() {
    let home = tempfile::TempDir::new().expect("temp home");
    let bashrc = home.path().join(".bashrc");
    std::fs::write(
        &bashrc,
        "# <<< jj-navi shell init <<<\n# >>> jj-navi shell init >>>\n",
    )
    .expect("write invalid bashrc");

    command("navi")
        .env("HOME", home.path())
        .args(["config", "shell", "install", "--shell", "bash"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(bashrc.display().to_string()))
        .stderr(predicate::str::contains(
            "managed block markers are out of order",
        ));
}

#[cfg(unix)]
#[test]
fn config_shell_install_fails_for_non_utf8_rc_file() {
    let home = tempfile::TempDir::new().expect("temp home");
    let bashrc = home.path().join(".bashrc");
    std::fs::write(&bashrc, [0xFF]).expect("write non-utf8 bashrc");

    command("navi")
        .env("HOME", home.path())
        .args(["config", "shell", "install", "--shell", "bash"])
        .assert()
        .failure();
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
