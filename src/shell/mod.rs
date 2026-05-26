mod directive;
mod init;
mod install;
mod managed_block;

pub use directive::{DIRECTIVE_FILE_ENV_VAR, escape_shell_single_quotes, write_cd_directive};
pub use init::{render_fish_completion, render_fish_function, render_shell_init};
pub use install::render_shell_install_block;
pub(crate) use install::{doctor_findings, fish_config_dir, shell_rc_path, upsert_managed_block};
pub use managed_block::{MANAGED_BLOCK_END, MANAGED_BLOCK_START};
pub(crate) use managed_block::{ManagedBlockState, inspect_managed_block};

pub(crate) fn shell_command_names(primary: &str) -> [&str; 2] {
    if primary == "nv" {
        ["nv", "navi"]
    } else {
        ["navi", "nv"]
    }
}
