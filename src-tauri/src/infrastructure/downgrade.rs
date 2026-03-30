//! Downgrade file operations and ACF management.
//!
//! Infrastructure layer functions for applying a downgrade:
//! - Copying downloaded files over the game directory
//! - Deleting removed files
//! - Patching the ACF manifest
//! - Locking the ACF file with platform-specific immutability

use std::path::Path;

use crate::domain::vdf::{self, AcfPatchParams, AppState};
use crate::error::RewindError;

/// Apply downloaded files to the game's install directory.
///
/// Copies all files from `download_dir` into `install_path`, preserving
/// relative directory structure. Creates subdirectories as needed.
pub async fn apply_files(install_path: &Path, download_dir: &Path) -> Result<(), RewindError> {
    copy_dir_recursive(download_dir, install_path).await
}

/// Recursively copy contents of `src` into `dst`.
///
/// Preserves directory structure. Overwrites existing files.
fn copy_dir_recursive<'a>(
    src: &'a Path,
    dst: &'a Path,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), RewindError>> + Send + 'a>> {
    Box::pin(async move {
        let mut entries = tokio::fs::read_dir(src).await.map_err(|e| {
            RewindError::Infrastructure(format!(
                "failed to read download dir {}: {}",
                src.display(),
                e
            ))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            RewindError::Infrastructure(format!(
                "failed to read entry in {}: {}",
                src.display(),
                e
            ))
        })? {
            let src_path = entry.path();
            let file_name = match src_path.file_name() {
                Some(n) => n,
                None => continue,
            };
            let dst_path = dst.join(file_name);

            let file_type = entry.file_type().await.map_err(|e| {
                RewindError::Infrastructure(format!(
                    "failed to get file type for {}: {}",
                    src_path.display(),
                    e
                ))
            })?;

            if file_type.is_dir() {
                tokio::fs::create_dir_all(&dst_path).await.map_err(|e| {
                    RewindError::Infrastructure(format!(
                        "failed to create directory {}: {}",
                        dst_path.display(),
                        e
                    ))
                })?;
                copy_dir_recursive(&src_path, &dst_path).await?;
            } else {
                if let Some(parent) = dst_path.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|e| {
                        RewindError::Infrastructure(format!(
                            "failed to create parent dir {}: {}",
                            parent.display(),
                            e
                        ))
                    })?;
                }
                tokio::fs::copy(&src_path, &dst_path).await.map_err(|e| {
                    RewindError::Infrastructure(format!(
                        "failed to copy {} to {}: {}",
                        src_path.display(),
                        dst_path.display(),
                        e
                    ))
                })?;
            }
        }

        Ok(())
    })
}

/// Delete files classified as "removed" in the manifest diff.
///
/// Each path in `removed_files` is relative to `install_path`.
pub async fn delete_removed_files(
    install_path: &Path,
    removed_files: &[String],
) -> Result<(), RewindError> {
    for relative_path in removed_files {
        let full_path = install_path.join(relative_path);
        if full_path.exists() {
            tokio::fs::remove_file(&full_path).await.map_err(|e| {
                RewindError::Infrastructure(format!(
                    "failed to delete {}: {}",
                    full_path.display(),
                    e
                ))
            })?;
            eprintln!("[downgrade] deleted removed file: {}", relative_path);
        }
    }
    Ok(())
}

/// Patch an ACF manifest file for a downgrade.
///
/// Reads the ACF file, modifies fields per the downgrade rules, and writes
/// the patched content back. See docs/domain/downgrade-process.md for the
/// field values.
pub async fn patch_acf(acf_path: &Path, params: &AcfPatchParams) -> Result<(), RewindError> {
    let content = tokio::fs::read_to_string(acf_path).await.map_err(|e| {
        RewindError::Infrastructure(format!(
            "failed to read ACF file {}: {}",
            acf_path.display(),
            e
        ))
    })?;

    let doc = vdf::parse(&content).map_err(|e| RewindError::Domain(e.to_string()))?;
    let mut app_state =
        AppState::from_vdf(&doc).map_err(|e| RewindError::Domain(e.to_string()))?;

    app_state.patch_for_downgrade(params);

    let patched_doc = app_state.to_vdf();
    let serialized = vdf::serialize(&patched_doc);

    tokio::fs::write(acf_path, &serialized).await.map_err(|e| {
        RewindError::Infrastructure(format!(
            "failed to write patched ACF file {}: {}",
            acf_path.display(),
            e
        ))
    })?;

    eprintln!("[downgrade] patched ACF: {}", acf_path.display());
    Ok(())
}

