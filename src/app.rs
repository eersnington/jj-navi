use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

use crate::cli;
use crate::output::{clap_styles, render_error_message};
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
    #[command(about = "Switch to an existing workspace or create one with --create")]
    Switch {
        #[arg(long, short = 'c', help = "Create the workspace if it does not exist")]
        create: bool,

        #[arg(long, help = "Revision to base a newly created workspace on")]
        revision: Option<String>,

        #[arg(help = "Workspace name")]
        workspace: String,
    },
    #[command(about = "List known workspaces with path and commit details")]
    List,
    #[command(about = "Inspect repo, workspace, and shell health")]
    Doctor {
        #[arg(long, help = "Render diagnostics as JSON")]
        json: bool,

        #[arg(long, help = "Render compact JSON", requires = "json")]
        compact: bool,
    },
    #[command(about = "Forget a non-current workspace")]
    Remove {
        #[arg(help = "Workspace name to forget")]
        workspace: String,
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
        #[arg(long, help = "Shell to install for; defaults to $SHELL", value_enum)]
        shell: Option<ShellKind>,
    },
}

enum AppError {
    Cli(clap::Error),
    Domain(crate::Error),
}

impl From<clap::Error> for AppError {
    fn from(value: clap::Error) -> Self {
        Self::Cli(value)
    }
}

impl From<crate::Error> for AppError {
    fn from(value: crate::Error) -> Self {
        Self::Domain(value)
    }
}

#[must_use]
/// Run the CLI entrypoint for the provided binary name and argv.
pub fn main(bin_name: &'static str, args: impl IntoIterator<Item = OsString>) -> ExitCode {
    match run(bin_name, args) {
        Ok(exit_code) => exit_code,
        Err(AppError::Cli(error)) => {
            let exit_code = error.exit_code();
            if error.print().is_err() {
                eprintln!("{}", render_error_message(&error.to_string()));
            }
            ExitCode::from(u8::try_from(exit_code).unwrap_or(1))
        }
        Err(AppError::Domain(error)) => {
            eprintln!("{}", render_error_message(&error.to_string()));
            ExitCode::FAILURE
        }
    }
}

fn run(
    bin_name: &'static str,
    args: impl IntoIterator<Item = OsString>,
) -> Result<ExitCode, AppError> {
    let cli = parse_cli(bin_name, args)?;
    let path = PathBuf::from(".");

    match cli.command {
        Commands::Switch {
            create,
            revision,
            workspace,
        } => cli::run_switch(&path, &workspace, create, revision.as_deref())?,
        Commands::List => cli::run_list(&path)?,
        Commands::Doctor { json, compact } => {
            return Ok(cli::run_doctor(&path, bin_name, json, compact)?);
        }
        Commands::Remove { workspace } => cli::run_remove(&path, &workspace)?,
        Commands::Config { command } => match command {
            ConfigCommands::Shell { command } => match command {
                ShellCommands::Init { shell } => cli::run_shell_init(bin_name, shell)?,
                ShellCommands::Install { shell } => {
                    cli::run_shell_install(bin_name, shell)?;
                }
            },
        },
    }

    Ok(ExitCode::SUCCESS)
}

fn parse_cli(
    bin_name: &'static str,
    args: impl IntoIterator<Item = OsString>,
) -> Result<Cli, clap::Error> {
    let mut command = build_command();
    command = command.name(bin_name);
    let matches = command.try_get_matches_from(args)?;
    Cli::from_arg_matches(&matches)
}

fn build_command() -> clap::Command {
    Cli::command().styles(clap_styles())
}
