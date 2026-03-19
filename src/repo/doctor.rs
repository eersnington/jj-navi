use std::path::{Path, PathBuf};

use crate::diagnostics::{
    DoctorFinding, DoctorFindingCode, DoctorReport, DoctorScope, DoctorSeverity,
};
use crate::error::{Error, Result};
use crate::shell;
use crate::types::{
    RepoConfig, WorkspaceMetadataStatus, WorkspaceName, WorkspacePathSource, WorkspacePathState,
};

use super::config::load_repo_config;
use super::discovery::{find_workspace_root, resolve_repo_storage_path};
use super::jj::JjClient;
use super::metadata::{WorkspaceMetadataEntry, WorkspaceMetadataStore};
use super::paths::{
    derive_repo_name_for_doctor, display_path_for_list, should_report_missing_navi_metadata,
};
use super::workspace::{WorkspaceSnapshotInputs, collect_workspace_snapshots};

pub(crate) fn build_doctor_report(path: &Path, command_name: &str) -> Result<DoctorReport> {
    let cwd = path.canonicalize()?;
    let workspace_root = find_workspace_root(&cwd)?;
    let repo_storage_path = std::fs::canonicalize(resolve_repo_storage_path(&workspace_root)?)?;
    let mut report = DoctorReport::default();
    let current_workspace = {
        let jj = JjClient::new(&workspace_root);
        jj.ensure_supported_version()?;
        match jj.current_workspace_name() {
            Ok(current_workspace) => Some(current_workspace),
            Err(Error::OrphanedWorkspace) => {
                report.push(DoctorFinding {
                    severity: DoctorSeverity::Error,
                    code: DoctorFindingCode::OrphanedWorkspace,
                    scope: DoctorScope::Repo,
                    message: String::from(
                        "current directory is no longer a registered jj workspace",
                    ),
                    path: Some(workspace_root.display().to_string()),
                    hint: Some(String::from(
                        "cd into another workspace or recreate this workspace with jj",
                    )),
                });
                None
            }
            Err(error) => return Err(error),
        }
    };
    let repo_name = derive_repo_name_for_doctor(
        &repo_storage_path,
        &workspace_root,
        current_workspace.as_ref(),
    )?;
    let (config, config_is_valid) = match load_repo_config(&repo_storage_path) {
        Ok(config) => (config, true),
        Err(Error::InvalidRepoConfig { path, message }) => {
            report.push(DoctorFinding {
                severity: DoctorSeverity::Error,
                code: DoctorFindingCode::InvalidRepoConfig,
                scope: DoctorScope::Repo,
                message: format!("invalid repo config in {}", path.display()),
                path: Some(path.display().to_string()),
                hint: Some(message),
            });
            (RepoConfig::default(), false)
        }
        Err(error) => return Err(error),
    };
    let (metadata, metadata_is_valid) = match WorkspaceMetadataStore::load(&repo_storage_path) {
        Ok(metadata) => (metadata, true),
        Err(Error::InvalidWorkspaceMetadata { path, message }) => {
            report.push(DoctorFinding {
                severity: DoctorSeverity::Error,
                code: DoctorFindingCode::InvalidWorkspaceMetadata,
                scope: DoctorScope::Repo,
                message: format!("invalid workspace metadata in {}", path.display()),
                path: Some(path.display().to_string()),
                hint: Some(message),
            });
            (WorkspaceMetadataStore::default(), false)
        }
        Err(error) => return Err(error),
    };
    let repo = DoctorWorkspace {
        workspace_root,
        repo_storage_path,
        current_workspace,
        config,
        config_is_valid,
        metadata_is_valid,
        repo_name,
    };
    let jj = JjClient::new(&repo.workspace_root);

    report
        .findings
        .extend(repo.collect_workspace_findings(&jj, &metadata)?);
    report
        .findings
        .extend(shell::doctor_findings(command_name)?);
    report.sort();
    Ok(report)
}

struct DoctorWorkspace {
    workspace_root: PathBuf,
    repo_storage_path: PathBuf,
    current_workspace: Option<WorkspaceName>,
    config: RepoConfig,
    config_is_valid: bool,
    metadata_is_valid: bool,
    repo_name: String,
}

