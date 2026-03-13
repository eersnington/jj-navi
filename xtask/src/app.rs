use std::env;
use std::ffi::OsString;

use crate::cli::{Command, HELP_TEXT, PrCommand, parse_command};
use crate::error::ToolError;
use crate::github;
use crate::project;

pub(crate) fn run(args: Vec<OsString>) -> Result<(), ToolError> {
    match parse_command(args)? {
        Command::Prepare(command) => {
            let repo_root = project::find_repo_root(&env::current_dir()?)?;
            let version = project::parse_version(&command.version)?;
            let current_version = project::current_release_version(&repo_root)?;

            let repo = match command.repo {
                Some(repo) => repo,
                None => github::default_repo()?,
            };
            let prepared = github::prepare_release(&repo_root, &repo)?;
            github::ensure_version_matches_release(
                &current_version,
                &version,
                prepared.highest_release,
            )?;
            let version_text = version.to_string();

            project::sync_versions(&repo_root, &version_text)?;
            let section = github::render_changelog_section(
                &version_text,
                &project::today(),
                &prepared.included_prs,
            );
            project::prepend_changelog(&repo_root, &section)?;

            if let Some(path) = command.pr_body_path {
                let body = github::render_release_pr_body(&version_text, &prepared);
                project::write_text(&path, &body)?;
            }

            println!(
                "Prepared release {} from {} PR(s).",
                version,
                prepared.included_prs.len(),
            );
            Ok(())
        }
        Command::Pr(PrCommand::Validate(command)) => {
            let event_path = match command.event_path {
                Some(path) => path,
                None => env::var("GITHUB_EVENT_PATH")
                    .map(std::path::PathBuf::from)
                    .map_err(|_| {
                        ToolError::message("--event-path required outside GitHub Actions")
                    })?,
            };
            github::validate_pull_request_event(&event_path)
        }
        Command::Validate { version } => {
            let repo_root = project::find_repo_root(&env::current_dir()?)?;
            let target = match version {
                Some(value) => project::parse_version(&value)?,
                None => project::current_release_version(&repo_root)?,
            };

            project::ensure_versions_match(&repo_root, &target)?;
            if !project::changelog_has_version(&repo_root, target.to_string().as_str())? {
                return Err(ToolError::message(format!(
                    "CHANGELOG entry for {target} not found"
                )));
            }

            println!("Validated release files for {target}.");
            Ok(())
        }
        Command::Notes { version } => {
            let repo_root = project::find_repo_root(&env::current_dir()?)?;
            print!("{}", project::release_notes(&repo_root, &version)?);
            Ok(())
        }
        Command::CurrentVersion => {
            let repo_root = project::find_repo_root(&env::current_dir()?)?;
            println!("{}", project::current_release_version(&repo_root)?);
            Ok(())
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
