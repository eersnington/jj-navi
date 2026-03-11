use jj_navi::output::render_workspace_table;
use jj_navi::types::{WorkspaceListEntry, WorkspaceName};
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
        commit_id: String::from("abc123"),
        message: String::from("Feature auth work"),
    }];

    let rendered = render_workspace_table(&entries);

    assert!(rendered.starts_with("marker"));
    assert!(rendered.contains("../repo.feature-auth"));
    assert!(rendered.contains("abc123"));
    assert!(rendered.contains("Feature auth work"));
}
