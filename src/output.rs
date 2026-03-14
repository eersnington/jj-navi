//! Output rendering helpers for CLI-facing text and shell integration.

use anstyle::{Ansi256Color, Effects, Style};
use clap::builder::styling::Styles;
use serde::Serialize;
use std::fmt::Write;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::io::{IsTerminal, stderr, stdout};
use std::path::Path;

use crate::doctor::{DoctorFinding, DoctorReport, DoctorScope, DoctorSeverity, DoctorSummary};
use crate::types::{ShellKind, WorkspaceListEntry, WorkspacePathState};

const WORKSPACE_HEADER: &str = "workspace";
const PATH_HEADER: &str = "path";
const COMMIT_HEADER: &str = "commit";
const SOFT_YELLOW: Ansi256Color = Ansi256Color(179);
const SOFT_GREEN: Ansi256Color = Ansi256Color(108);
const INFERRED_ISSUE_URL: &str = "https://github.com/eersnington/jj-navi/issues/36";

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
            path: render_workspace_path(entry),
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
        .map(|entry| entry.path.visible_len)
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
            "{:<6}  {:<workspace_width$}  {}  {:<commit_width$}  {}",
            if entry.is_current { "@" } else { "" },
            entry.name,
            pad_visible(&entry.path.rendered, entry.path.visible_len, path_width),
            entry.commit_id,
            entry.message,
            workspace_width = workspace_width,
            commit_width = commit_width
        )
        .expect("write table row");
    }

    output
}

/// Render a doctor report as human-facing text.
#[must_use]
pub fn render_doctor_report(report: &DoctorReport) -> String {
    let summary = report.summary();
    let mut output = String::new();

    writeln!(
        output,
        "{} {}",
        style_doctor_heading("Doctor"),
        style_status_badge(
            render_doctor_headline(summary),
            doctor_headline_severity(summary)
        )
    )
    .expect("write doctor headline");
    writeln!(
        output,
        "{} {}",
        style_section_label("Summary"),
        render_doctor_summary(summary)
    )
    .expect("write doctor summary");
    writeln!(output).expect("write doctor spacing");
    writeln!(output, "{}", style_section_label("Checks")).expect("write doctor checks heading");

    for scope_summary in scope_summaries(report) {
        writeln!(
            output,
            "  {} {}",
            scope_icon(&scope_summary),
            render_scope_line(&scope_summary)
        )
        .expect("write doctor scope summary");
    }

    if !report.is_empty() {
        writeln!(output).expect("write doctor spacing");
        writeln!(output, "{}", style_section_label("Findings"))
            .expect("write doctor findings heading");

        for finding in &report.findings {
            writeln!(
                output,
                "  {} {}  {}",
                finding_icon(finding.severity),
                style_status_badge(finding.severity.label(), finding.severity),
                render_finding_title(&finding.scope, &finding.message)
            )
            .expect("write doctor finding");
            writeln!(
                output,
                "      {} {}",
                style_detail_label("scope"),
                render_finding_scope(&finding.scope)
            )
            .expect("write doctor scope detail");
            if let Some(path) = &finding.path {
                writeln!(output, "      {} {}", style_detail_label("path"), path)
                    .expect("write doctor path");
            }
            if let Some(hint) = &finding.hint {
                writeln!(output, "      {} {}", style_detail_label("hint"), hint)
                    .expect("write doctor hint");
            }
        }
    }

    output
}

/// Render a doctor report as JSON.
///
/// # Errors
///
/// Returns an error if the report cannot be serialized.
pub fn render_doctor_report_json(report: &DoctorReport, compact: bool) -> crate::Result<String> {
    let payload = DoctorJsonOutput {
        summary: report.summary(),
        findings: &report.findings,
    };
    let rendered = if compact {
        serde_json::to_string(&payload)
    } else {
        serde_json::to_string_pretty(&payload)
    }
    .map_err(|error| crate::Error::JsonSerialization(error.to_string()))?;

    Ok(rendered)
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

fn render_doctor_summary(summary: DoctorSummary) -> String {
    let mut parts = Vec::new();
    if summary.errors > 0 {
        parts.push(pluralize(summary.errors, "error"));
    }
    if summary.warnings > 0 {
        parts.push(pluralize(summary.warnings, "warning"));
    }
    if summary.info > 0 {
        parts.push(pluralize(summary.info, "info"));
    }

    if parts.is_empty() {
        String::from("ok")
    } else {
        parts.join(", ")
    }
}

fn render_doctor_headline(summary: DoctorSummary) -> &'static str {
    if summary.errors > 0 {
        "attention needed"
    } else if summary.warnings > 0 {
        "warnings found"
    } else {
        "healthy"
    }
}

