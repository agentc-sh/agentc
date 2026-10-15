use opentelemetry_otlp::ExporterBuildError;
use thiserror::Error;
use tracing_subscriber::{filter::ParseError, util::TryInitError};

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("invalid environment variable {name}: {source}")]
    Environment {
        name: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("unsupported telemetry exporter {value:?}: expected otlp or none")]
    UnsupportedExporter { value: String },

    #[error("{name} is required")]
    MissingConfiguration { name: &'static str },

    #[error("invalid log filter: {0}")]
    LogFilter(#[from] ParseError),

    #[error("telemetry exporter construction failed: {0}")]
    Exporter(#[from] ExporterBuildError),

    #[error("telemetry subscriber initialization failed: {0}")]
    Subscriber(#[from] TryInitError),

    #[error("telemetry I/O operation failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("application execution failed: {0}")]
    Application(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl TelemetryError {
    pub fn environment(
        name: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::Environment { name: name.into(), source: source.into() }
    }

    pub fn unsupported_exporter(value: impl Into<String>) -> Self {
        Self::UnsupportedExporter { value: value.into() }
    }

    pub fn missing_configuration(name: &'static str) -> Self {
        Self::MissingConfiguration { name }
    }

    pub fn log_filter(source: impl Into<ParseError>) -> Self {
        Self::LogFilter(source.into())
    }

    pub fn exporter(source: impl Into<ExporterBuildError>) -> Self {
        Self::Exporter(source.into())
    }

    pub fn subscriber(source: impl Into<TryInitError>) -> Self {
        Self::Subscriber(source.into())
    }

    pub fn io(source: impl Into<std::io::Error>) -> Self {
        Self::Io(source.into())
    }

    pub fn application(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Application(source.into())
    }
}

impl From<anyhow::Error> for TelemetryError {
    fn from(error: anyhow::Error) -> Self {
        Self::Application(error.into())
    }
}
