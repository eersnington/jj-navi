use std::env;
use std::path::PathBuf;

use crate::changelog::{changelog_entry, prepend_changelog, release_notes, today};
use crate::cli::Command;
use crate::error::ToolError;
use crate::project::{
    current_cargo_version, current_package_version, ensure_versions_match, find_repo_root,
    parse_version, read_json_typed, sync_versions,
};
use crate::release::{ReleaseInput, sort_pull_requests, validate_release_input};

pub(crate) fn run(command: Command) -> Result<(), ToolError> {
    match command {
        Command::Prepare {
            version,
            input_path,
        } => {
            let repo_root = find_repo_root(&env::current_dir()?)?;
            run_prepare(&repo_root, &version, input_path.as_deref())
        }
        Command::Validate { version } => {
            let repo_root = find_repo_root(&env::current_dir()?)?;
            run_validate(&repo_root, version.as_deref())
        }
        Command::Notes { version } => {
            let repo_root = find_repo_root(&env::current_dir()?)?;
            run_notes(&repo_root, &version)
        }
        Command::Help | Command::Version => unreachable!("handled in main"),
    }
}

fn run_prepare(
    repo_root: &std::path::Path,
    version: &str,
    input_path: Option<&str>,
) -> Result<(), ToolError> {
    let version = parse_version(version)?;
    let cargo_version = current_cargo_version(repo_root)?;
    let package_version = current_package_version(repo_root)?;

    if cargo_version != package_version {
        return Err(ToolError::Message(format!(
            "version drift before release: Cargo={cargo_version}, npm={package_version}",
        )));
    }

    if version <= cargo_version {
        return Err(ToolError::Message(format!(
            "release version must be greater than current version {cargo_version}",
        )));
    }

    let input_path = input_path
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.join(".github/release-input.json"));
    let mut release_input: ReleaseInput = read_json_typed(&input_path)?;
    let _ = &release_input.previous_tag;
    validate_release_input(&release_input)?;
    sort_pull_requests(&mut release_input.prs);

    let included_prs = release_input
        .prs
        .into_iter()
        .filter(|pr| pr.release.is_user_facing())
        .collect::<Vec<_>>();
    if included_prs.is_empty() {
        return Err(ToolError::Message(
            "release input has no user-facing PRs".to_owned(),
        ));
    }

    let version_text = version.to_string();
    sync_versions(repo_root, &version_text)?;
    prepend_changelog(repo_root, &version_text, &today(), &included_prs)?;

    println!(
        "Prepared release {} from {} PR(s).",
        version,
        included_prs.len()
    );
    Ok(())
}

fn run_validate(repo_root: &std::path::Path, version: Option<&str>) -> Result<(), ToolError> {
    let target = match version {
        Some(value) => parse_version(value)?,
        None => current_cargo_version(repo_root)?,
    };

    ensure_versions_match(repo_root, &target)?;
    changelog_entry(repo_root, &target.to_string())?;

    println!("Validated release files for {target}.");
    Ok(())
}

fn run_notes(repo_root: &std::path::Path, version: &str) -> Result<(), ToolError> {
    print!("{}", release_notes(repo_root, version)?);
    Ok(())
}
