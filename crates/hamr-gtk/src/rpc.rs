//! RPC client bridge for GTK.
//!
//! Bridges the tokio-based `RpcClient` with GTK's `GLib` main loop using channels.

use hamr_rpc::{
    ClientRole, CoreEvent, CoreUpdate, Message, RpcClient, notification_to_update, send_event,
};
use std::time::Duration;
use tracing::{error, info, warn};

/// RPC connection handle for GTK.
///
/// Provides channels for sending events and receiving updates,
/// bridging the tokio RPC client with GTK's main loop.
pub struct RpcHandle {
    /// Channel to send `CoreEvents` to the RPC task
    event_tx: async_channel::Sender<CoreEvent>,
    /// Channel to receive `CoreUpdates` from the RPC task
    update_rx: async_channel::Receiver<CoreUpdate>,
}

impl RpcHandle {
    /// Connect to the daemon and start the RPC task.
    ///
    /// Returns a handle for sending/receiving.
    pub fn connect() -> Self {
        let (event_tx, event_rx) = async_channel::bounded::<CoreEvent>(32);
        let (update_tx, update_rx) = async_channel::bounded::<CoreUpdate>(64);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async move {
                if let Err(e) = rpc_task(event_rx, update_tx).await {
                    error!("RPC task error: {e}");
                }
            });
        });

        Self {
            event_tx,
            update_rx,
        }
    }

    /// Get the update receiver for use with `glib::spawn_future_local`.
    pub fn update_receiver(&self) -> async_channel::Receiver<CoreUpdate> {
        self.update_rx.clone()
    }

    /// Get the event sender for cloning to handlers.
    pub fn event_sender(&self) -> async_channel::Sender<CoreEvent> {
        self.event_tx.clone()
    }
}

/// Background RPC task that runs in a tokio runtime.
async fn rpc_task(
    event_rx: async_channel::Receiver<CoreEvent>,
    update_tx: async_channel::Sender<CoreUpdate>,
) -> anyhow::Result<()> {
    let mut daemon_error_sent = false;
    loop {
        let mut client = match RpcClient::connect().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to connect to hamr daemon: {}", e);
                error!("Make sure hamr-daemon is running: hamr-daemon &");
                if !daemon_error_sent {
                    let _ = update_tx
                        .send(CoreUpdate::Error {
                            message: format!("Failed to connect to daemon: {e}"),
                        })
                        .await;
                    daemon_error_sent = true;
                }
                if !wait_before_reconnect(&event_rx).await {
                    break;
                }
                continue;
            }
        };

        info!("Connected to hamr daemon");

        if let Err(e) = client
            .register(ClientRole::Ui {
                name: "hamr-gtk".to_string(),
            })
            .await
        {
            error!("Failed to register with daemon: {}", e);
            if !daemon_error_sent {
                let _ = update_tx
                    .send(CoreUpdate::Error {
                        message: format!("Failed to register: {e}"),
                    })
                    .await;
                daemon_error_sent = true;
            }
            if !wait_before_reconnect(&event_rx).await {
                break;
            }
            continue;
        }

        info!("Registered with daemon as hamr-gtk");
        daemon_error_sent = false;

        match rpc_session(&mut client, &event_rx, &update_tx).await? {
            SessionOutcome::Reconnect => {
                if !daemon_error_sent {
                    let _ = update_tx
                        .send(CoreUpdate::Error {
                            message: "Daemon disconnected. Restart hamr-daemon if needed."
                                .to_string(),
                        })
                        .await;
                    daemon_error_sent = true;
                }
                if !wait_before_reconnect(&event_rx).await {
                    break;
                }
            }
            SessionOutcome::Shutdown => break,
        }
    }

    Ok(())
}

enum SessionOutcome {
    Reconnect,
    Shutdown,
}

async fn rpc_session(
    client: &mut RpcClient,
    event_rx: &async_channel::Receiver<CoreEvent>,
    update_tx: &async_channel::Sender<CoreUpdate>,
) -> anyhow::Result<SessionOutcome> {
    loop {
        tokio::select! {
            event = event_rx.recv() => {
                if let Ok(event) = event {
                    if let Err(e) = send_event(client, event).await {
                        error!("Failed to send event: {}", e);
                    }
                } else {
                    info!("Event channel closed, shutting down RPC task");
                    return Ok(SessionOutcome::Shutdown);
                }
            }

            msg = client.recv() => {
                let Some(msg) = msg else {
                    info!("Daemon connection closed");
                    return Ok(SessionOutcome::Reconnect);
                };

                let (method, params) = match &msg {
                    Message::Notification(notif) => (notif.method.as_str(), notif.params.clone()),
                    Message::Request(req) if req.id.is_none() => (req.method.as_str(), req.params.clone()),
                    _ => continue,
                };

                if let Some(update) = notification_to_update(method, params) {
                    if update_tx.send(update).await.is_err() {
                        info!("Update channel closed, shutting down RPC task");
                        return Ok(SessionOutcome::Shutdown);
                    }
                } else {
                    warn!("Failed to parse notification: method={}", method);
                }
            }
        }
    }
}

async fn wait_before_reconnect(event_rx: &async_channel::Receiver<CoreEvent>) -> bool {
    let sleep = tokio::time::sleep(Duration::from_secs(1));
    tokio::pin!(sleep);

    loop {
        tokio::select! {
            () = &mut sleep => {
                return true;
            }

            event = event_rx.recv() => {
                if event.is_err() {
                    info!("Event channel closed while waiting to reconnect");
                    return false;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_notification_to_update_results() {
        let params = json!({
            "results": []
        });
        let result = notification_to_update("results", Some(params));
        assert!(result.is_some());
    }

    #[test]
    fn test_notification_to_update_close() {
        let result = notification_to_update("close", None);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), CoreUpdate::Close));
    }

    #[test]
    fn test_notification_to_update_invalid_params_type() {
        let result = notification_to_update("results", Some(json!(["not", "an", "object"])));
        assert!(result.is_none());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "Protocol mismatch")]
    fn test_notification_to_update_invalid_schema_panics() {
        notification_to_update("results", Some(json!({"results": "not an array"})));
    }
}
