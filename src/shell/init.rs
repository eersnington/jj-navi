use crate::types::ShellKind;

use super::DIRECTIVE_FILE_ENV_VAR;

/// Render shell initialization code for the chosen shell.
#[must_use]
pub fn render_shell_init(command_name: &str, shell: ShellKind) -> String {
    let source_cmd = match shell {
        ShellKind::Bash | ShellKind::Zsh => "source",
    };

    format!(
        "# jj-navi shell integration for {shell}\nif command -v {command_name} >/dev/null 2>&1; then\n    {command_name}() {{\n        local directive_file exit_code=0 source_exit_code=0\n        directive_file=\"$(mktemp)\"\n        {directive_env}=\"$directive_file\" command {command_name} \"$@\" || exit_code=$?\n        if [[ -s \"$directive_file\" ]]; then\n            {source_cmd} \"$directive_file\"\n            source_exit_code=$?\n            if [[ $exit_code -eq 0 ]]; then\n                exit_code=$source_exit_code\n            fi\n        fi\n        rm -f \"$directive_file\"\n        return \"$exit_code\"\n    }}\nfi\n",
        shell = shell.as_str(),
        command_name = command_name,
        directive_env = DIRECTIVE_FILE_ENV_VAR,
        source_cmd = source_cmd,
    )
}
