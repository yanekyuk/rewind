# TUI Download Experience — Design Spec

## Problem

DepotDownloader output is currently shown in the raw terminal by suspending the TUI. This breaks the user experience — progress is not visible inside the app, and users see no feedback for post-download steps (backup, linking, patching, locking).

## Solution

Keep the entire download flow inside the TUI with a dual-log layout:

1. **Our process steps** (top) — high-level numbered steps with status indicators
2. **DepotDownloader output** (bottom) — raw output in a bordered pane with dim/muted text

Credentials are handled by piping stdin to DepotDownloader and detecting prompt lines. A fallback to terminal-suspend mode is offered if the process appears stuck.

---

## Wizard Layout (During Download)

```
┌─ Downgrade Wizard ──────────────────────────┐
│ SteamDB URL: https://steamdb.info/...       │
│ Manifest ID: [___________________________]  │
│                                              │
│  [✓] Checking .NET runtime                  │
│  [✓] Downloading DepotDownloader             │
│  […] Downloading manifest files...           │
│  [ ] Backing up current files                │
│  [ ] Linking manifest files to game dir      │
│  [ ] Patching Steam manifest                 │
│  [ ] Locking manifest file                   │
│                                              │
│ ┌─ DepotDownloader ────────────────────────┐ │
│ │ Downloading depot 123456...              │ │
│ │ 45.2% | 234.5 MB / 518.7 MB             │ │
│ │                                          │ │
│ └──────────────────────────────────────────┘ │
│                                              │
│  [Esc] Cancel                                │
└──────────────────────────────────────────────┘
```

- Our process steps sit above the DepotDownloader pane
- DepotDownloader pane has a visible border with title "DepotDownloader"
- DepotDownloader text is dim/dark gray (muted, less prominent)
- DepotDownloader pane auto-scrolls to latest output

---

## Process Steps

These phases are shown in order with status indicators:

| # | Step | Source |
|---|------|--------|
| 1 | Checking .NET runtime | `depot::check_dotnet()` |
| 2 | Downloading DepotDownloader | `depot::ensure_depot_downloader()` (skipped if present) |
| 3 | Downloading manifest files | DepotDownloader process (DepotDownloader pane active) |
| 4 | Backing up current files | `cache::apply_downloaded()` — copy phase |
| 5 | Linking manifest files to game directory | `cache::apply_downloaded()` — symlink phase |
| 6 | Patching Steam manifest | `patcher::patch_acf_file()` |
| 7 | Locking manifest file | `immutability::lock_file()` |

### Step Indicators

- `[ ]` — pending (dim)
- `[…]` — in progress (yellow/spinner)
- `[✓]` — completed (green)
- `[✗]` — failed (red, error shown inline or in DepotDownloader pane)

Steps 1–2 already flow through the progress channel. Steps 4–7 currently run synchronously in `finalize_downgrade_from()` with no feedback — they will now send progress updates.

---

## Credential Handling

### Primary Path (Approach A)

1. Read DepotDownloader stdout/stderr byte-by-byte (not line-by-line) to catch prompts like `Password:` that don't end with a newline
2. When a prompt is detected (keywords: `password`, `guard`, `2fa`, `code`), send `DepotProgress::Prompt(String)`
3. Wizard shows a text input field below the DepotDownloader pane, labeled with the prompt text
4. User types response, presses Enter → written to DepotDownloader's stdin + newline
5. Input field disappears after submission

### Fallback (Approach C)

If DepotDownloader produces no output for 30+ seconds during the download phase (suggesting an undetected prompt or `Console.ReadKey()` usage), show:

> "DepotDownloader may be waiting for input. Press [R] to restart with terminal mode."

This restarts the download using the current TUI-suspend approach (inherited stdio).

Note: `-remember-password` is always passed, so most runs after first login will not need credentials at all.

---

## Implementation Changes

### rewind-core/src/depot.rs

- Add `DepotProgress::Prompt(String)` variant to the enum
- Modify `run_depot_downloader()` to also pipe stdin; return a stdin handle so the caller can write to it
- Switch from line-based `BufReader::lines()` to byte-level reading that flushes partial lines on short reads (to detect prompts without trailing newlines)

### rewind-cli/src/app.rs

- Add to `DowngradeWizardState`:
  - `steps: Vec<StepStatus>` — status of each process step
  - `prompt_input: Option<String>` — current credential input text
  - `depot_stdin: Option<ChildStdin>` — handle to DepotDownloader's stdin for credential forwarding
- Add `StepStatus` enum: `Pending`, `InProgress`, `Done`, `Failed(String)`

### rewind-cli/src/main.rs

- Remove the TUI suspend/resume path (the `pending_download` + `ratatui::restore()` logic)
- Run DepotDownloader via piped `run_depot_downloader()` instead of `run_depot_downloader_interactive()`
- After `DepotProgress::Done`, run finalize steps (backup, link, patch, lock) sending progress updates to the step list
- Add 30-second timeout detection → offer `[R]` to restart in terminal mode

### rewind-cli/src/ui/downgrade_wizard.rs

- New download layout: our steps list on top, bordered DepotDownloader pane below with dim text
- Render step indicators (`[ ]`, `[…]`, `[✓]`, `[✗]`) with appropriate colors
- Optional credential input field at bottom when `prompt_input` is `Some`
- DepotDownloader pane title: "DepotDownloader" with visible border
