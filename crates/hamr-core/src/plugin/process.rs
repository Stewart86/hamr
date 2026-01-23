use super::protocol::{PluginInput, PluginResponse};
use crate::{Error, Result};
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

/// A running plugin process with split send/receive
pub struct PluginProcess {
    child: Child,
    sender: PluginSender,
    receiver: Option<PluginReceiver>,
    plugin_id: String,
}

/// Sender half - can be cloned and used independently
#[derive(Clone)]
pub struct PluginSender {
    stdin_tx: mpsc::Sender<String>,
    stdin_close_signal: Arc<AtomicBool>,
    plugin_id: String,
}

/// Receiver half - owns the response channel
pub struct PluginReceiver {
    response_rx: mpsc::Receiver<PluginResponse>,
    plugin_id: String,
}

impl PluginProcess {
    /// Spawn a new plugin process.
    ///
    /// # Errors
    ///
    /// Returns an error if the process fails to spawn or I/O setup fails.
    #[allow(clippy::too_many_lines)]
    pub fn spawn(plugin_id: &str, handler_path: &Path, working_dir: &Path) -> Result<Self> {
        let mut child = Command::new(handler_path)
            .current_dir(working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                Error::Process(format!("Failed to spawn {}: {}", handler_path.display(), e))
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Process("Failed to get stdin handle".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Process("Failed to get stdout handle".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::Process("Failed to get stderr handle".to_string()))?;

        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(32);
        let (response_tx, response_rx) = mpsc::channel::<PluginResponse>(32);
        let stdin_close_signal = Arc::new(AtomicBool::new(false));

        let pid = plugin_id.to_string();

        let mut stdin_writer = stdin;
        let close_signal = stdin_close_signal.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    line = stdin_rx.recv() => {
                        match line {
                            Some(line) => {
                                if let Err(e) = stdin_writer.write_all(line.as_bytes()).await {
                                    error!("Failed to write to plugin stdin: {}", e);
                                    break;
                                }
                                if let Err(e) = stdin_writer.flush().await {
                                    error!("Failed to flush plugin stdin: {}", e);
                                    break;
                                }
                                if close_signal.load(Ordering::SeqCst) {
                                    debug!("Closing plugin stdin after write");
                                    drop(stdin_writer);
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
        });

        let plugin_id_clone = pid.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<PluginResponse>(&line) {
                    Ok(response) => {
                        if response_tx.send(response).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let error_msg =
                            format!("Plugin '{plugin_id_clone}' returned invalid JSON: {e}");
                        warn!("{error_msg} - Raw: {line}");
                        if response_tx
                            .send(PluginResponse::Error {
                                message: error_msg,
                                details: Some(line),
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        });

        let plugin_id_clone = pid.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                warn!("[{}] stderr: {}", plugin_id_clone, line);
            }
        });

        let sender = PluginSender {
            stdin_tx,
            stdin_close_signal,
            plugin_id: pid.clone(),
        };

        let receiver = PluginReceiver {
            response_rx,
            plugin_id: pid.clone(),
        };

        Ok(Self {
            child,
            sender,
            receiver: Some(receiver),
            plugin_id: pid,
        })
    }

    /// Get a clone of the sender (can send without locking)
    #[must_use]
    pub fn sender(&self) -> PluginSender {
        self.sender.clone()
    }

    /// Take the receiver (for spawning a listener task)
    pub fn take_receiver(&mut self) -> Option<PluginReceiver> {
        self.receiver.take()
    }

    /// Send input to the plugin (convenience method).
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or channel send fails.
    pub async fn send(&self, input: &PluginInput) -> Result<()> {
        self.sender.send(input).await
    }

    /// Send input and signal to close stdin afterwards.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or channel send fails.
    pub async fn send_and_close(&self, input: &PluginInput) -> Result<()> {
        self.sender.send_and_close(input).await
    }

    /// Kill the process.
    ///
    /// # Errors
    ///
    /// Returns an error if the process cannot be killed.
    pub async fn kill(&mut self) -> Result<()> {
        self.child
            .kill()
            .await
            .map_err(|e| Error::Process(format!("Failed to kill plugin: {e}")))
    }

    /// Get the plugin ID
    #[must_use]
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
}

impl PluginSender {
    /// Send input to the plugin.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or channel send fails.
    pub async fn send(&self, input: &PluginInput) -> Result<()> {
        let json = serde_json::to_string(input)? + "\n";
        debug!("[{}] Sending: {}", self.plugin_id, json.trim());
        self.stdin_tx
            .send(json)
            .await
            .map_err(|e| Error::Process(format!("Failed to send to plugin: {e}")))
    }

    /// Send input and signal to close stdin afterwards.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or channel send fails.
    pub async fn send_and_close(&self, input: &PluginInput) -> Result<()> {
        let json = serde_json::to_string(input)? + "\n";
        debug!(
            "[{}] Sending (then closing stdin): {}",
            self.plugin_id,
            json.trim()
        );

        self.stdin_close_signal.store(true, Ordering::SeqCst);

        self.stdin_tx
            .send(json)
            .await
            .map_err(|e| Error::Process(format!("Failed to send to plugin: {e}")))
    }
}

impl PluginReceiver {
    /// Receive the next response from the plugin
    pub async fn recv(&mut self) -> Option<PluginResponse> {
        self.response_rx.recv().await
    }

    /// Get the plugin ID
    #[must_use]
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
}

impl Drop for PluginProcess {
    fn drop(&mut self) {
        debug!("[{}] Plugin process dropped", self.plugin_id);
    }
}
