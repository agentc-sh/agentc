// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_telemetry;

pub mod errors;

pub mod metrics {
    pub use opentelemetry::{
        KeyValue,
        global::meter,
        metrics::{Counter, Histogram, Meter, ObservableGauge, UpDownCounter},
    };
}

pub mod semconv;

use crate::errors::TelemetryError;
use opentelemetry::{KeyValue, global, trace::TracerProvider};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    Resource,
    logs::{BatchLogProcessor, SdkLoggerProvider},
    metrics::{PeriodicReader, SdkMeterProvider},
    trace::SdkTracerProvider,
};
use std::{
    env,
    env::VarError,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tracing::Metadata;
use tracing_subscriber::{
    EnvFilter, Layer,
    filter::filter_fn,
    fmt::{self, format::FmtSpan, time::ChronoUtc},
    layer::{Context, SubscriberExt},
    util::SubscriberInitExt,
};

pub use tracing::{
    Instrument, Level, Span, debug, error, event, field, info, info_span, instrument,
    level_filters::LevelFilter, trace, warn,
};

type LogFilter = Arc<dyn Fn(&Metadata<'_>) -> bool + Send + Sync>;

/// A [`tracing_subscriber::Layer`] that can be dynamically disabled at runtime.
///
/// When disabled, [`enabled`](Self::enabled) returns `false` for every event,
/// effectively silencing all log output without reinitialising the subscriber.
#[derive(Debug, Clone)]
struct LoggingSwitch(Arc<AtomicBool>);

impl LoggingSwitch {
    pub fn new(enabled: bool) -> Self {
        Self(Arc::new(AtomicBool::new(enabled)))
    }

    pub fn is_enabled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub fn enable(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn disable(&self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for LoggingSwitch {
    fn enabled(&self, _metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        self.is_enabled()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TelemetryExporter {
    #[default]
    Otlp,
    None,
}

impl FromStr for TelemetryExporter {
    type Err = TelemetryError;

    fn from_str(value: &str) -> Result<Self, TelemetryError> {
        if value.eq_ignore_ascii_case("otlp") {
            Ok(Self::Otlp)
        } else if value.eq_ignore_ascii_case("none") {
            Ok(Self::None)
        } else {
            Err(TelemetryError::unsupported_exporter(value))
        }
    }
}

/// Supported telemetry protocols.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TelemetryProtocol {
    #[default]
    Http,
}

impl std::fmt::Debug for TelemetryProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelemetryProtocol::Http => write!(f, "Http"),
        }
    }
}

impl std::fmt::Display for TelemetryProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelemetryProtocol::Http => write!(f, "http"),
        }
    }
}

/// Primary manager for telemetry providers and resources.
#[derive(Default, Clone)]
pub struct Telemetry {
    resource: Option<Resource>,
    tracer_provider: Option<Arc<SdkTracerProvider>>,
    meter_provider: Option<Arc<SdkMeterProvider>>,
    logger_provider: Option<Arc<SdkLoggerProvider>>,
    logging_switch: Option<LoggingSwitch>,
}

impl Telemetry {
    pub fn builder() -> TelemetryBuilder {
        TelemetryBuilder::default()
    }

    pub fn resource(&self) -> Option<&Resource> {
        self.resource.as_ref()
    }

    pub fn tracer_provider(&self) -> Option<&Arc<SdkTracerProvider>> {
        self.tracer_provider.as_ref()
    }

    pub fn meter_provider(&self) -> Option<&Arc<SdkMeterProvider>> {
        self.meter_provider.as_ref()
    }

    pub fn logger_provider(&self) -> Option<&Arc<SdkLoggerProvider>> {
        self.logger_provider.as_ref()
    }

    pub fn disable_logging(&self) {
        if let Some(ref switch) = self.logging_switch {
            switch.disable();
        }
    }

    pub fn enable_logging(&self) {
        if let Some(ref switch) = self.logging_switch {
            switch.enable();
        }
    }

