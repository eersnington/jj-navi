use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::Error;
use crate::Result;
use crate::output::{
    MANAGED_BLOCK_END, MANAGED_BLOCK_START, render_shell_init, render_shell_install_block,
};
use crate::types::ShellKind;

/// Run `config shell init`.
///
/// # Errors
///
/// Returns an error if the shell is not supported.
pub fn run_shell_init(command_name: &str, shell: &str) -> Result<()> {
    let shell = ShellKind::new(shell)?;

    print!("{}", render_shell_init(command_name, shell));
    Ok(())
}

/// Run `config shell install`.
///
/// # Errors
///
/// Returns an error if the shell is not supported, if shell detection fails,
/// or if the shell rc file cannot be updated.
pub fn run_shell_install(command_name: &str, shell: Option<&str>) -> Result<()> {
    let shell = match shell {
        Some(shell) => ShellKind::new(shell)?,
        None => ShellKind::detect()?,
    };
    let rc_path = shell_rc_path(shell)?;
    let block = render_shell_install_block(command_name, shell);
    let existing = match fs::read_to_string(&rc_path) {
        Ok(existing) => existing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    let updated = upsert_managed_block(&existing, &block, &rc_path)?;

    if let Some(parent) = rc_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&rc_path, updated)?;
    println!("installed shell integration in {}", rc_path.display());
    Ok(())
}

fn shell_rc_path(shell: ShellKind) -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| Error::HomeDirectory)?;
    Ok(PathBuf::from(home).join(shell.rc_file_name()))
}

fn upsert_managed_block(existing: &str, block: &str, rc_path: &Path) -> Result<String> {
    match (
        existing.find(MANAGED_BLOCK_START),
        existing.find(MANAGED_BLOCK_END),
    ) {
        (Some(start), Some(end)) if end >= start => {
            let end = end + MANAGED_BLOCK_END.len();
            let mut updated = String::new();
            updated.push_str(&existing[..start]);
            if !updated.is_empty() && !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push_str(block);
            let suffix = existing[end..].trim_start_matches('\n');
            if !suffix.is_empty() {
                if !updated.ends_with('\n') {
                    updated.push('\n');
                }
                updated.push_str(suffix);
                if !updated.ends_with('\n') {
                    updated.push('\n');
                }
            }
            Ok(updated)
        }
        (Some(_), Some(_)) => Err(Error::InvalidShellRcFile {
            path: rc_path.to_path_buf(),
            message: "managed block markers are out of order",
        }),
        (Some(_), None) | (None, Some(_)) => Err(Error::InvalidShellRcFile {
            path: rc_path.to_path_buf(),
            message: "managed block markers are unbalanced",
        }),
        (None, None) => {
            let mut updated = String::new();
            updated.push_str(existing);
            if !updated.is_empty() && !updated.ends_with('\n') {
                updated.push('\n');
            }
            if !updated.is_empty() && !updated.ends_with("\n\n") {
                updated.push('\n');
            }
            updated.push_str(block);
            Ok(updated)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::upsert_managed_block;
    use crate::output::{MANAGED_BLOCK_END, MANAGED_BLOCK_START};

    #[test]
    fn updates_existing_managed_block() {
        let existing =
            format!("line before\n{MANAGED_BLOCK_START}\nold\n{MANAGED_BLOCK_END}\nline after\n");
        let block = format!("{MANAGED_BLOCK_START}\nnew\n{MANAGED_BLOCK_END}\n");

        let updated = upsert_managed_block(&existing, &block, Path::new(".bashrc"))
            .expect("update managed block");

        assert!(updated.contains("line before"));
        assert!(updated.contains("line after"));
        assert!(updated.contains("new"));
        assert!(!updated.contains("old"));
    }
}
