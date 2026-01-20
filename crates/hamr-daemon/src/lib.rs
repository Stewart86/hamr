//! Hamr daemon library providing socket server and client management.
//!
//! This crate provides the socket server that handles communication between
//! the hamr launcher core and various clients (UI, control, plugins).

pub(crate) mod config_watcher;
pub mod error;
pub(crate) mod handlers;
pub mod plugin_rpc;
pub(crate) mod plugin_spawner;
pub(crate) mod plugin_watcher;
pub(crate) mod registry;
pub mod server;
pub(crate) mod session;

pub use error::{DaemonError, Result};
pub use plugin_rpc::{
    send_action, send_form_submitted, send_initial, send_search, send_slider_changed,
    send_switch_toggled,
};
pub use registry::{ConnectedPlugin, DiscoveredPlugin, PluginRegistry};
pub use server::{DaemonState, run};
pub use session::{ClientInfo, ControlSession, PluginSession, Session, SessionId, UiSession};
