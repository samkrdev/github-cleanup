# github-cleanup

A terminal UI for bulk-deleting repositories from your GitHub account. Built with [ratatui](https://ratatui.rs/).

```
┌─ GitHub Cleanup — 87 repos, 4 selected ─────────────────────────┐
│ filter: experiment_                                              │
├──────────────────────────────────────────────────────────────────┤
│ [x] me/experiment-foo       ★0  priv                             │
│ [x] me/experiment-bar       ★2                                   │
│ [ ] me/experiment-baz       ★0  fork                             │
│ ▶[x] me/experiment-qux      ★1  arch                             │
└──────────────────────────────────────────────────────────────────┘
```

## Features

- Lists every repo where you are the owner (paginated, sorted by most-recently-updated)
- Multi-select with checkboxes; visual flags for `priv` / `fork` / `arch`
- Live substring filter (matches name and description)
- Confirmation modal before deletion, per-repo success/failure reporting
- Async — fetches and deletes run in tokio tasks without blocking the UI

## Install

Requires Rust ≥ 1.75.

```sh
git clone <this-repo> github-cleanup
cd github-cleanup
cargo build --release
# binary at ./target/release/github-cleanup
```

## Authentication

The app reads a GitHub token from the `GITHUB_TOKEN` environment variable.

### Classic Personal Access Token

Create one at <https://github.com/settings/tokens/new> with **both** scopes:

| Scope         | Needed for                                              |
|---------------|---------------------------------------------------------|
| `repo`        | Listing private repos (without it, only public appears) |
| `delete_repo` | Deleting repos                                          |

### Fine-grained Personal Access Token

Create one at <https://github.com/settings/personal-access-tokens/new>:

- **Repository access** → select the repos you want to manage (or "All repositories")
- **Permissions** → **Administration: Read and write** and **Metadata: Read-only**

### Export the token

```sh
export GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

> ⚠️ Never commit the token. The included `.gitignore` blocks `.env*` and `*.token` files, but don't paste it into source either.

## Run

```sh
cargo run --release
# or, after building:
./target/release/github-cleanup
```

## Keybindings

### Browsing

| Key                   | Action                          |
|-----------------------|---------------------------------|
| `↑` / `↓` or `j` / `k`| Move cursor                     |
| `PgUp` / `PgDn`       | Jump 10 rows                    |
| `g` / `G`             | Jump to top / bottom            |
| `Space`               | Toggle selection on current row |
| `a`                   | Clear all selections            |
| `/`                   | Enter filter mode               |
| `x`                   | Clear filter                    |
| `r`                   | Reload from GitHub              |
| `d`                   | Delete selected (with confirm)  |
| `q` or `Ctrl-C`       | Quit                            |

### Filter mode

| Key                | Action                |
|--------------------|-----------------------|
| (any printable)    | Append to filter      |
| `Backspace`        | Remove last character |
| `↑` / `↓`          | Move cursor in list   |
| `Enter` or `Esc`   | Return to browsing    |

### Confirmation modal

| Key             | Action            |
|-----------------|-------------------|
| `y`             | Permanently delete|
| `n` or `Esc`    | Cancel            |

## How it works

| File             | Responsibility                                                                |
|------------------|-------------------------------------------------------------------------------|
| `src/main.rs`    | Terminal setup, event loop, key dispatch, mpsc bridge to async tasks          |
| `src/github.rs`  | `GET /user/repos` (paginated, `visibility=all&affiliation=owner`) and `DELETE`|
| `src/app.rs`     | App state — repos, filter, selection set, mode                                |
| `src/ui.rs`      | All `ratatui` widgets: list, details pane, confirm + progress modals          |

The UI thread polls `crossterm` events at 100ms; HTTP requests run on a `tokio` runtime and communicate back via an unbounded `mpsc` channel, so the interface stays responsive while pagination or deletions are in flight.

## Safety notes

- **Deletion is permanent.** GitHub does not soft-delete or retain deleted repos.
- The app **only deletes when you press `d` then `y`**. There is no batch mode, no flag, no auto-confirm.
- Deletions can fail per-repo (org policy, missing scope, branch protections on the org level) — failures are reported individually in the progress modal; successful ones are removed from the list afterward.
- The app only ever calls `GET /user/repos` and `DELETE /repos/{owner}/{repo}`. No other endpoints are touched.

## Troubleshooting

| Symptom                                       | Likely cause                                                            |
|-----------------------------------------------|-------------------------------------------------------------------------|
| `GITHUB_TOKEN env var not set` on startup     | Token not exported in this shell                                        |
| No private repos shown / status: `0 private`  | Token missing `repo` scope (classic) or `Metadata: Read` (fine-grained) |
| Delete fails with 403                         | Token missing `delete_repo` scope or repo is in an org that blocks it   |
| Delete fails with 404                         | Token can't see the repo (scope/visibility mismatch)                    |
| Hitting rate limits                           | Authenticated calls are 5000/hour; wait or use a different token        |

## License

MIT
