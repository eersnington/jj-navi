//! Output rendering helpers for CLI-facing text and shell integration.

mod render;

use anstyle::{Ansi256Color, AnsiColor, Color, Effects, RgbColor, Style};
use clap::builder::styling::Styles;
use std::io::{IsTerminal, stderr, stdout};
use std::path::Path;
use std::sync::OnceLock;

use crate::repo::config_list;

pub use crate::shell::{
    DIRECTIVE_FILE_ENV_VAR, MANAGED_BLOCK_END, MANAGED_BLOCK_START, escape_shell_single_quotes,
    render_shell_init, render_shell_install_block, write_cd_directive,
};
#[doc(hidden)]
pub use render::render_workspace_table_with_width;
pub use render::{
    render_merge_preview, render_merge_preview_json, render_workspace_list_json,
    render_workspace_table,
};

const INFERRED_ISSUE_URL: &str = "https://github.com/eersnington/jj-navi/issues/36";
static OUTPUT_THEME: OnceLock<OutputTheme> = OnceLock::new();

#[derive(Clone, Copy)]
pub(super) struct OutputTheme {
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
    diff_files: Style,
    diff_insertions: Style,
    diff_deletions: Style,
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
            diff_files: style_from_candidates(&["\"navi.diff.files\"", "rest", "separator"]),
            diff_insertions: style_from_candidates(&[
                "\"navi.diff.insertions\"",
                "\"diff added token\"",
                "working_copies",
                "green",
            ]),
            diff_deletions: style_from_candidates(&[
                "\"navi.diff.deletions\"",
                "\"diff removed token\"",
                "conflict",
                "red",
            ]),
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

pub(super) fn theme() -> &'static OutputTheme {
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

pub(super) fn styled_prefix(prefix: &str, style: Style) -> String {
    apply_style(prefix, style, stderr_color_enabled())
}

pub(super) fn style_doctor_heading(label: &str) -> String {
    styled_stdout_text(label, theme().doctor_heading)
}

pub(super) fn style_table_header(label: &str) -> String {
    styled_stdout_text(label, theme().table_header)
}

pub(super) fn style_section_label(label: &str) -> String {
    styled_stdout_text(label, theme().section_label)
}

pub(super) fn style_detail_label(label: &str) -> String {
    styled_stdout_text(&format!("{label}:"), theme().detail_label)
}

pub(crate) fn style_status_badge(
    label: &str,
    severity: crate::diagnostics::DoctorSeverity,
) -> String {
    let plain = format!("[ {label} ]");
    let style = match severity {
        crate::diagnostics::DoctorSeverity::Error => theme().error_badge,
        crate::diagnostics::DoctorSeverity::Warning => theme().warning_badge,
        crate::diagnostics::DoctorSeverity::Info => theme().ok_badge,
    };
    styled_stdout_text(&plain, style)
}

pub(super) fn style_list_badge(label: &str, style: Style) -> String {
    styled_stdout_text(&format!("[ {label} ]"), style)
}

pub(super) fn style_workspace_name(name: &str, is_current: bool) -> String {
    if is_current {
        styled_stdout_text(name, theme().current_workspace)
    } else {
        name.to_owned()
    }
}

pub(super) fn style_meta(text: &str) -> String {
    styled_stdout_text(text, theme().meta)
}

pub(super) fn styled_stdout_text(text: &str, style: Style) -> String {
    apply_style(text, style, stdout_color_enabled())
}

fn apply_style(text: &str, style: Style, enabled: bool) -> String {
    if !enabled || style == Style::new() {
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

pub(super) fn hyperlink_enabled() -> bool {
    if !stdout().is_terminal() {
        return false;
    }

    !matches!(std::env::var("TERM"), Ok(term) if term == "dumb")
}

pub(super) fn hyperlink(label: &str, url: &str) -> String {
    format!("\u{1b}]8;;{url}\u{1b}\\{label}\u{1b}]8;;\u{1b}\\")
}

pub(super) fn pad_visible(value: &str, visible_len: usize, width: usize) -> String {
    let padding = width.saturating_sub(visible_len);
    format!("{value}{}", " ".repeat(padding))
}
