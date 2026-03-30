# Rewind

Steam game downgrader — a cross-platform Tauri 2 desktop app that automates downgrading Steam games to previous versions.

## Project Structure

- `src/` — React + TypeScript frontend (Vite, port 1420)
- `src-tauri/` — Rust backend (Tauri 2)
- `docs/` — Project documentation and swe-config

## Tech Stack

- **Frontend:** TypeScript, React 19, Vite 7
- **Backend:** Rust (2021 edition), Tauri 2
- **Runtime:** Bun
- **External dependency:** DepotDownloader (bundled as Tauri sidecar, GPL-2.0)

## Commands

```bash
bun run dev          # Start Vite dev server only
bun run build        # TypeScript check + Vite build
bun run tauri dev    # Full Tauri dev (frontend + backend)
bun run tauri build  # Production build
cargo test           # Run Rust tests (from src-tauri/)
cargo clippy         # Lint Rust code (from src-tauri/)
```

## Architecture

Layered architecture with Tauri IPC boundary:

- **Domain layer** — Steam types, VDF/ACF parsing, manifest diffing. No infrastructure imports.
- **Application layer** — Downgrade orchestration and workflow coordination. No direct infrastructure imports.
- **Infrastructure layer** — Filesystem I/O, DepotDownloader subprocess, Steam path detection, manifest locking. Implements domain interfaces.
- **Frontend** — React UI communicating with Rust backend exclusively through Tauri IPC commands.

## Key Conventions

- Frontend-backend communication: Tauri IPC commands only, no direct filesystem access from React
- Error handling: Rust uses `Result<T, E>` with custom error types, propagated to frontend as typed responses
- Cross-platform: All filesystem paths must work on Linux, macOS, and Windows
- License: GPL-2.0 (required for DepotDownloader bundling)
- Commits: Conventional commits (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`)
- Branches: Conventional branches (`feat/`, `fix/`, `chore/`, `docs/`, `refactor/`)
