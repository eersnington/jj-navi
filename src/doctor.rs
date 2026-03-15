//! Typed diagnostics produced by `navi doctor`.

use serde::Serialize;

/// Severity level for a doctor finding.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorSeverity {
    /// Repo state is unhealthy and needs intervention.
    Error,
    /// Repo state is degraded but still usable.
    Warning,
    /// Repo state is notable but not unhealthy.
    Info,
}

impl DoctorSeverity {
    /// Return the lowercase label used in human-facing output.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

/// Stable code for a doctor finding.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorFindingCode {
    /// Current directory still has `.jj` but no live workspace registration.
    OrphanedWorkspace,
    /// Repo-scoped config could not be parsed or validated.
    InvalidRepoConfig,
    /// Repo-scoped metadata could not be parsed or validated.
    InvalidWorkspaceMetadata,
    /// Workspace path fell back to validated non-JJ data.
    WorkspacePathInferred,
    /// Workspace directory is missing.
    WorkspaceDirectoryMissing,
    /// Workspace directory exists but no longer validates.
    WorkspaceDirectoryStale,
    /// Metadata mentions a workspace that JJ no longer knows.
    MetadataOnlyWorkspace,
    /// JJ knows a workspace that navi metadata does not track.
    JjOnlyWorkspace,
    /// Shell could not be detected from `$SHELL`.
    ShellDetectionFailed,
    /// `$SHELL` points to an unsupported shell.
    UnsupportedShell,
    /// `$HOME` is missing, so shell rc checks cannot run.
    HomeDirectoryMissing,
    /// Shell rc file does not exist yet.
    ShellRcMissing,
    /// Shell rc file managed block markers are invalid.
    InvalidShellRcFile,
    /// Shell integration block is not installed.
    ShellIntegrationMissing,
}

/// Diagnostic scope for a doctor finding.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DoctorScope {
    /// Repo-wide finding.
    Repo,
    /// Workspace-scoped finding.
    Workspace {
        /// Affected workspace name.
        workspace: String,
    },
    /// Shell integration finding.
    Shell,
}

/// One doctor finding.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DoctorFinding {
    /// Severity of the finding.
    pub severity: DoctorSeverity,
    /// Stable finding code.
    pub code: DoctorFindingCode,
    /// Finding scope.
    pub scope: DoctorScope,
    /// Human-readable message.
    pub message: String,
    /// Optional filesystem path rendered for humans and JSON consumers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Optional remediation hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Doctor summary counters.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct DoctorSummary {
    /// Error count.
    pub errors: usize,
    /// Warning count.
    pub warnings: usize,
    /// Info count.
    pub info: usize,
}

/// Full doctor report.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DoctorReport {
    /// Ordered findings.
    pub findings: Vec<DoctorFinding>,
}

impl DoctorReport {
    /// Append a finding.
    pub fn push(&mut self, finding: DoctorFinding) {
        self.findings.push(finding);
    }

    /// Order findings deterministically.
    pub fn sort(&mut self) {
        self.findings.sort_by(|left, right| {
            left.severity
                .cmp(&right.severity)
                .then_with(|| left.scope.cmp(&right.scope))
                .then_with(|| left.code.cmp(&right.code))
                .then_with(|| left.message.cmp(&right.message))
        });
    }

    /// Count findings by severity.
    #[must_use]
    pub fn summary(&self) -> DoctorSummary {
        self.findings
            .iter()
            .fold(DoctorSummary::default(), |mut summary, finding| {
                match finding.severity {
                    DoctorSeverity::Error => summary.errors += 1,
                    DoctorSeverity::Warning => summary.warnings += 1,
                    DoctorSeverity::Info => summary.info += 1,
                }
                summary
            })
    }

    /// Whether the report contains any errors.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == DoctorSeverity::Error)
    }

    /// Whether the report is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }
}
