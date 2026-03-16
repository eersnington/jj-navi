//! Output rendering helpers for CLI-facing text and shell integration.

use anstyle::{Ansi256Color, AnsiColor, Color, Effects, RgbColor, Style};
use clap::builder::styling::Styles;
use serde::Serialize;
use std::fmt::Write;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::io::{IsTerminal, stderr, stdout};
use std::path::Path;
use std::sync::OnceLock;

use crate::doctor::{DoctorFinding, DoctorReport, DoctorScope, DoctorSeverity, DoctorSummary};
use crate::repo::config_list;
use crate::types::{ShellKind, WorkspaceListEntry};

const CURRENT_HEADER: &str = "cur";
const WORKSPACE_HEADER: &str = "workspace";
const STATUS_HEADER: &str = "status";
const PATH_HEADER: &str = "path";
const COMMIT_HEADER: &str = "commit";
const INFERRED_ISSUE_URL: &str = "https://github.com/eersnington/jj-navi/issues/36";
static OUTPUT_THEME: OnceLock<OutputTheme> = OnceLock::new();

#[derive(Clone, Copy)]
struct OutputTheme {
    clap_header: Style,
    clap_literal: Style,
    clap_placeholder: Style,
    clap_error: Style,
    clap_valid: Style,
    clap_invalid: Style,
    error_prefix: Style,
    warning_prefix: Style,
    hint_prefix: Style,
    doctor_heading: Style,
    table_header: Style,
    section_label: Style,
    detail_label: Style,
    warning_badge: Style,
    error_badge: Style,
    ok_badge: Style,
    inferred_badge: Style,
    missing_badge: Style,
    stale_badge: Style,
    jj_only_badge: Style,
    current_workspace: Style,
    meta: Style,
    inferred_path: Style,
    missing_path: Style,
    stale_path: Style,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ParsedStyle {
    fg: Option<Color>,
    bg: Option<Color>,
    effects: Effects,
}

enum ColorValue {
    Default,
    Color(Color),
}

impl ColorValue {
    const fn into_option(self) -> Option<Color> {
        match self {
            Self::Default => None,
            Self::Color(color) => Some(color),
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum RawStyleValue {
    Color(String),
    Style(RawStyleTable),
}

#[derive(serde::Deserialize)]
struct RawStyleTable {
    fg: Option<String>,
    bg: Option<String>,
    bold: Option<bool>,
    dim: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    reverse: Option<bool>,
}

#[derive(serde::Deserialize)]
struct RawStyleWrapper {
    style: RawStyleValue,
}

/// Environment variable used by shell integration to pass a directive file.
pub const DIRECTIVE_FILE_ENV_VAR: &str = "NAVI_DIRECTIVE_FILE";
/// Marker for the start of the managed shell block.
pub const MANAGED_BLOCK_START: &str = "# >>> jj-navi shell init >>>";
/// Marker for the end of the managed shell block.
pub const MANAGED_BLOCK_END: &str = "# <<< jj-navi shell init <<<";

impl OutputTheme {
    fn load() -> Self {
        Self {
            clap_header: style_from_candidates(&["\"navi.header\"", "\"warning heading\""]),
            clap_literal: style_from_candidates(&["\"navi.literal\"", "working_copies"]),
            clap_placeholder: style_from_candidates(&["\"navi.placeholder\"", "working_copies"]),
            clap_error: style_from_candidates(&["\"navi.error\"", "\"error heading\""]),
            clap_valid: style_from_candidates(&["\"navi.valid\"", "working_copies"]),
            clap_invalid: style_from_candidates(&["\"navi.invalid\"", "\"error heading\""]),
            error_prefix: style_from_candidates(&["\"navi.error\"", "\"error heading\"", "error"]),
            warning_prefix: style_from_candidates(&[
                "\"navi.warning\"",
                "\"warning heading\"",
                "warning",
            ]),
            hint_prefix: style_from_candidates(&["\"navi.hint\"", "\"hint heading\"", "hint"]),
            doctor_heading: style_from_candidates(&["\"navi.header\"", "\"warning heading\""]),
            table_header: style_from_candidates(&["\"navi.section\"", "\"hint heading\""]),
            section_label: style_from_candidates(&["\"navi.section\"", "\"hint heading\""]),
            detail_label: style_from_candidates(&["\"navi.detail\"", "hint"]),
            warning_badge: style_from_candidates(&["\"navi.warning\"", "warning"]),
            error_badge: style_from_candidates(&["\"navi.error\"", "error"]),
            ok_badge: style_from_candidates(&["\"navi.status.ok\"", "working_copies"]),
            inferred_badge: style_from_candidates(&["\"navi.status.inferred\"", "bookmarks"]),
            missing_badge: style_from_candidates(&[
                "\"navi.status.missing\"",
                "\"description placeholder\"",
            ]),
            stale_badge: style_from_candidates(&["\"navi.status.stale\"", "conflict"]),
            jj_only_badge: style_from_candidates(&["\"navi.status.jj_only\"", "tag"]),
            current_workspace: style_from_candidates(&[
                "\"navi.current\"",
                "working_copy",
                "\"working_copy bookmarks\"",
            ]),
            meta: style_from_candidates(&["\"navi.meta\"", "rest", "separator"]),
            inferred_path: style_from_candidates(&["\"navi.path.inferred\"", "bookmark"]),
            missing_path: style_from_candidates(&[
                "\"navi.path.missing\"",
                "\"description placeholder\"",
            ]),
            stale_path: style_from_candidates(&["\"navi.path.stale\"", "conflict"]),
        }
    }
}

impl ParsedStyle {
    fn into_style(self) -> Style {
        Style::new()
            .fg_color(self.fg)
            .bg_color(self.bg)
            .effects(self.effects)
    }
}

fn theme() -> &'static OutputTheme {
    OUTPUT_THEME.get_or_init(OutputTheme::load)
}

fn style_from_candidates(candidates: &[&str]) -> Style {
    let styles = load_color_styles();
    candidates
        .iter()
        .find_map(|name| styles.get(*name).copied())
        .unwrap_or_default()
        .into_style()
}

fn load_color_styles() -> &'static std::collections::BTreeMap<String, ParsedStyle> {
    static COLOR_STYLES: OnceLock<std::collections::BTreeMap<String, ParsedStyle>> =
        OnceLock::new();

    COLOR_STYLES.get_or_init(|| {
        parse_color_styles(&config_list(Path::new("."), "colors").unwrap_or_default())
    })
}

fn parse_color_styles(config: &str) -> std::collections::BTreeMap<String, ParsedStyle> {
    let mut styles = std::collections::BTreeMap::new();

    for line in config.lines().filter(|line| !line.trim().is_empty()) {
        let Some((name, value)) = line.split_once(" = ") else {
            continue;
        };
        let Some(stripped_name) = name.strip_prefix("colors.") else {
            continue;
        };
        let (label, field) = split_color_key(stripped_name);
        let entry = styles.entry(label.to_owned()).or_default();
        apply_config_field(entry, field, value);
    }

    styles
}

fn split_color_key(key: &str) -> (&str, Option<&str>) {
    if !key.starts_with('"') {
        return key
            .rsplit_once('.')
            .map_or((key, None), |(label, field)| (label, Some(field)));
    }

    let Some(rest) = key.strip_prefix('"') else {
        return (key, None);
    };
    let Some(quote_end) = rest.find('"') else {
        return (key, None);
    };
    let label_end = quote_end + 2;
    let label = &key[..label_end];
    let remainder = &key[label_end..];

    if let Some(field) = remainder.strip_prefix('.') {
        (label, Some(field))
    } else {
        (label, None)
    }
}

fn apply_config_field(style: &mut ParsedStyle, field: Option<&str>, value: &str) {
    match field {
        None => {
            if let Some(parsed) = parse_style(value) {
                *style = parsed;
            }
        }
        Some("fg") => {
            style.fg = parse_toml_string(value)
                .and_then(|color| parse_color_value(&color))
                .and_then(ColorValue::into_option);
        }
        Some("bg") => {
            style.bg = parse_toml_string(value)
                .and_then(|color| parse_color_value(&color))
                .and_then(ColorValue::into_option);
        }
        Some("bold") => set_effect(style, Effects::BOLD, parse_toml_bool(value)),
        Some("dim") => set_effect(style, Effects::DIMMED, parse_toml_bool(value)),
        Some("italic") => set_effect(style, Effects::ITALIC, parse_toml_bool(value)),
        Some("underline") => set_effect(style, Effects::UNDERLINE, parse_toml_bool(value)),
        Some("reverse") => set_effect(style, Effects::INVERT, parse_toml_bool(value)),
        Some(_) => {}
    }
}

fn set_effect(style: &mut ParsedStyle, effect: Effects, enabled: Option<bool>) {
    if let Some(enabled) = enabled {
        style.effects = style.effects.set(effect, enabled);
    }
}

fn parse_toml_string(raw: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct StringWrapper {
        value: String,
    }

    toml::from_str::<StringWrapper>(&format!("value = {raw}"))
        .ok()
        .map(|value| value.value)
}

fn parse_toml_bool(raw: &str) -> Option<bool> {
    match raw.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_style(raw: &str) -> Option<ParsedStyle> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }

