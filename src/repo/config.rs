use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::types::{RepoConfig, WorkspaceTemplate};

const NAVI_DIR: &str = "navi";
const CONFIG_FILE: &str = "config.toml";

#[derive(Deserialize, Serialize)]
struct RepoConfigFile {
    workspace_template: String,
}

pub(crate) fn load_repo_config(repo_storage_path: &Path) -> Result<RepoConfig> {
    let path = repo_config_path(repo_storage_path);
    if !path.is_file() {
        return Ok(RepoConfig::default());
    }

    let contents = fs::read_to_string(&path)?;
    let file =
        toml::from_str::<RepoConfigFile>(&contents).map_err(|error| Error::InvalidRepoConfig {
            path: path.clone(),
            message: error.to_string(),
        })?;

    let workspace_template = WorkspaceTemplate::new(file.workspace_template).map_err(|error| {
        Error::InvalidRepoConfig {
            path: path.clone(),
            message: error.to_string(),
        }
    })?;

    Ok(RepoConfig { workspace_template })
}

pub(crate) fn ensure_repo_config(repo_storage_path: &Path, config: &RepoConfig) -> Result<PathBuf> {
    let navi_dir = navi_dir_path(repo_storage_path);
    fs::create_dir_all(&navi_dir)?;

    let path = repo_config_path(repo_storage_path);
    if !path.exists() {
        let file = RepoConfigFile {
            workspace_template: config.workspace_template.as_str().to_owned(),
        };
        let contents = toml::to_string_pretty(&file).map_err(|error| Error::InvalidRepoConfig {
            path: path.clone(),
            message: error.to_string(),
        })?;
        fs::write(&path, contents)?;
    }

    Ok(path)
}

pub(crate) fn navi_dir_path(repo_storage_path: &Path) -> PathBuf {
    repo_storage_path.join(NAVI_DIR)
}

pub(crate) fn repo_config_path(repo_storage_path: &Path) -> PathBuf {
    navi_dir_path(repo_storage_path).join(CONFIG_FILE)
}
