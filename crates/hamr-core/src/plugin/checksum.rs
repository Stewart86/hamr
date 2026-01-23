//! Plugin checksum verification for security auditing.
//!
//! This module provides SHA256 checksum verification for plugin files against
//! a known-good checksums.json file (generated at release time).

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};
use tracing::{debug, warn};

/// Result of verifying a single plugin's files
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginVerifyStatus {
    /// All files match expected checksums
    Verified,
    /// Some files have mismatched checksums
    Modified(Vec<String>),
    /// Plugin is not in the checksums file (user-installed or new)
    Unknown,
}

/// Checksums data loaded from checksums.json
#[derive(Debug, Clone, Default)]
pub struct ChecksumsData {
    /// Map of plugin_id to (filename to sha256 hex string)
    plugins: HashMap<String, HashMap<String, String>>,
}

impl ChecksumsData {
    /// Load checksums from a checksums.json file.
    ///
    /// Returns `None` if the file doesn't exist or can't be parsed.
    #[must_use]
    pub fn load(path: &Path) -> Option<Self> {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                debug!("Could not read checksums file {:?}: {}", path, e);
                return None;
            }
        };

        let json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                warn!("Failed to parse checksums file {:?}: {}", path, e);
                return None;
            }
        };

        let version = json
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        if version != 1 {
            warn!(
                "Unsupported checksums.json version: {} (expected 1)",
                version
            );
            return None;
        }

        let plugins_obj = json.get("plugins")?.as_object()?;
        let mut plugins = HashMap::new();

        for (plugin_id, files_value) in plugins_obj {
            let files_obj = files_value.as_object()?;
            let mut files = HashMap::new();

            for (filename, hash_value) in files_obj {
                if let Some(hash) = hash_value.as_str() {
                    files.insert(filename.clone(), hash.to_string());
                }
            }

            plugins.insert(plugin_id.clone(), files);
        }

        Some(Self { plugins })
    }

    /// Verify a plugin's files against expected checksums.
    ///
    /// Returns the verification status indicating whether files match, were modified,
    /// or the plugin is unknown (not in checksums).
    #[must_use]
    pub fn verify_plugin(&self, plugin_id: &str, plugin_path: &Path) -> PluginVerifyStatus {
        let Some(expected_files) = self.plugins.get(plugin_id) else {
            return PluginVerifyStatus::Unknown;
        };

        let mut modified = Vec::new();

        for (filename, expected_hash) in expected_files {
            let file_path = plugin_path.join(filename);

            match compute_file_hash(&file_path) {
                Some(actual_hash) => {
                    if actual_hash != *expected_hash {
                        modified.push(filename.clone());
                    }
                }
                None => {
                    modified.push(filename.clone());
                }
            }
        }

        if modified.is_empty() {
            PluginVerifyStatus::Verified
        } else {
            PluginVerifyStatus::Modified(modified)
        }
    }

    /// Check if checksums data is available (non-empty)
    #[must_use]
    pub fn is_available(&self) -> bool {
        !self.plugins.is_empty()
    }

    /// Get the number of plugins with checksums
    #[must_use]
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

/// Compute SHA256 hash of a file, returning hex string
fn compute_file_hash(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).ok()?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Some(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_checksums_json(dir: &Path, content: &str) {
        let path = dir.join("checksums.json");
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    fn create_plugin_file(plugin_dir: &Path, filename: &str, content: &str) {
        let path = plugin_dir.join(filename);
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_load_valid_checksums() {
        let temp = TempDir::new().unwrap();
        create_checksums_json(
            temp.path(),
            r#"{
                "version": 1,
                "generated": "2026-01-23",
                "plugins": {
                    "test-plugin": {
                        "manifest.json": "abc123",
                        "handler.py": "def456"
                    }
                }
            }"#,
        );

        let checksums = ChecksumsData::load(&temp.path().join("checksums.json"));
        assert!(checksums.is_some());
        let checksums = checksums.unwrap();
        assert!(checksums.is_available());
        assert_eq!(checksums.plugin_count(), 1);
    }

    #[test]
    fn test_load_missing_file() {
        let temp = TempDir::new().unwrap();
        let checksums = ChecksumsData::load(&temp.path().join("nonexistent.json"));
        assert!(checksums.is_none());
    }

    #[test]
    fn test_load_invalid_version() {
        let temp = TempDir::new().unwrap();
        create_checksums_json(
            temp.path(),
            r#"{
                "version": 99,
                "plugins": {}
            }"#,
        );

        let checksums = ChecksumsData::load(&temp.path().join("checksums.json"));
        assert!(checksums.is_none());
    }

    #[test]
    fn test_verify_plugin_verified() {
        let temp = TempDir::new().unwrap();
        let plugin_dir = temp.path().join("test-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        create_plugin_file(&plugin_dir, "handler.py", "print('hello')");

        let handler_hash = compute_file_hash(&plugin_dir.join("handler.py")).unwrap();

        create_checksums_json(
            temp.path(),
            &format!(
                r#"{{
                "version": 1,
                "plugins": {{
                    "test-plugin": {{
                        "handler.py": "{handler_hash}"
                    }}
                }}
            }}"#
            ),
        );

        let checksums = ChecksumsData::load(&temp.path().join("checksums.json")).unwrap();
        let status = checksums.verify_plugin("test-plugin", &plugin_dir);
        assert_eq!(status, PluginVerifyStatus::Verified);
    }

    #[test]
    fn test_verify_plugin_modified() {
        let temp = TempDir::new().unwrap();
        let plugin_dir = temp.path().join("test-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        create_plugin_file(&plugin_dir, "handler.py", "print('modified')");

        create_checksums_json(
            temp.path(),
            r#"{
                "version": 1,
                "plugins": {
                    "test-plugin": {
                        "handler.py": "wrong_hash_here"
                    }
                }
            }"#,
        );

        let checksums = ChecksumsData::load(&temp.path().join("checksums.json")).unwrap();
        let status = checksums.verify_plugin("test-plugin", &plugin_dir);
        assert!(matches!(status, PluginVerifyStatus::Modified(_)));
        if let PluginVerifyStatus::Modified(files) = status {
            assert!(files.contains(&"handler.py".to_string()));
        }
    }

    #[test]
    fn test_verify_plugin_unknown() {
        let temp = TempDir::new().unwrap();
        let plugin_dir = temp.path().join("unknown-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        create_checksums_json(
            temp.path(),
            r#"{
                "version": 1,
                "plugins": {}
            }"#,
        );

        let checksums = ChecksumsData::load(&temp.path().join("checksums.json")).unwrap();
        let status = checksums.verify_plugin("unknown-plugin", &plugin_dir);
        assert_eq!(status, PluginVerifyStatus::Unknown);
    }

    #[test]
    fn test_verify_plugin_missing_file() {
        let temp = TempDir::new().unwrap();
        let plugin_dir = temp.path().join("test-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        create_checksums_json(
            temp.path(),
            r#"{
                "version": 1,
                "plugins": {
                    "test-plugin": {
                        "handler.py": "some_hash"
                    }
                }
            }"#,
        );

        let checksums = ChecksumsData::load(&temp.path().join("checksums.json")).unwrap();
        let status = checksums.verify_plugin("test-plugin", &plugin_dir);
        assert!(matches!(status, PluginVerifyStatus::Modified(_)));
    }

    #[test]
    fn test_compute_file_hash() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(b"hello world").unwrap();

        let hash = compute_file_hash(&file_path);
        assert!(hash.is_some());
        assert_eq!(
            hash.unwrap(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
