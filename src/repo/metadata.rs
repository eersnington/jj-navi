use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::error::{Error, Result};
use crate::types::{WorkspaceName, WorkspaceTemplate};

use super::config::navi_dir_path;

const WORKSPACES_FILE: &str = "workspaces.toml";

#[derive(Default)]
pub(crate) struct WorkspaceMetadataStore {
    path: PathBuf,
    records: Vec<WorkspaceMetadataRecord>,
}

#[derive(Clone)]
struct WorkspaceMetadataRecord {
    name: WorkspaceName,
    created_by_navi: bool,
    created_at: String,
    template: WorkspaceTemplate,
    revision: String,
}

#[derive(Default, Deserialize, Serialize)]
struct WorkspaceMetadataFile {
    #[serde(default, rename = "workspace")]
    workspaces: Vec<WorkspaceMetadataRecordFile>,
}

#[derive(Deserialize, Serialize)]
struct WorkspaceMetadataRecordFile {
    name: String,
    created_by_navi: bool,
    created_at: String,
    template: String,
    revision: String,
}

impl WorkspaceMetadataStore {
    pub(crate) fn load(repo_storage_path: &Path) -> Result<Self> {
        let path = workspace_metadata_path(repo_storage_path);
        if !path.is_file() {
            return Ok(Self {
                path,
                records: Vec::new(),
            });
        }

        let contents = fs::read_to_string(&path)?;
        let file = toml::from_str::<WorkspaceMetadataFile>(&contents).map_err(|error| {
            Error::InvalidWorkspaceMetadata {
                path: path.clone(),
                message: error.to_string(),
            }
        })?;

        let records = file
            .workspaces
            .into_iter()
            .map(|record| parse_record_file(record, &path))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { path, records })
    }

    pub(crate) fn record_workspace(
        &mut self,
        workspace: &WorkspaceName,
        template: &WorkspaceTemplate,
        revision: Option<&str>,
    ) -> Result<()> {
        let created_at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|error| Error::InvalidWorkspaceMetadata {
                path: self.path.clone(),
                message: error.to_string(),
            })?;

        let new_record = WorkspaceMetadataRecord {
            name: workspace.clone(),
            created_by_navi: true,
            created_at,
            template: template.clone(),
            revision: revision.unwrap_or_default().to_owned(),
        };

        if let Some(existing) = self
            .records
            .iter_mut()
            .find(|record| record.name == *workspace)
        {
            *existing = new_record;
        } else {
            self.records.push(new_record);
            self.records
                .sort_by(|left, right| left.name.cmp(&right.name));
        }

        Ok(())
    }

    pub(crate) fn remove_workspace(&mut self, workspace: &WorkspaceName) {
        self.records.retain(|record| record.name != *workspace);
    }

    pub(crate) fn template_for(&self, workspace: &WorkspaceName) -> Option<&WorkspaceTemplate> {
        self.records
            .iter()
            .find(|record| record.name == *workspace)
            .map(|record| &record.template)
    }

    pub(crate) fn save(&self) -> Result<()> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| Error::InvalidWorkspaceMetadata {
                path: self.path.clone(),
                message: String::from("metadata path has no parent"),
            })?;
        fs::create_dir_all(parent)?;

        let file = WorkspaceMetadataFile {
            workspaces: self
                .records
                .iter()
                .map(|record| WorkspaceMetadataRecordFile {
                    name: record.name.as_str().to_owned(),
                    created_by_navi: record.created_by_navi,
                    created_at: record.created_at.clone(),
                    template: record.template.as_str().to_owned(),
                    revision: record.revision.clone(),
                })
                .collect(),
        };

        let contents =
            toml::to_string_pretty(&file).map_err(|error| Error::InvalidWorkspaceMetadata {
                path: self.path.clone(),
                message: error.to_string(),
            })?;
        fs::write(&self.path, contents)?;
        Ok(())
    }
}

fn parse_record_file(
    record: WorkspaceMetadataRecordFile,
    path: &Path,
) -> Result<WorkspaceMetadataRecord> {
    Ok(WorkspaceMetadataRecord {
        name: WorkspaceName::new(record.name).map_err(|error| Error::InvalidWorkspaceMetadata {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?,
        created_by_navi: record.created_by_navi,
        created_at: record.created_at,
        template: WorkspaceTemplate::new(record.template).map_err(|error| {
            Error::InvalidWorkspaceMetadata {
                path: path.to_path_buf(),
                message: error.to_string(),
            }
        })?,
        revision: record.revision,
    })
}

pub(crate) fn workspace_metadata_path(repo_storage_path: &Path) -> PathBuf {
    navi_dir_path(repo_storage_path).join(WORKSPACES_FILE)
}
