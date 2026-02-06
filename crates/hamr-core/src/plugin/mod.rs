mod checksum;
mod convert;
mod manifest;
mod process;
mod protocol;

pub use checksum::{ChecksumsData, PluginVerifyStatus};
pub use convert::{plugin_response_to_updates, process_status_data};

pub use manifest::{
    DaemonConfig, FrecencyMode, Handler, HandlerType, InputMode, Manifest, MatchConfig,
    StaticIndexItem,
};
pub use process::{PluginProcess, PluginReceiver, PluginSender, invoke_match};
pub use protocol::{
    ActionSource, AmbientItemData, CardBlockData, CardResponseData, ExecuteData, FabData,
    ImageBrowserInner, IndexItem, IndexMode, PluginInput, PluginResponse, SelectedItem, StatusData,
    Step, UpdateItem,
};

use crate::config::Directories;
use crate::platform::{self, Platform};
use crate::{Error, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

const MANIFEST_FILENAME: &str = "manifest.json";
const DEFAULT_HANDLER_FILENAME: &str = "handler.py";

/// A loaded plugin
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Plugin {
    pub id: String,
    pub path: PathBuf,
    pub manifest: Manifest,
    pub handler_path: PathBuf,
    /// Whether this plugin uses socket communication (handled by daemon)
    pub is_socket: bool,
    /// Pre-compiled regex patterns from manifest match config
    #[serde(skip)]
    pub compiled_patterns: Vec<regex::Regex>,
}

impl Plugin {
    /// Load a plugin from a directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the manifest is missing, invalid, or the handler is missing.
    pub fn load(path: PathBuf) -> Result<Self> {
        let manifest_path = path.join(MANIFEST_FILENAME);
        if !manifest_path.exists() {
            return Err(Error::Plugin(format!(
                "{} not found in {}",
                MANIFEST_FILENAME,
                path.display()
            )));
        }

        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let manifest: Manifest = serde_json::from_str(&manifest_content).map_err(|e| {
            Error::Plugin(format!(
                "Failed to parse manifest at {}: {}",
                manifest_path.display(),
                e
            ))
        })?;

        let id = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::Plugin(format!("Invalid plugin directory: {}", path.display())))?
            .to_string();

        let handler_path = path.join(DEFAULT_HANDLER_FILENAME);
        if !handler_path.exists() && manifest.static_index.is_none() {
            return Err(Error::Plugin(format!(
                "{} not found in {} (required unless staticIndex is provided)",
                DEFAULT_HANDLER_FILENAME,
                path.display()
            )));
        }

        let mut compiled_patterns = Vec::new();

        if let Some(pattern) = &manifest.match_pattern {
            match regex::Regex::new(pattern) {
                Ok(re) => compiled_patterns.push(re),
                Err(err) => warn!(
                    "Plugin {}: invalid match pattern '{}': {}",
                    id, pattern, err
                ),
            }
        }

        if let Some(match_config) = &manifest.match_config {
            for pattern in &match_config.patterns {
                match regex::Regex::new(pattern) {
                    Ok(re) => compiled_patterns.push(re),
                    Err(err) => {
                        warn!(
                            "Plugin {}: invalid match pattern '{}': {}",
                            id, pattern, err
                        );
                    }
                }
            }
        }

        let is_socket = manifest.is_socket();
        Ok(Self {
            id,
            path,
            manifest,
            handler_path,
            is_socket,
            compiled_patterns,
        })
    }

    /// Check if plugin matches a query based on prefix or pattern
    #[must_use]
    pub fn matches_query(&self, query: &str) -> Option<String> {
        if let Some(prefix) = &self.manifest.prefix
            && query.starts_with(prefix)
        {
            return Some(query[prefix.len()..].to_string());
        }

        for re in &self.compiled_patterns {
            if re.is_match(query) {
                return Some(query.to_string());
            }
        }

        None
    }

    /// Check if plugin is a daemon (persistent process)
    #[must_use]
    pub fn is_daemon(&self) -> bool {
        self.manifest.daemon.as_ref().is_some_and(|d| d.enabled)
    }

    /// Check if plugin is a background daemon (starts on hamr launch)
    #[must_use]
    pub fn is_background_daemon(&self) -> bool {
        self.manifest
            .daemon
            .as_ref()
            .is_some_and(|d| d.enabled && d.background)
    }

    /// Check if plugin has indexed items
    #[must_use]
    pub fn has_index(&self) -> bool {
        self.manifest.static_index.is_some() || self.is_daemon()
    }

    /// Check if plugin uses socket communication (handled by daemon)
    #[must_use]
    pub fn is_socket(&self) -> bool {
        self.is_socket
    }
}

