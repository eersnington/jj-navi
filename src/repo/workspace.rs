use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pathdiff::diff_paths;

use crate::error::{Error, Result};
use crate::types::{WorkspaceEntry, WorkspaceName};

pub struct NaviWorkspace {
    cwd: PathBuf,
    workspace_root: PathBuf,
    repo_storage_path: PathBuf,
    current_workspace: WorkspaceName,
    repo_name: String,
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
        let repo_storage_path = resolve_repo_storage_path(&workspace_root)?;
        let current_workspace = detect_current_workspace_name(&workspace_root)?;
        let repo_name = derive_repo_name(&workspace_root, &current_workspace)?;

        Ok(Self {
            cwd,
            workspace_root,
            repo_storage_path,
            current_workspace,
            repo_name,
        })
    }

    #[must_use]
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    #[must_use]
    pub fn repo_storage_path(&self) -> &Path {
        &self.repo_storage_path
    }

    #[must_use]
    pub fn current_workspace(&self) -> &WorkspaceName {
        &self.current_workspace
    }

    /// Compute the absolute workspace root for `workspace`.
    ///
    /// # Errors
    ///
    /// Returns an error if the current workspace root has no parent.
    pub fn planned_workspace_root(&self, workspace: &WorkspaceName) -> Result<PathBuf> {
        if workspace == &self.current_workspace {
            return Ok(self.workspace_root.clone());
        }

        let parent = self
            .workspace_root
            .parent()
            .ok_or_else(|| Error::WorkspaceRootHasNoParent(self.workspace_root.clone()))?;

        Ok(parent.join(format!("{}.{}", self.repo_name, workspace.as_str())))
    }

    #[must_use]
    pub fn planned_workspace_display(&self, workspace: &WorkspaceName) -> PathBuf {
        if workspace == &self.current_workspace {
            PathBuf::from(".")
        } else {
            PathBuf::from(format!("../{}.{}", self.repo_name, workspace.as_str()))
        }
    }

    #[must_use]
    pub fn display_path_for_switch(&self, target_root: &Path) -> PathBuf {
        diff_paths(target_root, &self.cwd).unwrap_or_else(|| target_root.to_path_buf())
    }

    /// Check if the target workspace directory already exists.
    ///
    /// # Errors
    ///
    /// Returns an error if path planning fails.
    pub fn workspace_exists(&self, workspace: &WorkspaceName) -> Result<bool> {
        Ok(self.planned_workspace_root(workspace)?.is_dir())
    }

    /// Create a workspace via `jj workspace add`.
    ///
    /// # Errors
    ///
    /// Returns an error if path planning fails or if `jj` returns an error.
    pub fn create_workspace(
        &self,
        workspace: &WorkspaceName,
        revision: Option<&str>,
    ) -> Result<PathBuf> {
        let target_root = self.planned_workspace_root(workspace)?;
        let mut args = vec![
            String::from("workspace"),
            String::from("add"),
            String::from("--name"),
            workspace.as_str().to_owned(),
        ];

        if let Some(revision) = revision {
            args.push(String::from("-r"));
            args.push(revision.to_owned());
        }

        args.push(target_root.display().to_string());

        run_jj_command(&self.workspace_root, &args)?;

        Ok(target_root)
    }

    /// List repo workspaces with navi display paths.
    ///
    /// # Errors
    ///
    /// Returns an error if `jj workspace list` fails or if a workspace name is
    /// invalid for navi.
    pub fn list_workspaces(&self) -> Result<Vec<WorkspaceEntry>> {
        let output = run_jj_command(
            &self.workspace_root,
            &[
                String::from("workspace"),
                String::from("list"),
                String::from("-T"),
                String::from(
                    "name ++ \"\\t\" ++ if(target.current_working_copy(), \"1\", \"0\") ++ \"\\n\"",
                ),
            ],
        )?;

        let mut entries = output
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| self.parse_workspace_line(line))
            .collect::<Result<Vec<_>>>()?;

        entries.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(entries)
    }

    fn parse_workspace_line(&self, line: &str) -> Result<WorkspaceEntry> {
        let mut parts = line.splitn(2, '\t');
        let name = parts.next().unwrap_or_default();
        let is_current = parts.next().unwrap_or_default() == "1";
        let workspace_name = WorkspaceName::new(name.to_owned())?;

        let path = if is_current {
            PathBuf::from(".")
        } else {
            self.planned_workspace_display(&workspace_name)
        };

        Ok(WorkspaceEntry {
            name: workspace_name,
            path,
        })
    }
}

fn find_workspace_root(path: &Path) -> Result<PathBuf> {
    path.ancestors()
        .find(|ancestor| ancestor.join(".jj").is_dir())
        .map(Path::to_path_buf)
        .ok_or(Error::NotInWorkspace)
}

fn resolve_repo_storage_path(workspace_root: &Path) -> Result<PathBuf> {
    let repo_path = workspace_root.join(".jj").join("repo");

    if repo_path.is_dir() {
        return Ok(repo_path);
    }

    let pointer = fs::read_to_string(&repo_path)?;
    let pointer = pointer.trim();
    if pointer.is_empty() {
        return Err(Error::InvalidRepoPointer(repo_path));
    }

    let pointer_path = PathBuf::from(pointer);
    let resolved = if pointer_path.is_absolute() {
        pointer_path
    } else {
        workspace_root.join(".jj").join(pointer_path)
    };

    Ok(fs::canonicalize(&resolved).unwrap_or(resolved))
}

fn detect_current_workspace_name(workspace_root: &Path) -> Result<WorkspaceName> {
    let output = run_jj_command(
        workspace_root,
        &[
            String::from("workspace"),
            String::from("list"),
            String::from("-T"),
            String::from("if(target.current_working_copy(), name ++ \"\\n\", \"\")"),
        ],
    )?;

    let name = output
        .lines()
        .find(|line| !line.is_empty())
        .ok_or(Error::RepoName)?;

    WorkspaceName::new(name.to_owned())
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

fn run_jj_command(workspace_root: &Path, args: &[String]) -> Result<String> {
    let output = Command::new("jj")
        .args(args)
        .current_dir(workspace_root)
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(Error::JjCommandFailed {
            command: format!("jj {}", args.join(" ")),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::resolve_repo_storage_path;

    #[test]
    fn resolves_relative_repo_pointer() {
        let temp = TempDir::new().expect("temp dir");
        let workspace_root = temp.path().join("workspace");
        let shared = temp.path().join("shared").join("repo");

        fs::create_dir_all(workspace_root.join(".jj")).expect("workspace .jj");
        fs::create_dir_all(&shared).expect("shared repo");
        fs::write(
            workspace_root.join(".jj").join("repo"),
            "../../shared/repo\n",
        )
        .expect("write repo pointer");

        let resolved = resolve_repo_storage_path(&workspace_root).expect("resolve repo pointer");

        assert_eq!(
            resolved,
            fs::canonicalize(shared).expect("canonical shared repo")
        );
    }
}
