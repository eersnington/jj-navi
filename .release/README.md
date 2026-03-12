# Release Fragments

Add one fragment for every user-facing change.

More maintainer release docs live in `xtask/README.md`.

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

Defaults:

- no bump arg means `patch`
- pass `minor` or `major` only when needed
- run `navi-release` with no args for an interactive wizard

Fragment format:

```md
---
bump: patch
scope: cli
---
- fix nested workspace discovery
- improve `nv` parity with `navi`
```

Rules:

- `bump` must be `patch`, `minor`, or `major`
- `scope` is optional
- body bullets become changelog and GitHub release notes
- fragments are deleted by the release-prepare workflow after they are rolled into a release PR
