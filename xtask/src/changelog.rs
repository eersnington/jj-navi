use std::fs;
use std::path::{Path, PathBuf};

use time::OffsetDateTime;

use crate::error::ToolError;
use crate::release::{PullRequestMetadata, ReleaseLabel, validate_changelog_prs};

pub(crate) fn today() -> String {
    let today = OffsetDateTime::now_utc().date();
    format!(
        "{:04}-{:02}-{:02}",
        today.year(),
        u8::from(today.month()),
        today.day()
    )
}

pub(crate) fn prepend_changelog(
    repo_root: &Path,
    version: &str,
    date: &str,
    prs: &[PullRequestMetadata],
) -> Result<(), ToolError> {
    let current = read_text(&changelog_path(repo_root))?;
    let section = build_changelog_section(version, date, prs)?;
    let first_entry_index = current.find("\n## ");
    let header = first_entry_index
        .map_or(current.trim_end(), |index| current[..index].trim_end())
        .to_owned();
    let rest = first_entry_index.map_or(String::new(), |index| {
        current[index..].trim_start().to_owned()
    });

    let next = if rest.is_empty() {
        format!("{header}\n\n{section}\n")
    } else {
        format!("{header}\n\n{section}\n\n{rest}\n")
    };

    write_text(&changelog_path(repo_root), &next)
}

pub(crate) fn release_notes(repo_root: &Path, version: &str) -> Result<String, ToolError> {
    Ok(format!(
        "{}\n",
        changelog_entry(repo_root, version)?.trim_end()
    ))
}

pub(crate) fn changelog_entry(repo_root: &Path, version: &str) -> Result<String, ToolError> {
    let changelog = read_text(&changelog_path(repo_root))?;
    changelog_entry_from_text(&changelog, version)
}

fn changelog_path(repo_root: &Path) -> PathBuf {
    repo_root.join("CHANGELOG.md")
}

fn build_changelog_section(
    version: &str,
    date: &str,
    prs: &[PullRequestMetadata],
) -> Result<String, ToolError> {
    validate_changelog_prs(prs)?;

    let body = build_changelog_body(prs)?;
    let mut lines = vec![format!("## v{version} - {date}")];
    if !body.is_empty() {
        lines.push(String::new());
        lines.push(body);
    }
    Ok(lines.join("\n").trim_end().to_owned())
}

fn build_changelog_body(prs: &[PullRequestMetadata]) -> Result<String, ToolError> {
    validate_changelog_prs(prs)?;

    let mut lines = Vec::new();
    for label in [
        ReleaseLabel::Major,
        ReleaseLabel::Minor,
        ReleaseLabel::Patch,
    ] {
        let section_prs = prs
            .iter()
            .filter(|pr| pr.release == label)
            .collect::<Vec<_>>();
        if section_prs.is_empty() {
            continue;
        }

        if let Some(heading) = label.heading() {
            lines.push(format!("### {heading}"));
            lines.push(String::new());
        }
        for pr in section_prs {
            lines.push(format!(
                "- {} (#{}, {})",
                pr.title.trim(),
                pr.number,
                pr.author.changelog_name()
            ));
        }
        lines.push(String::new());
    }

    Ok(lines.join("\n").trim_end().to_owned())
}

fn changelog_entry_from_text(changelog: &str, version: &str) -> Result<String, ToolError> {
    let marker = format!("## v{version} - ");
    let Some(start) = changelog.find(&marker) else {
        return Err(ToolError::Message(format!(
            "CHANGELOG entry for {version} not found"
        )));
    };

    let rest = &changelog[start..];
    let next_section = rest[marker.len()..]
        .find("\n## ")
        .map(|offset| offset + marker.len());
    let entry = next_section.map_or(rest, |end| &rest[..end]);
    Ok(entry.trim_end().to_owned())
}

fn read_text(path: &Path) -> Result<String, ToolError> {
    Ok(fs::read_to_string(path)?)
}

fn write_text(path: &Path, value: &str) -> Result<(), ToolError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, value.replace("\r\n", "\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::release::{PullRequestMetadata, ReleaseAuthor, ReleaseLabel};

    use super::build_changelog_section;

    fn pr(number: u64, release: ReleaseLabel, title: &str, merged_at: &str) -> PullRequestMetadata {
        PullRequestMetadata {
            number,
            title: title.to_owned(),
            author: ReleaseAuthor {
                login: format!("user-{number}"),
                display_name: format!("@user-{number}"),
            },
            merged_at: merged_at.to_owned(),
            merge_commit_sha: format!("sha-{number}"),
            release,
        }
    }

    #[test]
    fn builds_changelog_grouped_by_release_label() {
        let section = build_changelog_section(
            "1.2.3",
            "2026-03-13",
            &[
                pr(
                    10,
                    ReleaseLabel::Patch,
                    "fix workspace switching",
                    "2026-03-13T10:00:00Z",
                ),
                pr(
                    11,
                    ReleaseLabel::Major,
                    "rewrite config layout",
                    "2026-03-13T11:00:00Z",
                ),
                pr(
                    12,
                    ReleaseLabel::Minor,
                    "add doctor command",
                    "2026-03-13T12:00:00Z",
                ),
            ],
        )
        .expect("build changelog");

        assert!(section.contains("## v1.2.3 - 2026-03-13"));
        assert!(section.contains("### Major\n\n- rewrite config layout (#11, @user-11)"));
        assert!(section.contains("### Minor\n\n- add doctor command (#12, @user-12)"));
        assert!(section.contains("### Patch\n\n- fix workspace switching (#10, @user-10)"));
    }
}
