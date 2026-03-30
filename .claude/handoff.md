---
trigger: "Wire game listing to frontend — connect the list_games Tauri IPC command to the Select Game step in the app shell. Show installed games in the UI, let the user select one to proceed to the next step."
type: feat
branch: feat/game-select-ui
base-branch: main
created: 2026-03-30
version-bump: patch
---

## Related Files
- src/App.tsx (main app component — manages step state, needs to pass selected game down)
- src/components/StepView.tsx (placeholder step view — needs to be replaced with step-specific components)
- src/steps.ts (step definitions — Select Game step is index 0, id "select-game")
- src/App.css (styles — will need game list styles)
- src/App.test.tsx (existing tests — update for new behavior)
- src-tauri/src/lib.rs (list_games command is already registered)
- src-tauri/src/domain/game.rs (GameInfo and DepotInfo types — the shape of data from backend)

## Relevant Docs
- docs/specs/game-listing.md (GameInfo type shape, IPC command contract)
- docs/specs/app-shell.md (step navigation, layout constraints, 800x600 window)
- docs/specs/mvp-scope.md (core flow: detect Steam → list games → user picks game)
- docs/domain/downgrade-process.md (steps 1-3: detect, list, select)

## Related Issues
None — no related issues found.

## Scope
Replace the placeholder Select Game step with a real game list fetched from the backend, with selection handling and state management.

### What to build

1. **GameInfo TypeScript type**: Mirror the Rust GameInfo/DepotInfo structs as TypeScript interfaces for type-safe IPC:
   - `GameInfo { appid, name, buildid, installdir, depots: DepotInfo[], install_path }`
   - `DepotInfo { depot_id, manifest, size }`

2. **useGameList hook** (or inline in component): Call `invoke("list_games")` on mount, manage loading/error/data states. Use `@tauri-apps/api/core` invoke.

3. **GameSelect component**: Replaces the StepView placeholder for the "select-game" step:
   - Loading state while fetching
   - Error state if Steam not found or fetch fails
   - List of installed games showing name, appid, and buildid
   - Click to select a game — highlight the selected row
   - Selected game enables the "Next" button

4. **App state management**: Lift the selected game into App.tsx state so downstream steps can access it. The Next button on step 0 should only be enabled when a game is selected.

5. **Step-specific rendering**: Update App.tsx or StepView to render GameSelect when on the "select-game" step, and the generic placeholder for other steps.

### Constraints
- Use `@tauri-apps/api/core` invoke for IPC — already a dependency
- Keep components small and focused
- Dark theme styling consistent with existing App.css
- Must work at 800x600 — game list should scroll if many games
- Handle empty list gracefully (Steam not installed message)
- Tests: update existing tests and add tests for the new GameSelect component
