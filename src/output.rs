//! Output rendering helpers for CLI-facing text and shell integration.

use anstyle::{Ansi256Color, Effects, Style};
use clap::builder::styling::Styles;
use std::fmt::Write;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::io::{IsTerminal, stderr};
use std::path::Path;

use crate::types::{ShellKind, WorkspaceListEntry};

const WORKSPACE_HEADER: &str = "workspace";
const PATH_HEADER: &str = "path";
const COMMIT_HEADER: &str = "commit";
const SOFT_YELLOW: Ansi256Color = Ansi256Color(179);
const SOFT_GREEN: Ansi256Color = Ansi256Color(108);

/// Environment variable used by shell integration to pass a directive file.
pub const DIRECTIVE_FILE_ENV_VAR: &str = "NAVI_DIRECTIVE_FILE";
/// Marker for the start of the managed shell block.
pub const MANAGED_BLOCK_START: &str = "# >>> jj-navi shell init >>>";
/// Marker for the end of the managed shell block.
pub const MANAGED_BLOCK_END: &str = "# <<< jj-navi shell init <<<";

/// Clap styles for restrained help and parser output.
#[must_use]
pub fn clap_styles() -> Styles {
    Styles::styled()
        .header(
            Style::new()
                .fg_color(Some(SOFT_YELLOW.into()))
                .effects(Effects::BOLD),
        )
        .usage(
            Style::new()
                .fg_color(Some(SOFT_YELLOW.into()))
                .effects(Effects::BOLD),
        )
        .literal(Style::new().fg_color(Some(SOFT_GREEN.into())))
        .placeholder(Style::new().fg_color(Some(SOFT_GREEN.into())))
        .error(
            Style::new()
                .fg_color(Some(SOFT_YELLOW.into()))
                .effects(Effects::BOLD),
        )
        .valid(Style::new().fg_color(Some(SOFT_GREEN.into())))
        .invalid(
            Style::new()
                .fg_color(Some(SOFT_YELLOW.into()))
                .effects(Effects::BOLD),
        )
}

/// Render a human-facing error message with restrained semantic colors.
#[must_use]
pub fn render_error_message(message: &str) -> String {
    message
        .lines()
        .map(colorize_error_line)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render a table of workspaces for `navi list`.
#[must_use]
pub fn render_workspace_table(entries: &[WorkspaceListEntry]) -> String {
    let rendered_entries = entries
        .iter()
        .map(|entry| RenderedWorkspaceEntry {
            is_current: entry.is_current,
            name: entry.name.as_str(),
            path: entry.path.display().to_string(),
            commit_id: entry.commit_id.as_str(),
            message: entry.message.as_str(),
        })
        .collect::<Vec<_>>();

    let workspace_width = rendered_entries
        .iter()
        .map(|entry| entry.name.len())
        .fold(WORKSPACE_HEADER.len(), usize::max);
    let path_width = rendered_entries
        .iter()
        .map(|entry| entry.path.len())
        .fold(PATH_HEADER.len(), usize::max);
    let commit_width = rendered_entries
        .iter()
        .map(|entry| entry.commit_id.len())
        .fold(COMMIT_HEADER.len(), usize::max);

    let mut output = String::new();
    writeln!(
        output,
        "marker  {WORKSPACE_HEADER:<workspace_width$}  {PATH_HEADER:<path_width$}  {COMMIT_HEADER:<commit_width$}  message"
    )
    .expect("write table header");

    for entry in rendered_entries {
        writeln!(
            output,
            "{:<6}  {:<workspace_width$}  {:<path_width$}  {:<commit_width$}  {}",
            if entry.is_current { "@" } else { "" },
            entry.name,
            entry.path,
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

/// Render shell initialization code for the chosen shell.
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

/// Render the managed shell block inserted into a shell rc file.
#[must_use]
pub fn render_shell_install_block(command_name: &str, shell: ShellKind) -> String {
    format!(
        "{MANAGED_BLOCK_START}\neval \"$(command {command_name} config shell init {shell})\"\n{MANAGED_BLOCK_END}\n",
        command_name = command_name,
        shell = shell.as_str(),
    )
}

/// Write a shell-safe `cd` directive if shell integration is active.
///
/// Returns `true` if a directive was written.
///
/// # Errors
///
/// Returns an error if the directive file path is invalid or writing fails.
pub fn write_cd_directive(path: &Path) -> crate::Result<bool> {
    let Ok(directive_file) = std::env::var(DIRECTIVE_FILE_ENV_VAR) else {
        return Ok(false);
    };

    if directive_file.trim().is_empty() {
        return Ok(false);
    }

    let escaped_path = escape_shell_single_quotes(
        path.to_str()
            .ok_or(crate::Error::ShellDirectivePathNotUtf8)?,
    );
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(directive_file)?;
    writeln!(file, "cd -- '{escaped_path}'")?;
    Ok(true)
}

/// Escape single quotes for POSIX shell single-quoted strings.
#[must_use]
pub fn escape_shell_single_quotes(value: &str) -> String {
    value.replace('\'', "'\\''")
}

fn colorize_error_line(line: &str) -> String {
    if let Some(rest) = line.strip_prefix("error:") {
        return format!("{}{}", styled_prefix("error:", SOFT_YELLOW), rest);
    }

    if let Some(rest) = line.strip_prefix("warning:") {
        return format!("{}{}", styled_prefix("warning:", SOFT_YELLOW), rest);
    }

    if let Some(rest) = line.strip_prefix("hint:") {
        return format!("{}{}", styled_prefix("hint:", SOFT_GREEN), rest);
    }

    line.to_owned()
}

fn styled_prefix(prefix: &str, color: Ansi256Color) -> String {
    if !color_enabled() {
        return prefix.to_owned();
    }

    format!(
        "\u{1b}[38;5;{}m{}\u{1b}[0m",
        ansi_256_color_code(color),
        prefix
    )
}

fn color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && stderr().is_terminal()
}

const fn ansi_256_color_code(color: Ansi256Color) -> u8 {
    color.0
}

struct RenderedWorkspaceEntry<'a> {
    is_current: bool,
    name: &'a str,
    path: String,
    commit_id: &'a str,
    message: &'a str,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::types::{ShellKind, WorkspaceListEntry, WorkspaceName};

    use super::{
        DIRECTIVE_FILE_ENV_VAR, MANAGED_BLOCK_END, MANAGED_BLOCK_START, escape_shell_single_quotes,
        render_error_message, render_shell_init, render_shell_install_block,
        render_workspace_table,
    };

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

    #[test]
    fn renders_shell_install_block() {
        let rendered = render_shell_install_block("navi", ShellKind::Zsh);

        assert!(rendered.contains(MANAGED_BLOCK_START));
        assert!(rendered.contains("eval \"$(command navi config shell init zsh)\""));
        assert!(rendered.contains(MANAGED_BLOCK_END));
    }

    #[test]
    fn escapes_single_quotes_for_shell_directives() {
        assert_eq!(
            escape_shell_single_quotes("../space dir/feature-auth's"),
            "../space dir/feature-auth'\\''s"
        );
    }

    #[test]
    fn renders_error_message_without_losing_prefixes() {
        let rendered = render_error_message("error: bad\nhint: try again");

        assert!(rendered.contains("error:"));
        assert!(rendered.contains("hint:"));
    }
}
