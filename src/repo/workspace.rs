use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use pathdiff::diff_paths;

use crate::cli::{ManagedBlockState, inspect_managed_block};
use crate::doctor::{DoctorFinding, DoctorFindingCode, DoctorReport, DoctorScope, DoctorSeverity};
use crate::error::{Error, Result};
use crate::types::{
    RepoConfig, WorkspaceListEntry, WorkspaceListStatus, WorkspaceName, WorkspacePathState,
    WorkspaceTemplate,
};

use super::config::{ensure_repo_config, load_repo_config};
use super::discovery::{find_workspace_root, resolve_repo_storage_path};
use super::jj::JjClient;
use super::metadata::{WorkspaceMetadataEntry, WorkspaceMetadataStore};

const DEFAULT_WORKSPACE_NAME: &str = "default";

pub struct NaviWorkspace {
    cwd: PathBuf,
    workspace_root: PathBuf,
    repo_storage_path: PathBuf,
    current_workspace: WorkspaceName,
    config: RepoConfig,
    repo_name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkspacePathSource {
    CurrentWorkspace,
    JjRecorded,
    RepoPrimary,
    NaviMetadata,
    Template,
}

impl WorkspacePathSource {
    const fn is_inferred(self) -> bool {
        matches!(self, Self::NaviMetadata | Self::Template)
    }

