//! Utility functions and helpers for the A2A SDK.

#![allow(dead_code)]

use std::collections::HashMap;
use uuid::Uuid;

/// Generates a new UUID v4 string.
pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

/// Generates a current timestamp in ISO 8601 format.
pub fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Encodes bytes to base64.
pub fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// Decodes base64 string to bytes.
pub fn base64_decode(data: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(data)
}

/// Constant-time comparison to prevent timing attacks.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Extracts query parameters from a URL (simple implementation).
pub fn parse_query_params(url: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    if let Some(query_start) = url.find('?') {
        let query = &url[query_start + 1..];
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(key.to_string(), value.to_string());
            }
        }
    }
    params
}

/// Builds a URL with query parameters.
pub fn build_url_with_params(base: &str, params: &HashMap<String, String>) -> String {
    if params.is_empty() {
        return base.to_string();
    }
    let query: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    format!("{}?{}", base.trim_end_matches('?'), query.join("&"))
}

/// Tracing span helper for A2A operations.
#[cfg(feature = "telemetry")]
pub mod telemetry {
    use tracing::{Span, info_span};

    /// Creates a span for a send_message operation.
    pub fn send_message_span(task_id: &str, context_id: &str) -> Span {
        info_span!(
            "a2a.send_message",
            task_id = %task_id,
            context_id = %context_id,
            otel.kind = "client"
        )
    }

    /// Creates a span for a get_task operation.
    pub fn get_task_span(task_id: &str) -> Span {
        info_span!(
            "a2a.get_task",
            task_id = %task_id,
            otel.kind = "client"
        )
    }

    /// Creates a span for agent execution.
    pub fn execute_span(task_id: &str, context_id: &str) -> Span {
        info_span!(
            "a2a.execute",
            task_id = %task_id,
            context_id = %context_id,
            otel.kind = "server"
        )
    }

    /// Creates a span for handling a request.
    pub fn handle_request_span(method: &str) -> Span {
        info_span!(
            "a2a.handle_request",
            method = %method,
            otel.kind = "server"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID v4 format
    }

    #[test]
    fn test_base64_roundtrip() {
        let original = b"Hello, World!";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(original.to_vec(), decoded);
    }
}
