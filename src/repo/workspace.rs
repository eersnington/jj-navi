use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use pathdiff::diff_paths;

use crate::error::{Error, Result};
use crate::types::{RepoConfig, WorkspaceListEntry, WorkspaceName, WorkspaceTemplate};

use super::config::{ensure_repo_config, load_repo_config};
use super::discovery::{find_workspace_root, resolve_repo_storage_path};
use super::jj::JjClient;
use super::metadata::WorkspaceMetadataStore;

pub struct NaviWorkspace {
    cwd: PathBuf,
    workspace_root: PathBuf,
    repo_storage_path: PathBuf,
    current_workspace: WorkspaceName,
    config: RepoConfig,
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
        jj.ensure_supported_version()?;
        let config = load_repo_config(&repo_storage_path)?;
        let current_workspace = jj.current_workspace_name()?;
        let repo_name = derive_repo_name(&workspace_root, &current_workspace)?;

        Ok(Self {
            cwd,
            workspace_root,
            repo_storage_path,
            current_workspace,
            config,
            repo_name,
        })
    }

    /// Compute the absolute workspace root for `workspace`.
    #[must_use]
    pub fn planned_workspace_root(&self, workspace: &WorkspaceName) -> PathBuf {
        if workspace == &self.current_workspace {
            return self.workspace_root.clone();
        }

        self.workspace_root_from_template(&self.config.workspace_template, workspace)
    }

    #[must_use]
    pub fn display_path_for_switch(&self, target_root: &Path) -> PathBuf {
        diff_paths(target_root, &self.cwd).unwrap_or_else(|| target_root.to_path_buf())
    }

    #[must_use]
    pub fn display_path_for_list(&self, target_root: &Path) -> PathBuf {
        diff_paths(target_root, &self.workspace_root).unwrap_or_else(|| target_root.to_path_buf())
    }

    /// Resolve the actual workspace root recorded by `jj`.
    ///
    /// # Errors
    ///
    /// Returns an error if `jj workspace root --name` fails.
    pub fn actual_workspace_root(&self, workspace: &WorkspaceName) -> Result<PathBuf> {
        let jj = JjClient::new(&self.workspace_root);

        jj.workspace_root(workspace)
    }

    /// Check if the target workspace directory already exists.
    ///
    /// # Errors
    ///
    /// Returns an error if `jj workspace list` fails.
    pub fn workspace_exists(&self, workspace: &WorkspaceName) -> Result<bool> {
        let jj = JjClient::new(&self.workspace_root);

        Ok(jj
            .list_workspaces()?
            .into_iter()
            .any(|entry| entry.name == *workspace))
    }

    /// Forget a workspace via `jj workspace forget`.
    ///
    /// # Errors
    ///
    /// Returns an error if the workspace does not exist or if `jj` returns an
    /// error.
    pub fn forget_workspace(&self, workspace: &WorkspaceName) -> Result<WorkspaceName> {
        let mut metadata = WorkspaceMetadataStore::load(&self.repo_storage_path)?;
        let workspace = self.resolve_workspace_forget_target(workspace)?;
        let jj = JjClient::new(&self.workspace_root);

        jj.workspace_forget(&workspace)?;
        metadata.remove_workspace(&workspace);
        metadata.save()?;

        Ok(workspace)
    }

    /// Create a workspace via `jj workspace add`.
    ///
    /// # Errors
    ///
    /// Returns an error if `jj` returns an error.
    pub fn create_workspace(
        &self,
        workspace: &WorkspaceName,
        revision: Option<&str>,
    ) -> Result<PathBuf> {
        let mut metadata = WorkspaceMetadataStore::load(&self.repo_storage_path)?;
        let target_root = self.planned_workspace_root(workspace);
        let jj = JjClient::new(&self.workspace_root);

        ensure_repo_config(&self.repo_storage_path, &self.config)?;

        jj.workspace_add(workspace, &target_root, revision)?;
        metadata.record_workspace(workspace, &self.config.workspace_template, revision);
        metadata.save()?;

        Ok(target_root)
    }

    /// List repo workspaces with navi display paths.
    ///
    /// # Errors
    ///
    /// Returns an error if `jj workspace list` fails or if a workspace name is
    /// invalid for navi.
    pub fn list_workspaces(&self) -> Result<Vec<WorkspaceListEntry>> {
        let jj = JjClient::new(&self.workspace_root);
        let workspace_entries = jj.list_workspaces()?;
        let mut entries = Vec::with_capacity(workspace_entries.len());

        for entry in workspace_entries {
            let actual_root = jj.workspace_root(&entry.name)?;
            entries.push(self.workspace_entry(entry, &actual_root));
        }

        entries.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(entries)
    }

    fn workspace_entry(
        &self,
        entry: super::jj::JjWorkspaceListEntry,
        actual_root: &Path,
    ) -> WorkspaceListEntry {
        let path = if entry.is_current {
            PathBuf::from(".")
        } else {
            self.display_path_for_list(actual_root)
        };

        WorkspaceListEntry {
            is_current: entry.is_current,
            name: entry.name,
            path,
            commit_id: entry.commit_id,
            message: entry.message,
        }
    }

    fn resolve_workspace_forget_target(&self, workspace: &WorkspaceName) -> Result<WorkspaceName> {
        if workspace == &self.current_workspace {
            return Err(Error::CannotRemoveCurrentWorkspace);
        }

        let jj = JjClient::new(&self.workspace_root);
        let exists = jj
            .list_workspaces()?
            .into_iter()
            .any(|entry| entry.name == *workspace);

        if exists {
            Ok(workspace.clone())
        } else {
            Err(Error::WorkspaceNotFound(workspace.as_str().to_owned()))
        }
    }

    fn workspace_root_from_template(
        &self,
        template: &WorkspaceTemplate,
        workspace: &WorkspaceName,
    ) -> PathBuf {
        let path = template.render(&self.repo_name, workspace);

        if path.is_absolute() {
            path
        } else {
            self.workspace_root.join(path)
        }
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
