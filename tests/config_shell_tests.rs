mod common;

use predicates::prelude::*;

use common::command;

#[test]
fn config_shell_init_rejects_unsupported_shell() {
    command("navi")
        .args(["config", "shell", "init", "pwsh"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'pwsh'"))
        .stderr(predicate::str::contains(
            "[possible values: bash, zsh, fish]",
        ));
}

#[test]
fn config_shell_init_missing_shell_mentions_supported_values() {
    command("navi")
        .args(["config", "shell", "init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("shell name required"))
        .stderr(predicate::str::contains(
            "hint: use one of: bash, zsh, fish",
        ));
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
fn config_shell_init_bash_registers_completion() {
    command("navi")
        .args(["config", "shell", "init", "bash"])
        .assert()
        .success()
        .stdout(predicates::str::contains("COMPLETE=bash command navi"))
        .stdout(predicates::str::contains(
            "complete -o nospace -o bashdefault -F _navi_lazy_complete navi",
        ))
        .stdout(predicates::str::contains("COMPLETE=bash command nv"))
        .stdout(predicates::str::contains(
            "complete -o nospace -o bashdefault -F _nv_lazy_complete nv",
        ));
}

#[test]
fn config_shell_init_bash_from_nv_registers_nv_first() {
    command("nv")
        .args(["config", "shell", "init", "bash"])
        .assert()
        .success()
        .stdout(predicates::str::contains("COMPLETE=bash command nv"))
        .stdout(predicates::str::contains(
            "complete -o nospace -o bashdefault -F _nv_lazy_complete nv",
        ));
}

#[test]
fn config_shell_init_zsh_registers_completion() {
    command("navi")
        .args(["config", "shell", "init", "zsh"])
        .assert()
        .success()
        .stdout(predicates::str::contains("COMPLETE=zsh command navi"))
        .stdout(predicates::str::contains(
            "compdef _navi_lazy_complete navi",
        ))
        .stdout(predicates::str::contains("COMPLETE=zsh command nv"))
        .stdout(predicates::str::contains("compdef _nv_lazy_complete nv"));
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
fn config_shell_init_fish_emits_function_wrappers() {
    command("navi")
        .args(["config", "shell", "init", "fish"])
        .assert()
        .success()
        .stdout(predicates::str::contains("function navi"))
        .stdout(predicates::str::contains("function nv"))
        .stdout(predicates::str::contains(
            "set -lx NAVI_DIRECTIVE_FILE $directive_file",
        ));
}

#[test]
fn config_shell_install_creates_fish_functions_and_completions() {
    let home = tempfile::TempDir::new().expect("temp home");
    let navi_func = home.path().join(".config/fish/functions/navi.fish");
    let nv_func = home.path().join(".config/fish/functions/nv.fish");
    let navi_comp = home.path().join(".config/fish/completions/navi.fish");
    let nv_comp = home.path().join(".config/fish/completions/nv.fish");

    command("navi")
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join(".config"))
        .args(["config", "shell", "install", "--shell", "fish"])
        .assert()
        .success();

    let navi_function = std::fs::read_to_string(navi_func).expect("read navi function");
    assert!(navi_function.contains("function navi"));

    let nv_function = std::fs::read_to_string(nv_func).expect("read nv function");
    assert!(nv_function.contains("function nv"));

    let navi_completions = std::fs::read_to_string(navi_comp).expect("read navi completions");
    assert!(navi_completions.contains("complete --keep-order --exclusive --command navi"));

    let nv_completions = std::fs::read_to_string(nv_comp).expect("read nv completions");
    assert!(nv_completions.contains("complete --keep-order --exclusive --command nv"));
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
