//! Prometheus metrics endpoint

use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::response::Response;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Prometheus metrics collector
#[derive(Debug, Clone)]
pub struct PrometheusMetrics {
    /// HTTP request counters by method and status
    http_requests: Arc<RwLock<HashMap<String, u64>>>,
    /// HTTP request duration histogram buckets
    http_duration: Arc<RwLock<Vec<(String, u64)>>>,
    /// Search query counters
    search_queries: Arc<RwLock<u64>>,
    /// Search query duration
    search_duration: Arc<RwLock<Vec<Duration>>>,
    /// Indexing operations counter
    indexing_ops: Arc<RwLock<u64>>,
    /// System metrics
    system: Arc<RwLock<SystemMetrics>>,
}

#[derive(Debug, Clone)]
struct SystemMetrics {
    memory_bytes: u64,
    cpu_percent: f64,
    #[allow(dead_code)]
    disk_bytes: u64,
    thread_count: u64,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            memory_bytes: 0,
            cpu_percent: 0.0,
            disk_bytes: 0,
            thread_count: 0,
        }
    }
}

impl PrometheusMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            http_requests: Arc::new(RwLock::new(HashMap::new())),
            http_duration: Arc::new(RwLock::new(Vec::new())),
            search_queries: Arc::new(RwLock::new(0)),
            search_duration: Arc::new(RwLock::new(Vec::new())),
            indexing_ops: Arc::new(RwLock::new(0)),
            system: Arc::new(RwLock::new(SystemMetrics::default())),
        }
    }

    /// Record HTTP request
    pub async fn record_http_request(&self, method: &str, status: u16, duration: Duration) {
        let key = format!("{method}_{status}");
        let mut requests = self.http_requests.write().await;
        *requests.entry(key.clone()).or_insert(0) += 1;

        let mut durations = self.http_duration.write().await;
        durations.push((key, duration.as_millis() as u64));
        if durations.len() > 1000 {
            durations.drain(0..100);
        }
    }

    /// Record search query
    pub async fn record_search_query(&self, duration: Duration) {
        let mut count = self.search_queries.write().await;
        *count += 1;

        let mut durations = self.search_duration.write().await;
        durations.push(duration);
        if durations.len() > 1000 {
            durations.drain(0..100);
        }
    }

    /// Record indexing operation
    pub async fn record_indexing_op(&self) {
        let mut count = self.indexing_ops.write().await;
        *count += 1;
    }

    /// Update system metrics
    pub async fn update_system_metrics(&self) {
        let mut system = self.system.write().await;
        // Use sys-info for actual metrics
        if let Ok(mem_info) = sys_info::mem_info() {
            system.memory_bytes = mem_info.total.saturating_sub(mem_info.free) * 1024;
        }
        // CPU usage would require more complex tracking, using thread count as proxy
        system.cpu_percent = 0.0; // Placeholder - would need CPU time tracking
        system.thread_count = num_cpus::get() as u64;
    }

    /// Generate Prometheus format metrics
    pub async fn to_prometheus(&self) -> String {
        let mut output = String::new();

        // HTTP request metrics
        {
            let requests = self.http_requests.read().await;
            for (key, count) in requests.iter() {
                let parts: Vec<&str> = key.split('_').collect();
                if parts.len() >= 2 {
                    let method = parts[0];
                    let status = parts[1];
                    output.push_str(&format!(
                        "lexum_http_requests_total{{method=\"{method}\",status=\"{status}\"}} {count}\n"
                    ));
                }
            }
        }

        // HTTP duration histogram
        {
            let durations = self.http_duration.read().await;
            if !durations.is_empty() {
                let mut buckets: HashMap<String, Vec<u64>> = HashMap::new();
                for (key, ms) in durations.iter() {
                    buckets.entry(key.clone()).or_default().push(*ms);
                }
                for (key, values) in buckets.iter() {
                    let parts: Vec<&str> = key.split('_').collect();
                    if parts.len() >= 2 {
                        let method = parts[0];
                        let status = parts[1];
                        let sum: u64 = values.iter().sum();
                        let count = values.len();
                        output.push_str(&format!(
                            "lexum_http_request_duration_seconds_sum{{method=\"{method}\",status=\"{status}\"}} {}\n",
                            sum as f64 / 1000.0
                        ));
                        output.push_str(&format!(
                            "lexum_http_request_duration_seconds_count{{method=\"{method}\",status=\"{status}\"}} {count}\n"
                        ));
                    }
                }
            }
        }

        // Search metrics
        {
            let queries = self.search_queries.read().await;
            output.push_str(&format!("lexum_search_queries_total {queries}\n"));

            let durations = self.search_duration.read().await;
            if !durations.is_empty() {
                let sum: u64 = durations.iter().map(|d| d.as_millis() as u64).sum();
                let count = durations.len();
                output.push_str(&format!(
                    "lexum_search_duration_seconds_sum {}\n",
                    sum as f64 / 1000.0
                ));
                output.push_str(&format!("lexum_search_duration_seconds_count {count}\n"));
            }
        }

        // Indexing metrics
        {
            let ops = self.indexing_ops.read().await;
            output.push_str(&format!("lexum_indexing_operations_total {ops}\n"));
        }

        // System metrics
        {
            let system = self.system.read().await;
            output.push_str(&format!(
                "lexum_process_memory_bytes {}\n",
                system.memory_bytes
            ));
            output.push_str(&format!(
                "lexum_process_cpu_percent {}\n",
                system.cpu_percent
            ));
            output.push_str(&format!(
                "lexum_process_threads_total {}\n",
                system.thread_count
            ));
        }

        output
    }
}

impl Default for PrometheusMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics endpoint handler
pub async fn metrics_handler(metrics: axum::extract::State<Arc<PrometheusMetrics>>) -> Response {
    metrics.0.update_system_metrics().await;
    let prometheus_output = metrics.0.to_prometheus().await;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; version=0.0.4")
        .body(prometheus_output)
        .unwrap()
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[lexum_macros::tokio_test]
    async fn test_metrics_collection() {
        let metrics = PrometheusMetrics::new();

        metrics
            .record_http_request("GET", 200, Duration::from_millis(100))
            .await;
        metrics
            .record_http_request("GET", 200, Duration::from_millis(200))
            .await;
        metrics.record_search_query(Duration::from_millis(50)).await;

        let output = metrics.to_prometheus().await;
        assert!(output.contains("lexum_http_requests_total"));
        assert!(output.contains("lexum_search_queries_total"));
    }
}