/// Result of plugin rescan diff
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PluginDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub updated: Vec<String>,
}

impl PluginDiff {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.updated.is_empty()
    }
}

/// Compute diff between plugin sets
pub fn diff_plugins<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    before: &HashMap<String, Plugin, S1>,
    after: &HashMap<String, Plugin, S2>,
) -> PluginDiff {
    let mut diff = PluginDiff::default();

    for id in before.keys() {
        if !after.contains_key(id) {
            diff.removed.push(id.clone());
        }
    }

    for (id, plugin) in after {
        match before.get(id) {
            None => diff.added.push(id.clone()),
            Some(old)
                if old.manifest != plugin.manifest
                    || old.handler_path != plugin.handler_path
                    || old.is_socket != plugin.is_socket =>
            {
                diff.updated.push(id.clone());
            }
            _ => {}
        }
    }
    diff.added.sort();
    diff.removed.sort();
    diff.updated.sort();
    diff
}

/// Manages plugin discovery and lifecycle
pub struct PluginManager {
    plugins: HashMap<String, Plugin>,
    plugin_order: Vec<String>,
    builtin_path: PathBuf,
    user_path: PathBuf,
    platform: Platform,
}

impl PluginManager {
    pub fn new(dirs: &Directories) -> Self {
        let platform = platform::detect();
        info!("Detected platform: {}", platform.as_str());
        Self {
            plugins: HashMap::new(),
            plugin_order: Vec::new(),
            builtin_path: dirs.builtin_plugins.clone(),
            user_path: dirs.user_plugins.clone(),
            platform,
        }
    }

    /// Get the current platform
    #[must_use]
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// Discover and load all plugins.
    ///
    /// # Errors
    ///
    /// Returns an error if loading any plugin directory fails.
    pub fn discover(&mut self) -> Result<()> {
        self.plugins.clear();
        self.plugin_order.clear();

        if self.builtin_path.exists() {
            self.load_plugins_from(&self.builtin_path.clone())?;
        }

        if self.user_path.exists() {
            self.load_plugins_from(&self.user_path.clone())?;
        }

        info!(
            "Loaded {} plugins: {:?}",
            self.plugins.len(),
            self.plugin_order
        );
        Ok(())
    }

    /// Retry platform detection if unknown at startup (compositor wasn't ready).
    ///
    /// # Errors
    /// Returns an error if plugin discovery fails.
    pub fn retry_platform_detection(&mut self) -> Result<bool> {
        if self.platform != Platform::Unknown {
            return Ok(false);
        }

        let new_platform = platform::detect();
        if new_platform == Platform::Unknown {
            return Ok(false);
        }

        info!("Platform detected on retry: {}", new_platform.as_str());
        self.platform = new_platform;
        self.discover()?;
        Ok(true)
    }

    /// Rescan for plugin changes.
    ///
    /// # Errors
    ///
    /// Returns an error if plugin discovery fails.
    pub fn rescan(&mut self) -> Result<PluginDiff> {
        debug!("Rescanning plugins for changes");
        let before = self.plugins.clone();
        self.discover()?;
        let after = self.plugins.clone();
        Ok(diff_plugins(&before, &after))
    }

    #[cfg(test)]
    pub fn rescan_from_snapshot_for_test(&mut self, next: HashMap<String, Plugin>) -> PluginDiff {
        let before = self.plugins.clone();
        self.plugins = next;
        diff_plugins(&before, &self.plugins)
    }

    /// Rescan and return diff with list of plugins.
    ///
    /// # Errors
    ///
    /// Returns an error if plugin discovery fails.
    pub fn rescan_with_plugins(&mut self) -> Result<(PluginDiff, Vec<Plugin>)> {
        let diff = self.rescan()?;
        Ok((diff, self.all_owned()))
    }

    /// Rescan and return diff with snapshot map.
    ///
    /// # Errors
    ///
    /// Returns an error if plugin discovery fails.
    pub fn rescan_and_snapshot(&mut self) -> Result<(PluginDiff, HashMap<String, Plugin>)> {
        let diff = self.rescan()?;
        Ok((diff, self.snapshot()))
    }