    let parsed = toml::from_str::<RawStyleWrapper>(&format!("style = {value}"))
        .or_else(|_| toml::from_str::<RawStyleWrapper>(&format!("style = {value:?}")))
        .ok()?;

    match parsed.style {
        RawStyleValue::Color(color) => Some(ParsedStyle {
            fg: parse_color_value(&color).and_then(ColorValue::into_option),
            ..ParsedStyle::default()
        }),
        RawStyleValue::Style(style) => Some(parse_style_table(&style)),
    }
}

fn parse_style_table(style: &RawStyleTable) -> ParsedStyle {
    let mut effects = Effects::new();
    if style.bold.unwrap_or(false) {
        effects |= Effects::BOLD;
    }
    if style.dim.unwrap_or(false) {
        effects |= Effects::DIMMED;
    }
    if style.italic.unwrap_or(false) {
        effects |= Effects::ITALIC;
    }
    if style.underline.unwrap_or(false) {
        effects |= Effects::UNDERLINE;
    }
    if style.reverse.unwrap_or(false) {
        effects |= Effects::INVERT;
    }

    ParsedStyle {
        fg: style.fg.as_deref().and_then(parse_color),
        bg: style.bg.as_deref().and_then(parse_color),
        effects,
    }
}

fn parse_color_value(value: &str) -> Option<ColorValue> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized == "default" {
        return Some(ColorValue::Default);
    }

