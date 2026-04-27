use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::Path;

use clap::Command;
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate, ValueCompleter};
use clap_complete::env::CompleteEnv;

use crate::cli;
use crate::repo::NaviWorkspace;

/// Handle shell-initiated completion requests via `COMPLETE=$SHELL navi`.
#[must_use]
pub(crate) fn maybe_handle_env_completion(bin_name: &'static str) -> bool {
    let Some(shell_name) = std::env::var_os("COMPLETE") else {
        return false;
    };

    if shell_name.is_empty() || shell_name == "0" {
        return false;
    }

    let mut args: Vec<OsString> = std::env::args_os().collect();

    args.remove(0);
    let escape_index = args
        .iter()
        .position(|arg| *arg == "--")
        .map_or(args.len(), |index| index + 1);
    args.drain(0..escape_index);

    let current_dir = std::env::current_dir().ok();

    if args.is_empty() {
        let all_args: Vec<OsString> = std::env::args_os().collect();
        let _ = CompleteEnv::with_factory(|| completion_command(bin_name))
            .try_complete(all_args, current_dir.as_deref());
        return true;
    }

    let mut command = completion_command(bin_name);
    command.build();

    let index = std::env::var("_CLAP_COMPLETE_INDEX")
        .ok()
        .and_then(|index| index.parse::<usize>().ok())
        .unwrap_or_else(|| args.len().saturating_sub(1));

    let current_word = args.get(index).map(|arg| arg.to_string_lossy());
    let include_long_flags = current_word.as_deref() == Some("-");

    if let Some(completions) = complete_workspace_context(bin_name, &args, index) {
        write_completions(&shell_name.to_string_lossy(), &completions);
        return true;
    }

    let Ok(completions) =
        clap_complete::engine::complete(&mut command, args.clone(), index, current_dir.as_deref())
    else {
        return true;
    };

    let completions = if include_long_flags {
        complete_short_and_long_flags(
            bin_name,
            args.clone(),
            index,
            current_dir.as_deref(),
            completions,
        )
    } else {
        completions
    };
    let completions = filter_contextual_completions(bin_name, completions, &args, index);

    write_completions(&shell_name.to_string_lossy(), &completions);
    true
}

/// Return a workspace-name completer for workspace arguments.
#[must_use]
pub(crate) fn workspace_value_completer() -> ArgValueCompleter {
    ArgValueCompleter::new(WorkspaceCompleter)
}

#[derive(Clone, Copy)]
struct WorkspaceCompleter;

impl ValueCompleter for WorkspaceCompleter {
    fn complete(&self, current: &OsStr) -> Vec<CompletionCandidate> {
        if current.to_str().is_some_and(|value| value.starts_with('-')) {
            return Vec::new();
        }

        let prefix = current.to_string_lossy();
        complete_workspaces()
            .into_iter()
            .filter(|candidate| {
                candidate
                    .get_value()
                    .to_string_lossy()
                    .starts_with(&*prefix)
            })
            .collect()
    }
}

fn complete_workspaces() -> Vec<CompletionCandidate> {
    let Ok(repo) = NaviWorkspace::open(Path::new(".")) else {
        return Vec::new();
    };
    let Ok(workspaces) = repo.workspace_names() else {
        return Vec::new();
    };

    workspaces
        .into_iter()
        .map(|workspace| CompletionCandidate::new(workspace.as_str().to_owned()))
        .collect()
}

fn complete_workspace_context(
    bin_name: &'static str,
    args: &[OsString],
    index: usize,
) -> Option<Vec<CompletionCandidate>> {
    let current = args.get(index)?.to_string_lossy();
    if current.starts_with('-') {
        return None;
    }

    let command_index = command_index(bin_name, args)?;
    let command = args.get(command_index)?.to_string_lossy();
    let relative_index = index.checked_sub(command_index)?;

    match command.as_ref() {
        "switch" | "cd" | "remove" | "rm" if relative_index == 1 => {
            complete_workspaces_for_prefix(&current)
        }
        "merge" => complete_merge_workspace_context(args, index, command_index, &current),
        _ => None,
    }
}

fn complete_merge_workspace_context(
    args: &[OsString],
    index: usize,
    command_index: usize,
    current: &str,
) -> Option<Vec<CompletionCandidate>> {
    let previous = index
        .checked_sub(1)
        .and_then(|previous_index| args.get(previous_index))
        .map(|arg| arg.to_string_lossy());
    if previous
        .as_deref()
        .is_some_and(|arg| matches!(arg, "--from" | "-f" | "--into" | "-i"))
    {
        return complete_workspaces_for_prefix(current);
    }

    let relative_index = index.checked_sub(command_index)?;
    if relative_index < 2 {
        return None;
    }

    for option in ["--from=", "--into=", "-f=", "-i="] {
        if let Some(prefix) = current.strip_prefix(option) {
            return complete_workspaces_for_prefix(prefix).map(|candidates| {
                candidates
                    .into_iter()
                    .map(|candidate| candidate.add_prefix(option))
                    .collect()
            });
        }
    }

    None
}

