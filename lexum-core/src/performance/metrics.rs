//! Performance metrics collection and tracking

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Performance metrics collector
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    metrics: Arc<RwLock<Metrics>>,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Metrics::new())),
        }
    }

    /// Record a timing measurement
    pub async fn record_timing(&self, operation: &str, duration: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.record_timing(operation, duration);
    }

    /// Record a counter increment
    pub async fn increment_counter(&self, counter: &str, value: u64) {
        let mut metrics = self.metrics.write().await;
        metrics.increment_counter(counter, value);
    }

    /// Record a gauge value
    pub async fn record_gauge(&self, gauge: &str, value: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.record_gauge(gauge, value);
    }

    /// Get current metrics snapshot
    pub async fn get_metrics(&self) -> Metrics {
        self.metrics.read().await.clone()
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = Metrics::new();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance metrics data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    /// Timing measurements
    pub timings: HashMap<String, TimingStats>,
    /// Counter values
    pub counters: HashMap<String, u64>,
    /// Gauge values
    pub gauges: HashMap<String, f64>,
    /// System metrics
    pub system: SystemMetrics,
}

impl Metrics {
    /// Create new empty metrics
    pub fn new() -> Self {
        Self {
            timings: HashMap::new(),
            counters: HashMap::new(),
            gauges: HashMap::new(),
            system: SystemMetrics::new(),
        }
    }

    /// Record a timing measurement
    pub fn record_timing(&mut self, operation: &str, duration: Duration) {
        let stats = self.timings.entry(operation.to_string()).or_insert_with(TimingStats::new);
        stats.add_measurement(duration);
    }

    /// Increment a counter
    pub fn increment_counter(&mut self, counter: &str, value: u64) {
        *self.counters.entry(counter.to_string()).or_insert(0) += value;
    }

    /// Record a gauge value
    pub fn record_gauge(&mut self, gauge: &str, value: f64) {
        self.gauges.insert(gauge.to_string(), value);
    }

    /// Get timing statistics for an operation
    pub fn get_timing_stats(&self, operation: &str) -> Option<&TimingStats> {
        self.timings.get(operation)
    }

    /// Get counter value
    pub fn get_counter(&self, counter: &str) -> Option<u64> {
        self.counters.get(counter).copied()
    }

    /// Get gauge value
    pub fn get_gauge(&self, gauge: &str) -> Option<f64> {
        self.gauges.get(gauge).copied()
    }

    /// Get all operations with their timing stats
    pub fn get_all_timings(&self) -> &HashMap<String, TimingStats> {
        &self.timings
    }

    /// Get all counters
    pub fn get_all_counters(&self) -> &HashMap<String, u64> {
        &self.counters
    }

    /// Get all gauges
    pub fn get_all_gauges(&self) -> &HashMap<String, f64> {
        &self.gauges
    }
}

/// Timing statistics for an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingStats {
    /// Number of measurements
    pub count: u64,
    /// Total time
    pub total: Duration,
    /// Minimum time
    pub min: Duration,
    /// Maximum time
    pub max: Duration,
    /// Average time
    pub avg: Duration,
    /// P50 percentile
    pub p50: Duration,
    /// P95 percentile
    pub p95: Duration,
    /// P99 percentile
    pub p99: Duration,
    /// Recent measurements (for percentile calculation)
    recent: Vec<Duration>,
}

impl TimingStats {
    /// Create new timing stats
    pub fn new() -> Self {
        Self {
            count: 0,
            total: Duration::ZERO,
            min: Duration::MAX,
            max: Duration::ZERO,
            avg: Duration::ZERO,
            p50: Duration::ZERO,
            p95: Duration::ZERO,
            p99: Duration::ZERO,
            recent: Vec::new(),
        }
    }

    /// Add a timing measurement
    pub fn add_measurement(&mut self, duration: Duration) {
        self.count += 1;
        self.total += duration;
        
        if duration < self.min {
            self.min = duration;
        }
        if duration > self.max {
            self.max = duration;
        }
        
        self.avg = Duration::from_nanos((self.total.as_nanos() / self.count as u128) as u64);
        
        // Keep recent measurements for percentile calculation
        self.recent.push(duration);
        if self.recent.len() > 1000 {
            self.recent.drain(0..100); // Keep only last 900
        }
        
        self.calculate_percentiles();
    }

    /// Calculate percentiles from recent measurements
    fn calculate_percentiles(&mut self) {
        if self.recent.is_empty() {
            return;
        }
        
        let mut sorted = self.recent.clone();
        sorted.sort();
        
        let len = sorted.len();
        self.p50 = sorted[len * 50 / 100];
        self.p95 = sorted[len * 95 / 100];
        self.p99 = sorted[len * 99 / 100];
    }
}

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Disk usage in bytes
    pub disk_usage: u64,
    /// Number of open file descriptors
    pub open_files: u64,
    /// Number of threads
    pub thread_count: u64,
}

