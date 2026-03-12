mod app;
mod cli;
mod error;
mod github;
mod project;

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match app::run(env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
