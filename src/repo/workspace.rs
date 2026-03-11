use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use pathdiff::diff_paths;

use crate::error::{Error, Result};
use crate::types::{WorkspaceEntry, WorkspaceName};

use super::discovery::{find_workspace_root, resolve_repo_storage_path};
use super::jj::JjClient;

pub struct NaviWorkspace {
    cwd: PathBuf,
    workspace_root: PathBuf,
    repo_storage_path: PathBuf,
    current_workspace: WorkspaceName,
    repo_name: String,
}

impl NaviWorkspace {
    /// Open the nearest jj workspace from `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if `path` is not inside a jj workspace or if
    /// discovery needs a `jj` command that fails.
    pub fn open(path: &Path) -> Result<Self> {
        let cwd = path.canonicalize()?;
        let workspace_root = find_workspace_root(&cwd)?;
        let repo_storage_path = resolve_repo_storage_path(&workspace_root)?;
        let jj = JjClient::new(&workspace_root);
        let current_workspace = jj.current_workspace_name()?;
        let repo_name = derive_repo_name(&workspace_root, &current_workspace)?;

        Ok(Self {
            cwd,
            workspace_root,
            repo_storage_path,
            current_workspace,
            repo_name,
        })
    }

    #[must_use]
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    #[must_use]
    pub fn repo_storage_path(&self) -> &Path {
        &self.repo_storage_path
    }

    #[must_use]
    pub fn current_workspace(&self) -> &WorkspaceName {
        &self.current_workspace
    }

    /// Compute the absolute workspace root for `workspace`.
    ///
    /// # Errors
    ///
    /// Returns an error if the current workspace root has no parent.
    pub fn planned_workspace_root(&self, workspace: &WorkspaceName) -> Result<PathBuf> {
        if workspace == &self.current_workspace {
            return Ok(self.workspace_root.clone());
        }

        let parent = self
            .workspace_root
            .parent()
            .ok_or_else(|| Error::WorkspaceRootHasNoParent(self.workspace_root.clone()))?;

        Ok(parent.join(format!("{}.{}", self.repo_name, workspace.as_str())))
    }

    #[must_use]
    pub fn planned_workspace_display(&self, workspace: &WorkspaceName) -> PathBuf {
        if workspace == &self.current_workspace {
            PathBuf::from(".")
        } else {
            PathBuf::from(format!("../{}.{}", self.repo_name, workspace.as_str()))
        }
    }

    #[must_use]
    pub fn display_path_for_switch(&self, target_root: &Path) -> PathBuf {
        diff_paths(target_root, &self.cwd).unwrap_or_else(|| target_root.to_path_buf())
    }

    /// Check if the target workspace directory already exists.
    ///
    /// # Errors
    ///
    /// Returns an error if path planning fails.
    pub fn workspace_exists(&self, workspace: &WorkspaceName) -> Result<bool> {
        Ok(self.planned_workspace_root(workspace)?.is_dir())
    }

    /// Create a workspace via `jj workspace add`.
    ///
    /// # Errors
    ///
    /// Returns an error if path planning fails or if `jj` returns an error.
    pub fn create_workspace(
        &self,
        workspace: &WorkspaceName,
        revision: Option<&str>,
    ) -> Result<PathBuf> {
        let target_root = self.planned_workspace_root(workspace)?;
        let jj = JjClient::new(&self.workspace_root);

        jj.workspace_add(workspace, &target_root, revision)?;

        Ok(target_root)
    }

    /// List repo workspaces with navi display paths.
    ///
    /// # Errors
    ///
    /// Returns an error if `jj workspace list` fails or if a workspace name is
    /// invalid for navi.
    pub fn list_workspaces(&self) -> Result<Vec<WorkspaceEntry>> {
        let jj = JjClient::new(&self.workspace_root);

        let mut entries = jj
            .list_workspaces()?
            .into_iter()
            .map(|entry| self.workspace_entry(entry.name, entry.is_current))
            .collect::<Vec<_>>();

        entries.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(entries)
    }

    fn workspace_entry(&self, name: WorkspaceName, is_current: bool) -> WorkspaceEntry {
        let path = if is_current {
            PathBuf::from(".")
        } else {
            self.planned_workspace_display(&name)
        };

        WorkspaceEntry { name, path }
    }
}

fn derive_repo_name(workspace_root: &Path, current_workspace: &WorkspaceName) -> Result<String> {
    let basename = workspace_root
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or(Error::RepoName)?;

    let suffix = format!(".{}", current_workspace.as_str());

    if basename.ends_with(&suffix) {
        let repo_name = basename.trim_end_matches(&suffix);
        if repo_name.is_empty() {
            return Err(Error::RepoName);
        }
        Ok(repo_name.to_owned())
    } else {
        Ok(basename.to_owned())
    }
}
