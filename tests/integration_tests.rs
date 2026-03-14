mod common;

use assert_cmd::Command;
use predicates::prelude::*;

use common::TempJjRepo;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn command(bin: &str) -> Command {
    match bin {
        "navi" => Command::new(assert_cmd::cargo::cargo_bin!("navi")),
        "nv" => Command::new(assert_cmd::cargo::cargo_bin!("nv")),
        _ => panic!("unknown bin: {bin}"),
    }
}

fn command_output(bin: &str, current_dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    command(bin)
        .current_dir(current_dir)
        .args(args)
        .output()
        .expect("run command")
}

fn install_bash_shell_integration(home: &std::path::Path) {
    command("navi")
        .env("HOME", home)
        .env("SHELL", "/bin/bash")
        .args(["config", "shell", "install", "--shell", "bash"])
        .assert()
        .success();
}

#[cfg(unix)]
fn fake_old_jj_path() -> tempfile::TempDir {
    use std::env;
    use std::fs;

    let temp = tempfile::TempDir::new().expect("temp fake jj dir");
    let wrapper_path = temp.path().join("jj");
    let real_jj = env::split_paths(&env::var_os("PATH").expect("PATH set"))
        .map(|dir| dir.join("jj"))
        .find(|candidate| candidate.is_file())
        .expect("find real jj binary");
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'jj 0.38.0\\n'\nelse\n  exec \"{}\" \"$@\"\nfi\n",
        real_jj.display()
    );

    fs::write(&wrapper_path, script).expect("write fake jj wrapper");
    let mut permissions = fs::metadata(&wrapper_path)
        .expect("fake jj metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&wrapper_path, permissions).expect("set fake jj permissions");

    temp
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
        .stdout(predicate::str::contains(format!(
            "../{}.feature-auth",
            repo.repo_name()
        )))
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
        .stderr(predicate::str::contains("invalid value 'fish'"))
        .stderr(predicate::str::contains("[possible values: bash, zsh]"));
}

#[test]
fn config_shell_init_help_lists_supported_shells() {
    command("navi")
        .args(["config", "shell", "init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Supported shell"))
        .stdout(predicate::str::contains("bash"))
        .stdout(predicate::str::contains("zsh"));
}

#[test]
fn config_shell_init_missing_shell_mentions_supported_values() {
    command("navi")
        .args(["config", "shell", "init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error: shell name required"))
        .stderr(predicate::str::contains("hint: use one of: bash, zsh"))
        .stderr(predicate::str::contains("bash"))
        .stderr(predicate::str::contains("zsh"));
}

#[test]
fn config_help_describes_shell_integration_commands() {
    command("navi")
        .args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Shell integration and future config commands",
        ))
        .stdout(predicate::str::contains(
            "shell  Shell integration commands",
        ));
}

#[test]
fn top_level_help_describes_config_command() {
    command("navi")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "config  Shell integration and future config commands",
        ));
}

#[test]
fn doctor_reports_ok_for_healthy_repo() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    install_bash_shell_integration(home.path());

    command("navi")
        .current_dir(repo.path())
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Doctor [ healthy ]"))
        .stdout(predicate::str::contains("Summary ok"))
        .stdout(predicate::str::contains("Checks"));
}

#[test]
fn doctor_reports_missing_navi_metadata_for_current_named_workspace() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    install_bash_shell_integration(home.path());
    let feature_path = repo.create_workspace("feature-auth");

    command("navi")
        .current_dir(&feature_path)
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "workspace 'feature-auth' exists in jj but has no navi metadata",
        ));
}

#[test]
fn doctor_reports_invalid_repo_config() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    install_bash_shell_integration(home.path());
    repo.write_navi_config("workspace_template = \"../{repo\"\n");

    command("navi")
        .current_dir(repo.path())
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Doctor [ attention needed ]"))
        .stdout(predicate::str::contains("Summary 1 error"))
        .stdout(predicate::str::contains("invalid repo config"));
}

#[test]
fn doctor_reports_missing_workspace_directory() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    install_bash_shell_integration(home.path());

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success();
    let feature_path =
        repo.path()
            .with_file_name(format!("{}.{}", repo.repo_name(), "feature-auth"));
    std::fs::remove_dir_all(&feature_path).expect("remove workspace dir");

    command("navi")
        .current_dir(repo.path())
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "! [ warning ]  feature-auth - workspace 'feature-auth' directory is missing",
        ))
        .stdout(predicate::str::contains("hint: last known path:"));
}

#[test]
fn doctor_reports_metadata_only_workspace() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    install_bash_shell_integration(home.path());

    command("navi")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-auth"])
        .assert()
        .success();
    repo.run(&["workspace", "forget", "feature-auth"]);

    command("navi")
        .current_dir(repo.path())
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "! [ warning ]  feature-auth - metadata exists for workspace 'feature-auth' but jj no longer lists it",
        ))
        .stdout(predicate::str::contains("hint: safe prune candidate"));
}

