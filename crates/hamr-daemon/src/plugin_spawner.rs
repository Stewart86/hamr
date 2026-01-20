//! Socket plugin spawner for discovered plugins.
//!
//! This module handles spawning socket plugins that are discovered via filesystem
//! manifests. Spawned plugins connect back to the daemon to register themselves.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStderr, Command};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::registry::DiscoveredPlugin;

fn spawn_stderr_logger(plugin_id: String, stderr: ChildStderr) {
    tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            debug!("[{}] plugin stderr: {}", plugin_id, line);
        }
    });
}

fn spawn_command(command: &str, working_dir: &Path) -> Result<Child, String> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    let program = parts[0];
    let args = &parts[1..];

    Command::new(program)
        .args(args)
        .current_dir(working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to spawn: {e}"))
}

#[derive(Debug, Clone)]
pub struct SpawnConfig {
    pub max_restarts: u32,
    pub restart_delay: Duration,
    pub max_restart_delay: Duration,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            max_restarts: 5,
            restart_delay: Duration::from_secs(1),
            max_restart_delay: Duration::from_secs(60),
        }
    }
}

#[derive(Debug)]
pub struct SpawnedPlugin {
    pub child: Child,
    pub restart_count: u32,
    pub working_dir: PathBuf,
    pub command: String,
}

impl SpawnedPlugin {
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    pub async fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill().await
    }
}

#[derive(Debug, Default)]
pub struct PluginSpawner {
    spawned: HashMap<String, SpawnedPlugin>,
    config: SpawnConfig,
}

impl PluginSpawner {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub fn spawn(&mut self, plugin: &DiscoveredPlugin) -> Result<(), String> {
        let working_dir =
            std::env::current_dir().map_err(|e| format!("Failed to get current dir: {e}"))?;
        self.spawn_in_dir(plugin, &working_dir)
    }

    pub fn spawn_in_dir(
        &mut self,
        plugin: &DiscoveredPlugin,
        working_dir: &Path,
    ) -> Result<(), String> {
        let command = plugin
            .spawn_command
            .as_ref()
            .ok_or_else(|| format!("Plugin {} has no spawn command", plugin.id))?;

        if command.split_whitespace().next().is_none() {
            return Err(format!("Empty spawn command for plugin {}", plugin.id));
        }

        debug!("[{}] Spawning: {} in {:?}", plugin.id, command, working_dir);

        let mut child = spawn_command(command, working_dir)
            .map_err(|e| format!("Failed to spawn plugin {}: {}", plugin.id, e))?;

        if let Some(stderr) = child.stderr.take() {
            spawn_stderr_logger(plugin.id.clone(), stderr);
        }

        let spawned = SpawnedPlugin {
            child,
            restart_count: 0,
            working_dir: working_dir.to_path_buf(),
            command: command.clone(),
        };

        self.spawned.insert(plugin.id.clone(), spawned);
        debug!("[{}] Plugin spawned successfully", plugin.id);

        Ok(())
    }

    pub async fn check_and_restart(&mut self) {
        let mut to_restart = Vec::new();

        for (id, spawned) in &mut self.spawned {
            if !spawned.is_running() {
                let exit_status = match spawned.child.try_wait() {
                    Ok(Some(status)) => format!("{status}"),
                    Ok(None) => "unknown".to_string(),
                    Err(e) => {
                        warn!("[{}] Failed to get exit status: {}", id, e);
                        "error".to_string()
                    }
                };

                warn!("[{}] Plugin exited with status: {}", id, exit_status);

                if spawned.restart_count < self.config.max_restarts {
                    to_restart.push((
                        id.clone(),
                        spawned.command.clone(),
                        spawned.working_dir.clone(),
                        spawned.restart_count + 1,
                    ));
                } else {
                    error!(
                        "[{}] Max restarts ({}) exceeded, not restarting",
                        id, self.config.max_restarts
                    );
                }
            }
        }

        for (id, command, working_dir, restart_count) in to_restart {
            let delay = self.config.restart_delay * 2u32.pow(restart_count.saturating_sub(1));
            let delay = delay.min(self.config.max_restart_delay);

            info!(
                "[{}] Restarting plugin (attempt {}/{}) after {:?}",
                id, restart_count, self.config.max_restarts, delay
            );

            sleep(delay).await;

            match spawn_command(&command, &working_dir) {
                Ok(mut child) => {
                    if let Some(stderr) = child.stderr.take() {
                        spawn_stderr_logger(id.clone(), stderr);
                    }

                    self.spawned.insert(
                        id.clone(),
                        SpawnedPlugin {
                            child,
                            restart_count,
                            working_dir,
                            command,
                        },
                    );
                    info!("[{}] Plugin restarted successfully", id);
                }
                Err(e) => {
                    error!("[{}] Failed to restart plugin: {}", id, e);
                }
            }
        }
    }

