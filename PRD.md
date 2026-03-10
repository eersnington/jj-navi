# jj-navi PRD

## 1. Overview

`jj-navi` is a standalone Rust CLI for fast, predictable navigation between Jujutsu workspaces.

Its core idea is simple:

`workspace name -> deterministic path -> fast navigation`

The tool is a UX layer over native `jj workspace` primitives. It does not implement its own workspace engine, version-control model, or repository state machine.

`jj-navi` exists to make Jujutsu workspaces practical for parallel human and AI agent workflows.

More specifically, it should make Jujutsu workspaces feel as lightweight and navigable as directories or worktrees in day-to-day use.

## 2. Product Goals

1. Make switching Jujutsu workspaces fast and predictable.
2. Make creating new workspaces feel like a natural mode of switching.
3. Make parallel human and AI agent workflows easy to manage.
4. Provide a high-signal view of available workspaces.
5. Stay fully compatible with native `jj` behavior.
6. Remain small, safe, and easy to reason about.

## 3. Non-Goals

The product must not try to become a general workflow platform.

Out of scope for the product core:

- implementing new VCS behavior
- replacing Jujutsu commands
- managing Git branches directly
- managing Git worktrees
- modifying Jujutsu internals
- merge workflows
- pull request workflows
- CI integration
- interactive TUI interfaces
- build cache sharing

Some adjacent workflow features may be considered later, but `jj-navi` should stay navigation-first.

## 4. Target Users

Primary users:

- developers using Jujutsu daily
- developers running multiple parallel tasks
- developers using AI coding agents in separate workspaces
- users moving from Git worktree workflows
- users who want a Worktrunk-style workspace UX for `jj`

## 5. Product Philosophy

`jj-navi` should feel obvious.

- `switch` is the center of the UX
- creating a workspace is a mode of switching, not a separate mental model
- workspace names are first-class navigation handles
- paths should be deterministic and boring
- behavior should be easy to predict from reading the command line alone
- path, switching, and discovery friction should be hidden behind a small set of obvious commands
- human and agent workflows should both feel first-class

Example workflow:

```sh
navi switch --create feature-auth
navi switch fix-pagination
navi list
navi remove feature-auth
```

## 6. Command Model

Primary command model:

- `navi switch <workspace>`
- `navi switch --create <workspace>`
- `navi switch --create <workspace> --revision <revset>`
- `navi list`
- `navi remove <workspace>`

Shorthand binary:

- `nv`

The product UX centers `switch`, like Worktrunk. Creating a workspace is a mode of switching, not a separate command family.

## 7. Compatibility Rules

Minimum supported `jj` version:

- `0.39.0`

Why this is explicit:

- `jj` workspace behavior has evolved materially across releases
- `jj-navi` is specifically about workspaces, so it should target a modern workspace baseline instead of smoothing over every earlier version difference
- the PRD should make this version floor obvious to both humans and agents

Relevant workspace milestones in `jj`:

- `0.7.0` - `jj workspace root` exists
- `0.35.0` - `jj git colocation enable|disable` lands
- `0.38.0` - `jj workspace root --name` lands
- `0.39.0` - `jj workspace add` links workspaces with relative paths

Compatibility expectations:

- use `jj workspace add --name ...`
- use `jj workspace list -T ...`
- assume post-`0.39.0` workspace behavior in product design
- prefer reading state from stable `jj` command output over relying on internal storage formats unless there is no better option

This version floor does not change the product goal. `jj-navi` should still make `jj` workspaces feel as close as possible to lightweight, navigable worktrees for humans and coding agents.

## 8. Architecture Principles

The architecture should stay stable even as file layout evolves.

### Core boundaries

- binary entrypoints stay thin
- CLI parsing and dispatch should have one source of truth
- command handlers should orchestrate, not own repo mechanics
- repo/discovery logic should live behind a dedicated repo layer
- output formatting should stay separate from repo logic
- domain constraints should be expressed in typed values where practical

