use std::path::Path;

use crate::Result;
use crate::output::{render_workspace_list_json, render_workspace_table};
use crate::repo::NaviWorkspace;

/// Run the `list` command.
///
/// # Errors
///
/// Returns an error if workspace discovery fails or if `jj workspace list`
/// fails.
pub fn run_list(path: &Path, json: bool, compact: bool) -> Result<()> {
    let repo = NaviWorkspace::open(path)?;

    if json {
        let snapshots = repo.list_workspace_snapshots()?;
        println!(
            "{}",
            render_workspace_list_json(repo.workspace_root(), &snapshots, compact)?
        );
        return Ok(());
    }

    let entries = repo.list_workspaces()?;

    print!("{}", render_workspace_table(&entries));
    Ok(())
}
