use std::fmt::Write;

use crate::types::{ShellKind, WorkspaceListEntry};

pub const DIRECTIVE_FILE_ENV_VAR: &str = "NAVI_DIRECTIVE_FILE";

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

#[must_use]
pub fn render_shell_init(command_name: &str, shell: ShellKind) -> String {
    let source_cmd = match shell {
        ShellKind::Bash | ShellKind::Zsh => "source",
    };

    format!(
        "# jj-navi shell integration for {shell}\nif command -v {command_name} >/dev/null 2>&1; then\n    {command_name}() {{\n        local directive_file exit_code=0\n        directive_file=\"$(mktemp)\"\n        {directive_env}=\"$directive_file\" command {command_name} \"$@\" || exit_code=$?\n        if [[ -s \"$directive_file\" ]]; then\n            {source_cmd} \"$directive_file\"\n            if [[ $exit_code -eq 0 ]]; then\n                exit_code=$?\n            fi\n        fi\n        rm -f \"$directive_file\"\n        return \"$exit_code\"\n    }}\nfi\n",
        shell = shell.as_str(),
        command_name = command_name,
        directive_env = DIRECTIVE_FILE_ENV_VAR,
        source_cmd = source_cmd,
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::types::{ShellKind, WorkspaceListEntry, WorkspaceName};

    use super::{DIRECTIVE_FILE_ENV_VAR, render_shell_init, render_workspace_table};

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

    #[test]
    fn renders_bash_shell_init() {
        let rendered = render_shell_init("navi", ShellKind::Bash);

        assert!(rendered.contains("navi()"));
        assert!(rendered.contains(DIRECTIVE_FILE_ENV_VAR));
        assert!(rendered.contains("command navi \"$@\""));
    }
}
