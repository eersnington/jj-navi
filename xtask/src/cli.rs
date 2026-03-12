use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::ToolError;

pub(crate) const HELP_TEXT: &str = "navi-release

Usage:
  navi-release prepare <version> [--repo <owner/name>] [--pr-body-path <path>]
  navi-release pr validate [--event-path <path>]
  navi-release validate [version]
  navi-release notes <version>
  navi-release current-version

Commands:
  prepare          Build release files from merged PR metadata.
  pr validate      Validate PR release labels from a GitHub event payload.
  validate         Verify synced release files.
  notes            Print release notes for a version.
  current-version  Print the synced repo version.
";

#[derive(Debug, Clone)]
pub(crate) struct PrepareCommand {
    pub(crate) version: String,
    pub(crate) repo: Option<String>,
    pub(crate) pr_body_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub(crate) struct PrValidateCommand {
    pub(crate) event_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub(crate) enum PrCommand {
    Validate(PrValidateCommand),
}

#[derive(Debug, Clone)]
pub(crate) enum Command {
    Prepare(PrepareCommand),
    Pr(PrCommand),
    Validate { version: Option<String> },
    Notes { version: String },
    CurrentVersion,
    Help,
    Version,
}

pub(crate) fn parse_command(args: Vec<OsString>) -> Result<Command, ToolError> {
    let args = args
        .into_iter()
        .map(|value| {
            value
                .into_string()
                .map_err(|_| ToolError::message("non-utf8 arguments are not supported"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if args.is_empty() {
        return Ok(Command::Help);
    }

    match args[0].as_str() {
        "-h" | "--help" => Ok(Command::Help),
        "-V" | "--version" => Ok(Command::Version),
        "prepare" => Ok(Command::Prepare(parse_prepare_args(&args[1..])?)),
        "pr" => Ok(Command::Pr(parse_pr_args(&args[1..])?)),
        "validate" => Ok(Command::Validate {
            version: args.get(1).cloned(),
        }),
        "notes" => {
            let version = args
                .get(1)
                .ok_or_else(|| ToolError::message("notes requires <version>"))?;
            Ok(Command::Notes {
                version: version.clone(),
            })
        }
        "current-version" => Ok(Command::CurrentVersion),
        other => Err(ToolError::message(format!("unknown command: {other}"))),
    }
}

fn parse_prepare_args(args: &[String]) -> Result<PrepareCommand, ToolError> {
    let version = args
        .first()
        .ok_or_else(|| ToolError::message("prepare requires <version>"))?
        .clone();

    let mut repo = None;
    let mut pr_body_path = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--repo" => {
                repo = Some(value_after_flag(args, index, "--repo")?);
                index += 2;
            }
            "--pr-body-path" => {
                pr_body_path = Some(PathBuf::from(value_after_flag(
                    args,
                    index,
                    "--pr-body-path",
                )?));
                index += 2;
            }
            other => {
                return Err(ToolError::message(format!(
                    "unexpected arg for prepare: {other}"
                )));
            }
        }
    }

    Ok(PrepareCommand {
        version,
        repo,
        pr_body_path,
    })
}

fn parse_pr_args(args: &[String]) -> Result<PrCommand, ToolError> {
    match args.first().map(String::as_str) {
        Some("validate") => Ok(PrCommand::Validate(parse_pr_validate_args(&args[1..])?)),
        Some(other) => Err(ToolError::message(format!("unknown pr command: {other}"))),
        None => Err(ToolError::message("pr requires a subcommand")),
    }
}

fn parse_pr_validate_args(args: &[String]) -> Result<PrValidateCommand, ToolError> {
    let mut event_path = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--event-path" => {
                event_path = Some(PathBuf::from(value_after_flag(
                    args,
                    index,
                    "--event-path",
                )?));
                index += 2;
            }
            other => {
                return Err(ToolError::message(format!(
                    "unexpected arg for pr validate: {other}"
                )));
            }
        }
    }

    Ok(PrValidateCommand { event_path })
}

fn value_after_flag(args: &[String], index: usize, flag: &str) -> Result<String, ToolError> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| ToolError::message(format!("{flag} requires a value")))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::{Command, PrCommand, parse_command};

    #[test]
    fn parses_prepare_args() {
        let command = parse_command(vec![
            OsString::from("prepare"),
            OsString::from("0.2.0"),
            OsString::from("--repo"),
            OsString::from("owner/repo"),
            OsString::from("--pr-body-path"),
            OsString::from("/tmp/release.md"),
        ])
        .expect("command");

        match command {
            Command::Prepare(prepare) => {
                assert_eq!(prepare.version, "0.2.0");
                assert_eq!(prepare.repo.as_deref(), Some("owner/repo"));
                assert_eq!(prepare.pr_body_path, Some(PathBuf::from("/tmp/release.md")));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_pr_validate_args() {
        let command = parse_command(vec![
            OsString::from("pr"),
            OsString::from("validate"),
            OsString::from("--event-path"),
            OsString::from("/tmp/event.json"),
        ])
        .expect("command");

        match command {
            Command::Pr(PrCommand::Validate(validate)) => {
                assert_eq!(validate.event_path, Some(PathBuf::from("/tmp/event.json")));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }
}
