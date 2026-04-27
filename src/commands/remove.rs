use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::Error;
use crate::Result;
use crate::repo::NaviWorkspace;
use crate::types::WorkspaceName;

/// Run the `remove` command.
///
/// # Errors
///
/// Returns an error if workspace validation, discovery, confirmation,
/// `jj workspace forget`, or directory deletion fails.
pub fn run_remove(path: &Path, workspace: &str, yes: bool) -> Result<()> {
    let workspace = WorkspaceName::new(workspace.to_owned())?;
    let repo = NaviWorkspace::open(path)?;
    let target_root = repo.resolve_removable_workspace_path(&workspace)?;

    if !yes {
        confirm_remove(&workspace, &target_root)?;
    }

    let removed = repo.forget_workspace(&workspace)?;
    fs::remove_dir_all(&target_root).map_err(|source| {
        Error::WorkspaceDirectoryDeleteAfterForgetFailed {
            workspace: removed.as_str().to_owned(),
            path: target_root.display().to_string(),
            source,
        }
    })?;

    println!("forgot workspace '{removed}'");
    println!("deleted workspace directory '{}'", target_root.display());
    Ok(())
}

fn confirm_remove(workspace: &WorkspaceName, target_root: &Path) -> Result<()> {
    println!(
        "This will permanently remove workspace '{}'.",
        workspace.as_str()
    );
    println!("Directory to delete: {}", target_root.display());
    print!("Type 'yes' to continue: ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if answer.trim() == "yes" {
        Ok(())
    } else {
        Err(Error::RemoveCancelled)
    }
}
