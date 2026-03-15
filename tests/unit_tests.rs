use jj_navi::doctor::{
    DoctorFinding, DoctorFindingCode, DoctorReport, DoctorScope, DoctorSeverity,
};
use jj_navi::output::{
    DIRECTIVE_FILE_ENV_VAR, MANAGED_BLOCK_END, MANAGED_BLOCK_START, render_doctor_report,
    render_doctor_report_json, render_shell_init, render_shell_install_block,
    render_workspace_table,
};
use jj_navi::types::{
    ShellKind, WorkspaceListEntry, WorkspaceListStatus, WorkspaceName, WorkspacePathState,
};
use std::path::PathBuf;

#[test]
fn rejects_invalid_workspace_names() {
    assert!(WorkspaceName::new("").is_err());
    assert!(WorkspaceName::new("feat/auth").is_err());
    assert!(WorkspaceName::new("feat auth").is_err());
}

#[test]
fn renders_table_with_header() {
    let entries = vec![WorkspaceListEntry {
        is_current: false,
        name: WorkspaceName::new("feature-auth").expect("valid workspace"),
        statuses: vec![WorkspaceListStatus::Inferred],
        path: PathBuf::from("../repo.feature-auth"),
        path_is_inferred: true,
        path_state: WorkspacePathState::Inferred,
        commit_id: String::from("abc123"),
        message: String::from("Feature auth work"),
    }];

    let rendered = render_workspace_table(&entries);

    assert!(rendered.starts_with("cur"));
    assert!(rendered.contains("status"));
    assert!(rendered.contains("../repo.feature-auth"));
    assert!(rendered.contains("[inferred]"));
    assert!(!rendered.contains("../repo.feature-auth [inferred]"));
    assert!(rendered.contains("abc123"));
    assert!(rendered.contains("Feature auth work"));
}

#[test]
fn renders_missing_status_without_inferred_status_for_non_inferred_path() {
    let entries = vec![WorkspaceListEntry {
        is_current: false,
        name: WorkspaceName::new("feature-auth").expect("valid workspace"),
        statuses: vec![WorkspaceListStatus::Missing],
        path: PathBuf::from("../repo.feature-auth"),
        path_is_inferred: false,
        path_state: WorkspacePathState::Missing,
        commit_id: String::from("abc123"),
        message: String::from("Feature auth work"),
    }];

    let rendered = render_workspace_table(&entries);

    assert!(rendered.contains("[missing]"));
    assert!(rendered.contains("../repo.feature-auth"));
    assert!(!rendered.contains("[inferred] [missing]"));
}

#[test]
fn renders_combined_workspace_statuses() {
    let entries = vec![WorkspaceListEntry {
        is_current: false,
        name: WorkspaceName::new("feature-auth").expect("valid workspace"),
        statuses: vec![WorkspaceListStatus::Inferred, WorkspaceListStatus::Missing],
        path: PathBuf::from("../repo.feature-auth"),
        path_is_inferred: true,
        path_state: WorkspacePathState::Missing,
        commit_id: String::from("abc123"),
        message: String::from("Feature auth work"),
    }];

    let rendered = render_workspace_table(&entries);

    assert!(rendered.contains("[inferred] [missing]"));
    assert!(!rendered.contains("../repo.feature-auth [missing]"));
}

#[test]
fn renders_zsh_shell_init() {
    let rendered = render_shell_init("navi", ShellKind::Zsh);

    assert!(rendered.contains("navi()"));
    assert!(rendered.contains(DIRECTIVE_FILE_ENV_VAR));
    assert!(rendered.contains("source \"$directive_file\""));
}

#[test]
fn renders_shell_install_block() {
    let rendered = render_shell_install_block("navi", ShellKind::Bash);

    assert!(rendered.contains(MANAGED_BLOCK_START));
    assert!(rendered.contains(MANAGED_BLOCK_END));
    assert!(rendered.contains("eval \"$(command navi config shell init bash)\""));
}

#[test]
fn renders_doctor_report() {
    let report = DoctorReport {
        findings: vec![DoctorFinding {
            severity: DoctorSeverity::Warning,
            code: DoctorFindingCode::WorkspaceDirectoryMissing,
            scope: DoctorScope::Workspace {
                workspace: String::from("feature-auth"),
            },
            message: String::from("workspace 'feature-auth' directory is missing"),
            path: Some(String::from("../repo.feature-auth")),
            hint: Some(String::from("last known path: ../repo.feature-auth")),
        }],
    };

    let rendered = render_doctor_report(&report);

    assert!(rendered.contains("Doctor [ warnings found ]"));
    assert!(rendered.contains("Summary 1 warning"));
    assert!(rendered.contains("Checks"));
    assert!(rendered.contains("! workspaces [ warning (1 finding) ]"));
    assert!(rendered.contains("Findings"));
    assert!(
        rendered.contains(
            "! [ warning ]  feature-auth - workspace 'feature-auth' directory is missing"
        )
    );
    assert!(rendered.contains("scope: workspace:feature-auth"));
    assert!(rendered.contains("path: ../repo.feature-auth"));
    assert!(rendered.contains("hint: last known path: ../repo.feature-auth"));
}

#[test]
fn renders_healthy_doctor_report_with_checks() {
    let report = DoctorReport::default();

    let rendered = render_doctor_report(&report);

    assert!(rendered.contains("Doctor [ healthy ]"));
    assert!(rendered.contains("Summary ok"));
    assert!(rendered.contains("o repo       [ ok ]"));
    assert!(rendered.contains("o workspaces [ ok ]"));
    assert!(rendered.contains("o shell      [ ok ]"));
    assert!(!rendered.contains("Findings"));
}

#[test]
fn renders_doctor_report_json() {
    let report = DoctorReport {
        findings: vec![DoctorFinding {
            severity: DoctorSeverity::Info,
            code: DoctorFindingCode::JjOnlyWorkspace,
            scope: DoctorScope::Workspace {
                workspace: String::from("feature-auth"),
            },
            message: String::from("workspace 'feature-auth' exists in jj but has no navi metadata"),
            path: None,
            hint: None,
        }],
    };

    let rendered = render_doctor_report_json(&report, false).expect("render doctor json");

    assert!(rendered.starts_with("{\n"));
    assert!(rendered.contains("\"warnings\": 0"));
    assert!(rendered.contains("\"code\": \"jj_only_workspace\""));
}
