# Changelog

## [0.5.2] - 2026-04-03

### Fixed
- CI macOS builds now set `PKG_CONFIG_PATH` so the linker can find Homebrew's keg-only libarchive
- Added CI workflow that runs builds on `hotfix/**`, `release/**` pushes and PRs targeting `main` or `next`

## [0.5.1] - 2026-04-03

### Fixed
- CI release builds now install `libarchive` on Linux and macOS before compiling, fixing failed binary builds for v0.5.0

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
