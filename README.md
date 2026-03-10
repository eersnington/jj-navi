# jj-navi

`jj-navi` is workspace navigation and management for Jujutsu, built for parallel human and AI agent workflows.

It is a UX layer over native `jj workspace` commands.

Core idea:

`workspace name -> deterministic path -> fast navigation`

## What it does

- create workspaces through `switch --create`
- switch to existing workspaces quickly
- list known workspaces
- make multi-workspace flows easier to reason about

## Who it is for

- people working on multiple tasks in parallel
- people running coding agents in separate workspaces
- people who want a worktree-like workspace UX on top of `jj`

## Requirements
- minimum supported `jj`: `0.39.0`

## Install

```sh
cargo install jj-navi --version 0.0.1-alpha.1
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

The same commands also work with `nv`.

## Usage

Current v0 flow, until shell integration lands:

```sh
navi switch --create feature-auth
cd "$(navi switch feature-auth)"
navi list
```

`navi switch ...` currently prints the target path. The `$(...)` part passes that path into `cd`.

Goal state in a later version:

```sh
navi switch feature-auth
```

and your shell changes directory directly.

## Notes

- current scope is v0
- shell integration is not implemented yet, so `switch` prints the target path for now
- future versions are planned to add shell integration, remove, metadata, and richer workspace visibility

## Release Fragments

Every user-facing PR should add one fragment in `.release/`.

Create one with:

```sh
./scripts/release/new "fix nested workspace discovery" -s cli
```

The fragment body becomes changelog and GitHub release notes.

Default bump is `patch`, so only pass `minor` or `major` when needed.

## Releasing

Release flow is manual and two-step:

1. Run `Prepare Release` in GitHub Actions with the target version.
2. Review and merge the generated release PR.
3. Run `Publish Release` in GitHub Actions with the same version.

`Prepare Release` rolls `.release/*.md` fragments into `CHANGELOG.md`, syncs versions, and opens a release PR.

`Publish Release` builds binaries, publishes crates.io and npm packages, tags `v<version>`, and creates the GitHub Release.

For npm, configure trusted publishing for this GitHub repo on each `jj-navi*` package so the publish workflow can use OIDC instead of an npm token.
