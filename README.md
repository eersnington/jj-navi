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

## Current version

- `0.0.1-alpha.1`
- minimum supported `jj`: `0.39.0`

## Install

```sh
cargo install --path .
```

This installs both `navi` and `nv`.

## Current commands

- `navi switch <workspace>`
- `navi switch --create <workspace>`
- `navi switch --create <workspace> --revision <revset>`
- `navi list`

The same commands also work with `nv`.

## Usage

```sh
navi switch --create feature-auth
cd "$(navi switch feature-auth)"
navi list
```

## Notes

- current scope is v0
- shell integration is not implemented yet, so `switch` prints the target path for now
- future versions are planned to add shell integration, remove, metadata, and richer workspace visibility
