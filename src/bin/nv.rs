use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use jj_navi::cli;

#[derive(Parser)]
#[command(name = "nv")]
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
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> jj_navi::Result<()> {
    let cli = Cli::parse();
    let path = PathBuf::from(".");

    match cli.command {
        Commands::Switch {
            create,
            revision,
            workspace,
        } => cli::run_switch(&path, &workspace, create, revision.as_deref()),
        Commands::List => cli::run_list(&path),
    }
}
