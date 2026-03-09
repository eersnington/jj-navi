use jj_navi::output::render_workspace_table;
use jj_navi::types::{WorkspaceEntry, WorkspaceName};
use std::path::PathBuf;

#[test]
fn rejects_invalid_workspace_names() {
    assert!(WorkspaceName::new("").is_err());
    assert!(WorkspaceName::new("feat/auth").is_err());
    assert!(WorkspaceName::new("feat auth").is_err());
}

#[test]
fn renders_table_with_header() {
    let entries = vec![WorkspaceEntry {
        name: WorkspaceName::new("feature-auth").expect("valid workspace"),
        path: PathBuf::from("../repo.feature-auth"),
    }];

    let rendered = render_workspace_table(&entries);

    assert!(rendered.starts_with("workspace"));
    assert!(rendered.contains("../repo.feature-auth"));
}
