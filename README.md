# jj-navi

`jj-navi` is workspace navigation and management for Jujutsu, built for parallel human and AI agent workflows.

It is a UX layer over native `jj workspace` commands.

Core idea:

`workspace name -> deterministic path -> fast navigation`

## What it does

- create workspaces through `switch --create`
- switch to existing workspaces quickly
- remove workspaces safely with forget-only defaults
- list workspaces with current marker, path, commit, and message
- install bash and zsh shell integration so `switch` can change directories directly
- keep repo-scoped config and navi metadata in shared `.jj/repo/navi/`
- make multi-workspace flows easier to reason about

## Who it is for

- people working on multiple tasks in parallel
- people running coding agents in separate workspaces
- people who want a worktree-like workspace UX on top of `jj`

## Requirements
- minimum supported `jj`: `0.39.0`

## Install

```sh
cargo install jj-navi --version 0.0.1-alpha.2
```

or

```sh
npm i -g jj-navi
```

or

```sh
git clone https://github.com/eersnington/jj-navi.git
cd jj-navi
cargo install --path .
```

This installs both `navi` and `nv`.

Prebuilt npm binaries currently target:

- macOS arm64
- macOS x64
- Linux arm64 (gnu)
- Linux x64 (gnu)
- Linux arm64 (musl)
- Linux x64 (musl)

## Current commands

- `navi switch <workspace>`
- `navi switch --create <workspace>`
- `navi switch --create <workspace> --revision <revset>`
- `navi list`
- `navi remove [workspace]`
- `navi config shell init <bash|zsh>`
- `navi config shell install [--shell <bash|zsh>]`

The same commands also work with `nv`.

## Usage

Without shell integration:

```sh
navi switch --create feature-auth
cd "$(navi switch feature-auth)"
navi list
```

`navi switch ...` prints the target path unless shell integration is active.

With shell integration installed:

```sh
navi config shell install --shell zsh
source ~/.zshrc

navi switch --create feature-auth
navi switch feature-auth
navi remove feature-auth
```

When shell integration is active, `navi switch ...` writes a `cd` directive for your shell wrapper instead of printing the path.

`navi list` shows:

- current marker
- workspace name
- navigation path
- working-copy commit short id
- first-line commit message

Repo-scoped config and metadata live under:

```text
.jj/repo/navi/config.toml
.jj/repo/navi/workspaces.toml
```

## Notes

- current scope is v1 core workspace UX
- `remove` is forget-only by default; it does not delete workspace directories
- bash and zsh shell integration are supported; fish is not yet supported
- cross-workspace dirty status, hooks, `doctor`, and `prune` remain future work

## Release Fragments

Every user-facing PR should add one fragment in `.release/`.

Install the maintainer helper once:

```sh
cargo install --path xtask --force
```

If `navi-release` is not found, add Cargo's bin dir to your shell path:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

Create one with:

```sh
navi-release "fix nested workspace discovery" -s cli
```

The fragment body becomes changelog and GitHub release notes.

Default bump is `patch`, so only pass `minor` or `major` when needed.

Run `navi-release` with no args for an interactive fragment wizard.

## Releasing

Release flow is manual and two-step:

1. Run `Prepare Release` in GitHub Actions with the target version.
2. Review and merge the generated release PR.
3. Run `Publish Release` in GitHub Actions with the same version.

`Prepare Release` rolls `.release/*.md` fragments into `CHANGELOG.md`, syncs versions, and opens a release PR.

`Publish Release` builds binaries, publishes crates.io and npm packages, tags `v<version>`, and creates the GitHub Release.

For npm, configure trusted publishing for this GitHub repo on each `jj-navi*` package so the publish workflow can use OIDC instead of an npm token.
