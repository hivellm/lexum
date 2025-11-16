//! HTTP metrics collection middleware

use crate::handlers::metrics::PrometheusMetrics;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::sync::Arc;
use std::time::Instant;

/// Middleware to collect HTTP request metrics
pub async fn metrics_middleware(
    metrics: Arc<PrometheusMetrics>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().to_string();
    let start = Instant::now();

    let response = next.run(request).await;
    let status = response.status().as_u16();
    let duration = start.elapsed();

    // Record metrics asynchronously
    metrics.record_http_request(&method, status, duration).await;

    response
}
