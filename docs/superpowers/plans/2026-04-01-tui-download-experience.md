# TUI Download Experience Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the entire DepotDownloader download flow inside the TUI with a dual-log layout (our process steps + raw DepotDownloader output) and credential prompt forwarding.

**Architecture:** Replace the TUI-suspend approach with piped stdin/stdout/stderr. DepotDownloader output streams into a bordered pane in the wizard. Our high-level process steps (dotnet check, binary download, manifest download, backup, symlink, patch, lock) are shown above with status indicators. Credential prompts are detected via byte-level reading and forwarded through a TUI input field. A 30-second timeout offers fallback to terminal mode.

**Tech Stack:** Rust, ratatui, tokio (async byte-level I/O, mpsc channels), crossterm

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `rewind-core/src/depot.rs` | Modify | Add `Prompt` variant, refactor `run_depot_downloader` for piped stdin + byte-level reading |
| `rewind-cli/src/app.rs` | Modify | Add `StepStatus` enum, new wizard state fields (`steps`, `depot_lines`, `prompt_input`, `depot_stdin`) |
| `rewind-cli/src/main.rs` | Modify | Replace TUI-suspend path with in-TUI streaming, add finalize steps with progress, add timeout + fallback |
| `rewind-cli/src/ui/downgrade_wizard.rs` | Modify | New dual-log layout with step indicators, bordered DepotDownloader pane, credential input field |

---

### Task 1: Add `StepStatus` enum and new wizard state fields to `app.rs`

**Files:**
- Modify: `rewind-cli/src/app.rs`

- [ ] **Step 1: Add `StepStatus` enum and `StepKind` enum**

At the top of `app.rs`, after the existing imports, add:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Done,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StepKind {
    CheckDotnet,
    DownloadDepot,
    DownloadManifest,
    BackupFiles,
    LinkFiles,
    PatchManifest,
    LockManifest,
}

impl StepKind {
    pub fn label(&self) -> &'static str {
        match self {
            StepKind::CheckDotnet => "Checking .NET runtime",
            StepKind::DownloadDepot => "Downloading DepotDownloader",
            StepKind::DownloadManifest => "Downloading manifest files",
            StepKind::BackupFiles => "Backing up current files",
            StepKind::LinkFiles => "Linking manifest files to game directory",
            StepKind::PatchManifest => "Patching Steam manifest",
            StepKind::LockManifest => "Locking manifest file",
        }
    }
}
```

- [ ] **Step 2: Update `DowngradeWizardState` with new fields**

Replace the existing `DowngradeWizardState` struct with:

```rust
#[derive(Debug, Default)]
pub struct DowngradeWizardState {
    pub manifest_input: String,
    pub steamdb_url: String,
    pub progress_lines: Vec<String>,
    pub is_downloading: bool,
    pub error: Option<String>,
    /// When set, pressing [O] opens this URL instead of the SteamDB manifests page.
    pub error_url: Option<String>,
    /// High-level process steps with their status.
    pub steps: Vec<(StepKind, StepStatus)>,
    /// Raw output lines from DepotDownloader (shown in the bordered pane).
    pub depot_lines: Vec<String>,
    /// When Some, a credential prompt is active and this holds the user's input so far.
    pub prompt_input: Option<String>,
    /// The prompt label from DepotDownloader (e.g. "Password:").
    pub prompt_label: Option<String>,
}
```

- [ ] **Step 3: Add `depot_stdin` field to `App`**

In the `App` struct, add after `pending_download`:

```rust
    /// Stdin handle for the running DepotDownloader process (used to forward credential input).
    pub depot_stdin: Option<tokio::process::ChildStdin>,
