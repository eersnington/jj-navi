use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

use crate::commands;
use crate::completion;
use crate::output::render_error_message;
use crate::types::ShellKind;

#[derive(Parser)]
#[command(about = "Workspace navigator for Jujutsu")]
#[command(arg_required_else_help = true)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(
        about = "Switch to an existing workspace, '^' for primary, '@' for current, '-' for previous, or create one with --create",
        visible_alias = "cd"
    )]
    Switch {
        #[arg(long, short = 'c', help = "Create the workspace if it does not exist")]
        create: bool,

        #[arg(
            long,
            short = 'r',
            help = "Revision to base a newly created workspace on"
        )]
        revision: Option<String>,

        #[arg(
            help = "Workspace name, '^' for primary, '@' for current, or '-' for previous",
            add = completion::workspace_value_completer()
        )]
        workspace: String,
    },
    #[command(
        about = "List known workspaces with path and commit details",
        visible_alias = "ls"
    )]
    List {
        #[arg(long, short = 'j', help = "Render workspaces as JSON")]
        json: bool,

        #[arg(long, short = 'c', help = "Render compact JSON", requires = "json")]
        compact: bool,
    },
    #[command(about = "Inspect repo, workspace, and shell health")]
    Doctor {
        #[arg(long, short = 'j', help = "Render diagnostics as JSON")]
        json: bool,

        #[arg(long, short = 'c', help = "Render compact JSON", requires = "json")]
        compact: bool,
    },
    #[command(
        about = "Forget a non-current workspace and delete its directory",
        visible_alias = "rm"
    )]
    Remove {
        #[arg(long, short = 'y', help = "Skip destructive confirmation")]
        yes: bool,

        #[arg(help = "Workspace name to remove", add = completion::workspace_value_completer())]
        workspace: String,
    },
    #[command(about = "Merge work from another JJ workspace")]
    Merge {
        #[arg(
            long,
            short = 'f',
            help = "Source workspace to merge from",
            add = completion::workspace_value_completer()
        )]
        from: String,

        #[arg(
            long,
            short = 'i',
            help = "Target workspace to merge into; defaults to current",
            add = completion::workspace_value_completer()
        )]
        into: Option<String>,
    },
    #[command(about = "Shell integration and future config commands")]
    #[command(arg_required_else_help = true)]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    #[command(about = "Shell integration commands")]
    #[command(arg_required_else_help = true)]
    Shell {
        #[command(subcommand)]
        command: ShellCommands,
    },
}

#[derive(Subcommand)]
enum ShellCommands {
    #[command(about = "Print shell integration script for a supported shell")]
    Init {
        #[arg(value_name = "SHELL", help = "Supported shell", value_enum)]
        shell: Option<ShellKind>,
    },
    #[command(about = "Install the managed shell integration block into your rc file")]
    Install {
        #[arg(
            long,
            short = 's',
            help = "Shell to install for; defaults to $SHELL",
            value_enum
        )]
        shell: Option<ShellKind>,
    },
}

enum CliError {
    Clap(clap::Error),
    Domain(crate::Error),
}

impl From<clap::Error> for CliError {
    fn from(value: clap::Error) -> Self {
        Self::Clap(value)
    }
}

impl From<crate::Error> for CliError {
    fn from(value: crate::Error) -> Self {
        Self::Domain(value)
    }
}

/// Run the CLI binary entrypoint with the provided binary name and argv.
#[must_use]
pub fn run(bin_name: &'static str, args: impl IntoIterator<Item = OsString>) -> ExitCode {
    if completion::maybe_handle_env_completion(bin_name) {
        return ExitCode::SUCCESS;
    }

    match try_run(bin_name, args) {
        Ok(exit_code) => exit_code,
        Err(CliError::Clap(error)) => {
            let exit_code = error.exit_code();
            if error.print().is_err() {
                eprintln!("{}", render_error_message(&error.to_string()));
            }
            ExitCode::from(u8::try_from(exit_code).unwrap_or(1))
        }
        Err(CliError::Domain(error)) => {
            eprintln!("{}", render_error_message(&error.to_string()));
            ExitCode::FAILURE
        }
    }
}

fn try_run(
    bin_name: &'static str,
    args: impl IntoIterator<Item = OsString>,
) -> Result<ExitCode, CliError> {
    let cli = parse(bin_name, args)?;
    let path = PathBuf::from(".");

    match cli.command {
        Commands::Switch {
            create,
            revision,
            workspace,
        } => commands::switch::run_switch(&path, &workspace, create, revision.as_deref())?,
        Commands::List { json, compact } => commands::list::run_list(&path, json, compact)?,
        Commands::Doctor { json, compact } => {
            return Ok(commands::doctor::run_doctor(
                &path, bin_name, json, compact,
            )?);
        }
        Commands::Remove { yes, workspace } => {
            commands::remove::run_remove(&path, &workspace, yes)?;
        }
        Commands::Merge { from, into } => {
            commands::merge::run_merge(&path, &from, into.as_deref())?;
        }
        Commands::Config { command } => match command {
            ConfigCommands::Shell { command } => match command {
                ShellCommands::Init { shell } => {
                    commands::config_shell::run_shell_init(bin_name, shell)?;
                }
                ShellCommands::Install { shell } => {
                    commands::config_shell::run_shell_install(bin_name, shell)?;
                }
            },
        },
    }

    Ok(ExitCode::SUCCESS)
}

fn parse(
    bin_name: &'static str,
    args: impl IntoIterator<Item = OsString>,
) -> Result<Cli, clap::Error> {
    let mut command = build_command();
    command = command.name(bin_name);
    let matches = command.try_get_matches_from(args)?;
    Cli::from_arg_matches(&matches)
}

pub(crate) fn build_command() -> clap::Command {
    Cli::command().styles(crate::output::clap_styles())
}
