mod common;

use predicates::prelude::*;

use common::command;

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
fn config_shell_init_missing_shell_mentions_supported_values() {
    command("navi")
        .args(["config", "shell", "init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("shell name required"))
        .stderr(predicate::str::contains("hint: use one of: bash, zsh"));
}

#[test]
fn config_shell_install_creates_bashrc_managed_block() {
    let home = tempfile::TempDir::new().expect("temp home");
    let bashrc = home.path().join(".bashrc");

    command("navi")
        .env("HOME", home.path())
        .args(["config", "shell", "install", "-s", "bash"])
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
fn config_shell_install_rejects_duplicated_managed_block_markers() {
    let home = tempfile::TempDir::new().expect("temp home");
    let bashrc = home.path().join(".bashrc");
    std::fs::write(
        &bashrc,
        "# >>> jj-navi shell init >>>\nold\n# >>> jj-navi shell init >>>\nnew\n# <<< jj-navi shell init <<<\n",
    )
    .expect("write invalid bashrc");

    command("navi")
        .env("HOME", home.path())
        .args(["config", "shell", "install", "--shell", "bash"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(bashrc.display().to_string()))
        .stderr(predicate::str::contains(
            "managed block markers are duplicated",
        ));
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
