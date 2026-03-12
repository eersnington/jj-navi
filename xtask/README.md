# xtask

Maintainer docs for `jj-navi` release tooling.

`xtask` builds the `navi-release` helper used for release prep, PR metadata validation, version sync, and release note generation.

## Install

```sh
cargo install --path xtask --force
```

If `navi-release` is not on your shell path:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

## PR release metadata

Every PR needs exactly one `release:*` label:

- `release:major`
- `release:minor`
- `release:patch`
- `release:none`

Rules:

- PR title becomes the changelog bullet
- `release:none` PRs are skipped from changelog output
- labels are validated by `.github/workflows/release-metadata.yml`

This is written in two places:

- `.github/PULL_REQUEST_TEMPLATE.md`
- this file

And enforced in CI by:

- `.github/workflows/release-metadata.yml`

## Prepare a release

Run `Prepare Release` in GitHub Actions with the target version.

That workflow will:

- collect merged PRs since the last release tag
- skip PRs labeled `release:none`
- ignore generated release PRs from earlier release attempts
- build a flat `CHANGELOG.md` section from PR titles, explicit PR links, and author names
- enforce that the chosen version matches the highest included `release:*` label
- sync versions in `Cargo.toml`, `README.md`, and `npm/jj-navi/package.json`
- open or update the release PR

## Publish a release

Merging the generated release PR auto-runs `release-publish.yml`.

That workflow will:

- validate synced release files on the merged release commit
- tag `v<version>`
- build release binaries
- publish crates.io and npm packages
- create the GitHub Release

`workflow_dispatch` stays available as a retry path, but it now requires both the version and the exact ref to publish, test, and tag.

## Notes

- `README.md` must keep the exact `cargo install jj-navi --version ...` line because release prep updates it in place
- create the GitHub `release:*` labels once before turning this flow on
- npm publishing expects trusted publishing to be configured for the `jj-navi*` packages