```

And initialize it as `None` in `App::new()`.

- [ ] **Step 4: Add helper method to update step status**

Add to the `impl App` block:

```rust
    pub fn set_step_status(&mut self, kind: &StepKind, status: StepStatus) {
        if let Some(step) = self.wizard_state.steps.iter_mut().find(|s| s.0 == *kind) {
            step.1 = status;
        }
    }
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check 2>&1`
Expected: compiles with no errors (warnings OK)

- [ ] **Step 6: Commit**

```bash
git add rewind-cli/src/app.rs
git commit -m "feat: add StepStatus/StepKind enums and new wizard state fields"
```

---

### Task 2: Add `Prompt` variant and refactor `run_depot_downloader` for piped stdin + byte-level reading

**Files:**
- Modify: `rewind-core/src/depot.rs`

- [ ] **Step 1: Add `Prompt` variant to `DepotProgress`**

In `depot.rs`, update the `DepotProgress` enum:

```rust
#[derive(Debug, Clone)]
pub enum DepotProgress {
    /// A status/info line to display while preparing the download.
    Line(String),
    /// DepotDownloader binary is ready at this path; interactive download can start.
    ReadyToDownload { binary: std::path::PathBuf },
    /// DepotDownloader is waiting for user input (e.g. password, Steam Guard).
    Prompt(String),
    Done,
    Error(String),
}
```

- [ ] **Step 2: Add prompt detection helper**

Add this function above `run_depot_downloader`:

```rust
/// Check whether a partial output line looks like a credential prompt.
fn looks_like_prompt(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("password") || lower.contains("guard") || lower.contains("2fa") || lower.contains("code")
}
```

- [ ] **Step 3: Add byte-level reader that detects partial lines**

Add this function:

```rust
/// Read from an async reader byte-by-byte, flushing partial lines after a timeout.
/// This detects prompts like "Password: " that don't end with a newline.
async fn stream_output(
    reader: impl tokio::io::AsyncRead + Unpin,
    tx: mpsc::Sender<DepotProgress>,
) {
    use tokio::io::AsyncReadExt;

    let mut buf = [0u8; 1024];
    let mut line_buf = Vec::new();
    let mut reader = reader;

    loop {
        let read_result = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            reader.read(&mut buf),
        )
        .await;

        match read_result {
            Ok(Ok(0)) => break, // EOF
            Ok(Ok(n)) => {
                for &byte in &buf[..n] {
                    if byte == b'\n' || byte == b'\r' {
                        if !line_buf.is_empty() {
                            let line = String::from_utf8_lossy(&line_buf).to_string();
                            let msg = if looks_like_prompt(&line) {
                                DepotProgress::Prompt(line)
                            } else {
                                DepotProgress::Line(line)
                            };
                            let _ = tx.send(msg).await;
                            line_buf.clear();
                        }
                    } else {
                        line_buf.push(byte);
                    }
                }
            }
            Ok(Err(_)) => break, // read error
            Err(_) => {
                // Timeout — flush partial line (likely a prompt waiting for input).
                if !line_buf.is_empty() {
                    let line = String::from_utf8_lossy(&line_buf).to_string();
                    let msg = if looks_like_prompt(&line) {
                        DepotProgress::Prompt(line)
                    } else {
                        DepotProgress::Line(line)
                    };
                    let _ = tx.send(msg).await;
                    line_buf.clear();
                }
            }
        }
    }
    // Flush any remaining bytes.
    if !line_buf.is_empty() {
        let line = String::from_utf8_lossy(&line_buf).to_string();
        let _ = tx.send(DepotProgress::Line(line)).await;
    }
}
```

- [ ] **Step 4: Rewrite `run_depot_downloader` to pipe stdin and use byte-level reading**

Replace the existing `run_depot_downloader` function:

```rust
/// Run DepotDownloader with piped stdin/stdout/stderr.
/// Returns a ChildStdin handle so the caller can forward credential input.
/// Output is streamed via the mpsc sender, with prompt detection.
pub async fn run_depot_downloader(
    binary: &Path,
    app_id: u32,
    depot_id: u32,
    manifest_id: &str,
    username: &str,
    cache_dir: &Path,
    tx: mpsc::Sender<DepotProgress>,
) -> Result<tokio::process::ChildStdin, DepotError> {
    std::fs::create_dir_all(cache_dir)?;

    let args = build_args(
        app_id,
        depot_id,
        manifest_id,
        username,
        cache_dir.to_string_lossy().as_ref(),
    );

    let mut child = Command::new(binary)
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let tx_out = tx.clone();
    let tx_err = tx.clone();

    tokio::spawn(async move {
        stream_output(stdout, tx_out).await;
    });
    tokio::spawn(async move {
        stream_output(stderr, tx_err).await;
    });

    let tx_done = tx.clone();
    tokio::spawn(async move {
        let status = child.wait().await;
        match status {
            Ok(s) if s.success() => {
                let _ = tx_done.send(DepotProgress::Done).await;
            }
            Ok(s) => {
                let code = s.code().unwrap_or(-1);
                let _ = tx_done
                    .send(DepotProgress::Error(format!("exit code {}", code)))
                    .await;
            }
            Err(e) => {
                let _ = tx_done
                    .send(DepotProgress::Error(format!("process error: {}", e)))
                    .await;
            }
        }
    });

    Ok(stdin)
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check 2>&1`
Expected: compiles (there will be warnings about unused `run_depot_downloader_interactive` and the old import — that's fine, we keep it for fallback)

- [ ] **Step 6: Commit**

```bash
git add rewind-core/src/depot.rs
git commit -m "feat: add Prompt variant and byte-level streaming to depot downloader"
```

---

### Task 3: Rewrite `start_download` and main loop to use in-TUI streaming

**Files:**
- Modify: `rewind-cli/src/main.rs`
- Modify: `rewind-cli/src/app.rs` (import cleanup)

- [ ] **Step 1: Update `start_download` to initialize steps and run DepotDownloader in-TUI**

Replace the `start_download` function in `main.rs`:

```rust
fn start_download(app: &mut App) {
    use crate::app::{StepKind, StepStatus};

    if app.config.steam_username.is_none() {
        app.wizard_state.error = Some("Steam username not set. Go to [S]ettings.".into());
        return;
    };

    let Ok(bin_dir) = config::bin_dir() else { return };

    let Some(game) = app.selected_game().cloned() else { return };
    let Some(username) = app.config.steam_username.clone() else { return };
    let Ok(cache_root) = config::cache_dir() else { return };

    let manifest_id = app.wizard_state.manifest_input.trim().to_string();
    let cache_dir = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        game.app_id,
        game.depot_id,
        &manifest_id,
    );

    let (tx, rx) = mpsc::channel(64);
    app.progress_rx = Some(rx);
    app.wizard_state.is_downloading = true;
    app.wizard_state.progress_lines.clear();
    app.wizard_state.depot_lines.clear();
    app.wizard_state.error = None;
    app.wizard_state.error_url = None;
    app.wizard_state.prompt_input = None;
    app.wizard_state.prompt_label = None;
    app.wizard_state.steps = vec![
        (StepKind::CheckDotnet, StepStatus::InProgress),
        (StepKind::DownloadDepot, StepStatus::Pending),
        (StepKind::DownloadManifest, StepStatus::Pending),
        (StepKind::BackupFiles, StepStatus::Pending),
        (StepKind::LinkFiles, StepStatus::Pending),
        (StepKind::PatchManifest, StepStatus::Pending),
        (StepKind::LockManifest, StepStatus::Pending),
    ];

    // Store download info for finalize phase (replaces PendingDownload).
    app.pending_download = Some(PendingDownload {
        binary: PathBuf::new(), // will be set later, not used for suspend anymore
        app_id: game.app_id,
        depot_id: game.depot_id,
        manifest_id,
        username: username.clone(),
        cache_dir: cache_dir.clone(),
        game_name: game.name.clone(),
        game_install_path: game.install_path.clone(),
        current_manifest_id: game.manifest_id.clone(),
        acf_path: game.acf_path.clone(),
    });

    let tx2 = tx.clone();
    tokio::spawn(async move {
        // Step 1: Check .NET runtime
        if !rewind_core::depot::check_dotnet().await {
            let _ = tx2
                .send(rewind_core::depot::DepotProgress::Error(
                    ".NET runtime not found. Press [O] to open the download page.".into(),
                ))
                .await;
            return;
        }
        let _ = tx2
            .send(rewind_core::depot::DepotProgress::Line(
                "__STEP_DONE:CheckDotnet".into(),
            ))
            .await;

        // Step 2: Ensure DepotDownloader binary
        let _ = tx2
            .send(rewind_core::depot::DepotProgress::Line(
                "__STEP_START:DownloadDepot".into(),
            ))
            .await;
        let binary = match rewind_core::depot::ensure_depot_downloader(&bin_dir).await {
            Ok(b) => b,
            Err(e) => {
                let _ = tx2
                    .send(rewind_core::depot::DepotProgress::Error(e.to_string()))
                    .await;
                return;
            }
        };
        let _ = tx2
            .send(rewind_core::depot::DepotProgress::Line(
                "__STEP_DONE:DownloadDepot".into(),
            ))
            .await;

        // Step 3: Run DepotDownloader (output streams via tx2 as Line/Prompt/Done/Error)
        let _ = tx2
            .send(rewind_core::depot::DepotProgress::Line(
                "__STEP_START:DownloadManifest".into(),
            ))
            .await;
        let _ = tx2
            .send(rewind_core::depot::DepotProgress::ReadyToDownload { binary })
            .await;
    });
}
```

- [ ] **Step 2: Rewrite the main loop progress handling**

In the main `loop` in `run()`, remove the TUI-suspend block (lines 53–79 of current `main.rs`). Replace the entire progress message handling section (lines 50–140) with:

```rust
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        // Poll progress channel.
        let progress_msgs: Vec<rewind_core::depot::DepotProgress> = {
            if let Some(rx) = &mut app.progress_rx {
                let mut msgs = Vec::new();
                while let Ok(msg) = rx.try_recv() {
                    msgs.push(msg);
                }
                msgs
            } else {
                Vec::new()
            }
        };
        for msg in progress_msgs {
            use crate::app::{StepKind, StepStatus};
            match msg {
                rewind_core::depot::DepotProgress::Line(line) => {
                    // Handle internal step signals.
                    if let Some(step_name) = line.strip_prefix("__STEP_DONE:") {
                        if let Some(kind) = step_kind_from_str(step_name) {
                            app.set_step_status(&kind, StepStatus::Done);
                        }
                    } else if let Some(step_name) = line.strip_prefix("__STEP_START:") {
                        if let Some(kind) = step_kind_from_str(step_name) {
                            app.set_step_status(&kind, StepStatus::InProgress);
                        }
                    } else {
                        // Regular DepotDownloader output line.
                        app.wizard_state.depot_lines.push(line);
                    }
                }
                rewind_core::depot::DepotProgress::ReadyToDownload { binary } => {
                    // Start DepotDownloader in piped mode.
                    if let Some(ref dl) = app.pending_download {
                        let tx_depot = mpsc::channel(64);
                        let (tx_d, rx_d) = tx_depot;

                        // Swap the receiver to get depot output.
                        app.progress_rx = Some(rx_d);

                        match rewind_core::depot::run_depot_downloader(
                            &binary,
                            dl.app_id,
                            dl.depot_id,
                            &dl.manifest_id,
                            &dl.username,
                            &dl.cache_dir,
                            tx_d,
                        )
                        .await
                        {
                            Ok(stdin) => {
                                app.depot_stdin = Some(stdin);
                            }
                            Err(e) => {
                                app.set_step_status(
                                    &StepKind::DownloadManifest,
                                    StepStatus::Failed(e.to_string()),
                                );
                                app.wizard_state.error =
                                    Some(format!("Failed to start download: {}", e));
                                app.wizard_state.is_downloading = false;
                            }
                        }
                    }
                }
                rewind_core::depot::DepotProgress::Prompt(label) => {
                    app.wizard_state.prompt_label = Some(label);
                    app.wizard_state.prompt_input = Some(String::new());
                }
                rewind_core::depot::DepotProgress::Done => {
                    app.set_step_status(&StepKind::DownloadManifest, StepStatus::Done);
                    app.depot_stdin = None;
                    // Run finalize steps with progress feedback.
                    if let Some(dl) = app.pending_download.take() {
                        finalize_downgrade_with_steps(&mut app, dl);
                    }
                }
                rewind_core::depot::DepotProgress::Error(e) => {
                    app.wizard_state.is_downloading = false;
                    app.depot_stdin = None;
                    if e.contains(".NET runtime not found") {
                        app.wizard_state.error_url =
                            Some("https://dotnet.microsoft.com/download".into());
                    }
                    // Mark the current in-progress step as failed.
                    if let Some(step) = app
                        .wizard_state
                        .steps
                        .iter_mut()
                        .find(|s| s.1 == StepStatus::InProgress)
                    {
                        step.1 = StepStatus::Failed(e.clone());
                    }
                    app.wizard_state.error = Some(e);
                }
            }
        }

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            handle_key(&mut app, key.code, key.modifiers).await;
        }

        if app.should_quit {
            break;
        }
    }
