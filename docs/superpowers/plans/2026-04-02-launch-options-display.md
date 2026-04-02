# Launch Options Display Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a game's Steam launch options (read-only) in the detail panel on the main screen.

**Architecture:** Add a private VDF parser for `localconfig.vdf` and two public functions to `rewind-core/src/scanner.rs`; add a per-appid cache to `App` in `rewind-cli`; populate lazily in `draw_detail_panel` and display as a wrapped line.

**Tech Stack:** Rust, ratatui, steamlocate (already a dep of rewind-core), hand-written line-by-line VDF parser (existing pattern in scanner.rs)

---

## File Map

| File | Change |
|------|--------|
| `rewind-core/src/scanner.rs` | Add `extract_launch_options_from_vdf` (private), `read_launch_options` (pub), `find_launch_options` (pub) |
| `rewind-cli/src/app.rs` | Add `launch_options_cache: HashMap<u32, Option<String>>` to `App` |
| `rewind-cli/src/ui/main_screen.rs` | Populate cache on miss; display launch options line |

---

## Task 1: VDF parser for localconfig.vdf

**Files:**
- Modify: `rewind-core/src/scanner.rs`

The existing ACF parser uses a line-by-line depth-tracking approach. This task adds the same pattern for `localconfig.vdf`, which nests the `LaunchOptions` key five levels deep under `apps > <appid>`.

Depth tracking convention (same as existing `extract_first_depot`):
- On `{`: increment depth, then record block depth if entering a tracked section
- On `}`: decrement depth, then exit tracked section if depth dropped below its recorded depth

- [ ] **Step 1: Write the failing test**

Add this test to the `#[cfg(test)]` block at the bottom of `rewind-core/src/scanner.rs`:

```rust
#[test]
fn extract_launch_options_finds_value() {
    let vdf = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "12345"
                    {
                        "LaunchOptions"		"-novid %command%"
                    }
                }
            }
        }
    }
}"#;
    assert_eq!(
        extract_launch_options_from_vdf(vdf, 12345),
        Some("-novid %command%".to_string())
    );
}

#[test]
fn extract_launch_options_returns_none_for_missing_app() {
    let vdf = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "99999"
                    {
                        "LaunchOptions"		"-novid %command%"
                    }
                }
            }
        }
    }
}"#;
    assert_eq!(extract_launch_options_from_vdf(vdf, 12345), None);
}

#[test]
fn extract_launch_options_returns_none_for_empty_value() {
    let vdf = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "12345"
                    {
                        "LaunchOptions"		""
                    }
                }
            }
        }
    }
}"#;
    assert_eq!(extract_launch_options_from_vdf(vdf, 12345), None);
}

#[test]
fn extract_launch_options_returns_none_for_absent_key() {
    let vdf = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "12345"
                    {
                        "LastPlayed"		"1234567890"
                    }
                }
            }
        }
    }
}"#;
    assert_eq!(extract_launch_options_from_vdf(vdf, 12345), None);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```sh
cargo test -p rewind-core extract_launch_options
```

Expected: compile error — `extract_launch_options_from_vdf` not yet defined.

- [ ] **Step 3: Implement the parser**

Add this private function to `rewind-core/src/scanner.rs` (place it before the `#[cfg(test)]` block, alongside the other private helpers):

```rust
fn extract_launch_options_from_vdf(content: &str, app_id: u32) -> Option<String> {
    let app_id_key = format!("\"{}\"", app_id);
    let mut in_apps = false;
    let mut in_target_app = false;
    let mut depth = 0i32;
    let mut apps_depth = -1i32;
    let mut app_depth = -1i32;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "{" {
            depth += 1;
            if in_apps && !in_target_app && apps_depth < 0 {
                apps_depth = depth;
            }
            if in_target_app && app_depth < 0 {
                app_depth = depth;
            }
            continue;
        }

        if trimmed == "}" {
            depth -= 1;
            if in_target_app && depth < app_depth {
                in_target_app = false;
                app_depth = -1;
            }
            if in_apps && !in_target_app && depth < apps_depth {
                in_apps = false;
                apps_depth = -1;
            }
            continue;
        }

        if !in_apps && trimmed == "\"apps\"" {
            in_apps = true;
            continue;
        }

        if in_apps && !in_target_app && trimmed == app_id_key {
            in_target_app = true;
            continue;
        }

        if in_target_app && trimmed.starts_with("\"LaunchOptions\"") {
            let rest = &trimmed["\"LaunchOptions\"".len()..];
            if let Some(val) = extract_quoted(rest) {
                return if val.is_empty() { None } else { Some(val.to_string()) };
            }
        }
    }
    None
}
```

