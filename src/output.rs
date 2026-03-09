use std::fmt::Write;

use crate::types::WorkspaceEntry;

#[must_use]
pub fn render_workspace_table(entries: &[WorkspaceEntry]) -> String {
    let width = entries
        .iter()
        .map(|entry| entry.name.as_str().len())
        .chain(std::iter::once("workspace".len()))
        .max()
        .unwrap_or("workspace".len());

    let mut output = String::new();
    writeln!(output, "{:<width$}  path", "workspace", width = width).expect("write table header");

    for entry in entries {
        writeln!(
            output,
            "{:<width$}  {}",
            entry.name,
            entry.path.display(),
            width = width
        )
        .expect("write table row");
    }

    output
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::types::{WorkspaceEntry, WorkspaceName};

    use super::render_workspace_table;

    #[test]
    fn renders_workspace_table() {
        let entries = vec![
            WorkspaceEntry {
                name: WorkspaceName::new("default").expect("valid workspace"),
                path: PathBuf::from("."),
            },
            WorkspaceEntry {
                name: WorkspaceName::new("feature-auth").expect("valid workspace"),
                path: PathBuf::from("../repo.feature-auth"),
            },
        ];

        let rendered = render_workspace_table(&entries);

        assert!(rendered.contains("workspace"));
        assert!(rendered.contains("feature-auth"));
    }
}
