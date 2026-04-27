use pathdiff::diff_paths;
use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::types::{
    WorkspaceDiffSnapshot, WorkspaceDiffStatus, WorkspaceListEntry, WorkspaceListStatus,
    WorkspacePathState, WorkspaceSnapshot,
};

use super::{
    INFERRED_ISSUE_URL, hyperlink, hyperlink_enabled, pad_visible, style_list_badge, style_meta,
    style_table_header, style_workspace_name, styled_stdout_text, theme,
};

use std::env;
use std::fmt::Write;
use std::path::Path;

const CURRENT_HEADER: &str = "cur";
const WORKSPACE_HEADER: &str = "workspace";
const STATUS_HEADER: &str = "status";
const DIFF_HEADER: &str = "diff";
const PATH_HEADER: &str = "path";
const COMMIT_HEADER: &str = "commit";
const AGE_HEADER: &str = "age";
const MESSAGE_HEADER: &str = "message";
const COLUMN_SEPARATOR_WIDTH: usize = 2;
const MESSAGE_MIN_WIDTH: usize = 10;

/// Render a table of workspaces for `navi list`.
#[must_use]
pub fn render_workspace_table(entries: &[WorkspaceListEntry]) -> String {
    render_workspace_table_with_width(entries, terminal_width_from_columns_env())
}

/// Render a table of workspaces using an explicit terminal width.
///
/// This is exposed for integration tests; CLI code should use
/// [`render_workspace_table`] so width detection stays centralized.
#[doc(hidden)]
#[must_use]
pub fn render_workspace_table_with_width(
    entries: &[WorkspaceListEntry],
    terminal_width: Option<usize>,
) -> String {
    let rendered_entries = entries
        .iter()
        .map(|entry| RenderedWorkspaceEntry {
            is_current: entry.is_current,
            name: render_workspace_name(entry.name.as_str(), entry.is_current),
            status: render_workspace_status(entry),
            diff: render_workspace_diff(&entry.diff),
            path: render_workspace_path(entry),
            commit_id: render_commit_id(entry.commit_id.as_str()),
            message: entry.message.as_str(),
            age: render_workspace_age(entry.age.created_at),
        })
        .collect::<Vec<_>>();

    let workspace_width = rendered_entries
        .iter()
        .map(|entry| entry.name.visible_len)
        .fold(WORKSPACE_HEADER.len(), usize::max);
    let path_width = rendered_entries
        .iter()
        .map(|entry| entry.path.visible_len)
        .fold(PATH_HEADER.len(), usize::max);
    let status_width = rendered_entries
        .iter()
        .map(|entry| entry.status.visible_len)
        .fold(STATUS_HEADER.len(), usize::max);
    let commit_width = rendered_entries
        .iter()
        .map(|entry| entry.commit_id.visible_len)
        .fold(COMMIT_HEADER.len(), usize::max);
    let diff_width = rendered_entries
        .iter()
        .map(|entry| entry.diff.visible_len)
        .fold(DIFF_HEADER.len(), usize::max);
    let age_width = rendered_entries
        .iter()
        .map(|entry| entry.age.visible_len)
        .fold(AGE_HEADER.len(), usize::max);
    let fixed_width_before_message = CURRENT_HEADER.len()
        + workspace_width
        + status_width
        + diff_width
        + path_width
        + commit_width
        + age_width
        + (COLUMN_SEPARATOR_WIDTH * 7);
    let message_width = terminal_width.map(|width| {
        width
            .saturating_sub(fixed_width_before_message)
            .max(MESSAGE_MIN_WIDTH)
    });

    let mut output = String::new();
    writeln!(
        output,
        "{}  {}  {}  {}  {}  {}  {}  {}",
        style_table_header(&format!("{CURRENT_HEADER:<3}")),
        style_table_header(&format!("{WORKSPACE_HEADER:<workspace_width$}")),
        style_table_header(&format!("{STATUS_HEADER:<status_width$}")),
        style_table_header(&format!("{DIFF_HEADER:<diff_width$}")),
        style_table_header(&format!("{PATH_HEADER:<path_width$}")),
        style_table_header(&format!("{COMMIT_HEADER:<commit_width$}")),
        style_table_header(&format!("{AGE_HEADER:<age_width$}")),
        style_table_header(&truncate_to_width(MESSAGE_HEADER, message_width)),
    )
    .expect("write table header");

    for entry in rendered_entries {
        let current_marker = if entry.is_current { "@" } else { "" };
        writeln!(
            output,
            "{}  {}  {}  {}  {}  {}  {}  {}",
            pad_visible(current_marker, current_marker.len(), CURRENT_HEADER.len()),
            pad_visible(
                &entry.name.rendered,
                entry.name.visible_len,
                workspace_width
            ),
            pad_visible(
                &entry.status.rendered,
                entry.status.visible_len,
                status_width
            ),
            pad_visible(&entry.diff.rendered, entry.diff.visible_len, diff_width),
            pad_visible(&entry.path.rendered, entry.path.visible_len, path_width),
            pad_visible(
                &entry.commit_id.rendered,
                entry.commit_id.visible_len,
                commit_width
            ),
            pad_visible(&entry.age.rendered, entry.age.visible_len, age_width),
            truncate_to_width(entry.message, message_width),
        )
        .expect("write table row");
    }

    output
}

