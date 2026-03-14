use std::path::Path;
use std::process::ExitCode;

use crate::Result;
use crate::output::{render_doctor_report, render_doctor_report_json};
use crate::repo::NaviWorkspace;

/// Run the `doctor` command.
///
/// # Errors
///
pub fn run_doctor(path: &Path, command_name: &str, json: bool, compact: bool) -> Result<ExitCode> {
    let report = NaviWorkspace::doctor(path, command_name)?;

    if json {
        println!("{}", render_doctor_report_json(&report, compact)?);
    } else {
        print!("{}", render_doctor_report(&report));
    }

    Ok(if report.has_errors() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