#[test]
fn doctor_reports_jj_only_workspace() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    install_bash_shell_integration(home.path());
    repo.create_workspace("feature-auth");

    command("navi")
        .current_dir(repo.path())
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "o [ info ]  feature-auth - workspace 'feature-auth' exists in jj but has no navi metadata",
        ));
}

#[test]
fn doctor_does_not_treat_pathless_metadata_as_missing_metadata() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    install_bash_shell_integration(home.path());
    repo.create_workspace("feature-auth");
    repo.write_navi_metadata(
        "[[workspace]]\nname = \"feature-auth\"\ncreated_by_navi = true\ncreated_at = \"2026-03-11T00:00:00Z\"\ntemplate = \"../{repo}.{workspace}\"\nrevision = \"\"\n",
    );

    command("navi")
        .current_dir(repo.path())
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Doctor [ healthy ]"))
        .stdout(predicate::str::contains("has no navi metadata").not());
}

#[test]
fn doctor_does_not_treat_empty_metadata_path_as_missing_metadata() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    install_bash_shell_integration(home.path());
    repo.create_workspace("feature-auth");
    repo.write_navi_metadata(
        "[[workspace]]\nname = \"feature-auth\"\npath = \"\"\ncreated_by_navi = true\ncreated_at = \"2026-03-11T00:00:00Z\"\ntemplate = \"../{repo}.{workspace}\"\nrevision = \"\"\n",
    );

    command("navi")
        .current_dir(repo.path())
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Doctor [ healthy ]"))
        .stdout(predicate::str::contains("has no navi metadata").not());
}

#[test]
fn doctor_reports_orphaned_current_workspace() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    install_bash_shell_integration(home.path());
    let feature_path = repo.create_workspace("feature-auth");
    repo.run(&["workspace", "forget", "feature-auth"]);

    command("navi")
        .current_dir(&feature_path)
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "x [ error ]  current directory is no longer a registered jj workspace",
        ));
}

#[test]
fn doctor_json_is_pretty_by_default() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    install_bash_shell_integration(home.path());
    repo.create_workspace("feature-auth");

    command("navi")
        .current_dir(repo.path())
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{\n"))
        .stdout(predicate::str::contains("\"code\": \"jj_only_workspace\""));
}

#[test]
fn doctor_json_compact_renders_single_line() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    install_bash_shell_integration(home.path());
    repo.create_workspace("feature-auth");

    command("navi")
        .current_dir(repo.path())
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor", "--json", "--compact"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{"))
        .stdout(
            predicate::str::is_match("\n.+")
                .expect("newline regex")
                .not(),
        )
        .stdout(predicate::str::contains("\"code\":\"jj_only_workspace\""));
}

#[test]
fn doctor_reports_invalid_shell_rc_block() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    let bashrc = home.path().join(".bashrc");
    std::fs::write(
        &bashrc,
        "# <<< jj-navi shell init <<<\n# >>> jj-navi shell init >>>\n",
    )
    .expect("write invalid bashrc");

    command("navi")
        .current_dir(repo.path())
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "x [ error ]  invalid shell rc file at",
        ))
        .stdout(predicate::str::contains(
            "managed block markers are out of order",
        ));
}

#[test]
fn doctor_reports_duplicated_shell_markers() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    let bashrc = home.path().join(".bashrc");
    std::fs::write(
        &bashrc,
        "# >>> jj-navi shell init >>>\none\n# >>> jj-navi shell init >>>\ntwo\n# <<< jj-navi shell init <<<\n",
    )
    .expect("write invalid bashrc");

    command("navi")
        .current_dir(repo.path())
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "managed block markers are duplicated",
        ));
}

#[test]
fn nv_doctor_uses_nv_in_shell_install_hint() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");

    command("nv")
        .current_dir(repo.path())
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "hint: run: nv config shell install --shell bash",
        ));
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
        .stdout(predicate::str::contains("@       default"))
        .stdout(predicate::str::contains("feature-auth"))
        .stdout(predicate::str::contains(format!(
            "../{}.feature-auth [inferred]",
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
        .stdout(predicate::str::contains(format!(
            "../{}.feature-auth [inferred] [missing]",
            repo.repo_name()
        )));
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
        .stdout(predicate::str::contains(format!(
            "../{}.feature-auth [stale]",
            repo.repo_name()
        )))
        .stdout(predicate::str::contains("feature-auth"))
        .stdout(predicate::str::contains("[inferred] [stale]").not());
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

#[cfg(unix)]
#[test]
fn repo_commands_fail_fast_on_unsupported_jj_version() {
    let repo = TempJjRepo::new();
    let fake_jj = fake_old_jj_path();
    let mut paths = vec![fake_jj.path().to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").expect("PATH set"),
    ));
    let path = std::env::join_paths(paths).expect("join PATH");

    command("navi")
        .current_dir(repo.path())
        .env("PATH", path)
        .args(["list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "error: jj 0.39.0 or newer required",
        ))
        .stderr(predicate::str::contains("hint: found jj 0.38.0"));
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