fn terminal_width_from_columns_env() -> Option<usize> {
    env::var("COLUMNS")
        .ok()
        .and_then(|columns| columns.parse::<usize>().ok())
        .filter(|width| *width > 0)
}

fn truncate_to_width(value: &str, width: Option<usize>) -> String {
    let Some(width) = width else {
        return value.to_owned();
    };

    let value_width = value.chars().count();
    if value_width <= width {
        return value.to_owned();
    }

    if width <= 3 {
        return ".".repeat(width);
    }

    let keep = width - 3;
    let mut truncated = value.chars().take(keep).collect::<String>();
    truncated.push_str("...");
    truncated
}

/// Render workspace snapshots as JSON.
///
/// # Errors
///
/// Returns an error if the payload cannot be serialized.
pub fn render_workspace_list_json(
    workspace_root: &Path,
    snapshots: &[WorkspaceSnapshot],
    compact: bool,
) -> crate::Result<String> {
    let payload = WorkspaceListJsonOutput {
        workspaces: snapshots
            .iter()
            .map(|snapshot| WorkspaceJsonEntry {
                name: snapshot.name.as_str(),
                is_current: snapshot.is_current,
                commit_id: snapshot.commit_id.as_str(),
                message: snapshot.message.as_str(),
                path: WorkspacePathJson {
                    display: if snapshot.is_current {
                        String::from(".")
                    } else {
                        diff_paths(&snapshot.path.path, workspace_root)
                            .unwrap_or_else(|| snapshot.path.path.clone())
                            .display()
                            .to_string()
                    },
                    absolute: snapshot.path.path.display().to_string(),
                    state: snapshot.path.state.label(),
                    source: snapshot.path.source.label(),
                },
                health: WorkspaceHealthJson {
                    statuses: snapshot
                        .health
                        .statuses
                        .iter()
                        .map(|status| status.label())
                        .collect(),
                    metadata_status: snapshot.health.metadata_status.label(),
                },
                freshness: WorkspaceFreshnessJson {
                    status: snapshot.freshness.status.label(),
                    reason: snapshot.freshness.reason.as_deref(),
                },
                age: WorkspaceAgeJson {
                    created_at: snapshot
                        .age
                        .created_at
                        .and_then(|created_at| created_at.format(&Rfc3339).ok()),
                    display: snapshot.age.created_at.map(format_workspace_age),
                },
                diff: WorkspaceDiffJson {
                    status: snapshot.diff.status.label(),
                    files_changed: snapshot.diff.files_changed,
                    insertions: snapshot.diff.insertions,
                    deletions: snapshot.diff.deletions,
                    display: render_diff_plain(&snapshot.diff),
                },
            })
            .collect(),
    };

    let rendered = if compact {
        serde_json::to_string(&payload)
    } else {
        serde_json::to_string_pretty(&payload)
    }
    .map_err(|error| crate::Error::JsonSerialization(error.to_string()))?;

    Ok(rendered)
}

fn render_workspace_status(entry: &WorkspaceListEntry) -> RenderedCell {
    let plain = plain_workspace_status(entry);
    let rendered = entry
        .statuses
        .iter()
        .map(|status| render_status_badge(*status))
        .collect::<Vec<_>>()
        .join(" ");

    RenderedCell {
        rendered,
        visible_len: plain.len(),
    }
}

