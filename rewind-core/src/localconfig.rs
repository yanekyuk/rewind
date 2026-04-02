// rewind-core/src/localconfig.rs
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LocalConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("app {0} not found in localconfig.vdf")]
    AppNotFound(u32),
}

/// Scan Steam's userdata directory for all localconfig.vdf files.
/// Uses steamlocate to find the Steam root — works on Linux, Windows, macOS.
/// Returns paths sorted by modification time, most recently modified first.
/// When multiple Steam accounts exist, callers use the first (most recent) path.
pub fn find_localconfig_paths() -> Vec<PathBuf> {
    let Ok(steam) = steamlocate::SteamDir::locate() else {
        return Vec::new();
    };
    let userdata = steam.path().join("userdata");
    if !userdata.exists() {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(&userdata) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path().join("config").join("localconfig.vdf"))
        .filter(|p| p.exists())
        .collect();
    // Sort most-recently-modified first so callers get the active account's file.
    paths.sort_by_key(|p| {
        std::cmp::Reverse(
            p.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
        )
    });
    paths
}

/// Read the LaunchOptions value for `app_id` from a localconfig.vdf file.
pub fn read_launch_options(path: &Path, app_id: u32) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    extract_launch_options(&content, app_id)
}

/// Write (replace or insert) the LaunchOptions value for `app_id` in a localconfig.vdf file.
/// Pass an empty string to remove the LaunchOptions key.
pub fn write_launch_options(
    path: &Path,
    app_id: u32,
    options: &str,
) -> Result<(), LocalConfigError> {
    let content = std::fs::read_to_string(path)?;
    let updated = set_launch_options(&content, app_id, options);
    if updated == content && !options.is_empty() {
        // set_launch_options made no change — app_id not present in file
        return Err(LocalConfigError::AppNotFound(app_id));
    }
    std::fs::write(path, updated)?;
    Ok(())
}

// ── Internal helpers ─────────────────────────────────────────────────────────

pub(crate) fn extract_launch_options(content: &str, app_id: u32) -> Option<String> {
    let app_key = format!("\"{}\"", app_id);
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        if lines[i].trim() == app_key {
            // Expect an opening brace on the next non-empty line
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() && lines[j].trim() == "{" {
                let mut depth = 1i32;
                j += 1;
                while j < lines.len() {
                    let t = lines[j].trim();
                    if t == "{" {
                        depth += 1;
                    } else if t == "}" {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    } else if depth == 1 {
                        if let Some(val) = parse_kv(t, "LaunchOptions") {
                            return Some(val.to_string());
                        }
                    }
                    j += 1;
                }
            }
        }
        i += 1;
    }
    None
}

/// Replace or insert `LaunchOptions` for `app_id` in the VDF content string.
pub(crate) fn set_launch_options(content: &str, app_id: u32, options: &str) -> String {
    let app_key = format!("\"{}\"", app_id);
    let lines: Vec<&str> = content.lines().collect();
    let mut result: Vec<String> = Vec::with_capacity(lines.len() + 2);
    let mut i = 0;

    while i < lines.len() {
        if lines[i].trim() == app_key {
            result.push(lines[i].to_string());
            i += 1;
            // Skip to opening brace
            while i < lines.len() && lines[i].trim().is_empty() {
                result.push(lines[i].to_string());
                i += 1;
            }
            if i < lines.len() && lines[i].trim() == "{" {
                result.push(lines[i].to_string());
                i += 1;

                // Determine indentation from surrounding lines
                let indent = lines[i..]
                    .iter()
                    .find(|l| !l.trim().is_empty() && l.trim() != "}")
                    .map(|l| {
                        let trimmed_len = l.trim_start().len();
                        &l[..l.len() - trimmed_len]
                    })
                    .unwrap_or("\t\t\t\t\t\t\t\t");

                let mut depth = 1i32;
                let mut inserted = false;

                while i < lines.len() {
                    let t = lines[i].trim();
                    if t == "{" {
                        depth += 1;
                        result.push(lines[i].to_string());
                        i += 1;
                    } else if t == "}" {
                        depth -= 1;
                        if depth == 0 {
                            // Closing brace of app block — insert if not yet done
                            if !inserted && !options.is_empty() {
                                result.push(format!(
                                    "{}\"LaunchOptions\"\t\t\"{}\"",
                                    indent, options
                                ));
                            }
                            result.push(lines[i].to_string());
                            i += 1;
                            break;
                        }
                        result.push(lines[i].to_string());
                        i += 1;
                    } else if depth == 1 && parse_kv(t, "LaunchOptions").is_some() {
                        // Replace or remove this line
                        if !options.is_empty() {
                            result.push(format!(
                                "{}\"LaunchOptions\"\t\t\"{}\"",
                                indent, options
                            ));
                        }
                        // If options is empty, we drop the line (removes the key)
                        inserted = true;
                        i += 1;
                    } else {
                        result.push(lines[i].to_string());
                        i += 1;
                    }
                }
                continue;
            }
        } else {
            result.push(lines[i].to_string());
            i += 1;
        }
    }

    result.join("\n")
}

