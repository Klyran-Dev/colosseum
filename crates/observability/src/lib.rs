//! Bounded, dependency-free service metrics.
//!
//! The registry deliberately accepts a fixed operation set at construction.
//! User input can therefore never create unbounded metric labels or memory
//! growth. The output follows the Prometheus text exposition format so a
//! collector can scrape it without adding a dependency to domain crates.

use std::{
    collections::BTreeMap,
    env,
    fmt::{Display, Formatter},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

pub const DEFAULT_TRACE_QUEUE_SIZE: usize = 2_048;
pub const DEFAULT_TRACE_BATCH_SIZE: usize = 256;
pub const DEFAULT_TRACE_TIMEOUT_MS: u64 = 5_000;
pub const DEFAULT_TRACE_DELAY_MS: u64 = 1_000;

const OTEL_ENDPOINT_ENV: &str = "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT";
const OTEL_ENVIRONMENT_ENV: &str = "OTEL_ENVIRONMENT";
const OTEL_ALLOW_INSECURE_ENV: &str = "OTEL_ALLOW_INSECURE_TELEMETRY";
const OTEL_SAMPLE_RATIO_ENV: &str = "OTEL_TRACE_SAMPLE_RATIO";
const OTEL_QUEUE_ENV: &str = "OTEL_TRACE_MAX_QUEUE_SIZE";
const OTEL_BATCH_ENV: &str = "OTEL_TRACE_MAX_EXPORT_BATCH_SIZE";
const OTEL_TIMEOUT_ENV: &str = "OTEL_TRACE_EXPORT_TIMEOUT_MS";
const OTEL_DELAY_ENV: &str = "OTEL_TRACE_SCHEDULED_DELAY_MS";

#[derive(Clone, Debug, PartialEq)]
pub struct TelemetryConfig {
    pub service_name: String,
    pub endpoint: Option<String>,
    pub sample_ratio: f64,
    pub max_queue_size: usize,
    pub max_export_batch_size: usize,
    pub export_timeout: Duration,
    pub scheduled_delay: Duration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TelemetryConfigError {
    InvalidServiceName,
    InvalidEndpoint,
    InsecureEndpoint,
    InvalidEnvironment,
    InvalidBoolean,
    InvalidSampleRatio,
    InvalidBound,
    InvalidBatchSize,
}

impl Display for TelemetryConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidServiceName => "service name is not allow-listed",
            Self::InvalidEndpoint => "OTLP endpoint is malformed or too long",
            Self::InsecureEndpoint => {
                "insecure OTLP endpoints are allowed only for local development"
            }
            Self::InvalidEnvironment => "OTEL_ENVIRONMENT must be local, staging, or production",
            Self::InvalidBoolean => "OTEL_ALLOW_INSECURE_TELEMETRY must be true or false",
            Self::InvalidSampleRatio => {
                "OTEL_TRACE_SAMPLE_RATIO must be a finite number from 0.0 to 1.0"
            }
            Self::InvalidBound => "telemetry queue, timeout, or delay is outside its bounded range",
            Self::InvalidBatchSize => "trace batch size must not exceed the queue size",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for TelemetryConfigError {}

#[derive(Debug)]
pub enum TelemetryError {
    Configuration(TelemetryConfigError),
    ExporterBuild(String),
}

impl Display for TelemetryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(error) => error.fmt(formatter),
            Self::ExporterBuild(error) => {
                write!(formatter, "OTLP exporter configuration failed: {error}")
            }
        }
    }
}

impl std::error::Error for TelemetryError {}

impl From<TelemetryConfigError> for TelemetryError {
    fn from(error: TelemetryConfigError) -> Self {
        Self::Configuration(error)
    }
}

