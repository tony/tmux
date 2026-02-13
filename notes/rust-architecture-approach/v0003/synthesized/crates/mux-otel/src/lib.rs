//! # mux-otel
//!
//! OpenTelemetry tracing integration for `TermForge`.
//! Provides span context propagation across server, client, and language bindings.
//!
//! ## Features
//! - `SpanContext`: portable span context for cross-process propagation
//! - `TraceConfig`: configuration for OTLP export endpoints
//! - `TracingLayer`: named tracing layers for the 3-thread architecture
//! - Scaffold: no actual OTLP export; all operations are no-ops with correct types
//!
//! L5 external surface crate.

#![forbid(unsafe_code)]

use thiserror::Error;
use tracing::Level;

/// OpenTelemetry integration errors.
#[derive(Debug, Error)]
pub enum OtelError {
    /// Failed to initialize the tracing pipeline.
    #[error("init error: {0}")]
    Init(String),
    /// Invalid configuration.
    #[error("config error: {0}")]
    Config(String),
    /// Export failed.
    #[error("export error: {0}")]
    Export(String),
}

/// Portable span context for cross-process propagation.
/// Follows W3C Trace Context format (trace-id, span-id, trace-flags).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanContext {
    /// 32-hex-character trace ID.
    pub trace_id: String,
    /// 16-hex-character span ID.
    pub span_id: String,
    /// Trace flags (e.g., "01" for sampled).
    pub trace_flags: String,
}

impl SpanContext {
    /// Create a new span context.
    #[must_use]
    pub fn new(trace_id: &str, span_id: &str) -> Self {
        Self {
            trace_id: trace_id.to_owned(),
            span_id: span_id.to_owned(),
            trace_flags: "01".to_owned(),
        }
    }

    /// Create from W3C traceparent header value.
    /// Format: `00-<trace_id>-<span_id>-<trace_flags>`
    pub fn from_traceparent(header: &str) -> Result<Self, OtelError> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 {
            return Err(OtelError::Config(
                "traceparent must have 4 dash-separated parts".into(),
            ));
        }
        if parts[0] != "00" {
            return Err(OtelError::Config("unsupported traceparent version".into()));
        }
        if parts[1].len() != 32 {
            return Err(OtelError::Config("trace_id must be 32 hex chars".into()));
        }
        if parts[2].len() != 16 {
            return Err(OtelError::Config("span_id must be 16 hex chars".into()));
        }

        Ok(Self {
            trace_id: parts[1].to_owned(),
            span_id: parts[2].to_owned(),
            trace_flags: parts[3].to_owned(),
        })
    }

    /// Serialize to W3C traceparent format.
    #[must_use]
    pub fn to_traceparent(&self) -> String {
        format!("00-{}-{}-{}", self.trace_id, self.span_id, self.trace_flags)
    }

    /// Check if the trace is sampled.
    #[must_use]
    pub fn is_sampled(&self) -> bool {
        // Last bit of trace_flags indicates sampling.
        self.trace_flags
            .as_bytes()
            .last()
            .is_some_and(|&b| (b - b'0') & 1 == 1)
    }
}

/// Configuration for the OTLP tracing pipeline.
#[derive(Debug, Clone)]
pub struct TraceConfig {
    /// OTLP endpoint URL.
    pub endpoint: String,
    /// Service name for trace attribution.
    pub service_name: String,
    /// Minimum log level to export.
    pub min_level: Level,
    /// Whether tracing is enabled.
    pub enabled: bool,
}

impl TraceConfig {
    /// Create a default trace configuration.
    #[must_use]
    pub fn new(service_name: &str) -> Self {
        Self {
            endpoint: "http://localhost:4317".to_owned(),
            service_name: service_name.to_owned(),
            min_level: Level::INFO,
            enabled: false, // Off by default in scaffold.
        }
    }

    /// Set the OTLP endpoint.
    #[must_use]
    pub fn endpoint(mut self, url: &str) -> Self {
        url.clone_into(&mut self.endpoint);
        self
    }

    /// Enable or disable tracing.
    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the minimum level.
    #[must_use]
    pub const fn min_level(mut self, level: Level) -> Self {
        self.min_level = level;
        self
    }
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self::new("termforge")
    }
}

