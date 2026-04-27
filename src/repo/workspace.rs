use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use pathdiff::diff_paths;

use crate::error::{Error, Result};
use crate::types::{
    MergePreview, MergePreviewWorkspace, MergePreviewWorkspaceRole, RepoConfig,
    WorkspaceDiffSnapshot, WorkspaceFreshnessSnapshot, WorkspaceFreshnessStatus,
    WorkspaceHealthSnapshot, WorkspaceListEntry, WorkspaceListStatus, WorkspaceMetadataStatus,
    WorkspaceName, WorkspacePathSnapshot, WorkspacePathState, WorkspaceSnapshot,
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
use super::state::RepoStateStore;

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
    pub(crate) fn current_workspace_name(&self) -> &WorkspaceName {
        &self.current_workspace
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

    /// Resolve the repo-scoped previous workspace for `switch -`.
    ///
    /// # Errors
    ///
    /// Returns an error if no previous workspace is recorded, if the recorded
    /// workspace no longer exists, or if repo-scoped state cannot be loaded.
    pub(crate) fn resolve_previous_workspace_path(
        &self,
    ) -> Result<(WorkspaceName, ResolvedWorkspacePath)> {
        let state = RepoStateStore::load(&self.repo_storage_path)?;
        let workspace = state
            .previous_workspace()
            .filter(|workspace| *workspace != &self.current_workspace)
            .cloned()
            .ok_or(Error::NoPreviousWorkspace)?;

        if !self.workspace_exists(&workspace)? {
            return Err(Error::PreviousWorkspaceNotFound(
                workspace.as_str().to_owned(),
            ));
        }

        Ok((workspace.clone(), self.resolve_workspace_path(&workspace)?))
    }

    /// Record the workspace being left after a successful switch.
    ///
    /// # Errors
    ///
    /// Returns an error if repo-scoped state cannot be loaded or saved.
    pub(crate) fn record_previous_workspace_after_switch(
        &self,
        target: &WorkspaceName,
    ) -> Result<()> {
        if target == &self.current_workspace {
            return Ok(());
        }

        RepoStateStore::save_previous_workspace(&self.repo_storage_path, &self.current_workspace)
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

    /// Resolve a non-current workspace directory that is safe for removal.
    ///
    /// # Errors
    ///
    /// Returns an error if the workspace is current, missing from `jj`, or its
    /// directory cannot be validated against the shared repo storage.
    pub fn resolve_removable_workspace_path(&self, workspace: &WorkspaceName) -> Result<PathBuf> {
        self.resolve_workspace_forget_target(workspace)?;

        let resolved_path = self.resolve_workspace_path(workspace)?;
        if !resolved_path.is_switchable() {
            let display_path = self.display_path_for_switch(&resolved_path.path);
            return Err(Error::WorkspaceDirectoryUnavailable {
                workspace: workspace.as_str().to_owned(),
                path: display_path.display().to_string(),
            });
        }
        self.ensure_workspace_directory_does_not_own_repo_storage(workspace, &resolved_path.path)?;

        Ok(resolved_path.path)
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
        let snapshots = self.list_fresh_workspace_snapshots()?;

        Ok(snapshots
            .iter()
            .map(|snapshot| self.list_entry(snapshot))
            .collect())
    }

    /// List repo workspaces after making healthy workspaces current.
    ///
    /// # Errors
    ///
    /// Returns an error if `jj workspace list` fails or if a workspace name is
    /// invalid for navi.
    pub(crate) fn list_fresh_workspace_snapshots(&self) -> Result<Vec<WorkspaceSnapshot>> {
        let metadata = WorkspaceMetadataStore::load(&self.repo_storage_path)?;
        let mut snapshots = self.discover_workspace_snapshots_with_metadata(&metadata)?;
        let freshness = snapshots
            .iter()
            .map(|snapshot| {
                let freshness = match snapshot.path.state {
                    WorkspacePathState::Confirmed | WorkspacePathState::Inferred => {
                        super::jj::snapshot_working_copy_at(&snapshot.path.path)
                    }
                    WorkspacePathState::Missing => WorkspaceFreshnessSnapshot::skipped_missing(),
                    WorkspacePathState::Stale => WorkspaceFreshnessSnapshot::skipped_stale(),
                };
                (snapshot.name.clone(), freshness)
            })
            .collect::<BTreeMap<_, _>>();

        self.refresh_workspace_targets(&mut snapshots)?;
        for snapshot in &mut snapshots {
            let freshness = freshness
                .get(&snapshot.name)
                .cloned()
                .unwrap_or_else(WorkspaceFreshnessSnapshot::skipped_untrusted);
            apply_workspace_details(snapshot, &metadata, freshness);
        }

        Ok(snapshots)
    }

    /// Build a read-only merge preview between two workspaces.
    ///
    /// # Errors
    ///
    /// Returns an error if either workspace is missing, stale, not current, or
    /// otherwise unsafe to use for a merge recommendation.
    pub fn merge_preview(
        &self,
        source: &WorkspaceName,
        target: Option<&WorkspaceName>,
    ) -> Result<MergePreview> {
        let target = target.unwrap_or(&self.current_workspace);
        if source == target {
            return Err(Error::MergePreviewSameWorkspace(source.as_str().to_owned()));
        }

        let snapshots = self.list_read_only_workspace_snapshots()?;
        let source = self.resolve_merge_preview_workspace(
            &snapshots,
            source,
            MergePreviewWorkspaceRole::Source,
        )?;
        let target = self.resolve_merge_preview_workspace(
            &snapshots,
            target,
            MergePreviewWorkspaceRole::Target,
        )?;
        let commands = vec![
            format!("jj duplicate {}", source.snapshot.change_id),
            format!("jj rebase -s <duplicate> -d {}", target.snapshot.change_id),
        ];

        Ok(MergePreview {
            source,
            target,
            commands,
        })
    }

    fn discover_workspace_snapshots_with_metadata(
        &self,
        metadata: &WorkspaceMetadataStore,
    ) -> Result<Vec<WorkspaceSnapshot>> {
        let jj = JjClient::new(&self.workspace_root);
        collect_workspace_snapshots(
            WorkspaceSnapshotInputs {
                workspace_root: &self.workspace_root,
                repo_storage_path: &self.repo_storage_path,
                current_workspace: Some(&self.current_workspace),
                config: &self.config,
                repo_name: &self.repo_name,
                metadata,
                metadata_is_valid: true,
                allow_switchable_path: true,
            },
            &jj,
        )
    }

    fn list_read_only_workspace_snapshots(&self) -> Result<Vec<WorkspaceSnapshot>> {
        let metadata = WorkspaceMetadataStore::load(&self.repo_storage_path)?;
        let mut snapshots = self.discover_workspace_snapshots_with_metadata(&metadata)?;
        for snapshot in &mut snapshots {
            if matches!(
                snapshot.path.state,
                WorkspacePathState::Confirmed | WorkspacePathState::Inferred
            ) {
                snapshot.diff = super::jj::diff_stat_at(&snapshot.path.path);
            }
            snapshot.age = crate::types::WorkspaceAgeSnapshot {
                created_at: metadata.workspace_created_at(&snapshot.name),
            };
        }
        Ok(snapshots)
    }

    fn refresh_workspace_targets(&self, snapshots: &mut [WorkspaceSnapshot]) -> Result<()> {
        let jj = JjClient::new(&self.workspace_root);
        let targets = jj
            .list_workspaces()?
            .into_iter()
            .map(|entry| (entry.name.clone(), entry))
            .collect::<BTreeMap<_, _>>();

        for snapshot in snapshots {
            if let Some(entry) = targets.get(&snapshot.name) {
                snapshot.is_current = entry.is_current;
                snapshot.commit_id.clone_from(&entry.commit_id);
                snapshot.change_id.clone_from(&entry.change_id);
                snapshot.message.clone_from(&entry.message);
            }
        }

        Ok(())
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
            change_id: entry.change_id,
            message: entry.message,
            freshness: WorkspaceFreshnessSnapshot::current(),
            diff: WorkspaceDiffSnapshot::unknown(),
            age: crate::types::WorkspaceAgeSnapshot::unknown(),
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
            change_id: snapshot.change_id.clone(),
            message: snapshot.message.clone(),
            freshness: snapshot.freshness.clone(),
            diff: snapshot.diff.clone(),
            age: snapshot.age.clone(),
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

    fn resolve_merge_preview_workspace(
        &self,
        snapshots: &[WorkspaceSnapshot],
        workspace: &WorkspaceName,
        role: MergePreviewWorkspaceRole,
    ) -> Result<MergePreviewWorkspace> {
        let matches = snapshots
            .iter()
            .filter(|snapshot| snapshot.name == *workspace)
            .collect::<Vec<_>>();
        let [snapshot] = matches.as_slice() else {
            return if matches.is_empty() {
                Err(Error::MergePreviewWorkspaceMissing {
                    role,
                    workspace: workspace.as_str().to_owned(),
                })
            } else {
                Err(Error::MergePreviewWorkspaceAmbiguous {
                    role,
                    workspace: workspace.as_str().to_owned(),
                })
            };
        };

        validate_merge_preview_snapshot(snapshot, role)?;

        Ok(MergePreviewWorkspace {
            snapshot: (*snapshot).clone(),
            display_path: display_path_for_list(&self.workspace_root, &snapshot.path.path),
        })
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

    fn ensure_workspace_directory_does_not_own_repo_storage(
        &self,
        workspace: &WorkspaceName,
        target_root: &Path,
    ) -> Result<()> {
        let target_root = fs::canonicalize(target_root)?;
        if self.repo_storage_path.starts_with(&target_root) {
            return Err(Error::CannotRemoveWorkspaceWithSharedRepoStorage {
                workspace: workspace.as_str().to_owned(),
                path: target_root.display().to_string(),
            });
        }

        Ok(())
    }
}

fn apply_workspace_details(
    snapshot: &mut WorkspaceSnapshot,
    metadata: &WorkspaceMetadataStore,
    freshness: WorkspaceFreshnessSnapshot,
) {
    let diff = if freshness.status == WorkspaceFreshnessStatus::Current
        && matches!(
            snapshot.path.state,
            WorkspacePathState::Confirmed | WorkspacePathState::Inferred
        ) {
        super::jj::diff_stat_at(&snapshot.path.path)
    } else {
        WorkspaceDiffSnapshot::unknown()
    };

    if matches!(
        freshness.status,
        WorkspaceFreshnessStatus::Failed | WorkspaceFreshnessStatus::TimedOut
    ) {
        snapshot
            .health
            .statuses
            .retain(|status| *status != WorkspaceListStatus::Ok);
        if !snapshot
            .health
            .statuses
            .contains(&WorkspaceListStatus::NotCurrent)
        {
            snapshot
                .health
                .statuses
                .push(WorkspaceListStatus::NotCurrent);
        }
    }

    snapshot.freshness = freshness;
    snapshot.diff = diff;
    snapshot.age = crate::types::WorkspaceAgeSnapshot {
        created_at: metadata.workspace_created_at(&snapshot.name),
    };
}

fn validate_merge_preview_snapshot(
    snapshot: &WorkspaceSnapshot,
    role: MergePreviewWorkspaceRole,
) -> Result<()> {
    let reason = match snapshot.path.state {
        WorkspacePathState::Confirmed | WorkspacePathState::Inferred => None,
        WorkspacePathState::Missing => Some(String::from("workspace path is missing")),
        WorkspacePathState::Stale => Some(String::from("workspace path is stale")),
    }
    .or_else(|| {
        (snapshot.freshness.status != WorkspaceFreshnessStatus::Current).then(|| {
            snapshot
                .freshness
                .reason
                .clone()
                .unwrap_or_else(|| String::from("workspace could not be made current"))
        })
    })
    .or_else(|| {
        snapshot
            .health
            .statuses
            .iter()
            .find(|status| {
                matches!(
                    status,
                    WorkspaceListStatus::Missing
                        | WorkspaceListStatus::Stale
                        | WorkspaceListStatus::NotCurrent
                )
            })
            .map(|status| format!("workspace health is {}", status.label()))
    });

    if let Some(reason) = reason {
        Err(Error::MergePreviewWorkspaceUnavailable {
            role,
            workspace: snapshot.name.as_str().to_owned(),
            reason,
        })
    } else {
        Ok(())
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
