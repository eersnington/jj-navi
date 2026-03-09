use std::path::Path;

use jj_navi::Result;
use jj_navi::output::render_workspace_table;
use jj_navi::repo::NaviWorkspace;

pub fn run_list(path: &Path) -> Result<()> {
    let repo = NaviWorkspace::open(path)?;
    let entries = repo.list_workspaces()?;

    print!("{}", render_workspace_table(&entries));
    Ok(())
}
