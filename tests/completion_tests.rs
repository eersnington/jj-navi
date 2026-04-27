mod common;

use common::{TempJjRepo, command};

fn completion_output(current_dir: &std::path::Path, args: &[&str]) -> String {
    let output = command("navi")
        .current_dir(current_dir)
        .env("COMPLETE", "bash")
        .args(args)
        .output()
        .expect("run completion command");

    assert!(
        output.status.success(),
        "completion failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn switch_completion_shows_workspace_names() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");
    repo.create_workspace("hotfix-bug");

    let stdout = completion_output(repo.path(), &["--", "navi", "switch", ""]);

    assert!(stdout.lines().any(|line| line == "feature-auth"));
    assert!(stdout.lines().any(|line| line == "hotfix-bug"));
}

#[test]
fn switch_completion_filters_workspace_prefix() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");
    repo.create_workspace("hotfix-bug");

    let stdout = completion_output(repo.path(), &["--", "navi", "switch", "fea"]);

    assert!(stdout.lines().any(|line| line == "feature-auth"));
    assert!(!stdout.lines().any(|line| line == "hotfix-bug"));
}

#[test]
fn remove_completion_shows_workspace_names() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");

    let stdout = completion_output(repo.path(), &["--", "navi", "remove", ""]);

    assert!(stdout.lines().any(|line| line == "feature-auth"));
}

#[test]
fn merge_completion_shows_workspace_names_for_flags() {
    let repo = TempJjRepo::new();
    repo.create_workspace("feature-auth");

    let stdout = completion_output(repo.path(), &["--", "navi", "merge", "--from", ""]);

    assert!(stdout.lines().any(|line| line == "feature-auth"));
}

#[test]
fn switch_flag_completion_still_shows_options() {
    let repo = TempJjRepo::new();

    let stdout = completion_output(repo.path(), &["--", "navi", "switch", "--"]);

    assert!(stdout.lines().any(|line| line == "--create"));
    assert!(stdout.lines().any(|line| line == "--revision"));
}

#[test]
fn list_completion_hides_compact_until_json_is_present() {
    let repo = TempJjRepo::new();

    let stdout = completion_output(repo.path(), &["--", "navi", "ls", "--"]);

    assert!(stdout.lines().any(|line| line == "--json"));
    assert!(!stdout.lines().any(|line| line == "--compact"));
}

#[test]
fn list_completion_shows_compact_after_json_is_present() {
    let repo = TempJjRepo::new();

    let stdout = completion_output(repo.path(), &["--", "navi", "ls", "--json", "--"]);

    assert!(stdout.lines().any(|line| line == "--compact"));
}

#[test]
fn doctor_completion_hides_compact_until_json_is_present() {
    let repo = TempJjRepo::new();

    let stdout = completion_output(repo.path(), &["--", "navi", "doctor", "--"]);

    assert!(stdout.lines().any(|line| line == "--json"));
    assert!(!stdout.lines().any(|line| line == "--compact"));
}

#[test]
fn switch_completion_outside_jj_repo_does_not_error() {
    let temp = tempfile::TempDir::new().expect("temp dir");

    let stdout = completion_output(temp.path(), &["--", "navi", "switch", ""]);

    assert!(!stdout.lines().any(|line| line == "feature-auth"));
}
