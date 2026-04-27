use crate::types::ShellKind;

use super::DIRECTIVE_FILE_ENV_VAR;

/// Render shell initialization code for the chosen shell.
#[must_use]
pub fn render_shell_init(command_name: &str, shell: ShellKind) -> String {
    let source_cmd = match shell {
        ShellKind::Bash | ShellKind::Zsh => "source",
    };
    let completion = render_completion_init(command_name, shell);

    format!(
        "# jj-navi shell integration for {shell}\nif command -v {command_name} >/dev/null 2>&1; then\n    {command_name}() {{\n        if [[ -n \"${{COMPLETE:-}}\" ]]; then\n            command {command_name} \"$@\"\n            return\n        fi\n\n        local directive_file exit_code=0 source_exit_code=0\n        directive_file=\"$(mktemp)\"\n        {directive_env}=\"$directive_file\" command {command_name} \"$@\" || exit_code=$?\n        if [[ -s \"$directive_file\" ]]; then\n            {source_cmd} \"$directive_file\"\n            source_exit_code=$?\n            if [[ $exit_code -eq 0 ]]; then\n                exit_code=$source_exit_code\n            fi\n        fi\n        rm -f \"$directive_file\"\n        return \"$exit_code\"\n    }}\n\n{completion}fi\n",
        shell = shell.as_str(),
        command_name = command_name,
        directive_env = DIRECTIVE_FILE_ENV_VAR,
        source_cmd = source_cmd,
        completion = completion,
    )
}

fn render_completion_init(command_name: &str, shell: ShellKind) -> String {
    match shell {
        ShellKind::Bash => format!(
            "    _{command_name}_lazy_complete() {{\n        if ! declare -F _clap_complete_{command_name} >/dev/null; then\n            eval \"$(COMPLETE=bash command {command_name} 2>/dev/null)\" || return\n        fi\n        _clap_complete_{command_name} \"$@\"\n    }}\n\n    complete -o nospace -o bashdefault -F _{command_name}_lazy_complete {command_name}\n",
        ),
        ShellKind::Zsh => format!(
            "    _{command_name}_lazy_complete() {{\n        if ! (( $+functions[_clap_dynamic_completer_{command_name}] )); then\n            eval \"$(COMPLETE=zsh command {command_name} 2>/dev/null)\" || return\n        fi\n        _clap_dynamic_completer_{command_name} \"$@\"\n    }}\n\n    if (( $+functions[compdef] )); then\n        compdef _{command_name}_lazy_complete {command_name}\n    fi\n",
        ),
    }
}
