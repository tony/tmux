//! OpenTelemetry instrumentation bridge.
//!
//! Provides span creation and metric recording for kernel operations.

#![forbid(unsafe_code)]

use mux_time::Timestamp;
use thiserror::Error;

/// OTel integration errors.
#[derive(Debug, Error)]
pub enum OtelError {
    #[error("otel not configured")]
    NotConfigured,
    #[error("export failed: {0}")]
    ExportFailed(String),
}

/// Span kind for categorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    Internal,
    Server,
    Client,
}

/// A recorded span.
#[derive(Debug, Clone)]
pub struct SpanRecord {
    pub name: String,
    pub kind: SpanKind,
    pub start: Timestamp,
    pub end: Option<Timestamp>,
    pub attributes: Vec<(String, String)>,
}

impl SpanRecord {
    pub fn new(name: &str, kind: SpanKind, start: Timestamp) -> Self {
        Self {
            name: name.to_owned(),
            kind,
            start,
            end: None,
            attributes: Vec::new(),
        }
    }

    pub fn finish(&mut self, end: Timestamp) {
        self.end = Some(end);
    }

    pub fn attr(&mut self, key: &str, value: &str) {
        self.attributes.push((key.to_owned(), value.to_owned()));
    }

    pub fn duration_ns(&self) -> Option<u64> {
        self.end.map(|e| e.as_nanos().saturating_sub(self.start.as_nanos()))
    }
}

/// A metrics collector (no-op in scaffold).
#[derive(Debug, Default)]
pub struct MetricsCollector {
    spans: Vec<SpanRecord>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_span(&mut self, span: SpanRecord) {
        self.spans.push(span);
    }

    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    pub fn spans(&self) -> &[SpanRecord] {
        &self.spans
    }

    pub fn clear(&mut self) {
        self.spans.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_creation() {
        let span = SpanRecord::new("test", SpanKind::Internal, Timestamp::from_nanos(0));
        assert_eq!(span.name, "test");
        assert_eq!(span.kind, SpanKind::Internal);
    }

    #[test]
    fn span_finish() {
        let mut span = SpanRecord::new("op", SpanKind::Server, Timestamp::from_nanos(100));
        span.finish(Timestamp::from_nanos(200));
        assert_eq!(span.duration_ns(), Some(100));
    }

    #[test]
    fn span_attributes() {
        let mut span = SpanRecord::new("op", SpanKind::Internal, Timestamp::from_nanos(0));
        span.attr("pane_id", "5");
        assert_eq!(span.attributes.len(), 1);
    }

    #[test]
    fn metrics_collector() {
        let mut mc = MetricsCollector::new();
        mc.record_span(SpanRecord::new("op1", SpanKind::Internal, Timestamp::from_nanos(0)));
        mc.record_span(SpanRecord::new("op2", SpanKind::Server, Timestamp::from_nanos(0)));
        assert_eq!(mc.span_count(), 2);
    }

    #[test]
    fn metrics_clear() {
        let mut mc = MetricsCollector::new();
        mc.record_span(SpanRecord::new("op", SpanKind::Internal, Timestamp::from_nanos(0)));
        mc.clear();
        assert_eq!(mc.span_count(), 0);
    }

    #[test]
    fn span_kind_variants() {
        assert_ne!(SpanKind::Internal, SpanKind::Server);
        assert_ne!(SpanKind::Server, SpanKind::Client);
    }

    #[test]
    fn span_unfinished_duration() {
        let span = SpanRecord::new("op", SpanKind::Internal, Timestamp::from_nanos(0));
        assert!(span.duration_ns().is_none());
    }

    #[test]
    fn otel_error_display() {
        let e = OtelError::NotConfigured;
        assert_eq!(e.to_string(), "otel not configured");
    }

    #[test]
    fn default_collector() {
        let mc = MetricsCollector::default();
        assert_eq!(mc.span_count(), 0);
    }

    #[test]
    fn spans_accessor() {
        let mut mc = MetricsCollector::new();
        mc.record_span(SpanRecord::new("op", SpanKind::Internal, Timestamp::from_nanos(0)));
        assert_eq!(mc.spans().len(), 1);
    }

    #[test]
    fn export_failed_error() {
        let e = OtelError::ExportFailed("timeout".into());
        assert!(e.to_string().contains("timeout"));
    }

    #[test]
    fn zero_duration_span() {
        let mut span = SpanRecord::new("op", SpanKind::Internal, Timestamp::from_nanos(100));
        span.finish(Timestamp::from_nanos(100));
        assert_eq!(span.duration_ns(), Some(0));
    }

    #[test]
    fn span_multiple_attributes() {
        let mut span = SpanRecord::new("op", SpanKind::Internal, Timestamp::from_nanos(0));
        span.attr("session", "dev");
        span.attr("window", "main");
        span.attr("pane", "0");
        assert_eq!(span.attributes.len(), 3);
    }

    #[test]
    fn span_clone() {
        let mut span = SpanRecord::new("op", SpanKind::Server, Timestamp::from_nanos(0));
        span.attr("key", "val");
        span.finish(Timestamp::from_nanos(500));
        let span2 = span.clone();
        assert_eq!(span.name, span2.name);
        assert_eq!(span.duration_ns(), span2.duration_ns());
    }

    #[test]
    fn span_kind_clone() {
        let k = SpanKind::Client;
        let k2 = k;
        assert_eq!(k, k2);
    }

    #[test]
    fn collector_multiple_clear() {
        let mut mc = MetricsCollector::new();
        mc.record_span(SpanRecord::new("a", SpanKind::Internal, Timestamp::from_nanos(0)));
        mc.clear();
        mc.record_span(SpanRecord::new("b", SpanKind::Internal, Timestamp::from_nanos(0)));
        mc.clear();
        assert_eq!(mc.span_count(), 0);
    }

    #[test]
    fn span_debug() {
        let span = SpanRecord::new("op", SpanKind::Internal, Timestamp::from_nanos(0));
        let dbg = format!("{span:?}");
        assert!(dbg.contains("SpanRecord"));
    }

    #[test]
    fn large_duration_span() {
        let mut span = SpanRecord::new("op", SpanKind::Internal, Timestamp::from_nanos(0));
        span.finish(Timestamp::from_nanos(1_000_000_000)); // 1 second
        assert_eq!(span.duration_ns(), Some(1_000_000_000));
    }

    #[test]
    fn collector_spans_order() {
        let mut mc = MetricsCollector::new();
        mc.record_span(SpanRecord::new("first", SpanKind::Internal, Timestamp::from_nanos(0)));
        mc.record_span(SpanRecord::new("second", SpanKind::Server, Timestamp::from_nanos(100)));
        assert_eq!(mc.spans()[0].name, "first");
        assert_eq!(mc.spans()[1].name, "second");
    }
}
