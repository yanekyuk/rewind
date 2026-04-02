# Rewind

Steam game version downgrade manager with a ratatui TUI.

## Architecture

Rust workspace with two crates:

- **rewind-core** — Business logic library: DepotDownloader management, file caching/symlinking, ACF patching, immutability locking, Steam library scanning, ReShade download/symlink management
- **rewind-cli** — TUI binary using ratatui + crossterm. Screens: Main, DowngradeWizard, VersionPicker, SwitchOverlay, Settings, FirstRun, ReshadeSetup

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

## Commit Style

Conventional commits: `feat:`, `fix:`, `chore:`, `docs:`, `style:`

## Branching Strategy

- **`next`** — integration branch for all new development; features and fixes land here first
- **`main`** — stable release branch; only receives merges from `next` when cutting a release

All feature branches are based off `next` and merged back into `next`. Never merge directly to `main` during development.

## Implementation Workflow

When implementing features or fixes, always use subagent-driven development within a git worktree for isolation. Base worktrees off `next`.

## Before Finishing a Branch

Before merging or creating a PR (targeting `next`), always:

1. **Bump versions** in `rewind-core/Cargo.toml` and `rewind-cli/Cargo.toml` if any features or fixes were added
2. **Update CLAUDE.md** if architecture, conventions, or build instructions changed
3. **Update README.md** if user-facing behavior, keybindings, or setup instructions changed

## Creating Releases

When `next` is ready to release:

1. Merge `next` → `main`
2. Tag `main` with an annotated tag:

```sh
git tag -a v0.2.0 -m "Release notes here..."
git push origin v0.2.0
```
