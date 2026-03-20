use std::path::Path;

use crate::repo::{NaviWorkspace, ResolvedWorkspacePath};
use crate::shell::write_cd_directive;
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
    let repo = NaviWorkspace::open(path)?;

    if workspace == "-" && !create && revision.is_none() {
        let (workspace, resolved_path) = repo.resolve_previous_workspace_path()?;
        emit_existing_switch(&repo, &workspace, resolved_path)?;
        warn_if_previous_workspace_state_update_fails(
            &repo.record_previous_workspace_after_switch(&workspace),
        );
        return Ok(());
    }

    let workspace = normalize_workspace_alias(&repo, workspace);
    let workspace = WorkspaceName::new(workspace)?;

    let resolved_path = if repo.workspace_exists(&workspace)? {
        repo.resolve_workspace_path(&workspace)?
    } else if create {
        let target_root = repo.create_workspace(&workspace, revision)?;
        emit_switch_destination(&repo, &target_root)?;
        warn_if_previous_workspace_state_update_fails(
            &repo.record_previous_workspace_after_switch(&workspace),
        );
        return Ok(());
    } else {
        return Err(Error::WorkspaceDoesNotExist);
    };

    emit_existing_switch(&repo, &workspace, resolved_path)?;
    warn_if_previous_workspace_state_update_fails(
        &repo.record_previous_workspace_after_switch(&workspace),
    );
    Ok(())
}

fn normalize_workspace_alias(repo: &NaviWorkspace, workspace: &str) -> String {
    if workspace == "@" {
        repo.current_workspace_name().as_str().to_owned()
    } else {
        workspace.to_owned()
    }
}

fn warn_if_previous_workspace_state_update_fails(result: &Result<()>) {
    if result.is_err() {
        eprintln!(
            "warning: failed to record previous workspace state\nhint: check .jj/repo/navi/state.toml permissions and contents"
        );
    }
}

fn emit_existing_switch(
    repo: &NaviWorkspace,
    workspace: &WorkspaceName,
    resolved_path: ResolvedWorkspacePath,
) -> Result<()> {
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

    emit_switch_destination(repo, &target_root)
}

fn emit_switch_destination(repo: &NaviWorkspace, target_root: &Path) -> Result<()> {
    let display_path = repo.display_path_for_switch(target_root);
    if !write_cd_directive(&display_path)? {
        println!("{}", display_path.display());
    }
    Ok(())
}
