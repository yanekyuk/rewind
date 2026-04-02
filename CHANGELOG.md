# Changelog

## [0.5.0] - 2026-04-02

### Added
- **ReShade integration** — download, install, enable, and disable ReShade for any installed game via the new `ReshadeSetup` overlay (`R` keybind from the main screen)
- **ReShade shader support** — automatically fetches shaders from the reshade-shaders slim branch
- **Steam process detection** — warns before download/switch operations when Steam is running to prevent conflicts
- **Launch options display** — shows configured Steam launch options in the game detail panel

### Fixed
- NSIS installer extraction on Linux now uses `libarchive` (compress-tools) — no external tools required
- ReShade can be applied to any installed game, not only rewind-tracked ones
- Launch option parsing correctly handles escaped quotes
- Missing `app_id` in `set_launch_options` is now detected and reported correctly

### Changed
- Replaced automatic `localconfig.vdf` writes with a manual launch command prompt to avoid conflicts with a running Steam client

## [0.4.0] - initial release
