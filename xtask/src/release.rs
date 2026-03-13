use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::error::ToolError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ReleaseLabel {
    Major,
    Minor,
    Patch,
    None,
}

impl ReleaseLabel {
    pub(crate) fn heading(self) -> Option<&'static str> {
        match self {
            Self::Major => Some("Major"),
            Self::Minor => Some("Minor"),
            Self::Patch => Some("Patch"),
            Self::None => None,
        }
    }

    pub(crate) fn is_user_facing(self) -> bool {
        self != Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReleaseInput {
    pub(crate) schema: u32,
    pub(crate) previous_tag: Option<String>,
    pub(crate) prs: Vec<PullRequestMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PullRequestMetadata {
    pub(crate) number: u64,
    pub(crate) title: String,
    pub(crate) author: ReleaseAuthor,
    pub(crate) merged_at: String,
    pub(crate) merge_commit_sha: String,
    pub(crate) release: ReleaseLabel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReleaseAuthor {
    pub(crate) login: String,
    pub(crate) display_name: String,
}

impl ReleaseAuthor {
    pub(crate) fn changelog_name(&self) -> &str {
        if self.display_name.trim().is_empty() {
            self.login.trim()
        } else {
            self.display_name.trim()
        }
    }
}

pub(crate) fn validate_release_input(input: &ReleaseInput) -> Result<(), ToolError> {
    if input.schema != 1 {
        return Err(ToolError::Message(format!(
            "release input schema mismatch: expected 1, got {}",
            input.schema
        )));
    }
    validate_pull_requests(&input.prs, true)
}

pub(crate) fn validate_changelog_prs(prs: &[PullRequestMetadata]) -> Result<(), ToolError> {
    validate_pull_requests(prs, false)
}

pub(crate) fn sort_pull_requests(prs: &mut [PullRequestMetadata]) {
    prs.sort_by(|left, right| {
        left.merged_at
            .cmp(&right.merged_at)
            .then(left.number.cmp(&right.number))
    });
}

fn validate_pull_requests(
    prs: &[PullRequestMetadata],
    allow_release_none: bool,
) -> Result<(), ToolError> {
    if prs.is_empty() {
        return Err(ToolError::Message(
            "release metadata must include PRs".to_owned(),
        ));
    }

    let mut seen_numbers = BTreeSet::new();
    for pr in prs {
        if !seen_numbers.insert(pr.number) {
            return Err(ToolError::Message(format!(
                "duplicate PR in release metadata: #{}",
                pr.number
            )));
        }
        if pr.title.trim().is_empty() {
            return Err(ToolError::Message(format!(
                "release metadata title missing for #{}",
                pr.number
            )));
        }
        if pr.author.login.trim().is_empty() {
            return Err(ToolError::Message(format!(
                "release metadata author login missing for #{}",
                pr.number
            )));
        }
        if pr.merged_at.trim().is_empty() {
            return Err(ToolError::Message(format!(
                "release metadata mergedAt missing for #{}",
                pr.number
            )));
        }
        if pr.merge_commit_sha.trim().is_empty() {
            return Err(ToolError::Message(format!(
                "release metadata mergeCommitSha missing for #{}",
                pr.number
            )));
        }
        if !allow_release_none && !pr.release.is_user_facing() {
            return Err(ToolError::Message(format!(
                "release changelog must exclude release:none PR #{}",
                pr.number
            )));
        }
    }

    Ok(())
}
