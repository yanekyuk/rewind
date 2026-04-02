# ReShade Design

**Date:** 2026-04-02  
**Status:** Approved

## Overview

Add ReShade support to rewind: automatically download the official ReShade DLL in the background, symlink it into the game directory, and (on Linux) display the Steam launch command needed to activate it. ReShade can be enabled or disabled per game without reinstalling.

## Decisions

- **Source:** Official ReShade installer from reshade.me (extract `ReShade64.dll` from the embedded NSIS 7z stream using `sevenz-rust`)
- **Delivery:** Symlink from game dir → cached DLL in `bin_dir` (consistent with manifest file symlinks)
- **Shaders:** Optional community shader pack (`crosire/reshade-shaders`) downloaded once to `cache_dir/reshade-shaders/`, symlinked per game
- **API selection:** User picks per game (Dxgi / D3d9 / OpenGl32 / Vulkan1); stored in config
- **Toggle:** Enable/disable per game (place or remove symlinks); state persisted in `games.toml`
- **UI entry:** `[R]` keybind on the main screen game detail panel

## Architecture

### `rewind-core/src/reshade.rs` (new module)

**Types:**

`ReshadeApi` and `ReshadeEntry` are defined in `config.rs` (see below) and imported here.

```rust
// Defined in this module:
pub enum ReshadeProgress { Line(String), Done, Error(String) }

pub enum ReshadeError { Io(std::io::Error), Http(reqwest::Error), ExtractionFailed, NotFound, SymlinkConflict }

// impl block on ReshadeApi (defined in config.rs, impl added here):
impl ReshadeApi {
    pub fn dll_name(&self) -> &'static str
    // Returns: "dxgi.dll", "d3d9.dll", "opengl32.dll", "vulkan-1.dll"

    pub fn linux_launch_command(&self) -> String
    // Returns e.g.: WINEDLLOVERRIDES="dxgi=n,b" %command%
    // Available on all platforms; UI display is gated with #[cfg(target_os = "linux")]
}
```

**Functions:**

```rust
pub fn reshade_dll_path(bin_dir: &Path) -> PathBuf
// bin_dir/ReShade64.dll

pub fn reshade_shaders_cache_path(cache_dir: &Path) -> PathBuf
// cache_dir/reshade-shaders/

pub async fn download_reshade(bin_dir: &Path, tx: Sender<ReshadeProgress>) -> Result<PathBuf, ReshadeError>
// If bin_dir/ReShade64.dll already exists, sends Done and returns immediately.
// Otherwise: fetches the ReShade installer .exe from reshade.me,
// locates the embedded 7z stream (NSIS stores LZMA payload after the PE header,
// identified by the 7z magic bytes `37 7A BC AF 27 1C`),
// extracts ReShade64.dll using sevenz-rust, writes to bin_dir, marks executable.

pub async fn download_shaders(shaders_dir: &Path, tx: Sender<ReshadeProgress>) -> Result<(), ReshadeError>
// If shaders_dir already exists, sends Done and returns immediately.
// Otherwise: downloads reshade-shaders zip from github.com/crosire/reshade-shaders,
// extracts to shaders_dir.

pub fn enable_reshade(
    game_dir: &Path,
    api: &ReshadeApi,
    reshade_dll: &Path,
    shaders_src: Option<&Path>,
) -> Result<(), ReshadeError>
// Symlinks reshade_dll → game_dir/api.dll_name().
// Returns ReshadeError::SymlinkConflict if a real (non-symlink) file already exists at that path.
// If shaders_src is Some, also symlinks shaders_src → game_dir/reshade-shaders.

pub fn disable_reshade(game_dir: &Path, api: &ReshadeApi) -> Result<(), ReshadeError>
// Removes game_dir/api.dll_name() if it is a symlink.
// Removes game_dir/reshade-shaders if it is a symlink.
// No-ops if the symlinks don't exist.
```