impl SystemMetrics {
    /// Create new system metrics
    pub fn new() -> Self {
        Self {
            memory_usage: 0,
            cpu_usage: 0.0,
            disk_usage: 0,
            open_files: 0,
            thread_count: 0,
        }
    }

    /// Update system metrics
    pub fn update(&mut self) {
        // In a real implementation, this would read from system APIs
        // For now, we'll use placeholder values
        self.memory_usage = get_memory_usage();
        self.cpu_usage = get_cpu_usage();
        self.disk_usage = get_disk_usage();
        self.open_files = get_open_files();
        self.thread_count = get_thread_count();
    }
}

/// Get current memory usage
fn get_memory_usage() -> u64 {
    // Placeholder implementation
    // In a real implementation, this would read from /proc/self/status or similar
    1024 * 1024 * 100 // 100MB placeholder
}

/// Get current CPU usage
fn get_cpu_usage() -> f64 {
    // Placeholder implementation
    // In a real implementation, this would read from /proc/stat or similar
    25.0 // 25% placeholder
}

/// Get current disk usage
fn get_disk_usage() -> u64 {
    // Placeholder implementation
    // In a real implementation, this would read from statvfs or similar
    1024 * 1024 * 1024 * 10 // 10GB placeholder
}

/// Get number of open file descriptors
fn get_open_files() -> u64 {
    // Placeholder implementation
    // In a real implementation, this would read from /proc/self/fd
    50 // 50 files placeholder
}

/// Get number of threads
fn get_thread_count() -> u64 {
    // Placeholder implementation
    // In a real implementation, this would read from /proc/self/status
    8 // 8 threads placeholder
}

/// Performance measurement helper
pub struct PerformanceTimer {
    operation: String,
    start: Instant,
    collector: Arc<MetricsCollector>,
}

impl PerformanceTimer {
    /// Start timing an operation
    pub fn start(operation: &str, collector: Arc<MetricsCollector>) -> Self {
        Self {
            operation: operation.to_string(),
            start: Instant::now(),
            collector,
        }
    }

    /// Finish timing and record the measurement
    pub async fn finish(self) {
        let duration = self.start.elapsed();
        self.collector.record_timing(&self.operation, duration).await;
    }
}

/// Macro for easy performance measurement
#[macro_export]
macro_rules! measure_performance {
    ($operation:expr, $collector:expr, $code:block) => {{
        let timer = $crate::performance::PerformanceTimer::start($operation, $collector);
        let result = $code;
        timer.finish().await;
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        
        // Test timing recording
        collector.record_timing("test_operation", Duration::from_millis(100)).await;
        collector.record_timing("test_operation", Duration::from_millis(200)).await;
        
        // Test counter increment
        collector.increment_counter("test_counter", 5).await;
        collector.increment_counter("test_counter", 3).await;
        
        // Test gauge recording
        collector.record_gauge("test_gauge", 42.5).await;
        
        let metrics = collector.get_metrics().await;
        
        // Verify timing stats
        let timing_stats = metrics.get_timing_stats("test_operation").unwrap();
        assert_eq!(timing_stats.count, 2);
        assert_eq!(timing_stats.total, Duration::from_millis(300));
        assert_eq!(timing_stats.min, Duration::from_millis(100));
        assert_eq!(timing_stats.max, Duration::from_millis(200));
        
        // Verify counter
        assert_eq!(metrics.get_counter("test_counter"), Some(8));
        
        // Verify gauge
        assert_eq!(metrics.get_gauge("test_gauge"), Some(42.5));
    }

    #[tokio::test]
    async fn test_performance_timer() {
        let collector = Arc::new(MetricsCollector::new());
        
        let timer = PerformanceTimer::start("test_timer", collector.clone());
        tokio::time::sleep(Duration::from_millis(50)).await;
        timer.finish().await;
        
        let metrics = collector.get_metrics().await;
        let timing_stats = metrics.get_timing_stats("test_timer").unwrap();
        assert_eq!(timing_stats.count, 1);
        assert!(timing_stats.total >= Duration::from_millis(50));
    }

    #[test]
    fn test_timing_stats() {
        let mut stats = TimingStats::new();
        
        stats.add_measurement(Duration::from_millis(100));
        stats.add_measurement(Duration::from_millis(200));
        stats.add_measurement(Duration::from_millis(300));
        
        assert_eq!(stats.count, 3);
        assert_eq!(stats.total, Duration::from_millis(600));
        assert_eq!(stats.min, Duration::from_millis(100));
        assert_eq!(stats.max, Duration::from_millis(300));
        assert_eq!(stats.avg, Duration::from_millis(200));
    }
}