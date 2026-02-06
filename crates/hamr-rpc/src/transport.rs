//! Length-prefixed transport codec for JSON-RPC messages.
//!
//! This module provides a codec that frames JSON-RPC messages with a 4-byte
//! big-endian length prefix for reliable message delimitation over stream sockets.
//!
//! Frame format:
//! ```text
//! +----------------+------------------+
//! |  4 bytes       |  N bytes         |
//! |  (length BE)   |  (JSON payload)  |
//! +----------------+------------------+
//! ```

use bytes::{Buf, BufMut, BytesMut};
use std::io;
use tokio_util::codec::{Decoder, Encoder};

use crate::protocol::Message;

/// Maximum message size (16 MB)
const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Length prefix size in bytes
const LENGTH_PREFIX_SIZE: usize = 4;

/// Codec for length-prefixed JSON-RPC messages
#[derive(Debug, Default)]
pub struct JsonRpcCodec {
    current_length: Option<usize>,
}

impl JsonRpcCodec {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Decoder for JsonRpcCodec {
    type Item = Message;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if self.current_length.is_none() {
            if src.len() < LENGTH_PREFIX_SIZE {
                return Ok(None);
            }

            let len = src.get_u32() as usize;

            if len > MAX_MESSAGE_SIZE {
                return Err(CodecError::MessageTooLarge(len));
            }

            self.current_length = Some(len);
        }

        let Some(length) = self.current_length else {
            return Ok(None);
        };

        if src.len() < length {
            src.reserve(length - src.len());
            return Ok(None);
        }

        let payload = src.split_to(length);
        self.current_length = None;

        let json_str = std::str::from_utf8(&payload)?;
        let message: Message = serde_json::from_str(json_str)?;

        Ok(Some(message))
    }
}

impl Encoder<Message> for JsonRpcCodec {
    type Error = CodecError;

    // Message size is checked against MAX_MESSAGE_SIZE (fits in u32)
    #[allow(clippy::cast_possible_truncation)]
    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let json = serde_json::to_string(&item)?;
        let json_bytes = json.as_bytes();

        if json_bytes.len() > MAX_MESSAGE_SIZE {
            return Err(CodecError::MessageTooLarge(json_bytes.len()));
        }

        dst.reserve(LENGTH_PREFIX_SIZE + json_bytes.len());
        dst.put_u32(json_bytes.len() as u32);
        dst.put_slice(json_bytes);

        Ok(())
    }
}