Export `reshade` from `rewind-core/src/lib.rs`.

### `rewind-core/src/config.rs` (additions)

```rust
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReshadeApi { Dxgi, D3d9, OpenGl32, Vulkan1 }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReshadeEntry {
    pub api: ReshadeApi,
    pub enabled: bool,
    pub shaders_enabled: bool,
}
```

`GameEntry` gains:
```rust
#[serde(default)]
pub reshade: Option<ReshadeEntry>,
```

`#[serde(default)]` ensures existing `games.toml` files without a `reshade` key deserialize without error.

### `rewind-cli/src/app.rs` (additions)

New screen variant:
```rust
pub enum Screen {
    // existing variants unchanged
    ReshadeSetup,
}
```

New state:
```rust
#[derive(Debug, Default)]
pub enum ReshadeSetupStep { #[default] PickApi, ConfirmShaders, Downloading }

#[derive(Debug, Default)]
pub struct ReshadeSetupState {
    pub step: ReshadeSetupStep,
    pub selected_api: usize,      // index into [Dxgi, D3d9, OpenGl32, Vulkan1]
    pub download_shaders: bool,
    pub lines: Vec<String>,       // streamed progress lines
    pub done: bool,
    pub error: Option<String>,
}
```

Added to `App`:
```rust
pub reshade_state: ReshadeSetupState,
pub reshade_progress_rx: Option<mpsc::Receiver<ReshadeProgress>>,
```

### `rewind-cli/src/ui/reshade_setup.rs` (new screen)

Rendered as a centered overlay (same visual pattern as `switch_overlay.rs`).

