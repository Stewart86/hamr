//! Hamr daemon entry point.
//!
//! This binary starts the hamr daemon socket server that handles
//! communication with UI clients, control commands, and plugins.

use std::path::PathBuf;

use clap::Parser;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod config_watcher;
mod error;
mod handlers;
mod plugin_rpc;
mod plugin_spawner;
mod plugin_watcher;
mod registry;
mod server;
mod session;

/// Hamr daemon - socket server for the hamr launcher
#[derive(Parser, Debug)]
#[command(name = "hamr-daemon")]
#[command(version, about, long_about = None)]
struct Args {
    /// Custom socket path (defaults to `$XDG_RUNTIME_DIR/hamr.sock` or `/tmp/hamr.sock`)
    #[arg(long, value_name = "PATH")]
    socket_path: Option<PathBuf>,
}

/// Set up logging with file output for debugging.
/// In debug builds, defaults to debug level and logs to timestamped file.
/// In release builds, defaults to info level and logs to stderr.
fn setup_logging() {
    let default_level = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("hamr={default_level}")));

    if cfg!(debug_assertions) {
        let temp_dir = std::env::temp_dir();
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let log_filename = format!("hamr-daemon-{timestamp}.log");
        let log_path = temp_dir.join(&log_filename);

        #[cfg(unix)]
        {
            let symlink_path = temp_dir.join("hamr-daemon.log");
            let _ = std::fs::remove_file(&symlink_path);
            let _ = std::os::unix::fs::symlink(&log_path, &symlink_path);
        }

        let file_appender = tracing_appender::rolling::never(&temp_dir, &log_filename);
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        std::mem::forget(guard);

        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true)
            .with_line_number(true);

        let stderr_layer = fmt::layer()
            .with_writer(std::io::stderr)
            .with_ansi(true)
            .with_target(true)
            .with_line_number(true);

        tracing_subscriber::registry()
            .with(file_layer)
            .with(stderr_layer)
            .with(filter)
            .init();

        eprintln!("Logging to: {} (and stderr)", log_path.display());
    } else {
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(filter)
            .init();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    setup_logging();

    info!("Starting hamr daemon...");

    server::run(args.socket_path).await?;

    info!("Hamr daemon stopped");
    Ok(())
}
