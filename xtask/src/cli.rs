use std::ffi::OsString;

use crate::error::ToolError;

pub(crate) const HELP_TEXT: &str = "navi-release

Usage:
  navi-release prepare <version> [release-input.json]
  navi-release validate [version]
  navi-release notes <version>

Commands:
  prepare   Generate changelog and synced release files.
  validate  Verify synced release files.
  notes     Print release notes for a version.
";

#[derive(Debug)]
pub(crate) enum Command {
    Prepare {
        version: String,
        input_path: Option<String>,
    },
    Validate {
        version: Option<String>,
    },
    Notes {
        version: String,
    },
    Help,
    Version,
}

pub(crate) fn parse_command(args: Vec<OsString>) -> Result<Command, ToolError> {
    let args = args
        .into_iter()
        .map(|value| {
            value
                .into_string()
                .map_err(|_| ToolError::Message("non-utf8 arguments are not supported".to_owned()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if args.is_empty() {
        return Ok(Command::Help);
    }

    match args[0].as_str() {
        "-h" | "--help" => Ok(Command::Help),
        "-V" | "--version" => Ok(Command::Version),
        "prepare" => {
            let version = args
                .get(1)
                .ok_or_else(|| ToolError::Message("prepare requires <version>".to_owned()))?;
            Ok(Command::Prepare {
                version: version.clone(),
                input_path: args.get(2).cloned(),
            })
        }
        "validate" => Ok(Command::Validate {
            version: args.get(1).cloned(),
        }),
        "notes" => {
            let version = args
                .get(1)
                .ok_or_else(|| ToolError::Message("notes requires <version>".to_owned()))?;
            Ok(Command::Notes {
                version: version.clone(),
            })
        }
        _ => Err(ToolError::Message(format!("unknown command: {}", args[0]))),
    }
}
