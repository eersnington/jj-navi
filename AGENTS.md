# jj-navi

## OVERVIEW

`jj-navi` is a CLI for workspace management for Jujutsu, built for parallel human and AI agent workflows.

- It makes JJ workspaces easier to create, switch, inspect, merge, and clean up.
- Workspace workflows should feel simple, predictable, and low-friction.
- `jj` is the source of truth for repo and workspace state.
- `jj-navi` stores only additive repo-scoped config and metadata under shared JJ storage.

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add or change a command | `src/cli/mod.rs`, `src/commands/` | Keep parsing and handlers thin |
| JJ-facing behavior | `src/repo/` | Discovery, validation, JJ calls, repo-scoped state |
| Output changes | `src/output/`, `src/diagnostics/render.rs`, `src/shell/` | Keep rendering separate from repo logic |
| Domain rules | `src/types.rs`, `src/error.rs`, `src/diagnostics/` | Validate at boundaries, keep types explicit |
| Real repo behavior tests | `tests/repo_integration_tests.rs`, `tests/common/` | Prefer these over mocks |

## JJ Rules

- Always do VCS detection first. If this repo is using `jj`, use `jj`, not `git`.
- Before changing any JJ-facing behavior, inspect relevant `jj-vcs/jj` issues and PRs and surface them in chat so the user can review it too.
- Treat JJ as moving software. Do not rely on memory for semantics, limitations, or edge cases.
- When making JJ-related decisions, anchor reasoning in upstream issue/PR IDs, not vague recollection.
- If upstream behavior is unclear or unstable, choose the conservative behavior and preserve graceful degradation.

## RUST CONVENTIONS

- `unsafe_code = "deny"` - no unsafe allowed
- No `.unwrap()` in production code (post-validation exceptions only).
- Don't add features to lib.rs unless interface-agnostic
- Prefer explicit types and validated domain values at boundaries.
- Make illegal states hard or impossible to represent.
- Prefer borrowing over cloning when ownership transfer is not needed.
- Use `Result` for fallible paths. Do not introduce panics into production code.
- Use typed errors with `thiserror`; keep error messages precise and actionable.
- Add comments only for non-obvious invariants, safety, or why a choice exists.
- Avoid speculative abstractions, dead helpers, and convenience wrappers with weak ownership.

## ANTI-PATTERNS

- Do not use metadata path lookup as a proxy for metadata record existence.
- Do not bypass the repo layer from CLI handlers.
- Do not trust JJ-reported paths blindly; validate them against repo/workspace.
- Do not paper over JJ semantics with ad hoc heuristics when the model can be made explicit.
- Do not add noisy or gimmicky CLI output.
- Do not add mocks when a real-`jj` integration test is practical.
- Do not add unit tests for private plumbing just because it is easy.
- Do not add inline tests. All tests in tests/ directory, no mod tests { } blocks.

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

**CI gotcha:** After any `README.md` change, run `node npm/scripts/sync-wrapper-readme.mjs` or CI will fail.

## TESTING

- Unit tests for pure black-box behavior only: validation, formatting, small deterministic transforms. Never test private plumbing or implementation shape.
- Prefer integration/e2e tests over unit tests for real system behavior. No mocks, ever.
- JJ-dependent behavior gets real `jj` integration tests (`tests/repo_integration_tests.rs`).
- No inline tests. All tests live in `tests/`. No `mod tests {}` blocks.
- Deterministic CLI output is part of the contract. When relevant, test both `navi` and `nv`.

## WHEN STUCK

Ask a clarifying question or propose a plan. Do not make large speculative changes without confirmation.