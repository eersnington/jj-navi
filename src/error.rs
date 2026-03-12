use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("error: not in a jj workspace")]
    NotInWorkspace,

    #[error("error: invalid workspace name '{0}'")]
    InvalidWorkspaceName(String),

    #[error("error: failed to determine repo name")]
    RepoName,

    #[error("error: workspace root has no parent: {0}")]
    WorkspaceRootHasNoParent(PathBuf),

    #[error("error: workspace does not exist\nhint: use --create")]
    WorkspaceDoesNotExist,

    #[error("error: workspace '{0}' does not exist")]
    WorkspaceNotFound(String),

    #[error("error: invalid repo pointer in {0}")]
    InvalidRepoPointer(PathBuf),

    #[error("error: invalid workspace template '{0}'")]
    InvalidWorkspaceTemplate(String),

    #[error("error: invalid repo config in {path}\n{message}")]
    InvalidRepoConfig { path: PathBuf, message: String },

    #[error("error: invalid workspace metadata in {path}\n{message}")]
    InvalidWorkspaceMetadata { path: PathBuf, message: String },

    #[error("error: unsupported shell '{0}'")]
    UnsupportedShell(String),

    #[error("error: unable to detect shell from $SHELL")]
    ShellDetection,

    #[error("error: $HOME is not set")]
    HomeDirectory,

    #[error("error: invalid shell rc file at {path}\n{message}")]
    InvalidShellRcFile {
        path: PathBuf,
        message: &'static str,
    },

    #[error("error: shell integration requires a UTF-8 workspace path")]
    ShellDirectivePathNotUtf8,

    #[error("error: jj command failed: {command}\n{stderr}")]
    JjCommandFailed { command: String, stderr: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
