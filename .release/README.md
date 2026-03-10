# Release Fragments

Add one fragment for every user-facing change.

Create one with:

```sh
./scripts/release/new "fix nested workspace discovery" -s cli
```

Defaults:

- no bump arg means `patch`
- pass `minor` or `major` only when needed

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
