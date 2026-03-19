use crate::Error;

/// Marker for the start of the managed shell block.
pub const MANAGED_BLOCK_START: &str = "# >>> jj-navi shell init >>>";
/// Marker for the end of the managed shell block.
pub const MANAGED_BLOCK_END: &str = "# <<< jj-navi shell init <<<";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ManagedBlockState {
    Missing,
    Present { start: usize, end: usize },
    Invalid(&'static str),
}

pub(crate) fn inspect_managed_block(existing: &str) -> ManagedBlockState {
    let starts = existing
        .match_indices(MANAGED_BLOCK_START)
        .collect::<Vec<_>>();
    let ends = existing
        .match_indices(MANAGED_BLOCK_END)
        .collect::<Vec<_>>();

    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => ManagedBlockState::Missing,
        ([(_, _)], [(_, _)]) => {
            let start = starts[0].0;
            let end = ends[0].0;
            if end < start {
                ManagedBlockState::Invalid("managed block markers are out of order")
            } else {
                ManagedBlockState::Present {
                    start,
                    end: end + MANAGED_BLOCK_END.len(),
                }
            }
        }
        ([], _) | (_, []) => ManagedBlockState::Invalid("managed block markers are unbalanced"),
        _ => ManagedBlockState::Invalid("managed block markers are duplicated"),
    }
}

pub(crate) fn invalid_shell_rc_file(path: &std::path::Path, message: &'static str) -> Error {
    Error::InvalidShellRcFile {
        path: path.to_path_buf(),
        message,
    }
}
