//! OpenTelemetry Tracing Utilities for A2A Rust SDK.
//!
//! This module provides comprehensive tracing utilities for instrumenting A2A operations.
//! It mirrors Python's `a2a/utils/telemetry.py` functionality.
//!
//! # Features
//!
//! - Automatic span creation for A2A operations
//! - Support for both client and server span kinds
//! - Custom span attributes and extractors
//! - Integration with the `tracing` ecosystem
//!
//! # Configuration
//!
//! Enable the `telemetry` feature to use OpenTelemetry integration:
//!
//! ```toml
//! ra2a = { version = "0.1", features = ["telemetry"] }
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use ra2a::telemetry::{A2ASpan, SpanKind};
//!
//! async fn my_operation() {
//!     let span = A2ASpan::new("my_operation", SpanKind::Client)
//!         .with_attribute("task_id", "123")
//!         .start();
//!     
//!     // ... do work ...
//!     
//!     span.end();
//! }
//! ```

use std::collections::HashMap;
use std::time::Instant;

/// The name of the instrumentation module.
pub const INSTRUMENTATION_MODULE_NAME: &str = "a2a-rust-sdk";

/// The version of the instrumentation module.
pub const INSTRUMENTATION_MODULE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Environment variable to enable/disable A2A SDK instrumentation.
pub const ENABLED_ENV_VAR: &str = "OTEL_INSTRUMENTATION_A2A_SDK_ENABLED";

/// Span kind for categorizing spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    /// Client-side operation (making requests).
    Client,
    /// Server-side operation (handling requests).
    Server,
    /// Internal operation within the SDK.
    Internal,
    /// Producer operation (sending messages).
    Producer,
    /// Consumer operation (receiving messages).
    Consumer,
}

impl SpanKind {
    /// Returns the string representation for OpenTelemetry.
    pub fn as_str(&self) -> &'static str {
        match self {
            SpanKind::Client => "client",
            SpanKind::Server => "server",
            SpanKind::Internal => "internal",
            SpanKind::Producer => "producer",
            SpanKind::Consumer => "consumer",
        }
    }
}

impl std::fmt::Display for SpanKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Span status for indicating success or failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    /// Operation completed successfully.
    Ok,
    /// Operation encountered an error.
    Error,
    /// Status is unset (default).
    Unset,
}

/// A builder for creating traced spans.
#[derive(Debug, Clone)]
pub struct A2ASpan {
    name: String,
    kind: SpanKind,
    attributes: HashMap<String, String>,
    parent_context: Option<String>,
}

impl A2ASpan {
    /// Creates a new span builder with the given name and kind.
    pub fn new(name: impl Into<String>, kind: SpanKind) -> Self {
        Self {
            name: name.into(),
            kind,
            attributes: HashMap::new(),
            parent_context: None,
        }
    }

    /// Creates a client span.
    pub fn client(name: impl Into<String>) -> Self {
        Self::new(name, SpanKind::Client)
    }

    /// Creates a server span.
    pub fn server(name: impl Into<String>) -> Self {
        Self::new(name, SpanKind::Server)
    }

    /// Creates an internal span.
    pub fn internal(name: impl Into<String>) -> Self {
        Self::new(name, SpanKind::Internal)
    }

    /// Creates a producer span.
    pub fn producer(name: impl Into<String>) -> Self {
        Self::new(name, SpanKind::Producer)
    }

    /// Creates a consumer span.
    pub fn consumer(name: impl Into<String>) -> Self {
        Self::new(name, SpanKind::Consumer)
    }

    /// Adds an attribute to the span.
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Adds multiple attributes to the span.
    pub fn with_attributes(mut self, attrs: HashMap<String, String>) -> Self {
        self.attributes.extend(attrs);
        self
    }

    /// Sets the parent context for the span.
    pub fn with_parent(mut self, context: impl Into<String>) -> Self {
        self.parent_context = Some(context.into());
        self
    }

