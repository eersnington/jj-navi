use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostics::{DoctorFinding, DoctorFindingCode, DoctorScope, DoctorSeverity};
use crate::error::{Error, Result};
use crate::types::ShellKind;

use super::managed_block::invalid_shell_rc_file;
use super::{MANAGED_BLOCK_END, MANAGED_BLOCK_START, ManagedBlockState, inspect_managed_block};

/// Render the managed shell block inserted into a shell rc file.
#[must_use]
pub fn render_shell_install_block(command_name: &str, shell: ShellKind) -> String {
    format!(
        "{MANAGED_BLOCK_START}\neval \"$(command {command_name} config shell init {shell})\"\n{MANAGED_BLOCK_END}\n",
        command_name = command_name,
        shell = shell.as_str(),
    )
}

pub(crate) fn shell_rc_path(shell: ShellKind) -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| Error::HomeDirectory)?;
    Ok(PathBuf::from(home).join(shell.rc_file_name()))
}

pub(crate) fn upsert_managed_block(existing: &str, block: &str, rc_path: &Path) -> Result<String> {
    match inspect_managed_block(existing) {
        ManagedBlockState::Present { start, end } => {
            let mut updated = String::new();
            updated.push_str(&existing[..start]);
            if !updated.is_empty() && !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push_str(block);
            let suffix = existing[end..].trim_start_matches('\n');
            if !suffix.is_empty() {
                if !updated.ends_with('\n') {
                    updated.push('\n');
                }
                updated.push_str(suffix);
                if !updated.ends_with('\n') {
                    updated.push('\n');
                }
            }
            Ok(updated)
        }
        ManagedBlockState::Invalid(message) => Err(invalid_shell_rc_file(rc_path, message)),
        ManagedBlockState::Missing => {
            let mut updated = String::new();
            updated.push_str(existing);
            if !updated.is_empty() && !updated.ends_with('\n') {
                updated.push('\n');
            }
            if !updated.is_empty() && !updated.ends_with("\n\n") {
                updated.push('\n');
            }
            updated.push_str(block);
            Ok(updated)
        }
    }
}

pub(crate) fn doctor_findings(command_name: &str) -> Result<Vec<DoctorFinding>> {
    let Ok(shell_var) = std::env::var("SHELL") else {
        return Ok(vec![shell_finding(
            DoctorSeverity::Warning,
            DoctorFindingCode::ShellDetectionFailed,
            String::from("unable to detect shell from $SHELL"),
            None,
            Some(String::from(
                "set $SHELL or pass --shell when installing integration",
            )),
        )]);
    };
    let shell_name = Path::new(&shell_var)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or(Error::ShellDetection)?;
    let shell = match ShellKind::new(shell_name) {
        Ok(shell) => shell,
        Err(Error::UnsupportedShell(shell)) => {
            return Ok(vec![shell_finding(
                DoctorSeverity::Warning,
                DoctorFindingCode::UnsupportedShell,
                format!("shell '{shell}' is not supported"),
                None,
                Some(String::from("supported shells: bash, zsh")),
            )]);
        }
        Err(error) => return Err(error),
    };

    let rc_path = match shell_rc_path(shell) {
        Ok(path) => path,
        Err(Error::HomeDirectory) => {
            return Ok(vec![shell_finding(
                DoctorSeverity::Warning,
                DoctorFindingCode::HomeDirectoryMissing,
                String::from("$HOME is not set; shell integration could not be checked"),
                None,
                None,
            )]);
        }
        Err(error) => return Err(error),
    };

    let contents = match fs::read_to_string(&rc_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(vec![shell_finding(
                DoctorSeverity::Info,
                DoctorFindingCode::ShellRcMissing,
                format!("shell rc file {} does not exist yet", rc_path.display()),
                Some(rc_path.display().to_string()),
                Some(shell_install_hint(command_name, shell)),
            )]);
        }
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
            return Ok(vec![shell_finding(
                DoctorSeverity::Error,
                DoctorFindingCode::InvalidShellRcFile,
                format!("shell rc file {} is not valid UTF-8", rc_path.display()),
                Some(rc_path.display().to_string()),
                None,
            )]);
        }
        Err(error) => return Err(error.into()),
    };

    let finding = match inspect_managed_block(&contents) {
        ManagedBlockState::Missing => Some(shell_finding(
            DoctorSeverity::Info,
            DoctorFindingCode::ShellIntegrationMissing,
            format!(
                "shell integration managed block is missing from {}",
                rc_path.display()
            ),
            Some(rc_path.display().to_string()),
            Some(shell_install_hint(command_name, shell)),
        )),
        ManagedBlockState::Present { .. } => None,
        ManagedBlockState::Invalid(message) => Some(shell_finding(
            DoctorSeverity::Error,
            DoctorFindingCode::InvalidShellRcFile,
            format!("invalid shell rc file at {}", rc_path.display()),
            Some(rc_path.display().to_string()),
            Some(message.to_owned()),
        )),
    };

    Ok(finding.into_iter().collect())
}

fn shell_finding(
    severity: DoctorSeverity,
    code: DoctorFindingCode,
    message: String,
    path: Option<String>,
    hint: Option<String>,
) -> DoctorFinding {
    DoctorFinding {
        severity,
        code,
        scope: DoctorScope::Shell,
        message,
        path,
        hint,
    }
}

fn shell_install_hint(command_name: &str, shell: ShellKind) -> String {
    format!(
        "run: {command_name} config shell install --shell {}",
        shell.as_str()
    )
}
