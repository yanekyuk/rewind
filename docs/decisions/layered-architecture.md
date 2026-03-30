---
title: "Layered Architecture"
type: decision
tags: [architecture, layers, domain, application, infrastructure, tauri, ipc]
created: 2026-03-30
updated: 2026-03-30
---

# Layered Architecture

## Context

Rewind has a Rust backend (Tauri) and a React frontend. The backend handles file I/O, subprocess management, Steam path detection, VDF parsing, and manifest diffing. The frontend handles user interaction. The two communicate via Tauri IPC commands.

A clear separation of concerns is needed to keep the codebase maintainable as the feature set grows.

## Decision

Use a three-layer architecture in the Rust backend: domain, application, and infrastructure. The React frontend communicates with the backend exclusively through Tauri IPC commands.

## Layer Responsibilities

### Domain Layer

Pure business logic and type definitions. No I/O, no external dependencies.

- Steam types (App, Depot, Manifest, BuildId)
- VDF/ACF parsing logic
- Manifest diffing algorithm
- Filelist generation

**Rule**: The domain layer must not import from application or infrastructure layers.

### Application Layer

Workflow orchestration. Coordinates domain logic with infrastructure capabilities.

- Downgrade workflow (the 9-step process as a state machine)
- Progress tracking and event emission
- Error aggregation and user-facing error construction

**Rule**: The application layer must not import from the infrastructure layer directly. It depends on trait interfaces defined in the domain layer, which the infrastructure layer implements.

### Infrastructure Layer

External world interactions. Implements domain-defined interfaces.

- Filesystem I/O (reading ACF files, copying game files)
- DepotDownloader subprocess management (spawn, stdin/stdout, cancellation)
- Steam installation path detection
- Manifest file locking (chattr, chflags, SetFileAttributes)
- OS-level notifications

**Rule**: The infrastructure layer implements interfaces defined in the domain layer.

### Frontend (React)

- User interface for game selection, manifest input, progress display
- Communicates with the Rust backend only through Tauri IPC commands
- No direct filesystem access

**Rule**: Frontend communicates with backend only through Tauri IPC commands -- no direct filesystem access.

## IPC Boundary

Shared types between frontend and backend are defined in the Rust domain layer and mirrored to TypeScript via Tauri's type generation. This ensures type safety across the IPC boundary.

```
Frontend (React/TS)  <--IPC-->  Application Layer  <-->  Domain Layer
                                      |
                                      v
                               Infrastructure Layer
                               (FS, subprocesses, OS)
```

## Rationale

- **Testability**: The domain layer can be unit-tested without mocking I/O. The application layer can be tested with mock infrastructure implementations.
- **Cross-platform isolation**: Platform-specific code (path detection, manifest locking) lives entirely in the infrastructure layer. Adding a new platform means adding infrastructure implementations, not changing domain logic.
- **Tauri convention**: Tauri's command system naturally forms a boundary between frontend and backend. The layered architecture extends this separation within the backend.

## Trade-offs

- **Indirection**: Trait-based dependency inversion adds some boilerplate (trait definitions, implementations, wiring).
- **Premature for MVP?**: A simpler flat structure might be faster to build initially. However, the cross-platform requirements and subprocess management complexity justify the upfront investment in structure.
