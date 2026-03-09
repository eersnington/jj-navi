use std::path::Path;

use jj_navi::repo::NaviWorkspace;
use jj_navi::types::WorkspaceName;
use jj_navi::{Error, Result};

pub fn run_switch(
    path: &Path,
    workspace: &str,
    create: bool,
    revision: Option<&str>,
) -> Result<()> {
    let workspace = WorkspaceName::new(workspace.to_owned())?;
    let repo = NaviWorkspace::open(path)?;

    let target_root = if repo.workspace_exists(&workspace)? {
        repo.planned_workspace_root(&workspace)?
    } else if create {
        repo.create_workspace(&workspace, revision)?
    } else {
        return Err(Error::WorkspaceDoesNotExist);
    };

    println!("{}", repo.display_path_for_switch(&target_root).display());
    Ok(())
}