fn doctor_headline_severity(summary: DoctorSummary) -> DoctorSeverity {
    if summary.errors > 0 {
        DoctorSeverity::Error
    } else if summary.warnings > 0 {
        DoctorSeverity::Warning
    } else {
        DoctorSeverity::Info
    }
}

fn scope_summaries(report: &DoctorReport) -> [ScopeSummary; 3] {
    let mut repo = ScopeSummary::new("repo");
    let mut workspaces = ScopeSummary::new("workspaces");
    let mut shell = ScopeSummary::new("shell");

    for finding in &report.findings {
        match &finding.scope {
            DoctorScope::Repo => repo.record(finding.severity),
            DoctorScope::Workspace { .. } => workspaces.record(finding.severity),
            DoctorScope::Shell => shell.record(finding.severity),
        }
    }

    [repo, workspaces, shell]
}

fn render_scope_status(summary: &ScopeSummary) -> String {
    match summary.worst {
        None => String::from("ok"),
        Some(severity) => format!(
            "{} ({})",
            severity.label(),
            pluralize(summary.count, "finding")
        ),
    }
}

fn render_scope_line(summary: &ScopeSummary) -> String {
    format!(
        "{:<10} {}",
        summary.label,
        style_status_badge(&render_scope_status(summary), scope_severity(summary))
    )
}

fn scope_severity(summary: &ScopeSummary) -> DoctorSeverity {
    summary.worst.unwrap_or(DoctorSeverity::Info)
}

fn render_finding_scope(scope: &DoctorScope) -> String {
    match scope {
        DoctorScope::Repo => String::from("repo"),
        DoctorScope::Workspace { workspace } => format!("workspace:{workspace}"),
        DoctorScope::Shell => String::from("shell"),
    }
}

fn render_finding_title(scope: &DoctorScope, message: &str) -> String {
    match scope {
        DoctorScope::Repo | DoctorScope::Shell => message.to_owned(),
        DoctorScope::Workspace { workspace } => format!("{workspace} - {message}"),
    }
}

fn pluralize(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

fn styled_prefix(prefix: &str, color: Ansi256Color) -> String {
    if !stderr_color_enabled() {
        return prefix.to_owned();
    }

    format!(
        "\u{1b}[38;5;{}m{}\u{1b}[0m",
        ansi_256_color_code(color),
        prefix
    )
}

fn style_doctor_heading(label: &str) -> String {
    styled_stdout_text(label, SOFT_YELLOW, true)
}

fn style_section_label(label: &str) -> String {
    styled_stdout_text(label, SOFT_GREEN, true)
}

fn style_detail_label(label: &str) -> String {
    styled_stdout_text(&format!("{label}:"), SOFT_GREEN, false)
}

fn style_status_badge(label: &str, severity: DoctorSeverity) -> String {
    let plain = format!("[ {label} ]");
    let color = match severity {
        DoctorSeverity::Error | DoctorSeverity::Warning => SOFT_YELLOW,
        DoctorSeverity::Info => SOFT_GREEN,
    };
    styled_stdout_text(&plain, color, true)
}

fn scope_icon(summary: &ScopeSummary) -> &'static str {
    finding_icon(scope_severity(summary))
}

fn finding_icon(severity: DoctorSeverity) -> &'static str {
    match severity {
        DoctorSeverity::Error => "x",
        DoctorSeverity::Warning => "!",
        DoctorSeverity::Info => "o",
    }
}

