# github-cleanup

A terminal UI for bulk-deleting **repositories and gists** from your GitHub account. Built with [ratatui](https://ratatui.rs/).

```
┌─ GitHub Cleanup — Repositories · 87 repos, 4 selected ──────────┐
│ filter: experiment_                                              │
├──────────────────────────────────────────────────────────────────┤
│ [x] me/experiment-foo       ★0  priv                             │
│ [x] me/experiment-bar       ★2                                   │
│ [ ] me/experiment-baz       ★0  fork                             │
│ ▶[x] me/experiment-qux      ★1  arch                             │
└──────────────────────────────────────────────────────────────────┘
```

Press `Tab` to switch to the Gists view:

```
┌─ GitHub Cleanup — Gists · 23 gists, 2 selected ─────────────────┐
│ filter: deploy_                                                  │
├──────────────────────────────────────────────────────────────────┤
│ [x] deploy.sh              +1 files  public                      │
│ ▶[x] notes.md                        secret                      │
└──────────────────────────────────────────────────────────────────┘
```

## Features

- Two views, toggled with `Tab`: **Repositories** (owned, paginated, sorted by most-recently-updated) and **Gists** (paginated)
- Multi-select with checkboxes; visual flags for `priv` / `fork` / `arch` on repos, `public` / `secret` and file count on gists
- Live substring filter (repos: name + description; gists: filenames + description)
- Per-view selections are kept independently as you switch with `Tab`
- Confirmation modal before deletion, per-item success/failure reporting
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

Create one at <https://github.com/settings/tokens/new> with these scopes:

| Scope         | Needed for                                              |
|---------------|---------------------------------------------------------|
| `repo`        | Listing private repos (without it, only public appears) |
| `delete_repo` | Deleting repos                                          |
| `gist`        | Listing and deleting gists                              |

### Fine-grained Personal Access Token

Create one at <https://github.com/settings/personal-access-tokens/new>:

- **Repository access** → select the repos you want to manage (or "All repositories")
- **Permissions** → **Administration: Read and write**, **Metadata: Read-only**, and **Gists: Read and write**

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
| `Tab`                 | Switch Repos ↔ Gists view       |
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
| `src/github.rs`  | `GET /user/repos`, `GET /gists` (both paginated), and `DELETE` for each       |
| `src/app.rs`     | App state — repos, gists, active view, filter, per-view selection, mode       |
| `src/ui.rs`      | All `ratatui` widgets: list, details pane, confirm + progress modals          |

The UI thread polls `crossterm` events at 100ms; HTTP requests run on a `tokio` runtime and communicate back via an unbounded `mpsc` channel, so the interface stays responsive while pagination or deletions are in flight.

## Safety notes

- **Deletion is permanent.** GitHub does not soft-delete or retain deleted repos or gists.
- The app **only deletes when you press `d` then `y`**, and only items selected in the active view. There is no batch mode, no flag, no auto-confirm.
- Deletions can fail per-item (org policy, missing scope, branch protections on the org level) — failures are reported individually in the progress modal; successful ones are removed from the list afterward.
- The app only ever calls `GET /user/repos`, `GET /gists`, `DELETE /repos/{owner}/{repo}`, and `DELETE /gists/{id}`. No other endpoints are touched.

## Troubleshooting

| Symptom                                       | Likely cause                                                            |
|-----------------------------------------------|-------------------------------------------------------------------------|
| `GITHUB_TOKEN env var not set` on startup     | Token not exported in this shell                                        |
| No private repos shown / status: `0 private`  | Token missing `repo` scope (classic) or `Metadata: Read` (fine-grained) |
| Delete fails with 403                         | Token missing `delete_repo`/`gist` scope, or org blocks deletion        |
| Delete fails with 404                         | Token can't see the repo/gist (scope/visibility mismatch)               |
| No gists shown / `gist load failed`           | Token missing `gist` scope (classic) or `Gists` perm (fine-grained)     |
| Hitting rate limits                           | Authenticated calls are 5000/hour; wait or use a different token        |

## License

MIT
