use jj_navi::output::{
    DIRECTIVE_FILE_ENV_VAR, MANAGED_BLOCK_END, MANAGED_BLOCK_START, render_shell_init,
    render_shell_install_block, render_workspace_table,
};
use jj_navi::types::{ShellKind, WorkspaceListEntry, WorkspaceName, WorkspacePathState};
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
        path: PathBuf::from("../repo.feature-auth"),
        path_is_inferred: true,
        path_state: WorkspacePathState::Inferred,
        commit_id: String::from("abc123"),
        message: String::from("Feature auth work"),
    }];

    let rendered = render_workspace_table(&entries);

    assert!(rendered.starts_with("marker"));
    assert!(rendered.contains("../repo.feature-auth"));
    assert!(rendered.contains("[inferred]"));
    assert!(rendered.contains("abc123"));
    assert!(rendered.contains("Feature auth work"));
}

#[test]
fn renders_missing_without_inferred_marker_for_non_inferred_path() {
    let entries = vec![WorkspaceListEntry {
        is_current: false,
        name: WorkspaceName::new("feature-auth").expect("valid workspace"),
        path: PathBuf::from("../repo.feature-auth"),
        path_is_inferred: false,
        path_state: WorkspacePathState::Missing,
        commit_id: String::from("abc123"),
        message: String::from("Feature auth work"),
    }];

    let rendered = render_workspace_table(&entries);

    assert!(rendered.contains("../repo.feature-auth [missing]"));
    assert!(!rendered.contains("[inferred] [missing]"));
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
