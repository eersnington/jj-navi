# jj-navi PRD

## 1. Product in One Sentence

`jj-navi` is a small Rust CLI that makes Jujutsu workspaces fast to create, switch, inspect, and clean up for parallel human and AI-agent workflows.

It is a navigation and workspace-visibility layer over native `jj workspace` primitives.
It is not a replacement VCS workflow engine.

## 2. Current Product Phase

Current phase:

- still rolling out v2

What that means:

- v1 established the core workspace navigator
- v2 should finish the core day-to-day workflow around `switch`, `list`, and safe cleanup
- v3 can add optional automation such as hooks and merge-oriented workflows

This PRD intentionally recenters the roadmap away from side quests.
`doctor` matters, but it is not the main product.
`prune` is not currently implemented and is not part of the near-term core.

## 3. Core Product Thesis

The main promise of `jj-navi` is:

```text
workspace name -> trusted path -> fast navigation
```

That promise only works if three things are true:

1. switching is obvious and low-friction
2. workspace paths are trustworthy even when JJ metadata is incomplete or stale
3. users can understand workspace state quickly without learning JJ internals

## 4. Product Goals

1. Make switching Jujutsu workspaces fast and predictable.
2. Make creating a workspace feel like a mode of switching, not a separate concept.
3. Make many parallel workspaces understandable at a glance.
4. Support parallel human and AI-agent workflows without hiding JJ semantics.
5. Stay conservative around JJ edge cases and path ambiguity.
6. Stay small, type-safe, and easy to reason about.

## 5. Non-Goals

Out of scope for the product core:

- implementing new VCS behavior
- replacing `jj` commands in general
- managing Git branches directly
- managing Git worktrees
- modifying Jujutsu internals
- pull request workflows in v2
- CI orchestration in v2
- interactive TUI-first UX in v2
- broad automation before the navigation loop is complete
- speculative command families copied from Worktrunk without a JJ-native reason

## 6. Primary Users

Primary users:

- developers using Jujutsu daily
- developers running multiple parallel tasks
- developers using AI coding agents in separate workspaces
- users migrating from Git worktree workflows
- users who want a Worktrunk-style navigation UX for `jj`

## 7. Product Philosophy

`jj-navi` should feel obvious from the command line alone.

Core philosophy:

- `switch` is the center of the UX
- creation is a mode of switching
- `list` is the main visibility command
- `doctor` is a support command, not the headline workflow
- workspace names are first-class navigation handles
- paths should be deterministic and boring
- metadata are additive fallback state, never the source of truth
- destructive actions should stay conservative while JJ semantics remain subtle

## 8. Core User Workflow

The product is healthy when the main loop feels natural:

```sh
navi switch --create feature-auth
navi switch -
navi switch @
navi list
navi remove feature-auth
```

The intended mental model is:

1. create or jump with `switch`
2. inspect with `list`
3. diagnose weirdness only when needed with `doctor`
4. clean up safely with `remove`

## 9. Worktrunk Inspiration vs jj-navi Scope

`jj-navi` is inspired by Worktrunk, but it should not try to clone Worktrunk wholesale.

What to borrow aggressively:

- `switch` as the primary command
- creation as a mode of switching
- strong `list` ergonomics
- safe cleanup ergonomics
- explicit support for parallel agent workflows

What to borrow carefully, later, or not at all:

- merge automation
- broad step-command families
- PR and CI integration
- interactive picker UX
- Git-specific branch/worktree assumptions

Worktrunk influences product shape.
JJ constraints determine final behavior.

## 10. JJ Constraints and Upstream Behaviour to Know About

JJ workspace behavior is still evolving.
`jj-navi` must remain conservative where upstream semantics are still subtle.

Key rules:

- always prefer stable `jj` commands over internal storage formats
- never treat missing `navi` metadata as proof that a workspace does not exist
- never treat a recorded path as trustworthy until it is validated against the repo and workspace identity
- preserve graceful degradation when JJ path lookup is incomplete or stale

Relevant workspace milestones in `jj`:

- `0.7.0` - `jj workspace root` exists
- `0.35.0` - `jj git colocation enable|disable` lands
- `0.38.0` - `jj workspace root --name` lands
- `0.39.0` - `jj workspace add` links workspaces with relative paths

