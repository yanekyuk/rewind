---
title: "App Shell"
type: spec
tags: [frontend, ui, shell, navigation, steps, theme]
status: superseded
superseded-by: steam-ui-overhaul.md
created: 2026-03-30
updated: 2026-03-31
---

> **Superseded:** This spec describes the original step-wizard layout. It has been replaced by the Steam UI Overhaul spec (`docs/specs/steam-ui-overhaul.md`), which introduces view-based navigation.

# App Shell

## Behavior

The app shell is the top-level React component structure that provides the visual frame and step-based navigation for the Rewind downgrade workflow.

### Layout

- A header area displaying the app name "Rewind" and branding.
- A step indicator showing the user's position in the linear workflow.
- A main content area rendering the active step's placeholder view.
- Dark theme optimized for the gaming audience and an 800x600 default window size.

### Step Navigation

The downgrade workflow is presented as 6 sequential steps:

| Step | Label | Description |
|------|-------|-------------|
| 1 | Select Game | Choose an installed Steam game to downgrade |
| 2 | Enter Manifest ID | Paste the target manifest ID from SteamDB |
| 3 | Comparing Versions | Diffing current vs target manifests |
| 4 | Downloading Files | Downloading changed files via DepotDownloader |
| 5 | Applying Downgrade | Applying files, patching ACF, locking manifest |
| 6 | Complete | Downgrade finished, reminder to set update preference |

Navigation state is managed via React `useState`. No routing library is used.

### Step Mapping to Backend Workflow

The 6 UI steps map to the 9-step backend process (see domain/downgrade-process.md):

- **Select Game** = backend steps 1 (Detect Steam) + 2 (List Games) + 3 (User Picks Game)
- **Enter Manifest ID** = backend step 4
- **Comparing Versions** = backend step 5
- **Downloading Files** = backend step 6
- **Applying Downgrade** = backend steps 7 (Apply Files) + 8 (Patch ACF & Lock Manifest)
- **Complete** = backend step 9 (Remind User)

## Constraints

- No Tauri IPC calls -- purely frontend scaffolding.
- No routing library -- step state via `useState`.
- Components must be small and focused.
- Must render correctly at 800x600 (Tauri default window size).
- Dark theme only (no light mode toggle).
- All template assets and code must be removed (react.svg, vite.svg, greet logic, template CSS).

## Acceptance Criteria

- [ ] Template UI is fully removed (no Tauri greet form, no template logos, no template CSS).
- [ ] index.html title is "Rewind".
- [ ] App renders a dark-themed layout with header, step indicator, and content area.
- [ ] Step indicator displays all 6 steps with the current step visually highlighted.
- [ ] Each step has a placeholder component with title and description text.
- [ ] Step navigation works (next/back or programmatic progression).
- [ ] Layout fits within 800x600 without scrollbars on the main structure.
- [ ] No Tauri IPC imports or calls exist in the frontend code.
- [ ] Template assets (react.svg, vite.svg) are deleted.