fn plain_workspace_status(entry: &WorkspaceListEntry) -> String {
    entry
        .statuses
        .iter()
        .map(|status| format!("[ {} ]", status.label()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_workspace_path(entry: &WorkspaceListEntry) -> RenderedCell {
    let plain = entry.path.display().to_string();
    let rendered = match entry.path_state {
        WorkspacePathState::Confirmed => plain.clone(),
        WorkspacePathState::Inferred => styled_stdout_text(&plain, theme().inferred_path),
        WorkspacePathState::Missing => styled_stdout_text(&plain, theme().missing_path),
        WorkspacePathState::Stale => styled_stdout_text(&plain, theme().stale_path),
    };

    RenderedCell {
        rendered,
        visible_len: plain.len(),
    }
}

fn render_workspace_diff(diff: &WorkspaceDiffSnapshot) -> RenderedCell {
    let plain = render_diff_plain(diff);
    let rendered = render_diff_styled(diff, &plain);

    RenderedCell {
        rendered,
        visible_len: plain.len(),
    }
}

fn render_diff_styled(diff: &WorkspaceDiffSnapshot, plain: &str) -> String {
    if diff.status == WorkspaceDiffStatus::Unknown || plain == "0" {
        return plain.to_owned();
    }

    let files_changed = diff.files_changed.unwrap_or(0);
    let insertions = diff.insertions.unwrap_or(0);
    let deletions = diff.deletions.unwrap_or(0);

    format!(
        "{} {} {}",
        styled_stdout_text(&format!("{files_changed}f"), theme().diff_files),
        styled_stdout_text(&format!("+{insertions}"), theme().diff_insertions),
        styled_stdout_text(&format!("-{deletions}"), theme().diff_deletions),
    )
}

fn render_diff_plain(diff: &WorkspaceDiffSnapshot) -> String {
    if diff.status == WorkspaceDiffStatus::Unknown {
        return String::from("unknown");
    }

    let files_changed = diff.files_changed.unwrap_or(0);
    let insertions = diff.insertions.unwrap_or(0);
    let deletions = diff.deletions.unwrap_or(0);

    if files_changed == 0 && insertions == 0 && deletions == 0 {
        return String::from("0");
    }

    format!("{files_changed}f +{insertions} -{deletions}")
}

fn render_workspace_age(created_at: Option<OffsetDateTime>) -> RenderedCell {
    let plain = created_at.map_or_else(|| String::from("-"), format_workspace_age);

    RenderedCell {
        rendered: plain.clone(),
        visible_len: plain.len(),
    }
}

fn format_workspace_age(created_at: OffsetDateTime) -> String {
    let elapsed = OffsetDateTime::now_utc() - created_at;
    let seconds = elapsed.whole_seconds().max(0);
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;
    let years = days / 365;

    if years > 0 {
        format!("{years}y")
    } else if days > 0 {
        format!("{days}d")
    } else if hours > 0 {
        format!("{hours}h")
    } else {
        format!("{}m", minutes.max(1))
    }
}

fn render_workspace_name(name: &str, is_current: bool) -> RenderedCell {
    RenderedCell {
        rendered: style_workspace_name(name, is_current),
        visible_len: name.len(),
    }
}

fn render_commit_id(commit_id: &str) -> RenderedCell {
    RenderedCell {
        rendered: style_meta(commit_id),
        visible_len: commit_id.len(),
    }
}

fn render_status_badge(status: WorkspaceListStatus) -> String {
    let label = status.label();
    let colored = match status {
        WorkspaceListStatus::Ok => style_list_badge(label, theme().ok_badge),
        WorkspaceListStatus::Inferred => style_list_badge(label, theme().inferred_badge),
        WorkspaceListStatus::Missing => style_list_badge(label, theme().missing_badge),
        WorkspaceListStatus::Stale => style_list_badge(label, theme().stale_badge),
        WorkspaceListStatus::JjOnly => style_list_badge(label, theme().jj_only_badge),
        WorkspaceListStatus::NotCurrent => style_list_badge(label, theme().warning_badge),
    };

    if status == WorkspaceListStatus::Inferred && hyperlink_enabled() {
        hyperlink(&colored, INFERRED_ISSUE_URL)
    } else {
        colored
    }
}

struct RenderedWorkspaceEntry<'a> {
    is_current: bool,
    name: RenderedCell,
    status: RenderedCell,
    diff: RenderedCell,
    path: RenderedCell,
    commit_id: RenderedCell,
    message: &'a str,
    age: RenderedCell,
}

struct RenderedCell {
    rendered: String,
    visible_len: usize,
}

#[derive(Serialize)]
struct WorkspaceListJsonOutput<'a> {
    workspaces: Vec<WorkspaceJsonEntry<'a>>,
}

#[derive(Serialize)]
struct WorkspaceJsonEntry<'a> {
    name: &'a str,
    is_current: bool,
    commit_id: &'a str,
    message: &'a str,
    path: WorkspacePathJson,
    health: WorkspaceHealthJson<'a>,
    freshness: WorkspaceFreshnessJson<'a>,
    age: WorkspaceAgeJson,
    diff: WorkspaceDiffJson,
}

#[derive(Serialize)]
struct WorkspacePathJson {
    display: String,
    absolute: String,
    state: &'static str,
    source: &'static str,
}

#[derive(Serialize)]
struct WorkspaceHealthJson<'a> {
    statuses: Vec<&'a str>,
    metadata_status: &'static str,
}

#[derive(Serialize)]
struct WorkspaceFreshnessJson<'a> {
    status: &'static str,
    reason: Option<&'a str>,
}

#[derive(Serialize)]
struct WorkspaceAgeJson {
    created_at: Option<String>,
    display: Option<String>,
}

#[derive(Serialize)]
struct WorkspaceDiffJson {
    status: &'static str,
    files_changed: Option<u32>,
    insertions: Option<u32>,
    deletions: Option<u32>,
    display: String,
}