Minimum supported `jj` version:

- `0.39.0`

This is explicit because `jj-navi` is about workspace semantics, not generic CLI wrapping.

## 11. Upstream JJ Issues That Matter

These upstream issues and discussions remain important design context and must stay visible in this PRD:

- Multiple working copies origin: <https://github.com/jj-vcs/jj/issues/13>
- Retrieve another workspace path: <https://github.com/jj-vcs/jj/issues/6854>
- `jj workspace root --name` edge cases: <https://github.com/jj-vcs/jj/issues/8758>
- Tracking issue for colocated workspaces: <https://github.com/jj-vcs/jj/issues/8052>
- Colocated repos with multiple workspaces discussion: <https://github.com/jj-vcs/jj/discussions/7470>
- `v0.35.0` release discussion: <https://github.com/jj-vcs/jj/discussions/7956>

Design implications:

- path lookup for non-current workspaces exists, but stale or missing path records still happen
- the default or primary workspace remains semantically special in some recovery paths
- fallback logic is necessary today
- fallback logic must stay validated and conservative

## 12. Current Command Model

Current shipping command families:

- `navi switch <workspace>`
- `navi switch --create <workspace>`
- `navi switch --create <workspace> --revision <revset>`
- `navi list`
- `navi doctor`
- `navi remove <workspace>`
- `navi config shell init <bash|zsh>`
- `navi config shell install [--shell <bash|zsh>]`

Shorthand binary:

- `nv`

## 13. Planned Command Direction

### v2 command priorities

These are the command additions or refinements that best complete the core workflow:

- `navi switch -` - switch to the previous workspace
- `navi switch @` - resolve the current workspace explicitly
- `navi list --json` - structured output for shells and agents
- `navi remove --delete-dir` - optional deletion of a validated non-current workspace directory

### v3 command priorities

These are valuable, but they belong after the core navigation loop is solid:

- `navi merge` - merge-oriented workspace completion flow
- lifecycle hooks
- `navi switch -x <cmd>` or equivalent execute-after-switch support
- optional presets or batch creation workflows

### Explicitly deferred

- interactive picker in v2
- PR shortcuts like `pr:123` in v2
- broad `step` command family in v2
- statusline and marker systems in v2

## 14. Core Behavioral Rules

### `switch`

`switch` is the center of the UX.

Rules:

- workspace names are validated early
- existing workspaces resolve through the strongest trustworthy path source
- if the workspace does not exist, `switch` fails unless `--create` is set
- creation remains part of `switch`, not a separate top-level `create` command
- shell integration should allow `switch` to change directories directly when installed
- without shell integration, `switch` prints the path to stdout

Planned v2 additions:

- `switch -` uses repo-scoped previous-workspace state
- `switch @` resolves the current workspace explicitly

Not planned yet:

- `switch ^` until JJ main/default workspace semantics are modeled cleanly enough

### `list`

`list` is the visibility command.

Rules:

- it should be useful before `doctor`
- it should show enough signal to answer “what is going on?” quickly
- it should surface degraded path states inline instead of failing the whole command
- JSON output should exist for scripts and agents

### `doctor`

`doctor` is a support command.

Rules:

- it explains weird or degraded repo state
- it should reuse the same underlying health model that powers `list`
- it should not become the primary day-to-day user story

### `remove`

`remove` is the safe cleanup command.

Rules:

- explicit workspace name required
- must refuse to remove the current workspace
- forget-only remains the default behavior
- any directory deletion must be explicit and validated

## 15. Workspace Discovery and Path Recovery

The CLI must work from any directory inside a workspace.

Discovery algorithm:

1. walk up directories until `.jj` is found
2. treat that directory as the current workspace root
3. resolve shared repo storage from `.jj/repo`
4. if `.jj/repo` is a pointer file, resolve relative to `.jj/`, not the current working directory

This is a correctness requirement.

## 16. Workspace Path Resolution Strategy

For non-current workspaces, path lookup should use the strongest validated source in order:

1. JJ-recorded workspace path
2. validated repo-primary root for the default workspace
3. validated `navi` metadata path
4. validated deterministic template path

Important rules:

- the current workspace path comes from local discovery, not fallback heuristics
- every non-current candidate path must be validated before trusted use
- `list` may show degraded rows inline
- `switch` must only navigate to paths that validate as the requested workspace in the current repo

## 17. Path Template System

Workspace paths are generated from a deterministic template.

Default template:

```text
../{repo}.{workspace}
```

Supported variables:

- `{repo}`
- `{workspace}`

Rules:

- same workspace name should resolve to the same planned path
- path generation should be deterministic
- invalid workspace names should be rejected early
- template support must stay intentionally small unless stronger use cases appear

## 18. Metadata Contract

`jj-navi` metadata belong in shared Jujutsu repo storage.

Locations:

```text
.jj/repo/navi/config.toml
.jj/repo/navi/workspaces.toml
```

Metadata principles:

- store only `navi`-specific data
- derive repo truth from `jj`
- metadata record presence and metadata path availability are distinct states
- a missing stored path does not mean the metadata record is missing
- metadata should support fallback recovery, not shadow JJ state

Illustrative workspace metadata shape:

```toml
[[workspace]]
name = "feature-auth"
path = "../repo.feature-auth"
created_by_navi = true
created_at = "2026-03-10T12:00:00Z"
template = "../{repo}.{workspace}"
revision = "main"
```

## 19. Repo-Scoped State Beyond Metadata

v2 should introduce minimal repo-scoped navigation state for the core workflow.

Planned state:

- previous workspace pointer for `switch -`

Rules:

- repo-scoped, not shell-session-scoped
- updated only after successful switches to a different workspace
- intended to behave like a workspace-level equivalent of `cd -`
- should stay minimal and explicit

## 20. Shell Integration Contract

Shell integration exists so `switch` can actually change directories.

Current supported shells:

- bash
- zsh

Current behavior:

- `config shell init` prints a wrapper script
- `config shell install` installs one managed block in the shell rc file
- `switch` writes shell-safe `cd` directives when integration is active
- otherwise `switch` prints the destination path

Fish is deferred.

## 21. Architecture Principles

The architecture should stay stable even if files move.

Core boundaries:

- binary entrypoints stay thin
- CLI parsing and dispatch have one source of truth
- command handlers orchestrate, not own JJ mechanics
- repo logic lives behind `src/repo/`
- output formatting stays separate from repo logic
- typed domain constraints should exist at boundaries

Do:

- keep `switch` primary
- centralize path recovery in the repo layer
- favor real `jj` integration tests
- keep metadata additive
- keep non-obvious JJ behavior explicit in the model

Do not:

- add a top-level `create` command
- bypass the repo layer from CLI handlers
- trust JJ-reported paths blindly
- use metadata path lookup as a proxy for metadata record existence
- reimplement JJ workspace internals
- bloat the CLI surface before the core loop is complete

## 22. Current Code Reference Points

Current code landmarks:

- `src/main.rs` - `navi` binary entrypoint
- `src/bin/nv.rs` - `nv` shorthand binary entrypoint
- `src/app.rs` - CLI parsing and dispatch
- `src/cli/` - command handlers
- `src/repo/` - workspace discovery, path planning, metadata, and JJ integration
- `src/types.rs` - typed domain and render-ready values
- `src/error.rs` - typed app errors
- `src/output.rs` - CLI rendering and shell directive helpers
- `src/doctor.rs` - typed diagnostics model
- `tests/common/` - real `jj` integration harness

## 23. Current Implementation Status

Current target crate version:

- `0.2.0`

Current implemented scope:

- repo discovery from nested paths
- `.jj/repo` pointer resolution including relative pointers
- JJ version floor enforcement
- deterministic workspace path planning
- `switch`
- `switch --create`
- `switch --create --revision`
- path recovery for missing JJ workspace-path records
- `list` with marker, statuses, path, commit, and message
- `doctor`
- `remove` as safe forget-only cleanup
- repo-scoped config file creation
- workspace metadata storage
- shell integration for bash and zsh
- both `navi` and `nv`
- real `jj` integration tests

Not implemented yet:

- `switch -`
- `switch @`
- `list --json`
- `remove --delete-dir`
- hooks
- merge workflow
- fish shell support
- cross-workspace action/status view beyond current path-health table