- [ ] **Step 4: Run tests to verify they pass**

```sh
cargo test -p rewind-core extract_launch_options
```

Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```sh
git add rewind-core/src/scanner.rs
git commit -m "feat: add localconfig.vdf launch options parser"
```

---

## Task 2: Public functions for reading launch options

**Files:**
- Modify: `rewind-core/src/scanner.rs`

Expose two public functions: one path-based (testable) and one convenience wrapper that resolves the Steam root via `steamlocate` (used by rewind-cli).

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block in `rewind-core/src/scanner.rs`:

```rust
#[test]
fn read_launch_options_finds_value_in_userdata() {
    let tmp = TempDir::new().unwrap();
    let user_dir = tmp.path().join("userdata").join("123456").join("config");
    fs::create_dir_all(&user_dir).unwrap();

    let vdf_content = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "42"
                    {
                        "LaunchOptions"		"DXVK_ASYNC=1 %command%"
                    }
                }
            }
        }
    }
}"#;
    fs::write(user_dir.join("localconfig.vdf"), vdf_content).unwrap();

    assert_eq!(
        read_launch_options(tmp.path(), 42),
        Some("DXVK_ASYNC=1 %command%".to_string())
    );
}

#[test]
fn read_launch_options_returns_none_when_no_userdata() {
    let tmp = TempDir::new().unwrap();
    // No userdata dir created
    assert_eq!(read_launch_options(tmp.path(), 42), None);
}

#[test]
fn read_launch_options_returns_none_when_app_not_found() {
    let tmp = TempDir::new().unwrap();
    let user_dir = tmp.path().join("userdata").join("123456").join("config");
    fs::create_dir_all(&user_dir).unwrap();

    let vdf_content = r#""UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "99"
                    {
                        "LaunchOptions"		"-novid"
                    }
                }
            }
        }
    }
}"#;
    fs::write(user_dir.join("localconfig.vdf"), vdf_content).unwrap();

    assert_eq!(read_launch_options(tmp.path(), 42), None);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```sh
cargo test -p rewind-core read_launch_options
```

Expected: compile error — `read_launch_options` not yet defined.

- [ ] **Step 3: Implement the public functions**

Add to `rewind-core/src/scanner.rs` (before the `#[cfg(test)]` block):

