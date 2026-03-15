# jj-navi

## OVERVIEW

`jj-navi` is workspace management for Jujutsu, built for parallel human and AI agent workflows.

- It makes JJ workspaces easier to create, switch, inspect, and clean up.
- Workspace workflows should feel simple, predictable, and low-friction.
- `jj` is the source of truth for repo and workspace state.
- `jj-navi` stores only additive repo-scoped config and metadata under shared JJ storage.
- CLI UX should feel deliberate, calm, and exact.

## STRUCTURE

```text
src/
├── main.rs + bin/nv.rs   # thin binary entrypoints
├── app.rs                # clap parsing and top-level dispatch
├── cli/                  # command handlers
├── repo/                 # JJ integration, discovery, config, metadata
├── output.rs             # human/json/shell rendering
├── doctor.rs             # typed diagnostics model
├── types.rs              # validated domain and presentation types
└── error.rs              # crate error type
tests/
├── integration_tests.rs  # real jj integration coverage
├── unit_tests.rs         # pure black-box tests only
└── common/               # TempJjRepo harness
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add or change a command | `src/app.rs`, `src/cli/` | Keep command handlers thin |
| JJ-facing behavior | `src/repo/` | Discovery, validation, JJ calls, repo-scoped state |
| Output changes | `src/output.rs` | Keep rendering separate from repo logic |
| Domain rules | `src/types.rs`, `src/error.rs`, `src/doctor.rs` | Validate at boundaries, keep types explicit |
| Real repo behavior tests | `tests/integration_tests.rs`, `tests/common/` | Prefer these over mocks |

## ARCHITECTURE

- Command handlers orchestrate. Repo logic belongs in `src/repo/`. Rendering belongs in `src/output.rs`.
- Commands must work from nested directories inside a workspace, not only the workspace root.
- `navi` and `nv` should behave the same apart from the displayed command name.
- Preserve existing path-recovery behavior unless intentionally redesigning it.
- Metadata record presence and metadata path availability are distinct states. Never treat missing path data as proof that metadata is missing.
- Do not duplicate JJ-owned repo state in `navi` metadata when `jj` can answer it directly.

## JJ RESEARCH RULES

- Always do VCS detection first. If this repo is using `jj`, use `jj`, not `git`.
- Before changing any JJ-facing behavior, inspect relevant `jj-vcs/jj` issues and PRs and surface them in chat so the user can review it too.
- Treat JJ as moving software. Do not rely on memory for semantics, limitations, or edge cases.
- When making JJ-related decisions, anchor reasoning in upstream issue/PR IDs, not vague recollection.
- If upstream behavior is unclear or unstable, choose the conservative behavior and preserve graceful degradation.

## RUST CONVENTIONS

- Make minimal, surgical changes.
- Prefer explicit types and validated domain values at boundaries.
- Make illegal states hard or impossible to represent.
- Prefer borrowing over cloning when ownership transfer is not needed.
- Use `Result` for fallible paths. Do not introduce panics into production code.
- Use typed errors with `thiserror`; keep error messages precise and actionable.
- Add comments only for non-obvious invariants, safety, or why a choice exists.
- Avoid speculative abstractions, dead helpers, and convenience wrappers with weak ownership.
- Keep public docs and CLI wording concise, accurate, and stable.

## ANTI-PATTERNS

- Do not use metadata path lookup as a proxy for metadata record existence.
- Do not bypass the repo layer from CLI handlers.
- Do not trust JJ-reported paths blindly; validate them against repo/workspace reality.
- Do not paper over JJ semantics with ad hoc heuristics when the model can be made explicit.
- Do not add noisy or gimmicky CLI output.
- Do not add mocks when a real-`jj` integration test is practical.
- Do not add unit tests for private plumbing just because it is easy.

## LINTS

- `unsafe` stays forbidden.
- Code should pass `cargo clippy --all-targets -- -D warnings`.
- Treat lint suppressions as exceptional; scope them tightly and justify them.
- When README.md changes, remember to run `node npm/scripts/sync-wrapper-readme.mjs` as this is a CI requirement.

## COMMANDS

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

## TESTING

- Prefer well-designed integration or end-to-end tests over narrow white-box unit tests.
- If behavior depends on JJ semantics, use real `jj` integration tests.
- Unit tests are for pure black-box behavior worth isolating: validation, formatting, and small deterministic transforms.
- Do not unit-test implementation details, private helper shape, or incidental control flow.
- Deterministic CLI output is part of the contract.
- When relevant, test both binary names: `navi` and `nv`.
