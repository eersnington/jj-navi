use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use pathdiff::diff_paths;

use crate::error::{Error, Result};
use crate::types::{
    WorkspaceListStatus, WorkspaceMetadataStatus, WorkspaceName, WorkspacePathSource,
    WorkspacePathState, WorkspaceTemplate,
};

use super::discovery::resolve_repo_storage_path;
use super::jj::JjClient;
use super::metadata::WorkspaceMetadataStore;

pub(crate) const DEFAULT_WORKSPACE_NAME: &str = "default";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedWorkspacePath {
    pub(crate) path: PathBuf,
    pub(crate) state: WorkspacePathState,
    pub(crate) source: WorkspacePathSource,
}

impl ResolvedWorkspacePath {
    pub(crate) const fn is_switchable(&self) -> bool {
        matches!(
            self.state,
            WorkspacePathState::Confirmed | WorkspacePathState::Inferred
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CandidateState {
    Valid,
    Missing,
    Stale,
}

#[derive(Clone, Copy)]
pub(crate) struct WorkspaceTemplateInputs<'a> {
    pub(crate) repo_name: &'a str,
    pub(crate) template: &'a WorkspaceTemplate,
    pub(crate) allow_switchable_path: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct WorkspacePathResolutionOptions<'a> {
    pub(crate) current_workspace: Option<&'a WorkspaceName>,
    pub(crate) metadata: Option<&'a WorkspaceMetadataStore>,
    pub(crate) template: WorkspaceTemplateInputs<'a>,
}

pub(crate) fn derive_repo_name(
    workspace_root: &Path,
    current_workspace: &WorkspaceName,
) -> Result<String> {
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

pub(crate) fn derive_repo_name_for_doctor(
    repo_storage_path: &Path,
    workspace_root: &Path,
    current_workspace: Option<&WorkspaceName>,
) -> Result<String> {
    match current_workspace {
        Some(current_workspace) => derive_repo_name(workspace_root, current_workspace),
        None => repo_storage_path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            .map(str::to_owned)
            .ok_or(Error::RepoName),
    }
}

pub(crate) fn should_report_missing_navi_metadata(workspace: &WorkspaceName) -> bool {
    workspace.as_str() != DEFAULT_WORKSPACE_NAME
}

pub(crate) fn planned_workspace_root(
    workspace_root: &Path,
    current_workspace: Option<&WorkspaceName>,
    repo_name: &str,
    template: &WorkspaceTemplate,
    workspace: &WorkspaceName,
) -> PathBuf {
    if current_workspace == Some(workspace) {
        return workspace_root.to_path_buf();
    }

    let path = template.render(repo_name, workspace);
    if path.is_absolute() {
        path
    } else {
        workspace_root.join(path)
    }
}

pub(crate) fn display_path_for_list(workspace_root: &Path, target_root: &Path) -> PathBuf {
    diff_paths(target_root, workspace_root).unwrap_or_else(|| target_root.to_path_buf())
}

pub(crate) fn resolve_workspace_path_from_sources(
    workspace_root: &Path,
    repo_storage_path: &Path,
    workspace: &WorkspaceName,
    jj: &JjClient<'_>,
    options: WorkspacePathResolutionOptions<'_>,
) -> ResolvedWorkspacePath {
    if options.current_workspace == Some(workspace) {
        return ResolvedWorkspacePath {
            path: workspace_root.to_path_buf(),
            state: WorkspacePathState::Confirmed,
            source: WorkspacePathSource::CurrentWorkspace,
        };
    }

    let mut fallback = None;

    if let Ok(path) = jj.workspace_root(workspace) {
        let candidate = resolve_candidate_path(
            repo_storage_path,
            path,
            workspace,
            WorkspacePathSource::JjRecorded,
        );
        if candidate.is_switchable() {
            return candidate;
        }
        fallback = Some(candidate);
    }

    if let Some(path) = repo_primary_workspace_root(repo_storage_path, workspace) {
        let candidate = resolve_candidate_path(
            repo_storage_path,
            path,
            workspace,
            WorkspacePathSource::RepoPrimary,
        );
        if candidate.is_switchable() {
            return candidate;
        }
        if fallback.is_none() {
            fallback = Some(candidate);
        }
    }

    if let Some(metadata) = options.metadata
        && let Some(path) = metadata.workspace_path(workspace)
    {
        let candidate = resolve_candidate_path(
            repo_storage_path,
            path,
            workspace,
            WorkspacePathSource::NaviMetadata,
        );
        if candidate.is_switchable() {
            return candidate;
        }
        if fallback.is_none() {
            fallback = Some(candidate);
        }
    }

    let template = options.template;
    let candidate = resolve_candidate_path(
        repo_storage_path,
        planned_workspace_root(
            workspace_root,
            options.current_workspace,
            template.repo_name,
            template.template,
            workspace,
        ),
        workspace,
        WorkspacePathSource::Template,
    );
    if template.allow_switchable_path && candidate.is_switchable() {
        return candidate;
    }
    fallback.unwrap_or(candidate)
}

fn resolve_candidate_path(
    repo_storage_path: &Path,
    path: PathBuf,
    workspace: &WorkspaceName,
    source: WorkspacePathSource,
) -> ResolvedWorkspacePath {
    let state = match classify_candidate_path(repo_storage_path, &path, workspace) {
        CandidateState::Valid if source.is_inferred() => WorkspacePathState::Inferred,
        CandidateState::Valid => WorkspacePathState::Confirmed,
        CandidateState::Missing => WorkspacePathState::Missing,
        CandidateState::Stale => WorkspacePathState::Stale,
    };

    ResolvedWorkspacePath {
        path,
        state,
        source,
    }
}

fn classify_candidate_path(
    repo_storage_path: &Path,
    path: &Path,
    workspace: &WorkspaceName,
) -> CandidateState {
    if !path.is_dir() {
        return CandidateState::Missing;
    }

    if !path.join(".jj").is_dir() {
        return CandidateState::Stale;
    }

    let Ok(candidate_repo_storage_path) = resolve_repo_storage_path(path) else {
        return CandidateState::Stale;
    };
    let Ok(candidate_repo_storage_path) = fs::canonicalize(candidate_repo_storage_path) else {
        return CandidateState::Stale;
    };
    if candidate_repo_storage_path != repo_storage_path {
        return CandidateState::Stale;
    }

    let jj = JjClient::new(path);
    match jj.current_workspace_name() {
        Ok(current_workspace) if &current_workspace == workspace => CandidateState::Valid,
        _ => CandidateState::Stale,
    }
}

pub(crate) fn metadata_status(
    workspace: &WorkspaceName,
    metadata: &WorkspaceMetadataStore,
) -> WorkspaceMetadataStatus {
    if !metadata.contains_workspace(workspace) {
        return WorkspaceMetadataStatus::MissingRecord;
    }

    if metadata.workspace_path(workspace).is_some() {
        WorkspaceMetadataStatus::PresentWithPath
    } else {
        WorkspaceMetadataStatus::PresentWithoutPath
    }
}

fn repo_primary_workspace_root(
    repo_storage_path: &Path,
    workspace: &WorkspaceName,
) -> Option<PathBuf> {
    (workspace.as_str() == DEFAULT_WORKSPACE_NAME)
        .then_some(repo_storage_path)
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

pub(crate) fn primary_workspace_root(repo_storage_path: &Path) -> Option<PathBuf> {
    repo_storage_path
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

pub(crate) fn workspace_list_statuses(
    workspace: &WorkspaceName,
    resolved: &ResolvedWorkspacePath,
    metadata_status: WorkspaceMetadataStatus,
) -> Vec<WorkspaceListStatus> {
    let mut statuses = Vec::new();

    if resolved.source.is_inferred() {
        statuses.push(WorkspaceListStatus::Inferred);
    }

    match resolved.state {
        WorkspacePathState::Confirmed | WorkspacePathState::Inferred => {}
        WorkspacePathState::Missing => statuses.push(WorkspaceListStatus::Missing),
        WorkspacePathState::Stale => statuses.push(WorkspaceListStatus::Stale),
    }

    if matches!(metadata_status, WorkspaceMetadataStatus::MissingRecord)
        && should_report_missing_navi_metadata(workspace)
    {
        statuses.push(WorkspaceListStatus::JjOnly);
    }

    if statuses.is_empty() {
        statuses.push(WorkspaceListStatus::Ok);
    }

    statuses
}