/// Named tracing layers matching the 3-thread architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracingLayer {
    /// The kernel thread (sans-IO reducer).
    Kernel,
    /// The IO thread (PTY, socket, signal bridge).
    Io,
    /// The render thread (composite, diff, write).
    Render,
    /// Client-side tracing.
    Client,
    /// Language binding tracing (Node.js, Python).
    Binding,
}

impl TracingLayer {
    /// Get the span name prefix for this layer.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::Io => "io",
            Self::Render => "render",
            Self::Client => "client",
            Self::Binding => "binding",
        }
    }

    /// Format a span name with this layer's prefix.
    #[must_use]
    pub fn span_name(self, operation: &str) -> String {
        format!("{}.{operation}", self.prefix())
    }
}

/// Initialize the tracing pipeline (scaffold: no-op).
pub const fn init_tracing(_config: &TraceConfig) -> Result<(), OtelError> {
    // Scaffold: no actual OTLP initialization.
    Ok(())
}

/// Shut down the tracing pipeline (scaffold: no-op).
pub const fn shutdown_tracing() -> Result<(), OtelError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_context_new() {
        let ctx = SpanContext::new(
            "0af7651916cd43dd8448eb211c80319c",
            "b7ad6b7169203331",
        );
        assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.span_id, "b7ad6b7169203331");
        assert_eq!(ctx.trace_flags, "01");
    }

    #[test]
    fn span_context_roundtrip() {
        let original = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let ctx = SpanContext::from_traceparent(original);
        assert!(ctx.is_ok());
        let ctx = ctx.unwrap_or_else(|_| SpanContext::new("", ""));
        assert_eq!(ctx.to_traceparent(), original);
    }

    #[test]
    fn span_context_sampled() {
        let ctx = SpanContext::new("a".repeat(32).as_str(), "b".repeat(16).as_str());
        assert!(ctx.is_sampled());
    }

    #[test]
    fn span_context_not_sampled() {
        let mut ctx = SpanContext::new("a".repeat(32).as_str(), "b".repeat(16).as_str());
        ctx.trace_flags = "00".to_owned();
        assert!(!ctx.is_sampled());
    }

    #[test]
    fn span_context_invalid_traceparent() {
        assert!(SpanContext::from_traceparent("invalid").is_err());
    }

    #[test]
    fn span_context_bad_version() {
        let bad = "01-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        assert!(SpanContext::from_traceparent(bad).is_err());
    }

    #[test]
    fn span_context_short_trace_id() {
        let bad = "00-short-b7ad6b7169203331-01";
        assert!(SpanContext::from_traceparent(bad).is_err());
    }

    #[test]
    fn trace_config_defaults() {
        let config = TraceConfig::default();
        assert_eq!(config.service_name, "termforge");
        assert_eq!(config.endpoint, "http://localhost:4317");
        assert!(!config.enabled);
    }

    #[test]
    fn trace_config_builder() {
        let config = TraceConfig::new("my-service")
            .endpoint("http://otel:4317")
            .enabled(true)
            .min_level(Level::DEBUG);
        assert_eq!(config.service_name, "my-service");
        assert_eq!(config.endpoint, "http://otel:4317");
        assert!(config.enabled);
    }

    #[test]
    fn tracing_layer_prefix() {
        assert_eq!(TracingLayer::Kernel.prefix(), "kernel");
        assert_eq!(TracingLayer::Io.prefix(), "io");
        assert_eq!(TracingLayer::Render.prefix(), "render");
        assert_eq!(TracingLayer::Client.prefix(), "client");
        assert_eq!(TracingLayer::Binding.prefix(), "binding");
    }

    #[test]
    fn tracing_layer_span_name() {
        assert_eq!(
            TracingLayer::Kernel.span_name("create_session"),
            "kernel.create_session"
        );
    }

    #[test]
    fn tracing_layer_equality() {
        assert_eq!(TracingLayer::Kernel, TracingLayer::Kernel);
        assert_ne!(TracingLayer::Kernel, TracingLayer::Io);
    }

    #[test]
    fn init_tracing_noop() {
        let config = TraceConfig::default();
        assert!(init_tracing(&config).is_ok());
    }

    #[test]
    fn shutdown_tracing_noop() {
        assert!(shutdown_tracing().is_ok());
    }
}
