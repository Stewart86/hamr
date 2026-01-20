//! Configuration file watcher for hot-reload support.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::mpsc;
use std::time::Duration;

use notify::Watcher;
use tokio::sync::mpsc as tokio_mpsc;
use tracing::{debug, error, info};

use crate::error::Result;

pub fn spawn_config_watcher(config_path: PathBuf, tx: tokio_mpsc::UnboundedSender<()>) {
    let (sync_tx, sync_rx) = mpsc::channel::<()>();

    std::thread::spawn(move || {
        if let Err(e) = watch_config_file(&config_path, &sync_tx) {
            error!("Config watcher error: {e}");
        }
    });

    std::thread::spawn(move || {
        loop {
            if let Ok(()) = sync_rx.recv() {
                debug!("Config file changed, sending reload notification");
                std::thread::sleep(Duration::from_millis(100));
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
}

fn watch_config_file(config_path: &PathBuf, tx: &mpsc::Sender<()>) -> Result<()> {
    let debounce = Arc::new(StdMutex::new(std::time::Instant::now()));
    let debounce_duration = Duration::from_millis(500);

    let config_path_for_closure = config_path.to_owned();

    let (watcher_tx, watcher_rx) = mpsc::channel();

    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<notify::Event>| match result {
            Ok(event) => match event.kind {
                notify::EventKind::Modify(_) | notify::EventKind::Create(_) => {
                    if event.paths.iter().any(|p| {
                        p.file_name() == config_path_for_closure.file_name()
                            || p.ends_with("config.json")
                    }) {
                        let mut last_event = debounce.lock().unwrap();
                        let now = std::time::Instant::now();
                        if now.duration_since(*last_event) > debounce_duration {
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
