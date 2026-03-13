mod app;
mod changelog;
mod cli;
mod error;
mod project;
mod release;

use std::env;
use std::process::ExitCode;

use crate::cli::{Command, HELP_TEXT, parse_command};
use crate::error::ToolError;

fn main() -> ExitCode {
    match try_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn try_main() -> Result<(), ToolError> {
    match parse_command(env::args_os().skip(1).collect())? {
        command @ (Command::Prepare { .. } | Command::Validate { .. } | Command::Notes { .. }) => {
            app::run(command)
        }
        Command::Help => {
            print!("{HELP_TEXT}");
            Ok(())
        }
        Command::Version => {
            println!("navi-release {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
