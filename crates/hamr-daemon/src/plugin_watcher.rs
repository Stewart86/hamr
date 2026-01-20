//! Plugin directory watcher for hot-reload support.

use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex, mpsc};
use std::time::{Duration, Instant};

use notify::{EventKind, RecursiveMode, Watcher};
use tokio::sync::mpsc as tokio_mpsc;
use tracing::{debug, error, info};

use crate::error::Result;

pub struct PluginWatcher {
    _watcher_thread: std::thread::JoinHandle<()>,
}

impl PluginWatcher {
    pub fn spawn(plugin_dirs: Vec<PathBuf>, tx: tokio_mpsc::UnboundedSender<()>) -> Self {
        let (sync_tx, sync_rx) = mpsc::channel::<()>();

        let watcher_thread = std::thread::spawn(move || {
            if let Err(e) = watch_plugin_dirs(plugin_dirs, &sync_tx) {
                error!("Plugin watcher error: {}", e);
            }
        });

        std::thread::spawn(move || {
            for () in sync_rx {
                debug!("Plugin change detected, sending reload notification");
                if tx.send(()).is_err() {
                    break;
                }
            }
        });

        Self {
            _watcher_thread: watcher_thread,
        }
    }
}

fn watch_plugin_dirs(dirs: Vec<PathBuf>, tx: &mpsc::Sender<()>) -> Result<()> {
    let debounce = Arc::new(StdMutex::new(Instant::now()));
    let debounce_duration = Duration::from_millis(500);

    let (watcher_tx, watcher_rx) = mpsc::channel();

    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<notify::Event>| match result {
            Ok(event) => match event.kind {
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                    let mut last = debounce.lock().unwrap();
                    let now = Instant::now();
                    if now.duration_since(*last) > debounce_duration {
                        *last = now;
                        let _ = watcher_tx.send(());
                    }
                }
                _ => {}
            },
            Err(e) => error!("Watcher error: {}", e),
        })?;

    for dir in dirs {
        watcher.watch(&dir, RecursiveMode::Recursive)?;
        info!("Watching plugin directory: {:?}", dir);
    }

    while watcher_rx.recv().is_ok() {
        let _ = tx.send(());
    }

    Ok(())
}