    /// Starts the span and returns an active span handle.
    #[cfg(feature = "telemetry")]
    pub fn start(self) -> ActiveSpan {
        use tracing::info_span;

        let span = info_span!(
            target: INSTRUMENTATION_MODULE_NAME,
            "a2a.operation",
            otel.name = %self.name,
            otel.kind = %self.kind,
        );

        for (key, value) in &self.attributes {
            span.record(key.as_str(), value.as_str());
        }

        ActiveSpan {
            name: self.name,
            kind: self.kind,
            start_time: Instant::now(),
            #[cfg(feature = "telemetry")]
            span: Some(span),
            status: SpanStatus::Unset,
            error_message: None,
        }
    }

    /// Starts the span (no-op when telemetry is disabled).
    #[cfg(not(feature = "telemetry"))]
    pub fn start(self) -> ActiveSpan {
        ActiveSpan {
            name: self.name,
            kind: self.kind,
            start_time: Instant::now(),
            status: SpanStatus::Unset,
            error_message: None,
        }
    }
}

/// An active span that is currently being traced.
pub struct ActiveSpan {
    name: String,
    kind: SpanKind,
    start_time: Instant,
    #[cfg(feature = "telemetry")]
    span: Option<tracing::Span>,
    status: SpanStatus,
    error_message: Option<String>,
}

impl ActiveSpan {
    /// Records an attribute on the active span.
    pub fn record_attribute(&mut self, key: &str, value: &str) {
        #[cfg(feature = "telemetry")]
        if let Some(ref span) = self.span {
            span.record(key, value);
        }
        let _ = (key, value); // Suppress unused warnings when telemetry disabled
    }

    /// Records an error on the span.
    pub fn record_error(&mut self, error: &str) {
        self.status = SpanStatus::Error;
        self.error_message = Some(error.to_string());

        #[cfg(feature = "telemetry")]
        if let Some(ref span) = self.span {
            span.record("error", true);
            span.record("error.message", error);
        }
    }

    /// Marks the span as successful.
    pub fn mark_ok(&mut self) {
        self.status = SpanStatus::Ok;
    }

    /// Returns the duration since the span started.
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Ends the span.
    pub fn end(self) {
        let duration = self.start_time.elapsed();
        tracing::debug!(
            target: INSTRUMENTATION_MODULE_NAME,
            name = %self.name,
            kind = %self.kind,
            duration_ms = duration.as_millis(),
            status = ?self.status,
            "Span ended"
        );
    }
}

impl Drop for ActiveSpan {
    fn drop(&mut self) {
        // Span is automatically ended when dropped
    }
}

/// Creates a span for the `message/send` operation.
pub fn send_message_span(task_id: &str, context_id: &str) -> A2ASpan {
    A2ASpan::client("a2a.message.send")
        .with_attribute("a2a.task_id", task_id)
        .with_attribute("a2a.context_id", context_id)
}

/// Creates a span for the `message/stream` operation.
pub fn stream_message_span(task_id: &str, context_id: &str) -> A2ASpan {
    A2ASpan::client("a2a.message.stream")
        .with_attribute("a2a.task_id", task_id)
        .with_attribute("a2a.context_id", context_id)
}

/// Creates a span for the `tasks/get` operation.
pub fn get_task_span(task_id: &str) -> A2ASpan {
    A2ASpan::client("a2a.tasks.get").with_attribute("a2a.task_id", task_id)
}

/// Creates a span for the `tasks/cancel` operation.
pub fn cancel_task_span(task_id: &str) -> A2ASpan {
    A2ASpan::client("a2a.tasks.cancel").with_attribute("a2a.task_id", task_id)
}

/// Creates a span for agent execution.
pub fn execute_span(task_id: &str, context_id: &str) -> A2ASpan {
    A2ASpan::server("a2a.execute")
        .with_attribute("a2a.task_id", task_id)
        .with_attribute("a2a.context_id", context_id)
}

/// Creates a span for handling a request.
pub fn handle_request_span(method: &str) -> A2ASpan {
    A2ASpan::server("a2a.handle_request").with_attribute("a2a.method", method)
}