impl TelemetryConfig {
    pub fn from_env(service_name: &str) -> Result<Self, TelemetryConfigError> {
        let endpoint = env::var(OTEL_ENDPOINT_ENV).ok();
        let environment = env::var(OTEL_ENVIRONMENT_ENV).unwrap_or_else(|_| "production".into());
        let allow_insecure = match env::var(OTEL_ALLOW_INSECURE_ENV) {
            Ok(value) => parse_bool(&value)?,
            Err(_) => false,
        };
        Self::from_values(
            service_name,
            endpoint.as_deref(),
            &environment,
            allow_insecure,
            env::var(OTEL_SAMPLE_RATIO_ENV).ok().as_deref(),
            env::var(OTEL_QUEUE_ENV).ok().as_deref(),
            env::var(OTEL_BATCH_ENV).ok().as_deref(),
            env::var(OTEL_TIMEOUT_ENV).ok().as_deref(),
            env::var(OTEL_DELAY_ENV).ok().as_deref(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_values(
        service_name: &str,
        endpoint: Option<&str>,
        environment: &str,
        allow_insecure: bool,
        sample_ratio: Option<&str>,
        queue_size: Option<&str>,
        batch_size: Option<&str>,
        timeout_ms: Option<&str>,
        delay_ms: Option<&str>,
    ) -> Result<Self, TelemetryConfigError> {
        if !matches!(
            service_name,
            "api" | "worker" | "indexer" | "route_verifier"
        ) {
            return Err(TelemetryConfigError::InvalidServiceName);
        }
        if !matches!(environment, "local" | "staging" | "production") {
            return Err(TelemetryConfigError::InvalidEnvironment);
        }
        let endpoint = endpoint
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        if let Some(endpoint) = &endpoint {
            if endpoint.len() > 2_048
                || endpoint
                    .chars()
                    .any(|character| character.is_whitespace() || character.is_control())
                || !(endpoint.starts_with("http://") || endpoint.starts_with("https://"))
            {
                return Err(TelemetryConfigError::InvalidEndpoint);
            }
            if endpoint.starts_with("http://") && !(allow_insecure && environment == "local") {
                return Err(TelemetryConfigError::InsecureEndpoint);
            }
        }
        let sample_ratio = sample_ratio
            .map(|value| {
                value
                    .parse::<f64>()
                    .map_err(|_| TelemetryConfigError::InvalidSampleRatio)
            })
            .transpose()?
            .unwrap_or(1.0);
        if !sample_ratio.is_finite() || !(0.0..=1.0).contains(&sample_ratio) {
            return Err(TelemetryConfigError::InvalidSampleRatio);
        }
        let max_queue_size =
            parse_bound(queue_size, 1, 8_192, DEFAULT_TRACE_QUEUE_SIZE as u64)? as usize;
        let max_export_batch_size =
            parse_bound(batch_size, 1, 1_024, DEFAULT_TRACE_BATCH_SIZE as u64)? as usize;
        if max_export_batch_size > max_queue_size {
            return Err(TelemetryConfigError::InvalidBatchSize);
        }
        let timeout_ms = parse_bound(timeout_ms, 50, 10_000, DEFAULT_TRACE_TIMEOUT_MS)?;
        let delay_ms = parse_bound(delay_ms, 100, 60_000, DEFAULT_TRACE_DELAY_MS)?;
        Ok(Self {
            service_name: service_name.to_string(),
            endpoint,
            sample_ratio,
            max_queue_size,
            max_export_batch_size,
            export_timeout: Duration::from_millis(timeout_ms),
            scheduled_delay: Duration::from_millis(delay_ms),
        })
    }
}

fn parse_bool(value: &str) -> Result<bool, TelemetryConfigError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(TelemetryConfigError::InvalidBoolean),
    }
}

fn parse_bound(
    value: Option<&str>,
    minimum: u64,
    maximum: u64,
    default: u64,
) -> Result<u64, TelemetryConfigError> {
    let parsed = value
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| TelemetryConfigError::InvalidBound)
        })
        .transpose()?
        .unwrap_or(default);
    if !(minimum..=maximum).contains(&parsed) {
        return Err(TelemetryConfigError::InvalidBound);
    }
    Ok(parsed)
}

pub struct TelemetryGuard {
    provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
}

impl TelemetryGuard {
    pub fn shutdown(&mut self) {
        if let Some(provider) = self.provider.take()
            && let Err(error) = provider.shutdown_with_timeout(Duration::from_secs(2))
        {
            tracing::warn!(%error, "OTLP trace provider shutdown was incomplete");
        }
    }
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn init_tracing(service_name: &str) -> Result<TelemetryGuard, TelemetryError> {
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_otlp::WithExportConfig as _;
    use opentelemetry_sdk::{
        Resource,
        trace::{BatchConfigBuilder, BatchSpanProcessor, Sampler, SdkTracerProvider},
    };
    use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

    let config = TelemetryConfig::from_env(service_name)?;
    let provider = if let Some(endpoint) = &config.endpoint {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .with_timeout(config.export_timeout)
            .build()
            .map_err(|error| TelemetryError::ExporterBuild(error.to_string()))?;
        let batch_config = BatchConfigBuilder::default()
            .with_max_queue_size(config.max_queue_size)
            .with_max_export_batch_size(config.max_export_batch_size)
            .with_scheduled_delay(config.scheduled_delay)
            .build();
        let processor = BatchSpanProcessor::builder(exporter)
            .with_batch_config(batch_config)
            .build();
        let provider = SdkTracerProvider::builder()
            .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                config.sample_ratio,
            ))))
            .with_resource(
                Resource::builder()
                    .with_service_name(config.service_name.clone())
                    .build(),
            )
            .with_span_processor(processor)
            .build();
        opentelemetry::global::set_tracer_provider(provider.clone());
        Some(provider)
    } else {
        None
    };
    let otel_layer = provider.as_ref().map(|provider| {
        tracing_opentelemetry::layer().with_tracer(provider.tracer(config.service_name.clone()))
    });
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .with(otel_layer)
        .init();
    tracing::info!(service = %config.service_name, otlp_tracing = provider.is_some(), "observability initialized");
    Ok(TelemetryGuard { provider })
}

