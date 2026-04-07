use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::config::ConfigError;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ManifestDb {
    #[serde(default)]
    pub manifests: HashMap<String, ManifestMeta>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ManifestMeta {
    pub label: Option<String>,
}

impl ManifestDb {
    pub fn get_label(&self, manifest_id: &str) -> Option<&str> {
        self.manifests.get(manifest_id)?.label.as_deref()
    }

    pub fn set_label(&mut self, manifest_id: &str, label: String) {
        self.manifests
            .entry(manifest_id.to_string())
            .or_default()
            .label = Some(label);
    }

    pub fn clear_label(&mut self, manifest_id: &str) {
        if let Some(meta) = self.manifests.get_mut(manifest_id) {
            meta.label = None;
        }
    }
}

pub fn load_manifest_db() -> Result<ManifestDb, ConfigError> {
    let path = crate::config::data_dir()?.join("manifests.toml");
    if !path.exists() {
        return Ok(ManifestDb::default());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&content)?)
}

pub fn save_manifest_db(db: &ManifestDb) -> Result<(), ConfigError> {
    let path = crate::config::data_dir()?.join("manifests.toml");
    let content = toml::to_string_pretty(db)?;
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_label_returns_none_when_no_entry() {
        let db = ManifestDb::default();
        assert!(db.get_label("12345").is_none());
    }

    #[test]
    fn set_and_get_label_roundtrip() {
        let mut db = ManifestDb::default();
        db.set_label("12345", "pre-nerf".to_string());
        assert_eq!(db.get_label("12345"), Some("pre-nerf"));
    }

    #[test]
    fn clear_label_removes_it() {
        let mut db = ManifestDb::default();
        db.set_label("12345", "pre-nerf".to_string());
        db.clear_label("12345");
        assert!(db.get_label("12345").is_none());
    }

    #[test]
    fn clear_label_on_missing_entry_is_noop() {
        let mut db = ManifestDb::default();
        db.clear_label("nonexistent"); // must not panic
    }

    #[test]
    fn toml_roundtrip_preserves_labels() {
        let mut db = ManifestDb::default();
        db.set_label("7291048563840537431", "pre-nerf".to_string());
        db.set_label("8812034512345678901", "1.04".to_string());

        let serialized = toml::to_string_pretty(&db).unwrap();
        let parsed: ManifestDb = toml::from_str(&serialized).unwrap();

        assert_eq!(parsed.get_label("7291048563840537431"), Some("pre-nerf"));
        assert_eq!(parsed.get_label("8812034512345678901"), Some("1.04"));
    }

    #[test]
    fn missing_file_returns_default() {
        // load_manifest_db uses data_dir() which we can't easily redirect in tests,
        // but we can verify that an empty TOML string deserializes to default.
        let db: ManifestDb = toml::from_str("").unwrap();
        assert!(db.manifests.is_empty());
    }
}