/// Creates a span for SSE streaming.
pub fn sse_stream_span(task_id: &str) -> A2ASpan {
    A2ASpan::server("a2a.sse.stream").with_attribute("a2a.task_id", task_id)
}

/// Creates a span for push notification sending.
pub fn push_notification_span(task_id: &str, url: &str) -> A2ASpan {
    A2ASpan::producer("a2a.push_notification")
        .with_attribute("a2a.task_id", task_id)
        .with_attribute("a2a.push_url", url)
}

/// Creates a span for task store operations.
pub fn task_store_span(operation: &str, task_id: &str) -> A2ASpan {
    A2ASpan::internal(&format!("a2a.store.{}", operation))
        .with_attribute("a2a.task_id", task_id)
        .with_attribute("a2a.store.operation", operation)
}

/// Checks if A2A SDK instrumentation is enabled.
pub fn is_instrumentation_enabled() -> bool {
    std::env::var(ENABLED_ENV_VAR)
        .map(|v| v.to_lowercase() != "false")
        .unwrap_or(true)
}

/// A guard that times operations and logs their duration.
pub struct TimingGuard {
    name: String,
    start: Instant,
    threshold_ms: Option<u64>,
}

impl TimingGuard {
    /// Creates a new timing guard.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
            threshold_ms: None,
        }
    }

    /// Sets a threshold in milliseconds for warning logs.
    pub fn with_threshold(mut self, threshold_ms: u64) -> Self {
        self.threshold_ms = Some(threshold_ms);
        self
    }

    /// Returns the elapsed duration.
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }
}

impl Drop for TimingGuard {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;

        if let Some(threshold) = self.threshold_ms {
            if elapsed_ms > threshold {
                tracing::warn!(
                    target: INSTRUMENTATION_MODULE_NAME,
                    operation = %self.name,
                    duration_ms = elapsed_ms,
                    threshold_ms = threshold,
                    "Operation exceeded threshold"
                );
                return;
            }
        }

        tracing::trace!(
            target: INSTRUMENTATION_MODULE_NAME,
            operation = %self.name,
            duration_ms = elapsed_ms,
            "Operation completed"
        );
    }
}

/// Records metrics for A2A operations.
#[derive(Debug, Default)]
pub struct Metrics {
    /// Total number of requests.
    pub requests_total: u64,
    /// Number of successful requests.
    pub requests_success: u64,
    /// Number of failed requests.
    pub requests_failed: u64,
    /// Total request duration in milliseconds.
    pub request_duration_ms: u64,
}

impl Metrics {
    /// Creates new empty metrics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a successful request.
    pub fn record_success(&mut self, duration_ms: u64) {
        self.requests_total += 1;
        self.requests_success += 1;
        self.request_duration_ms += duration_ms;
    }

    /// Records a failed request.
    pub fn record_failure(&mut self, duration_ms: u64) {
        self.requests_total += 1;
        self.requests_failed += 1;
        self.request_duration_ms += duration_ms;
    }

    /// Returns the average request duration in milliseconds.
    pub fn average_duration_ms(&self) -> f64 {
        if self.requests_total == 0 {
            0.0
        } else {
            self.request_duration_ms as f64 / self.requests_total as f64
        }
    }

    /// Returns the success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        if self.requests_total == 0 {
            100.0
        } else {
            (self.requests_success as f64 / self.requests_total as f64) * 100.0
        }
    }
}

/// Thread-safe metrics collector.
#[derive(Debug, Default)]
pub struct MetricsCollector {
    inner: std::sync::RwLock<HashMap<String, Metrics>>,
}

impl MetricsCollector {
    /// Creates a new metrics collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a successful operation.
    pub fn record_success(&self, operation: &str, duration_ms: u64) {
        let mut inner = self.inner.write().unwrap();
        inner
            .entry(operation.to_string())
            .or_default()
            .record_success(duration_ms);
    }

    /// Records a failed operation.
    pub fn record_failure(&self, operation: &str, duration_ms: u64) {
        let mut inner = self.inner.write().unwrap();
        inner
            .entry(operation.to_string())
            .or_default()
            .record_failure(duration_ms);
    }

