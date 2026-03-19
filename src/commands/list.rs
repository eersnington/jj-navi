use std::path::Path;

use crate::Result;
use crate::output::render_workspace_table;
use crate::repo::NaviWorkspace;

/// Run the `list` command.
///
/// # Errors
///
/// Returns an error if workspace discovery fails or if `jj workspace list`
/// fails.
pub fn run_list(path: &Path) -> Result<()> {
    let repo = NaviWorkspace::open(path)?;
    let entries = repo.list_workspaces()?;

    print!("{}", render_workspace_table(&entries));
    Ok(())
}
