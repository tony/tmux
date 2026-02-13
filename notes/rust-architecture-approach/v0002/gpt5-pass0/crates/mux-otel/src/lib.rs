use thiserror::Error;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Error)]
pub enum OtelError {
    #[error("failed to configure tracing: {0}")]
    Tracing(String),
}

#[derive(Debug)]
pub struct TracingGuard;

pub fn init_tracing(service_name: &str) -> Result<TracingGuard, OtelError> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init()
        .map_err(|error| OtelError::Tracing(error.to_string()))?;

    tracing::info!(service_name, "tracing initialized");
    Ok(TracingGuard)
}

pub trait TraceCarrier {
    fn set(&mut self, key: &str, value: String);
    fn get(&self, key: &str) -> Option<&str>;
}
