use std::fmt;
use std::path::PathBuf;

use crate::error::{Error, Result};

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
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

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
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn render(&self, repo: &str, workspace: &WorkspaceName) -> PathBuf {
        PathBuf::from(
            self.0
                .replace("{repo}", repo)
                .replace("{workspace}", workspace.as_str()),
        )
    }
}

impl Default for WorkspaceTemplate {
    fn default() -> Self {
        Self(String::from("../{repo}.{workspace}"))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RepoConfig {
    pub workspace_template: WorkspaceTemplate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceListEntry {
    pub is_current: bool,
    pub name: WorkspaceName,
    pub path: PathBuf,
    pub commit_id: String,
    pub message: String,
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