pub const HISTOGRAM_BUCKETS_MS: [u64; 7] = [10, 25, 50, 100, 250, 500, 1_000];

#[derive(Clone, Debug)]
pub struct ServiceMetrics {
    inner: Arc<MetricsInner>,
}

#[derive(Debug)]
struct MetricsInner {
    service: String,
    operations: BTreeMap<String, OperationMetrics>,
}

#[derive(Debug)]
struct OperationMetrics {
    requests: AtomicU64,
    errors: AtomicU64,
    duration_sum_ms: AtomicU64,
    duration_count: AtomicU64,
    buckets: [AtomicU64; HISTOGRAM_BUCKETS_MS.len()],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetricsError {
    InvalidMetricName,
    UnknownOperation,
}

impl Display for MetricsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMetricName => {
                formatter.write_str("metric names must be lowercase ASCII identifiers")
            }
            Self::UnknownOperation => formatter.write_str("operation is not registered"),
        }
    }
}

impl std::error::Error for MetricsError {}

impl ServiceMetrics {
    pub fn new(service: &str, operations: &[&str]) -> Result<Self, MetricsError> {
        if !valid_name(service) || operations.iter().any(|operation| !valid_name(operation)) {
            return Err(MetricsError::InvalidMetricName);
        }
        let mut registered = BTreeMap::new();
        for operation in operations {
            if registered
                .insert((*operation).to_string(), OperationMetrics::default())
                .is_some()
            {
                return Err(MetricsError::InvalidMetricName);
            }
        }
        Ok(Self {
            inner: Arc::new(MetricsInner {
                service: service.to_string(),
                operations: registered,
            }),
        })
    }

