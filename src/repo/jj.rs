use std::ffi::OsString;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::error::{Error, Result};
use crate::types::{
    WorkspaceDiffSnapshot, WorkspaceDiffStatus, WorkspaceFreshnessSnapshot, WorkspaceName,
};

const WORKSPACE_CURRENT_TIMEOUT: Duration = Duration::from_secs(2);
const WORKSPACE_DIFF_TIMEOUT: Duration = Duration::from_secs(2);
static TEMP_OUTPUT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JjWorkspaceListEntry {
    pub(crate) name: WorkspaceName,
    pub(crate) is_current: bool,
    pub(crate) commit_id: String,
    pub(crate) change_id: String,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JjRevisionSummary {
    pub(crate) commit_id: String,
    pub(crate) change_id: String,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JjCommandOutput {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) struct JjClient<'a> {
    workspace_root: &'a Path,
}

pub(crate) fn config_list(path: &Path, name: &str) -> Option<String> {
    let output = Command::new("jj")
        .args([
            OsString::from("--color=never"),
            OsString::from("--no-pager"),
            OsString::from("config"),
            OsString::from("list"),
            OsString::from("--include-defaults"),
            OsString::from(name),
        ])
        .current_dir(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(crate) fn snapshot_working_copy_at(path: &Path) -> WorkspaceFreshnessSnapshot {
    let args = [
        OsString::from("--quiet"),
        OsString::from("--no-pager"),
        OsString::from("util"),
        OsString::from("snapshot"),
    ];

    match run_with_timeout(path, &args, WORKSPACE_CURRENT_TIMEOUT) {
        TimedCommandResult::Success(_) => WorkspaceFreshnessSnapshot::current(),
        TimedCommandResult::Failure(stderr) => WorkspaceFreshnessSnapshot::failed(
            meaningful_stderr(&stderr, "jj could not make the workspace current"),
        ),
        TimedCommandResult::TimedOut => WorkspaceFreshnessSnapshot::timed_out(),
        TimedCommandResult::Io(error) => WorkspaceFreshnessSnapshot::failed(format!(
            "failed to run jj while making the workspace current: {error}"
        )),
    }
}

pub(crate) fn diff_stat_at(path: &Path) -> WorkspaceDiffSnapshot {
    let args = [
        OsString::from("--ignore-working-copy"),
        OsString::from("--color=never"),
        OsString::from("--no-pager"),
        OsString::from("diff"),
        OsString::from("--stat"),
        OsString::from("-r"),
        OsString::from("@"),
    ];

    match run_with_timeout(path, &args, WORKSPACE_DIFF_TIMEOUT) {
        TimedCommandResult::Success(stdout) => parse_diff_stat(&stdout),
        TimedCommandResult::Failure(_)
        | TimedCommandResult::TimedOut
        | TimedCommandResult::Io(_) => WorkspaceDiffSnapshot::unknown(),
    }
}

const MINIMUM_JJ_VERSION: JjVersion = JjVersion {
    major: 0,
    minor: 39,
    patch: 0,
};
const MINIMUM_JJ_VERSION_STR: &str = "0.39.0";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct JjVersion {
    pub(crate) major: u64,
    pub(crate) minor: u64,
    pub(crate) patch: u64,
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
        let output = self.run_ignoring_working_copy(&[
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
        let output = self.run_ignoring_working_copy(&[
            OsString::from("workspace"),
            OsString::from("list"),
            OsString::from("-T"),
            OsString::from(
                "name ++ \"\\0\" ++ if(target.current_working_copy(), \"1\", \"0\") ++ \"\\0\" ++ target.commit_id().short(12) ++ \"\\0\" ++ target.change_id().short(12) ++ \"\\0\" ++ target.description().first_line() ++ \"\\n\"",
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

    pub(crate) fn revisions(&self, revset: &str) -> Result<Vec<JjRevisionSummary>> {
        let output = self.run_ignoring_working_copy(&[
            OsString::from("log"),
            OsString::from("-r"),
            OsString::from(revset),
            OsString::from("--no-graph"),
            OsString::from("-T"),
            OsString::from(
                "commit_id.short(12) ++ \"\\0\" ++ change_id.short(12) ++ \"\\0\" ++ description.first_line() ++ \"\\n\"",
            ),
        ])?;

        output
            .lines()
            .filter(|line| !line.is_empty())
            .map(parse_revision_line)
            .collect()
    }

    pub(crate) fn duplicate(&self, revset: &str) -> Result<JjCommandOutput> {
        self.run_capture(&[OsString::from("duplicate"), OsString::from(revset)])
    }

    pub(crate) fn rebase_source_onto(&self, source: &str, target: &str) -> Result<JjCommandOutput> {
        let output = self.run_capture(&[
            OsString::from("rebase"),
            OsString::from("-s"),
            OsString::from(source),
            OsString::from("-d"),
            OsString::from(target),
        ]);

        output.map_err(|error| match error {
            Error::JjCommandFailed { stderr, .. } => Error::MergeRebaseFailed { stderr },
            other => other,
        })
    }

    pub(crate) fn new_working_copy(&self, revision: &str) -> Result<JjCommandOutput> {
        self.run_capture(&[OsString::from("new"), OsString::from(revision)])
    }

    pub(crate) fn has_conflicts(&self, revision: &str) -> Result<bool> {
        let revset = format!("{revision} & conflicts()");
        Ok(!self.revisions(&revset)?.is_empty())
    }

    pub(crate) fn workspace_root(&self, workspace: &WorkspaceName) -> Result<PathBuf> {
        let args = [
            OsString::from("workspace"),
            OsString::from("root"),
            OsString::from("--name"),
            OsString::from(workspace.as_str()),
        ];
        let output = self.run_ignoring_working_copy(&args)?;
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
        let output = self.run_capture(args)?;
        Ok(output.stdout)
    }

    fn run_capture(&self, args: &[OsString]) -> Result<JjCommandOutput> {
        let output = Command::new("jj")
            .args(args)
            .current_dir(self.workspace_root)
            .output()?;

        if output.status.success() {
            Ok(JjCommandOutput {
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        } else {
            Err(Error::JjCommandFailed {
                command: format_command(args),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            })
        }
    }

    fn run_ignoring_working_copy(&self, args: &[OsString]) -> Result<String> {
        let mut full_args = Vec::with_capacity(args.len() + 1);
        full_args.push(OsString::from("--ignore-working-copy"));
        full_args.extend(args.iter().cloned());
        self.run(&full_args)
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

enum TimedCommandResult {
    Success(String),
    Failure(String),
    TimedOut,
    Io(std::io::Error),
}

fn run_with_timeout(path: &Path, args: &[OsString], timeout: Duration) -> TimedCommandResult {
    let (stdout_file, stdout_path) = match temp_output_file("stdout") {
        Ok(output) => output,
        Err(error) => return TimedCommandResult::Io(error),
    };
    let (stderr_file, stderr_path) = match temp_output_file("stderr") {
        Ok(output) => output,
        Err(error) => {
            let _ = fs::remove_file(stdout_path);
            return TimedCommandResult::Io(error);
        }
    };

    let mut child = match Command::new("jj")
        .args(args)
        .current_dir(path)
        .stdout(stdout_file)
        .stderr(stderr_file)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            let _ = fs::remove_file(stdout_path);
            let _ = fs::remove_file(stderr_path);
            return TimedCommandResult::Io(error);
        }
    };

    let deadline = Instant::now() + timeout;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = read_temp_output(&stdout_path);
                let stderr = read_temp_output(&stderr_path);
                let _ = fs::remove_file(stdout_path);
                let _ = fs::remove_file(stderr_path);

                return match (status.success(), stdout, stderr) {
                    (true, Ok(stdout), _) => TimedCommandResult::Success(stdout),
                    (false, _, Ok(stderr)) => TimedCommandResult::Failure(stderr.trim().to_owned()),
                    (_, Err(error), _) | (_, _, Err(error)) => TimedCommandResult::Io(error),
                };
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = fs::remove_file(stdout_path);
                    let _ = fs::remove_file(stderr_path);
                    return TimedCommandResult::TimedOut;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => {
                let _ = fs::remove_file(stdout_path);
                let _ = fs::remove_file(stderr_path);
                return TimedCommandResult::Io(error);
            }
        }
    }
}

fn temp_output_file(kind: &str) -> std::io::Result<(File, PathBuf)> {
    let id = TEMP_OUTPUT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("jj-navi-{}-{id}-{kind}.tmp", std::process::id()));
    let file = File::options().write(true).create_new(true).open(&path)?;
    Ok((file, path))
}

fn read_temp_output(path: &Path) -> std::io::Result<String> {
    let mut output = String::new();
    File::open(path)?.read_to_string(&mut output)?;
    Ok(output)
}

fn meaningful_stderr(stderr: &str, fallback: &str) -> String {
    if stderr.trim().is_empty() {
        fallback.to_owned()
    } else {
        stderr.trim().to_owned()
    }
}

fn parse_diff_stat(output: &str) -> WorkspaceDiffSnapshot {
    let Some(line) = output.lines().rev().find(|line| line.contains(" changed")) else {
        return WorkspaceDiffSnapshot::unknown();
    };

    WorkspaceDiffSnapshot {
        status: WorkspaceDiffStatus::Available,
        files_changed: number_before(line, " file"),
        insertions: number_before(line, " insertion"),
        deletions: number_before(line, " deletion"),
    }
}

fn number_before(line: &str, marker: &str) -> Option<u32> {
    let before_marker = line.split(marker).next()?;
    before_marker
        .split(|ch: char| !ch.is_ascii_digit())
        .rfind(|part| !part.is_empty())?
        .parse()
        .ok()
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
    let mut parts = line.splitn(5, '\0');
    let (Some(name), Some(is_current), Some(commit_id), Some(change_id), Some(message)) = (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) else {
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
        change_id: change_id.to_owned(),
        message: message.to_owned(),
    })
}

fn parse_revision_line(line: &str) -> Result<JjRevisionSummary> {
    let mut parts = line.splitn(3, '\0');
    let (Some(commit_id), Some(change_id), Some(message)) =
        (parts.next(), parts.next(), parts.next())
    else {
        return Err(Error::InvalidJjWorkspaceListEntry(line.to_owned()));
    };

    Ok(JjRevisionSummary {
        commit_id: commit_id.to_owned(),
        change_id: change_id.to_owned(),
        message: message.to_owned(),
    })
}
