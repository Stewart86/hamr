//! RPC helper functions for clients.
//!
//! Common utilities for converting between RPC messages and core types.

use crate::client::{ClientError, RpcClient};
use hamr_types::{CoreEvent, CoreUpdate};

/// Convert a notification method + params into a `CoreUpdate`.
///
/// Injects the "type" field required for tagged enum deserialization.
/// Returns `None` if params is an invalid type (not object or null).
/// In debug builds, panics on deserialization failure to catch protocol mismatches.
///
/// # Panics
///
/// Panics in debug builds if deserialization fails (protocol mismatch).
pub fn notification_to_update(
    method: &str,
    params: Option<serde_json::Value>,
) -> Option<CoreUpdate> {
    use serde_json::json;

    let mut obj = match params {
        Some(serde_json::Value::Object(m)) => m,
        Some(other) => {
            tracing::error!(
                "Invalid params type for '{}': expected object or null, got {:?}",
                method,
                other
            );
            return None;
        }
        None => serde_json::Map::new(),
    };

    obj.insert("type".to_string(), json!(method));

    match serde_json::from_value::<CoreUpdate>(serde_json::Value::Object(obj)) {
        Ok(update) => Some(update),
        Err(e) => {
            #[cfg(debug_assertions)]
            panic!("Protocol mismatch - failed to deserialize '{method}':\n  Error: {e}");

            #[cfg(not(debug_assertions))]
            {
                tracing::error!("Failed to deserialize '{}': {}", method, e);
                None
            }
        }
    }
}

/// Send a `CoreEvent` over RPC as a notification.
///
/// Extracts the "type" field as the method name and remaining fields as params.
///
/// # Errors
///
/// Returns an error if serialization fails or the RPC notification cannot be sent.
pub async fn send_event(client: &RpcClient, event: CoreEvent) -> Result<(), ClientError> {
    let event_json = serde_json::to_value(event)?;

    let serde_json::Value::Object(mut params) = event_json else {
        return Err(ClientError::SerializationInvariant(
            "CoreEvent did not serialize to a JSON object".to_string(),
        ));
    };

    let Some(serde_json::Value::String(method)) = params.remove("type") else {
        return Err(ClientError::SerializationInvariant(
            "CoreEvent missing 'type' field after serialization".to_string(),
        ));
    };

    let params_value = if params.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(params))
    };

    tracing::debug!(
        "Sending event: method={}, params={:?}",
        method,
        params_value
    );
    client.notify(&method, params_value).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_notification_deserializes() {
        let params = json!({
            "results": []
        });
        let result = notification_to_update("results", Some(params));
        assert!(result.is_some());
    }

    #[test]
    fn test_unknown_method_returns_none_or_panics() {
        let result =
            std::panic::catch_unwind(|| notification_to_update("unknown_method_xyz", None));

        #[cfg(debug_assertions)]
        assert!(result.is_err(), "Should panic in debug builds");

        #[cfg(not(debug_assertions))]
        assert!(
            result.unwrap().is_none(),
            "Should return None in release builds"
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "Protocol mismatch")]
    fn test_invalid_params_panics_in_debug() {
        let params = json!({
            "results": "not an array"
        });
        notification_to_update("results", Some(params));
    }

    #[test]
    fn test_null_params_for_simple_updates() {
        let result = notification_to_update("close", None);
        assert!(result.is_some());
    }

    #[test]
    fn test_invalid_params_type_returns_none() {
        let result = notification_to_update("results", Some(json!(["not", "an", "object"])));
        assert!(result.is_none());
    }
}
