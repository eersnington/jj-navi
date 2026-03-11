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

    let resolved =
        fs::canonicalize(&resolved).map_err(|_| Error::InvalidRepoPointer(repo_path.clone()))?;
    if !resolved.is_dir() {
        return Err(Error::InvalidRepoPointer(repo_path));
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use crate::error::Error;

    use super::resolve_repo_storage_path;

    #[test]
    fn resolves_relative_repo_pointer() {
        let temp = TempDir::new().expect("temp dir");
        let workspace_root = temp.path().join("workspace");
        let shared = temp.path().join("shared").join("repo");

        fs::create_dir_all(workspace_root.join(".jj")).expect("workspace .jj");
        fs::create_dir_all(&shared).expect("shared repo");
        fs::write(
            workspace_root.join(".jj").join("repo"),
            "../../shared/repo\n",
        )
        .expect("write repo pointer");

        let resolved = resolve_repo_storage_path(&workspace_root).expect("resolve repo pointer");

        assert_eq!(
            resolved,
            fs::canonicalize(shared).expect("canonical shared repo")
        );
    }

    #[test]
    fn rejects_missing_repo_pointer_target() {
        let temp = TempDir::new().expect("temp dir");
        let workspace_root = temp.path().join("workspace");
        let repo_pointer = workspace_root.join(".jj").join("repo");

        fs::create_dir_all(workspace_root.join(".jj")).expect("workspace .jj");
        fs::write(&repo_pointer, "../../shared/repo\n").expect("write repo pointer");

        let error = resolve_repo_storage_path(&workspace_root).expect_err("missing repo target");

        assert!(matches!(error, Error::InvalidRepoPointer(path) if path == repo_pointer));
    }

    #[test]
    fn rejects_repo_pointer_to_non_directory() {
        let temp = TempDir::new().expect("temp dir");
        let workspace_root = temp.path().join("workspace");
        let shared_file = temp.path().join("shared").join("repo");
        let repo_pointer = workspace_root.join(".jj").join("repo");

        fs::create_dir_all(workspace_root.join(".jj")).expect("workspace .jj");
        fs::create_dir_all(shared_file.parent().expect("shared file parent"))
            .expect("shared file parent dir");
        fs::write(&shared_file, "not a dir").expect("write shared file");
        fs::write(&repo_pointer, "../../shared/repo\n").expect("write repo pointer");

        let error =
            resolve_repo_storage_path(&workspace_root).expect_err("repo pointer to file target");

        assert!(matches!(error, Error::InvalidRepoPointer(path) if path == repo_pointer));
    }
}
