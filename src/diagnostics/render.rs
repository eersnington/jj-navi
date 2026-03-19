use std::fmt::Write;

use serde::Serialize;

use crate::diagnostics::{DoctorFinding, DoctorReport, DoctorScope, DoctorSeverity, DoctorSummary};
use crate::output::{
    style_detail_label, style_doctor_heading, style_section_label, style_status_badge,
};

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