fn styled_stdout_text(text: &str, color: Ansi256Color, bold: bool) -> String {
    if !stdout_color_enabled() {
        return text.to_owned();
    }

    let style = if bold {
        Style::new()
            .fg_color(Some(color.into()))
            .effects(Effects::BOLD)
    } else {
        Style::new().fg_color(Some(color.into()))
    };

    format!("{}{}{}", style.render(), text, style.render_reset())
}

fn stdout_color_enabled() -> bool {
    should_color_output(stdout().is_terminal())
}

fn stderr_color_enabled() -> bool {
    should_color_output(stderr().is_terminal())
}

fn should_color_output(stream_is_terminal: bool) -> bool {
    std::env::var_os("NO_COLOR").is_none() && stream_is_terminal
}

fn render_workspace_path(entry: &WorkspaceListEntry) -> RenderedPath {
    let plain = plain_workspace_path(entry);
    let mut rendered = plain.clone();

    if hyperlink_enabled() {
        rendered = rendered.replacen(
            "[inferred]",
            &hyperlink("[inferred]", INFERRED_ISSUE_URL),
            1,
        );
    }

    RenderedPath {
        rendered,
        visible_len: plain.len(),
    }
}

fn plain_workspace_path(entry: &WorkspaceListEntry) -> String {
    let mut rendered = entry.path.display().to_string();

    match entry.path_state {
        WorkspacePathState::Confirmed => {}
        WorkspacePathState::Inferred => rendered.push_str(" [inferred]"),
        WorkspacePathState::Missing => {
            if entry.path_is_inferred {
                rendered.push_str(" [inferred]");
            }
            rendered.push_str(" [missing]");
        }
        WorkspacePathState::Stale => {
            if entry.path_is_inferred {
                rendered.push_str(" [inferred]");
            }
            rendered.push_str(" [stale]");
        }
    }

    rendered
}

fn hyperlink_enabled() -> bool {
    if !stdout().is_terminal() {
        return false;
    }

    !matches!(std::env::var("TERM"), Ok(term) if term == "dumb")
}

fn hyperlink(label: &str, url: &str) -> String {
    format!("\u{1b}]8;;{url}\u{1b}\\{label}\u{1b}]8;;\u{1b}\\")
}

fn pad_visible(value: &str, visible_len: usize, width: usize) -> String {
    let padding = width.saturating_sub(visible_len);
    format!("{value}{}", " ".repeat(padding))
}

const fn ansi_256_color_code(color: Ansi256Color) -> u8 {
    color.0
}

struct RenderedWorkspaceEntry<'a> {
    is_current: bool,
    name: &'a str,
    path: RenderedPath,
    commit_id: &'a str,
    message: &'a str,
}

struct RenderedPath {
    rendered: String,
    visible_len: usize,
}

#[derive(Serialize)]
struct DoctorJsonOutput<'a> {
    summary: DoctorSummary,
    findings: &'a [DoctorFinding],
}

struct ScopeSummary {
    label: &'static str,
    worst: Option<DoctorSeverity>,
    count: usize,
}

impl ScopeSummary {
    const fn new(label: &'static str) -> Self {
        Self {
            label,
            worst: None,
            count: 0,
        }
    }

    fn record(&mut self, severity: DoctorSeverity) {
        self.count += 1;
        self.worst = Some(match self.worst {
            None => severity,
            Some(current) => current.min(severity),
        });
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::types::{ShellKind, WorkspaceListEntry, WorkspaceName, WorkspacePathState};

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
                path_is_inferred: false,
                path_state: WorkspacePathState::Confirmed,
                commit_id: String::from("abc123"),
                message: String::from("Current work"),
            },
            WorkspaceListEntry {
                is_current: false,
                name: WorkspaceName::new("feature-auth").expect("valid workspace"),
                path: PathBuf::from("../repo.feature-auth"),
                path_is_inferred: true,
                path_state: WorkspacePathState::Inferred,
                commit_id: String::from("def456"),
                message: String::from("Feature auth work"),
            },
        ];

        let rendered = render_workspace_table(&entries);

        assert!(rendered.contains("marker"));
        assert!(rendered.contains("workspace"));
        assert!(rendered.contains("commit"));
        assert!(rendered.contains("Feature auth work"));
        assert!(rendered.contains("[inferred]"));
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
