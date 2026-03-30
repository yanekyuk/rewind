//! Domain types for the downgrade pipeline.
//!
//! These types define the progress events emitted during the downgrade process.
//! They cross the IPC boundary to the frontend via Tauri event emission.

use serde::Serialize;

/// Progress event payload for the downgrade pipeline.
///
/// Emitted on the `downgrade-progress` Tauri event channel to inform the
/// frontend about the current phase of the downgrade process.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase")]
pub enum DowngradeProgress {
    /// Manifest fetch + diff in progress.
    #[serde(rename = "comparing")]
    Comparing,

    /// File download in progress.
    #[serde(rename = "downloading")]
    Downloading {
        /// Download progress as a percentage (0.0 - 100.0).
        percent: f64,
        /// Bytes downloaded so far.
        bytes_downloaded: u64,
        /// Total bytes to download.
        bytes_total: u64,
    },

    /// File copy, ACF patch, and lock in progress.
    #[serde(rename = "applying")]
    Applying,

    /// Pipeline finished successfully.
    #[serde(rename = "complete")]
    Complete,

    /// Pipeline failed.
    #[serde(rename = "error")]
    Error {
        /// Human-readable error message.
        message: String,
    },
}

/// Parameters for starting a downgrade operation.
///
/// Passed from the frontend to the `start_downgrade` Tauri IPC command.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DowngradeParams {
    /// Steam application ID.
    pub app_id: String,
    /// Steam depot ID to downgrade.
    pub depot_id: String,
    /// Target manifest ID (the version to downgrade to).
    pub target_manifest_id: String,
    /// Current manifest ID (from the installed game's ACF).
    pub current_manifest_id: String,
    /// The latest build ID (for ACF patching -- set to current since it's
    /// what Steam's servers report as latest).
    pub latest_buildid: String,
    /// The latest manifest ID for ACF patching.
    pub latest_manifest_id: String,
    /// The latest depot size for ACF patching.
    pub latest_size: String,
    /// Full path to the game's install directory (steamapps/common/<installdir>).
    pub install_path: String,
    /// Full path to the steamapps directory containing the ACF file.
    pub steamapps_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_comparing_serializes_correctly() {
        let event = DowngradeProgress::Comparing;
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"phase\":\"comparing\""));
    }

    #[test]
    fn progress_downloading_serializes_with_fields() {
        let event = DowngradeProgress::Downloading {
            percent: 45.5,
            bytes_downloaded: 4500000000,
            bytes_total: 9900000000,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"phase\":\"downloading\""));
        assert!(json.contains("\"percent\":45.5"));
        assert!(json.contains("\"bytes_downloaded\":4500000000"));
        assert!(json.contains("\"bytes_total\":9900000000"));
    }

    #[test]
    fn progress_applying_serializes_correctly() {
        let event = DowngradeProgress::Applying;
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"phase\":\"applying\""));
    }

    #[test]
    fn progress_complete_serializes_correctly() {
        let event = DowngradeProgress::Complete;
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"phase\":\"complete\""));
    }

    #[test]
    fn progress_error_serializes_with_message() {
        let event = DowngradeProgress::Error {
            message: "download failed".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"phase\":\"error\""));
        assert!(json.contains("\"message\":\"download failed\""));
    }

    #[test]
    fn downgrade_params_deserializes() {
        let json = r#"{
            "app_id": "3321460",
            "depot_id": "3321461",
            "target_manifest_id": "1234567890",
            "current_manifest_id": "9876543210",
            "latest_buildid": "22560074",
            "latest_manifest_id": "9876543210",
            "latest_size": "133575233011",
            "install_path": "/home/user/.local/share/Steam/steamapps/common/Crimson Desert",
            "steamapps_path": "/home/user/.local/share/Steam/steamapps"
        }"#;
        let params: DowngradeParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.app_id, "3321460");
        assert_eq!(params.depot_id, "3321461");
        assert_eq!(params.target_manifest_id, "1234567890");
    }
}
