mod common;

use predicates::prelude::*;

use common::{TempJjRepo, command, install_bash_shell_integration};

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
fn doctor_uses_repo_primary_root_for_default_when_jj_path_is_missing() {
    let repo = TempJjRepo::new();
    let home = tempfile::TempDir::new().expect("temp home");
    install_bash_shell_integration(home.path());

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
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Doctor [ healthy ]"))
        .stdout(
            predicate::str::contains("default - workspace 'default' directory is missing").not(),
        )
        .stdout(predicate::str::contains(format!("../{}.default", repo.repo_name())).not());
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
