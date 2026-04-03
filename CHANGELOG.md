# Changelog

## [0.6.0] - 2026-04-03

### Added
- **Game search / filter** — press `/` on the main screen to fuzzy-search your library by name; `Esc` clears the filter and restores your selection
- **Multiple Steam account support** — rewind now detects all accounts on the machine; pick your preferred account during first run or change it any time in Settings; launch options are loaded for the correct account automatically

## [0.5.2] - 2026-04-03

### Fixed
- Missing Linux and macOS binaries for v0.5.0 and v0.5.1 are now available — if you couldn't download those releases before, grab v0.5.2 instead

## [0.5.0] - 2026-04-02

### Added
- **ReShade integration** — install, enable, and disable ReShade post-processing for any game right from rewind; press `R` on the main screen to get started
- **ReShade shader support** — a curated set of shaders is fetched automatically so you can start experimenting immediately
- **Steam conflict detection** — rewind now warns you if Steam is open before starting a download or version switch, preventing save and config conflicts
- **Launch options display** — your per-game Steam launch options are shown in the game detail panel so you always know what flags are active

### Fixed
- ReShade installation on Linux no longer requires any extra tools — everything is handled internally
- ReShade can now be applied to any installed game, not just ones you've previously downgraded with rewind
- Launch options containing quoted arguments are now parsed correctly
- rewind now shows a clear error message when it can't find a game to configure, instead of doing nothing

### Changed
- Setting launch options no longer writes to your Steam config automatically; instead, rewind shows you the exact command to run — this avoids conflicts when Steam is already open

## [0.4.0] - initial release
