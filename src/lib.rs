#![warn(missing_docs)]

//! `jj-navi` is a small CLI-focused library for navigating Jujutsu workspaces.
//! It exposes the binary entrypoint plus a narrow set of formatting and domain
//! types used by the test suite.

mod cli;
mod commands;
mod completion;
pub mod diagnostics;
mod error;
pub mod output;
mod repo;
mod shell;
pub mod types;

/// Run the CLI binary entrypoint with the provided binary name and argv.
pub use cli::run;
/// Crate-wide error type.
pub use error::{Error, Result};
