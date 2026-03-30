---
title: "Embedded Progress UI and Background Notifications"
type: decision
tags: [progress, ui, notifications, ux, downloads]
created: 2026-03-30
updated: 2026-03-30
---

# Embedded Progress UI and Background Notifications

## Context

Downloading game files via DepotDownloader can take a long time -- tens of GB over consumer internet connections may take hours. Users need visibility into download progress and the freedom to do other things while waiting.

## Decision

Provide an embedded progress UI within the app window and background OS notifications for key events.

## Design

### Embedded Progress UI

- Real-time progress bar showing bytes downloaded vs total.
- Current file being downloaded and estimated time remaining.
- Download speed indicator.
- Cancel button to abort the download at any time.

Progress data is extracted by parsing DepotDownloader's stdout in the Rust backend and relaying it to the React frontend via Tauri IPC events.

### Background Notifications

When the app is minimized or not focused, send OS-level notifications for:

- Download completion.
- Download failure (with error summary).
- Steps requiring user action (e.g., "Close Steam to proceed with file application").

## Rationale

- **Long operations**: Downloads of 30-80 GB are common. Users will not stare at the app for the duration.
- **Transparency**: Showing progress builds trust that the tool is working correctly.
- **Cancellability**: Users must be able to abort if they realize they selected the wrong version or need the bandwidth.
- **Platform support**: Tauri 2 supports OS-level notifications on all target platforms via the notification plugin.
