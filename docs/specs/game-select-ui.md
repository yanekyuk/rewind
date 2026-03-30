---
title: "Game Select UI"
type: spec
tags: [frontend, ui, game, selection, ipc, steps]
created: 2026-03-30
updated: 2026-03-30
---

## Behavior

Wire the `list_games` Tauri IPC command to the "Select Game" step (step 0) in the app shell. Replace the placeholder step view with a real game list that allows the user to pick an installed Steam game.

### TypeScript Type Mirroring

Mirror the Rust domain types as TypeScript interfaces for type-safe IPC:

- `GameInfo`: `appid: string`, `name: string`, `buildid: string`, `installdir: string`, `depots: DepotInfo[]`, `install_path: string`
- `DepotInfo`: `depot_id: string`, `manifest: string`, `size: string`

### Data Fetching

- A `useGameList` hook (or equivalent) calls `invoke("list_games")` from `@tauri-apps/api/core` on mount.
- Manages three states: loading, error, and success (with data).
- Returns the game list and loading/error state to the consuming component.

### GameSelect Component

Renders in place of the generic `StepView` placeholder when the active step is "select-game":

- **Loading state**: Spinner or loading indicator while the IPC call is in flight.
- **Error state**: Message displayed if Steam is not found or the fetch fails. Includes a retry mechanism.
- **Empty state**: Message indicating no games were found (Steam may not be installed or no games installed).
- **Game list**: Each row shows the game name, app ID, and build ID. The list scrolls vertically if it exceeds the available space.
- **Selection**: Clicking a row selects that game (visually highlighted). Only one game can be selected at a time.

### App State Integration

- The selected `GameInfo` is lifted into `App.tsx` state so downstream steps can access it.
- The "Next" button on the Select Game step is disabled until a game is selected.
- Selecting a game and clicking "Next" advances to step 1.

### Step-Specific Rendering

- `App.tsx` (or a router component) renders `GameSelect` when on step 0, and the generic `StepView` placeholder for all other steps.

## Constraints

- Use `@tauri-apps/api/core` `invoke` for IPC -- already a project dependency.
- Dark theme styling consistent with existing `App.css`.
- Must render correctly at 800x600 window size -- game list scrolls if content overflows.
- Components must be small and focused (per project directives).
- Frontend communicates with backend only through Tauri IPC -- no direct filesystem access.

## Acceptance Criteria

1. `GameInfo` and `DepotInfo` TypeScript interfaces exist and match the Rust domain types.
2. `useGameList` hook calls `invoke("list_games")` on mount and exposes loading, error, and data states.
3. `GameSelect` component renders a loading indicator while fetching.
4. `GameSelect` component renders an error message if the fetch fails, with a retry button.
5. `GameSelect` component renders a list of games with name, app ID, and build ID.
6. Clicking a game row selects it (highlighted) and only one game is selected at a time.
7. The "Next" button is disabled when no game is selected and enabled when one is.
8. Selecting a game and clicking "Next" advances to step 1.
9. Other steps still render the generic placeholder.
10. The game list scrolls when content exceeds available space at 800x600.
11. All existing tests pass and new tests cover the GameSelect component.