### Do

- keep `switch` primary
- keep binary wrappers tiny
- keep one shared CLI entry
- isolate workspace discovery and path planning
- derive repository state from `jj`
- store only `navi`-specific metadata
- favor real-`jj` integration tests
- preserve a `jj-ryu`-style separation of concerns

### Don’t

- add a standalone `create` command
- duplicate CLI definitions across binaries
- mix output formatting with repo operations
- reimplement workspace internals
- overfit to unstable `jj` internal storage for core features
- store duplicate state that can already be derived from `jj`

## 9. Current Code Reference Points

The PRD should not depend on one frozen file tree, but the current repository is organized around these landmarks:

- `src/main.rs` - `navi` binary entrypoint
- `src/bin/nv.rs` - `nv` shorthand binary entrypoint
- `src/cli/` - command handlers
- `src/repo/` - workspace discovery, path planning, and `jj` interaction
- `src/types.rs` - typed domain values
- `src/error.rs` - typed app errors
- `src/output.rs` - CLI formatting
- `tests/common/` - real `jj` integration harness

The exact files may change over time. The important part is preserving the architectural boundaries and do/don't rules from Section 8.

## 10. Workspace Discovery

The CLI must work from any directory inside a workspace.

Workspace discovery algorithm:

1. Walk up directories until `.jj` is found.
2. Treat that directory as the current workspace root.
3. Resolve repository storage from `.jj/repo`.

In multi-workspace repos, `.jj/repo` may be a pointer file.

Important rule:

- resolve relative `.jj/repo` pointer paths relative to `.jj/`, not the current working directory

This is a correctness requirement.

## 11. Workspace Semantics and Ergonomics

Jujutsu workspaces are separate working directories backed by shared repo storage.

Important practical facts:

- each workspace has its own working-copy state
- multiple workspaces can share one underlying repo store
- a secondary workspace may point to shared repo storage through `.jj/repo`
- a workspace can become stale if its working-copy commit is rewritten from another workspace
- secondary workspaces may not always behave like ordinary Git worktrees to every tool or agent

This matters because the core purpose of `jj-navi` is not just path generation. It is to make Jujutsu workspaces usable and navigable for parallel human and AI agent workflows despite this underlying complexity.

`jj-navi` should hide as much of this friction as possible through navigation UX, shell integration, and clear workspace visibility. It should not pretend these upstream semantics do not exist.

## 12. Path Template System

Workspace paths are generated from a deterministic template.

Default template:

```text
../{repo}.{workspace}
```

Supported variables:

- `{repo}`
- `{workspace}`

Example:

```text
repo = api-server
workspace = feature-auth
path = ../api-server.feature-auth
```

Rules:

- the same workspace name should always resolve to the same path
- path generation should be deterministic
- invalid workspace names should be rejected early

## 13. Native `jj` Primitives

`jj-navi` should build on existing `jj` commands.

Core primitives:

```text
jj workspace add
jj workspace list
jj workspace forget
jj workspace root
```

Later versions may use revsets like:

```text
<workspace>@
```

for workspace-specific commit/status views.

## 14. Metadata Contract

`jj-navi` metadata belongs in shared Jujutsu repo storage.

Location:

```text
.jj/repo/navi/
```

Workspace metadata file:

```text
.jj/repo/navi/workspaces.toml
```

Illustrative schema:

```toml
[[workspace]]
name = "feature-auth"
path = "../repo.feature-auth"
created_by_navi = true
created_at = "2026-03-10T12:00:00Z"
template = "../{repo}.{workspace}"
revision = "main"
```

Rules:

- store only metadata that is specific to `navi`
- derive repository truth from `jj`
- avoid duplicating state that can become stale

## 15. Configuration Contract

Repo-scoped config should live at:

```text
.jj/repo/navi/config.toml
```

Planned config keys:

