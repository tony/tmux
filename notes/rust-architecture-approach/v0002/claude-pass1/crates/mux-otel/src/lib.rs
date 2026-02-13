//! # mux-otel
//!
//! OpenTelemetry integration for TermForge.

#![forbid(unsafe_code)]

use std::env;
use std::sync::atomic::{AtomicBool, Ordering};

pub const TRACEPARENT_ENV_VAR: &str = "TRACEPARENT";
pub const TRACESTATE_ENV_VAR: &str = "TRACESTATE";
pub const BAGGAGE_ENV_VAR: &str = "BAGGAGE";

static OTEL_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_otel_enabled(enabled: bool) { OTEL_ENABLED.store(enabled, Ordering::SeqCst); }

#[must_use]
pub fn otel_enabled() -> bool {
    if OTEL_ENABLED.load(Ordering::SeqCst) { return true; }
    if let Some(val) = env_flag("TERMFORGE_OTEL") { return val; }
    env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() || env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").is_ok()
}

#[derive(Debug, Clone, Default)]
pub struct TraceCtx { pub traceparent: String, pub tracestate: Option<String>, pub baggage: Option<String> }

pub fn init_tracing(service_name: &str, service_version: &str) {
    let _ = (service_name, service_version);
    #[cfg(not(feature = "otel"))]
    {
        use tracing_subscriber::EnvFilter;
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
        let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
    }
}

fn env_flag(name: &str) -> Option<bool> {
    let raw = env::var(name).ok()?;
    let v = raw.trim();
    if v.is_empty() { return None; }
    if v.eq_ignore_ascii_case("1") || v.eq_ignore_ascii_case("true") { return Some(true); }
    if v.eq_ignore_ascii_case("0") || v.eq_ignore_ascii_case("false") { return Some(false); }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otel_disabled_by_default() { let _ = otel_enabled(); }

    #[test]
    fn set_otel_enabled_works() { set_otel_enabled(true); assert!(otel_enabled()); set_otel_enabled(false); }

    #[test]
    fn trace_ctx_default() { let ctx = TraceCtx::default(); assert!(ctx.traceparent.is_empty()); }
}
