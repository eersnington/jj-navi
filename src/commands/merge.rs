use std::path::Path;

use crate::Result;
use crate::output::{render_merge_preview, render_merge_preview_json};
use crate::repo::NaviWorkspace;
use crate::types::WorkspaceName;

/// Run the `merge` command.
///
/// # Errors
///
/// Returns an error if source or target resolution fails, if either workspace
/// is unhealthy, or if JSON serialization fails.
pub fn run_merge_preview(path: &Path, from: &str, into: Option<&str>, json: bool) -> Result<()> {
    let repo = NaviWorkspace::open(path)?;
    let source = WorkspaceName::new(from.to_owned())?;
    let target = into
        .map(|workspace| WorkspaceName::new(workspace.to_owned()))
        .transpose()?;
    let preview = repo.merge_preview(&source, target.as_ref())?;

    if json {
        println!("{}", render_merge_preview_json(&preview)?);
    } else {
        print!("{}", render_merge_preview(&preview));
    }

    Ok(())
}
