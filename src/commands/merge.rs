use std::path::Path;

use crate::Result;
use crate::output::render_merge_outcome;
use crate::repo::NaviWorkspace;
use crate::types::WorkspaceName;

/// Run the `merge` command.
///
/// # Errors
///
/// Returns an error if source or target resolution fails, if either workspace
/// is unhealthy, or if `jj duplicate`/`jj rebase` fails.
pub fn run_merge(path: &Path, from: &str, into: Option<&str>) -> Result<()> {
    let repo = NaviWorkspace::open(path)?;
    let source = WorkspaceName::new(from.to_owned())?;
    let target = into
        .map(|workspace| WorkspaceName::new(workspace.to_owned()))
        .transpose()?;
    let outcome = repo.merge_workspace(&source, target.as_ref())?;

    eprint!("{}", render_merge_outcome(&outcome));

    Ok(())
}