```rust
/// Read launch options for a game from the most recently modified localconfig.vdf
/// found under `steam_root/userdata/*/config/localconfig.vdf`.
/// Returns `None` if not found, Steam root has no userdata, or options are empty.
pub fn read_launch_options(steam_root: &Path, app_id: u32) -> Option<String> {
    let userdata = steam_root.join("userdata");
    let entries = std::fs::read_dir(&userdata).ok()?;

    let mut best: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for entry in entries.flatten() {
        let vdf_path = entry.path().join("config").join("localconfig.vdf");
        if vdf_path.exists() {
            if let Ok(meta) = std::fs::metadata(&vdf_path) {
                if let Ok(mtime) = meta.modified() {
                    if best.as_ref().map_or(true, |(t, _)| mtime > *t) {
                        best = Some((mtime, vdf_path));
                    }
                }
            }
        }
    }

    let (_, vdf_path) = best?;
    let content = std::fs::read_to_string(&vdf_path).ok()?;
    extract_launch_options_from_vdf(&content, app_id)
}

/// Convenience wrapper: resolves Steam root via steamlocate, then calls `read_launch_options`.
/// Returns `None` if Steam is not found or the game has no launch options set.
pub fn find_launch_options(app_id: u32) -> Option<String> {
    use steamlocate::SteamDir;
    let steam_dir = SteamDir::locate().ok()?;
    read_launch_options(steam_dir.path(), app_id)
}
```

- [ ] **Step 4: Run tests to verify they pass**

```sh
cargo test -p rewind-core read_launch_options
```

Expected: all 3 tests pass.

- [ ] **Step 5: Run all rewind-core tests to check for regressions**

```sh
cargo test -p rewind-core
```

Expected: all tests pass (2 immutability tests may fail on macOS — known issue, acceptable).

- [ ] **Step 6: Commit**

```sh
git add rewind-core/src/scanner.rs
git commit -m "feat: expose read_launch_options and find_launch_options in scanner"
```

---

## Task 3: Add launch options cache to App state

**Files:**
- Modify: `rewind-cli/src/app.rs`

- [ ] **Step 1: Add the cache field to the `App` struct**

In `rewind-cli/src/app.rs`, find the `App` struct (line 130) and add the field after `image_picker`:

```rust
pub struct App {
    // ... existing fields ...
    pub image_picker: Option<ratatui_image::picker::Picker>,
    /// Launch options per appid, loaded lazily. Missing = not yet attempted.
    /// Some(None) = loaded, no options. Some(Some(s)) = options string s.
    pub launch_options_cache: HashMap<u32, Option<String>>,
    // ... rest of fields ...
}
```

- [ ] **Step 2: Initialize the field in `App::new`**

In the `App::new` function body, add:

```rust
launch_options_cache: HashMap::new(),
```

- [ ] **Step 3: Verify it compiles**

```sh
cargo check -p rewind-cli
```

Expected: no errors.

- [ ] **Step 4: Commit**

```sh
git add rewind-cli/src/app.rs
git commit -m "feat: add launch_options_cache to App state"
```

---

## Task 4: Display launch options in the detail panel

**Files:**
- Modify: `rewind-cli/src/ui/main_screen.rs`

`draw_detail_panel` already takes `&mut App`, so it can populate the cache on first render for a given appid. The existing `Wrap { trim: false }` paragraph handles long lines automatically.

- [ ] **Step 1: Add the cache population and launch options line**

In `rewind-cli/src/ui/main_screen.rs`, find `draw_detail_panel` (line 127). After `game_app_id` is set (around line 144) and before `let text = format!(...)`, add:

```rust
// Populate launch options cache on first access for this appid.
if !app.launch_options_cache.contains_key(&game_app_id) {
    let opts = rewind_core::scanner::find_launch_options(game_app_id);
    app.launch_options_cache.insert(game_app_id, opts);
}

let launch_line = match app.launch_options_cache.get(&game_app_id) {
    None => "\n  Launch:    \u{2026}".to_string(),         // "…" — still loading (shouldn't happen after insert above)
    Some(None) => String::new(),
    Some(Some(s)) => format!("\n  Launch:    {}", s),
};
```

- [ ] **Step 2: Add `launch_line` to the format string**

Replace the existing `let text = format!(...)` block with:

```rust
let text = format!(
    "  {name}\n  App ID:    {app_id}\n  Depot:     {depot_id}\n\n  Status:    {status}\n  Installed: {active}{spoofed}\n  Cached:    {cached}{launch}",
    name = game.name,
    app_id = game.app_id,
    depot_id = game.depot_id,
    status = status_line,
    active = active_manifest,
    spoofed = spoofed_line,
    cached = cached_list,
    launch = launch_line,
);
```

Note: the action hints (`[D] Download...` etc.) are appended after `{launch}` — preserve them by continuing the format string:

```rust
let text = format!(
    "  {name}\n  App ID:    {app_id}\n  Depot:     {depot_id}\n\n  Status:    {status}\n  Installed: {active}{spoofed}\n  Cached:    {cached}{launch}\n\n  [D] Download new version\n  [U] Switch version\n  [O] Open app on SteamDB",
    name = game.name,
    app_id = game.app_id,
    depot_id = game.depot_id,
    status = status_line,
    active = active_manifest,
    spoofed = spoofed_line,
    cached = cached_list,
    launch = launch_line,
);
```

- [ ] **Step 3: Verify it compiles**

```sh
cargo check -p rewind-cli
```

Expected: no errors.

- [ ] **Step 4: Build and smoke-test manually**

```sh
cargo build
```

Run the binary and navigate to a game with known launch options (e.g. NBA 2K26, appid 3472040). Verify the `Launch:` line appears with the correct options. Navigate to a game with no launch options set — verify the line is absent.

- [ ] **Step 5: Commit**

```sh
git add rewind-cli/src/ui/main_screen.rs
git commit -m "feat: display launch options in game detail panel"
```

---

## Task 5: Bump versions and finish

**Files:**
- Modify: `rewind-core/Cargo.toml`
- Modify: `rewind-cli/Cargo.toml`

Per CLAUDE.md, versions must be bumped when features are added before merging.

- [ ] **Step 1: Bump patch version in rewind-core**

In `rewind-core/Cargo.toml`, increment the patch version (e.g. `0.4.1` → `0.4.2`).

- [ ] **Step 2: Bump patch version in rewind-cli**

In `rewind-cli/Cargo.toml`, increment the patch version by the same amount, and update the `rewind-core` dependency version to match.

- [ ] **Step 3: Verify everything builds and tests pass**

```sh
cargo build && cargo test
```

Expected: build succeeds, all tests pass.

- [ ] **Step 4: Commit**

```sh
git add rewind-core/Cargo.toml rewind-cli/Cargo.toml
git commit -m "chore: bump versions to 0.4.2 for launch options feature"
```