/// Parse a VDF key-value line of the form `"Key"\t\t"Value"`.
/// Returns the value string if the key matches, otherwise None.
pub(crate) fn parse_kv<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let expected_key = format!("\"{}\"", key);
    let rest = line.strip_prefix(expected_key.as_str())?.trim_start_matches(['\t', ' ']);
    rest.strip_prefix('"')?.strip_suffix('"')
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_VDF: &str = r#""UserLocalConfigStore"
{
	"Software"
	{
		"Valve"
		{
			"Steam"
			{
				"apps"
				{
					"3472040"
					{
						"LastPlayed"		"1700000000"
						"LaunchOptions"		"mangohud %command%"
					}
					"570"
					{
						"LastPlayed"		"1700000001"
					}
				}
			}
		}
	}
}"#;

    #[test]
    fn read_existing_launch_options() {
        let result = extract_launch_options(SAMPLE_VDF, 3472040);
        assert_eq!(result, Some("mangohud %command%".to_string()));
    }

    #[test]
    fn read_missing_launch_options_returns_none() {
        // App 570 has no LaunchOptions key
        let result = extract_launch_options(SAMPLE_VDF, 570);
        assert_eq!(result, None);
    }

    #[test]
    fn read_unknown_app_returns_none() {
        let result = extract_launch_options(SAMPLE_VDF, 99999);
        assert_eq!(result, None);
    }

    #[test]
    fn write_replaces_existing_launch_options() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("localconfig.vdf");
        std::fs::write(&path, SAMPLE_VDF).unwrap();

        write_launch_options(&path, 3472040, "WINEDLLOVERRIDES=\"dxgi=n,b\" %command%").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("WINEDLLOVERRIDES=\"dxgi=n,b\" %command%"));
        // mangohud line should be gone
        assert!(!content.contains("mangohud %command%"));
    }

    #[test]
    fn write_inserts_when_key_absent() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("localconfig.vdf");
        std::fs::write(&path, SAMPLE_VDF).unwrap();

        // App 570 has no LaunchOptions — should be inserted
        write_launch_options(&path, 570, "WINEDLLOVERRIDES=\"d3d9=n,b\" %command%").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("WINEDLLOVERRIDES=\"d3d9=n,b\" %command%"));
    }

    #[test]
    fn write_returns_error_when_app_not_found() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("localconfig.vdf");
        std::fs::write(&path, SAMPLE_VDF).unwrap();

        let result = write_launch_options(&path, 99999, "WINEDLLOVERRIDES=\"dxgi=n,b\" %command%");
        assert!(matches!(result, Err(LocalConfigError::AppNotFound(99999))));
    }

    #[test]
    fn write_empty_string_removes_launch_options() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("localconfig.vdf");
        std::fs::write(&path, SAMPLE_VDF).unwrap();

        write_launch_options(&path, 3472040, "").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        // LaunchOptions key should be removed (empty value = restore to nothing)
        assert!(!content.contains("\"LaunchOptions\""));
    }
}
