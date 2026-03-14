use std::path::Path;

use crate::output::write_cd_directive;
use crate::repo::NaviWorkspace;
use crate::types::{WorkspaceName, WorkspacePathState};
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

    let resolved_path = if repo.workspace_exists(&workspace)? {
        repo.resolve_workspace_path(&workspace)?
    } else if create {
        let target_root = repo.create_workspace(&workspace, revision)?;
        let display_path = repo.display_path_for_switch(&target_root);
        if !write_cd_directive(&display_path)? {
            println!("{}", display_path.display());
        }
        return Ok(());
    } else {
        return Err(Error::WorkspaceDoesNotExist);
    };

    if !resolved_path.is_switchable() {
        let display_path = repo.display_path_for_switch(&resolved_path.path);
        return Err(Error::WorkspaceDirectoryUnavailable {
            workspace: workspace.as_str().to_owned(),
            path: display_path.display().to_string(),
        });
    }

    let target_root = resolved_path.path;
    if resolved_path.state == WorkspacePathState::Inferred
        && resolved_path.source.needs_switch_warning()
    {
        let display_path = repo.display_path_for_switch(&target_root);
        eprintln!(
            "warning: jj could not resolve this workspace path; using navi fallback\nhint: resolved to {}",
            display_path.display()
        );
    }

    let display_path = repo.display_path_for_switch(&target_root);
    if !write_cd_directive(&display_path)? {
        println!("{}", display_path.display());
    }
    Ok(())
}
