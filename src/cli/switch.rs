use std::path::Path;

use crate::output::write_cd_directive;
use crate::repo::NaviWorkspace;
use crate::types::WorkspaceName;
use crate::{Error, Result};

/// Run the `switch` command.
///
/// # Errors
///
/// Returns an error if workspace validation, discovery, or `jj workspace add`
/// fails.
pub fn run_switch(
    path: &Path,
    workspace: &str,
    create: bool,
    revision: Option<&str>,
) -> Result<()> {
    let workspace = WorkspaceName::new(workspace.to_owned())?;
    let repo = NaviWorkspace::open(path)?;

    let target_root = if repo.workspace_exists(&workspace)? {
        repo.actual_workspace_root(&workspace)?
    } else if create {
        repo.create_workspace(&workspace, revision)?
    } else {
        return Err(Error::WorkspaceDoesNotExist);
    };

    let display_path = repo.display_path_for_switch(&target_root);
    if !write_cd_directive(&display_path)? {
        println!("{}", display_path.display());
    }
    Ok(())
}
