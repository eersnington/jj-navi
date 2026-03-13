use std::path::Path;

use crate::Result;
use crate::repo::NaviWorkspace;
use crate::types::WorkspaceName;

/// Run the `remove` command.
///
/// # Errors
///
/// Returns an error if workspace validation, discovery, or `jj workspace forget`
/// fails.
pub fn run_remove(path: &Path, workspace: &str) -> Result<()> {
    let workspace = WorkspaceName::new(workspace.to_owned())?;
    let repo = NaviWorkspace::open(path)?;
    let removed = repo.forget_workspace(&workspace)?;

    println!("forgot workspace '{removed}'");
    Ok(())
}
