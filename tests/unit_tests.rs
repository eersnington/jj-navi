use jj_navi::diagnostics::{
    DoctorFinding, DoctorFindingCode, DoctorReport, DoctorScope, DoctorSeverity,
    render_doctor_report, render_doctor_report_json,
};
use jj_navi::output::{
    DIRECTIVE_FILE_ENV_VAR, MANAGED_BLOCK_END, MANAGED_BLOCK_START, escape_shell_single_quotes,
    render_error_message, render_shell_init, render_shell_install_block, render_workspace_table,
    render_workspace_table_with_width,
};
use jj_navi::types::{
    ShellKind, WorkspaceAgeSnapshot, WorkspaceDiffSnapshot, WorkspaceFreshnessSnapshot,
    WorkspaceListEntry, WorkspaceListStatus, WorkspaceName, WorkspacePathState, WorkspaceTemplate,
};
use std::path::PathBuf;

// =============================================================================
// Workspace Name Tests
// =============================================================================

#[test]
fn rejects_invalid_workspace_names() {
    assert!(WorkspaceName::new("").is_err());
    assert!(WorkspaceName::new("feat/auth").is_err());
    assert!(WorkspaceName::new("feat auth").is_err());
}

#[test]
fn rejects_invalid_workspace_templates() {
    assert!(WorkspaceTemplate::new("../{repo").is_err());
    assert!(WorkspaceTemplate::new("../{repo}.{branch}").is_err());
    assert!(WorkspaceTemplate::new("../repo}").is_err());
}

#[test]
fn renders_workspace_template_placeholders() {
    let template = WorkspaceTemplate::new("../{repo}.{workspace}").expect("valid template");
    let workspace = WorkspaceName::new("feature-auth").expect("valid workspace");

    let rendered = template.render("jj-navi", &workspace);

    assert_eq!(rendered, PathBuf::from("../jj-navi.feature-auth"));
}

#[test]
fn renders_repeated_workspace_template_placeholders() {
    let template =
        WorkspaceTemplate::new("../{repo}.{workspace}.{workspace}").expect("valid template");
    let workspace = WorkspaceName::new("feature-auth").expect("valid workspace");

    let rendered = template.render("jj-navi", &workspace);

    assert_eq!(
        rendered,
        PathBuf::from("../jj-navi.feature-auth.feature-auth")
    );
}

#[test]
fn renders_table_with_header() {
    let entries = vec![WorkspaceListEntry {
        is_current: false,
        name: WorkspaceName::new("feature-auth").expect("valid workspace"),
        statuses: vec![WorkspaceListStatus::Inferred],
        path: PathBuf::from("../repo.feature-auth"),
        path_state: WorkspacePathState::Inferred,
        commit_id: String::from("abc123"),
        message: String::from("Feature auth work"),
        freshness: WorkspaceFreshnessSnapshot::default(),
        diff: WorkspaceDiffSnapshot::default(),
        age: WorkspaceAgeSnapshot::default(),
    }];

    let rendered = render_workspace_table(&entries);

    assert!(rendered.starts_with("cur"));
    assert!(rendered.contains("status"));
    assert!(rendered.contains("../repo.feature-auth"));
    assert!(rendered.contains("[ inferred ]"));
    assert!(!rendered.contains("../repo.feature-auth [ inferred ]"));
    assert!(rendered.contains("abc123"));
    assert!(rendered.contains("Feature auth work"));
}

#[test]
fn renders_table_with_message_as_final_column() {
    let entries = vec![WorkspaceListEntry {
        is_current: false,
        name: WorkspaceName::new("feature-auth").expect("valid workspace"),
        statuses: vec![WorkspaceListStatus::Ok],
        path: PathBuf::from("../repo.feature-auth"),
        path_state: WorkspacePathState::Confirmed,
        commit_id: String::from("abc123"),
        message: String::from("Feature auth work"),
        freshness: WorkspaceFreshnessSnapshot::default(),
        diff: WorkspaceDiffSnapshot::default(),
        age: WorkspaceAgeSnapshot::default(),
    }];

    let rendered = render_workspace_table_with_width(&entries, None);
    let header = rendered.lines().next().expect("table header");
    let row = rendered.lines().nth(1).expect("table row");

    assert!(header.contains("commit  age  message"));
    assert!(row.ends_with("-    Feature auth work"));
}

