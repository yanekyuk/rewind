---
trigger: "Rust project structure — set up the module layout (domain/, application/, infrastructure/) in src-tauri/src/ following the layered architecture decision. Create the module hierarchy so feature branches have a clear place to put files."
type: chore
branch: chore/rust-module-layout
base-branch: main
created: 2026-03-30
version-bump: none
---

## Related Files
- src-tauri/src/lib.rs (current entry point — needs to declare new modules)
- src-tauri/src/main.rs (binary entry)
- src-tauri/Cargo.toml (may need new dependencies like thiserror for error types)

## Relevant Docs
- docs/decisions/layered-architecture.md (defines the three layers and their rules)
- docs/domain/steam-internals.md (domain types that will live in domain/)
- docs/domain/downgrade-process.md (application layer workflow)

## Related Issues
None — no related issues found.

## Scope
Set up the Rust module hierarchy in src-tauri/src/ to match the layered architecture decision.

### What to create
- `src-tauri/src/domain/mod.rs` — domain layer root (re-exports submodules)
- `src-tauri/src/application/mod.rs` — application layer root
- `src-tauri/src/infrastructure/mod.rs` — infrastructure layer root
- `src-tauri/src/error.rs` — shared error types (custom error enum with thiserror)
- Update `lib.rs` to declare the three layer modules and the error module

### Constraints
- Empty modules with doc comments explaining what belongs in each layer
- No functional code beyond module declarations and the error type skeleton
- Keep the existing Tauri setup (greet command can stay as a placeholder)
- The VDF parser (feat/vdf-parser) is being built in parallel — it will target domain/ when it merges, so the module structure must be ready