```

- [ ] **Step 3: Add `step_kind_from_str` helper**

Add this function in `main.rs`:

```rust
fn step_kind_from_str(s: &str) -> Option<app::StepKind> {
    match s {
        "CheckDotnet" => Some(app::StepKind::CheckDotnet),
        "DownloadDepot" => Some(app::StepKind::DownloadDepot),
        "DownloadManifest" => Some(app::StepKind::DownloadManifest),
        "BackupFiles" => Some(app::StepKind::BackupFiles),
        "LinkFiles" => Some(app::StepKind::LinkFiles),
        "PatchManifest" => Some(app::StepKind::PatchManifest),
        "LockManifest" => Some(app::StepKind::LockManifest),
        _ => None,
    }
}
```

- [ ] **Step 4: Add `finalize_downgrade_with_steps`**

Add this function, replacing the old `finalize_downgrade_from`. Keep `finalize_downgrade_from` for now (will be removed in a later cleanup) but rename it to avoid confusion:

```rust
fn finalize_downgrade_with_steps(app: &mut App, dl: PendingDownload) {
    use crate::app::{StepKind, StepStatus};

    let Ok(cache_root) = config::cache_dir() else { return };

    let target_cache = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        dl.app_id,
        dl.depot_id,
        &dl.manifest_id,
    );
    let current_cache = rewind_core::cache::manifest_cache_dir(
        &cache_root,
        dl.app_id,
        dl.depot_id,
        &dl.current_manifest_id,
    );

    // Step 4: Backup + Step 5: Link (both happen inside apply_downloaded)
    app.set_step_status(&StepKind::BackupFiles, StepStatus::InProgress);
    if let Err(e) =
        rewind_core::cache::apply_downloaded(&dl.game_install_path, &target_cache, &current_cache)
    {
        app.set_step_status(&StepKind::BackupFiles, StepStatus::Failed(e.to_string()));
        app.wizard_state.error = Some(format!("Failed to apply files: {}", e));
        app.wizard_state.is_downloading = false;
        return;
    }
    app.set_step_status(&StepKind::BackupFiles, StepStatus::Done);
    app.set_step_status(&StepKind::LinkFiles, StepStatus::Done);

    // Update game config.
    let existing = app
        .games_config
        .games
        .iter_mut()
        .find(|e| e.app_id == dl.app_id);

    let latest_buildid = rewind_core::scanner::read_acf_buildid(&dl.acf_path)
        .unwrap_or_else(|_| "0".to_string());

    if let Some(entry) = existing {
        entry.active_manifest_id = dl.manifest_id.clone();
        if !entry.cached_manifest_ids.contains(&dl.manifest_id) {
            entry.cached_manifest_ids.push(dl.manifest_id.clone());
        }
        if !entry.cached_manifest_ids.contains(&dl.current_manifest_id) {
            entry.cached_manifest_ids.push(dl.current_manifest_id.clone());
        }
        entry.latest_buildid = latest_buildid.clone();
        entry.acf_locked = true;
    } else {
        app.games_config.games.push(rewind_core::config::GameEntry {
            name: dl.game_name.clone(),
            app_id: dl.app_id,
            depot_id: dl.depot_id,
            install_path: dl.game_install_path.clone(),
            active_manifest_id: dl.manifest_id.clone(),
            latest_manifest_id: dl.current_manifest_id.clone(),
            latest_buildid: latest_buildid.clone(),
            cached_manifest_ids: vec![dl.current_manifest_id.clone(), dl.manifest_id.clone()],
            acf_locked: true,
        });
    }

    // Step 6: Patch ACF
    app.set_step_status(&StepKind::PatchManifest, StepStatus::InProgress);
    if let Err(e) = rewind_core::patcher::patch_acf_file(
        &dl.acf_path,
        &latest_buildid,
        &dl.current_manifest_id,
        dl.depot_id,
    ) {
        app.set_step_status(&StepKind::PatchManifest, StepStatus::Failed(e.to_string()));
        app.wizard_state.error = Some(format!("Failed to patch ACF: {}", e));
        app.wizard_state.is_downloading = false;
        return;
    }
    app.set_step_status(&StepKind::PatchManifest, StepStatus::Done);

    // Step 7: Lock ACF
    app.set_step_status(&StepKind::LockManifest, StepStatus::InProgress);
    if let Err(e) = rewind_core::immutability::lock_file(&dl.acf_path) {
        app.set_step_status(&StepKind::LockManifest, StepStatus::Failed(e.to_string()));
        app.wizard_state.error = Some(format!("Failed to lock ACF: {}", e));
        app.wizard_state.is_downloading = false;
        return;
    }
    app.set_step_status(&StepKind::LockManifest, StepStatus::Done);

    let _ = config::save_games(&app.games_config);
    app.wizard_state.is_downloading = false;
    app.screen = Screen::Main;
}
```

- [ ] **Step 5: Update imports in `main.rs`**

Update the import at the top to include the new types:

```rust
use app::{App, DowngradeWizardState, PendingDownload, Screen};
```

No new imports needed — `StepKind` and `StepStatus` are referenced via `crate::app::` in the code above.

Also add `use std::path::PathBuf;` if not already present.

- [ ] **Step 6: Remove the old `finalize_downgrade_from` function**

Delete the `finalize_downgrade_from` function (lines 541–615 of current `main.rs`) since it's replaced by `finalize_downgrade_with_steps`.

- [ ] **Step 7: Verify it compiles**

Run: `cargo check 2>&1`
Expected: compiles (may have warnings about unused `run_depot_downloader_interactive`)

- [ ] **Step 8: Commit**

```bash
git add rewind-cli/src/main.rs rewind-cli/src/app.rs
git commit -m "feat: replace TUI-suspend download with in-TUI streaming and step tracking"
```

---

### Task 4: Handle credential prompt input in the wizard key handler

**Files:**
- Modify: `rewind-cli/src/main.rs`

- [ ] **Step 1: Update `handle_wizard` to handle prompt input mode**

Replace the `handle_wizard` function:

```rust
fn handle_wizard(app: &mut App, key: KeyCode) {
    // If a credential prompt is active, handle input for that.
    if app.wizard_state.prompt_input.is_some() {
        match key {
            KeyCode::Char(c) => {
                if let Some(ref mut input) = app.wizard_state.prompt_input {
                    input.push(c);
                }
            }
            KeyCode::Backspace => {
                if let Some(ref mut input) = app.wizard_state.prompt_input {
                    input.pop();
                }
            }
            KeyCode::Enter => {
                if let Some(input) = app.wizard_state.prompt_input.take() {
                    // Write input to DepotDownloader's stdin.
                    if let Some(ref mut stdin) = app.depot_stdin {
                        use tokio::io::AsyncWriteExt;
                        let mut stdin_taken = app.depot_stdin.take().unwrap();
                        let response = format!("{}\n", input);
                        tokio::spawn(async move {
                            let _ = stdin_taken.write_all(response.as_bytes()).await;
                            let _ = stdin_taken.flush().await;
                            stdin_taken
                        });
                        // Note: stdin handle is consumed. If another prompt comes,
                        // we won't be able to respond. For the fallback, the timeout
                        // will trigger. We need to keep the handle — see step 2.
                    }
                    app.wizard_state.prompt_label = None;
                }
            }
            KeyCode::Esc => {
                // Cancel the prompt (and the whole download).
                app.wizard_state.prompt_input = None;
                app.wizard_state.prompt_label = None;
                app.wizard_state.is_downloading = false;
                app.depot_stdin = None;
                app.screen = Screen::Main;
                app.wizard_state = DowngradeWizardState::default();
            }
            _ => {}
        }
        return;
    }

    // Normal wizard key handling.
    match key {
        KeyCode::Esc => {
            app.depot_stdin = None;
            app.screen = Screen::Main;
            app.wizard_state = DowngradeWizardState::default();
        }
        KeyCode::Char('o') => {
            let url = if let Some(ref err_url) = app.wizard_state.error_url {
                err_url.clone()
            } else {
                app.wizard_state.steamdb_url.clone()
            };
            let _ = open::that(url);
        }
        KeyCode::Backspace => {
            if !app.wizard_state.is_downloading {
                app.wizard_state.manifest_input.pop();
            }
        }
        KeyCode::Char(c) => {
            if !app.wizard_state.is_downloading {
                app.wizard_state.manifest_input.push(c);
            }
        }
        KeyCode::Enter if !app.wizard_state.is_downloading => {
            if !app.wizard_state.manifest_input.trim().is_empty() {
                start_download(app);
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Fix stdin handling to preserve the handle across prompts**

The stdin handle needs to survive after writing. Update the Enter handling to use a channel-based approach instead. Replace the `KeyCode::Enter` arm in the prompt section:

```rust
            KeyCode::Enter => {
                if let Some(input) = app.wizard_state.prompt_input.take() {
                    if let Some(mut stdin) = app.depot_stdin.take() {
                        use tokio::io::AsyncWriteExt;
                        let response = format!("{}\n", input);
                        // Spawn a task to write, then return the stdin handle.
                        let (tx, mut rx) = mpsc::channel::<tokio::process::ChildStdin>(1);
                        tokio::spawn(async move {
                            let _ = stdin.write_all(response.as_bytes()).await;
                            let _ = stdin.flush().await;
                            let _ = tx.send(stdin).await;
                        });
                        // Try to get it back immediately on next poll.
                        // Store a pending stdin return.
                        app.pending_stdin_return = Some(rx);
                    }
                    app.wizard_state.prompt_label = None;
                }
            }
```

Add `pending_stdin_return` field to `App`:

In `app.rs`, add to the `App` struct:

```rust
    /// Receiver to get the stdin handle back after writing credentials.
    pub pending_stdin_return: Option<mpsc::Receiver<tokio::process::ChildStdin>>,
```

Initialize as `None` in `App::new()`.

Add polling in the main loop (after progress polling, before event polling):

```rust
        // Recover stdin handle after credential write.
        if let Some(ref mut rx) = app.pending_stdin_return {
            if let Ok(stdin) = rx.try_recv() {
                app.depot_stdin = Some(stdin);
                app.pending_stdin_return = None;
            }
        }
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check 2>&1`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add rewind-cli/src/main.rs rewind-cli/src/app.rs
git commit -m "feat: handle credential prompts via TUI input forwarded to piped stdin"
```

---

### Task 5: Rewrite the downgrade wizard UI with dual-log layout

**Files:**
- Modify: `rewind-cli/src/ui/downgrade_wizard.rs`

- [ ] **Step 1: Rewrite the entire `draw` function**

Replace the contents of `downgrade_wizard.rs`:

```rust
use crate::app::{App, StepStatus};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App) {
    let area = crate::ui::centered_rect(70, 75, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Downgrade Game ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let content = inner.inner(Margin {
        horizontal: 1,
        vertical: 0,
    });

    if app.wizard_state.is_downloading || !app.wizard_state.steps.is_empty() {
        draw_download_view(f, app, content);
    } else {
        draw_input_view(f, app, content);
    }
}

/// The initial view: SteamDB URL, manifest input, output log, help line.
fn draw_input_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    // SteamDB URL
    let url_block = Block::default()
        .title(" 1. Open this URL in your browser to find the manifest ID ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let url_para = Paragraph::new(app.wizard_state.steamdb_url.as_str())
        .style(Style::default().fg(Color::Cyan))
        .block(url_block);
    f.render_widget(url_para, layout[0]);

    // Manifest ID input
    let cursor = if !app.wizard_state.is_downloading {
        "\u{2588}"
    } else {
        ""
    };
    let input_style = if app.wizard_state.is_downloading {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    };
    let input_block = Block::default()
        .title(" 2. Enter target manifest ID then press [Enter] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let input_para =
        Paragraph::new(format!("{}{}", app.wizard_state.manifest_input, cursor))
            .style(input_style)
            .block(input_block);
    f.render_widget(input_para, layout[1]);

    // Error / output log
    let (log_title, log_border_style) = if app.wizard_state.error.is_some() {
        (" Error ", Style::default().fg(Color::Red))
    } else {
        (" Output ", Style::default().fg(Color::DarkGray))
    };

    let log_items: Vec<ListItem> = if let Some(err) = &app.wizard_state.error {
        vec![ListItem::new(err.as_str()).style(Style::default().fg(Color::Red))]
    } else {
        app.wizard_state
            .progress_lines
            .iter()
            .map(|l| ListItem::new(l.as_str()))
            .collect()
    };

    let log_block = Block::default()
        .title(log_title)
        .borders(Borders::ALL)
        .border_style(log_border_style);
    let log_list = List::new(log_items).block(log_block);
    f.render_widget(log_list, layout[2]);

    // Help line
    let help_text = if app.wizard_state.error_url.is_some() {
        " [O] open download page   [Esc] cancel   [Ctrl+C] quit "
    } else {
        " [O] open SteamDB in browser   [Esc] cancel   [Ctrl+C] quit "
    };
    let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, layout[3]);
}

/// The download-in-progress view: steps on top, DepotDownloader output below.
fn draw_download_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let step_count = app.wizard_state.steps.len() as u16;

    // Determine if credential prompt is active.
    let prompt_height = if app.wizard_state.prompt_input.is_some() {
        3u16
    } else {
        0
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(step_count + 1), // steps + top margin
            Constraint::Min(5),                 // DepotDownloader pane
            Constraint::Length(prompt_height),   // credential input (0 if hidden)
            Constraint::Length(1),               // help line
        ])
        .split(area);

    // --- Steps ---
    let step_items: Vec<ListItem> = app
        .wizard_state
        .steps
        .iter()
        .map(|(kind, status)| {
            let (icon, style) = match status {
                StepStatus::Pending => (
                    "[ ]",
                    Style::default().fg(Color::DarkGray),
                ),
                StepStatus::InProgress => (
                    "[\u{2026}]",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                StepStatus::Done => (
                    "[\u{2713}]",
                    Style::default().fg(Color::Green),
                ),
                StepStatus::Failed(_) => (
                    "[\u{2717}]",
                    Style::default().fg(Color::Red),
                ),
            };
            ListItem::new(format!(" {} {}", icon, kind.label())).style(style)
        })
        .collect();
    let step_list = List::new(step_items);
    f.render_widget(step_list, layout[0]);

    // --- DepotDownloader output pane ---
    let depot_block = Block::default()
        .title(" DepotDownloader ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let depot_inner_height = depot_block.inner(layout[1]).height as usize;
    let depot_items: Vec<ListItem> = app
        .wizard_state
        .depot_lines
        .iter()
        .rev()
        .take(depot_inner_height)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|l| ListItem::new(l.as_str()).style(Style::default().fg(Color::DarkGray)))
        .collect();

    let depot_list = List::new(depot_items).block(depot_block);
    f.render_widget(depot_list, layout[1]);

    // --- Credential prompt input (if active) ---
    if let Some(ref input) = app.wizard_state.prompt_input {
        let label = app
            .wizard_state
            .prompt_label
            .as_deref()
            .unwrap_or("Input required:");
        let prompt_block = Block::default()
            .title(format!(" {} ", label))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let prompt_para = Paragraph::new(format!("{}\u{2588}", input))
            .style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .block(prompt_block);
        f.render_widget(prompt_para, layout[2]);
    }

    // --- Help line ---
    let help_text = if app.wizard_state.error.is_some() {
        " [Esc] cancel   [Ctrl+C] quit "
    } else {
        " [Esc] cancel   [Ctrl+C] quit "
    };
    let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, layout[3]);
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check 2>&1`
Expected: compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add rewind-cli/src/ui/downgrade_wizard.rs
git commit -m "feat: dual-log wizard layout with step indicators and DepotDownloader pane"
```

---

### Task 6: Add 30-second timeout fallback to terminal mode

**Files:**
- Modify: `rewind-cli/src/app.rs`
- Modify: `rewind-cli/src/main.rs`

- [ ] **Step 1: Add timeout tracking field to `App`**

In `app.rs`, add to the `App` struct:

```rust
    /// Tracks when the last DepotDownloader output was received (for timeout detection).
    pub last_depot_output: Option<std::time::Instant>,
```

Initialize as `None` in `App::new()`.

- [ ] **Step 2: Update main loop to track last output time**

In the main loop progress handling, when receiving a `DepotProgress::Line` that goes to `depot_lines`, update the timestamp:

```rust
                    } else {
                        // Regular DepotDownloader output line.
                        app.wizard_state.depot_lines.push(line);
                        app.last_depot_output = Some(std::time::Instant::now());
                    }
```

Also set it when receiving `Prompt`:

```rust
                rewind_core::depot::DepotProgress::Prompt(label) => {
                    app.wizard_state.prompt_label = Some(label);
                    app.wizard_state.prompt_input = Some(String::new());
                    app.last_depot_output = Some(std::time::Instant::now());
                }
```

- [ ] **Step 3: Add timeout check in the main loop**

After the progress polling section, before event polling, add:

```rust
        // Timeout detection: if DepotDownloader has been silent for 30s during download,
        // it may be stuck on an undetected prompt.
        if app.depot_stdin.is_some() && app.wizard_state.prompt_input.is_none() {
            if let Some(last) = app.last_depot_output {
                if last.elapsed() > Duration::from_secs(30)
                    && app.wizard_state.error.is_none()
                {
                    app.wizard_state.error = Some(
                        "DepotDownloader may be waiting for input. Press [R] to restart with terminal mode.".into()
                    );
                }
            }
        }
```

- [ ] **Step 4: Add `[R]` key handler for fallback restart**

In `handle_wizard`, in the normal (non-prompt) key handling, add a case for `KeyCode::Char('r')`:

```rust
        KeyCode::Char('r') if app.wizard_state.is_downloading => {
            // Fallback: restart download in terminal mode.
            app.depot_stdin = None;
            app.progress_rx = None;
            app.wizard_state.error = None;
            app.last_depot_output = None;

            if let Some(dl) = app.pending_download.take() {
                // Store info needed to restart.
                app.pending_download = Some(dl);
                app.wizard_state.is_downloading = false;
                app.wizard_state.steps.clear();
                app.wizard_state.depot_lines.clear();
                // Signal that we should do a terminal-mode restart.
                // We'll re-use start_download but with a flag.
                app.wizard_state.error = Some(
                    "Terminal mode not yet implemented. Please restart the download.".into(),
                );
            }
        }
```

Note: Full terminal-mode fallback (re-suspending TUI and running `run_depot_downloader_interactive`) can be added as a follow-up if needed. For now, the timeout detection and message are the important parts.

- [ ] **Step 5: Update help text to show [R] when timeout error is displayed**

In `draw_download_view`, update the help line:

```rust
    let help_text = if app
        .wizard_state
        .error
        .as_ref()
        .map(|e| e.contains("[R]"))
        .unwrap_or(false)
    {
        " [R] restart in terminal   [Esc] cancel   [Ctrl+C] quit "
    } else {
        " [Esc] cancel   [Ctrl+C] quit "
    };
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check 2>&1`
Expected: compiles with no errors

- [ ] **Step 7: Commit**

```bash
git add rewind-cli/src/app.rs rewind-cli/src/main.rs rewind-cli/src/ui/downgrade_wizard.rs
git commit -m "feat: add 30-second timeout detection with terminal-mode fallback offer"
```

---

### Task 7: Clean up dead code and verify end-to-end

**Files:**
- Modify: `rewind-cli/src/main.rs`
- Modify: `rewind-cli/src/app.rs`

- [ ] **Step 1: Remove `PendingDownload` doc comment about TUI suspension**

In `app.rs`, update the comment on `PendingDownload`:

```rust
/// Download parameters for the active DepotDownloader session.
pub struct PendingDownload {
```

And update the comment on `pending_download` in `App`:

```rust
    /// Active download parameters (set when download starts, consumed on completion).
    pub pending_download: Option<PendingDownload>,
```

- [ ] **Step 2: Remove unused import of `DowngradeWizardState` from `main.rs` if it's no longer used directly**

Check the import line at the top of `main.rs`:

```rust
use app::{App, DowngradeWizardState, PendingDownload, Screen};
```

`DowngradeWizardState` is used in `handle_wizard` when resetting (`app.wizard_state = DowngradeWizardState::default()`), so keep it.

- [ ] **Step 3: Run full build**

Run: `cargo build 2>&1`
Expected: builds successfully

- [ ] **Step 4: Run tests**

Run: `cargo test 2>&1`
Expected: all existing tests pass

- [ ] **Step 5: Commit**

```bash
git add rewind-cli/src/main.rs rewind-cli/src/app.rs
git commit -m "chore: clean up comments and verify build after TUI download refactor"
```
