use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::Error;
use crate::Result;
use crate::output::{
    MANAGED_BLOCK_END, MANAGED_BLOCK_START, render_shell_init, render_shell_install_block,
};
use crate::types::ShellKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ManagedBlockState {
    Missing,
    Present { start: usize, end: usize },
    Invalid(&'static str),
}

/// Run `config shell init`.
///
/// # Errors
///
/// Returns an error if the shell is missing.
pub fn run_shell_init(command_name: &str, shell: Option<ShellKind>) -> Result<()> {
    let shell = shell.ok_or(Error::ShellRequired)?;

    print!("{}", render_shell_init(command_name, shell));
    Ok(())
}

/// Run `config shell install`.
///
/// # Errors
///
/// Returns an error if the shell is not supported, if shell detection fails,
/// or if the shell rc file cannot be updated.
pub fn run_shell_install(command_name: &str, shell: Option<ShellKind>) -> Result<()> {
    let shell = match shell {
        Some(shell) => shell,
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
    match inspect_managed_block(existing) {
        ManagedBlockState::Present { start, end } => {
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
        ManagedBlockState::Invalid(message) => Err(Error::InvalidShellRcFile {
            path: rc_path.to_path_buf(),
            message,
        }),
        ManagedBlockState::Missing => {
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

pub(crate) fn inspect_managed_block(existing: &str) -> ManagedBlockState {
    let starts = existing
        .match_indices(MANAGED_BLOCK_START)
        .collect::<Vec<_>>();
    let ends = existing
        .match_indices(MANAGED_BLOCK_END)
        .collect::<Vec<_>>();

    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => ManagedBlockState::Missing,
        ([(_, _)], [(_, _)]) => {
            let start = starts[0].0;
            let end = ends[0].0;
            if end < start {
                ManagedBlockState::Invalid("managed block markers are out of order")
            } else {
                ManagedBlockState::Present {
                    start,
                    end: end + MANAGED_BLOCK_END.len(),
                }
            }
        }
        ([], _) | (_, []) => ManagedBlockState::Invalid("managed block markers are unbalanced"),
        _ => ManagedBlockState::Invalid("managed block markers are duplicated"),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{ManagedBlockState, inspect_managed_block, upsert_managed_block};
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

    #[test]
    fn rejects_duplicated_managed_block_markers() {
        let existing = format!(
            "{MANAGED_BLOCK_START}\nold\n{MANAGED_BLOCK_START}\nnew\n{MANAGED_BLOCK_END}\n"
        );

        assert_eq!(
            inspect_managed_block(&existing),
            ManagedBlockState::Invalid("managed block markers are duplicated")
        );
    }
}
