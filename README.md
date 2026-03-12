# jj-navi

<img width="788" height="640" alt="jj-navi" src="https://github.com/user-attachments/assets/88e8b46e-9a76-416b-9f76-b4480d6964e7" />

Navigation-first workspace UX for [Jujutsu](https://jj-vcs.github.io/jj/latest/).

Create, switch, list, and remove Jujutsu workspaces with predictable paths and optional shell integration for parallel human and AI workflows.

## What it does

```text
repo/
├── repo                 current workspace
├── repo.feature-auth    navi switch --create feature-auth
└── repo.fix-api         navi switch --create fix-api
```

`switch` is the center of the UX. Creating a workspace is just switching with `--create`.

## Install

```sh
# npm
npm install -g jj-navi

# cargo
cargo install jj-navi --version 0.0.1-alpha.4
```

Binary names:

- `navi`
- `nv`

Minimum supported `jj`: `0.39.0`
Minimum supported Node.js for npm install: `24`

## Quick start

```sh
navi switch --create feature-auth
navi switch feature-auth
navi list
navi remove feature-auth
```

## Shell integration

Install shell integration once if you want `navi switch ...` to change directories directly.

```sh
navi config shell install --shell zsh
source ~/.zshrc
```

Pick the shell you actually use: `bash` or `zsh`.

`navi config shell install` adds a managed block to your shell rc file so `switch` can update your current shell instead of only printing the destination path.

## Commands

```sh
navi switch <workspace>
navi switch --create <workspace>
navi switch --create <workspace> --revision <revset>
navi list
navi remove [workspace]
navi config shell init <bash|zsh>
navi config shell install [--shell <bash|zsh>]
```

## What `list` shows

- current marker
- workspace name
- path
- commit short id
- first-line commit message

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

- `remove` is forget-only by default; it does not delete workspace directories
- supported shells today: `bash`, `zsh`
- fish support, hooks, `doctor`, `prune`, and cross-workspace dirty status are planned in roadmap

## Maintainer notes

Release and `xtask` docs live in `xtask/README.md`.

## Thanks

This project was inspired by:

- [Worktrunk](https://github.com/max-sixty/worktrunk) - Worktrunk is a CLI for Git worktree management, designed for parallel AI agent workflows.
- [jj-ryu](https://github.com/dmmulroy/jj-ryu) - Stacked PRs for Jujutsu. Push bookmark stacks to GitHub and GitLab as chained pull requests.

## Art Credits
- [BoTW Link Pixel Art](https://www.reddit.com/r/zelda/comments/piy10r/botw_oc_hero_of_the_wild_pixel_art/)

## License

[MIT](https://github.com/eersnington/jj-navi/blob/main/LICENSE)
