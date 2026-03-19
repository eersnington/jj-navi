use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;

/// Environment variable used by shell integration to pass a directive file.
pub const DIRECTIVE_FILE_ENV_VAR: &str = "NAVI_DIRECTIVE_FILE";

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
