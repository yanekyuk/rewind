---
trigger: "Frontend scaffolding — strip the Tauri template UI and set up the real app shell with layout, step-based navigation for the downgrade workflow, and basic theme/styling. Pure React, no backend calls."
type: feat
branch: feat/app-shell
base-branch: main
created: 2026-03-30
version-bump: patch
---

## Related Files
- src/App.tsx (template app component — replace entirely)
- src/App.css (template styles — replace entirely)
- src/main.tsx (React entry — keep, minor updates if needed)
- src/assets/react.svg (template asset — remove)
- public/tauri.svg (keep as app icon placeholder)
- public/vite.svg (template asset — remove)
- index.html (entry HTML — may need title/meta updates)
- package.json (may need new dependencies)

## Relevant Docs
- docs/specs/mvp-scope.md (defines the core flow and steps the UI must represent)
- docs/decisions/progress-ui.md (progress bar, cancel button, download speed — informs layout needs)
- docs/decisions/manual-manifest-input.md (user workflow: paste manifest ID — informs input step)
- docs/domain/downgrade-process.md (the 9-step workflow the UI navigates through)

## Related Issues
None — no related issues found.

## Scope
Replace the Tauri template frontend with the real Rewind app shell. This is a frontend-only task — no Tauri IPC calls, no backend integration.

### What to build
- **App layout**: Clean, dark-themed desktop app layout suitable for a gaming tool. Sidebar or header with app branding, main content area for the active step.
- **Step navigation**: The downgrade workflow is a linear sequence of steps. Build a step indicator/stepper component showing progress through the flow:
  1. Select Game
  2. Enter Manifest ID
  3. Comparing Versions
  4. Downloading Files
  5. Applying Downgrade
  6. Complete
- **Placeholder step views**: Each step gets a placeholder component with a title and description. No real functionality — just the structure for future wiring.
- **Theme/styling**: Dark theme (gaming audience). Clean, minimal. Replace all template CSS. Use CSS modules or a single global stylesheet — no CSS framework required unless the orchestrator finds it beneficial.
- **Cleanup**: Remove template assets (react.svg, vite.svg), template greet logic, and template CSS. Update index.html title to "Rewind".

### Constraints
- No Tauri IPC calls — this is purely frontend scaffolding
- No routing library needed — step state can be managed with React useState for now
- Keep components small and focused per implementation directives
- Must look reasonable at the default 800x600 window size (from tauri.conf.json)
