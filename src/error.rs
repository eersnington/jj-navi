use std::path::PathBuf;

use thiserror::Error;

/// Crate-wide error type for CLI, discovery, and `jj` integration failures.
#[derive(Debug, Error)]
pub enum Error {
    /// The current directory is not inside a Jujutsu workspace.
    #[error("error: not in a jj workspace")]
    NotInWorkspace,

    /// A workspace name violates `jj-navi` validation rules.
    #[error("error: invalid workspace name '{0}'")]
    InvalidWorkspaceName(String),

    /// The current directory still contains `.jj`, but is no longer a live workspace.
    #[error(
        "error: current directory is no longer a registered jj workspace\nhint: cd into another workspace or recreate this workspace with jj"
    )]
    OrphanedWorkspace,

    /// The repo name could not be derived from the current workspace root.
    #[error("error: failed to determine repo name")]
    RepoName,

    /// The workspace root unexpectedly has no parent directory.
    #[error("error: workspace root has no parent: {0}")]
    WorkspaceRootHasNoParent(PathBuf),

    /// The requested workspace does not exist.
    #[error("error: workspace does not exist\nhint: use --create")]
    WorkspaceDoesNotExist,

    /// The named workspace does not exist in `jj`.
    #[error("error: workspace '{0}' does not exist")]
    WorkspaceNotFound(String),

    /// The workspace exists, but no validated directory could be found.
    #[error(
        "error: workspace '{workspace}' exists, but its directory could not be resolved\nhint: last known path: {path}"
    )]
    WorkspaceDirectoryUnavailable {
        /// Workspace name.
        workspace: String,
        /// Best-known display path.
        path: String,
    },

    /// Removing the current workspace would orphan the active directory.
    #[error("error: cannot remove current workspace\nhint: switch to another workspace first")]
    CannotRemoveCurrentWorkspace,

    /// The `.jj/repo` pointer file is empty or points to a non-directory.
    #[error("error: invalid repo pointer in {0}")]
    InvalidRepoPointer(PathBuf),

    /// The `.jj/repo` pointer could not be resolved to an on-disk path.
    #[error("error: invalid repo pointer in {path}\n{message}")]
    RepoPointerResolution {
        /// Path to the pointer file that failed to resolve.
        path: PathBuf,
        /// Underlying resolution error message.
        message: String,
    },

    /// The configured workspace template is syntactically invalid.
    #[error("error: invalid workspace template '{0}'")]
    InvalidWorkspaceTemplate(String),

    /// Repo config could not be parsed or validated.
    #[error("error: invalid repo config in {path}\n{message}")]
    InvalidRepoConfig {
        /// Config file path.
        path: PathBuf,
        /// Validation or parse message.
        message: String,
    },

    /// Workspace metadata could not be parsed or validated.
    #[error("error: invalid workspace metadata in {path}\n{message}")]
    InvalidWorkspaceMetadata {
        /// Metadata file path.
        path: PathBuf,
        /// Validation or parse message.
        message: String,
    },

    /// `jj workspace list` returned output that `jj-navi` could not parse.
    #[error("error: invalid jj workspace list entry\n{0}")]
    InvalidJjWorkspaceListEntry(String),

    /// The requested shell is not supported.
    #[error("error: unsupported shell '{0}'")]
    UnsupportedShell(String),

    /// A shell argument is required for shell-init generation.
    #[error("error: shell name required\nhint: use one of: bash, zsh")]
    ShellRequired,

    /// The current shell could not be inferred from `$SHELL`.
    #[error("error: unable to detect shell from $SHELL")]
    ShellDetection,

    /// `$HOME` is required for shell installation.
    #[error("error: $HOME is not set")]
    HomeDirectory,

    /// The target shell rc file contains an invalid managed block.
    #[error("error: invalid shell rc file at {path}\n{message}")]
    InvalidShellRcFile {
        /// Shell rc path.
        path: PathBuf,
        /// Validation message.
        message: &'static str,
    },

    /// Shell integration requires a UTF-8 renderable path.
    #[error("error: shell integration requires a UTF-8 workspace path")]
    ShellDirectivePathNotUtf8,

    /// A `jj` command failed.
    #[error("error: jj command failed: {command}\n{stderr}")]
    JjCommandFailed {
        /// Rendered `jj` command line.
        command: String,
        /// Trimmed stderr output from `jj`.
        stderr: String,
    },

    /// The installed `jj` version is older than the supported floor.
    #[error("error: jj {minimum} or newer required\nhint: found {found}")]
    UnsupportedJjVersion {
        /// Installed `jj --version` output.
        found: String,
        /// Minimum supported version.
        minimum: &'static str,
    },

    /// An underlying I/O operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Crate-wide result alias.
pub type Result<T> = std::result::Result<T, Error>;