- **PickApi step:** 4-item list `[Dxgi, D3d9, OpenGl32, Vulkan1]`, `↑↓/jk` to navigate, `Enter` advances to `ConfirmShaders`, `Esc` returns to `Main`
- **ConfirmShaders step:** Prompt "Download community shader pack? [Y/N]", sets `download_shaders`, `Enter`/`Y`/`N` advances to `Downloading`, spawns background tokio task
- **Downloading step:** Streams `ReshadeProgress` lines into `reshade_state.lines`; on `Done` sets `done = true` and saves `ReshadeEntry` to config; on `Error` sets `error`
- `Esc` during `Downloading` is ignored (can't cancel mid-download); `Esc` after `done` or on `error` returns to `Main`

### `rewind-cli/src/ui/main_screen.rs` (additions)

**Detail panel** gains a ReShade status line:
- `reshade.is_none()` → `ReShade: not installed  [R] set up`
- `reshade.enabled` → `ReShade: enabled  [R] disable`
- `!reshade.enabled` → `ReShade: disabled  [R] enable`

On Linux, when `reshade.enabled`:
```
Launch options: WINEDLLOVERRIDES="dxgi=n,b" %command%
```
Displayed in a dimmed style as a hint to paste into Steam's game properties.

The `[R]` hint is gated — only shown when a game entry exists in `games_config` (ReShade requires a tracked game).

**Status bar** gains `[R] reshade` to the existing hint line.

### `[R]` key handling (`main.rs`)

```
if no game_entry → do nothing (ReShade requires a tracked game)
else if reshade.is_none() → reset reshade_state, transition to Screen::ReshadeSetup
else if reshade.enabled  → disable_reshade(...), set enabled=false, save_games(...)
else                      → enable_reshade(...),  set enabled=true,  save_games(...)
```

Enable/disable errors (e.g. `SymlinkConflict`) are shown as an inline error in the detail panel.

## Error Handling

| Scenario | Behavior |
|---|---|
| DLL already in `bin_dir` | `download_reshade` skips download, returns immediately |
| Shaders already cached | `download_shaders` skips download, returns immediately |
| `dxgi.dll` is a real file in game dir | `enable_reshade` returns `SymlinkConflict`; shown as inline error in detail panel |
| reshade.me URL unreachable | Error shown in setup overlay with message: "Download failed — place ReShade64.dll manually in `~/.local/share/rewind/bin/`" |
| Disable with game running | No detection; same policy as the rest of rewind |

## Steam Launch Options Integration (Linux)

On Linux, after enabling ReShade, rewind automatically writes `WINEDLLOVERRIDES="<api>=n,b" %command%` to the game's `LaunchOptions` in Steam's `localconfig.vdf`. This removes the need to copy-paste the command manually.

### `rewind-core/src/localconfig.rs` (new module)

`localconfig.vdf` lives at `<Steam data dir>/userdata/<steamid>/config/localconfig.vdf`. The Steam data dir is found via `steamlocate::SteamDir` (already a dependency), making the path resolution cross-platform.

```rust
pub fn find_localconfig_paths() -> Vec<PathBuf>
// Uses SteamDir to find the Steam root, then scans userdata/<id>/config/localconfig.vdf
// for each Steam account directory. Cross-platform.

pub fn read_launch_options(path: &Path, app_id: u32) -> Option<String>
// Parses the VDF file and returns the LaunchOptions value for the given app_id.

pub fn write_launch_options(path: &Path, app_id: u32, options: &str) -> Result<(), LocalConfigError>
// Replaces (or inserts) the LaunchOptions key for the given app_id in the VDF file.
```

VDF parsing uses a simple line-by-line state machine — no additional crate required. The VDF KeyValues format is regular enough: quoted string keys/values, `{`/`}` for nesting.

### `ReshadeEntry` addition

```rust
pub struct ReshadeEntry {
    pub api: ReshadeApi,
    pub enabled: bool,
    pub shaders_enabled: bool,
    #[serde(default)]
    pub original_launch_options: Option<String>,  // saved before we overwrite
}
```

### On enable (Linux only, in `finalize_reshade`)

1. Call `find_localconfig_paths()` — use the first path found that contains the app
2. Call `read_launch_options(path, app_id)` — save result to `ReshadeEntry.original_launch_options`
3. Call `write_launch_options(path, app_id, api.linux_launch_command())` — writes `WINEDLLOVERRIDES="dxgi=n,b" %command%`
4. If no localconfig found or write fails: log silently, show the old hint in the detail panel as fallback

### On disable (Linux only, in `[R]` disable branch)

1. Call `find_localconfig_paths()`, `write_launch_options(path, app_id, original)` where `original` is `reshade_entry.original_launch_options.as_deref().unwrap_or("")`

### Detail panel (Linux)

- When enabled and localconfig write succeeded: `Launch options: written automatically`
- When enabled and localconfig write failed (or no localconfig found): show `WINEDLLOVERRIDES="..." %command%` as the manual hint (existing behaviour)

## Error Handling

| Scenario | Behavior |
|---|---|
| DLL already in `bin_dir` | `download_reshade` skips download, returns immediately |
| Shaders already cached | `download_shaders` skips download, returns immediately |
| `dxgi.dll` is a real file in game dir | `enable_reshade` returns `SymlinkConflict`; shown as inline error in detail panel |
| reshade.me URL unreachable | Error shown in setup overlay with message: "Download failed — place ReShade64.dll manually in `~/.local/share/rewind/bin/`" |
| localconfig.vdf not found | Silent — show manual launch hint in detail panel |
| localconfig.vdf write fails | Silent — show manual launch hint in detail panel |
| Disable with game running | No detection; same policy as the rest of rewind |

## Platform Notes

- `ReshadeApi::linux_launch_command` is available on all platforms but only used on Linux
- `localconfig.vdf` path resolution is cross-platform (via `steamlocate::SteamDir`)
- Launch options are only written/restored on Linux (`#[cfg(target_os = "linux")]`)
- Windows and macOS: DLL symlink is sufficient; no launch options written
- Only 64-bit (`ReShade64.dll`) is supported; 32-bit games are out of scope
