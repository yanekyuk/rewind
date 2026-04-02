# Rewind

Steam game version downgrade manager with a ratatui TUI.

## Architecture

Rust workspace with two crates:

- **rewind-core** — Business logic library: DepotDownloader management, file caching/symlinking, ACF patching, immutability locking, Steam library scanning
- **rewind-cli** — TUI binary using ratatui + crossterm. Screens: Main, DowngradeWizard, VersionPicker, SwitchOverlay, Settings, FirstRun

## Build & Test

```sh
cargo build              # debug build
cargo build --release    # release build
cargo test               # run all tests
cargo check              # type-check without building
```

Note: 2 immutability tests (`lock_and_unlock_file`, `is_locked_reflects_state`) fail on macOS due to platform-specific file locking — this is a known issue.

## Key Conventions

- Cross-platform: all platform-specific code uses `#[cfg(target_os = "...")]` guards
- ACF files must be unlocked before patching, then re-locked after
- DepotDownloader is spawned with `setsid` on Unix to prevent .NET Console from writing to the TUI's terminal
- The `-remember-password` flag is always passed to DepotDownloader
- Async work runs in `tokio::spawn` tasks communicating via `mpsc` channels to keep the TUI event loop responsive

## Steam User Data

Per-user game config is stored in:

```
<steam_root>/userdata/<steamid>/config/localconfig.vdf
```

The Steam root is resolved via the `steamlocate` crate (`SteamDir::locate()`), making it cross-platform. The `userdata/` directory is always a direct child of the Steam root.

Key data in `localconfig.vdf`:
- `LaunchOptions` — user-set launch flags/env vars, under `Software > Valve > Steam > apps > <appid>`

When multiple Steam accounts exist (multiple `<steamid>` dirs), Rewind currently uses the most recently modified `localconfig.vdf`. Full multi-account support is tracked in [#28](https://github.com/yanekyuk/rewind/issues/28).

## Commit Style

Conventional commits: `feat:`, `fix:`, `chore:`, `docs:`, `style:`

## Branching Strategy

- **`next`** — integration branch for all new development; features and fixes land here first
- **`main`** — stable release branch; only receives merges via `release/*` or `hotfix/*` branches
- **`release/X.Y.Z`** — release preparation branch cut from `next`; merged into `main` then back into `next`
- **`hotfix/X.Y.Z`** — emergency fix branch cut from `main`; merged into `main` then back into `next`

All feature branches are based off `next` and merged back into `next`. Never merge directly to `main` during development.

## Implementation Workflow

When implementing features or fixes, always use subagent-driven development within a git worktree for isolation. Base worktrees off `next`.

## Before Finishing a Branch

Before merging or creating a PR (targeting `next`), always:

1. **Update CLAUDE.md** if architecture, conventions, or build instructions changed
2. **Update README.md** if user-facing behavior, keybindings, or setup instructions changed

Version bumps and changelog entries happen on the `release/*` or `hotfix/*` branch, not on feature branches.

## Creating Releases

When `next` is ready to release:

1. Cut a `release/X.Y.Z` branch from `next`
2. Bump versions in `rewind-core/Cargo.toml` and `rewind-cli/Cargo.toml`
3. Write the changelog entry in `CHANGELOG.md`
4. Merge `release/X.Y.Z` → `main`
5. Tag `main` with an annotated tag (the tag triggers binary builds in CI):

```sh
git tag -a vX.Y.Z -m "$(cat <<'EOF'
Release vX.Y.Z

## What's Changed
- ...
EOF
)"
git push origin vX.Y.Z
```

6. Merge `main` back into `next` to keep branches in sync

## Creating Hotfixes

For urgent fixes on the current release:

1. Cut a `hotfix/X.Y.Z` branch from `main`
2. Apply the fix and bump the patch version in both `Cargo.toml` files
3. Write the changelog entry in `CHANGELOG.md`
4. Merge `hotfix/X.Y.Z` → `main`
5. Tag `main` with an annotated tag (same format as releases above)
6. Merge `main` back into `next`
