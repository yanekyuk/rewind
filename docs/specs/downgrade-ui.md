---
title: "Downgrade UI — SteamDB Webview, Progress View, and Completion State"
type: spec
tags: [ui, downgrade, webview, progress, ux]
created: 2026-03-31
---

# Downgrade UI

## Overview

This spec defines the frontend UI for the downgrade feature. It comprises three linked views:

1. **Version Selection** (enhanced) — SteamDB webview + manual ID input + sidecar manifests
2. **Downgrade Progress View** — Real-time progress for Comparing → Downloading → Applying phases
3. **Completion State** — Success message with Steam preference reminder

## Navigation

The downgrade flow is activated from GameDetail → "Change Version" button:

```
App.tsx
├── currentView: ViewId (add "downgrade" to the enum)
├── GameDetail
│   └── "Change Version" button
│       └── handleChangeVersion() → setCurrentView("version-select")
│
└── version-select view (VersionSelect component)
    ├── Manifest selection (SteamDB webview + sidecar + manual)
    │   └── onSelectManifest(manifestId)
    │       └── Calls start_downgrade IPC command
    │           └── setCurrentView("downgrade")
    │
    └── downgrade view (DowngradeProgress component)
        ├── Real-time progress (Comparing → Downloading → Applying → Complete)
        ├── Cancel button
        └── Completion state (success + reminder)
            └── "Return to Game" button → setCurrentView("game-detail")
```

## 1. Version Selection (Enhanced)

### Location
- `src/components/VersionSelect.tsx` (existing, enhanced to add webview)
- `src/components/SteamDBWebview.tsx` (new)

### Structure

VersionSelect now has three tabs/sections:

1. **Current Version** (info display)
   - Build ID
   - Current Manifest ID

2. **Available Versions** (three sources)

   a) **SteamDB Webview** (new)
   - Embedded Tauri webview showing `https://steamdb.info/depot/<depotId>/manifests/`
   - User can browse, log in (if needed), search history
   - JavaScript injection extracts manifest table from DOM
   - Extracted data shows in a list below webview
   - Selection from webview list → triggers downgrade

   b) **Sidecar Manifests** (existing)
   - Manifests from the installed game's depot
   - Fetched via `list_manifests` IPC command
   - Shows branch, timestamp, pwd_required badge
   - Current version highlighted
   - Selection → triggers downgrade

   c) **Manual Entry** (existing)
   - Free-form text input for manifest ID
   - Useful for community-sourced IDs or undocumented branches
   - Selection → triggers downgrade

### Manifest Selection Flow

When user selects a manifest (from any source):

1. Gather required parameters from GameDetail context (passed via App.tsx)
2. Call `start_downgrade` IPC command with DowngradeParams
3. On success, transition to "downgrade" view to show progress
4. On error, show error message with retry option

### Implementation Details

**VersionSelect props:**
```typescript
interface VersionSelectProps {
  game: GameInfo;
  selectedManifestId: string | null;
  onSelectManifest: (manifestId: string) => void;
  onAuthRequired?: () => void;
  onDowngradeStart?: () => void;  // NEW: called when downgrade begins
}
```

**SteamDBWebview component:**
```typescript
interface SteamDBWebviewProps {
  depotId: string;
  onSelectManifest: (manifestId: string, date?: string) => void;
  onError?: (error: string) => void;
}
```

**Webview injection strategy:**
- Load `https://steamdb.info/depot/<depotId>/manifests/` in a Tauri webview
- Wait for page load, inject JS to find the manifests table
- Extract manifest ID, branch, date from each row
- Return as array of objects to React
- Display extracted list; user clicks to select
- Fallback: if injection fails, user can still copy manifest ID manually from webview

## 2. Downgrade Progress View

### Location
- `src/components/DowngradeProgress.tsx` (new)
- `src/hooks/useDowngradeProgress.ts` (new, handles Tauri event listening)

### Props

```typescript
interface DowngradeProgressProps {
  game: GameInfo;
  targetManifestId: string;
  onComplete: () => void;      // called when downgrade finishes successfully
  onError?: (error: string) => void;  // called if user cancels or error occurs
}
```

### Hook: useDowngradeProgress

Listens to `downgrade-progress` Tauri events and tracks state:

```typescript
interface UseDowngradeProgressResult {
  phase: "comparing" | "downloading" | "applying" | "complete" | "error" | null;
  percent?: number;           // 0-100, only during "downloading"
  bytesDownloaded?: number;
  bytesTotal?: number;
  eta?: string;               // Calculated from speed and remaining
  speed?: string;             // e.g., "45.2 MB/s"
  error?: string;             // Error message if phase === "error"
  cancel: () => void;
}
```

Subscriptions:
- Listen to `downgrade-progress` event (emitted by Rust backend)
- Track phase transitions and update metrics
- Calculate ETA from speed and remaining bytes
- On `complete`: call onComplete()
- On `error`: show error message

### UI Layout

**Comparing Phase:**
```
┌─────────────────────────────────┐
│ Downgrading [Game Name]         │
├─────────────────────────────────┤
│                                 │
│  🔄 Comparing manifests...      │
│                                 │
│  Fetching version information   │
│  and calculating differences.   │
│                                 │
│        [Cancel] [X close]       │
└─────────────────────────────────┘
```