    #[cfg(test)]
    pub fn insert_for_test(&mut self, plugin: Plugin) {
        if !self.plugins.contains_key(&plugin.id) {
            self.plugin_order.push(plugin.id.clone());
        }
        self.plugins.insert(plugin.id.clone(), plugin);
    }

    #[cfg(test)]
    pub fn rescan_with_plugins_for_test(&mut self) -> (PluginDiff, Vec<Plugin>) {
        let diff = PluginDiff::default();
        (diff, self.all_owned())
    }

    #[must_use]
    pub fn snapshot(&self) -> HashMap<String, Plugin> {
        self.plugins.clone()
    }

    fn load_plugins_from(&mut self, path: &Path) -> Result<()> {
        let entries = std::fs::read_dir(path)?;
        let platform_str = self.platform.as_str();

        for entry in entries.flatten() {
            let plugin_path = entry.path();
            if !plugin_path.is_dir() {
                continue;
            }

            match Plugin::load(plugin_path.clone()) {
                Ok(plugin) => {
                    if !plugin.manifest.supports_platform(platform_str) {
                        debug!(
                            "Skipping plugin {} - not supported on platform '{}'",
                            plugin.id, platform_str
                        );
                        continue;
                    }

                    debug!(
                        "Loaded plugin: {} from {}",
                        plugin.id,
                        plugin_path.display()
                    );
                    if !self.plugins.contains_key(&plugin.id) {
                        self.plugin_order.push(plugin.id.clone());
                    }
                    self.plugins.insert(plugin.id.clone(), plugin);
                }
                Err(e) => {
                    warn!(
                        "Failed to load plugin from {}: {}",
                        plugin_path.display(),
                        e
                    );
                }
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Plugin> {
        self.plugins.get(id)
    }

    pub fn all(&self) -> impl Iterator<Item = &Plugin> {
        self.plugin_order
            .iter()
            .filter_map(|id| self.plugins.get(id))
    }

    #[must_use]
    pub fn all_owned(&self) -> Vec<Plugin> {
        self.all().cloned().collect()
    }

    /// Get plugins that should start as background daemons
    pub fn background_daemons(&self) -> impl Iterator<Item = &Plugin> {
        self.all().filter(|p| p.is_background_daemon())
    }

    /// Find a plugin that matches the given query (respects priority)
    #[must_use]
    pub fn find_matching(&self, query: &str) -> Option<(&Plugin, String)> {
        let mut best_match: Option<(&Plugin, String, i32)> = None;

        for plugin in self.all() {
            if let Some(remaining) = plugin.matches_query(query) {
                let priority = plugin
                    .manifest
                    .match_config
                    .as_ref()
                    .and_then(|c| c.priority)
                    .unwrap_or(0);

                if best_match.as_ref().is_none_or(|(_, _, p)| priority > *p) {
                    best_match = Some((plugin, remaining, priority));
                }
            }
        }

        best_match.map(|(plugin, remaining, _)| (plugin, remaining))
    }

    #[must_use]
    pub fn ids(&self) -> &[String] {
        &self.plugin_order
    }
}

#[cfg(test)]
mod diff_tests {
    use super::*;

    fn make_plugin(id: &str, priority: i32) -> Plugin {
        let manifest = Manifest {
            name: id.to_string(),
            description: None,
            icon: None,
            prefix: None,
            match_pattern: None,
            match_config: Some(MatchConfig {
                patterns: Vec::new(),
                priority: Some(priority),
            }),
            handler: None,
            daemon: None,
            frecency: None,
            static_index: None,
            input_mode: None,
            hidden: false,
            supported_platforms: Some(vec!["linux".to_string()]),
        };

        Plugin {
            id: id.to_string(),
            path: PathBuf::from("/tmp"),
            handler_path: PathBuf::from("/tmp/handler.py"),
            is_socket: false,
            manifest,
            compiled_patterns: Vec::new(),
        }
    }

    #[test]
    fn test_diff_plugins_added_removed_updated() {
        let mut before = HashMap::new();
        before.insert("a".into(), make_plugin("a", 0));

        let mut after = HashMap::new();
        after.insert("b".into(), make_plugin("b", 0));
        after.insert("a".into(), make_plugin("a", 1));

        let diff = diff_plugins(&before, &after);
        assert_eq!(diff.added, vec!["b".to_string()]);
        assert_eq!(diff.removed, Vec::<String>::new());
        assert_eq!(diff.updated, vec!["a".to_string()]);
    }
}
