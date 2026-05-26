use crate::types::ShellKind;

use super::{DIRECTIVE_FILE_ENV_VAR, shell_command_names};

/// Render shell initialization code for the chosen shell.
#[must_use]
pub fn render_shell_init(command_name: &str, shell: ShellKind) -> String {
    let command_blocks = shell_command_names(command_name)
        .into_iter()
        .map(|name| render_command_init(name, shell))
        .collect::<String>();

    format!(
        "# jj-navi shell integration for {shell}\n{command_blocks}",
        shell = shell.as_str(),
    )
}

fn render_command_init(command_name: &str, shell: ShellKind) -> String {
    match shell {
        ShellKind::Bash | ShellKind::Zsh => render_posix_command_init(command_name, shell),
        ShellKind::Fish => render_fish_function(command_name),
    }
}

fn render_posix_command_init(command_name: &str, shell: ShellKind) -> String {
    let directive_env = DIRECTIVE_FILE_ENV_VAR;
    let completion = render_posix_completion_init(command_name, shell);

    format!(
        "if command -v {command_name} >/dev/null 2>&1; then\n    {command_name}() {{\n        if [[ -n \"${{COMPLETE:-}}\" ]]; then\n            command {command_name} \"$@\"\n            return\n        fi\n\n        local directive_file exit_code=0 source_exit_code=0\n        directive_file=\"$(mktemp)\"\n        {directive_env}=\"$directive_file\" command {command_name} \"$@\" || exit_code=$?\n        if [[ -s \"$directive_file\" ]]; then\n            source \"$directive_file\"\n            source_exit_code=$?\n            if [[ $exit_code -eq 0 ]]; then\n                exit_code=$source_exit_code\n            fi\n        fi\n        rm -f \"$directive_file\"\n        return \"$exit_code\"\n    }}\n\n{completion}fi\n",
    )
}

fn render_posix_completion_init(command_name: &str, shell: ShellKind) -> String {
    match shell {
        ShellKind::Bash => format!(
            "    _{command_name}_lazy_complete() {{\n        if ! declare -F _clap_complete_{command_name} >/dev/null; then\n            eval \"$(COMPLETE=bash command {command_name} 2>/dev/null)\" || return\n        fi\n        _clap_complete_{command_name} \"$@\"\n    }}\n\n    complete -o nospace -o bashdefault -F _{command_name}_lazy_complete {command_name}\n",
        ),
        ShellKind::Zsh => format!(
            "    _{command_name}_lazy_complete() {{\n        if ! (( $+functions[_clap_dynamic_completer_{command_name}] )); then\n            eval \"$(COMPLETE=zsh command {command_name} 2>/dev/null)\" || return\n        fi\n        _clap_dynamic_completer_{command_name} \"$@\"\n    }}\n\n    if (( $+functions[compdef] )); then\n        compdef _{command_name}_lazy_complete {command_name}\n    fi\n",
        ),
        ShellKind::Fish => String::new(),
    }
}

/// Render a fish function file for a single command.
#[must_use]
pub fn render_fish_function(command_name: &str) -> String {
    let directive_env = DIRECTIVE_FILE_ENV_VAR;
    format!(
        "function {command_name}\n    if set -q COMPLETE\n        command {command_name} $argv\n        return\n    end\n    set -l directive_file (mktemp)\n    set -l exit_code 0\n    set -lx {directive_env} $directive_file\n    command {command_name} $argv\n    or set exit_code $status\n    if test -s $directive_file\n        source $directive_file\n        set -l source_exit $status\n        if test $exit_code -eq 0\n            set exit_code $source_exit\n        end\n    end\n    rm -f $directive_file\n    return $exit_code\nend\n",
    )
}

/// Render a fish completion file for a single command.
#[must_use]
pub fn render_fish_completion(command_name: &str) -> String {
    format!(
        "complete --keep-order --exclusive --command {command_name} --arguments \"(env COMPLETE=fish command {command_name} -- (commandline --current-process --tokenize --cut-at-cursor) (commandline --current-token))\"\n"
    )
}
