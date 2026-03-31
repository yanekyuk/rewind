// rewind-core/src/depot.rs
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum DepotError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("depotdownloader not found and could not be downloaded")]
    NotFound,
    #[error(".NET runtime not found — please install from https://dotnet.microsoft.com/download")]
    DotnetMissing,
    #[error("depotdownloader exited with code {0}")]
    ExitFailure(i32),
    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),
}

/// A progress message sent over the mpsc channel during download.
#[derive(Debug, Clone)]
pub enum DepotProgress {
    /// A status/info line to display while preparing the download.
    Line(String),
    /// DepotDownloader binary is ready at this path; interactive download can start.
    ReadyToDownload { binary: std::path::PathBuf },
    Done,
    Error(String),
}

/// Returns the platform-specific DepotDownloader zip asset name.
pub fn platform_asset_name() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "DepotDownloader-windows-x64.zip";
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "DepotDownloader-linux-x64.zip";
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "DepotDownloader-linux-arm64.zip";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "DepotDownloader-macos-x64.zip";
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "DepotDownloader-macos-arm64.zip";
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
    )))]
    compile_error!("Unsupported platform: no DepotDownloader asset available for this OS/arch combination.");
}

/// Build the argument list for a DepotDownloader invocation.
pub fn build_args(
    app_id: u32,
    depot_id: u32,
    manifest_id: &str,
    username: &str,
    output_dir: &str,
) -> Vec<String> {
    vec![
        "-app".into(),
        app_id.to_string(),
        "-depot".into(),
        depot_id.to_string(),
        "-manifest".into(),
        manifest_id.to_string(),
        "-username".into(),
        username.to_string(),
        "-remember-password".into(),
        "-dir".into(),
        output_dir.to_string(),
    ]
}

/// Returns the path to the DepotDownloader binary in the rewind bin dir.
pub fn depot_downloader_path(bin_dir: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    { bin_dir.join("DepotDownloader.exe") }
    #[cfg(not(target_os = "windows"))]
    { bin_dir.join("DepotDownloader") }
}

/// Check whether dotnet is available on PATH.
pub async fn check_dotnet() -> bool {
    Command::new("dotnet")
        .arg("--version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Download DepotDownloader from the latest GitHub release into bin_dir.
pub async fn download_depot_downloader(bin_dir: &Path) -> Result<PathBuf, DepotError> {
    std::fs::create_dir_all(bin_dir)?;

    let client = reqwest::Client::builder()
        .user_agent("rewind-cli/0.1")
        .build()?;

    let release: serde_json::Value = client
        .get("https://api.github.com/repos/SteamRE/DepotDownloader/releases/latest")
        .send()
        .await?
        .json()
        .await?;

    let asset_name = platform_asset_name();
    let download_url = release["assets"]
        .as_array()
        .and_then(|assets| {
            assets.iter().find(|a| {
                a["name"].as_str().map(|n| n == asset_name).unwrap_or(false)
            })
        })
        .and_then(|a| a["browser_download_url"].as_str())
        .ok_or(DepotError::NotFound)?
        .to_string();

    let zip_bytes = client.get(&download_url).send().await?.bytes().await?;
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    let binary_name = if cfg!(target_os = "windows") {
        "DepotDownloader.exe"
    } else {
        "DepotDownloader"
    };

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name() == binary_name || file.name().ends_with(binary_name) {
            let dest = depot_downloader_path(bin_dir);
            let mut outfile = std::fs::File::create(&dest)?;
            std::io::copy(&mut file, &mut outfile)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
            }

            return Ok(dest);
        }
    }

    Err(DepotError::NotFound)
}

/// Ensure DepotDownloader is present; download it if not.
pub async fn ensure_depot_downloader(bin_dir: &Path) -> Result<PathBuf, DepotError> {
    let path = depot_downloader_path(bin_dir);
    if path.exists() {
        return Ok(path);
    }
    download_depot_downloader(bin_dir).await
}

/// Run DepotDownloader with inherited stdio (interactive — handles password/Steam Guard prompts).
pub async fn run_depot_downloader_interactive(
    binary: &Path,
    app_id: u32,
    depot_id: u32,
    manifest_id: &str,
    username: &str,
    cache_dir: &Path,
) -> Result<(), DepotError> {
    std::fs::create_dir_all(cache_dir)?;
    let args = build_args(
        app_id,
        depot_id,
        manifest_id,
        username,
        cache_dir.to_string_lossy().as_ref(),
    );
    let status = Command::new(binary).args(&args).status().await?;
    if status.success() {
        Ok(())
    } else {
        Err(DepotError::ExitFailure(status.code().unwrap_or(-1)))
    }
}

/// Run DepotDownloader and stream output lines via the given mpsc sender.
pub async fn run_depot_downloader(
    binary: &Path,
    app_id: u32,
    depot_id: u32,
    manifest_id: &str,
    username: &str,
    cache_dir: &Path,
    tx: mpsc::Sender<DepotProgress>,
) -> Result<(), DepotError> {
    use tokio::io::{AsyncBufReadExt, BufReader};

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
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let tx_out = tx.clone();
    let tx_err = tx.clone();

    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = tx_out.send(DepotProgress::Line(line)).await;
        }
    });

    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = tx_err.send(DepotProgress::Line(line)).await;
        }
    });

    let status = child.wait().await?;
    let _ = tokio::join!(stdout_task, stderr_task);

    if status.success() {
        let _ = tx.send(DepotProgress::Done).await;
        Ok(())
    } else {
        let code = status.code().unwrap_or(-1);
        let _ = tx.send(DepotProgress::Error(format!("exit code {}", code))).await;
        Err(DepotError::ExitFailure(code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_name_is_nonempty() {
        let name = platform_asset_name();
        assert!(!name.is_empty());
        assert!(name.ends_with(".zip"));
        assert!(name.starts_with("DepotDownloader-"));
    }

    #[test]
    fn build_args_includes_remember_password() {
        let args = build_args(3321460, 3321461, "abc123", "testuser", "/tmp/cache");
        assert!(args.contains(&"-remember-password".to_string()));
        assert!(args.contains(&"-app".to_string()));
        assert!(args.contains(&"3321460".to_string()));
        assert!(args.contains(&"-depot".to_string()));
        assert!(args.contains(&"3321461".to_string()));
        assert!(args.contains(&"-manifest".to_string()));
        assert!(args.contains(&"abc123".to_string()));
        assert!(args.contains(&"-username".to_string()));
        assert!(args.contains(&"testuser".to_string()));
        assert!(args.contains(&"-dir".to_string()));
        assert!(args.contains(&"/tmp/cache".to_string()));
    }

    #[test]
    fn depot_downloader_path_uses_bin_dir() {
        let path = depot_downloader_path(std::path::Path::new("/tmp/bin"));
        assert!(path.starts_with("/tmp/bin"));
        #[cfg(not(target_os = "windows"))]
        assert!(path.ends_with("DepotDownloader"));
        #[cfg(target_os = "windows")]
        assert!(path.ends_with("DepotDownloader.exe"));
    }
}
