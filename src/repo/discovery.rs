use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

pub(crate) fn find_workspace_root(path: &Path) -> Result<PathBuf> {
    path.ancestors()
        .find(|ancestor| ancestor.join(".jj").is_dir())
        .map(Path::to_path_buf)
        .ok_or(Error::NotInWorkspace)
}

pub(crate) fn resolve_repo_storage_path(workspace_root: &Path) -> Result<PathBuf> {
    let repo_path = workspace_root.join(".jj").join("repo");

    if repo_path.is_dir() {
        return Ok(repo_path);
    }

    let pointer = fs::read_to_string(&repo_path)?;
    let pointer = pointer.trim();
    if pointer.is_empty() {
        return Err(Error::InvalidRepoPointer(repo_path));
    }

    let pointer_path = PathBuf::from(pointer);
    let resolved = if pointer_path.is_absolute() {
        pointer_path
    } else {
        workspace_root.join(".jj").join(pointer_path)
    };

    let resolved = fs::canonicalize(&resolved).map_err(|error| Error::RepoPointerResolution {
        path: repo_path.clone(),
        message: error.to_string(),
    })?;
    if !resolved.is_dir() {
        return Err(Error::InvalidRepoPointer(repo_path));
    }

    Ok(resolved)
}