#[test]
fn truncates_final_message_when_terminal_width_is_constrained() {
    let entries = vec![WorkspaceListEntry {
        is_current: false,
        name: WorkspaceName::new("feature-auth").expect("valid workspace"),
        statuses: vec![WorkspaceListStatus::Ok],
        path: PathBuf::from("../repo.feature-auth"),
        path_state: WorkspacePathState::Confirmed,
        commit_id: String::from("abc123"),
        message: String::from("Feature auth work with a much longer subject"),
        freshness: WorkspaceFreshnessSnapshot::default(),
        diff: WorkspaceDiffSnapshot::default(),
        age: WorkspaceAgeSnapshot::default(),
    }];

    let rendered = render_workspace_table_with_width(&entries, Some(1));
    let row = rendered.lines().nth(1).expect("table row");

    assert!(row.ends_with("-    Feature..."));
    assert!(!row.contains("much longer subject"));
}

#[test]
fn renders_workspace_table_for_current_workspace() {
    let entries = vec![
        WorkspaceListEntry {
            is_current: true,
            name: WorkspaceName::new("default").expect("valid workspace"),
            statuses: vec![WorkspaceListStatus::Ok],
            path: PathBuf::from("."),
            path_state: WorkspacePathState::Confirmed,
            commit_id: String::from("abc123"),
            message: String::from("Current work"),
            freshness: WorkspaceFreshnessSnapshot::default(),
            diff: WorkspaceDiffSnapshot::default(),
            age: WorkspaceAgeSnapshot::default(),
        },
        WorkspaceListEntry {
            is_current: false,
            name: WorkspaceName::new("feature-auth").expect("valid workspace"),
            statuses: vec![WorkspaceListStatus::Inferred],
            path: PathBuf::from("../repo.feature-auth"),
            path_state: WorkspacePathState::Inferred,
            commit_id: String::from("def456"),
            message: String::from("Feature auth work"),
            freshness: WorkspaceFreshnessSnapshot::default(),
            diff: WorkspaceDiffSnapshot::default(),
            age: WorkspaceAgeSnapshot::default(),
        },
    ];

    let rendered = render_workspace_table(&entries);

    assert!(rendered.contains("cur"));
    assert!(rendered.contains("workspace"));
    assert!(rendered.contains("status"));
    assert!(rendered.contains("commit"));
    assert!(rendered.contains("Feature auth work"));
    assert!(rendered.contains("[ inferred ]"));
    assert!(!rendered.contains("../repo.feature-auth [ inferred ]"));
}

// =============================================================================
// Shell Rendering Tests
// =============================================================================

#[test]
fn renders_bash_shell_init() {
    let rendered = render_shell_init("navi", ShellKind::Bash);

    assert!(rendered.contains("navi()"));
    assert!(rendered.contains(DIRECTIVE_FILE_ENV_VAR));
    assert!(rendered.contains("command navi \"$@\""));
    assert!(rendered.contains("local directive_file exit_code=0 source_exit_code=0"));
    assert!(rendered.contains("source_exit_code=$?"));
    assert!(rendered.contains("exit_code=$source_exit_code"));
    assert!(!rendered.contains("if [[ $exit_code -eq 0 ]]; then\n                exit_code=$?"));
}

#[test]
fn renders_missing_status_without_inferred_status_for_non_inferred_path() {
    let entries = vec![WorkspaceListEntry {
        is_current: false,
        name: WorkspaceName::new("feature-auth").expect("valid workspace"),
        statuses: vec![WorkspaceListStatus::Missing],
        path: PathBuf::from("../repo.feature-auth"),
        path_state: WorkspacePathState::Missing,
        commit_id: String::from("abc123"),
        message: String::from("Feature auth work"),
        freshness: WorkspaceFreshnessSnapshot::default(),
        diff: WorkspaceDiffSnapshot::default(),
        age: WorkspaceAgeSnapshot::default(),
    }];

    let rendered = render_workspace_table(&entries);

    assert!(rendered.contains("[ missing ]"));
    assert!(rendered.contains("../repo.feature-auth"));
    assert!(!rendered.contains("[ inferred ] [ missing ]"));
}

#[test]
fn renders_combined_workspace_statuses() {
    let entries = vec![WorkspaceListEntry {
        is_current: false,
        name: WorkspaceName::new("feature-auth").expect("valid workspace"),
        statuses: vec![WorkspaceListStatus::Inferred, WorkspaceListStatus::Missing],
        path: PathBuf::from("../repo.feature-auth"),
        path_state: WorkspacePathState::Missing,
        commit_id: String::from("abc123"),
        message: String::from("Feature auth work"),
        freshness: WorkspaceFreshnessSnapshot::default(),
        diff: WorkspaceDiffSnapshot::default(),
        age: WorkspaceAgeSnapshot::default(),
    }];

    let rendered = render_workspace_table(&entries);

    assert!(rendered.contains("[ inferred ] [ missing ]"));
    assert!(!rendered.contains("../repo.feature-auth [ missing ]"));
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

// =============================================================================
// Doctor Rendering Tests
// =============================================================================

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
