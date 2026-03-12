# xtask

Maintainer docs for `jj-navi` release tooling.

`xtask` builds the `navi-release` helper used for release fragments, version sync, and release validation.

## Install

```sh
cargo install --path xtask --force
```

If `navi-release` is not on your shell path:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

## Release fragments

Every user-facing PR should add one fragment in `.release/`.

Create one with:

```sh
navi-release "fix nested workspace discovery" -s cli
```

Or run the helper with no args for the interactive wizard:

```sh
navi-release
```

Defaults:

- no bump arg means `patch`
- use `minor` or `major` only when needed
- fragment bullets become changelog entries and GitHub release notes

Fragment format:

```md
---
bump: patch
scope: cli
---
- fix nested workspace discovery
- improve `nv` parity with `navi`
```

## Prepare a release

Run `Prepare Release` in GitHub Actions with the target version.

That workflow will:

- roll `.release/*.md` fragments into `CHANGELOG.md`
- sync versions in `Cargo.toml`, `README.md`, and `npm/jj-navi/package.json`
- open a release PR

## Publish a release

After the release PR is merged, run `Publish Release` in GitHub Actions with the same version.

That workflow will:

- build release binaries
- publish crates.io and npm packages
- tag `v<version>`
- create the GitHub Release

## Notes

- `README.md` must keep the exact `cargo install jj-navi --version ...` line because release prepare updates it in place
- fragments are deleted by the `release-prepare` flow after they are rolled into a release PR
- npm publishing expects trusted publishing to be configured for the `jj-navi*` packages