    parse_color(&normalized).map(ColorValue::Color)
}

fn parse_color(value: &str) -> Option<Color> {
    let normalized = value.trim().to_ascii_lowercase();
    let color = match normalized.as_str() {
        "black" => Color::Ansi(AnsiColor::Black),
        "red" => Color::Ansi(AnsiColor::Red),
        "green" => Color::Ansi(AnsiColor::Green),
        "yellow" => Color::Ansi(AnsiColor::Yellow),
        "blue" => Color::Ansi(AnsiColor::Blue),
        "magenta" => Color::Ansi(AnsiColor::Magenta),
        "cyan" => Color::Ansi(AnsiColor::Cyan),
        "white" => Color::Ansi(AnsiColor::White),
        "bright black" => Color::Ansi(AnsiColor::BrightBlack),
        "bright red" => Color::Ansi(AnsiColor::BrightRed),
        "bright green" => Color::Ansi(AnsiColor::BrightGreen),
        "bright yellow" => Color::Ansi(AnsiColor::BrightYellow),
        "bright blue" => Color::Ansi(AnsiColor::BrightBlue),
        "bright magenta" => Color::Ansi(AnsiColor::BrightMagenta),
        "bright cyan" => Color::Ansi(AnsiColor::BrightCyan),
        "bright white" => Color::Ansi(AnsiColor::BrightWhite),
        _ if normalized.starts_with("ansi-color-") => {
            let code = normalized
                .trim_start_matches("ansi-color-")
                .parse::<u8>()
                .ok()?;
            Color::Ansi256(Ansi256Color(code))
        }
        _ if normalized.starts_with('#') && normalized.len() == 7 => {
            let red = u8::from_str_radix(&normalized[1..3], 16).ok()?;
            let green = u8::from_str_radix(&normalized[3..5], 16).ok()?;
            let blue = u8::from_str_radix(&normalized[5..7], 16).ok()?;
            Color::Rgb(RgbColor(red, green, blue))
        }
        _ => return None,
    };

    Some(color)
}