impl DoctorWorkspace {
    fn collect_workspace_findings(
        &self,
        jj: &JjClient<'_>,
        metadata: &WorkspaceMetadataStore,
    ) -> Result<Vec<DoctorFinding>> {
        let snapshots = collect_workspace_snapshots(
            WorkspaceSnapshotInputs {
                workspace_root: &self.workspace_root,
                repo_storage_path: &self.repo_storage_path,
                current_workspace: self.current_workspace.as_ref(),
                config: &self.config,
                repo_name: &self.repo_name,
                metadata,
                metadata_is_valid: self.metadata_is_valid,
                allow_switchable_path: self.config_is_valid,
            },
            jj,
        )?;
        let mut findings = Vec::new();

        for snapshot in &snapshots {
            let display_path = display_path_for_list(&self.workspace_root, &snapshot.path.path)
                .display()
                .to_string();
            match snapshot.path.state {
                WorkspacePathState::Confirmed => {}
                WorkspacePathState::Inferred => {
                    let (message, hint) = match snapshot.path.source {
                        WorkspacePathSource::NaviMetadata => (
                            format!(
                                "workspace '{}' is using a validated metadata fallback path",
                                snapshot.name
                            ),
                            format!("resolved from navi metadata: {display_path}"),
                        ),
                        WorkspacePathSource::Template => (
                            format!(
                                "workspace '{}' is using a validated template path",
                                snapshot.name
                            ),
                            format!("resolved from workspace template: {display_path}"),
                        ),
                        WorkspacePathSource::CurrentWorkspace
                        | WorkspacePathSource::JjRecorded
                        | WorkspacePathSource::RepoPrimary => {
                            debug_assert!(
                                false,
                                "inferred workspace path used non-inferred source: {:?}",
                                snapshot.path.source
                            );
                            continue;
                        }
                    };
                    findings.push(inferred_path_finding(
                        &snapshot.name,
                        message,
                        hint,
                        display_path.clone(),
                    ));
                }
                WorkspacePathState::Missing => findings.push(workspace_finding(
                    DoctorSeverity::Warning,
                    DoctorFindingCode::WorkspaceDirectoryMissing,
                    &snapshot.name,
                    format!("workspace '{}' directory is missing", snapshot.name),
                    Some(format!("last known path: {display_path}")),
                    Some(display_path.clone()),
                )),
                WorkspacePathState::Stale => findings.push(workspace_finding(
                    DoctorSeverity::Warning,
                    DoctorFindingCode::WorkspaceDirectoryStale,
                    &snapshot.name,
                    format!("workspace '{}' directory is stale", snapshot.name),
                    Some(format!(
                        "best known path no longer validates: {display_path}"
                    )),
                    Some(display_path.clone()),
                )),
            }

            if self.metadata_is_valid
                && matches!(
                    snapshot.health.metadata_status,
                    WorkspaceMetadataStatus::MissingRecord
                )
                && should_report_missing_navi_metadata(&snapshot.name)
            {
                findings.push(workspace_finding(
                    DoctorSeverity::Info,
                    DoctorFindingCode::JjOnlyWorkspace,
                    &snapshot.name,
                    format!(
                        "workspace '{}' exists in jj but has no navi metadata",
                        snapshot.name
                    ),
                    None,
                    Some(display_path),
                ));
            }
        }

        if self.metadata_is_valid {
            findings.extend(self.collect_metadata_only_findings(&snapshots, metadata));
        }

        Ok(findings)
    }

    fn collect_metadata_only_findings(
        &self,
        snapshots: &[crate::types::WorkspaceSnapshot],
        metadata: &WorkspaceMetadataStore,
    ) -> Vec<DoctorFinding> {
        metadata
            .entries()
            .into_iter()
            .filter(|entry| !snapshots.iter().any(|snapshot| snapshot.name == entry.name))
            .map(|entry| self.metadata_only_finding(&entry))
            .collect()
    }

    fn metadata_only_finding(&self, entry: &WorkspaceMetadataEntry) -> DoctorFinding {
        let display_path = entry.path.as_ref().map(|path| {
            display_path_for_list(&self.workspace_root, path)
                .display()
                .to_string()
        });
        DoctorFinding {
            severity: DoctorSeverity::Warning,
            code: DoctorFindingCode::MetadataOnlyWorkspace,
            scope: DoctorScope::Workspace {
                workspace: entry.name.as_str().to_owned(),
            },
            message: format!(
                "metadata exists for workspace '{}' but jj no longer lists it",
                entry.name
            ),
            path: display_path,
            hint: Some(String::from("safe prune candidate")),
        }
    }
}

fn inferred_path_finding(
    workspace: &WorkspaceName,
    message: String,
    hint: String,
    display_path: String,
) -> DoctorFinding {
    workspace_finding(
        DoctorSeverity::Info,
        DoctorFindingCode::WorkspacePathInferred,
        workspace,
        message,
        Some(hint),
        Some(display_path),
    )
}

fn workspace_finding(
    severity: DoctorSeverity,
    code: DoctorFindingCode,
    workspace: &WorkspaceName,
    message: String,
    hint: Option<String>,
    path: Option<String>,
) -> DoctorFinding {
    DoctorFinding {
        severity,
        code,
        scope: DoctorScope::Workspace {
            workspace: workspace.as_str().to_owned(),
        },
        message,
        path,
        hint,
    }
}
