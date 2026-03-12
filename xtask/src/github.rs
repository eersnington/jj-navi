use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::Path;
use std::process::Command;

use semver::Version;
use serde::Deserialize;

use crate::error::ToolError;
use crate::project;

const RELEASE_MAJOR_LABEL: &str = "release:major";
const RELEASE_MINOR_LABEL: &str = "release:minor";
const RELEASE_PATCH_LABEL: &str = "release:patch";
const RELEASE_NONE_LABEL: &str = "release:none";
pub(crate) const RELEASE_PR_MARKER: &str = "<!-- navi-release:release-pr -->";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ReleaseKind {
    None,
    Patch,
    Minor,
    Major,
}

impl ReleaseKind {
    pub(crate) fn from_label(label: &str) -> Option<Self> {
        match label {
            RELEASE_NONE_LABEL => Some(Self::None),
            RELEASE_PATCH_LABEL => Some(Self::Patch),
            RELEASE_MINOR_LABEL => Some(Self::Minor),
            RELEASE_MAJOR_LABEL => Some(Self::Major),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Patch => "patch",
            Self::Minor => "minor",
            Self::Major => "major",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReleasePullRequest {
    pub(crate) number: u64,
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) author_display: String,
    pub(crate) release: ReleaseKind,
    pub(crate) merged_at: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedRelease {
    pub(crate) base_tag: Option<String>,
    pub(crate) included_prs: Vec<ReleasePullRequest>,
    pub(crate) skipped_prs: Vec<SkippedPullRequest>,
    pub(crate) highest_release: ReleaseKind,
}

#[derive(Debug, Clone)]
pub(crate) struct SkippedPullRequest {
    pub(crate) number: u64,
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) author_display: String,
}

#[derive(Debug, Deserialize)]
struct PullRequestEvent {
    pull_request: EventPullRequest,
}

#[derive(Debug, Deserialize)]
struct EventPullRequest {
    number: u64,
    title: String,
    labels: Vec<GithubLabel>,
}

#[derive(Debug, Deserialize)]
struct GithubLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GithubUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct AssociatedPullRequest {
    number: u64,
    title: String,
    #[serde(rename = "html_url")]
    url: String,
    body: Option<String>,
    #[serde(rename = "merged_at")]
    merged_at: Option<String>,
    user: GithubUser,
    labels: Vec<GithubLabel>,
}

#[derive(Debug, Deserialize)]
struct GithubUserProfile {
    name: Option<String>,
}

pub(crate) fn validate_pull_request_event(event_path: &Path) -> Result<(), ToolError> {
    let event: PullRequestEvent = serde_json::from_str(&project::read_text(event_path)?)?;
    let release = parse_release_label(
        event.pull_request.number,
        &event.pull_request.title,
        &event.pull_request.labels,
    )?;

    println!(
        "PR #{} valid for release: {}.",
        event.pull_request.number,
        release.as_str()
    );
    Ok(())
}

pub(crate) fn prepare_release(repo_root: &Path, repo: &str) -> Result<PreparedRelease, ToolError> {
    let base_tag = last_release_tag(repo_root)?;
    let commit_shas = commits_since_tag(repo_root, base_tag.as_deref())?;
    let mut pull_requests = BTreeMap::new();

    for sha in commit_shas {
        for pr in associated_pull_requests(repo, &sha)? {
            if pr.merged_at.is_none() {
                continue;
            }
            if is_generated_release_pr(pr.body.as_deref()) {
                continue;
            }
            pull_requests.entry(pr.number).or_insert(pr);
        }
    }

    if pull_requests.is_empty() {
        return Err(ToolError::message(match &base_tag {
            Some(tag) => format!("no merged releasable PRs found since {tag}"),
            None => "no merged releasable PRs found".to_owned(),
        }));
    }

    let author_names = author_display_names(
        &pull_requests
            .values()
            .map(|pr| pr.user.login.clone())
            .collect::<BTreeSet<_>>(),
    )?;

    let mut included_prs = Vec::new();
    let mut skipped_prs = Vec::new();
    let mut highest_release = ReleaseKind::None;

    for pr in pull_requests.into_values() {
        let release = parse_release_label(pr.number, &pr.title, &pr.labels)?;
        let author_display = author_names
            .get(&pr.user.login)
            .cloned()
            .unwrap_or_else(|| format!("@{}", pr.user.login));

        if release == ReleaseKind::None {
            skipped_prs.push(SkippedPullRequest {
                number: pr.number,
                title: pr.title,
                url: pr.url,
                author_display,
            });
            continue;
        }

        if release > highest_release {
            highest_release = release;
        }

        included_prs.push(ReleasePullRequest {
            number: pr.number,
            title: pr.title,
            url: pr.url,
            author_display,
            release,
            merged_at: pr.merged_at.unwrap_or_default(),
        });
    }

    if included_prs.is_empty() {
        return Err(ToolError::message(match &base_tag {
            Some(tag) => format!("all merged PRs since {tag} are marked {RELEASE_NONE_LABEL}"),
            None => format!("all merged PRs are marked {RELEASE_NONE_LABEL}"),
        }));
    }

    included_prs.sort_by(|left, right| {
        left.merged_at
            .cmp(&right.merged_at)
            .then(left.number.cmp(&right.number))
    });
    skipped_prs.sort_by(|left, right| left.number.cmp(&right.number));

    Ok(PreparedRelease {
        base_tag,
        included_prs,
        skipped_prs,
        highest_release,
    })
}

pub(crate) fn ensure_version_matches_release(
    current: &Version,
    target: &Version,
    required: ReleaseKind,
) -> Result<(), ToolError> {
    if target <= current {
        return Err(ToolError::message(format!(
            "release version must be greater than current version {current}"
        )));
    }

    let actual = classify_version_change(current, target);
    if actual != required {
        return Err(ToolError::message(format!(
            "requested version {target} is a {} bump from {current}, but included PRs require {}. suggested: {}",
            actual.as_str(),
            required.as_str(),
            suggested_version(current, required)
        )));
    }

    Ok(())
}

pub(crate) fn render_changelog_section(
    version: &str,
    date: &str,
    prs: &[ReleasePullRequest],
) -> String {
    let mut lines = vec![format!("## v{version} - {date}"), String::new()];
    for pr in prs {
        lines.push(format!(
            "- {} ([#{}]({}), {})",
            pr.title, pr.number, pr.url, pr.author_display
        ));
    }
    lines.join("\n") + "\n"
}

pub(crate) fn render_release_pr_body(version: &str, prepared: &PreparedRelease) -> String {
    let base = prepared.base_tag.as_deref().unwrap_or("repo start");
    let mut lines = vec![
        RELEASE_PR_MARKER.to_owned(),
        format!("Prepare release `v{version}`."),
        String::new(),
        format!("Included merged PRs since `{base}`."),
        format!(
            "Highest release label: `{}`",
            prepared.highest_release.as_str()
        ),
        String::new(),
        "Included PRs:".to_owned(),
    ];

    for pr in &prepared.included_prs {
        lines.push(format!(
            "- [#{}]({}) {} - {}",
            pr.number, pr.url, pr.author_display, pr.title
        ));
    }

    if !prepared.skipped_prs.is_empty() {
        lines.push(String::new());
        lines.push("Skipped PRs (`release:none`):".to_owned());
        for pr in &prepared.skipped_prs {
            lines.push(format!(
                "- [#{}]({}) {} - {}",
                pr.number, pr.url, pr.author_display, pr.title
            ));
        }
    }

    lines.push(String::new());
    lines.push("Generated from merged PR titles and labels.".to_owned());
    lines.join("\n") + "\n"
}

pub(crate) fn default_repo() -> Result<String, ToolError> {
    env::var("GITHUB_REPOSITORY")
        .map_err(|_| ToolError::message("--repo required outside GitHub Actions"))
}

fn parse_release_label(
    number: u64,
    title: &str,
    labels: &[GithubLabel],
) -> Result<ReleaseKind, ToolError> {
    if title.trim().is_empty() {
        return Err(ToolError::message(format!("PR #{number} title is empty")));
    }

    let release_labels = labels
        .iter()
        .filter_map(|label| ReleaseKind::from_label(&label.name))
        .collect::<Vec<_>>();
    if release_labels.len() != 1 {
        return Err(ToolError::message(format!(
            "PR #{number} must have exactly one release label: {RELEASE_MAJOR_LABEL}, {RELEASE_MINOR_LABEL}, {RELEASE_PATCH_LABEL}, or {RELEASE_NONE_LABEL}"
        )));
    }

    Ok(release_labels[0])
}

fn is_generated_release_pr(body: Option<&str>) -> bool {
    body.is_some_and(|value| value.contains(RELEASE_PR_MARKER))
}

fn classify_version_change(current: &Version, target: &Version) -> ReleaseKind {
    if target.major != current.major {
        ReleaseKind::Major
    } else if target.minor != current.minor {
        ReleaseKind::Minor
    } else if target.patch != current.patch
        || target.pre != current.pre
        || target.build != current.build
    {
        ReleaseKind::Patch
    } else {
        ReleaseKind::None
    }
}

fn suggested_version(current: &Version, required: ReleaseKind) -> Version {
    let mut next = current.clone();
    next.pre = semver::Prerelease::EMPTY;
    next.build = semver::BuildMetadata::EMPTY;

    match required {
        ReleaseKind::None => next,
        ReleaseKind::Patch => {
            next.patch += 1;
            next
        }
        ReleaseKind::Minor => {
            next.minor += 1;
            next.patch = 0;
            next
        }
        ReleaseKind::Major => {
            next.major += 1;
            next.minor = 0;
            next.patch = 0;
            next
        }
    }
}

fn last_release_tag(repo_root: &Path) -> Result<Option<String>, ToolError> {
    let output = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0", "--match", "v*"])
        .current_dir(repo_root)
        .output()?;

    if output.status.success() {
        return Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        ));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("No names found") || stderr.contains("No tags can describe") {
        Ok(None)
    } else {
        Err(ToolError::message(stderr.trim().to_owned()))
    }
}

fn commits_since_tag(repo_root: &Path, tag: Option<&str>) -> Result<Vec<String>, ToolError> {
    let range = tag.map_or_else(|| "HEAD".to_owned(), |value| format!("{value}..HEAD"));
    let output = Command::new("git")
        .args(["rev-list", "--reverse", &range])
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        return Err(ToolError::message(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect())
}

fn associated_pull_requests(
    repo: &str,
    sha: &str,
) -> Result<Vec<AssociatedPullRequest>, ToolError> {
    let output = Command::new("gh")
        .args([
            "api",
            "-H",
            "Accept: application/vnd.github+json",
            &format!("repos/{repo}/commits/{sha}/pulls"),
        ])
        .output()?;

    if !output.status.success() {
        return Err(ToolError::message(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }

    Ok(serde_json::from_slice(&output.stdout)?)
}

fn author_display_names(logins: &BTreeSet<String>) -> Result<BTreeMap<String, String>, ToolError> {
    let mut names = BTreeMap::new();
    for login in logins {
        let output = Command::new("gh")
            .args(["api", &format!("users/{login}")])
            .output()?;
        if !output.status.success() {
            names.insert(login.clone(), format!("@{login}"));
            continue;
        }

        let profile: GithubUserProfile = serde_json::from_slice(&output.stdout)?;
        let display = profile
            .name
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("@{login}"));
        names.insert(login.clone(), display);
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use super::{
        GithubLabel, RELEASE_NONE_LABEL, ReleaseKind, ensure_version_matches_release,
        is_generated_release_pr, parse_release_label, render_changelog_section,
    };

    fn label(name: &str) -> GithubLabel {
        GithubLabel {
            name: name.to_owned(),
        }
    }

    #[test]
    fn validates_releasable_pr_labels() {
        let release = parse_release_label(7, "add richer list output", &[label("release:minor")])
            .expect("label");

        assert_eq!(release, ReleaseKind::Minor);
    }

    #[test]
    fn rejects_multiple_release_labels() {
        let error = parse_release_label(
            7,
            "add richer list output",
            &[label("release:minor"), label("release:patch")],
        )
        .expect_err("multiple release labels should fail");

        assert!(error.to_string().contains("exactly one release label"));
    }

    #[test]
    fn allows_release_none() {
        let release = parse_release_label(9, "docs cleanup", &[label(RELEASE_NONE_LABEL)])
            .expect("release:none label");

        assert_eq!(release, ReleaseKind::None);
    }

    #[test]
    fn detects_generated_release_pr_marker() {
        assert!(is_generated_release_pr(Some(
            "intro\n<!-- navi-release:release-pr -->\noutro"
        )));
        assert!(!is_generated_release_pr(Some("normal body")));
        assert!(!is_generated_release_pr(None));
    }

    #[test]
    fn renders_author_name_and_pr_link_in_changelog() {
        let section = render_changelog_section(
            "1.2.3",
            "2026-03-13",
            &[super::ReleasePullRequest {
                number: 12,
                title: "add richer list output".to_owned(),
                url: "https://github.com/example/repo/pull/12".to_owned(),
                author_display: "E. Ersnington".to_owned(),
                release: ReleaseKind::Patch,
                merged_at: String::new(),
            }],
        );

        assert!(section.contains("[#12](https://github.com/example/repo/pull/12), E. Ersnington"));
    }

    #[test]
    fn rejects_wrong_bump_kind() {
        let current = Version::parse("0.1.0").expect("current");
        let target = Version::parse("0.1.1").expect("target");
        let error = ensure_version_matches_release(&current, &target, ReleaseKind::Minor)
            .expect_err("patch should not satisfy minor");

        assert!(error.to_string().contains("suggested: 0.2.0"));
    }

    #[test]
    fn accepts_matching_bump_kind() {
        let current = Version::parse("0.1.0").expect("current");
        let target = Version::parse("0.2.0").expect("target");

        ensure_version_matches_release(&current, &target, ReleaseKind::Minor)
            .expect("minor bump should pass");
    }
}
