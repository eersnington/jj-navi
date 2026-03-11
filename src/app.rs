use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

use crate::cli;

#[derive(Parser)]
#[command(about = "Workspace navigator for Jujutsu")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Switch {
        #[arg(long, short = 'c')]
        create: bool,

        #[arg(long)]
        revision: Option<String>,

        workspace: String,
    },
    List,
    Remove {
        workspace: Option<String>,
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
pub fn main(bin_name: &'static str, args: impl IntoIterator<Item = OsString>) -> ExitCode {
    match run(bin_name, args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(AppError::Cli(error)) => {
            let exit_code = error.exit_code();
            if error.print().is_err() {
                eprintln!("{error}");
            }
            ExitCode::from(u8::try_from(exit_code).unwrap_or(1))
        }
        Err(AppError::Domain(error)) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(bin_name: &'static str, args: impl IntoIterator<Item = OsString>) -> Result<(), AppError> {
    let cli = parse_cli(bin_name, args)?;
    let path = PathBuf::from(".");

    match cli.command {
        Commands::Switch {
            create,
            revision,
            workspace,
        } => cli::run_switch(&path, &workspace, create, revision.as_deref())?,
        Commands::List => cli::run_list(&path)?,
        Commands::Remove { workspace } => cli::run_remove(&path, workspace.as_deref())?,
    }

    Ok(())
}

fn parse_cli(
    bin_name: &'static str,
    args: impl IntoIterator<Item = OsString>,
) -> Result<Cli, clap::Error> {
    let mut command = Cli::command();
    command = command.name(bin_name);
    let matches = command.try_get_matches_from(args)?;
    Cli::from_arg_matches(&matches)
}
