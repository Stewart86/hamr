//! Hamr GTK4 UI - Desktop launcher with layer-shell support
//!
//! This is the GTK4 frontend for Hamr, designed for Wayland compositors
//! that support the wlr-layer-shell protocol (Hyprland, Niri, Sway, KDE Plasma).

mod click_catcher;
mod colors;
mod compositor;
mod config;
mod fab_window;
mod keybindings;
mod niri_ipc;
mod preview_window;
mod rpc;
mod state;
mod styles;
mod thumbnail_cache;
mod widgets;
mod window;

use gtk4::glib;
use gtk4::prelude::*;
use tracing::{debug, error, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::compositor::Compositor;
use crate::window::LauncherWindow;

const APP_ID: &str = "org.hamr.Launcher";
const DEV_APP_ID: &str = "org.hamr.Launcher.Dev";

fn is_dev_mode() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.ends_with("target/debug")))
        .unwrap_or(false)
}

fn setup_logging() {
    #[cfg(debug_assertions)]
    {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let log_filename = format!("hamr-gtk-{timestamp}.log");
        let log_path = std::path::Path::new("/tmp").join(&log_filename);

        let symlink_path = std::path::Path::new("/tmp/hamr-gtk.log");
        let _ = std::fs::remove_file(symlink_path);
        let _ = std::os::unix::fs::symlink(&log_path, symlink_path);

        let file_appender = tracing_appender::rolling::never("/tmp", &log_filename);
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false)
                    .with_target(true)
                    .with_line_number(true),
            )
            .with(EnvFilter::from_default_env().add_directive("hamr_gtk=debug".parse().unwrap()))
            .init();

        std::mem::forget(guard);
    }

    #[cfg(not(debug_assertions))]
    {
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(EnvFilter::from_default_env().add_directive("hamr_gtk=info".parse().unwrap()))
            .init();
    }
}

fn main() -> glib::ExitCode {
    setup_logging();

    info!("Starting hamr-gtk");

    let compositor = Compositor::detect();
    if !compositor.supports_layer_shell() {
        error!(
            "Layer shell not supported on this compositor. hamr-gtk requires a wlr-layer-shell compatible compositor (Hyprland, Niri, Sway, etc.)"
        );
        return glib::ExitCode::FAILURE;
    }

    let app_id = if is_dev_mode() { DEV_APP_ID } else { APP_ID };
    let app = gtk4::Application::builder().application_id(app_id).build();

    app.connect_activate(move |app| {
        debug!("Application activated");
        let window = LauncherWindow::new(app);
        window.run();
    });

    app.run()
}
