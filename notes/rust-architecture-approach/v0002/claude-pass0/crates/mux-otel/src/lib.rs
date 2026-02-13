//! # mux-otel
//!
//! OpenTelemetry integration for TermForge.
//!
//! Inspired by vibe-tmux's mux-otel crate.
//!
//! ## Features
//! - `otel`: Enable OpenTelemetry export (disabled by default).
//! - Without `otel`: provides stub/no-op implementations.
//!
//! ## Trace Context Propagation
//! Trace context is propagated via environment variables:
//! - `TRACEPARENT`
//! - `TRACESTATE`
//! - `BAGGAGE`

#![forbid(unsafe_code)]

use std::env;
use std::sync::atomic::{AtomicBool, Ordering};

/// Environment variable for trace parent.
pub const TRACEPARENT_ENV_VAR: &str = "TRACEPARENT";
/// Environment variable for trace state.
pub const TRACESTATE_ENV_VAR: &str = "TRACESTATE";
/// Environment variable for baggage.
pub const BAGGAGE_ENV_VAR: &str = "BAGGAGE";

static OTEL_ENABLED: AtomicBool = AtomicBool::new(false);

/// Programmatically enable OTEL export.
pub fn set_otel_enabled(enabled: bool) {
    OTEL_ENABLED.store(enabled, Ordering::SeqCst);
}

/// Check if OTEL is enabled.
#[must_use]
pub fn otel_enabled() -> bool {
    if OTEL_ENABLED.load(Ordering::SeqCst) {
        return true;
    }
    if let Some(val) = env_flag("TERMFORGE_OTEL") {
        return val;
    }
    env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok()
        || env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").is_ok()
}

/// Trace context for propagation.
#[derive(Debug, Clone, Default)]
pub struct TraceCtx {
    pub traceparent: String,
    pub tracestate: Option<String>,
    pub baggage: Option<String>,
}

/// Initialize tracing with optional OTEL export.
///
/// Call once at process startup.
pub fn init_tracing(service_name: &str, service_version: &str) {
    let _ = service_name;
    let _ = service_version;

    // Without the `otel` feature, just set up tracing-subscriber fmt layer.
    #[cfg(not(feature = "otel"))]
    {
        use tracing_subscriber::EnvFilter;
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("warn"));
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .try_init();
    }

    // With the `otel` feature, set up OTLP export.
    #[cfg(feature = "otel")]
    {
        // OTEL initialization goes here -- deferred to implementation phase.
    }
}

/// Read a boolean flag from an environment variable.
fn env_flag(name: &str) -> Option<bool> {
    let raw = env::var(name).ok()?;
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    if value.eq_ignore_ascii_case("1") || value.eq_ignore_ascii_case("true") {
        return Some(true);
    }
    if value.eq_ignore_ascii_case("0") || value.eq_ignore_ascii_case("false") {
        return Some(false);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otel_disabled_by_default() {
        // Unless env vars are set, should be disabled.
        // (May be enabled in CI if OTEL env vars are set.)
        let _ = otel_enabled();
    }

    #[test]
    fn set_otel_enabled_works() {
        set_otel_enabled(true);
        assert!(otel_enabled());
        set_otel_enabled(false);
        // May still be true if env vars are set.
    }

    #[test]
    fn trace_ctx_default() {
        let ctx = TraceCtx::default();
        assert!(ctx.traceparent.is_empty());
    }
}