    #[cfg(test)]
    pub async fn stop_all(&mut self) {
        for (id, mut spawned) in self.spawned.drain() {
            info!("[{}] Stopping plugin", id);
            if let Err(e) = spawned.kill().await {
                warn!("[{}] Failed to kill plugin: {}", id, e);
            }
        }
    }

    pub async fn stop_plugin(&mut self, id: &str) -> bool {
        if let Some(mut spawned) = self.spawned.remove(id) {
            info!("[{}] Stopping plugin (on-demand cleanup)", id);
            if let Err(e) = spawned.kill().await {
                warn!("[{}] Failed to kill plugin: {}", id, e);
            }
            true
        } else {
            debug!("[{}] Plugin not found in spawner (may not be running)", id);
            false
        }
    }

    #[cfg(test)]
    pub fn spawned_ids(&self) -> impl Iterator<Item = &String> {
        self.spawned.keys()
    }

    pub fn is_spawned(&self, id: &str) -> bool {
        self.spawned.contains_key(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hamr_rpc::PluginManifest;

    fn make_discovered(id: &str, command: &str) -> DiscoveredPlugin {
        DiscoveredPlugin {
            id: id.to_string(),
            manifest: PluginManifest {
                id: id.to_string(),
                name: id.to_string(),
                description: None,
                icon: None,
                prefix: None,
                priority: 0,
            },
            is_socket: true,
            spawn_command: Some(command.to_string()),
            is_background: true,
        }
    }

    #[test]
    fn test_spawn_config_default() {
        let config = SpawnConfig::default();
        assert_eq!(config.max_restarts, 5);
        assert_eq!(config.restart_delay, Duration::from_secs(1));
    }

    #[test]
    fn test_spawn_missing_command() {
        let mut spawner = PluginSpawner::new();
        let plugin = DiscoveredPlugin {
            id: "test".to_string(),
            manifest: PluginManifest {
                id: "test".to_string(),
                name: "test".to_string(),
                description: None,
                icon: None,
                prefix: None,
                priority: 0,
            },
            is_socket: true,
            spawn_command: None,
            is_background: true,
        };

        let result = spawner.spawn(&plugin);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no spawn command"));
    }

    #[test]
    fn test_spawn_empty_command() {
        let mut spawner = PluginSpawner::new();
        let plugin = make_discovered("test", "");

        let result = spawner.spawn(&plugin);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty spawn command"));
    }

    #[tokio::test]
    async fn test_spawn_valid_command() {
        let mut spawner = PluginSpawner::new();
        // Use 'sleep' which exists on all Unix systems
        let plugin = make_discovered("test", "sleep 10");

        let result = spawner.spawn(&plugin);
        assert!(result.is_ok());
        assert!(spawner.is_spawned("test"));

        // Cleanup
        spawner.stop_all().await;
    }

    #[tokio::test]
    async fn test_stop_all() {
        let mut spawner = PluginSpawner::new();
        let plugin = make_discovered("test", "sleep 10");

        spawner.spawn(&plugin).unwrap();
        assert!(spawner.is_spawned("test"));

        spawner.stop_all().await;
        assert!(!spawner.is_spawned("test"));
    }

    #[test]
    fn test_spawned_ids() {
        let spawner = PluginSpawner::new();
        let ids: Vec<_> = spawner.spawned_ids().collect();
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn test_stop_plugin() {
        let mut spawner = PluginSpawner::new();
        let plugin = make_discovered("test", "sleep 10");

        spawner.spawn(&plugin).unwrap();
        assert!(spawner.is_spawned("test"));

        // Stop specific plugin
        let stopped = spawner.stop_plugin("test").await;
        assert!(stopped);
        assert!(!spawner.is_spawned("test"));

        // Stop non-existent plugin returns false
        let stopped_again = spawner.stop_plugin("test").await;
        assert!(!stopped_again);
    }
}
