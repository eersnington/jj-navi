# xtask

Maintainer docs for `jj-navi` release tooling.

`xtask` builds the `navi-release` helper used for release prep, version sync, and release validation.

## Install

```sh
cargo install --path xtask --force
```

If `navi-release` is not on your shell path:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

## PR labels

Every PR must have exactly one GitHub label:

- `release:major`
- `release:minor`
- `release:patch`
- `release:none`

Use `release:none` for changes that are not user-facing.

`test.yml` enforces the label rule on pull requests.

## Prepare a release

Run `Prepare Release` in GitHub Actions with the target version.

That workflow will:

- gather merged PR metadata live from GitHub since the last published tag
- build `CHANGELOG.md` entries with PR refs and author names
- sync versions in `Cargo.toml`, `README.md`, `npm/jj-navi/README.md`, and `npm/jj-navi/package.json`
- open or update the generated release PR

## Publish a release

Merging the generated release PR starts `Publish Release` automatically.

If publish fails after merge, rerun `Publish Release` manually with the same version.

That workflow will:

- validate the merged release commit
- build release binaries
- publish crates.io and npm packages
- tag `v<version>`
- create or update the GitHub Release

## Notes

- `README.md` must keep the exact `cargo install jj-navi --version ...` line because release prepare updates it in place
- npm publishing expects trusted publishing to be configured for the `jj-navi*` packages
- release prep reads live merged PR titles and labels at release time
