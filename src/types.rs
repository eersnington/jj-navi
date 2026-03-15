//! Core domain and presentation types used by `jj-navi`.

use clap::ValueEnum;
use std::fmt;
use std::path::PathBuf;

use crate::error::{Error, Result};

/// Validated workspace name.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WorkspaceName(String);

impl WorkspaceName {
    /// Create a validated workspace name.
    ///
    /// # Errors
    ///
    /// Returns an error if the name is empty, uses path separators, or
    /// contains whitespace.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();

        if value.is_empty()
            || value == "."
            || value == ".."
            || value.contains('/')
            || value.contains('\\')
            || value.chars().any(char::is_whitespace)
        {
            return Err(Error::InvalidWorkspaceName(value));
        }

        Ok(Self(value))
    }

    #[must_use]
    /// Borrow the validated workspace name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated workspace path template.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceTemplate(String);

impl WorkspaceTemplate {
    /// Create a validated workspace template.
    ///
    /// # Errors
    ///
    /// Returns an error if the template contains unsupported placeholders or
    /// unmatched braces.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_workspace_template(&value)?;
        Ok(Self(value))
    }

    #[must_use]
    /// Borrow the template as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    /// Render the template for a repo and workspace name.
    pub fn render(&self, repo: &str, workspace: &WorkspaceName) -> PathBuf {
        let mut rendered = String::new();
        let mut chars = self.0.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                let mut placeholder = String::new();

                for next in chars.by_ref() {
                    if next == '}' {
                        break;
                    }
                    placeholder.push(next);
                }

                match placeholder.as_str() {
                    "repo" => rendered.push_str(repo),
                    "workspace" => rendered.push_str(workspace.as_str()),
                    _ => {
                        rendered.push('{');
                        rendered.push_str(&placeholder);
                        rendered.push('}');
                    }
                }
            } else {
                rendered.push(ch);
            }
        }

        PathBuf::from(rendered)
    }
}

impl Default for WorkspaceTemplate {
    fn default() -> Self {
        Self(String::from("../{repo}.{workspace}"))
    }
}

/// Shell kinds supported by shell integration.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ShellKind {
    /// Bash shell.
    Bash,
    /// Zsh shell.
    Zsh,
}

impl ShellKind {
    /// Parse a supported shell kind.
    ///
    /// # Errors
    ///
    /// Returns an error if the shell is not supported.
    pub fn new(value: &str) -> Result<Self> {
        match value {
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            other => Err(Error::UnsupportedShell(other.to_owned())),
        }
    }

    /// Detect a supported shell from the `SHELL` environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error if `SHELL` is missing or unsupported.
    pub fn detect() -> Result<Self> {
        let shell = std::env::var("SHELL").map_err(|_| Error::ShellDetection)?;
        let shell_name = std::path::Path::new(&shell)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(Error::ShellDetection)?;
        Self::new(shell_name)
    }

    #[must_use]
    /// Return the shell name used in CLI output and shell code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
        }
    }

    #[must_use]
    /// Return the shell rc filename for this shell.
    pub fn rc_file_name(self) -> &'static str {
        match self {
            Self::Bash => ".bashrc",
            Self::Zsh => ".zshrc",
        }
    }
}

/// Repo-scoped `jj-navi` configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RepoConfig {
    /// Template used when planning new workspace paths.
    pub workspace_template: WorkspaceTemplate,
}

/// Render-ready workspace row for `navi list`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceListEntry {
    /// Whether this row represents the active workspace.
    pub is_current: bool,
    /// Workspace name.
    pub name: WorkspaceName,
    /// Compact status labels shown in the list table.
    pub statuses: Vec<WorkspaceListStatus>,
    /// Display path shown in the table.
    pub path: PathBuf,
    /// Whether the rendered path comes from `navi` fallback logic.
    pub path_is_inferred: bool,
    /// How trustworthy the rendered path is.
    pub path_state: WorkspacePathState,
    /// Short commit identifier.
    pub commit_id: String,
    /// First-line commit description.
    pub message: String,
}

/// Display state for a workspace path rendered by `navi list`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspacePathState {
    /// Path is confirmed from the current workspace or JJ.
    Confirmed,
    /// Path was inferred from validated `navi` fallback data.
    Inferred,
    /// Best known path does not exist on disk.
    Missing,
    /// Best known path exists but no longer validates.
    Stale,
}

/// Compact status label rendered by `navi list`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceListStatus {
    /// Workspace looks healthy.
    Ok,
    /// Workspace path came from validated fallback data.
    Inferred,
    /// Best known workspace path is missing.
    Missing,
    /// Best known workspace path is stale.
    Stale,
    /// JJ knows the workspace but `navi` metadata does not.
    JjOnly,
}

impl WorkspaceListStatus {
    /// Return the human-facing label for this status.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Inferred => "inferred",
            Self::Missing => "missing",
            Self::Stale => "stale",
            Self::JjOnly => "jj-only",
        }
    }
}

fn validate_workspace_template(value: &str) -> Result<()> {
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                let mut placeholder = String::new();

                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(next) => placeholder.push(next),
                        None => {
                            return Err(Error::InvalidWorkspaceTemplate(value.to_owned()));
                        }
                    }
                }

                if placeholder != "repo" && placeholder != "workspace" {
                    return Err(Error::InvalidWorkspaceTemplate(value.to_owned()));
                }
            }
            '}' => return Err(Error::InvalidWorkspaceTemplate(value.to_owned())),
            _ => {}
        }
    }

    Ok(())
}
