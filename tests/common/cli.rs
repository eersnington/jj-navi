#![allow(dead_code)]

use std::path::Path;

use assert_cmd::Command;

pub fn command(bin: &str) -> Command {
    match bin {
        "navi" => Command::new(assert_cmd::cargo::cargo_bin!("navi")),
        "nv" => Command::new(assert_cmd::cargo::cargo_bin!("nv")),
        _ => panic!("unknown bin: {bin}"),
    }
}

pub fn command_output(bin: &str, current_dir: &Path, args: &[&str]) -> std::process::Output {
    command(bin)
        .current_dir(current_dir)
        .args(args)
        .output()
        .expect("run command")
}

pub fn install_bash_shell_integration(home: &Path) {
    command("navi")
        .env("HOME", home)
        .env("SHELL", "/bin/bash")
        .args(["config", "shell", "install", "--shell", "bash"])
        .assert()
        .success();
}

#[cfg(unix)]
pub fn fake_old_jj_path() -> tempfile::TempDir {
    fake_jj_wrapper(Some("jj 0.38.0\n"), None, None, None)
}

#[cfg(unix)]
pub fn fake_jj_wrapper(
    version_output: Option<&str>,
    workspace_list_stdout: Option<&str>,
    workspace_list_stderr: Option<&str>,
    workspace_list_status: Option<i32>,
) -> tempfile::TempDir {
    use std::env;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::TempDir::new().expect("temp fake jj dir");
    let wrapper_path = temp.path().join("jj");
    let real_jj = env::split_paths(&env::var_os("PATH").expect("PATH set"))
        .map(|dir| dir.join("jj"))
        .find(|candidate| candidate.is_file())
        .expect("find real jj binary");
    let version_clause = version_output.map_or_else(
        || format!("  exec \"{}\" \"$@\"\n", real_jj.display()),
        |output| format!("  printf '{}'\n", output.escape_default()),
    );
    let workspace_list_clause = workspace_list_stdout.map_or_else(
        || format!("  exec \"{}\" \"$@\"\n", real_jj.display()),
        |stdout| {
            let stderr = workspace_list_stderr.unwrap_or_default().escape_default();
            let status = workspace_list_status.unwrap_or(0);
            format!(
                "  printf '{}'\n  printf '{}' >&2\n  exit {}\n",
                stdout.escape_default(),
                stderr,
                status
            )
        },
    );
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n{}elif [ \"$1\" = \"workspace\" ] && [ \"$2\" = \"list\" ]; then\n{}else\n  exec \"{}\" \"$@\"\nfi\n",
        version_clause,
        workspace_list_clause,
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

#[cfg(unix)]
pub fn path_with_fake_jj(fake_jj: &tempfile::TempDir) -> std::ffi::OsString {
    let mut paths = vec![fake_jj.path().to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").expect("PATH set"),
    ));
    std::env::join_paths(paths).expect("join PATH")
}
