use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};
use crate::types::WorkspaceName;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JjWorkspaceListEntry {
    pub(crate) name: WorkspaceName,
    pub(crate) is_current: bool,
    pub(crate) commit_id: String,
    pub(crate) message: String,
}

pub(crate) struct JjClient<'a> {
    workspace_root: &'a Path,
}

const MINIMUM_JJ_VERSION: JjVersion = JjVersion {
    major: 0,
    minor: 39,
    patch: 0,
};
const MINIMUM_JJ_VERSION_STR: &str = "0.39.0";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct JjVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl<'a> JjClient<'a> {
    pub(crate) fn new(workspace_root: &'a Path) -> Self {
        Self { workspace_root }
    }

    pub(crate) fn ensure_supported_version(&self) -> Result<()> {
        let args = [OsString::from("--version")];
        let output = self.run(&args)?;
        let found = output.trim().to_owned();
        let Some(version) = parse_jj_version(&output) else {
            return Err(Error::UnsupportedJjVersion {
                found,
                minimum: MINIMUM_JJ_VERSION_STR,
            });
        };

        if version < MINIMUM_JJ_VERSION {
            return Err(Error::UnsupportedJjVersion {
                found,
                minimum: MINIMUM_JJ_VERSION_STR,
            });
        }

        Ok(())
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
            .ok_or(Error::OrphanedWorkspace)?;

        WorkspaceName::new(name.to_owned())
    }

    pub(crate) fn list_workspaces(&self) -> Result<Vec<JjWorkspaceListEntry>> {
        let output = self.run(&[
            OsString::from("workspace"),
            OsString::from("list"),
            OsString::from("-T"),
            OsString::from(
                "name ++ \"\\0\" ++ if(target.current_working_copy(), \"1\", \"0\") ++ \"\\0\" ++ target.commit_id().short(12) ++ \"\\0\" ++ target.description().first_line() ++ \"\\n\"",
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

    pub(crate) fn workspace_root(&self, workspace: &WorkspaceName) -> Result<PathBuf> {
        let args = [
            OsString::from("workspace"),
            OsString::from("root"),
            OsString::from("--name"),
            OsString::from(workspace.as_str()),
        ];
        let output = self.run(&args)?;
        let root = output.trim();

        if root.is_empty() {
            return Err(Error::JjCommandFailed {
                command: format_command(&args),
                stderr: String::from("jj returned an empty workspace root"),
            });
        }

        Ok(PathBuf::from(root))
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

fn parse_jj_version(output: &str) -> Option<JjVersion> {
    let token = output
        .split_whitespace()
        .find(|part| part.chars().next().is_some_and(|ch| ch.is_ascii_digit()))?;
    let mut parts = token.split('.');

    Some(JjVersion {
        major: parse_version_component(parts.next()?)?,
        minor: parse_version_component(parts.next()?)?,
        patch: parse_version_component(parts.next()?)?,
    })
}

fn parse_version_component(component: &str) -> Option<u64> {
    let digits = component
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();

    if digits.is_empty() {
        return None;
    }

    digits.parse().ok()
}

fn parse_workspace_line(line: &str) -> Result<JjWorkspaceListEntry> {
    let mut parts = line.splitn(4, '\0');
    let (Some(name), Some(is_current), Some(commit_id), Some(message)) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(Error::InvalidJjWorkspaceListEntry(line.to_owned()));
    };

    let is_current = match is_current {
        "0" => false,
        "1" => true,
        _ => return Err(Error::InvalidJjWorkspaceListEntry(line.to_owned())),
    };

    Ok(JjWorkspaceListEntry {
        name: WorkspaceName::new(name.to_owned())?,
        is_current,
        commit_id: commit_id.to_owned(),
        message: message.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use crate::types::WorkspaceName;

    use super::{JjVersion, parse_jj_version, parse_workspace_line, workspace_add_args};

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

    #[test]
    fn parses_plain_jj_version_output() {
        assert_eq!(
            parse_jj_version("jj 0.39.0\n"),
            Some(JjVersion {
                major: 0,
                minor: 39,
                patch: 0,
            })
        );
    }

    #[test]
    fn parses_jj_version_with_suffix() {
        assert_eq!(
            parse_jj_version("jj 0.39.0-12-gabcdef\n"),
            Some(JjVersion {
                major: 0,
                minor: 39,
                patch: 0,
            })
        );
    }

    #[test]
    fn rejects_unparseable_jj_version_output() {
        assert_eq!(parse_jj_version("jj dev build\n"), None);
    }

    #[test]
    fn rejects_workspace_list_line_with_missing_fields() {
        let error = parse_workspace_line("default\0").expect_err("reject malformed line");

        assert_eq!(
            error.to_string(),
            "error: invalid jj workspace list entry\ndefault\0"
        );
    }

    #[test]
    fn rejects_workspace_list_line_with_invalid_current_marker() {
        let error = parse_workspace_line("default\0x\0abc123\0message")
            .expect_err("reject malformed current marker");

        assert_eq!(
            error.to_string(),
            "error: invalid jj workspace list entry\ndefault\0x\0abc123\0message"
        );
    }
}
