//! # mux-otel
//!
//! OpenTelemetry tracing integration for TermForge.
//! Optional observability layer for production deployments.
//!
//! L5 surface crate.

#![forbid(unsafe_code)]

use thiserror::Error;

/// OTel configuration errors.
#[derive(Debug, Error)]
pub enum OtelError {
    /// Failed to initialize the OTLP exporter.
    #[error("otlp init failed: {0}")]
    InitFailed(String),
}

/// Configuration for OpenTelemetry integration.
#[derive(Debug, Clone)]
pub struct OtelConfig {
    /// OTLP endpoint URL.
    pub endpoint: String,
    /// Service name for traces.
    pub service_name: String,
    /// Whether tracing is enabled.
    pub enabled: bool,
}

impl OtelConfig {
    /// Create a default (disabled) configuration.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            endpoint: String::new(),
            service_name: "termforge".into(),
            enabled: false,
        }
    }

    /// Create an enabled configuration with the given endpoint.
    #[must_use]
    pub fn with_endpoint(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            service_name: "termforge".into(),
            enabled: true,
        }
    }
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otel_disabled_by_default() {
        let config = OtelConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn otel_with_endpoint() {
        let config = OtelConfig::with_endpoint("http://localhost:4317");
        assert!(config.enabled);
        assert_eq!(config.endpoint, "http://localhost:4317");
    }

    #[test]
    fn otel_service_name() {
        let config = OtelConfig::disabled();
        assert_eq!(config.service_name, "termforge");
    }
}
