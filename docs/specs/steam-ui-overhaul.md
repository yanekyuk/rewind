---
title: "Steam UI Overhaul"
type: spec
tags: [frontend, ui, theme, steam, navigation, auth, game-library, version-select]
created: 2026-03-31
updated: 2026-03-31
---

# Steam UI Overhaul

## Behavior

Replace the linear step wizard with a Steam-themed application using view-based navigation: Auth Gate, Game Library, Game Detail, and Version Select.

### Navigation Model

The app uses view-based navigation managed via React state (no routing library):

| View | Purpose | Transition |
|------|---------|------------|
| Auth Gate | Login screen shown when not authenticated | On auth success -> Game Library |
| Game Library | Grid of installed games with header images | Click game -> Game Detail |
| Game Detail | Game hero area with "Change Version" button | Click button -> Version Select; Back -> Game Library |
| Version Select | Current vs target version comparison | Select version -> ready for downgrade; Back -> Game Detail |

Authentication is persistent: if credentials exist in the AuthStore (or a session token is cached by the sidecar), the app skips the Auth Gate and goes directly to the Game Library.

### Steam Theming

- Replace indigo accent palette with Steam's dark blue/grey palette:
  - Background: #1b2838 (Steam dark blue)
  - Surface: #2a475e (Steam medium blue)
  - Surface hover: #66c0f4 at low opacity
  - Accent: #66c0f4 (Steam blue)
  - Accent hover: #ffffff
  - Text: #c7d5e0 (Steam light text)
  - Text muted: #8f98a0 (Steam muted text)
- Font: "Motiva Sans", Arial, Helvetica, sans-serif (Steam's font stack)
- Dark theme only

### Auth Gate View

- Full-screen centered login form (username, password)
- On submit, calls `set_credentials` IPC which spawns the SteamKit sidecar `login` command
- While the login call is in progress, display a Steam Guard waiting indicator:
  - Pulsing shield icon or spinner
  - Text: "Waiting for Steam Guard approval..."
  - Explains that the user should approve on their Steam mobile app
- The sidecar handles device confirmation natively -- no text input for guard codes needed as the primary flow
- On success, transition to Game Library
- On error, display error message with retry option

### Game Library View

- Header showing "Rewind" branding and a user indicator (signed in as username) with sign-out option
- Grid/card layout of installed games
- Each card shows:
  - Game header image from Steam CDN: `https://cdn.akamai.steamstatic.com/steam/apps/{appid}/header.jpg`
  - Game name
  - Current installed build ID
- Loading, error, and empty states
- Clicking a card navigates to Game Detail

### Game Detail View

- Game header image displayed prominently
- Game name and metadata (app ID, build ID, install path)
- "Change Version" button (primary action)
- Back button to return to Game Library
- Mostly empty for now -- future home for download progress, version history, etc.

### Version Select View

- Shows current version info: build ID, manifest ID (from the game's first depot)
- Side-by-side comparison against a selectable list of available versions
- Each version entry shows: manifest ID, date
- Manual manifest ID input as fallback
- Back button to return to Game Detail
- Auto-fetches manifest list on mount via `list_manifests` IPC

## Constraints

- Frontend communicates with backend only through Tauri IPC commands
- No routing library -- navigation state via React useState
- Components must be small and focused
- Must render correctly at 800x600 minimum window size
- Credentials must never be logged or persisted insecurely
- All filesystem paths must work cross-platform (Linux, macOS, Windows)
- Steam CDN image URLs use appid -- images may fail to load for some games (show fallback)

## Acceptance Criteria

1. App launches to Auth Gate when not authenticated
2. App launches directly to Game Library when credentials exist in AuthStore
3. Auth form submits via `set_credentials` IPC and shows Steam Guard waiting indicator during login
4. On auth success, app navigates to Game Library
5. Game Library displays games in a card grid with header images from Steam CDN
6. Each game card shows name, header image, and current build ID
7. Clicking a game card navigates to Game Detail view
8. Game Detail shows game info and a "Change Version" button
9. "Change Version" navigates to Version Select view
10. Version Select shows current version vs selectable target versions
11. CSS uses Steam's color palette (dark blue/grey) and font stack
12. Sign-out from Game Library returns to Auth Gate
13. Back navigation works: Version Select -> Game Detail -> Game Library
14. Loading, error, and empty states are handled in Game Library and Version Select
15. Linear step wizard (StepIndicator, StepView, steps.ts) is removed
