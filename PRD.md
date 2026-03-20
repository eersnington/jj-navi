# jj-navi PRD

## 1. Product in One Sentence

`jj-navi` is a small Rust CLI that makes Jujutsu workspaces fast to create, switch, inspect, and clean up for parallel human and AI-agent workflows.

It is a navigation and workspace-visibility layer over native `jj workspace` primitives.
It is not a replacement VCS workflow engine.

## 2. Current Product Phase

Current phase:

- still rolling out v2 around the human + agent parallel workspace loop

What that means:

- v1 established the core workspace navigator
- v2 should finish the core day-to-day workflow around `switch`, `list`, and agent launch
- v3 can add optional automation and richer parallel-workspace visibility

## 3. Core Product Thesis

The main promise of `jj-navi` is:

```text
workspace name -> trusted path -> fast navigation
```

That promise only works if three things are true:

1. switching is obvious and low-friction
2. workspace paths are trustworthy even when jj metadata is incomplete or stale
3. users can understand workspace state quickly without learning jj internals

## 4. Product Goals

1. Make switching Jujutsu workspaces fast and predictable.
2. Make creating a workspace feel like a mode of switching, not a separate concept.
3. Make many parallel workspaces understandable at a glance.
4. Support parallel human and AI-agent workflows without hiding jj semantics.
5. Make parallel human and AI-agent workflows first-class without requiring custom shell glue.
6. Stay conservative around jj edge cases and path ambiguity.
7. Stay small, type-safe, and easy to reason about.

## 5. Non-Goals

Out of scope for the product core:

- implementing new VCS behavior
- replacing `jj` commands in general
- managing Git branches directly
- managing Git worktrees
- modifying Jujutsu internals
- pull request workflows in v2
- interactive TUI-first UX in v2
- broad automation before the navigation loop is complete
- speculative command families copied from Worktrunk without a jj-native reason

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
- agent launch should compose directly with switching
- workspace names are first-class navigation handles
- paths should be deterministic and boring
- metadata are additive fallback state, never the source of truth
- destructive actions should stay conservative while jj semantics remain subtle

## 8. Core User Workflow

The product is healthy when the main loop feels natural:

```sh
navi switch --create feature-one
navi switch -c -x opencode feature-two
navi switch -
navi switch @
navi list
navi remove feature-auth
```

The intended mental model is:

1. create or jump with `switch`
2. optionally launch an agent as part of switching
3. inspect with `list`
4. diagnose weirdness only when needed with `doctor` (ideally never have to use it)
5. clean up safely with `remove`

## 9. Worktrunk Inspiration vs jj-navi Scope

`jj-navi` is inspired by `Worktrunk`, but it should not try to clone Worktrunk.

What ideas to borrow:

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
jj constraints determine final behavior.

## 10. jj Constraints and Upstream Behaviour to Know About

jj-vcs is beta software (but generally considered stable), but `jj workspace` behavior is still evolving.
`jj-navi` must remain conservative where upstream semantics are still subtle.

Key rules:

- always prefer stable `jj` commands over internal storage formats
- never treat missing `navi` metadata as proof that a workspace does not exist
- never treat a recorded path as trustworthy until it is validated against the repo and workspace identity
- preserve graceful degradation when jj path lookup is incomplete or stale

Relevant workspace milestones in `jj`:

- `0.7.0` - `jj workspace root` exists
- `0.35.0` - `jj git colocation enable|disable` lands
- `0.38.0` - `jj workspace root --name` lands
- `0.39.0` - `jj workspace add` links workspaces with relative paths

Minimum supported `jj` version:

- `0.39.0`

This is explicit because `jj-navi` is about workspace semantics, not generic CLI wrapping.

## 11. Upstream jj Issues That Matter

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

## 13. Planned Direction

### v2 Scope

Finish the core navigation workflow.

- faster switching
- stronger list output
- direct agent launch from switching
- support for shells and agents through stable machine-readable output

#### v2 features

1. `switch -` previous-workspace support
2. `switch @` current-workspace alias
3. `list --json`
4. `list --full`
5. better alignment between `list` and `doctor` health models
6. `switch -x <cmd>` execute-after-switch support
7. documentation and onboarding that center `switch` and `list`, not `doctor`

#### v2 non-goals

- hooks
- batch workspace creation
- PR or CI integration
- interactive picker

### v3 Scope

Add explicit, opt-in workflow accelerators on top of trusted navigation primitives.

#### v3 features

1. lifecycle hooks
2. interactive picker
3. richer parallel workflow visibility including GitHub PR and CI enrichments
4. `navi switch pr:123` - switch to a same-repo GitHub pull request workspace
5. GitHub PR and CI enrichments in `list --full`

#### v3 guardrail

Automation must compose on top of `switch`, `list`, and `remove`.
It must not distort the product into a parallel replacement for jj itself.

### Future planned

- GitLab support for PR/MR shortcuts and CI enrichments
- LLM-generated commit messages and summaries
- jj-native completion or landing workflow after more workflow research
- optional presets or batch creation workflows
- Fish support

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
- `switch -x <cmd>` runs a command after successful switch or create

Not planned yet:

- `switch ^` until jj main/default workspace semantics are modeled cleanly enough

### `list`

`list` is the visibility command.

Rules:

- it should be useful before `doctor`
- it should show enough signal to answer “what is going on?” quickly
- it should surface degraded path states inline instead of failing the whole command
- JSON output should exist for scripts and agents
- richer visibility modes such as `--full` should stay explicit so default `list` remains fast

