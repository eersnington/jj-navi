use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};
use crate::types::WorkspaceName;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JjWorkspaceListEntry {
    pub(crate) name: WorkspaceName,
    pub(crate) is_current: bool,
}

pub(crate) struct JjClient<'a> {
    workspace_root: &'a Path,
}

impl<'a> JjClient<'a> {
    pub(crate) fn new(workspace_root: &'a Path) -> Self {
        Self { workspace_root }
    }

    pub(crate) fn current_workspace_name(&self) -> Result<WorkspaceName> {
        let output = self.run(&[
            OsString::from("workspace"),
            OsString::from("list"),
            OsString::from("-T"),
            OsString::from("if(target.current_working_copy(), name ++ \"\\n\", \"\")"),
        ])?;

        let name = output
            .lines()
            .find(|line| !line.is_empty())
            .ok_or(Error::RepoName)?;

        WorkspaceName::new(name.to_owned())
    }

    pub(crate) fn list_workspaces(&self) -> Result<Vec<JjWorkspaceListEntry>> {
        let output = self.run(&[
            OsString::from("workspace"),
            OsString::from("list"),
            OsString::from("-T"),
            OsString::from(
                "name ++ \"\\t\" ++ if(target.current_working_copy(), \"1\", \"0\") ++ \"\\n\"",
            ),
        ])?;

        output
            .lines()
            .filter(|line| !line.is_empty())
            .map(parse_workspace_line)
            .collect()
    }

    pub(crate) fn workspace_forget(&self, workspace: &WorkspaceName) -> Result<()> {
        self.run(&[
            OsString::from("workspace"),
            OsString::from("forget"),
            OsString::from(workspace.as_str()),
        ])
        .map(|_| ())
    }

    pub(crate) fn workspace_add(
        &self,
        workspace: &WorkspaceName,
        target_root: &Path,
        revision: Option<&str>,
    ) -> Result<()> {
        let args = workspace_add_args(workspace, target_root, revision);

        self.run(&args).map(|_| ())
    }

    fn run(&self, args: &[OsString]) -> Result<String> {
        let output = Command::new("jj")
            .args(args)
            .current_dir(self.workspace_root)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(Error::JjCommandFailed {
                command: format_command(args),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            })
        }
    }
}

fn workspace_add_args(
    workspace: &WorkspaceName,
    target_root: &Path,
    revision: Option<&str>,
) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("workspace"),
        OsString::from("add"),
        OsString::from("--name"),
        OsString::from(workspace.as_str()),
    ];

    if let Some(revision) = revision {
        args.push(OsString::from("-r"));
        args.push(OsString::from(revision));
    }

    args.push(target_root.as_os_str().to_owned());
    args
}

fn format_command(args: &[OsString]) -> String {
    let rendered = args
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    format!("jj {rendered}")
}

fn parse_workspace_line(line: &str) -> Result<JjWorkspaceListEntry> {
    let mut parts = line.splitn(2, '\t');
    let name = parts.next().unwrap_or_default();
    let is_current = parts.next().unwrap_or_default() == "1";

    Ok(JjWorkspaceListEntry {
        name: WorkspaceName::new(name.to_owned())?,
        is_current,
    })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use crate::types::WorkspaceName;

    use super::workspace_add_args;

    #[cfg(unix)]
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    #[test]
    #[cfg(unix)]
    fn workspace_add_preserves_non_utf8_paths() {
        let workspace = WorkspaceName::new("feature-auth").expect("valid workspace name");
        let target_root = PathBuf::from(OsString::from_vec(vec![b'.', b'/', 0xFF]));

        let args = workspace_add_args(&workspace, &target_root, None);

        assert_eq!(
            args.last().expect("target path arg").as_os_str().as_bytes(),
            target_root.as_os_str().as_bytes()
        );
    }
}