    pub fn shutdown(self) -> Result<(), TelemetryError> {
        if let Some(tracer_provider) = self.tracer_provider {
            let _ = tracer_provider.shutdown();
        }
        if let Some(meter_provider) = self.meter_provider {
            let _ = meter_provider.shutdown();
        }
        if let Some(logger_provider) = self.logger_provider {
            let _ = logger_provider.shutdown();
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct TelemetryBuilder {
    service_name: Option<String>,
    environment: Option<String>,
    protocol: Option<TelemetryProtocol>,
    log_level: Option<String>,
    log_filter: Option<LogFilter>,
    traces_endpoint: Option<String>,
    metrics_endpoint: Option<String>,
    logs_endpoint: Option<String>,
    traces_exporter: TelemetryExporter,
    metrics_exporter: TelemetryExporter,
    logs_exporter: TelemetryExporter,
}

impl TelemetryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_env() -> Result<Self, TelemetryError> {
        let mut builder = Self::new();

        if let Some(endpoint) = Self::read_env("OTEL_EXPORTER_OTLP_ENDPOINT")? {
            builder = builder.endpoint(endpoint);
        }

        Ok(builder
            .maybe_traces_endpoint(Self::read_env("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")?)
            .maybe_metrics_endpoint(Self::read_env("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT")?)
            .maybe_logs_endpoint(Self::read_env("OTEL_EXPORTER_OTLP_LOGS_ENDPOINT")?)
            .traces_exporter(
                Self::read_env("OTEL_TRACES_EXPORTER")?
                    .map(|value| value.parse())
                    .transpose()
                    .map_err(|error| TelemetryError::environment("OTEL_TRACES_EXPORTER", error))?
                    .unwrap_or_default(),
            )
            .metrics_exporter(
                Self::read_env("OTEL_METRICS_EXPORTER")?
                    .map(|value| value.parse())
                    .transpose()
                    .map_err(|error| TelemetryError::environment("OTEL_METRICS_EXPORTER", error))?
                    .unwrap_or_default(),
            )
            .logs_exporter(
                Self::read_env("OTEL_LOGS_EXPORTER")?
                    .map(|value| value.parse())
                    .transpose()
                    .map_err(|error| TelemetryError::environment("OTEL_LOGS_EXPORTER", error))?
                    .unwrap_or_default(),
            ))
    }

    fn read_env(name: &str) -> Result<Option<String>, TelemetryError> {
        match env::var(name) {
            Ok(value) if value.is_empty() => Ok(None),
            Ok(value) => Ok(Some(value)),
            Err(VarError::NotPresent) => Ok(None),
            Err(error) => Err(TelemetryError::environment(name, error)),
        }
    }

    fn append_signal_path(endpoint: &str, path: &str) -> String {
        format!(
            "{endpoint}{}",
            if endpoint.ends_with('/') {
                path.trim_start_matches('/')
            } else {
                path
            }
        )
    }

    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        let endpoint = endpoint.into();

        self.traces_endpoint = Some(Self::append_signal_path(&endpoint, "/v1/traces"));
        self.metrics_endpoint = Some(Self::append_signal_path(&endpoint, "/v1/metrics"));
        self.logs_endpoint = Some(Self::append_signal_path(&endpoint, "/v1/logs"));

        self
    }

    pub fn maybe_endpoint(self, endpoint: Option<impl Into<String>>) -> Self {
        match endpoint {
            Some(endpoint) => self.endpoint(endpoint),
            None => self,
        }
    }

    pub fn traces_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.traces_endpoint = Some(endpoint.into());
        self
    }

    pub fn maybe_traces_endpoint(self, endpoint: Option<impl Into<String>>) -> Self {
        match endpoint {
            Some(endpoint) => self.traces_endpoint(endpoint),
            None => self,
        }
    }

