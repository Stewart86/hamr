//! Configuration file watcher for hot-reload support.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::mpsc;
use std::time::Duration;

use notify::Watcher;
use tokio::sync::mpsc as tokio_mpsc;
use tracing::{debug, error, info};

use crate::error::Result;

const RELOAD_SETTLE_DELAY: Duration = Duration::from_millis(100);
const CONFIG_DEBOUNCE_DURATION: Duration = Duration::from_millis(500);
const CONFIG_FILENAME: &str = "config.json";

pub struct ConfigWatcher {
    _watcher_thread: std::thread::JoinHandle<()>,
    _bridge_thread: std::thread::JoinHandle<()>,
}

pub fn spawn_config_watcher(
    config_path: PathBuf,
    tx: tokio_mpsc::UnboundedSender<()>,
) -> ConfigWatcher {
    let (sync_tx, sync_rx) = mpsc::channel::<()>();

    let watcher_thread = std::thread::spawn(move || {
        if let Err(e) = watch_config_file(&config_path, &sync_tx) {
            error!("Config watcher error: {e}");
        }
    });

    let bridge_thread = std::thread::spawn(move || {
        loop {
            if let Ok(()) = sync_rx.recv() {
                debug!("Config file changed, sending reload notification");
                std::thread::sleep(RELOAD_SETTLE_DELAY);
                if tx.send(()).is_err() {
                    debug!("Config reload receiver dropped, stopping watcher");
                    break;
                }
            } else {
                debug!("Config watcher channel closed");
                break;
            }
        }
    });

    ConfigWatcher {
        _watcher_thread: watcher_thread,
        _bridge_thread: bridge_thread,
    }
}

fn watch_config_file(config_path: &Path, tx: &mpsc::Sender<()>) -> Result<()> {
    let debounce = Arc::new(StdMutex::new(std::time::Instant::now()));

    let config_path_for_closure = config_path.to_owned();

    let (watcher_tx, watcher_rx) = mpsc::channel();

    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<notify::Event>| match result {
            Ok(event) => match event.kind {
                notify::EventKind::Modify(_) | notify::EventKind::Create(_) => {
                    if event.paths.iter().any(|p| {
                        p.file_name() == config_path_for_closure.file_name()
                            || p.ends_with(CONFIG_FILENAME)
                    }) {
                        let Ok(mut last_event) = debounce.lock() else {
                            error!("[config_watcher] Debounce mutex poisoned, skipping event");
                            return;
                        };
                        let now = std::time::Instant::now();
                        if now.duration_since(*last_event) > CONFIG_DEBOUNCE_DURATION {
                            *last_event = now;
                            let _ = watcher_tx.send(());
                        }
                    }
                }
                _ => {}
            },
            Err(e) => {
                error!("Watcher error: {}", e);
            }
        })?;

    if let Some(parent) = config_path.parent() {
        watcher.watch(parent, notify::RecursiveMode::NonRecursive)?;
        info!("Watching config directory: {:?}", parent);
    } else {
        return Err(crate::error::DaemonError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid config path",
        )));
    }

    while watcher_rx.recv().is_ok() {
        let _ = tx.send(());
    }

    Ok(())
}
