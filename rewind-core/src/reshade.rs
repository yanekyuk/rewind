// rewind-core/src/reshade.rs
use crate::config::ReshadeApi;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReshadeError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("extraction failed — could not locate 7z stream in installer")]
    ExtractionFailed,
    #[error("ReShade installer or DLL not found")]
    NotFound,
    #[error("symlink conflict: a real file already exists at {0}")]
    SymlinkConflict(String),
    #[error("sevenz error: {0}")]
    SevenZ(String),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

#[derive(Debug, Clone)]
pub enum ReshadeProgress {
    Line(String),
    Done,
    Error(String),
}

pub fn reshade_dll_path(bin_dir: &Path) -> PathBuf {
    bin_dir.join("ReShade64.dll")
}

pub fn reshade_shaders_cache_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("reshade-shaders")
}

impl ReshadeApi {
    pub fn dll_name(&self) -> &'static str {
        match self {
            ReshadeApi::Dxgi => "dxgi.dll",
            ReshadeApi::D3d9 => "d3d9.dll",
            ReshadeApi::OpenGl32 => "opengl32.dll",
            ReshadeApi::Vulkan1 => "vulkan-1.dll",
        }
    }

    /// Returns the WINEDLLOVERRIDES Steam launch command for this API (Linux/Proton).
    pub fn linux_launch_command(&self) -> String {
        // Strip ".dll" suffix for WINEDLLOVERRIDES key
        let key = self.dll_name().trim_end_matches(".dll");
        format!("WINEDLLOVERRIDES=\"{}=n,b\" %command%", key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dll_name_correct_for_each_api() {
        assert_eq!(ReshadeApi::Dxgi.dll_name(), "dxgi.dll");
        assert_eq!(ReshadeApi::D3d9.dll_name(), "d3d9.dll");
        assert_eq!(ReshadeApi::OpenGl32.dll_name(), "opengl32.dll");
        assert_eq!(ReshadeApi::Vulkan1.dll_name(), "vulkan-1.dll");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_launch_command_includes_dll_stem() {
        let cmd = ReshadeApi::Dxgi.linux_launch_command();
        assert!(cmd.contains("dxgi=n,b"));
        assert!(cmd.contains("%command%"));

        let cmd9 = ReshadeApi::D3d9.linux_launch_command();
        assert!(cmd9.contains("d3d9=n,b"));
    }

    #[test]
    fn reshade_dll_path_uses_bin_dir() {
        let p = reshade_dll_path(std::path::Path::new("/tmp/bin"));
        assert_eq!(p, std::path::Path::new("/tmp/bin/ReShade64.dll"));
    }

    #[test]
    fn reshade_shaders_cache_path_uses_cache_dir() {
        let p = reshade_shaders_cache_path(std::path::Path::new("/tmp/cache"));
        assert_eq!(p, std::path::Path::new("/tmp/cache/reshade-shaders"));
    }
}