    pub fn metrics_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.metrics_endpoint = Some(endpoint.into());
        self
    }

    pub fn maybe_metrics_endpoint(self, endpoint: Option<impl Into<String>>) -> Self {
        match endpoint {
            Some(endpoint) => self.metrics_endpoint(endpoint),
            None => self,
        }
    }

    pub fn logs_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.logs_endpoint = Some(endpoint.into());
        self
    }

    pub fn maybe_logs_endpoint(self, endpoint: Option<impl Into<String>>) -> Self {
        match endpoint {
            Some(endpoint) => self.logs_endpoint(endpoint),
            None => self,
        }
    }

    pub fn traces_exporter(mut self, exporter: TelemetryExporter) -> Self {
        self.traces_exporter = exporter;
        self
    }

    pub fn metrics_exporter(mut self, exporter: TelemetryExporter) -> Self {
        self.metrics_exporter = exporter;
        self
    }

    pub fn logs_exporter(mut self, exporter: TelemetryExporter) -> Self {
        self.logs_exporter = exporter;
        self
    }

    pub fn service_name(mut self, service_name: impl Into<String>) -> Self {
        self.service_name = Some(service_name.into());
        self
    }

    pub fn environment(mut self, environment: impl Into<String>) -> Self {
        self.environment = Some(environment.into());
        self
    }

    pub fn protocol(mut self, protocol: TelemetryProtocol) -> Self {
        self.protocol = Some(protocol);
        self
    }

    pub fn log_level(mut self, log_level: impl Into<String>) -> Self {
        self.log_level = Some(log_level.into());
        self
    }

    pub fn log_filter<F>(mut self, log_filter: F) -> Self
    where
        F: Fn(&Metadata<'_>) -> bool + Send + Sync + 'static,
    {
        self.log_filter = Some(Arc::new(log_filter));
        self
    }

    pub fn build(self) -> Result<Telemetry, TelemetryError> {
        let env_filter = EnvFilter::try_new(
            Self::read_env("LOG_LEVEL")?
                .or(self.log_level.clone())
                .unwrap_or_else(|| "info".to_string()),
        )?;
        let logging_switch = LoggingSwitch::new(true);

        let resource = if [
            (self.traces_endpoint.as_ref(), self.traces_exporter),
            (self.metrics_endpoint.as_ref(), self.metrics_exporter),
            (self.logs_endpoint.as_ref(), self.logs_exporter),
        ]
        .iter()
        .any(|(endpoint, exporter)| endpoint.is_some() && *exporter == TelemetryExporter::Otlp)
        {
            Some(
                Resource::builder()
                    .with_service_name(
                        self.service_name
                            .as_deref()
                            .ok_or(TelemetryError::missing_configuration("service_name"))?
                            .to_owned(),
                    )
                    .with_attributes([
                        KeyValue::new(
                            semconv::resource::SERVICE_VERSION,
                            env!("CARGO_PKG_VERSION"),
                        ),
                        KeyValue::new(
                            semconv::attribute::DEPLOYMENT_ENVIRONMENT_NAME,
                            self.environment
                                .as_deref()
                                .ok_or(TelemetryError::missing_configuration("environment"))?
                                .to_owned(),
                        ),
                        KeyValue::new(
                            semconv::attribute::HOST_NAME,
                            hostname::get()?
                                .to_string_lossy()
                                .into_owned(),
                        ),
                    ])
                    .build(),
            )
        } else {
            None
        };

        let tracer_provider =
            match (self.traces_endpoint.as_ref(), resource.as_ref(), self.traces_exporter) {
                (Some(endpoint), Some(resource), TelemetryExporter::Otlp) => Some(
                    SdkTracerProvider::builder()
                        .with_resource(resource.clone())
                        .with_batch_exporter(match self.protocol.unwrap_or_default() {
                            TelemetryProtocol::Http => SpanExporter::builder()
                                .with_http()
                                .with_endpoint(endpoint.clone())
                                .build()?,
                        })
                        .build(),
                ),
                _ => None,
            };

        let meter_provider =
            match (self.metrics_endpoint.as_ref(), resource.as_ref(), self.metrics_exporter) {
                (Some(endpoint), Some(resource), TelemetryExporter::Otlp) => Some(
                    SdkMeterProvider::builder()
                        .with_resource(resource.clone())
                        .with_reader(
                            PeriodicReader::builder(match self.protocol.unwrap_or_default() {
                                TelemetryProtocol::Http => MetricExporter::builder()
                                    .with_http()
                                    .with_endpoint(endpoint.clone())
                                    .build()?,
                            })
                            .build(),
                        )
                        .build(),
                ),
                _ => None,
            };

        let logger_provider =
            match (self.logs_endpoint.as_ref(), resource.as_ref(), self.logs_exporter) {
                (Some(endpoint), Some(resource), TelemetryExporter::Otlp) => Some(
                    SdkLoggerProvider::builder()
                        .with_resource(resource.clone())
                        .with_log_processor(
                            BatchLogProcessor::builder(match self.protocol.unwrap_or_default() {
                                TelemetryProtocol::Http => LogExporter::builder()
                                    .with_http()
                                    .with_endpoint(endpoint.clone())
                                    .build()?,
                            })
                            .build(),
                        )
                        .build(),
                ),
                _ => None,
            };

        tracing_subscriber::registry()
            .with(logging_switch.clone())
            .with(env_filter.clone())
            .with(
                fmt::layer()
                    .json()
                    .flatten_event(false)
                    .with_timer(ChronoUtc::rfc_3339())
                    .with_span_events(FmtSpan::CLOSE)
                    .with_ansi(false)
                    .with_filter(filter_fn({
                        let log_filter = self.log_filter.clone();

                        move |metadata| {
                            log_filter
                                .as_ref()
                                .is_none_or(|filter| filter(metadata))
                        }
                    }))
                    .boxed(),
            )
            .with(sentry::integrations::tracing::layer().event_filter(|metadata| {
                match *metadata.level() {
                    Level::ERROR => sentry::integrations::tracing::EventFilter::Event,
                    Level::WARN => sentry::integrations::tracing::EventFilter::Breadcrumb,
                    _ => sentry::integrations::tracing::EventFilter::Ignore,
                }
            }))
            .with(
                tracer_provider
                    .as_ref()
                    .map(|provider| {
                        tracing_opentelemetry::layer().with_tracer(
                            provider.tracer(
                                self.service_name
                                    .clone()
                                    .expect("trace provider requires service_name"),
                            ),
                        )
                    }),
            )
            .with(
                logger_provider
                    .as_ref()
                    .map(|provider| {
                        OpenTelemetryTracingBridge::new(provider)
                            .with_filter(
                                env_filter
                                    .add_directive("hyper=off".parse().unwrap())
                                    .add_directive("opentelemetry=off".parse().unwrap())
                                    .add_directive("tonic=off".parse().unwrap())
                                    .add_directive("h2=off".parse().unwrap())
                                    .add_directive("reqwest=off".parse().unwrap()),
                            )
                            .with_filter(filter_fn({
                                let log_filter = self.log_filter;

                                move |metadata| {
                                    log_filter
                                        .as_ref()
                                        .is_none_or(|filter| filter(metadata))
                                }
                            }))
                    }),
            )
            .try_init()?;

        if let Some(provider) = tracer_provider.as_ref() {
            global::set_tracer_provider(provider.clone());
        }
        if let Some(provider) = meter_provider.as_ref() {
            global::set_meter_provider(provider.clone());
        }

        Ok(Telemetry {
            resource,
            tracer_provider: tracer_provider.map(Arc::new),
            meter_provider: meter_provider.map(Arc::new),
            logger_provider: logger_provider.map(Arc::new),
            logging_switch: Some(logging_switch),
        })
    }
}

/// Bootstraps telemetry from environment variables and provided configuration for the
/// main function.
pub fn bootstrap<F, Fut, E>(service_name: &str, f: F) -> Result<(), TelemetryError>
where
    F: FnOnce(Telemetry) -> Fut,
    Fut: Future<Output = Result<(), E>>,
    E: Into<TelemetryError>,
{
    let _sentry_guard = std::env::var("SENTRY_DSN")
        .ok()
        .map(|dsn| {
            sentry::init(
                sentry::ClientOptions::new()
                    .dsn(&dsn)
                    .release(env!("CARGO_PKG_VERSION"))
                    .environment(
                        std::env::var("APP_ENV").unwrap_or_else(|_| "production".to_string()),
                    ),
            )
        });

    let telemetry = TelemetryBuilder::from_env()?
        .service_name(service_name)
        .environment(std::env::var("APP_ENV").unwrap_or_else(|_| "production".to_string()))
        .build()?;

    tokio::runtime::Runtime::new()?
        .block_on(f(telemetry.clone()))
        .map_err(Into::into)?;

    drop(_sentry_guard);
    telemetry.shutdown()?;

    Ok(())
}
