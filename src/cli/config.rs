use crate::Result;
use crate::output::render_shell_init;
use crate::types::ShellKind;

/// Run `config shell init`.
///
/// # Errors
///
/// Returns an error if the shell is not supported.
pub fn run_shell_init(command_name: &str, shell: &str) -> Result<()> {
    let shell = ShellKind::new(shell)?;

    print!("{}", render_shell_init(command_name, shell));
    Ok(())
}