/// Errors that can occur during codec operations
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Message too large: {0} bytes (max: {MAX_MESSAGE_SIZE})")]
    MessageTooLarge(usize),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::cast_possible_truncation)] // Test constants bounded to u32

    use super::*;
    use crate::protocol::{Notification, Request, Response, RpcError};

    #[test]
    fn test_encode_decode_roundtrip() {
        let mut codec = JsonRpcCodec::new();
        let mut buf = BytesMut::new();

        let request = Request::new("test", Some(serde_json::json!({"key": "value"})), 1.into());
        let msg = Message::Request(request);

        // Encode
        codec.encode(msg.clone(), &mut buf).unwrap();

        // Decode
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        // Compare
        if let (Message::Request(orig), Message::Request(dec)) = (msg, decoded) {
            assert_eq!(orig.method, dec.method);
            assert_eq!(orig.id, dec.id);
        } else {
            panic!("Message type mismatch");
        }
    }

    #[test]
    fn test_encode_decode_response() {
        let mut codec = JsonRpcCodec::new();
        let mut buf = BytesMut::new();

        let response = Response::success(42.into(), serde_json::json!({"result": "ok"}));
        let msg = Message::Response(response);

        codec.encode(msg.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        if let Message::Response(dec) = decoded {
            assert_eq!(dec.id, crate::protocol::RequestId::Number(42));
            assert!(dec.result.is_some());
        } else {
            panic!("Expected Response");
        }
    }

    #[test]
    fn test_encode_decode_notification() {
        let mut codec = JsonRpcCodec::new();
        let mut buf = BytesMut::new();

        let notification = Notification::new("update", Some(serde_json::json!({"count": 5})));
        let msg = Message::Notification(notification);

        codec.encode(msg, &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        // Note: Due to serde untagged enum, Notification may decode as Request without id
        // The important thing is the method and params are preserved
        match decoded {
            Message::Notification(dec) => {
                assert_eq!(dec.method, "update");
            }
            Message::Request(dec) => {
                // Also acceptable - Request with no id is functionally a notification
                assert_eq!(dec.method, "update");
                assert!(dec.id.is_none());
            }
            Message::Response(_) => panic!("Expected Notification or Request without id"),
        }
    }

    #[test]
    fn test_encode_decode_error_response() {
        let mut codec = JsonRpcCodec::new();
        let mut buf = BytesMut::new();

        let response = Response::error(1.into(), RpcError::method_not_found());
        let msg = Message::Response(response);

        codec.encode(msg, &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        if let Message::Response(dec) = decoded {
            assert!(dec.error.is_some());
            assert_eq!(dec.error.unwrap().code, -32601);
        } else {
            panic!("Expected Response");
        }
    }

    #[test]
    fn test_partial_decode() {
        let mut codec = JsonRpcCodec::new();
        let mut buf = BytesMut::new();

        // Encode a message
        let request = Request::new("test", None, 1.into());
        let msg = Message::Request(request);
        codec.encode(msg, &mut buf).unwrap();

        // Save the full buffer
        let full_buf = buf.clone();

        // Try decoding with only partial data
        let mut partial = BytesMut::new();
        partial.extend_from_slice(&full_buf[..2]); // Only 2 bytes of length prefix

        assert!(codec.decode(&mut partial).unwrap().is_none());

        // Add more data
        partial.extend_from_slice(&full_buf[2..6]); // Rest of prefix + some payload

        assert!(codec.decode(&mut partial).unwrap().is_none());

        // Add remaining data
        partial.extend_from_slice(&full_buf[6..]);

        let decoded = codec.decode(&mut partial).unwrap();
        assert!(decoded.is_some());
    }

    #[test]
    fn test_decode_empty_buffer() {
        let mut codec = JsonRpcCodec::new();
        let mut buf = BytesMut::new();

        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_none(), "Empty buffer should return None");
    }

    #[test]
    fn test_decode_insufficient_length_prefix() {
        let mut codec = JsonRpcCodec::new();
        let mut buf = BytesMut::new();

        // Only 3 bytes (need 4 for length prefix)
        buf.extend_from_slice(&[0, 0, 0]);

        let result = codec.decode(&mut buf).unwrap();
        assert!(
            result.is_none(),
            "Incomplete length prefix should return None"
        );
    }

    #[test]
    fn test_multiple_messages_in_buffer() {
        let mut codec = JsonRpcCodec::new();
        let mut buf = BytesMut::new();

        // Encode two messages
        let msg1 = Message::Request(Request::new("first", None, 1.into()));
        let msg2 = Message::Request(Request::new("second", None, 2.into()));

        codec.encode(msg1, &mut buf).unwrap();
        codec.encode(msg2, &mut buf).unwrap();

        // Decode first
        let decoded1 = codec.decode(&mut buf).unwrap().unwrap();
        if let Message::Request(req) = decoded1 {
            assert_eq!(req.method, "first");
        } else {
            panic!("Expected Request");
        }

        // Decode second
        let decoded2 = codec.decode(&mut buf).unwrap().unwrap();
        if let Message::Request(req) = decoded2 {
            assert_eq!(req.method, "second");
        } else {
            panic!("Expected Request");
        }

        // Buffer should be empty now
        assert!(buf.is_empty());
    }

    #[test]
    fn test_message_too_large() {
        let mut codec = JsonRpcCodec::new();
        let mut buf = BytesMut::new();

        // Write a length prefix that exceeds the limit
        buf.put_u32((MAX_MESSAGE_SIZE + 1) as u32);

        let result = codec.decode(&mut buf);
        assert!(matches!(result, Err(CodecError::MessageTooLarge(_))));
    }

    #[test]
    fn test_invalid_json() {
        let mut codec = JsonRpcCodec::new();
        let mut buf = BytesMut::new();

        // Write length prefix
        let invalid_json = b"not valid json";
        buf.put_u32(invalid_json.len() as u32);
        buf.extend_from_slice(invalid_json);

        let result = codec.decode(&mut buf);
        assert!(matches!(result, Err(CodecError::Json(_))));
    }

    #[test]
    fn test_invalid_utf8() {
        let mut codec = JsonRpcCodec::new();
        let mut buf = BytesMut::new();

        // Write length prefix and invalid UTF-8
        let invalid_utf8 = [0xff, 0xfe, 0x00, 0x01];
        buf.put_u32(invalid_utf8.len() as u32);
        buf.extend_from_slice(&invalid_utf8);

        let result = codec.decode(&mut buf);
        assert!(matches!(result, Err(CodecError::Utf8(_))));
    }

    #[test]
    fn test_codec_error_display() {
        let err = CodecError::MessageTooLarge(20_000_000);
        let msg = err.to_string();
        assert!(msg.contains("20000000"));
        assert!(msg.contains("too large"));
    }

    #[test]
    fn test_codec_error_display_io() {
        let io_err = io::Error::new(io::ErrorKind::ConnectionReset, "connection reset");
        let err = CodecError::Io(io_err);
        let msg = err.to_string();
        assert!(msg.contains("I/O error"));
        assert!(msg.contains("connection reset"));
    }

    #[test]
    fn test_codec_error_display_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid").unwrap_err();
        let err = CodecError::Json(json_err);
        let msg = err.to_string();
        assert!(msg.contains("JSON error"));
    }

    #[test]
    fn test_codec_error_display_utf8() {
        let invalid_utf8 = vec![0xff, 0xfe];
        let utf8_err = std::str::from_utf8(&invalid_utf8).unwrap_err();
        let err = CodecError::Utf8(utf8_err);
        let msg = err.to_string();
        assert!(msg.contains("UTF-8 error"));
    }

    #[test]
    fn test_codec_error_from_io() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let codec_err: CodecError = io_err.into();
        assert!(matches!(codec_err, CodecError::Io(_)));
    }

    #[test]
    fn test_codec_error_from_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let codec_err: CodecError = json_err.into();
        assert!(matches!(codec_err, CodecError::Json(_)));
    }

    #[test]
    fn test_codec_error_from_utf8() {
        let invalid = vec![0x80, 0x81];
        let utf8_err = std::str::from_utf8(&invalid).unwrap_err();
        let codec_err: CodecError = utf8_err.into();
        assert!(matches!(codec_err, CodecError::Utf8(_)));
    }

    #[test]
    fn test_codec_error_debug() {
        let err = CodecError::MessageTooLarge(12345);
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("MessageTooLarge"));
        assert!(debug_str.contains("12345"));
    }

    #[test]
    fn test_length_prefix_format() {
        let mut codec = JsonRpcCodec::new();
        let mut buf = BytesMut::new();

        let request = Request::new("x", None, 1.into());
        let msg = Message::Request(request);

        codec.encode(msg, &mut buf).unwrap();

        // First 4 bytes should be length prefix (big-endian)
        let length = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;

        // Length should match remaining bytes
        assert_eq!(length, buf.len() - 4);
    }
}
