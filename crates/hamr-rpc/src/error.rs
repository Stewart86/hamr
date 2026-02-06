//! Error types for the hamr-rpc crate.

use crate::client::ClientError;

pub type Result<T> = std::result::Result<T, ClientError>;