/// Lock an ACF manifest file using platform-specific immutability.
///
/// - Linux: `pkexec chattr +i <path>`
/// - macOS: `chflags uchg <path>` (via osascript for privilege escalation)
/// - Windows: Sets the read-only attribute
pub async fn lock_acf(acf_path: &Path) -> Result<(), RewindError> {
    let path_str = acf_path.to_string_lossy().to_string();

    #[cfg(target_os = "linux")]
    {
        lock_acf_linux(&path_str).await
    }

    #[cfg(target_os = "macos")]
    {
        lock_acf_macos(&path_str).await
    }

    #[cfg(target_os = "windows")]
    {
        lock_acf_windows(&path_str).await
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(RewindError::Infrastructure(format!(
            "manifest locking is not supported on this platform"
        )))
    }
}

#[cfg(target_os = "linux")]
async fn lock_acf_linux(path: &str) -> Result<(), RewindError> {
    let output: std::process::Output = tokio::process::Command::new("pkexec")
        .args(["chattr", "+i", path])
        .output()
        .await
        .map_err(|e| {
            RewindError::Infrastructure(format!("failed to run pkexec chattr: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RewindError::Infrastructure(format!(
            "pkexec chattr +i failed: {}",
            stderr.trim()
        )));
    }

    eprintln!("[downgrade] locked ACF with chattr +i: {}", path);
    Ok(())
}

#[cfg(target_os = "macos")]
async fn lock_acf_macos(path: &str) -> Result<(), RewindError> {
    // Use osascript to run chflags with admin privileges via macOS auth dialog
    let script = format!(
        "do shell script \"chflags uchg '{}'\" with administrator privileges",
        path.replace('\'', "'\\''")
    );

    let output = tokio::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .await
        .map_err(|e| {
            RewindError::Infrastructure(format!("failed to run osascript: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RewindError::Infrastructure(format!(
            "chflags uchg failed: {}",
            stderr.trim()
        )));
    }

    eprintln!("[downgrade] locked ACF with chflags uchg: {}", path);
    Ok(())
}

#[cfg(target_os = "windows")]
async fn lock_acf_windows(path: &str) -> Result<(), RewindError> {
    use std::os::windows::fs::OpenOptionsExt;

    // Set read-only attribute
    let metadata = tokio::fs::metadata(path).await.map_err(|e| {
        RewindError::Infrastructure(format!("failed to read metadata for {}: {}", path, e))
    })?;

    let mut perms = metadata.permissions();
    perms.set_readonly(true);

    tokio::fs::set_permissions(path, perms).await.map_err(|e| {
        RewindError::Infrastructure(format!(
            "failed to set read-only attribute on {}: {}",
            path, e
        ))
    })?;

    eprintln!("[downgrade] locked ACF with read-only attribute: {}", path);
    Ok(())
}