```toml
workspace_template = "../{repo}.{workspace}"
remove.delete_dir_by_default = false
list.show_absolute_paths = false
shell.auto_detect = true
```

Config should be additive and unsurprising.

## 16. Hook Contract

Planned minimal hooks:

- `post-create`
- `post-switch`
- `pre-remove`

Hooks are shell commands.

Planned environment variables:

```text
NAVI_WORKSPACE
NAVI_WORKSPACE_PATH
NAVI_REPO_ROOT
```

Example:

```toml
post-create = "npm install"
```

Hooks must remain optional and lightweight.

## 17. Command Specifications

### `switch`

```sh
navi switch <workspace>
```

Behavior:

1. discover current workspace
2. derive repo name
3. compute target path from template
4. if workspace exists, print the target path for shell usage
5. if it does not exist, return an error

Error shape:

```text
error: workspace does not exist
hint: use --create
```

### `switch --create`

```sh
navi switch --create <workspace>
```

Behavior:

```sh
jj workspace add --name <workspace> <path>
```

Optional revision:

```sh
navi switch --create <workspace> --revision <revset>
```

Behavior:

```sh
jj workspace add --name <workspace> -r <revset> <path>
```

### `list`

```sh
navi list
```

v0 output:

- `workspace`
- `path`

Target later output:

- marker
- workspace
- path
- commit
- message
- status

Illustrative later output:

```text
@ feature-auth  ../repo.feature-auth   a1b2c3d  Add authentication
  bugfix-api    ../repo.bugfix-api     d9f7e1a  Fix pagination
```

### `remove`

```sh
navi remove <workspace>
```

Default target if omitted:

- current workspace

Base behavior:

```sh
jj workspace forget <workspace>
```

Planned flags:

- `--delete-dir`
- `--force`

Safety rules:

- refuse deletion if the directory appears unsafe
- require `--force` for destructive removal when appropriate

### `config shell install`

```sh
navi config shell install
```

Goal:

- make `switch` actually change directories in supported shells

Planned supported shells:

- bash
- zsh
- optional fish later

## 18. Testing Strategy

Tests should primarily use real `jj`.

Coverage areas:

- workspace discovery
- `.jj/repo` pointer resolution
- multi-workspace behavior
- `switch`
- `switch --create`
- `switch --create --revision`
- `list` formatting
- `remove`
- metadata consistency
- hooks behavior
- shell integration generation

Testing philosophy:

- prefer integration tests that exercise real `jj`
- keep mock-heavy tests secondary
- test behavior, not internal implementation details

## 19. Acceptance Criteria

### Global product criteria

The product is healthy when:

1. workspace paths are deterministic
2. the CLI remains compatible with native `jj`
3. navigation stays fast and predictable
4. repo state is derived from `jj`, not shadowed by `navi`
5. binary entrypoints remain thin and shared logic stays centralized

### v0 acceptance criteria

1. `navi switch --create feature-x` creates a workspace and prints the target path.
2. `navi switch feature-x` prints the path to an existing workspace.
3. `navi list` shows a readable table with workspace and path.
4. workspace discovery works from nested directories.
5. workspace paths follow deterministic template rules.
6. `navi` and `nv` both work.

### v1 acceptance criteria

1. shell integration exists for bash and zsh.
2. `navi switch` can actually change directories through shell integration.
3. `navi remove` forgets a workspace safely.
4. `navi` metadata exists under `.jj/repo/navi/`.
5. repo-scoped config exists under `.jj/repo/navi/config.toml`.
6. `list` output includes richer workspace information.

### v2 acceptance criteria

1. `navi list` gives useful observability across many workspaces.
2. `navi doctor` identifies broken or suspicious workspace state.
3. `navi prune` can safely clean obsolete workspace records and directories.
4. workspace health/status reporting remains easy to read.

### v3 acceptance criteria

1. hooks run predictably with documented env vars.
2. batch workspace creation is possible.
3. automation features stay optional and do not distort the core navigation model.
4. `jj-navi` still acts as a UX layer, not a replacement VCS workflow engine.

