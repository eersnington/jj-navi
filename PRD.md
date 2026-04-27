# jj-navi PRD

## Product Summary

`jj-navi` is a small Rust CLI that makes Jujutsu workspaces fast to create, switch, inspect, merge, and remove for parallel human and AI-agent workflows.

It is a workspace lifecycle layer over native `jj workspace` primitives. It should make common parallel-workspace operations obvious without becoming a replacement VCS workflow engine.

## Product Thesis

The core promise is:

```text
workspace name -> trusted path -> useful action
```

That promise depends on three things:

1. workspace switching is fast and predictable
2. workspace paths are validated before use
3. many active workspaces are understandable at a glance

## Primary Users

- developers using Jujutsu daily
- developers running several parallel tasks
- developers using AI coding agents in separate workspaces
- users migrating from Git worktree workflows
- users who want a lightweight Worktrunk-style UX for JJ

## Current Product Shape

Shipping command families:

- `navi switch <workspace>`
- `navi switch --create <workspace>`
- `navi switch --create <workspace> --revision <revset>`
- `navi switch -`
- `navi switch @`
- `navi list [--json] [--compact]`
- `navi doctor [--json] [--compact]`
- `navi remove <workspace>`
- `navi config shell init <bash|zsh>`
- `navi config shell install [--shell <bash|zsh>]`

Supported binaries:

- `navi`
- `nv`

## Product Goals

1. Make switching JJ workspaces fast and predictable.
2. Make creating a workspace feel like a mode of switching.
3. Make many parallel workspaces understandable without deep JJ knowledge.
4. Keep JJ as the source of truth.
5. Make workspace removal destructive but guarded and explicit.
6. Add merge preview support without hiding JJ semantics.
7. Stay small and conservative around destructive actions.

## Non-Goals

- agent orchestration
- terminal pane, tmux, or iTerm session management
- port allocation
- environment file editing
- dev server launch or process management
- framework-specific monorepo setup
- replacing general JJ commands
- managing Git branches or Git worktrees directly
- PR/CI workflows in the near-term roadmap
- fully automatic merge selection

## Key Workflow

The intended loop:

```sh
navi switch --create feature-one
navi switch --create feature-two
navi switch -
navi list
navi merge preview --from feature-two
navi remove feature-two --yes
```

Mental model:

1. create or jump with `switch`
2. inspect workspace inventory and active work with `list`
3. merge deliberately with a read-only merge preview before running JJ commands
4. remove finished or abandoned workspaces with an explicit destructive guard
5. diagnose degraded state with `doctor` only when needed

## Near-Term Roadmap

### 1. Fresh Workspace List

Keep:

```sh
navi list [--json] [--compact]
```

Purpose:

- answer “what work is active across my workspaces?” without requiring users to know JJ snapshot mechanics
- solve stale cross-workspace visibility before merge decisions
- provide both human triage and scriptable JSON

Human output should be compact and scan-friendly. Fields:

- current marker
- workspace name
- path health
- currentness health for workspaces that could not be made current
- compact diff summary
- commit id
- first-line description
- workspace age when created by Navi

JSON output should expose the same concepts with stable field names.

Rules:

- `list` makes healthy workspace state current before rendering by running `jj util snapshot` internally in each switchable workspace
- snapshot mechanics are not exposed as command UX
- `list` does not run `jj workspace update-stale` or auto-repair workspace files
- `list` does not block forever on one workspace; degraded currentness remains visible per row
- stale or missing workspaces should remain visible instead of failing the whole command

### 2. Guarded Workspace Removal

Change:

```sh
navi remove <workspace> [--yes|-y]
```

Purpose:

- retire a local JJ workspace in one command
- keep JJ workspace records and local workspace directories from drifting apart
- make destructive deletion obvious and guarded

Rules:

- `remove` refuses to remove the current workspace
- `remove` validates the target workspace path before deletion
- `remove` forgets the JJ workspace and removes Navi metadata
- `remove` deletes the local workspace directory
- without `--yes`/`-y`, `remove` must clearly show the path that will be deleted and ask for confirmation
- with `--yes`/`-y`, `remove` skips confirmation for fast agent/human cleanup
- deletion must use Rust filesystem APIs, not shelling out to `rm -rf`
- failures should say which step failed and what state may remain

### 3. Merge Preview

Add:

```sh
navi merge preview --from <workspace> [--into <workspace>] [--json]
```

Purpose:

- help users consolidate useful work from parallel JJ workspaces
- make the duplicate-first merge pattern visible before mutation
- provide a bridge from `navi list` to explicit JJ merge commands

Rules:

- source workspace is always explicit
- target defaults to the current workspace unless `--into` is provided
- `preview` is read-only and does not mutate JJ state, workspace files, metadata, or bookmarks
- source and target paths must be healthy before a preview is produced
- stale, missing, ambiguous, or not-current states stop with guidance instead of guessing
- when the source belongs to another workspace, preview should recommend the safe `jj duplicate` then `jj rebase` pattern
- `merge apply` is out of scope until preview behavior is proven useful

## Port And Env Allocation Decision

Port and env allocation are out of scope.

Reasoning:

- Vite, Turborepo, Bazel, Rust services, backend stacks, direnv, and custom scripts all express runtime configuration differently
- automatic env-file edits can touch secrets or project-specific conventions
- assigning a single port is not enough for multi-service workspaces
- runtime isolation is a project concern, while Navi’s scope is workspace lifecycle and JJ state

Possible later compromise:

- user-authored workspace notes such as URL, port, or label
- display-only metadata in `list`
- no automatic allocation or env mutation

## Acceptance Criteria

### Current Core

1. `switch` resolves only validated workspace paths.
2. `switch --create` creates workspaces at deterministic paths.
3. `switch -` returns to the previously recorded workspace.
4. `switch @` resolves the current workspace explicitly.
5. `list` shows workspace inventory and degraded path state.
6. `list` makes healthy workspaces current before rendering active work.
7. `list` shows compact diff statistics and workspace age when known.
8. `doctor` explains degraded repo, workspace, and shell state.
9. `remove` refuses to remove the current workspace.
10. `remove --yes` forgets a non-current workspace and deletes its local directory.

### New Scope

1. `remove` clearly warns before destructive directory deletion unless `--yes`/`-y` is provided.
2. `merge preview` reports source, target, and intended JJ commands without applying them.
3. `merge preview` explains the duplicate-first pattern for work from another workspace.

## Testing Priorities

Tests should cover external behavior with real JJ repositories where practical.

Priority coverage:

- workspace creation and switching
- degraded path recovery and reporting
- fresh list human output
- fresh list JSON output
- list currentness, skipped workspace reporting, and failure reporting
- list diff summary and workspace age
- guarded destructive remove behavior
- remove `--yes`/`-y` fast path
- merge preview read-only behavior
- merge preview preflight behavior
- `navi` and `nv` parity for user-facing commands

## Documentation Priorities

Docs should explain the product in this order:

1. what `jj-navi` is
2. the core switch/list workflow
3. shell integration
4. fresh cross-workspace list behavior
5. guarded destructive remove
6. merge preview for parallel workspace consolidation
7. doctor and degraded-state recovery

## References

- Jujutsu: <https://github.com/jj-vcs/jj>
- Jujutsu working copy docs: <https://docs.jj-vcs.dev/latest/working-copy/>
- Jujutsu revsets: <https://github.com/jj-vcs/jj/blob/main/docs/revsets.md>
- Worktrunk: <https://github.com/max-sixty/worktrunk>
- Worktrunk docs: <https://worktrunk.dev>
- jj-ryu: <https://github.com/dmmulroy/jj-ryu>
