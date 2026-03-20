# jj-navi

<img width="788" height="640" alt="jj-navi" src="https://github.com/user-attachments/assets/88e8b46e-9a76-416b-9f76-b4480d6964e7" />

Workspace management for [Jujutsu](https://jj-vcs.github.io/jj/latest/), built for parallel human and AI agent workflows.

Make JJ workspaces easier to create, switch, inspect, and clean up with predictable paths and optional shell integration.

## What it does

```text
repo/
├── repo                 current workspace
├── repo.feature-auth    navi switch --create feature-auth
└── repo.fix-api         navi switch --create fix-api
```

`jj-navi` makes parallel workspace work feel simpler and more predictable. Creating a workspace is just switching with `--create`.

## Install

```sh
# npm
npm install -g jj-navi

# cargo
cargo install jj-navi --version 0.2.0
```

Binary names:

- `navi`
- `nv`

Minimum supported `jj`: `0.39.0`
Minimum supported Node.js for npm install: `24`


## Shell integration

Install shell integration once if you want `navi switch ...` to change directories directly.

```sh
navi config shell install --shell zsh
source ~/.zshrc
```

Pick the shell you actually use: `bash` or `zsh`.

`navi config shell install` adds a managed block to your shell rc file so `switch` can update your current shell instead of only printing the destination path.

## Quick start

```sh
navi doctor
navi switch --create feature-auth
navi list
navi switch -
navi remove feature-auth
```

## Commands

```sh
navi switch <workspace>
navi switch -
navi switch @
navi switch --create <workspace>
navi switch --create <workspace> --revision <revset>
navi list
navi list --json
navi list --full
navi doctor [--json] [--compact]
navi remove <workspace>
navi config shell init <bash|zsh>
navi config shell install [--shell <bash|zsh>]
```

## Repo config

Repo-scoped config and metadata live in shared Jujutsu storage:

```text
.jj/repo/navi/config.toml
.jj/repo/navi/workspaces.toml
```

Default workspace path template:

```text
../{repo}.{workspace}
```

## Notes

- `switch` can recover from missing JJ workspace-path records when `navi` can validate a fallback path
- `switch` only warns when it had to use a weaker template-based fallback
- `remove` requires an explicit workspace name and refuses to remove the current workspace
- `remove` is forget-only by default; it does not delete workspace directories
- supported shells today: `bash`, `zsh`

## Maintainer notes

Release and `xtask` docs live in `xtask/README.md`.

## Special thanks

This project was inspired by:

- [Worktrunk](https://github.com/max-sixty/worktrunk) - Worktrunk is a CLI for Git worktree management, designed for parallel AI agent workflows.
- [jj-ryu](https://github.com/dmmulroy/jj-ryu) - Stacked PRs for Jujutsu. Push bookmark stacks to GitHub and GitLab as chained pull requests.

## Art Credits
- [BoTW Link Pixel Art](https://www.reddit.com/r/zelda/comments/piy10r/botw_oc_hero_of_the_wild_pixel_art/)

## License

[MIT](https://github.com/eersnington/jj-navi/blob/main/LICENSE)