### `doctor`

`doctor` is a support command.

Rules:

- it explains weird or degraded repo state
- it should reuse the same underlying health model that powers `list`
- ideally, users should never have to use this

### `remove`

`remove` is the safe cleanup command.

Rules:

- explicit workspace name required
- must refuse to remove the current workspace
- forget-only remains the default behavior
- directory deletion is not part of the currently planned roadmap

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

1. jj-recorded workspace path
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
- metadata should support fallback recovery, not shadow jj state

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
- execute-after-switch behavior should compose cleanly with shell integration

## 21. Architecture Principles

The architecture should stay stable even if files move.

Core boundaries:

- binary entrypoints stay thin
- CLI parsing and dispatch have one source of truth
- command handlers orchestrate, not own jj mechanics
- repo logic lives behind `src/repo/`
- output formatting stays separate from repo logic
- typed domain constraints should exist at boundaries

Do:

- keep `switch` primary
- centralize path recovery in the repo layer
- favor real `jj` integration tests
- keep metadata additive
- keep non-obvious jj behavior explicit in the model

Do not:

- add a top-level `create` command
- bypass the repo layer from CLI handlers
- trust jj-reported paths blindly
- use metadata path lookup as a proxy for metadata record existence
- reimplement jj workspace internals
- bloat the CLI surface before the core loop is complete

## 22. Current Implementation Status

Current target crate version:

- `0.2.0`

Current implemented scope:

- repo discovery from nested paths
- `.jj/repo` pointer resolution including relative pointers
- jj version floor enforcement
- deterministic workspace path planning
- `switch`
- `switch --create`
- `switch --create --revision`
- `switch -`
- `switch @`
- path recovery for missing jj workspace-path records
- `list` with marker, statuses, path, commit, and message
- `list --json`
- `list --full`
- `doctor`
- shared health model across `list` and `doctor`
- `remove` as safe forget-only cleanup
- repo-scoped config file creation
- repo-scoped previous-workspace state
- workspace metadata storage
- shell integration for bash and zsh
- both `navi` and `nv`
- real `jj` integration tests

Not implemented yet:

- `switch -x <cmd>`
- hooks
- fish shell support
- GitHub PR shortcuts like `switch pr:123`
- GitHub PR/CI enrichments in `list --full`
- cross-workspace action/status view beyond current path-health table

## 23. Acceptance Criteria

### Global criteria

The product is healthy when:

1. workspace paths are deterministic
2. switching is fast and predictable
3. degraded path state is surfaced clearly instead of hidden
4. repo truth is derived from `jj`, not duplicated by `navi`
5. CLI behavior remains conservative when jj semantics are ambiguous
6. command handlers remain thin and shared logic stays centralized
7. parallel human and agent workflows feel first-class, not bolted on

### v2 acceptance criteria

1. `navi switch -` returns to the previous workspace using repo-scoped state.
2. `navi switch @` resolves the current workspace explicitly.
3. `navi list --json` emits structured machine-readable workspace data.
4. `navi list --full` provides richer explicit visibility without making default `list` noisy or slow.
5. `list` and `doctor` use the same underlying path-health model.
6. `navi switch -x <cmd>` executes only after successful destination resolution.
7. README and docs present `switch`, `switch -`, `switch @`, and `list` as the primary user workflow.

### v3 acceptance criteria

1. hooks run predictably with documented lifecycle points.
2. interactive switching remains optional and does not replace named switching.
3. richer parallel visibility helps users understand many active workspaces quickly.
4. GitHub PR shortcuts resolve same-repo PRs conservatively and predictably.
5. GitHub PR and CI enrichments stay explicit and do not slow default command paths.

## 24. Testing Strategy

Testing should primarily use real `jj`.

Coverage priorities:

- nested workspace discovery
- `.jj/repo` pointer resolution
- switching existing workspaces
- creating workspaces
- switching with degraded jj path metadata
- previous-workspace state
- `switch @`
- `switch -x`
- `list` human output and JSON output
- `list --full` enriched output
- safe cleanup behavior
- shell directive generation
- shell install block management
- `navi` and `nv` parity

Testing philosophy:

- prefer integration tests for jj-facing behavior
- keep unit tests focused on deterministic logic you own
- test behavior, not implementation details

## 25. Documentation Priorities

Docs should explain the product in this order:

1. what `jj-navi` is
2. the core workflow
3. how shell integration works
4. how to launch agents from `switch`
5. how to use `list`
6. how to debug using `doctor`
7. jj limitations and fallback behavior

## 26. References

Reference repositories and docs:

- Jujutsu: <https://github.com/jj-vcs/jj>
- Jujutsu CLI reference: <https://docs.jj-vcs.dev/latest/cli-reference/>
- Jujutsu working copy docs: <https://docs.jj-vcs.dev/latest/working-copy/>
- Jujutsu revsets: <https://github.com/jj-vcs/jj/blob/main/docs/revsets.md>
- Jujutsu Git/GitHub docs: <https://docs.jj-vcs.dev/latest/github/>
- Steve Klabnik's jj edit workflow: <https://steveklabnik.github.io/jujutsu-tutorial/real-world-workflows/the-edit-workflow.html>
- Steve Klabnik's jj squash workflow: <https://steveklabnik.github.io/jujutsu-tutorial/real-world-workflows/the-squash-workflow.html>
- Worktrunk: <https://github.com/max-sixty/worktrunk>
- Worktrunk docs: <https://worktrunk.dev>
- jj-ryu: <https://github.com/dmmulroy/jj-ryu>
