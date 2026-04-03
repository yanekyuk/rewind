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
- **`main`** — stable release branch; only receives merges via `release/*` or `hotfix/*` branches
- **`release/X.Y.Z`** — release preparation branch cut from `next`; merged into `main` then back into `next`
- **`hotfix/X.Y.Z`** — emergency fix branch cut from `main`; merged into `main` then back into `next`

All feature branches are based off `next` and merged back into `next`. Never merge directly to `main` during development.

### PR Issue References

- PRs targeting `next`: use `Implements #N` or `Part of #N` — do **not** use `closes`/`fixes`, as that would close the issue on merge to `next`
- PRs targeting `main` (i.e. `release/*` or `hotfix/*`): use `Closes #N` — this is when the issue is actually resolved

## Implementation Workflow

When implementing features or fixes, always use subagent-driven development within a git worktree for isolation. Base worktrees off `next`.

### Worktree & Branch Naming (applies when using /using-git-worktrees)

- **Branch names** follow conventional commit prefixes with a slash: `feat/something`, `fix/something`, `chore/something`
- **Worktree folder names** replace the slash with a dash: `feat-something`, `fix-something`, `chore-something`
  - This avoids nested subdirectories inside `.worktrees/`
  - Example: branch `feat/multi-account` → worktree at `.worktrees/feat-multi-account`

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
4. Open a PR from `release/X.Y.Z` targeting **both** `main` and `next` — merge into `main` first, then into `next`
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

## Creating Hotfixes

For urgent fixes on the current release:

1. Cut a `hotfix/X.Y.Z` branch from `main`
2. Apply the fix and bump the patch version in both `Cargo.toml` files
3. Write the changelog entry in `CHANGELOG.md`
4. Open a PR from `hotfix/X.Y.Z` targeting **both** `main` and `next` — merge into `main` first, then into `next`
5. Tag `main` with an annotated tag (same format as releases above)