    pub(crate) const fn needs_switch_warning(self) -> bool {
        matches!(self, Self::Template)
    }
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorkspaceMetadataStatus {
    MissingRecord,
    PresentWithoutPath,
    PresentWithPath,
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

    /// Inspect repo health without mutating repo state.
    ///
    /// # Errors
    ///
    /// Returns an error if repo discovery fails or if required `jj` probes fail.
    pub fn doctor(path: &Path, command_name: &str) -> Result<DoctorReport> {
        let cwd = path.canonicalize()?;
        let workspace_root = find_workspace_root(&cwd)?;
        let repo_storage_path = fs::canonicalize(resolve_repo_storage_path(&workspace_root)?)?;
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
        let repo_name = derive_repo_name_for_doctor(&workspace_root, current_workspace.as_ref())?;
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
            .extend(repo.workspace_findings(&jj, &metadata)?);
        report.findings.extend(shell_findings(command_name)?);
        report.sort();
        Ok(report)
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
        let jj = JjClient::new(&self.workspace_root);
        let metadata = WorkspaceMetadataStore::load(&self.repo_storage_path)?;
        let workspace_entries = jj.list_workspaces()?;
        let mut entries = Vec::with_capacity(workspace_entries.len());

        for entry in workspace_entries {
            let resolved = self.resolve_workspace_path_with_metadata(&entry.name, &metadata);
            let metadata_status = metadata_status(&entry.name, &metadata);
            entries.push(self.workspace_entry(entry, &resolved, metadata_status));
        }

        entries.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(entries)
    }

    fn workspace_entry(
        &self,
        entry: super::jj::JjWorkspaceListEntry,
        resolved: &ResolvedWorkspacePath,
        metadata_status: WorkspaceMetadataStatus,
    ) -> WorkspaceListEntry {
        let path = if entry.is_current {
            PathBuf::from(".")
        } else {
            self.display_path_for_list(&resolved.path)
        };
        let statuses = workspace_list_statuses(&entry.name, resolved, metadata_status);

        WorkspaceListEntry {
            is_current: entry.is_current,
            name: entry.name,
            statuses,
            path,
            path_is_inferred: resolved.source.is_inferred(),
            path_state: resolved.state,
            commit_id: entry.commit_id,
            message: entry.message,
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
        if workspace == &self.current_workspace {
            return ResolvedWorkspacePath {
                path: self.workspace_root.clone(),
                state: WorkspacePathState::Confirmed,
                source: WorkspacePathSource::CurrentWorkspace,
            };
        }

        let jj = JjClient::new(&self.workspace_root);
        let mut fallback = None;

        if let Ok(path) = jj.workspace_root(workspace) {
            let candidate =
                self.resolve_candidate(path, workspace, WorkspacePathSource::JjRecorded);
            if candidate.is_switchable() {
                return candidate;
            }
            fallback = Some(candidate);
        }

        if let Some(path) = repo_primary_workspace_root(&self.repo_storage_path, workspace) {
            let candidate =
                self.resolve_candidate(path, workspace, WorkspacePathSource::RepoPrimary);
            if candidate.is_switchable() {
                return candidate;
            }
            if fallback.is_none() {
                fallback = Some(candidate);
            }
        }

        if let Some(path) = metadata.workspace_path(workspace) {
            let candidate =
                self.resolve_candidate(path, workspace, WorkspacePathSource::NaviMetadata);
            if candidate.is_switchable() {
                return candidate;
            }
            if fallback.is_none() {
                fallback = Some(candidate);
            }
        }

        let candidate = self.resolve_candidate(
            self.planned_workspace_root(workspace),
            workspace,
            WorkspacePathSource::Template,
        );
        if candidate.is_switchable() {
            return candidate;
        }
        fallback.unwrap_or(candidate)
    }

    fn resolve_candidate(
        &self,
        path: PathBuf,
        workspace: &WorkspaceName,
        source: WorkspacePathSource,
    ) -> ResolvedWorkspacePath {
        let state = match self.classify_candidate_path(&path, workspace) {
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

    fn classify_candidate_path(&self, path: &Path, workspace: &WorkspaceName) -> CandidateState {
        if !path.is_dir() {
            return CandidateState::Missing;
        }

        if !path.join(".jj").is_dir() {
            return CandidateState::Stale;
        }

        let Ok(repo_storage_path) = resolve_repo_storage_path(path) else {
            return CandidateState::Stale;
        };
        let Ok(repo_storage_path) = fs::canonicalize(repo_storage_path) else {
            return CandidateState::Stale;
        };
        if repo_storage_path != self.repo_storage_path {
            return CandidateState::Stale;
        }

        let jj = JjClient::new(path);
        match jj.current_workspace_name() {
            Ok(current_workspace) if &current_workspace == workspace => CandidateState::Valid,
            _ => CandidateState::Stale,
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
    fn planned_workspace_root(&self, workspace: &WorkspaceName) -> PathBuf {
        if self.current_workspace.as_ref() == Some(workspace) {
            return self.workspace_root.clone();
        }

        let path = self
            .config
            .workspace_template
            .render(&self.repo_name, workspace);
        if path.is_absolute() {
            path
        } else {
            self.workspace_root.join(path)
        }
    }

    fn display_path_for_list(&self, target_root: &Path) -> PathBuf {
        diff_paths(target_root, &self.workspace_root).unwrap_or_else(|| target_root.to_path_buf())
    }

    fn workspace_findings(
        &self,
        jj: &JjClient<'_>,
        metadata: &WorkspaceMetadataStore,
    ) -> Result<Vec<DoctorFinding>> {
        let workspace_entries = jj.list_workspaces()?;
        let mut findings = Vec::new();

        for entry in &workspace_entries {
            let resolved = self.resolve_workspace_path_with_metadata(&entry.name, metadata, jj);
            let metadata_status = metadata_status(&entry.name, metadata);
            match resolved.state {
                WorkspacePathState::Confirmed => {}
                WorkspacePathState::Inferred => {
                    findings.push(self.inferred_path_finding(&entry.name, &resolved));
                }
                WorkspacePathState::Missing => findings.push(workspace_finding(
                    DoctorSeverity::Warning,
                    DoctorFindingCode::WorkspaceDirectoryMissing,
                    &entry.name,
                    format!("workspace '{}' directory is missing", entry.name),
                    Some(format!(
                        "last known path: {}",
                        self.display_path_for_list(&resolved.path).display()
                    )),
                    Some(
                        self.display_path_for_list(&resolved.path)
                            .display()
                            .to_string(),
                    ),
                )),
                WorkspacePathState::Stale => findings.push(workspace_finding(
                    DoctorSeverity::Warning,
                    DoctorFindingCode::WorkspaceDirectoryStale,
                    &entry.name,
                    format!("workspace '{}' directory is stale", entry.name),
                    Some(format!(
                        "best known path no longer validates: {}",
                        self.display_path_for_list(&resolved.path).display()
                    )),
                    Some(
                        self.display_path_for_list(&resolved.path)
                            .display()
                            .to_string(),
                    ),
                )),
            }

            // Metadata record presence and recorded path availability are
            // distinct states. Pathless metadata remains valid fallback state
            // for path recovery; doctor must not infer missing metadata from a
            // missing stored path.
            if self.metadata_is_valid
                && matches!(metadata_status, WorkspaceMetadataStatus::MissingRecord)
                && should_report_missing_navi_metadata(&entry.name)
            {
                findings.push(workspace_finding(
                    DoctorSeverity::Info,
                    DoctorFindingCode::JjOnlyWorkspace,
                    &entry.name,
                    format!(
                        "workspace '{}' exists in jj but has no navi metadata",
                        entry.name
                    ),
                    None,
                    Some(
                        self.display_path_for_list(&resolved.path)
                            .display()
                            .to_string(),
                    ),
                ));
            }
        }

        if self.metadata_is_valid {
            for entry in metadata.entries() {
                if !workspace_entries
                    .iter()
                    .any(|workspace| workspace.name == entry.name)
                {
                    findings.push(self.metadata_only_finding(&entry));
                }
            }
        }

        Ok(findings)
    }

    fn inferred_path_finding(
        &self,
        workspace: &WorkspaceName,
        resolved: &ResolvedWorkspacePath,
    ) -> DoctorFinding {
        let display_path = self
            .display_path_for_list(&resolved.path)
            .display()
            .to_string();
        let (message, hint) = match resolved.source {
            WorkspacePathSource::NaviMetadata => (
                format!("workspace '{workspace}' is using a validated metadata fallback path"),
                format!("resolved from navi metadata: {display_path}"),
            ),
            WorkspacePathSource::Template => (
                format!("workspace '{workspace}' is using a validated template path"),
                format!("resolved from workspace template: {display_path}"),
            ),
            WorkspacePathSource::CurrentWorkspace
            | WorkspacePathSource::JjRecorded
            | WorkspacePathSource::RepoPrimary => {
                unreachable!("only inferred sources should reach doctor inferred-path findings")
            }
        };

        workspace_finding(
            DoctorSeverity::Info,
            DoctorFindingCode::WorkspacePathInferred,
            workspace,
            message,
            Some(hint),
            Some(display_path),
        )
    }

    fn metadata_only_finding(&self, entry: &WorkspaceMetadataEntry) -> DoctorFinding {
        let display_path = entry
            .path
            .as_ref()
            .map(|path| self.display_path_for_list(path).display().to_string());
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

    fn resolve_workspace_path_with_metadata(
        &self,
        workspace: &WorkspaceName,
        metadata: &WorkspaceMetadataStore,
        jj: &JjClient<'_>,
    ) -> ResolvedWorkspacePath {
        if self.current_workspace.as_ref() == Some(workspace) {
            return ResolvedWorkspacePath {
                path: self.workspace_root.clone(),
                state: WorkspacePathState::Confirmed,
                source: WorkspacePathSource::CurrentWorkspace,
            };
        }

        let mut fallback = None;

        if let Ok(path) = jj.workspace_root(workspace) {
            let candidate =
                self.resolve_candidate(path, workspace, WorkspacePathSource::JjRecorded);
            if candidate.is_switchable() {
                return candidate;
            }
            fallback = Some(candidate);
        }

        if let Some(path) = repo_primary_workspace_root(&self.repo_storage_path, workspace) {
            let candidate =
                self.resolve_candidate(path, workspace, WorkspacePathSource::RepoPrimary);
            if candidate.is_switchable() {
                return candidate;
            }
            if fallback.is_none() {
                fallback = Some(candidate);
            }
        }

        if self.metadata_is_valid
            && let Some(path) = metadata.workspace_path(workspace)
        {
            let candidate =
                self.resolve_candidate(path, workspace, WorkspacePathSource::NaviMetadata);
            if candidate.is_switchable() {
                return candidate;
            }
            if fallback.is_none() {
                fallback = Some(candidate);
            }
        }

        let candidate = self.resolve_candidate(
            self.planned_workspace_root(workspace),
            workspace,
            WorkspacePathSource::Template,
        );
        if !self.config_is_valid {
            return fallback.unwrap_or(candidate);
        }
        if candidate.is_switchable() {
            return candidate;
        }
        fallback.unwrap_or(candidate)
    }

    fn resolve_candidate(
        &self,
        path: PathBuf,
        workspace: &WorkspaceName,
        source: WorkspacePathSource,
    ) -> ResolvedWorkspacePath {
        let state = match self.classify_candidate_path(&path, workspace) {
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

    fn classify_candidate_path(&self, path: &Path, workspace: &WorkspaceName) -> CandidateState {
        if !path.is_dir() {
            return CandidateState::Missing;
        }

        if !path.join(".jj").is_dir() {
            return CandidateState::Stale;
        }

        let Ok(repo_storage_path) = resolve_repo_storage_path(path) else {
            return CandidateState::Stale;
        };
        let Ok(repo_storage_path) = fs::canonicalize(repo_storage_path) else {
            return CandidateState::Stale;
        };
        if repo_storage_path != self.repo_storage_path {
            return CandidateState::Stale;
        }

        let jj = JjClient::new(path);
        match jj.current_workspace_name() {
            Ok(current_workspace) if &current_workspace == workspace => CandidateState::Valid,
            _ => CandidateState::Stale,
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

fn derive_repo_name_for_doctor(
    workspace_root: &Path,
    current_workspace: Option<&WorkspaceName>,
) -> Result<String> {
    match current_workspace {
        Some(current_workspace) => derive_repo_name(workspace_root, current_workspace),
        None => workspace_root
            .file_name()
            .and_then(OsStr::to_str)
            .map(str::to_owned)
            .ok_or(Error::RepoName),
    }
}

fn shell_findings(command_name: &str) -> Result<Vec<DoctorFinding>> {
    let Ok(shell_var) = std::env::var("SHELL") else {
        return Ok(vec![shell_finding(
            DoctorSeverity::Warning,
            DoctorFindingCode::ShellDetectionFailed,
            String::from("unable to detect shell from $SHELL"),
            None,
            Some(String::from(
                "set $SHELL or pass --shell when installing integration",
            )),
        )]);
    };
    let shell_name = std::path::Path::new(&shell_var)
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or(Error::ShellDetection)?;
    let shell = match crate::types::ShellKind::new(shell_name) {
        Ok(shell) => shell,
        Err(Error::UnsupportedShell(shell)) => {
            return Ok(vec![shell_finding(
                DoctorSeverity::Warning,
                DoctorFindingCode::UnsupportedShell,
                format!("shell '{shell}' is not supported"),
                None,
                Some(String::from("supported shells: bash, zsh")),
            )]);
        }
        Err(error) => return Err(error),
    };

    let Ok(home) = std::env::var("HOME") else {
        return Ok(vec![shell_finding(
            DoctorSeverity::Warning,
            DoctorFindingCode::HomeDirectoryMissing,
            String::from("$HOME is not set; shell integration could not be checked"),
            None,
            None,
        )]);
    };
    let rc_path = PathBuf::from(home).join(shell.rc_file_name());
    let contents = match fs::read_to_string(&rc_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(vec![shell_finding(
                DoctorSeverity::Info,
                DoctorFindingCode::ShellRcMissing,
                format!("shell rc file {} does not exist yet", rc_path.display()),
                Some(rc_path.display().to_string()),
                Some(shell_install_hint(command_name, shell)),
            )]);
        }
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
            return Ok(vec![shell_finding(
                DoctorSeverity::Error,
                DoctorFindingCode::InvalidShellRcFile,
                format!("shell rc file {} is not valid UTF-8", rc_path.display()),
                Some(rc_path.display().to_string()),
                None,
            )]);
        }
        Err(error) => return Err(error.into()),
    };

    let finding = match inspect_managed_block(&contents) {
        ManagedBlockState::Missing => Some(shell_finding(
            DoctorSeverity::Info,
            DoctorFindingCode::ShellIntegrationMissing,
            format!(
                "shell integration managed block is missing from {}",
                rc_path.display()
            ),
            Some(rc_path.display().to_string()),
            Some(shell_install_hint(command_name, shell)),
        )),
        ManagedBlockState::Present { .. } => None,
        ManagedBlockState::Invalid(message) => Some(shell_finding(
            DoctorSeverity::Error,
            DoctorFindingCode::InvalidShellRcFile,
            format!("invalid shell rc file at {}", rc_path.display()),
            Some(rc_path.display().to_string()),
            Some(message.to_owned()),
        )),
    };

    Ok(finding.into_iter().collect())
}

fn should_report_missing_navi_metadata(workspace: &WorkspaceName) -> bool {
    // The default workspace commonly predates `navi` metadata and still
    // behaves correctly with JJ as the source of truth, so doctor treats
    // missing metadata there as expected rather than drift.
    workspace.as_str() != DEFAULT_WORKSPACE_NAME
}

fn metadata_status(
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

fn workspace_list_statuses(
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

fn shell_finding(
    severity: DoctorSeverity,
    code: DoctorFindingCode,
    message: String,
    path: Option<String>,
    hint: Option<String>,
) -> DoctorFinding {
    DoctorFinding {
        severity,
        code,
        scope: DoctorScope::Shell,
        message,
        path,
        hint,
    }
}

fn shell_install_hint(command_name: &str, shell: crate::types::ShellKind) -> String {
    format!(
        "run: {command_name} config shell install --shell {}",
        shell.as_str()
    )
}
