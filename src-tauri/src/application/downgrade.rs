//! Downgrade pipeline orchestration.
//!
//! Coordinates the 4-phase downgrade workflow:
//! 1. Comparing — fetch manifests, diff, generate filelist
//! 2. Downloading — write filelist, spawn sidecar download
//! 3. Applying — copy files, delete removed, patch ACF, lock ACF
//! 4. Complete — emit success or error
//!
//! # Architecture Rule
//!
//! This module must NOT import from the infrastructure layer.
//! Infrastructure dependencies are injected via the `DowngradeServices` trait,
//! which is implemented at the composition root (`lib.rs`).

use std::path::{Path, PathBuf};

use crate::domain::auth::Credentials;
use crate::domain::downgrade::{DowngradeParams, DowngradeProgress};
use crate::domain::manifest::{diff_manifests, DepotManifest};
use crate::domain::vdf::AcfPatchParams;
use crate::error::RewindError;

/// Trait abstracting infrastructure operations for the downgrade pipeline.
///
/// Implemented at the composition root (`lib.rs`) to inject real infrastructure.
/// Tests provide mock implementations. Includes event emission so the full
/// orchestration flow is testable without a Tauri AppHandle.
#[allow(async_fn_in_trait)]
pub trait DowngradeServices: Send + Sync {
    /// Emit a progress event to the frontend.
    fn emit_progress(&self, progress: DowngradeProgress);

    /// Fetch manifest metadata for a specific depot manifest.
    async fn get_manifest(
        &self,
        app_id: &str,
        depot_id: &str,
        manifest_id: &str,
        credentials: &Credentials,
    ) -> Result<DepotManifest, RewindError>;

    /// Download depot files using a filelist.
    async fn download(
        &self,
        app_id: &str,
        depot_id: &str,
        manifest_id: &str,
        output_dir: &str,
        filelist_path: &str,
        credentials: &Credentials,
    ) -> Result<(), RewindError>;

    /// Apply downloaded files to the game's install directory.
    async fn apply_files(
        &self,
        install_path: &Path,
        download_dir: &Path,
    ) -> Result<(), RewindError>;

    /// Delete files classified as "removed" in the manifest diff.
    async fn delete_removed_files(
        &self,
        install_path: &Path,
        removed_files: &[String],
    ) -> Result<(), RewindError>;

    /// Patch an ACF manifest file for a downgrade.
    async fn patch_acf(
        &self,
        acf_path: &Path,
        params: &AcfPatchParams,
    ) -> Result<(), RewindError>;

    /// Lock an ACF manifest file using platform-specific immutability.
    async fn lock_acf(&self, acf_path: &Path) -> Result<(), RewindError>;

    /// Check if Steam is currently running.
    async fn is_steam_running(&self) -> bool;

    /// Write content to a file (for filelist).
    async fn write_file(&self, path: &Path, content: &str) -> Result<(), RewindError>;
}

