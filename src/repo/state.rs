use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::types::WorkspaceName;

use super::config::repo_state_path;

#[derive(Default)]
pub(crate) struct RepoStateStore {
    path: PathBuf,
    previous_workspace: Option<WorkspaceName>,
}

#[derive(Default, Deserialize, Serialize)]
struct RepoStateFile {
    #[serde(default)]
    switch: SwitchStateFile,
}

#[derive(Default, Deserialize, Serialize)]
struct SwitchStateFile {
    #[serde(default)]
    previous_workspace: Option<String>,
}

impl RepoStateStore {
    pub(crate) fn load(repo_storage_path: &Path) -> Result<Self> {
        let path = repo_state_path(repo_storage_path);
        if !path.is_file() {
            return Ok(Self {
                path,
                previous_workspace: None,
            });
        }

        let contents = fs::read_to_string(&path)?;
        let file = toml::from_str::<RepoStateFile>(&contents).map_err(|error| {
            Error::InvalidRepoState {
                path: path.clone(),
                message: error.to_string(),
            }
        })?;

        Ok(Self {
            path: path.clone(),
            previous_workspace: parse_workspace_name(file.switch.previous_workspace, &path)?,
        })
    }

    pub(crate) fn previous_workspace(&self) -> Option<&WorkspaceName> {
        self.previous_workspace.as_ref()
    }

    pub(crate) fn save_previous_workspace(
        repo_storage_path: &Path,
        workspace: &WorkspaceName,
    ) -> Result<()> {
        let store = Self {
            path: repo_state_path(repo_storage_path),
            previous_workspace: Some(workspace.clone()),
        };
        store.save()
    }

    pub(crate) fn save(&self) -> Result<()> {
        let parent = self.path.parent().ok_or_else(|| Error::InvalidRepoState {
            path: self.path.clone(),
            message: String::from("state path has no parent"),
        })?;
        fs::create_dir_all(parent)?;

        let file = RepoStateFile {
            switch: SwitchStateFile {
                previous_workspace: self
                    .previous_workspace
                    .as_ref()
                    .map(|workspace| workspace.as_str().to_owned()),
            },
        };
        let contents = toml::to_string_pretty(&file).map_err(|error| Error::InvalidRepoState {
            path: self.path.clone(),
            message: error.to_string(),
        })?;
        fs::write(&self.path, contents)?;
        Ok(())
    }
}

fn parse_workspace_name(value: Option<String>, path: &Path) -> Result<Option<WorkspaceName>> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| {
            WorkspaceName::new(value).map_err(|error| Error::InvalidRepoState {
                path: path.to_path_buf(),
                message: error.to_string(),
            })
        })
        .transpose()
}