    /// Gets metrics for a specific operation.
    pub fn get(&self, operation: &str) -> Option<Metrics> {
        let inner = self.inner.read().unwrap();
        inner.get(operation).cloned()
    }

    /// Gets all metrics.
    pub fn all(&self) -> HashMap<String, Metrics> {
        let inner = self.inner.read().unwrap();
        inner.clone()
    }

    /// Resets all metrics.
    pub fn reset(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.clear();
    }
}

impl Clone for Metrics {
    fn clone(&self) -> Self {
        Self {
            requests_total: self.requests_total,
            requests_success: self.requests_success,
            requests_failed: self.requests_failed,
            request_duration_ms: self.request_duration_ms,
        }
    }
}

/// Creates a traced block of code with automatic span creation.
#[macro_export]
macro_rules! traced {
    ($name:expr, $kind:expr, $body:expr) => {{
        let span = $crate::telemetry::A2ASpan::new($name, $kind).start();
        let result = $body;
        span.end();
        result
    }};
    ($name:expr, $body:expr) => {{ $crate::traced!($name, $crate::telemetry::SpanKind::Internal, $body) }};
}

/// Creates a traced async block of code.
#[macro_export]
macro_rules! traced_async {
    ($name:expr, $kind:expr, $body:expr) => {{
        let span = $crate::telemetry::A2ASpan::new($name, $kind).start();
        let result = $body.await;
        span.end();
        result
    }};
    ($name:expr, $body:expr) => {{ $crate::traced_async!($name, $crate::telemetry::SpanKind::Internal, $body) }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_kind_display() {
        assert_eq!(SpanKind::Client.as_str(), "client");
        assert_eq!(SpanKind::Server.as_str(), "server");
        assert_eq!(SpanKind::Internal.as_str(), "internal");
    }

    #[test]
    fn test_span_builder() {
        let span = A2ASpan::new("test_operation", SpanKind::Client)
            .with_attribute("key1", "value1")
            .with_attribute("key2", "value2");

        assert_eq!(span.name, "test_operation");
        assert_eq!(span.kind, SpanKind::Client);
        assert_eq!(span.attributes.len(), 2);
    }

    #[test]
    fn test_active_span() {
        let span = A2ASpan::client("test").start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(span.elapsed().as_millis() >= 10);
        span.end();
    }

    #[test]
    fn test_timing_guard() {
        let _guard = TimingGuard::new("test_op").with_threshold(1000);
        std::thread::sleep(std::time::Duration::from_millis(5));
        // Guard drops here and logs duration
    }

    #[test]
    fn test_metrics() {
        let mut metrics = Metrics::new();
        metrics.record_success(100);
        metrics.record_success(200);
        metrics.record_failure(50);

        assert_eq!(metrics.requests_total, 3);
        assert_eq!(metrics.requests_success, 2);
        assert_eq!(metrics.requests_failed, 1);
        assert_eq!(metrics.request_duration_ms, 350);
        assert!((metrics.average_duration_ms() - 116.67).abs() < 0.1);
        assert!((metrics.success_rate() - 66.67).abs() < 0.1);
    }

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        collector.record_success("op1", 100);
        collector.record_success("op1", 200);
        collector.record_failure("op2", 50);

        let op1 = collector.get("op1").unwrap();
        assert_eq!(op1.requests_total, 2);

        let op2 = collector.get("op2").unwrap();
        assert_eq!(op2.requests_failed, 1);

        collector.reset();
        assert!(collector.get("op1").is_none());
    }

    #[test]
    fn test_predefined_spans() {
        let span = send_message_span("task-1", "ctx-1");
        assert_eq!(span.name, "a2a.message.send");
        assert!(span.attributes.contains_key("a2a.task_id"));

        let span = get_task_span("task-2");
        assert_eq!(span.name, "a2a.tasks.get");

        let span = handle_request_span("message/send");
        assert_eq!(span.name, "a2a.handle_request");
    }
}