    pub fn observe(
        &self,
        operation: &str,
        elapsed: Duration,
        error: bool,
    ) -> Result<(), MetricsError> {
        let metrics = self
            .inner
            .operations
            .get(operation)
            .ok_or(MetricsError::UnknownOperation)?;
        metrics.requests.fetch_add(1, Ordering::Relaxed);
        if error {
            metrics.errors.fetch_add(1, Ordering::Relaxed);
        }
        let elapsed_ms = elapsed.as_millis().min(u64::MAX as u128) as u64;
        saturating_add(&metrics.duration_sum_ms, elapsed_ms);
        metrics.duration_count.fetch_add(1, Ordering::Relaxed);
        if let Some(index) = HISTOGRAM_BUCKETS_MS
            .iter()
            .position(|bucket| elapsed_ms <= *bucket)
        {
            metrics.buckets[index].fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn render_prometheus(&self) -> String {
        let mut output = String::new();
        output.push_str("# TYPE stp_requests_total counter\n");
        output.push_str("# TYPE stp_errors_total counter\n");
        output.push_str("# TYPE stp_request_duration_ms histogram\n");
        for (operation, metrics) in &self.inner.operations {
            let labels = format!(
                "service=\"{}\",operation=\"{}\"",
                self.inner.service, operation
            );
            output.push_str(&format!(
                "stp_requests_total{{{labels}}} {}\n",
                metrics.requests.load(Ordering::Relaxed)
            ));
            output.push_str(&format!(
                "stp_errors_total{{{labels}}} {}\n",
                metrics.errors.load(Ordering::Relaxed)
            ));
            let mut cumulative = 0_u64;
            for (index, bucket) in HISTOGRAM_BUCKETS_MS.iter().enumerate() {
                cumulative =
                    cumulative.saturating_add(metrics.buckets[index].load(Ordering::Relaxed));
                output.push_str(&format!(
                    "stp_request_duration_ms_bucket{{{labels},le=\"{bucket}\"}} {cumulative}\n"
                ));
            }
            output.push_str(&format!(
                "stp_request_duration_ms_bucket{{{labels},le=\"+Inf\"}} {}\n",
                metrics.duration_count.load(Ordering::Relaxed)
            ));
            output.push_str(&format!(
                "stp_request_duration_ms_sum{{{labels}}} {}\n",
                metrics.duration_sum_ms.load(Ordering::Relaxed)
            ));
            output.push_str(&format!(
                "stp_request_duration_ms_count{{{labels}}} {}\n",
                metrics.duration_count.load(Ordering::Relaxed)
            ));
        }
        output
    }
}

impl Default for OperationMetrics {
    fn default() -> Self {
        Self {
            requests: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            duration_sum_ms: AtomicU64::new(0),
            duration_count: AtomicU64::new(0),
            buckets: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn saturating_add(value: &AtomicU64, amount: u64) {
    value
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            Some(current.saturating_add(amount))
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_bounded_prometheus_series_and_cumulative_buckets() {
        let metrics = ServiceMetrics::new("api", &["quote"]).unwrap();
        metrics
            .observe("quote", Duration::from_millis(25), false)
            .unwrap();
        metrics
            .observe("quote", Duration::from_millis(1_500), true)
            .unwrap();
        let rendered = metrics.render_prometheus();

        assert!(rendered.contains("stp_requests_total{service=\"api\",operation=\"quote\"} 2"));
        assert!(rendered.contains("stp_errors_total{service=\"api\",operation=\"quote\"} 1"));
        assert!(rendered.contains("le=\"25\"} 1"));
        assert!(rendered.contains("le=\"+Inf\"} 2"));
        assert!(
            rendered
                .contains("stp_request_duration_ms_count{service=\"api\",operation=\"quote\"} 2")
        );
    }

    #[test]
    fn rejects_dynamic_or_duplicate_metric_operations() {
        assert_eq!(
            ServiceMetrics::new("api", &["quote", "quote"]).unwrap_err(),
            MetricsError::InvalidMetricName
        );
        let metrics = ServiceMetrics::new("api", &["quote"]).unwrap();
        assert_eq!(
            metrics.observe("user-controlled", Duration::ZERO, false),
            Err(MetricsError::UnknownOperation)
        );
    }

    #[test]
    fn disabled_trace_export_has_bounded_defaults_without_an_endpoint() {
        let config =
            TelemetryConfig::from_values("api", None, "local", false, None, None, None, None, None)
                .unwrap();

        assert_eq!(config.service_name, "api");
        assert_eq!(config.endpoint, None);
        assert_eq!(config.sample_ratio, 1.0);
        assert_eq!(config.max_queue_size, DEFAULT_TRACE_QUEUE_SIZE);
        assert_eq!(config.max_export_batch_size, DEFAULT_TRACE_BATCH_SIZE);
        assert_eq!(
            config.export_timeout,
            Duration::from_millis(DEFAULT_TRACE_TIMEOUT_MS)
        );
        assert_eq!(
            config.scheduled_delay,
            Duration::from_millis(DEFAULT_TRACE_DELAY_MS)
        );
    }

    #[test]
    fn validates_trace_endpoint_security_and_service_allow_list() {
        assert_eq!(
            TelemetryConfig::from_values(
                "unknown", None, "local", false, None, None, None, None, None
            ),
            Err(TelemetryConfigError::InvalidServiceName)
        );
        assert_eq!(
            TelemetryConfig::from_values(
                "api",
                Some("http://collector:4317"),
                "production",
                true,
                None,
                None,
                None,
                None,
                None
            ),
            Err(TelemetryConfigError::InsecureEndpoint)
        );
        assert!(
            TelemetryConfig::from_values(
                "api",
                Some("http://collector:4317"),
                "local",
                true,
                None,
                None,
                None,
                None,
                None
            )
            .is_ok()
        );
        assert!(
            TelemetryConfig::from_values(
                "api",
                Some("https://collector:4317"),
                "production",
                false,
                None,
                None,
                None,
                None,
                None
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_invalid_trace_sampling_and_batch_bounds() {
        assert_eq!(
            TelemetryConfig::from_values(
                "api",
                Some("https://collector:4317"),
                "production",
                false,
                Some("1.1"),
                None,
                None,
                None,
                None
            ),
            Err(TelemetryConfigError::InvalidSampleRatio)
        );
        assert_eq!(
            TelemetryConfig::from_values(
                "api",
                Some("https://collector:4317"),
                "production",
                false,
                Some("0.5"),
                Some("128"),
                Some("256"),
                None,
                None
            ),
            Err(TelemetryConfigError::InvalidBatchSize)
        );
        assert_eq!(
            TelemetryConfig::from_values(
                "api",
                Some("https://collector:4317"),
                "production",
                false,
                Some("0.5"),
                Some("0"),
                None,
                None,
                None
            ),
            Err(TelemetryConfigError::InvalidBound)
        );
    }
}
