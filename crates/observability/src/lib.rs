//! Bounded, dependency-free service metrics.
//!
//! The registry deliberately accepts a fixed operation set at construction.
//! User input can therefore never create unbounded metric labels or memory
//! growth. The output follows the Prometheus text exposition format so a
//! collector can scrape it without adding a dependency to domain crates.

use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

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
}
