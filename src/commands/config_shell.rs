use std::fs;

use crate::shell::{
    render_fish_completion, render_fish_function, render_shell_install_block, shell_command_names,
    shell_rc_path, upsert_managed_block,
};
use crate::types::ShellKind;
use crate::{Error, Result};

/// Run `config shell init`.
///
/// # Errors
///
/// Returns an error if the shell is missing.
pub fn run_shell_init(command_name: &str, shell: Option<ShellKind>) -> Result<()> {
    use crate::shell::render_shell_init;
    let shell = shell.ok_or(Error::ShellRequired)?;

    print!("{}", render_shell_init(command_name, shell));
    Ok(())
}

/// Run `config shell install`.
///
/// # Errors
///
/// Returns an error if the shell is not supported, if shell detection fails,
/// or if the shell rc file cannot be updated.
pub fn run_shell_install(command_name: &str, shell: Option<ShellKind>) -> Result<()> {
    let shell = match shell {
        Some(shell) => shell,
        None => ShellKind::detect()?,
    };

    if shell == ShellKind::Fish {
        return install_fish(command_name);
    }

    let rc_path = shell_rc_path(shell)?;
    let block = render_shell_install_block(command_name, shell);
    let existing = match fs::read_to_string(&rc_path) {
        Ok(existing) => existing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    let updated = upsert_managed_block(&existing, &block, &rc_path)?;

    if let Some(parent) = rc_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&rc_path, updated)?;
    println!("installed shell integration in {}", rc_path.display());
    Ok(())
}

fn install_fish(command_name: &str) -> Result<()> {
    let base = crate::shell::fish_config_dir()?;
    let functions_dir = base.join("functions");
    let completions_dir = base.join("completions");
    fs::create_dir_all(&functions_dir)?;
    fs::create_dir_all(&completions_dir)?;

    for name in shell_command_names(command_name) {
        let func_path = functions_dir.join(format!("{name}.fish"));
        fs::write(&func_path, render_fish_function(name))?;
        println!("installed {}", func_path.display());

        let comp_path = completions_dir.join(format!("{name}.fish"));
        fs::write(&comp_path, render_fish_completion(name))?;
        println!("installed {}", comp_path.display());
    }
    Ok(())
}
