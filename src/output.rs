use std::fmt::Write;

use crate::types::WorkspaceListEntry;

#[must_use]
pub fn render_workspace_table(entries: &[WorkspaceListEntry]) -> String {
    let workspace_width = entries
        .iter()
        .map(|entry| entry.name.as_str().len())
        .chain(std::iter::once("workspace".len()))
        .max()
        .unwrap_or("workspace".len());
    let path_width = entries
        .iter()
        .map(|entry| entry.path.display().to_string().len())
        .chain(std::iter::once("path".len()))
        .max()
        .unwrap_or("path".len());
    let commit_width = entries
        .iter()
        .map(|entry| entry.commit_id.len())
        .chain(std::iter::once("commit".len()))
        .max()
        .unwrap_or("commit".len());

    let mut output = String::new();
    writeln!(
        output,
        "marker  {:<workspace_width$}  {:<path_width$}  {:<commit_width$}  message",
        "workspace",
        "path",
        "commit",
        workspace_width = workspace_width,
        path_width = path_width,
        commit_width = commit_width
    )
    .expect("write table header");

    for entry in entries {
        writeln!(
            output,
            "{:<6}  {:<workspace_width$}  {:<path_width$}  {:<commit_width$}  {}",
            if entry.is_current { "@" } else { "" },
            entry.name,
            entry.path.display(),
            entry.commit_id,
            entry.message,
            workspace_width = workspace_width,
            path_width = path_width,
            commit_width = commit_width
        )
        .expect("write table row");
    }

    output
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::types::{WorkspaceListEntry, WorkspaceName};

    use super::render_workspace_table;

    #[test]
    fn renders_workspace_table() {
        let entries = vec![
            WorkspaceListEntry {
                is_current: true,
                name: WorkspaceName::new("default").expect("valid workspace"),
                path: PathBuf::from("."),
                commit_id: String::from("abc123"),
                message: String::from("Current work"),
            },
            WorkspaceListEntry {
                is_current: false,
                name: WorkspaceName::new("feature-auth").expect("valid workspace"),
                path: PathBuf::from("../repo.feature-auth"),
                commit_id: String::from("def456"),
                message: String::from("Feature auth work"),
            },
        ];

        let rendered = render_workspace_table(&entries);

        assert!(rendered.contains("marker"));
        assert!(rendered.contains("workspace"));
        assert!(rendered.contains("commit"));
        assert!(rendered.contains("Feature auth work"));
    }
}
