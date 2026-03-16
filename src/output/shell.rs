use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;

use crate::types::ShellKind;

/// Environment variable used by shell integration to pass a directive file.
pub const DIRECTIVE_FILE_ENV_VAR: &str = "NAVI_DIRECTIVE_FILE";
/// Marker for the start of the managed shell block.
pub const MANAGED_BLOCK_START: &str = "# >>> jj-navi shell init >>>";
/// Marker for the end of the managed shell block.
pub const MANAGED_BLOCK_END: &str = "# <<< jj-navi shell init <<<";

/// Render shell initialization code for the chosen shell.
#[must_use]
pub fn render_shell_init(command_name: &str, shell: ShellKind) -> String {
    let source_cmd = match shell {
        ShellKind::Bash | ShellKind::Zsh => "source",
    };

    format!(
        "# jj-navi shell integration for {shell}\nif command -v {command_name} >/dev/null 2>&1; then\n    {command_name}() {{\n        local directive_file exit_code=0\n        directive_file=\"$(mktemp)\"\n        {directive_env}=\"$directive_file\" command {command_name} \"$@\" || exit_code=$?\n        if [[ -s \"$directive_file\" ]]; then\n            {source_cmd} \"$directive_file\"\n            if [[ $exit_code -eq 0 ]]; then\n                exit_code=$?\n            fi\n        fi\n        rm -f \"$directive_file\"\n        return \"$exit_code\"\n    }}\nfi\n",
        shell = shell.as_str(),
        command_name = command_name,
        directive_env = DIRECTIVE_FILE_ENV_VAR,
        source_cmd = source_cmd,
    )
}

/// Render the managed shell block inserted into a shell rc file.
#[must_use]
pub fn render_shell_install_block(command_name: &str, shell: ShellKind) -> String {
    format!(
        "{MANAGED_BLOCK_START}\neval \"$(command {command_name} config shell init {shell})\"\n{MANAGED_BLOCK_END}\n",
        command_name = command_name,
        shell = shell.as_str(),
    )
}

/// Write a shell-safe `cd` directive if shell integration is active.
///
/// Returns `true` if a directive was written.
///
/// # Errors
///
/// Returns an error if the directive file path is invalid or writing fails.
pub fn write_cd_directive(path: &Path) -> crate::Result<bool> {
    let Ok(directive_file) = std::env::var(DIRECTIVE_FILE_ENV_VAR) else {
        return Ok(false);
    };

    if directive_file.trim().is_empty() {
        return Ok(false);
    }

    let escaped_path = escape_shell_single_quotes(
        path.to_str()
            .ok_or(crate::Error::ShellDirectivePathNotUtf8)?,
    );
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(directive_file)?;
    writeln!(file, "cd -- '{escaped_path}'")?;
    Ok(true)
}

/// Escape single quotes for POSIX shell single-quoted strings.
#[must_use]
pub fn escape_shell_single_quotes(value: &str) -> String {
    value.replace('\'', "'\\''")
}
