//! Shared JSON-RPC 2.0 protocol definitions for hamr.
//!
//! This crate provides the protocol types, transport codec, and client helper
//! for communication between hamr components over Unix sockets.
//!
//! # Architecture
//!
//! The crate is organized into the following modules:
//!
//! - [`types`]: Shared data types (`SearchResult`, `CardData`, `FormData`, etc.)
//! - [`protocol`]: JSON-RPC 2.0 message types (Request, Response, Notification)
//! - [`transport`]: Length-prefixed codec for message framing
//! - [`client`]: RPC client helper for connecting to the daemon
//! - [`error`]: Result type alias
//!
//! # Example
//!
//! ```no_run
//! use hamr_rpc::{RpcClient, ClientRole};
//!
//! # async fn example() -> Result<(), hamr_rpc::ClientError> {
//! // Connect to the daemon
//! let mut client = RpcClient::connect().await?;
//!
//! // Register as a UI client
//! let session_id = client.register(ClientRole::Ui {
//!     name: "my-ui".to_string(),
//! }).await?;
//!
//! println!("Registered with session: {}", session_id);
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod error;
pub mod helpers;
pub mod protocol;
pub mod transport;

// Re-export main client types
pub use client::{ClientError, RpcClient, dev_socket_path, socket_path};

// Re-export helper functions
pub use helpers::{notification_to_update, send_event};

// Re-export error types
pub use error::Result;

// Re-export protocol types
pub use protocol::{
    ALREADY_REGISTERED, CONTROL_REQUIRED, ClientRole, INTERNAL_ERROR, INVALID_PARAMS,
    INVALID_REQUEST, JSONRPC_VERSION, METHOD_NOT_FOUND, Message, NOT_ACTIVE_UI, NOT_REGISTERED,
    Notification, PARSE_ERROR, PLUGIN_NOT_FOUND, RegisterParams, RegisterResult, Request,
    RequestId, Response, RpcError, UI_OCCUPIED,
};

// Re-export transport types
pub use transport::{CodecError, JsonRpcCodec};

// Re-export commonly used data types from hamr-types
pub use hamr_types::{
    Action, ActionSource, AmbientItem, Badge, CardBlock, CardData, Chip, CoreEvent, CoreUpdate,
    ExecuteAction, FabOverride, FormData, FormField, FormFieldType, FormOption, GaugeData,
    GraphData, GridBrowserData, GridItem, IconSpec, ImageBrowserData, ImageItem, MetadataItem,
    PluginAction, PluginManifest, PluginStatus, PreviewData, ProgressData, ResultPatch, ResultType,
    SearchResult, SliderValue, WidgetData,
};
