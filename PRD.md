# jj-navi PRD

## Product Summary

`jj-navi` is a small Rust CLI that makes Jujutsu workspaces fast to create, switch, inspect, merge, and clean up for parallel human and AI-agent workflows.

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
5. Make cleanup safe and explicit.
6. Add guided merge support without hiding JJ semantics.
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
navi snapshot-all
navi overview
navi merge plan --from feature-two
navi cleanup plan
```

Mental model:

1. create or jump with `switch`
2. inspect workspace inventory with `list`
3. refresh working-copy state explicitly with `snapshot-all`
4. inspect work in flight with `overview`
5. merge deliberately with `merge plan/apply`
6. clean up safely with `cleanup plan/apply` or `remove`
7. diagnose degraded state with `doctor` only when needed

## Near-Term Roadmap

### 1. Explicit Snapshots

Add:

```sh
navi snapshot-all [--json] [--compact]
```

Purpose:

- solve stale cross-workspace visibility before overview or merge decisions
- make the action explicit instead of hiding mutation inside `list`

Behavior:

- run `jj snapshot` in every switchable workspace
- skip missing or stale workspace paths and report them
- return non-zero if any snapshot fails
- include machine-readable output for scripts and agents

### 2. Work-In-Flight Overview

Add:

```sh
navi overview [--json] [--compact] [--snapshot]
```

Purpose:

- answer “what work is active across my workspaces?”
- provide both human triage and scriptable JSON

Human output should be compact and scan-friendly. Initial fields:

- current marker
- workspace name
- path health
- commit id
- change id if available
- first-line description
- empty/non-empty state
- conflict state
- diff summary if cheap and reliable

JSON output should expose the same concepts with stable field names.

Rules:

- default `overview` should not mutate state
- `--snapshot` may run snapshot logic first
- stale or missing workspaces should remain visible instead of failing the whole command

### 3. Safe Cleanup

Add:

```sh
navi cleanup plan [--json] [--compact]
navi cleanup apply [--forget] [--delete-dirs] [--yes]
```

Purpose:

- clean stale workspace lifecycle state without risking active work
- separate inspection from mutation

Cleanup categories:

- Navi metadata exists but JJ no longer lists the workspace
- JJ lists a workspace but Navi has no metadata
- JJ lists a workspace whose directory is missing
- JJ lists a workspace whose directory is stale
- a forgotten workspace directory is a deletion candidate

Rules:

- `cleanup plan` is read-only
- `cleanup apply` must require explicit action flags
- current workspace deletion is always refused
- directory deletion requires `--delete-dirs` and confirmation or `--yes`
- `jj abandon` is out of scope for the first cleanup version

### 4. Guided Merge

Add:

```sh
navi merge plan --from <workspace> [--into <workspace>] [--json]
navi merge apply --from <workspace> [--into <workspace>] [--dry-run]
```

Purpose:

- help users move useful work from one workspace into another with explicit source and target
- make merge preflight safer without becoming an autonomous merge engine

Rules:

- source workspace is always explicit
- target defaults to the current workspace unless `--into` is provided
- `plan` prints the proposed JJ operation without mutation
- `apply` runs only after path health and source/target checks pass
- stale, missing, ambiguous, or conflicted states stop with guidance instead of guessing

## Port And Env Allocation Decision

Port and env allocation are out of scope.

Reasoning:

- Vite, Turborepo, Bazel, Rust services, backend stacks, direnv, and custom scripts all express runtime configuration differently
- automatic env-file edits can touch secrets or project-specific conventions
- assigning a single port is not enough for multi-service workspaces
- runtime isolation is a project concern, while Navi’s scope is workspace lifecycle and JJ state

Possible later compromise:

- user-authored workspace notes such as URL, port, or label
- display-only metadata in `overview`
- no automatic allocation or env mutation

## Acceptance Criteria

### Current Core

1. `switch` resolves only validated workspace paths.
2. `switch --create` creates workspaces at deterministic paths.
3. `switch -` returns to the previously recorded workspace.
4. `switch @` resolves the current workspace explicitly.
5. `list` shows workspace inventory and degraded path state.
6. `doctor` explains degraded repo, workspace, and shell state.
7. `remove` refuses to remove the current workspace.

### New Scope

1. `snapshot-all` snapshots every switchable workspace and reports skipped workspaces.
2. `overview` shows work-in-flight state in human and JSON output.
3. `cleanup plan` identifies cleanup candidates without mutation.
4. `cleanup apply` performs only explicit, confirmed cleanup actions.
5. `merge plan` reports source, target, and intended JJ operation without mutation.
6. `merge apply` refuses stale, missing, or ambiguous paths before running JJ operations.

## Testing Priorities

Tests should cover external behavior with real JJ repositories where practical.

Priority coverage:

- workspace creation and switching
- degraded path recovery and reporting
- list and overview human output
- list and overview JSON output
- snapshot-all success, skipped workspace reporting, and failure reporting
- cleanup plan read-only behavior
- cleanup apply safeguards
- merge plan read-only behavior
- merge apply preflight behavior
- `navi` and `nv` parity for user-facing commands

## Documentation Priorities

Docs should explain the product in this order:

1. what `jj-navi` is
2. the core switch/list workflow
3. shell integration
4. explicit snapshots
5. work-in-flight overview
6. safe cleanup
7. guided merge
8. doctor and degraded-state recovery

## References

- Jujutsu: <https://github.com/jj-vcs/jj>
- Jujutsu working copy docs: <https://docs.jj-vcs.dev/latest/working-copy/>
- Jujutsu revsets: <https://github.com/jj-vcs/jj/blob/main/docs/revsets.md>
- Worktrunk: <https://github.com/max-sixty/worktrunk>
- Worktrunk docs: <https://worktrunk.dev>
- jj-ryu: <https://github.com/dmmulroy/jj-ryu>
