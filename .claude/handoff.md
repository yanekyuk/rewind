---
trigger: "Change the way the app is organized. 1. Use Steam Colors and fonts if possible. 2. Start with authentication, keep it persistent. 2.1. Upon authenticating, we need a visual indicator that the user should approve via Steam Guard (Steam App). 3. Show the games as they appear in Steam, with images. 4. Show the current installed versions of the game. 5. Clicking on a game changes the view. Make it similar to steam game page, for now leave it empty but a button to change version. 6. Change version, when clicked, shows the current build id, manifest id, version etc against a selectable version with build id and manifest id."
type: feat
branch: feat/steam-ui
base-branch: main
created: 2026-03-31
version-bump: minor
---

## Related Files
- src/App.tsx — Main app shell, step wizard logic
- src/App.css — All styles (single file)
- src/steps.ts — Step definitions (wizard model being replaced)
- src/components/AuthInput.tsx — Auth form
- src/components/GameSelect.tsx — Game list
- src/components/ManifestSelect.tsx — Version selector
- src/hooks/useAuth.ts — Auth hook with Tauri IPC
- src/hooks/useGameList.ts — Game list hook
- src/hooks/useManifestList.ts — Manifest list hook
- src/types/game.ts — GameInfo type
- src-tauri/src/lib.rs — Tauri commands (list_games, set_credentials, list_manifests, etc.)

## Relevant Docs
- docs/specs/auth-ui.md
- docs/specs/game-select-ui.md
- docs/specs/app-shell.md
- docs/specs/manifest-select.md
- docs/domain/steamkit-sidecar.md

## Related Issues
None — no related issues found.

## Scope
Complete UI overhaul replacing the linear step wizard with a Steam-themed application:

1. **Steam theming** — Replace current indigo color palette with Steam's dark blue/grey palette and Motiva Sans font family. Update CSS custom properties across the board.

2. **Auth-first persistent flow** — Authentication becomes the entry point. On app launch, show login if not authenticated. Once authenticated, persist the session and go straight to the game library. No more wizard step for auth.

3. **Steam Guard visual indicator** — When the user submits credentials and Steam Guard 2FA is required, display a visual indicator (pulsing icon, spinner, or prompt) telling the user to approve the login on their Steam mobile app. This replaces the current text-input guard code flow (the SteamKit sidecar handles Steam Guard natively via device confirmation).

4. **Game library with images** — Replace the plain text game list with a Steam-style grid/card layout. Each game card shows the game's header image (fetched via Steam CDN using appid: `https://cdn.akamai.steamstatic.com/steam/apps/{appid}/header.jpg`), name, and current installed build ID.

5. **Game detail page** — Clicking a game navigates to a Steam-like detail view. For now, the page is mostly empty but includes a "Change Version" button.

6. **Version comparison view** — The "Change Version" button opens a view showing the current version (build ID, manifest ID) side-by-side against a selectable list of available versions (each with build ID, manifest ID, date).

Navigation model shifts from a linear wizard to: Auth Gate → Game Library → Game Detail → Version Select.
