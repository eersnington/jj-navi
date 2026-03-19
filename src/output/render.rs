use pathdiff::diff_paths;
use serde::Serialize;

use crate::types::{
    WorkspaceListEntry, WorkspaceListStatus, WorkspacePathState, WorkspaceSnapshot,
};

use super::{
    INFERRED_ISSUE_URL, hyperlink, hyperlink_enabled, pad_visible, style_list_badge, style_meta,
    style_table_header, style_workspace_name, styled_stdout_text, theme,
};

use std::fmt::Write;
use std::path::Path;

const CURRENT_HEADER: &str = "cur";
const WORKSPACE_HEADER: &str = "workspace";
const STATUS_HEADER: &str = "status";
const PATH_HEADER: &str = "path";
const COMMIT_HEADER: &str = "commit";

/// Render a table of workspaces for `navi list`.
#[must_use]
pub fn render_workspace_table(entries: &[WorkspaceListEntry]) -> String {
    let rendered_entries = entries
        .iter()
        .map(|entry| RenderedWorkspaceEntry {
            is_current: entry.is_current,
            name: render_workspace_name(entry.name.as_str(), entry.is_current),
            status: render_workspace_status(entry),
            path: render_workspace_path(entry),
            commit_id: render_commit_id(entry.commit_id.as_str()),
            message: entry.message.as_str(),
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

    let mut output = String::new();
    writeln!(
        output,
        "{}  {}  {}  {}  {}  {}",
        style_table_header(&format!("{CURRENT_HEADER:<3}")),
        style_table_header(&format!("{WORKSPACE_HEADER:<workspace_width$}")),
        style_table_header(&format!("{STATUS_HEADER:<status_width$}")),
        style_table_header(&format!("{PATH_HEADER:<path_width$}")),
        style_table_header(&format!("{COMMIT_HEADER:<commit_width$}")),
        style_table_header("message"),
    )
    .expect("write table header");

    for entry in rendered_entries {
        let current_marker = if entry.is_current { "@" } else { "" };
        writeln!(
            output,
            "{}  {}  {}  {}  {}  {}",
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
            pad_visible(&entry.path.rendered, entry.path.visible_len, path_width),
            pad_visible(
                &entry.commit_id.rendered,
                entry.commit_id.visible_len,
                commit_width
            ),
            entry.message,
        )
        .expect("write table row");
    }

    output
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
    path: RenderedCell,
    commit_id: RenderedCell,
    message: &'a str,
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