/// Clap styles for restrained help and parser output.
#[must_use]
pub fn clap_styles() -> Styles {
    Styles::styled()
        .header(theme().clap_header)
        .usage(theme().clap_header)
        .literal(theme().clap_literal)
        .placeholder(theme().clap_placeholder)
        .error(theme().clap_error)
        .valid(theme().clap_valid)
        .invalid(theme().clap_invalid)
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
        return format!("{}{}", styled_prefix("error:", theme().error_prefix), rest);
    }

    if let Some(rest) = line.strip_prefix("warning:") {
        return format!(
            "{}{}",
            styled_prefix("warning:", theme().warning_prefix),
            rest
        );
    }

    if let Some(rest) = line.strip_prefix("hint:") {
        return format!("{}{}", styled_prefix("hint:", theme().hint_prefix), rest);
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

fn styled_prefix(prefix: &str, style: Style) -> String {
    apply_style(prefix, style, stderr_color_enabled())
}

fn style_doctor_heading(label: &str) -> String {
    styled_stdout_text(label, theme().doctor_heading)
}

fn style_table_header(label: &str) -> String {
    styled_stdout_text(label, theme().table_header)
}

fn style_section_label(label: &str) -> String {
    styled_stdout_text(label, theme().section_label)
}

fn style_detail_label(label: &str) -> String {
    styled_stdout_text(&format!("{label}:"), theme().detail_label)
}

fn style_status_badge(label: &str, severity: DoctorSeverity) -> String {
    let plain = format!("[ {label} ]");
    let style = match severity {
        DoctorSeverity::Error => theme().error_badge,
        DoctorSeverity::Warning => theme().warning_badge,
        DoctorSeverity::Info => theme().ok_badge,
    };
    styled_stdout_text(&plain, style)
}

fn style_list_badge(label: &str, style: Style) -> String {
    styled_stdout_text(&format!("[ {label} ]"), style)
}

fn style_workspace_name(name: &str, is_current: bool) -> String {
    if is_current {
        styled_stdout_text(name, theme().current_workspace)
    } else {
        name.to_owned()
    }
}

fn style_meta(text: &str) -> String {
    styled_stdout_text(text, theme().meta)
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

fn styled_stdout_text(text: &str, style: Style) -> String {
    apply_style(text, style, stdout_color_enabled())
}

fn apply_style(text: &str, style: Style, enabled: bool) -> String {
    if !enabled {
        return text.to_owned();
    }

    if style == Style::new() {
        return text.to_owned();
    }

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
        crate::types::WorkspacePathState::Confirmed => plain.clone(),
        crate::types::WorkspacePathState::Inferred => {
            styled_stdout_text(&plain, theme().inferred_path)
        }
        crate::types::WorkspacePathState::Missing => {
            styled_stdout_text(&plain, theme().missing_path)
        }
        crate::types::WorkspacePathState::Stale => styled_stdout_text(&plain, theme().stale_path),
    };
    let visible_len = plain.len();

    RenderedCell {
        rendered,
        visible_len,
    }
}

fn render_workspace_name(name: &str, is_current: bool) -> RenderedCell {
    RenderedCell {
        rendered: style_workspace_name(name, is_current),
        visible_len: name.len(),
    }
}

fn render_commit_id(commit_id: &str) -> RenderedCell {
    let visible_len = commit_id.len();

    RenderedCell {
        rendered: style_meta(commit_id),
        visible_len,
    }
}

fn render_status_badge(status: crate::types::WorkspaceListStatus) -> String {
    let label = status.label();
    let colored = match status {
        crate::types::WorkspaceListStatus::Ok => style_list_badge(label, theme().ok_badge),
        crate::types::WorkspaceListStatus::Inferred => {
            style_list_badge(label, theme().inferred_badge)
        }
        crate::types::WorkspaceListStatus::Missing => {
            style_list_badge(label, theme().missing_badge)
        }
        crate::types::WorkspaceListStatus::Stale => style_list_badge(label, theme().stale_badge),
        crate::types::WorkspaceListStatus::JjOnly => style_list_badge(label, theme().jj_only_badge),
    };

    if status == crate::types::WorkspaceListStatus::Inferred && hyperlink_enabled() {
        hyperlink(&colored, INFERRED_ISSUE_URL)
    } else {
        colored
    }
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