## 24. v2 Scope

### v2 goal

Finish the core navigation workflow.

### v2 themes

- faster switching
- stronger list output
- safer but more complete cleanup
- support for shells and agents through stable machine-readable output

### v2 features

1. `switch -` previous-workspace support
2. `switch @` current-workspace alias
3. `list --json`
4. better alignment between `list` and `doctor` health models
5. optional `remove --delete-dir` for validated non-current workspaces
6. documentation and onboarding that center `switch` and `list`, not `doctor`

### v2 non-goals

- hooks
- merge automation
- batch workspace creation
- PR or CI integration
- interactive picker

## 25. v3 Scope

### v3 goal

Add optional automation on top of trusted navigation primitives.

### v3 likely features

1. `merge`
2. lifecycle hooks
3. execute-after-switch support
4. optional presets or batch creation

### v3 guardrail

Automation must compose on top of `switch`, `list`, and `remove`.
It must not distort the product into a parallel replacement for JJ itself.

## 26. Acceptance Criteria

### Global criteria

The product is healthy when:

1. workspace paths are deterministic
2. switching is fast and predictable
3. degraded path state is surfaced clearly instead of hidden
4. repo truth is derived from `jj`, not duplicated by `navi`
5. CLI behavior remains conservative when JJ semantics are ambiguous
6. command handlers remain thin and shared logic stays centralized

### v2 acceptance criteria

1. `navi switch -` returns to the previous workspace using repo-scoped state.
2. `navi switch @` resolves the current workspace explicitly.
3. `navi list --json` emits structured machine-readable workspace data.
4. `list` and `doctor` use the same underlying path-health model.
5. `remove --delete-dir` deletes only a validated, explicit, non-current workspace directory.
6. README and docs present `switch` and `list` as the primary user workflow.

### v3 acceptance criteria

1. `merge` provides a clear, JJ-compatible workspace completion flow.
2. hooks run predictably with documented lifecycle points.
3. execute-after-switch behavior composes safely with shell integration.
4. automation remains optional and does not obscure JJ semantics.

## 27. Testing Strategy

Testing should primarily use real `jj`.

Coverage priorities:

- nested workspace discovery
- `.jj/repo` pointer resolution
- switching existing workspaces
- creating workspaces
- switching with degraded JJ path metadata
- previous-workspace state
- `switch @`
- `list` human output and JSON output
- safe cleanup behavior
- shell directive generation
- shell install block management
- `navi` and `nv` parity

Testing philosophy:

- prefer integration tests for JJ-facing behavior
- keep unit tests focused on deterministic logic you own
- test behavior, not implementation details

## 28. Current Test Coverage Snapshot

Current tests already cover:

- invalid workspace name validation
- output rendering for list and doctor
- relative `.jj/repo` pointer handling
- switching existing workspaces
- creating workspaces
- creating with explicit revision
- workspace path fallback and warning behavior
- repo-scoped config creation and parse failures
- metadata write, cleanup, and malformed-state failures
- list path statuses such as inferred, missing, stale, and jj-only
- nested-dir discovery from secondary workspaces
- safe remove flows
- shell init rendering
- shell install managed block behavior
- shell directive emission and escaping
- `nv` shorthand behavior

## 29. Documentation Priorities

Docs should explain the product in this order:

1. what `jj-navi` is
2. the core workflow
3. how shell integration works
4. how `list` and `doctor` differ
5. JJ limitations and fallback behavior

Docs should avoid making `doctor` look like the center of the product.

## 30. References

Reference repositories and docs:

- Jujutsu: <https://github.com/jj-vcs/jj>
- Jujutsu CLI reference: <https://docs.jj-vcs.dev/latest/cli-reference/>
- Jujutsu working copy docs: <https://docs.jj-vcs.dev/latest/working-copy/>
- Jujutsu revsets: <https://github.com/jj-vcs/jj/blob/main/docs/revsets.md>
- Jujutsu Git/GitHub docs: <https://docs.jj-vcs.dev/latest/github/>
- Worktrunk: <https://github.com/max-sixty/worktrunk>
- Worktrunk docs: <https://worktrunk.dev>
- jj-ryu: <https://github.com/dmmulroy/jj-ryu>
