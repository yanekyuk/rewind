# GitHub Actions Release Workflow — Design Spec

## Overview

A single GitHub Actions workflow that builds cross-platform executables and creates a GitHub Release when a version tag is pushed. After the release, the README is updated with direct download links.

## Trigger

- Push of annotated tags matching `v*.*.*` (e.g., `v1.0.0`)
- Release notes are taken from the annotated tag message

## Build Matrix

Single `build` job using a matrix strategy with 4 targets, each on its native runner:

| Matrix Entry | Runner | Rust Target | Archive Name | Format |
|---|---|---|---|---|
| Linux x64 | `ubuntu-latest` | `x86_64-unknown-linux-gnu` | `rewind-x86_64-linux.tar.gz` | tar.gz |
| Windows x64 | `windows-latest` | `x86_64-pc-windows-msvc` | `rewind-x86_64-windows.zip` | zip |
| macOS Intel | `macos-13` | `x86_64-apple-darwin` | `rewind-x86_64-macos.tar.gz` | tar.gz |
| macOS ARM | `macos-14` | `aarch64-apple-darwin` | `rewind-aarch64-macos.tar.gz` | tar.gz |

### Build Steps (per matrix entry)

1. Checkout repository
2. Install Rust stable toolchain (via `dtolnay/rust-toolchain`)
3. `cargo build --release`
4. Package binary into archive:
   - Linux/macOS: `tar czf <archive-name> rewind`
   - Windows: PowerShell `Compress-Archive rewind.exe <archive-name>`
5. Upload archive as workflow artifact (via `actions/upload-artifact`)

## Release Job

Runs after all `build` matrix jobs succeed. Depends on `build` via `needs: build`.

1. Download all 4 artifacts (via `actions/download-artifact`)
2. Extract the annotated tag message using `git tag -l --format='%(contents)' $TAG`
3. Create a GitHub Release using `softprops/action-gh-release`:
   - Title: tag name (e.g., `v1.0.0`)
   - Body: annotated tag message
   - Attach all 4 archive files as release assets

## README Update Job

Runs after the `release` job succeeds. Depends on `release` via `needs: release`.

1. Checkout `main` branch
2. Use `sed` to replace download links in the "Step 2 — Download rewind" table:
   - Each platform row gets a direct link: `https://github.com/yanekyuk/rewind/releases/download/<tag>/<archive-name>`
3. Commit and push the updated README to `main`
   - Uses a bot identity for the commit (github-actions[bot])
   - Only commits if the README actually changed

## README Table Format (After Update)

```markdown
| Platform | File |
|----------|------|
| Windows (64-bit) | [`rewind-x86_64-windows.zip`](https://github.com/yanekyuk/rewind/releases/download/v1.0.0/rewind-x86_64-windows.zip) |
| macOS (Apple Silicon) | [`rewind-aarch64-macos.tar.gz`](https://github.com/yanekyuk/rewind/releases/download/v1.0.0/rewind-aarch64-macos.tar.gz) |
| macOS (Intel) | [`rewind-x86_64-macos.tar.gz`](https://github.com/yanekyuk/rewind/releases/download/v1.0.0/rewind-x86_64-macos.tar.gz) |
| Linux (64-bit) | [`rewind-x86_64-linux.tar.gz`](https://github.com/yanekyuk/rewind/releases/download/v1.0.0/rewind-x86_64-linux.tar.gz) |
```

## Workflow File

Single file: `.github/workflows/release.yml`

## Jobs Summary

```
build (matrix: 4 targets, parallel)
  └─► release (creates GH release, uploads assets)
       └─► update-readme (updates download links on main)
```

## Permissions

- `contents: write` — needed for creating releases and pushing README updates

## User Workflow

1. Write release notes
2. Create annotated tag: `git tag -a v1.0.0 -m "Release notes..."`
3. Push the tag: `git push origin v1.0.0`
4. Workflow runs automatically — builds, releases, updates README