**Downloading Phase:**
```
┌─────────────────────────────────┐
│ Downgrading [Game Name]         │
├─────────────────────────────────┤
│                                 │
│  ⬇️  Downloading files (45%)    │
│                                 │
│  [████████░░░░░░░░░░] 45%       │
│                                 │
│  2.4 GB / 5.3 GB                │
│  Speed: 12.5 MB/s               │
│  ETA: ~4 min                    │
│                                 │
│        [Cancel] [X close]       │
└─────────────────────────────────┘
```

**Applying Phase:**
```
┌─────────────────────────────────┐
│ Downgrading [Game Name]         │
├─────────────────────────────────┤
│                                 │
│  ✓ Download complete            │
│                                 │
│  🔧 Applying files...           │
│                                 │
│  Copying files, patching ACF,   │
│  and updating manifest lock.    │
│                                 │
│        [Cancel]                 │
└─────────────────────────────────┘
```

**Completion State:**
```
┌─────────────────────────────────┐
│ Downgrading [Game Name]         │
├─────────────────────────────────┤
│                                 │
│  ✅ Downgrade Complete          │
│                                 │
│  Successfully downgraded        │
│  [Game Name] to manifest        │
│  1234567890                     │
│                                 │
│  ⚠️  Important:                 │
│  Set Steam's update preference  │
│  to "Only update when I launch" │
│  to prevent automatic updates.  │
│                                 │
│  [ Return to Game ] [ Back ]    │
└─────────────────────────────────┘
```

**Error State:**
```
┌─────────────────────────────────┐
│ Downgrading [Game Name]         │
├─────────────────────────────────┤
│                                 │
│  ❌ Downgrade Failed            │
│                                 │
│  Error: [error message]         │
│                                 │
│  [ Retry ] [ Back ]             │
└─────────────────────────────────┘
```

### Key Features

- **Real-time progress**: Update on every Tauri event
- **Cancellation**: Cancel button sends signal to backend to abort download
- **Graceful degradation**: If ETA calculation fails, just omit it
- **No minimization during download**: Warn user if they close the app (optional OS notification)

## 3. Navigation Updates

### App.tsx changes

Add "downgrade" to ViewId enum:

```typescript
export type ViewId =
  | "auth-gate"
  | "game-library"
  | "game-detail"
  | "version-select"
  | "downgrade";  // NEW
```

Add state for tracking downgrade context:

```typescript
const [downgradeContext, setDowngradeContext] = useState<{
  game: GameInfo;
  targetManifestId: string;
} | null>(null);
```

Update handleChangeVersion and manifest selection flow:

```typescript
const handleSelectManifest = useCallback(
  (manifestId: string) => {
    if (selectedGame) {
      setDowngradeContext({ game: selectedGame, targetManifestId: manifestId });
      setCurrentView("downgrade");
    }
  },
  [selectedGame]
);

const handleDowngradeComplete = useCallback(() => {
  setDowngradeContext(null);
  setCurrentView("game-detail");
}, []);
```

Update view routing:

```typescript
{selectedGame && currentView === "downgrade" && downgradeContext && (
  <DowngradeProgress
    game={downgradeContext.game}
    targetManifestId={downgradeContext.targetManifestId}
    onComplete={handleDowngradeComplete}
  />
)}
```

## 4. Type Definitions

### Frontend types (src/types/)

**downgrade.ts** (new):
```typescript
export interface DowngradeProgressEvent {
  phase: "comparing" | "downloading" | "applying" | "complete" | "error";
  percent?: number;
  bytes_downloaded?: number;
  bytes_total?: number;
  message?: string;  // For error phase
}
```

**steamdb.ts** (new):
```typescript
export interface SteamDBManifest {
  manifest_id: string;
  branch?: string;
  date?: string;
  pwd_required?: boolean;
}
```

### Backend types (already exist)

From `src-tauri/src/domain/downgrade.rs`:

- `DowngradeProgress` enum (serialized to "downgrade-progress" event)
- `DowngradeParams` struct (passed to `start_downgrade` command)

## 5. Error Handling

### Frontend Errors

- **Manifest fetch failure**: Show retry button in VersionSelect
- **Webview injection failure**: Log to console, show fallback message, user can still use manual input
- **Downgrade start failure**: Show error modal with suggestion to check logs
- **Downgrade cancellation**: Show option to retry or go back

### Backend Errors

All errors from the Rust downgrade pipeline are:
1. Captured as `DowngradeProgress::Error { message }`
2. Emitted on `downgrade-progress` event
3. Displayed to user in error state
4. User can retry (which re-runs entire pipeline from start)

## 6. Styling Considerations

- DowngradeProgress should match existing GameDetail/VersionSelect styling
- Progress bar uses existing color theme
- Icon set: lucide-react (consistent with existing components)
- Responsive: works on all window sizes

## 7. Testing Strategy

- Unit tests for useDowngradeProgress hook (mock Tauri events)
- Integration tests for DowngradeProgress component (simulate event sequences)
- E2E test: Full downgrade flow in Tauri dev environment

## 8. Accessibility

- All phases should have ARIA labels for screen readers
- Progress percentage announced on update
- Error messages are clear and actionable
- Keyboard navigation: Tab through buttons, Enter to select
