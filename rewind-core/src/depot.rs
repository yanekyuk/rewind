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
    /// DepotDownloader binary is ready; interactive download can start.
    /// `filelist_path` is Some when only missing files need downloading (deduplication).
    /// If None, all files are already cached and the download should be skipped.
    ReadyToDownload { binary: std::path::PathBuf, filelist_path: Option<std::path::PathBuf> },
    /// DepotDownloader is waiting for user input (e.g. password, Steam Guard).
    Prompt(String),
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

/// Build args for a targeted download using a filelist.
/// Used when only a subset of manifest files need to be downloaded (deduplication).
pub fn build_filelist_args(
    app_id: u32,
    depot_id: u32,
    manifest_id: &str,
    username: &str,
    output_dir: &str,
    filelist_path: &str,
) -> Vec<String> {
    let mut args = build_args(app_id, depot_id, manifest_id, username, output_dir);
    args.push("-filelist".into());
    args.push(filelist_path.into());
    args
}

/// Returns the path to the DepotDownloader binary in the rewind bin dir.
pub fn depot_downloader_path(bin_dir: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    { bin_dir.join("DepotDownloader.exe") }
    #[cfg(not(target_os = "windows"))]
    { bin_dir.join("DepotDownloader") }
}

/// Check whether the .NET runtime is available.
///
/// First checks PATH, then falls back to well-known installation directories
/// so users don't need to configure their PATH after a default install.
pub async fn check_dotnet() -> bool {
    // 1. Try PATH first.
    if try_dotnet("dotnet").await {
        return true;
    }
    // 2. Check common installation paths.
    #[cfg(target_os = "macos")]
    const KNOWN_PATHS: &[&str] = &[
        "/usr/local/share/dotnet/dotnet",
        "/opt/homebrew/bin/dotnet",
    ];
    #[cfg(target_os = "linux")]
    const KNOWN_PATHS: &[&str] = &[
        "/usr/share/dotnet/dotnet",
        "/usr/lib/dotnet/dotnet",
        "/snap/dotnet-sdk/current/dotnet",
    ];
    #[cfg(target_os = "windows")]
    const KNOWN_PATHS: &[&str] = &[
        r"C:\Program Files\dotnet\dotnet.exe",
    ];
    for path in KNOWN_PATHS {
        if try_dotnet(path).await {
            return true;
        }
    }
    false
}

async fn try_dotnet(cmd: &str) -> bool {
    Command::new(cmd)
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

/// Check whether a partial output line looks like a credential prompt.
fn looks_like_prompt(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("password") || lower.contains("steam guard") || lower.contains("2fa")
}

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

/// Run DepotDownloader with inherited stdio (interactive — handles password/Steam Guard prompts).
///
/// Kept as a fallback for terminal-mode restart if piped I/O fails to handle credential prompts.
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

/// Run DepotDownloader with `-manifest-only` to produce a human-readable manifest file.
/// No game files are downloaded. The manifest txt is written to `cache_dir`.
pub async fn run_manifest_only(
    binary: &Path,
    app_id: u32,
    depot_id: u32,
    manifest_id: &str,
    username: &str,
    cache_dir: &Path,
) -> Result<(), DepotError> {
    std::fs::create_dir_all(cache_dir)?;
    let mut args = build_args(
        app_id,
        depot_id,
        manifest_id,
        username,
        cache_dir.to_string_lossy().as_ref(),
    );
    args.push("-manifest-only".into());

    let mut cmd = Command::new(binary);
    cmd.args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    let status = cmd.spawn()?.wait().await?;
    if status.success() {
        Ok(())
    } else {
        Err(DepotError::ExitFailure(status.code().unwrap_or(-1)))
    }
}

/// Run DepotDownloader with piped stdin/stdout/stderr.
/// Returns a ChildStdin handle so the caller can forward credential input,
/// and a kill sender that can be used to terminate the child process.
/// Output is streamed via the mpsc sender, with prompt detection.
pub async fn run_depot_downloader(
    binary: &Path,
    app_id: u32,
    depot_id: u32,
    manifest_id: &str,
    username: &str,
    cache_dir: &Path,
    filelist_path: Option<&Path>,
    tx: mpsc::Sender<DepotProgress>,
) -> Result<(tokio::process::ChildStdin, mpsc::Sender<()>), DepotError> {
    std::fs::create_dir_all(cache_dir)?;

    let args = match filelist_path {
        Some(fp) => build_filelist_args(
            app_id,
            depot_id,
            manifest_id,
            username,
            cache_dir.to_string_lossy().as_ref(),
            fp.to_string_lossy().as_ref(),
        ),
        None => build_args(
            app_id,
            depot_id,
            manifest_id,
            username,
            cache_dir.to_string_lossy().as_ref(),
        ),
    };

    let mut cmd = Command::new(binary);
    cmd.args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Detach from the controlling terminal so .NET's Console.Write
    // cannot write directly to /dev/tty and corrupt the TUI.
    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    let mut child = cmd.spawn()?;

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

    let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);
    let tx_done = tx.clone();
    tokio::spawn(async move {
        tokio::select! {
            status = child.wait() => {
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
            }
            _ = kill_rx.recv() => {
                let _ = child.kill().await;
            }
        }
    });

    Ok((stdin, kill_tx))
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

    #[test]
    fn build_filelist_args_includes_filelist_flag() {
        let args = build_filelist_args(570, 571, "abc123", "user", "/tmp/out", "/tmp/list.txt");
        let s: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        assert!(s.contains(&"-filelist"));
        let idx = s.iter().position(|&x| x == "-filelist").unwrap();
        assert_eq!(s[idx + 1], "/tmp/list.txt");
        assert!(s.contains(&"-app"));
        assert!(s.contains(&"-manifest"));
    }
}
