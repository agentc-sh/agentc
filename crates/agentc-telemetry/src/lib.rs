// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_telemetry;

pub mod metrics {
    pub use opentelemetry::{
        KeyValue,
        global::meter,
        metrics::{Counter, Histogram, Meter, ObservableGauge, UpDownCounter},
    };
}

/// Semantic-convention attribute keys and metric names.
pub mod semconv {
    pub use opentelemetry_semantic_conventions::*;

    /// `gen_ai.execute_tool` operation duration metric name.
    pub const GEN_AI_EXECUTE_TOOL_DURATION: &str = "gen_ai.execute_tool.duration";

    /// `gen_ai.invoke_agent` operation duration metric name.
    pub const GEN_AI_INVOKE_AGENT_DURATION: &str = "gen_ai.invoke_agent.duration";

    /// `gen_ai.invoke_workflow` operation duration metric name.
    pub const GEN_AI_WORKFLOW_DURATION: &str = "gen_ai.workflow.duration";
}

use anyhow::Result;
use opentelemetry::{KeyValue, global, trace::TracerProvider};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource,
    logs::{BatchLogProcessor, SdkLoggerProvider},
    metrics::{PeriodicReader, SdkMeterProvider},
    trace::SdkTracerProvider,
};
use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tracing::Metadata;
use tracing_subscriber::{
    EnvFilter, Layer as TracingLayer,
    filter::filter_fn,
    fmt::{self, format::FmtSpan, time::ChronoUtc},
    layer::{Context, SubscriberExt},
    util::SubscriberInitExt,
};

pub use tracing::{
    Level, Span, debug, error, event, info, instrument, level_filters::LevelFilter, trace, warn,
};

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

impl<S: tracing::Subscriber> TracingLayer<S> for LoggingSwitch {
    fn enabled(&self, _metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        self.is_enabled()
    }
}

/// Supported telemetry protocols.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TelemetryProtocol {
    #[default]
    Http,
}

impl Debug for TelemetryProtocol {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            TelemetryProtocol::Http => write!(f, "Http"),
        }
    }
}

impl Display for TelemetryProtocol {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
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

