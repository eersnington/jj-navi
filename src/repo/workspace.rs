use std::fs;
use std::path::{Path, PathBuf};

use pathdiff::diff_paths;

use crate::error::{Error, Result};
use crate::types::{
    RepoConfig, WorkspaceHealthSnapshot, WorkspaceListEntry, WorkspaceMetadataStatus,
    WorkspaceName, WorkspacePathSnapshot, WorkspaceSnapshot,
};

use super::config::{ensure_repo_config, load_repo_config};
use super::discovery::{find_workspace_root, resolve_repo_storage_path};
use super::jj::JjClient;
use super::metadata::WorkspaceMetadataStore;
use super::paths::{
    ResolvedWorkspacePath, WorkspacePathResolutionOptions, WorkspaceTemplateInputs,
    derive_repo_name, display_path_for_list, metadata_status, planned_workspace_root,
    resolve_workspace_path_from_sources, workspace_list_statuses,
};

pub struct NaviWorkspace {
    cwd: PathBuf,
    workspace_root: PathBuf,
    repo_storage_path: PathBuf,
    current_workspace: WorkspaceName,
    config: RepoConfig,
    repo_name: String,
}

#[derive(Clone, Copy)]
pub(crate) struct WorkspaceSnapshotInputs<'a> {
    pub(crate) workspace_root: &'a Path,
    pub(crate) repo_storage_path: &'a Path,
    pub(crate) current_workspace: Option<&'a WorkspaceName>,
    pub(crate) config: &'a RepoConfig,
    pub(crate) repo_name: &'a str,
    pub(crate) metadata: &'a WorkspaceMetadataStore,
    pub(crate) metadata_is_valid: bool,
    pub(crate) allow_switchable_path: bool,
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
        let repo_storage_path = fs::canonicalize(resolve_repo_storage_path(&workspace_root)?)?;
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
        planned_workspace_root(
            &self.workspace_root,
            Some(&self.current_workspace),
            &self.repo_name,
            &self.config.workspace_template,
            workspace,
        )
    }

    #[must_use]
    pub(crate) fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    #[must_use]
    pub fn display_path_for_switch(&self, target_root: &Path) -> PathBuf {
        diff_paths(target_root, &self.cwd).unwrap_or_else(|| target_root.to_path_buf())
    }

    /// Resolve the best available workspace path for `switch`.
    ///
    /// # Errors
    ///
    /// Returns an error if repo-scoped metadata cannot be loaded.
    pub(crate) fn resolve_workspace_path(
        &self,
        workspace: &WorkspaceName,
    ) -> Result<ResolvedWorkspacePath> {
        let metadata = WorkspaceMetadataStore::load(&self.repo_storage_path)?;

        Ok(self.resolve_workspace_path_with_metadata(workspace, &metadata))
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
        metadata.record_workspace(
            workspace,
            &target_root,
            &self.config.workspace_template,
            revision,
        );
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
        let snapshots = self.list_workspace_snapshots()?;

        Ok(snapshots
            .iter()
            .map(|snapshot| self.list_entry(snapshot))
            .collect())
    }

    /// List repo workspaces as shared snapshots.
    ///
    /// # Errors
    ///
    /// Returns an error if `jj workspace list` fails or if a workspace name is
    /// invalid for navi.
    pub(crate) fn list_workspace_snapshots(&self) -> Result<Vec<WorkspaceSnapshot>> {
        let jj = JjClient::new(&self.workspace_root);
        let metadata = WorkspaceMetadataStore::load(&self.repo_storage_path)?;
        collect_workspace_snapshots(
            WorkspaceSnapshotInputs {
                workspace_root: &self.workspace_root,
                repo_storage_path: &self.repo_storage_path,
                current_workspace: Some(&self.current_workspace),
                config: &self.config,
                repo_name: &self.repo_name,
                metadata: &metadata,
                metadata_is_valid: true,
                allow_switchable_path: true,
            },
            &jj,
        )
    }

    fn workspace_snapshot(
        entry: super::jj::JjWorkspaceListEntry,
        resolved: &ResolvedWorkspacePath,
        metadata_status: WorkspaceMetadataStatus,
    ) -> WorkspaceSnapshot {
        let statuses = workspace_list_statuses(&entry.name, resolved, metadata_status);

        WorkspaceSnapshot {
            is_current: entry.is_current,
            name: entry.name,
            path: WorkspacePathSnapshot {
                path: resolved.path.clone(),
                state: resolved.state,
                source: resolved.source,
            },
            health: WorkspaceHealthSnapshot {
                statuses,
                metadata_status,
            },
            commit_id: entry.commit_id,
            message: entry.message,
        }
    }

    fn list_entry(&self, snapshot: &WorkspaceSnapshot) -> WorkspaceListEntry {
        let path = if snapshot.is_current {
            PathBuf::from(".")
        } else {
            display_path_for_list(&self.workspace_root, &snapshot.path.path)
        };

        WorkspaceListEntry {
            is_current: snapshot.is_current,
            name: snapshot.name.clone(),
            statuses: snapshot.health.statuses.clone(),
            path,
            path_state: snapshot.path.state,
            commit_id: snapshot.commit_id.clone(),
            message: snapshot.message.clone(),
        }
    }

    /// Resolve a workspace path from the strongest trustworthy source.
    ///
    /// Resolution order:
    /// - current workspace root discovered from the local filesystem
    /// - JJ-recorded workspace path
    /// - validated primary workspace root derived from shared repo storage
    /// - `navi` metadata path for navi-created workspaces
    /// - deterministic template path
    ///
    /// Non-current paths are validated before trust so `list` can keep working
    /// on stale JJ metadata while `switch` still avoids navigating into the
    /// wrong directory. `list` renders degraded rows inline; `switch` only
    /// succeeds for validated paths.
    fn resolve_workspace_path_with_metadata(
        &self,
        workspace: &WorkspaceName,
        metadata: &WorkspaceMetadataStore,
    ) -> ResolvedWorkspacePath {
        let jj = JjClient::new(&self.workspace_root);
        resolve_workspace_path_from_sources(
            &self.workspace_root,
            &self.repo_storage_path,
            workspace,
            &jj,
            WorkspacePathResolutionOptions {
                current_workspace: Some(&self.current_workspace),
                metadata: Some(metadata),
                template: WorkspaceTemplateInputs {
                    repo_name: &self.repo_name,
                    template: &self.config.workspace_template,
                    allow_switchable_path: true,
                },
            },
        )
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
}

pub(crate) fn collect_workspace_snapshots(
    inputs: WorkspaceSnapshotInputs<'_>,
    jj: &JjClient<'_>,
) -> Result<Vec<WorkspaceSnapshot>> {
    let workspace_entries = jj.list_workspaces()?;
    let mut snapshots = Vec::with_capacity(workspace_entries.len());

    for entry in workspace_entries {
        let resolved = resolve_workspace_path_from_sources(
            inputs.workspace_root,
            inputs.repo_storage_path,
            &entry.name,
            jj,
            WorkspacePathResolutionOptions {
                current_workspace: inputs.current_workspace,
                metadata: inputs.metadata_is_valid.then_some(inputs.metadata),
                template: WorkspaceTemplateInputs {
                    repo_name: inputs.repo_name,
                    template: &inputs.config.workspace_template,
                    allow_switchable_path: inputs.allow_switchable_path,
                },
            },
        );
        let metadata_status = metadata_status(&entry.name, inputs.metadata);
        snapshots.push(NaviWorkspace::workspace_snapshot(
            entry,
            &resolved,
            metadata_status,
        ));
    }

    snapshots.sort_by(|left, right| left.name.cmp(&right.name));

    Ok(snapshots)
}
