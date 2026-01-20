//! Plugin registry for tracking discovered and connected plugins.
//!
//! The registry maintains two sets of plugins:
//! - Discovered: Plugins found via filesystem manifest.json
//! - Connected: Socket plugins that have registered via RPC
//!
//! Socket plugins override stdio plugins with the same ID.

use hamr_rpc::PluginManifest;
use hamr_rpc::protocol::Message;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::session::SessionId;
#[cfg(test)]
use hamr_core::plugin::PluginDiff;

#[derive(Debug, Clone)]
pub struct ConnectedPlugin {
    pub id: String,
    pub session_id: SessionId,
    pub sender: mpsc::UnboundedSender<Message>,
}

#[derive(Debug, Default)]
pub struct PluginRegistry {
    discovered: HashMap<String, DiscoveredPlugin>,
    connected: HashMap<String, ConnectedPlugin>,
    session_to_plugin: HashMap<SessionId, String>,
}

#[derive(Debug, Clone)]
pub struct DiscoveredPlugin {
    pub id: String,
    pub manifest: PluginManifest,
    pub is_socket: bool,
    pub spawn_command: Option<String>,
    pub is_background: bool,
}

impl PluginRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_discovered(&mut self, plugin: DiscoveredPlugin) {
        debug!("Registering discovered plugin: {}", plugin.id);
        self.discovered.insert(plugin.id.clone(), plugin);
    }

    pub fn register_connected(&mut self, plugin: ConnectedPlugin) {
        debug!("Socket plugin connected: {}", plugin.id);
        let plugin_id = plugin.id.clone();
        let session_id = plugin.session_id.clone();

        self.session_to_plugin.insert(session_id, plugin_id.clone());

        self.connected.insert(plugin_id, plugin);
    }

    pub fn unregister_session(&mut self, session_id: &SessionId) -> Option<String> {
        if let Some(plugin_id) = self.session_to_plugin.remove(session_id) {
            info!("Socket plugin disconnected: {}", plugin_id);
            self.connected.remove(&plugin_id);
            Some(plugin_id)
        } else {
            None
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn get_connected(&self, id: &str) -> Option<&ConnectedPlugin> {
        self.connected.get(id)
    }

    #[cfg(test)]
    #[must_use]
    pub fn get_discovered(&self, id: &str) -> Option<&DiscoveredPlugin> {
        self.discovered.get(id)
    }

    #[must_use]
    pub fn is_connected(&self, id: &str) -> bool {
        self.connected.contains_key(id)
    }

    #[cfg(test)]
    pub fn connected_ids(&self) -> impl Iterator<Item = &String> {
        self.connected.keys()
    }

    #[cfg(test)]
    pub fn discovered_ids(&self) -> impl Iterator<Item = &String> {
        self.discovered.keys()
    }

    pub fn all_plugins(&self) -> impl Iterator<Item = &DiscoveredPlugin> {
        self.discovered.values()
    }

    pub fn pending_background_plugins(&self) -> impl Iterator<Item = &DiscoveredPlugin> {
        self.discovered
            .values()
            .filter(|p| p.is_socket && p.is_background && !self.connected.contains_key(&p.id))
    }

    #[must_use]
    pub fn get_socket_plugin(&self, id: &str) -> Option<&DiscoveredPlugin> {
        self.discovered.get(id).filter(|p| p.is_socket)
    }

    #[must_use]
    pub fn get_plugin_sender(&self, id: &str) -> Option<&mpsc::UnboundedSender<Message>> {
        self.connected.get(id).map(|p| &p.sender)
    }

    #[cfg(test)]
    pub fn clear(&mut self) {
        self.discovered.clear();
        self.connected.clear();
        self.session_to_plugin.clear();
    }

    /// Diff current discovered plugins vs next snapshot
    #[cfg(test)]
    #[must_use]
    pub fn diff_discovered(&self, next: &HashMap<String, DiscoveredPlugin>) -> PluginDiff {
        let mut diff = PluginDiff::default();
        for id in self.discovered.keys() {
            if !next.contains_key(id) {
                diff.removed.push(id.clone());
            }
        }
        for (id, plugin) in next {
            match self.discovered.get(id) {
                None => diff.added.push(id.clone()),
                Some(old)
                    if serde_json::to_string(&old.manifest).ok()
                        != serde_json::to_string(&plugin.manifest).ok()
                        || old.is_socket != plugin.is_socket
                        || old.spawn_command != plugin.spawn_command
                        || old.is_background != plugin.is_background =>
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

    #[cfg(test)]
    pub fn apply_rescan(&mut self, diff: &PluginDiff) {
        debug!(
            "Applying plugin rescan diff: added={:?}, removed={:?}, updated={:?}",
            diff.added, diff.removed, diff.updated
        );
        for id in &diff.removed {
            self.discovered.remove(id);
            if let Some(conn) = self.connected.remove(id) {
                self.session_to_plugin.remove(&conn.session_id);
            }
        }

        for id in &diff.updated {
            self.discovered.remove(id);
            if let Some(conn) = self.connected.remove(id) {
                self.session_to_plugin.remove(&conn.session_id);
            }
        }
    }

    #[cfg(test)]
    pub fn rescan_discovered(&mut self, next: HashMap<String, DiscoveredPlugin>) -> PluginDiff {
        let diff = self.diff_discovered(&next);
        self.apply_rescan(&diff);
        for (id, plugin) in next {
            self.discovered.insert(id, plugin);
        }
        diff
    }

    /// Rescan using core Plugin list
    #[cfg(test)]
    pub fn rescan_from_core_plugins(
        &mut self,
        plugins: Vec<hamr_core::plugin::Plugin>,
    ) -> PluginDiff {
        let next: HashMap<String, DiscoveredPlugin> = plugins
            .into_iter()
            .map(|p| {
                let dp = DiscoveredPlugin {
                    id: p.id.clone(),
                    manifest: PluginManifest {
                        id: p.id.clone(),
                        name: p.manifest.name.clone(),
                        description: p.manifest.description.clone(),
                        icon: p.manifest.icon.clone(),
                        prefix: p.manifest.prefix.clone(),
                        priority: p
                            .manifest
                            .match_config
                            .as_ref()
                            .and_then(|c| c.priority)
                            .unwrap_or(0),
                    },
                    is_socket: p.is_socket(),
                    spawn_command: Some(p.handler_path.display().to_string()),
                    is_background: p.is_background_daemon(),
                };
                (dp.id.clone(), dp)
            })
            .collect();
        self.rescan_discovered(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manifest(id: &str) -> PluginManifest {
        PluginManifest {
            id: id.to_string(),
            name: id.to_string(),
            description: None,
            icon: None,
            prefix: None,
            priority: 0,
        }
    }

    #[test]
    fn test_register_discovered() {
        let mut registry = PluginRegistry::new();
        let plugin = DiscoveredPlugin {
            id: "test".to_string(),
            manifest: make_manifest("test"),
            is_socket: false,
            spawn_command: None,
            is_background: true,
        };

        registry.register_discovered(plugin);

        assert!(registry.get_discovered("test").is_some());
        assert!(registry.get_connected("test").is_none());
    }

    #[tokio::test]
    async fn test_register_connected() {
        let mut registry = PluginRegistry::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let session_id = SessionId::new();

        let plugin = ConnectedPlugin {
            id: "wifi".to_string(),
            session_id: session_id.clone(),
            sender: tx,
        };

        registry.register_connected(plugin);

        assert!(registry.is_connected("wifi"));
        assert!(registry.get_connected("wifi").is_some());
    }

    #[tokio::test]
    async fn test_unregister_session() {
        let mut registry = PluginRegistry::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let session_id = SessionId::new();

        let plugin = ConnectedPlugin {
            id: "timer".to_string(),
            session_id: session_id.clone(),
            sender: tx,
        };

        registry.register_connected(plugin);
        assert!(registry.is_connected("timer"));

        let removed = registry.unregister_session(&session_id);
        assert_eq!(removed, Some("timer".to_string()));
        assert!(!registry.is_connected("timer"));
    }

    #[test]
    fn test_pending_socket_plugins() {
        let mut registry = PluginRegistry::new();

        // Add a background socket plugin (not connected)
        registry.register_discovered(DiscoveredPlugin {
            id: "wifi".to_string(),
            manifest: make_manifest("wifi"),
            is_socket: true,
            spawn_command: Some("python handler.py".to_string()),
            is_background: true,
        });

        // Add a stdio plugin
        registry.register_discovered(DiscoveredPlugin {
            id: "calc".to_string(),
            manifest: make_manifest("calc"),
            is_socket: false,
            spawn_command: None,
            is_background: true,
        });

        let pending: Vec<_> = registry.pending_background_plugins().collect();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "wifi");
    }

    #[test]
    fn test_pending_background_excludes_foreground() {
        let mut registry = PluginRegistry::new();

        // Add a background socket plugin
        registry.register_discovered(DiscoveredPlugin {
            id: "timer".to_string(),
            manifest: make_manifest("timer"),
            is_socket: true,
            spawn_command: Some("python handler.py".to_string()),
            is_background: true,
        });

        // Add a foreground socket plugin (like topmem/topcpu)
        registry.register_discovered(DiscoveredPlugin {
            id: "topmem".to_string(),
            manifest: make_manifest("topmem"),
            is_socket: true,
            spawn_command: Some("python handler.py".to_string()),
            is_background: false,
        });

        // Only background plugins should be pending
        let pending: Vec<_> = registry.pending_background_plugins().collect();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "timer");

        // But we can get the foreground plugin via get_socket_plugin
        let topmem = registry.get_socket_plugin("topmem");
        assert!(topmem.is_some());
        assert!(!topmem.unwrap().is_background);
    }

    #[test]
    fn test_socket_plugin_override() {
        let mut registry = PluginRegistry::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let session_id = SessionId::new();

        // Register a discovered stdio plugin
        registry.register_discovered(DiscoveredPlugin {
            id: "apps".to_string(),
            manifest: make_manifest("apps"),
            is_socket: false,
            spawn_command: None,
            is_background: true,
        });

        // Verify it's in discovered, not connected
        assert!(registry.get_discovered("apps").is_some());
        assert!(!registry.is_connected("apps"));

        // Register a connected socket plugin with same ID
        let plugin = ConnectedPlugin {
            id: "apps".to_string(),
            session_id: session_id.clone(),
            sender: tx,
        };
        registry.register_connected(plugin);

        // Verify it's now connected (socket overrides stdio)
        assert!(registry.is_connected("apps"));
        assert!(registry.get_connected("apps").is_some());
    }

    #[test]
    fn test_session_to_plugin_mapping() {
        let mut registry = PluginRegistry::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let session_id1 = SessionId::new();
        let session_id2 = SessionId::new();

        registry.register_connected(ConnectedPlugin {
            id: "wifi".to_string(),
            session_id: session_id1.clone(),
            sender: tx1,
        });

        registry.register_connected(ConnectedPlugin {
            id: "timer".to_string(),
            session_id: session_id2.clone(),
            sender: tx2,
        });

        // Unregister one session
        let removed = registry.unregister_session(&session_id1);
        assert_eq!(removed, Some("wifi".to_string()));

        // Other should still be connected
        assert!(registry.is_connected("timer"));
        assert!(!registry.is_connected("wifi"));
    }

    #[test]
    fn test_get_plugin_sender() {
        let mut registry = PluginRegistry::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let session_id = SessionId::new();

        registry.register_connected(ConnectedPlugin {
            id: "apps".to_string(),
            session_id,
            sender: tx,
        });

        let sender = registry.get_plugin_sender("apps");
        assert!(sender.is_some());

        let missing = registry.get_plugin_sender("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_diff_discovered_added_removed_updated() {
        // updated manifest detection
        let mut registry = PluginRegistry::new();

        registry.register_discovered(DiscoveredPlugin {
            id: "a".to_string(),
            manifest: make_manifest("a"),
            is_socket: false,
            spawn_command: None,
            is_background: true,
        });

        let mut next = HashMap::new();
        next.insert(
            "b".to_string(),
            DiscoveredPlugin {
                id: "b".to_string(),
                manifest: make_manifest("b"),
                is_socket: false,
                spawn_command: None,
                is_background: true,
            },
        );

        let diff = registry.diff_discovered(&next);
        assert_eq!(diff.added, vec!["b".to_string()]);
        assert_eq!(diff.removed, vec!["a".to_string()]);
        assert!(diff.updated.is_empty());
    }

    #[test]
    fn test_rescan_discovered_updates() {
        // unchanged plugin should not be marked updated
        let mut registry = PluginRegistry::new();
        registry.register_discovered(DiscoveredPlugin {
            id: "a".to_string(),
            manifest: make_manifest("a"),
            is_socket: false,
            spawn_command: None,
            is_background: true,
        });
        let mut next = HashMap::new();
        next.insert(
            "a".to_string(),
            DiscoveredPlugin {
                id: "a".to_string(),
                manifest: make_manifest("a"),
                is_socket: false,
                spawn_command: None,
                is_background: true,
            },
        );
        let diff = registry.rescan_discovered(next);
        assert!(diff.updated.is_empty());
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn test_rescan_from_core_plugins() {
        let mut registry = PluginRegistry::new();
        let plugin = hamr_core::plugin::Plugin {
            id: "a".to_string(),
            path: std::path::PathBuf::from("/tmp/a"),
            handler_path: std::path::PathBuf::from("/tmp/a/handler.py"),
            is_socket: false,
            manifest: hamr_core::plugin::Manifest {
                name: "a".to_string(),
                description: None,
                icon: None,
                prefix: None,
                match_pattern: None,
                match_config: None,
                handler: None,
                daemon: None,
                frecency: None,
                static_index: None,
                input_mode: None,
                hidden: false,
                supported_platforms: Some(vec!["linux".to_string()]),
            },
        };
        let diff = registry.rescan_from_core_plugins(vec![plugin]);
        assert_eq!(diff.added, vec!["a".to_string()]);
    }

    #[test]
    fn test_clear() {
        let mut registry = PluginRegistry::new();
        let (tx, _rx) = mpsc::unbounded_channel();

        registry.register_discovered(DiscoveredPlugin {
            id: "test".to_string(),
            manifest: make_manifest("test"),
            is_socket: false,
            spawn_command: None,
            is_background: true,
        });

        registry.register_connected(ConnectedPlugin {
            id: "apps".to_string(),
            session_id: SessionId::new(),
            sender: tx,
        });

        assert!(!registry.discovered_ids().collect::<Vec<_>>().is_empty());
        assert!(!registry.connected_ids().collect::<Vec<_>>().is_empty());

        registry.clear();

        assert!(registry.discovered_ids().collect::<Vec<_>>().is_empty());
        assert!(registry.connected_ids().collect::<Vec<_>>().is_empty());
    }
}