/// Check if Steam is currently running.
///
/// Returns true if a Steam process is detected. Used to prevent applying
/// a downgrade while Steam is running (Steam monitors game directories).
pub async fn is_steam_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        // Check for Steam.exe process
        let output = tokio::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq steam.exe", "/NH"])
            .output()
            .await;
        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.to_lowercase().contains("steam.exe")
            }
            Err(_) => false,
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Check for steam process on Unix-like systems
        let output = tokio::process::Command::new("pgrep")
            .args(["-x", "steam"])
            .output()
            .await;
        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn apply_files_copies_to_install_dir() {
        let download_dir = tempfile::tempdir().unwrap();
        let install_dir = tempfile::tempdir().unwrap();

        // Create a file in the download dir
        tokio::fs::write(download_dir.path().join("game.exe"), b"new game binary")
            .await
            .unwrap();

        apply_files(install_dir.path(), download_dir.path())
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(install_dir.path().join("game.exe"))
            .await
            .unwrap();
        assert_eq!(content, "new game binary");
    }

    #[tokio::test]
    async fn apply_files_preserves_subdirectories() {
        let download_dir = tempfile::tempdir().unwrap();
        let install_dir = tempfile::tempdir().unwrap();

        // Create nested structure
        tokio::fs::create_dir_all(download_dir.path().join("data/textures"))
            .await
            .unwrap();
        tokio::fs::write(
            download_dir.path().join("data/textures/sky.dds"),
            b"texture data",
        )
        .await
        .unwrap();

        apply_files(install_dir.path(), download_dir.path())
            .await
            .unwrap();

        let content =
            tokio::fs::read_to_string(install_dir.path().join("data/textures/sky.dds"))
                .await
                .unwrap();
        assert_eq!(content, "texture data");
    }

    #[tokio::test]
    async fn apply_files_overwrites_existing() {
        let download_dir = tempfile::tempdir().unwrap();
        let install_dir = tempfile::tempdir().unwrap();

        // Existing file in install dir
        tokio::fs::write(install_dir.path().join("game.exe"), b"old version")
            .await
            .unwrap();

        // New version in download dir
        tokio::fs::write(download_dir.path().join("game.exe"), b"new version")
            .await
            .unwrap();

        apply_files(install_dir.path(), download_dir.path())
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(install_dir.path().join("game.exe"))
            .await
            .unwrap();
        assert_eq!(content, "new version");
    }

    #[tokio::test]
    async fn delete_removed_files_removes_existing() {
        let install_dir = tempfile::tempdir().unwrap();

        tokio::fs::write(install_dir.path().join("old_file.txt"), b"old data")
            .await
            .unwrap();

        delete_removed_files(install_dir.path(), &["old_file.txt".to_string()])
            .await
            .unwrap();

        assert!(!install_dir.path().join("old_file.txt").exists());
    }

    #[tokio::test]
    async fn delete_removed_files_ignores_nonexistent() {
        let install_dir = tempfile::tempdir().unwrap();

        // Should not error on nonexistent files
        let result =
            delete_removed_files(install_dir.path(), &["nonexistent.txt".to_string()]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn patch_acf_modifies_fields_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let acf_path = dir.path().join("appmanifest_3321460.acf");

        let content = r#""AppState"
{
	"appid"		"3321460"
	"name"		"Crimson Desert"
	"buildid"		"22560074"
	"installdir"		"Crimson Desert"
	"StateFlags"		"4"
	"InstalledDepots"
	{
		"3321461"
		{
			"manifest"		"7446650175280810671"
			"size"		"133575233011"
		}
	}
}"#;
        tokio::fs::write(&acf_path, content).await.unwrap();

        let params = AcfPatchParams {
            latest_buildid: "99999999".to_string(),
            latest_manifest: "8888888888888888888".to_string(),
            latest_size: "200000000000".to_string(),
            depot_id: "3321461".to_string(),
        };

        patch_acf(&acf_path, &params).await.unwrap();

        // Re-read and verify
        let patched = tokio::fs::read_to_string(&acf_path).await.unwrap();
        let doc = vdf::parse(&patched).unwrap();
        let app = AppState::from_vdf(&doc).unwrap();

        assert_eq!(app.buildid, "99999999");
        assert_eq!(app.state_flags, "4");
        assert_eq!(app.target_build_id, Some("0".to_string()));
        assert_eq!(app.bytes_to_download, Some("0".to_string()));
        assert_eq!(
            app.full_validate_after_next_update,
            Some("0".to_string())
        );
        let depot = app.installed_depots.get("3321461").unwrap();
        assert_eq!(depot.manifest, "8888888888888888888");
        assert_eq!(depot.size, "200000000000");
    }

    #[tokio::test]
    async fn patch_acf_errors_on_nonexistent_file() {
        let params = AcfPatchParams {
            latest_buildid: "1".to_string(),
            latest_manifest: "2".to_string(),
            latest_size: "3".to_string(),
            depot_id: "4".to_string(),
        };
        let result = patch_acf(Path::new("/nonexistent/appmanifest.acf"), &params).await;
        assert!(result.is_err());
    }
}