## 20. Version Roadmap

### v0 - Minimal Navigator

Goal:

- validate the workspace navigation model

Features:

- deterministic path resolution
- `switch`
- `switch --create`
- `switch --create --revision`
- `list`
- `nv` shorthand binary

Non-goals:

- shell integration
- metadata storage
- hooks
- directory deletion
- advanced workspace status

### v1 - Core Workspace UX

Goal:

- deliver production-ready navigation

Features:

- shell integration
- `remove`
- repo-scoped config
- metadata storage
- improved `list` output

Non-goals:

- workspace diagnostics
- automation workflows
- agent launching

### v2 - Workspace Observability

Goal:

- make many parallel workspaces manageable

Features:

- richer `list`
- workspace health checks
- `navi doctor`
- `navi prune`

Non-goals:

- workflow automation
- PR tooling

### v3 - Workflow Automation

Goal:

- enable optional automation on top of the navigation model

Features:

- hooks
- workspace presets
- command execution after switch
- batch workspace creation

Non-goals:

- replacing native `jj`
- changing Jujutsu semantics

## 21. Current Implementation Status

Current target version:

- `0.0.1-alpha.1`

Target `jj` baseline:

- `0.39.0`

Current implemented scope:

- this repo currently implements v0

Implemented today:

- Rust crate with shared library + binaries
- binaries: `navi`, `nv`
- deterministic path planning using `../{repo}.{workspace}`
- workspace discovery by walking up to `.jj`
- `.jj/repo` pointer resolution, including relative pointers
- current workspace detection via `jj workspace list -T ...`
- repo name derivation that works in default and secondary workspaces
- `switch`
- `switch --create`
- `switch --create --revision`
- minimal `list`
- real `jj` integration tests
- npm scaffold with matching version
- GitHub test and release scaffolding

Known v0 limits:

- no shell integration yet
- no `remove` yet
- no metadata/config/hooks yet
- `list` currently renders deterministic template-derived paths for non-current workspaces

Current code note:

- some current code paths are intentionally conservative and may work on older `jj` installs, but the product baseline going forward is `0.39.0`

## 22. Current Test Coverage

Current tests cover:

- invalid workspace name validation
- output table rendering
- relative `.jj/repo` pointer handling
- switching to an existing workspace
- creating a workspace
- creating with explicit revision
- listing workspaces
- nested-dir discovery from secondary workspaces
- `nv` shorthand behavior
- `navi --help` naming
- `nv --help` naming

## 23. References

Reference repositories and docs:

- Jujutsu: <https://github.com/jj-vcs/jj>
- Jujutsu CLI reference: <https://docs.jj-vcs.dev/latest/cli-reference/>
- Jujutsu working copy docs: <https://docs.jj-vcs.dev/latest/working-copy/>
- Jujutsu revsets: <https://github.com/jj-vcs/jj/blob/main/docs/revsets.md>
- Jujutsu Git/GitHub docs: <https://docs.jj-vcs.dev/latest/github/>
- Worktrunk: <https://github.com/max-sixty/worktrunk>
- Worktrunk docs: <https://worktrunk.dev>
- jj-ryu: <https://github.com/dmmulroy/jj-ryu>

Relevant upstream workspace history and ergonomics threads:

- Multiple working copies origin: <https://github.com/jj-vcs/jj/issues/13>
- Retrieve another workspace path: <https://github.com/jj-vcs/jj/issues/6854>
- `jj workspace root --name` edge cases: <https://github.com/jj-vcs/jj/issues/8758>
- Tracking issue for colocated workspaces: <https://github.com/jj-vcs/jj/issues/8052>
- Colocated repos with multiple workspaces discussion: <https://github.com/jj-vcs/jj/discussions/7470>
- `v0.35.0` release discussion: <https://github.com/jj-vcs/jj/discussions/7956>