fn complete_workspaces_for_prefix(prefix: &str) -> Option<Vec<CompletionCandidate>> {
    let workspaces = complete_workspaces();
    if workspaces.is_empty() {
        return None;
    }

    Some(
        workspaces
            .into_iter()
            .filter(|candidate| candidate.get_value().to_string_lossy().starts_with(prefix))
            .collect(),
    )
}

fn filter_contextual_completions(
    bin_name: &'static str,
    completions: Vec<CompletionCandidate>,
    args: &[OsString],
    index: usize,
) -> Vec<CompletionCandidate> {
    let Some(command_index) = command_index(bin_name, args) else {
        return completions;
    };
    let Some(command) = args.get(command_index).map(|arg| arg.to_string_lossy()) else {
        return completions;
    };

    if matches!(command.as_ref(), "list" | "ls" | "doctor")
        && !has_json_flag(args, command_index, index)
    {
        return completions
            .into_iter()
            .filter(|candidate| {
                let value = candidate.get_value().to_string_lossy();
                value != "--compact" && value != "-c"
            })
            .collect();
    }

    completions
}

fn has_json_flag(args: &[OsString], command_index: usize, index: usize) -> bool {
    args.iter()
        .enumerate()
        .filter(|(arg_index, _)| *arg_index > command_index && *arg_index < index)
        .any(|(_, arg)| matches!(arg.to_string_lossy().as_ref(), "--json" | "-j"))
}

fn command_index(bin_name: &'static str, args: &[OsString]) -> Option<usize> {
    if args.is_empty() {
        return None;
    }

    Some(usize::from(
        args.first()
            .is_some_and(|arg| arg.to_string_lossy() == bin_name),
    ))
}

fn complete_short_and_long_flags(
    bin_name: &'static str,
    mut args: Vec<OsString>,
    index: usize,
    current_dir: Option<&Path>,
    mut completions: Vec<CompletionCandidate>,
) -> Vec<CompletionCandidate> {
    if let Some(word) = args.get_mut(index) {
        *word = OsString::from("--");
    }

    let mut command = completion_command(bin_name);
    command.build();
    let Ok(long_completions) =
        clap_complete::engine::complete(&mut command, args, index, current_dir)
    else {
        return completions;
    };

    for candidate in long_completions {
        let value = candidate.get_value();
        if !completions
            .iter()
            .any(|existing| existing.get_value() == value)
        {
            completions.push(candidate);
        }
    }

    completions
}

fn write_completions(shell_name: &str, completions: &[CompletionCandidate]) {
    let separator = std::env::var("_CLAP_IFS").unwrap_or_else(|_| String::from("\n"));
    let help_separator = match shell_name {
        "zsh" => Some(":"),
        "fish" | "nu" => Some("\t"),
        _ => None,
    };

    let mut stdout = std::io::stdout();
    for (index, candidate) in completions.iter().enumerate() {
        if index != 0 {
            let _ = write!(stdout, "{separator}");
        }
        let value = candidate.get_value().to_string_lossy();
        match (help_separator, candidate.get_help()) {
            (Some(separator), Some(help)) => {
                let _ = write!(stdout, "{value}{separator}{help}");
            }
            _ => {
                let _ = write!(stdout, "{value}");
            }
        }
    }
}

fn completion_command(bin_name: &'static str) -> Command {
    hide_non_positional_options_for_completion(cli::build_command().name(bin_name))
}

fn hide_non_positional_options_for_completion(command: Command) -> Command {
    fn process_command(command: Command, is_root: bool) -> Command {
        let command = command.disable_help_flag(true).arg(
            clap::Arg::new("help")
                .short('h')
                .long("help")
                .action(clap::ArgAction::Help)
                .help("Print help"),
        );

        let command = if is_root {
            command.disable_version_flag(true).arg(
                clap::Arg::new("version")
                    .short('V')
                    .long("version")
                    .action(clap::ArgAction::Version)
                    .help("Print version"),
            )
        } else {
            command
        };

        let command = command.mut_args(|arg| {
            if arg.is_positional() || arg.is_hide_set() {
                arg
            } else {
                arg.hide(true)
            }
        });

        command.mut_subcommands(|subcommand| process_command(subcommand, false))
    }

    process_command(command, true)
}
