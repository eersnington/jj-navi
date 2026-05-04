# jj-navi

<img width="788" height="640" alt="jj-navi" src="https://github.com/user-attachments/assets/88e8b46e-9a76-416b-9f76-b4480d6964e7" />

Workspace management for [Jujutsu](https://jj-vcs.github.io/jj/latest/), built for parallel human and AI agent workflows.

## The problem

jj workspaces are great for parallel work, but the workflow around it is quite cumbersome:

- **Paths are unmanaged.** `jj workspace add ../name` works, but paths are arbitrary and easy to forget.
- **Cross-workspace visibility is stale.** jj snapshots the current workspace when you run a command, but not the others. So `jj log` from one workspace can show outdated commits for the rest — files on disk exist, but jj hasn't recorded them yet.
- **Cleanup is awkward.** Forgetting a workspace does not delete its directory, and deleting a directory does not forget the workspace. There is also no guard against removing the one you are currently in.
- **Switching doesn't switch your shell.** `jj workspace` changes the working copy, not your terminal's current directory.

## What `jj-navi` does

`jj-navi` manages workspace lifecycle: naming, paths, switching, visibility, and cleanup.

- **`switch --create`** — go to a workspace, creating it at a deterministic path if it doesn't exist
- **`list`** — snapshot each workspace and show path health, diff stats, commit info, and age
- **`merge`** — merge work from another workspace into the current or named workspace
- **`remove`** — forget a workspace and delete its local directory; refuses current workspace

With shell integration installed, `navi switch` also changes your current directory.

```text
repo/
├── repo                 current workspace
├── repo.feature-auth    navi switch --create feature-auth
└── repo.fix-api         navi switch --create fix-api
```

## Before and after

**Without `jj-navi`**

```sh
jj workspace add ../repo.feature-auth
cd ../repo.feature-auth
# ... do work ...
cd ../repo
jj log                          # stale view of other workspaces
jj workspace list               # names only
jj workspace forget feature-auth
rm -rf ../repo.feature-auth     # directory left behind
```

**With `jj-navi`**

```sh
navi switch --create feature-auth
# ... do work ...
navi switch -
navi list                       # snapshotted, with diff stats and age
navi remove feature-auth        # asks before deleting the workspace directory
```

## Install

```sh
# npm
npm install -g jj-navi

# cargo
cargo install jj-navi --version 0.2.3
```

Binaries: `navi`, `nv`

Minimum `jj`: `0.39.0`  
Minimum Node.js (tested): `24`

## Shell integration

Install once so `navi switch` can update your shell's current directory:

```sh
navi config shell install --shell zsh
source ~/.zshrc
```

Supports `bash` and `zsh`. This adds a managed block to your shell rc file.

## Quick start

```sh
navi doctor
navi switch --create feature-auth
navi list
navi switch -
navi remove feature-auth
```

## Commands

```sh
navi switch <workspace>          # switch to a workspace
navi cd <workspace>              # alias for switch
navi switch ^                    # switch to the primary workspace
navi switch -                    # switch to previous workspace
navi switch @                    # switch to current workspace explicitly
navi switch --create <workspace> # create and switch
navi switch -c <workspace>
navi switch --create <workspace> --revision <revset> # create from a revision
navi switch -c <workspace> -r <revset>

navi list                        # human-readable workspace inventory
navi ls                          # alias for list
navi list --json
navi list -j
navi list --json --compact
navi list -j -c

navi doctor [--json] [--compact] # diagnose repo, workspace, and shell state
navi doctor [-j] [-c]

navi merge --from <workspace>     # merge a workspace into the current workspace
navi merge -f <workspace>
navi merge --from <workspace> --into <workspace>
navi merge -f <workspace> -i <workspace>

navi remove <workspace>          # forget a workspace and delete its directory
navi rm <workspace>              # alias for remove
navi remove <workspace> --yes    # skip destructive confirmation
navi remove <workspace> -y

navi config shell init <bash|zsh>
navi config shell install [--shell <bash|zsh>]
navi config shell install [-s <bash|zsh>]
```

## How it works

Config and metadata live inside shared Jujutsu storage:

```text
.jj/repo/navi/config.toml
.jj/repo/navi/workspaces.toml
```

Default workspace path template: `../{repo}.{workspace}`

## Notes

- `switch` can recover from missing jj workspace-path records when it can validate a fallback path
- `switch` warns when it falls back to template-based path resolution
- `list` snapshots healthy workspaces before rendering so parallel changes are visible
- `list` reports missing, stale, or not-current workspaces instead of hiding them
- `list --json` exposes structured `freshness`, `diff`, and `age` fields
- `remove` forgets a workspace and deletes its directory after confirmation; `--yes` skips the prompt
- Supported shells: `bash`, `zsh`

## Special thanks

Inspired by:

- [Worktrunk](https://github.com/max-sixty/worktrunk) — Git worktree management for parallel AI agent workflows
- [jj-ryu](https://github.com/dmmulroy/jj-ryu) — Stacked PRs for Jujutsu

## Art credits

- [BoTW Link Pixel Art](https://www.reddit.com/r/zelda/comments/piy10r/botw_oc_hero_of_the_wild_pixel_art/)

## License

[MIT](https://github.com/eersnington/jj-navi/blob/main/LICENSE)