    pub fn shutdown(self) -> Result<()> {
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

type LogFilter = Arc<dyn Fn(&tracing::Metadata) -> bool + Send + Sync>;

#[derive(Default)]
pub struct TelemetryBuilder {
    endpoint: Option<String>,
    service_name: Option<String>,
    environment: Option<String>,
    protocol: Option<TelemetryProtocol>,
    log_level: Option<String>,
    log_filter: Option<LogFilter>,
}

impl TelemetryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn maybe_endpoint(mut self, endpoint: Option<impl Into<String>>) -> Self {
        if let Some(ep) = endpoint {
            self.endpoint = Some(ep.into());
        }
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
        F: Fn(&tracing::Metadata) -> bool + Send + Sync + 'static,
    {
        self.log_filter = Some(Arc::new(log_filter));
        self
    }

    pub fn build(self) -> Result<Telemetry> {
        let env_filter = EnvFilter::try_from_env("LOG_LEVEL").unwrap_or_else(|_| {
            EnvFilter::new(
                self.log_level
                    .unwrap_or_else(|| "info".to_string()),
            )
        });

        let logging_switch = LoggingSwitch::new(true);

        let subscriber = tracing_subscriber::registry()
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
                            if let Some(ref filter) = log_filter {
                                (filter)(metadata)
                            } else {
                                true
                            }
                        }
                    }))
                    .boxed(),
            )
            .with(sentry::integrations::tracing::layer().event_filter(|md| match *md.level() {
                tracing::Level::ERROR => sentry::integrations::tracing::EventFilter::Event,
                tracing::Level::WARN => sentry::integrations::tracing::EventFilter::Breadcrumb,
                _ => sentry::integrations::tracing::EventFilter::Ignore,
            }));

        if let Some(ref endpoint) = self.endpoint {
            let protocol = self
                .protocol
                .unwrap_or(TelemetryProtocol::Http);
            let service_name = self
                .service_name
                .expect("service_name is required");
            let resource = Resource::builder()
                .with_service_name(service_name.clone())
                .with_attributes([
                    KeyValue::new(semconv::resource::SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
                    KeyValue::new(
                        semconv::attribute::DEPLOYMENT_ENVIRONMENT_NAME,
                        self.environment
                            .expect("environment is required"),
                    ),
                    KeyValue::new(
                        semconv::attribute::HOST_NAME,
                        hostname::get()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                    ),
                ])
                .build();

            let tracer_provider = SdkTracerProvider::builder()
                .with_resource(resource.clone())
                .with_batch_exporter(match protocol {
                    TelemetryProtocol::Http => opentelemetry_otlp::SpanExporter::builder()
                        .with_http()
                        .with_endpoint(format!("{}/v1/traces", endpoint))
                        .build()
                        .unwrap(),
                })
                .build();

            let meter_provider = SdkMeterProvider::builder()
                .with_resource(resource.clone())
                .with_reader(
                    PeriodicReader::builder(match protocol {
                        TelemetryProtocol::Http => opentelemetry_otlp::MetricExporter::builder()
                            .with_http()
                            .with_endpoint(format!("{}/v1/metrics", endpoint))
                            .build()?,
                    })
                    .build(),
                )
                .build();

            let logger_provider = SdkLoggerProvider::builder()
                .with_resource(resource.clone())
                .with_log_processor(
                    BatchLogProcessor::builder(match protocol {
                        TelemetryProtocol::Http => opentelemetry_otlp::LogExporter::builder()
                            .with_http()
                            .with_endpoint(format!("{}/v1/logs", endpoint))
                            .build()?,
                    })
                    .build(),
                )
                .build();

            subscriber
                .with(
                    tracing_opentelemetry::layer()
                        .with_tracer(tracer_provider.tracer(service_name)),
                )
                .with(
                    opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(
                        &logger_provider,
                    )
                    .with_filter(
                        // https://github.com/open-telemetry/opentelemetry-rust/issues/2877
                        env_filter
                            .clone()
                            .add_directive("hyper=off".parse().unwrap())
                            .add_directive("opentelemetry=off".parse().unwrap())
                            .add_directive("tonic=off".parse().unwrap())
                            .add_directive("h2=off".parse().unwrap())
                            .add_directive("reqwest=off".parse().unwrap()),
                    )
                    .with_filter(filter_fn({
                        let log_filter = self.log_filter.clone();

                        move |metadata| {
                            if let Some(ref filter) = log_filter {
                                (filter)(metadata)
                            } else {
                                true
                            }
                        }
                    })),
                )
                .init();

            global::set_tracer_provider(tracer_provider.clone());
            global::set_meter_provider(meter_provider.clone());

            Ok(Telemetry {
                resource: Some(resource),
                tracer_provider: Some(Arc::new(tracer_provider)),
                meter_provider: Some(Arc::new(meter_provider)),
                logger_provider: Some(Arc::new(logger_provider)),
                logging_switch: Some(logging_switch),
            })
        } else {
            subscriber.init();
            Ok(Telemetry {
                logging_switch: Some(logging_switch),
                ..Default::default()
            })
        }
    }
}

/// Bootstraps telemetry from environment variables and provided configuration for the
/// main function.
pub fn bootstrap<F, Fut>(service_name: &str, f: F) -> Result<()>
where
    F: FnOnce(Telemetry) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let _sentry_guard = std::env::var("SENTRY_DSN")
        .ok()
        .and_then(|dsn| {
            Some(sentry::init(sentry::ClientOptions {
                dsn: Some(dsn.parse().ok()?),
                release: Some(env!("CARGO_PKG_VERSION").into()),
                environment: std::env::var("APP_ENV")
                    .ok()
                    .map(Into::into),
                ..Default::default()
            }))
        });

    let telemetry = Telemetry::builder()
        .service_name(service_name)
        .maybe_endpoint(std::env::var("OTLP_ENDPOINT").ok())
        .log_level(std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()))
        .environment(std::env::var("APP_ENV").unwrap_or_else(|_| "production".to_string()))
        .build()?;

    tokio::runtime::Runtime::new()?.block_on(f(telemetry.clone()))?;

    drop(_sentry_guard);
    telemetry.shutdown()?;

    Ok(())
}
