mod common;

use predicates::prelude::*;

use common::{TempJjRepo, command};
#[cfg(unix)]
use common::{fake_jj_wrapper, fake_old_jj_path, path_with_fake_jj};

#[cfg(unix)]
#[test]
fn repo_commands_fail_fast_on_unsupported_jj_version() {
    let repo = TempJjRepo::new();
    let fake_jj = fake_old_jj_path();
    let path = path_with_fake_jj(&fake_jj);

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

#[cfg(unix)]
#[test]
fn repo_commands_accept_supported_plain_jj_version() {
    let repo = TempJjRepo::new();
    let fake_jj = fake_jj_wrapper(Some("jj 0.39.0\n"), None, None, None);

    command("navi")
        .current_dir(repo.path())
        .env("PATH", path_with_fake_jj(&fake_jj))
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("default"));
}

#[cfg(unix)]
#[test]
fn repo_commands_accept_supported_suffixed_jj_version() {
    let repo = TempJjRepo::new();
    let fake_jj = fake_jj_wrapper(Some("jj 0.39.0-12-gabcdef\n"), None, None, None);

    command("navi")
        .current_dir(repo.path())
        .env("PATH", path_with_fake_jj(&fake_jj))
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("default"));
}

#[cfg(unix)]
#[test]
fn repo_commands_fail_for_unparseable_jj_version() {
    let repo = TempJjRepo::new();
    let fake_jj = fake_jj_wrapper(Some("jj dev build\n"), None, None, None);

    command("navi")
        .current_dir(repo.path())
        .env("PATH", path_with_fake_jj(&fake_jj))
        .args(["list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "error: jj 0.39.0 or newer required",
        ))
        .stderr(predicate::str::contains("hint: found jj dev build"));
}

#[test]
fn repo_commands_resolve_relative_repo_pointer_from_secondary_workspace() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");

    command("navi")
        .current_dir(&feature_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("feature-auth"));
}

#[test]
fn repo_commands_fail_for_missing_repo_pointer_target() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");
    let repo_pointer = feature_path.join(".jj").join("repo");

    std::fs::write(&repo_pointer, "../../missing/repo\n").expect("write repo pointer");

    command("navi")
        .current_dir(&feature_path)
        .args(["list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(repo_pointer.display().to_string()))
        .stderr(predicate::str::contains("invalid repo pointer"));
}

#[test]
fn repo_commands_fail_for_non_directory_repo_pointer_target() {
    let repo = TempJjRepo::new();
    let feature_path = repo.create_workspace("feature-auth");
    let repo_pointer = feature_path.join(".jj").join("repo");
    let shared_file = feature_path.join("not-a-directory");

    std::fs::write(&shared_file, "not a dir").expect("write shared file");
    std::fs::write(&repo_pointer, "../not-a-directory\n").expect("write repo pointer");

    command("navi")
        .current_dir(&feature_path)
        .args(["list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(repo_pointer.display().to_string()))
        .stderr(predicate::str::contains("invalid repo pointer"));
}

#[cfg(unix)]
#[test]
fn repo_commands_fail_for_malformed_workspace_list_entry() {
    let repo = TempJjRepo::new();
    let fake_jj = fake_jj_wrapper(Some("jj 0.39.0\n"), Some("default\0"), None, Some(0));

    command("navi")
        .current_dir(repo.path())
        .env("PATH", path_with_fake_jj(&fake_jj))
        .args(["list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error: invalid workspace name"))
        .stderr(predicate::str::contains("default"));
}

#[cfg(unix)]
#[test]
fn repo_commands_fail_for_invalid_workspace_list_current_marker() {
    let repo = TempJjRepo::new();
    let fake_jj = fake_jj_wrapper(
        Some("jj 0.39.0\n"),
        Some("default\0x\0abc123\0message"),
        None,
        Some(0),
    );

    command("navi")
        .current_dir(repo.path())
        .env("PATH", path_with_fake_jj(&fake_jj))
        .args(["list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error: invalid workspace name"))
        .stderr(predicate::str::contains("default"));
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