/// Run the full downgrade pipeline.
///
/// Orchestrates all 4 phases, emitting progress events via the services
/// trait. Infrastructure operations are delegated to the provided `services`
/// implementation.
///
/// # Phases
///
/// 1. **Comparing** -- Fetch target + current manifests, diff them, generate filelist
/// 2. **Downloading** -- Write filelist to temp file, call sidecar download
/// 3. **Applying** -- Copy files, delete removed, patch ACF, lock ACF
/// 4. **Complete** -- Emit success or error event
pub async fn run_downgrade<S: DowngradeServices>(
    params: &DowngradeParams,
    credentials: &Credentials,
    services: &S,
) -> Result<(), RewindError> {
    // Phase 1: Comparing
    services.emit_progress(DowngradeProgress::Comparing);
    eprintln!("[downgrade] phase 1: comparing manifests");

    let target_manifest = services
        .get_manifest(
            &params.app_id,
            &params.depot_id,
            &params.target_manifest_id,
            credentials,
        )
        .await?;

    let current_manifest = services
        .get_manifest(
            &params.app_id,
            &params.depot_id,
            &params.current_manifest_id,
            credentials,
        )
        .await?;

    let diff = diff_manifests(&current_manifest, &target_manifest);
    let filelist = diff.filelist();

    if filelist.is_empty() {
        eprintln!("[downgrade] no files to download — manifests are identical");
        services.emit_progress(DowngradeProgress::Complete);
        return Ok(());
    }

    eprintln!(
        "[downgrade] diff: {} changed, {} added, {} removed",
        diff.changed.len(),
        diff.added.len(),
        diff.removed.len()
    );

    // Phase 2: Downloading
    let download_dir =
        std::env::temp_dir().join(format!("rewind_{}_{}", params.app_id, params.depot_id));
    let filelist_path = std::env::temp_dir().join(format!(
        "rewind_filelist_{}_{}.txt",
        params.app_id, params.depot_id
    ));

    let filelist_content = filelist.join("\n");
    services
        .write_file(&filelist_path, &filelist_content)
        .await?;

    eprintln!(
        "[downgrade] phase 2: downloading {} files to {}",
        filelist.len(),
        download_dir.display()
    );

    services
        .download(
            &params.app_id,
            &params.depot_id,
            &params.target_manifest_id,
            &download_dir.to_string_lossy(),
            &filelist_path.to_string_lossy(),
            credentials,
        )
        .await?;

    // Phase 3: Applying
    services.emit_progress(DowngradeProgress::Applying);
    eprintln!("[downgrade] phase 3: applying downgrade");

    // Check if Steam is running before applying
    if services.is_steam_running().await {
        return Err(RewindError::Application(
            "Steam is currently running. Please close Steam before applying the downgrade."
                .to_string(),
        ));
    }

    let install_path = PathBuf::from(&params.install_path);

    services
        .apply_files(&install_path, &download_dir)
        .await?;

    let removed_file_names: Vec<String> = diff.removed.iter().map(|e| e.name.clone()).collect();
    services
        .delete_removed_files(&install_path, &removed_file_names)
        .await?;

    // Patch ACF with LATEST values (not target) to trick Steam
    let acf_path = PathBuf::from(&params.steamapps_path)
        .join(format!("appmanifest_{}.acf", params.app_id));

    let acf_params = AcfPatchParams {
        latest_buildid: params.latest_buildid.clone(),
        latest_manifest: params.latest_manifest_id.clone(),
        latest_size: params.latest_size.clone(),
        depot_id: params.depot_id.clone(),
    };

    services.patch_acf(&acf_path, &acf_params).await?;
    services.lock_acf(&acf_path).await?;

    // Phase 4: Complete
    services.emit_progress(DowngradeProgress::Complete);
    eprintln!("[downgrade] phase 4: complete");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Records which service methods were called, in order.
    #[derive(Debug, Clone, Default)]
    struct CallLog {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl CallLog {
        fn push(&self, call: &str) {
            self.calls.lock().unwrap().push(call.to_string());
        }

        fn get(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    /// A mock services implementation for testing the orchestration logic.
    struct MockServices {
        log: CallLog,
        /// Manifests returned by get_manifest, keyed by manifest_id.
        manifests: std::collections::HashMap<String, DepotManifest>,
        /// Whether is_steam_running returns true.
        steam_running: bool,
        /// If set, get_manifest returns this error.
        manifest_error: Option<RewindError>,
        /// If set, download returns this error.
        download_error: Option<RewindError>,
        /// If set, apply_files returns this error.
        apply_error: Option<RewindError>,
    }

    impl MockServices {
        fn new(log: CallLog) -> Self {
            Self {
                log,
                manifests: std::collections::HashMap::new(),
                steam_running: false,
                manifest_error: None,
                download_error: None,
                apply_error: None,
            }
        }

        fn with_manifest(mut self, manifest_id: &str, manifest: DepotManifest) -> Self {
            self.manifests.insert(manifest_id.to_string(), manifest);
            self
        }

        fn with_steam_running(mut self, running: bool) -> Self {
            self.steam_running = running;
            self
        }

        fn with_manifest_error(mut self, err: RewindError) -> Self {
            self.manifest_error = Some(err);
            self
        }

        fn with_download_error(mut self, err: RewindError) -> Self {
            self.download_error = Some(err);
            self
        }

        fn with_apply_error(mut self, err: RewindError) -> Self {
            self.apply_error = Some(err);
            self
        }
    }

    impl DowngradeServices for MockServices {
        fn emit_progress(&self, progress: DowngradeProgress) {
            let phase = match &progress {
                DowngradeProgress::Comparing => "comparing".to_string(),
                DowngradeProgress::Downloading { .. } => "downloading".to_string(),
                DowngradeProgress::Applying => "applying".to_string(),
                DowngradeProgress::Complete => "complete".to_string(),
                DowngradeProgress::Error { message } => format!("error:{}", message),
            };
            self.log.push(&format!("emit:{}", phase));
        }

        async fn get_manifest(
            &self,
            _app_id: &str,
            _depot_id: &str,
            manifest_id: &str,
            _credentials: &Credentials,
        ) -> Result<DepotManifest, RewindError> {
            self.log.push(&format!("get_manifest:{}", manifest_id));
            if let Some(ref err) = self.manifest_error {
                return Err(RewindError::Infrastructure(format!("{}", err)));
            }
            self.manifests
                .get(manifest_id)
                .cloned()
                .ok_or_else(|| {
                    RewindError::Infrastructure(format!("no mock manifest for {}", manifest_id))
                })
        }

        async fn download(
            &self,
            _app_id: &str,
            _depot_id: &str,
            _manifest_id: &str,
            _output_dir: &str,
            _filelist_path: &str,
            _credentials: &Credentials,
        ) -> Result<(), RewindError> {
            self.log.push("download");
            if let Some(ref err) = self.download_error {
                return Err(RewindError::Infrastructure(format!("{}", err)));
            }
            Ok(())
        }

        async fn apply_files(
            &self,
            _install_path: &Path,
            _download_dir: &Path,
        ) -> Result<(), RewindError> {
            self.log.push("apply_files");
            if let Some(ref err) = self.apply_error {
                return Err(RewindError::Infrastructure(format!("{}", err)));
            }
            Ok(())
        }

        async fn delete_removed_files(
            &self,
            _install_path: &Path,
            removed_files: &[String],
        ) -> Result<(), RewindError> {
            self.log
                .push(&format!("delete_removed_files:{}", removed_files.len()));
            Ok(())
        }

        async fn patch_acf(
            &self,
            _acf_path: &Path,
            _params: &AcfPatchParams,
        ) -> Result<(), RewindError> {
            self.log.push("patch_acf");
            Ok(())
        }

        async fn lock_acf(&self, _acf_path: &Path) -> Result<(), RewindError> {
            self.log.push("lock_acf");
            Ok(())
        }

        async fn is_steam_running(&self) -> bool {
            self.log.push("is_steam_running");
            self.steam_running
        }

        async fn write_file(&self, _path: &Path, _content: &str) -> Result<(), RewindError> {
            self.log.push("write_file");
            Ok(())
        }
    }

    use crate::domain::manifest::ManifestEntry;

    fn make_manifest(id: u64, entries: Vec<ManifestEntry>) -> DepotManifest {
        DepotManifest {
            depot_id: 3321461,
            manifest_id: id,
            date: "2026-03-22 16:01:45".to_string(),
            total_files: entries.len() as u64,
            total_chunks: 0,
            total_bytes_on_disk: 0,
            total_bytes_compressed: 0,
            entries,
        }
    }

    fn make_entry(name: &str, sha: &str) -> ManifestEntry {
        ManifestEntry {
            size: 100,
            chunks: 1,
            sha: sha.to_string(),
            flags: 0,
            name: name.to_string(),
        }
    }

    fn make_params() -> DowngradeParams {
        DowngradeParams {
            app_id: "3321460".to_string(),
            depot_id: "3321461".to_string(),
            target_manifest_id: "1111111111".to_string(),
            current_manifest_id: "2222222222".to_string(),
            latest_buildid: "99999999".to_string(),
            latest_manifest_id: "2222222222".to_string(),
            latest_size: "133575233011".to_string(),
            install_path: "/tmp/test_install".to_string(),
            steamapps_path: "/tmp/test_steamapps".to_string(),
        }
    }

    fn make_credentials() -> Credentials {
        Credentials {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            guard_code: None,
        }
    }

    fn make_services_with_diff(log: CallLog) -> MockServices {
        let current = make_manifest(
            2222222222,
            vec![
                make_entry("game.exe", "aaaa"),
                make_entry("data.pak", "bbbb"),
                make_entry("old_file.txt", "cccc"),
            ],
        );
        let target = make_manifest(
            1111111111,
            vec![
                make_entry("game.exe", "xxxx"),     // changed
                make_entry("data.pak", "bbbb"),      // unchanged
                make_entry("new_file.txt", "yyyy"),  // added
            ],
        );

        MockServices::new(log)
            .with_manifest("1111111111", target)
            .with_manifest("2222222222", current)
    }

    // ---- Orchestration tests (async, exercise run_downgrade) ----

    #[tokio::test]
    async fn full_pipeline_executes_phases_in_order() {
        let log = CallLog::default();
        let services = make_services_with_diff(log.clone());
        let params = make_params();
        let creds = make_credentials();

        let result = run_downgrade(&params, &creds, &services).await;
        assert!(result.is_ok(), "pipeline failed: {:?}", result.err());

        let calls = log.get();
        assert_eq!(calls[0], "emit:comparing");
        assert_eq!(calls[1], "get_manifest:1111111111"); // target first
        assert_eq!(calls[2], "get_manifest:2222222222"); // then current
        assert_eq!(calls[3], "write_file");
        assert_eq!(calls[4], "download");
        assert_eq!(calls[5], "emit:applying");
        assert_eq!(calls[6], "is_steam_running");
        assert_eq!(calls[7], "apply_files");
        assert_eq!(calls[8], "delete_removed_files:1"); // 1 removed file
        assert_eq!(calls[9], "patch_acf");
        assert_eq!(calls[10], "lock_acf");
        assert_eq!(calls[11], "emit:complete");
    }

    #[tokio::test]
    async fn identical_manifests_skip_to_complete() {
        let log = CallLog::default();
        let entries = vec![
            make_entry("game.exe", "aaaa"),
            make_entry("data.pak", "bbbb"),
        ];
        let services = MockServices::new(log.clone())
            .with_manifest("1111111111", make_manifest(1111111111, entries.clone()))
            .with_manifest("2222222222", make_manifest(2222222222, entries));

        let params = make_params();
        let creds = make_credentials();

        let result = run_downgrade(&params, &creds, &services).await;
        assert!(result.is_ok());

        let calls = log.get();
        assert_eq!(calls[0], "emit:comparing");
        assert_eq!(calls[1], "get_manifest:1111111111");
        assert_eq!(calls[2], "get_manifest:2222222222");
        assert_eq!(calls[3], "emit:complete");
        // No download, apply, patch, or lock calls
        assert_eq!(calls.len(), 4);
    }

    #[tokio::test]
    async fn steam_running_prevents_apply() {
        let log = CallLog::default();
        let services = make_services_with_diff(log.clone()).with_steam_running(true);

        let params = make_params();
        let creds = make_credentials();

        let result = run_downgrade(&params, &creds, &services).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            RewindError::Application(msg) => {
                assert!(msg.contains("Steam is currently running"));
            }
            other => panic!("expected Application error, got: {:?}", other),
        }

        // Should have stopped before apply_files
        let calls = log.get();
        assert!(calls.contains(&"is_steam_running".to_string()));
        assert!(!calls.contains(&"apply_files".to_string()));
    }

    #[tokio::test]
    async fn manifest_fetch_error_stops_pipeline() {
        let log = CallLog::default();
        let services = MockServices::new(log.clone())
            .with_manifest_error(RewindError::Infrastructure("network error".to_string()));

        let params = make_params();
        let creds = make_credentials();

        let result = run_downgrade(&params, &creds, &services).await;
        assert!(result.is_err());

        let calls = log.get();
        assert_eq!(calls[0], "emit:comparing");
        assert_eq!(calls[1], "get_manifest:1111111111");
        // Should stop here — no download or apply
        assert_eq!(calls.len(), 2);
    }

    #[tokio::test]
    async fn download_error_stops_pipeline() {
        let log = CallLog::default();
        let services = make_services_with_diff(log.clone())
            .with_download_error(RewindError::Infrastructure("download failed".to_string()));

        let params = make_params();
        let creds = make_credentials();

        let result = run_downgrade(&params, &creds, &services).await;
        assert!(result.is_err());

        let calls = log.get();
        assert!(calls.contains(&"download".to_string()));
        // Should not proceed to applying phase
        assert!(!calls.contains(&"emit:applying".to_string()));
    }

    #[tokio::test]
    async fn apply_error_stops_pipeline() {
        let log = CallLog::default();
        let services = make_services_with_diff(log.clone())
            .with_apply_error(RewindError::Infrastructure("copy failed".to_string()));

        let params = make_params();
        let creds = make_credentials();

        let result = run_downgrade(&params, &creds, &services).await;
        assert!(result.is_err());

        let calls = log.get();
        assert!(calls.contains(&"apply_files".to_string()));
        // Should not proceed to patch/lock/complete
        assert!(!calls.contains(&"patch_acf".to_string()));
        assert!(!calls.contains(&"emit:complete".to_string()));
    }

    // ---- Unit tests for data construction ----

    #[test]
    fn acf_path_constructed_correctly() {
        let params = make_params();
        let acf_path = PathBuf::from(&params.steamapps_path)
            .join(format!("appmanifest_{}.acf", params.app_id));
        assert_eq!(
            acf_path.to_string_lossy(),
            "/tmp/test_steamapps/appmanifest_3321460.acf"
        );
    }

    #[test]
    fn acf_patch_params_use_latest_values() {
        let params = make_params();
        let acf_params = AcfPatchParams {
            latest_buildid: params.latest_buildid.clone(),
            latest_manifest: params.latest_manifest_id.clone(),
            latest_size: params.latest_size.clone(),
            depot_id: params.depot_id.clone(),
        };
        // Must use LATEST values, not target
        assert_eq!(acf_params.latest_buildid, "99999999");
        assert_eq!(acf_params.latest_manifest, "2222222222");
        assert_eq!(acf_params.latest_size, "133575233011");
        assert_eq!(acf_params.depot_id, "3321461");
    }

    #[test]
    fn identical_manifests_produce_empty_filelist() {
        let entries = vec![
            make_entry("game.exe", "aaaa"),
            make_entry("data.pak", "bbbb"),
        ];
        let current = make_manifest(2222222222, entries.clone());
        let target = make_manifest(1111111111, entries);

        let diff = diff_manifests(&current, &target);
        assert!(diff.filelist().is_empty());
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn removed_files_collected_from_diff() {
        let current = make_manifest(
            2222222222,
            vec![
                make_entry("game.exe", "aaaa"),
                make_entry("old_file.txt", "cccc"),
            ],
        );
        let target = make_manifest(1111111111, vec![make_entry("game.exe", "aaaa")]);

        let diff = diff_manifests(&current, &target);
        let removed: Vec<String> = diff.removed.iter().map(|e| e.name.clone()).collect();
        assert_eq!(removed, vec!["old_file.txt"]);
    }

    #[test]
    fn download_dir_uses_temp_dir() {
        let params = make_params();
        let download_dir =
            std::env::temp_dir().join(format!("rewind_{}_{}", params.app_id, params.depot_id));
        assert!(download_dir
            .to_string_lossy()
            .contains("rewind_3321460_3321461"));
    }

    #[test]
    fn filelist_path_uses_temp_dir() {
        let params = make_params();
        let filelist_path = std::env::temp_dir().join(format!(
            "rewind_filelist_{}_{}.txt",
            params.app_id, params.depot_id
        ));
        assert!(filelist_path
            .to_string_lossy()
            .contains("rewind_filelist_3321460_3321461.txt"));
    }
}
