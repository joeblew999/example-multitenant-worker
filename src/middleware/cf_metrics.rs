//! CF Analytics Engine binding wired into this worker.
//!
//! Implements `connectrpc_cf_metrics::MetricSink` for `worker::AnalyticsEngineDataset`.
//! Schema decisions are made HERE (which labels become blobs, which numeric
//! values become doubles), because the AE data point shape is bespoke per
//! deployment — the crate stays schema-agnostic.

use std::sync::Arc;

use async_trait::async_trait;
use connectrpc_cf_metrics::{MetricSink, MetricsInterceptor, MetricsLayer};
use worker::AnalyticsEngineDataset;

/// Adapter from `worker::AnalyticsEngineDataset` → the crate's `MetricSink`
/// trait.
///
/// ### AE schema (this worker)
///
/// One data point per metric emission. Layout:
///
/// - `blobs[0]` = metric name (`"rpc_requests_total"` / `"rpc_latency_ms"`)
/// - `blobs[1]` = procedure (e.g. `"/workers.auth.v1.AuthService/Login"`)
/// - `blobs[2]` = status_class (`"2xx"` / `"4xx"` / `"5xx"`)
/// - `doubles[0]` = value (`1.0` for counter increments, `elapsed_ms`
///   for the histogram)
/// - `indexes[0]` = procedure (cardinality key — AE bills indexes
///   separately; one per high-cardinality grouping is the rule)
///
/// Query this dataset via the SQL API:
///
/// ```sql
/// SELECT blob1 AS metric, blob2 AS procedure, blob3 AS status_class,
///        SUM(double1) AS total,
///        avg(double1) AS avg_latency_ms
/// FROM workers_multitenant_rpc_metrics
/// WHERE timestamp > now() - INTERVAL '1' HOUR
/// GROUP BY metric, procedure, status_class
/// ```
pub struct AeMetricSink(pub Arc<AnalyticsEngineDataset>);

#[async_trait]
impl MetricSink for AeMetricSink {
    async fn counter(&self, name: &str, value: u64, labels: &[(&str, &str)]) {
        write_point(&self.0, name, value as f64, labels);
    }

    async fn histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        write_point(&self.0, name, value, labels);
    }
}

/// Map the (name, value, labels) tuple to an AE data point and write.
/// Errors logged but never propagated — a broken metric write must not
/// fail the request.
fn write_point(
    ds: &AnalyticsEngineDataset,
    name: &str,
    value: f64,
    labels: &[(&str, &str)],
) {
    // worker-rs 0.8 exposes AnalyticsEngineDataPoint via a builder. Pull
    // procedure + status_class out by name so the blob order is stable
    // even if the label slice order changes.
    let procedure = labels
        .iter()
        .find_map(|(k, v)| (*k == "procedure").then_some(*v))
        .unwrap_or("unknown");
    let status_class = labels
        .iter()
        .find_map(|(k, v)| (*k == "status_class").then_some(*v))
        .unwrap_or("unknown");

    // Use the builder — worker::AnalyticsEngineDataPoint itself has no
    // public constructor; you go through AnalyticsEngineDataPointBuilder
    // and `.build()` produces the data point.
    let point = worker::AnalyticsEngineDataPointBuilder::new()
        .indexes([procedure])
        .blobs([
            worker::BlobType::String(name.to_string()),
            worker::BlobType::String(procedure.to_string()),
            worker::BlobType::String(status_class.to_string()),
        ])
        .doubles([value])
        .build();

    if let Err(e) = ds.write_data_point(&point) {
        tracing::warn!(
            target: "connectrpc_cf_metrics",
            err = %e,
            metric = %name,
            "AE write_data_point failed — metric dropped",
        );
    }
}

/// Build the metrics layer. Default metric names
/// (`rpc_requests_total`, `rpc_latency_ms`) are fine; can override via
/// the builder methods if a downstream dashboard expects different names.
///
/// Kept for non-connectrpc tower stacks; this worker now wires the
/// interceptor below instead (see [`metrics_interceptor`]).
pub fn metrics_layer(binding: AnalyticsEngineDataset) -> MetricsLayer<AeMetricSink> {
    MetricsLayer::new(AeMetricSink(Arc::new(binding)))
}

/// Build the metrics INTERCEPTOR (`connectrpc::Interceptor` surface).
/// Same `AeMetricSink` as [`metrics_layer`] — register on the service
/// with `.with_interceptor(..)`. Preferred for this connectrpc worker:
/// the `procedure` label comes from `Spec::procedure` (proto-qualified)
/// and `status_class` from the typed `Result<_, ConnectError>` rather
/// than from sniffing the HTTP response status.
pub fn metrics_interceptor(binding: AnalyticsEngineDataset) -> MetricsInterceptor<AeMetricSink> {
    MetricsInterceptor::new(AeMetricSink(Arc::new(binding)))
}
