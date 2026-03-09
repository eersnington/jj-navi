# jj-navi

Minimal workspace navigator for Jujutsu.

## v0

- `navi switch <workspace>`
- `nv switch <workspace>`
- `navi switch --create <workspace>`
- `navi switch --create <workspace> --revision <revset>`
- `navi list`

## Install

```sh
cargo install --path .
```

This installs both `navi` and `nv`.

## Usage

```sh
navi switch --create feature-auth
cd "$(navi switch feature-auth)"
navi list
```
