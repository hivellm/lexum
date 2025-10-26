//! Performance monitoring and regression detection

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Performance metrics for a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Operation name
    pub operation: String,
    /// Duration of the operation
    pub duration: Duration,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Number of operations per second
    pub ops_per_second: f64,
    /// Timestamp when the operation was measured
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Performance baseline for comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaseline {
    /// Operation name
    pub operation: String,
    /// Average duration
    pub avg_duration: Duration,
    /// P95 duration
    pub p95_duration: Duration,
    /// P99 duration
    pub p99_duration: Duration,
    /// Average memory usage
    pub avg_memory_usage: u64,
    /// Average CPU usage
    pub avg_cpu_usage: f64,
    /// Average operations per second
    pub avg_ops_per_second: f64,
    /// Number of samples used for baseline
    pub sample_count: usize,
    /// Timestamp when baseline was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Performance regression detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionResult {
    /// Whether a regression was detected
    pub is_regression: bool,
    /// Regression severity (low, medium, high, critical)
    pub severity: RegressionSeverity,
    /// Performance degradation percentage
    pub degradation_percentage: f64,
    /// Baseline metrics
    pub baseline: PerformanceBaseline,
    /// Current metrics
    pub current: PerformanceMetrics,
    /// Regression details
    pub details: Vec<String>,
}

/// Regression severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegressionSeverity {
    /// Low severity regression (0-25% degradation)
    Low,
    /// Medium severity regression (25-50% degradation)
    Medium,
    /// High severity regression (50-100% degradation)
    High,
    /// Critical severity regression (100%+ degradation)
    Critical,
}

/// Performance monitor for tracking and detecting regressions
pub struct PerformanceMonitor {
    /// Current metrics for each operation
    current_metrics: HashMap<String, Vec<PerformanceMetrics>>,
    /// Baselines for each operation
    baselines: HashMap<String, PerformanceBaseline>,
    /// Maximum number of samples to keep per operation
    max_samples: usize,
    /// Regression thresholds
    thresholds: RegressionThresholds,
}

/// Regression detection thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionThresholds {
    /// Duration regression threshold (percentage)
    pub duration_threshold: f64,
    /// Memory regression threshold (percentage)
    pub memory_threshold: f64,
    /// CPU regression threshold (percentage)
    pub cpu_threshold: f64,
    /// OPS regression threshold (percentage)
    pub ops_threshold: f64,
}

impl Default for RegressionThresholds {
    fn default() -> Self {
        Self {
            duration_threshold: 20.0, // 20% degradation
            memory_threshold: 30.0,   // 30% degradation
            cpu_threshold: 25.0,      // 25% degradation
            ops_threshold: 15.0,      // 15% degradation
        }
    }
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(max_samples: usize) -> Self {
        Self {
            current_metrics: HashMap::new(),
            baselines: HashMap::new(),
            max_samples,
            thresholds: RegressionThresholds::default(),
        }
    }

    /// Record performance metrics for an operation
    pub fn record_metrics(&mut self, metrics: PerformanceMetrics) {
        let operation = metrics.operation.clone();
        let operation_metrics = self.current_metrics.entry(operation).or_default();

        operation_metrics.push(metrics);

        // Keep only the most recent samples
        if operation_metrics.len() > self.max_samples {
            operation_metrics.remove(0);
        }
    }

    /// Create a baseline from current metrics
    pub fn create_baseline(&mut self, operation: &str) -> Result<PerformanceBaseline> {
        let metrics = self
            .current_metrics
            .get(operation)
            .ok_or_else(|| anyhow::anyhow!("No metrics found for operation: {operation}"))?;

        if metrics.is_empty() {
            return Err(anyhow::anyhow!(
                "No metrics available for operation: {operation}"
            ));
        }

        let durations: Vec<Duration> = metrics.iter().map(|m| m.duration).collect();
        let memory_usages: Vec<u64> = metrics.iter().map(|m| m.memory_usage).collect();
        let cpu_usages: Vec<f64> = metrics.iter().map(|m| m.cpu_usage).collect();
        let ops_per_second: Vec<f64> = metrics.iter().map(|m| m.ops_per_second).collect();

        let avg_duration = Self::calculate_average_duration(&durations);
        let p95_duration = Self::calculate_percentile_duration(&durations, 95);
        let p99_duration = Self::calculate_percentile_duration(&durations, 99);
        let avg_memory_usage =
            memory_usages.iter().sum::<u64>() as f64 / memory_usages.len() as f64;
        let avg_cpu_usage = cpu_usages.iter().sum::<f64>() / cpu_usages.len() as f64;
        let avg_ops_per_second = ops_per_second.iter().sum::<f64>() / ops_per_second.len() as f64;

        let baseline = PerformanceBaseline {
            operation: operation.to_string(),
            avg_duration,
            p95_duration,
            p99_duration,
            avg_memory_usage: avg_memory_usage as u64,
            avg_cpu_usage,
            avg_ops_per_second,
            sample_count: metrics.len(),
            created_at: chrono::Utc::now(),
        };

        self.baselines
            .insert(operation.to_string(), baseline.clone());
        Ok(baseline)
    }

    /// Check for performance regressions
    pub fn check_regression(
        &self,
        operation: &str,
        current_metrics: &PerformanceMetrics,
    ) -> Option<RegressionResult> {
        let baseline = self.baselines.get(operation)?;

        let mut regressions = Vec::new();
        let mut max_degradation: f64 = 0.0;

        // Check duration regression
        let duration_degradation = Self::calculate_percentage_change(
            baseline.avg_duration.as_nanos() as f64,
            current_metrics.duration.as_nanos() as f64,
        );
        if duration_degradation > self.thresholds.duration_threshold {
            regressions.push(format!(
                "Duration regression: {:.1}% (baseline: {:?}, current: {:?})",
                duration_degradation, baseline.avg_duration, current_metrics.duration
            ));
            max_degradation = max_degradation.max(duration_degradation);
        }

        // Check memory regression
        let memory_degradation = Self::calculate_percentage_change(
            baseline.avg_memory_usage as f64,
            current_metrics.memory_usage as f64,
        );
        if memory_degradation > self.thresholds.memory_threshold {
            regressions.push(format!(
                "Memory regression: {:.1}% (baseline: {} bytes, current: {} bytes)",
                memory_degradation, baseline.avg_memory_usage, current_metrics.memory_usage
            ));
            max_degradation = max_degradation.max(memory_degradation);
        }

        // Check CPU regression
        let cpu_degradation =
            Self::calculate_percentage_change(baseline.avg_cpu_usage, current_metrics.cpu_usage);
        if cpu_degradation > self.thresholds.cpu_threshold {
            regressions.push(format!(
                "CPU regression: {:.1}% (baseline: {:.1}%, current: {:.1}%)",
                cpu_degradation, baseline.avg_cpu_usage, current_metrics.cpu_usage
            ));
            max_degradation = max_degradation.max(cpu_degradation);
        }

        // Check OPS regression
        let ops_degradation = Self::calculate_percentage_change(
            baseline.avg_ops_per_second,
            current_metrics.ops_per_second,
        );
        if ops_degradation > self.thresholds.ops_threshold {
            regressions.push(format!(
                "OPS regression: {:.1}% (baseline: {:.1}, current: {:.1})",
                ops_degradation, baseline.avg_ops_per_second, current_metrics.ops_per_second
            ));
            max_degradation = max_degradation.max(ops_degradation);
        }

        if regressions.is_empty() {
            return None;
        }

        let severity = Self::determine_severity(max_degradation);

        Some(RegressionResult {
            is_regression: true,
            severity,
            degradation_percentage: max_degradation,
            baseline: baseline.clone(),
            current: current_metrics.clone(),
            details: regressions,
        })
    }

    /// Get current metrics for an operation
    pub fn get_current_metrics(&self, operation: &str) -> Option<&Vec<PerformanceMetrics>> {
        self.current_metrics.get(operation)
    }

    /// Get baseline for an operation
    pub fn get_baseline(&self, operation: &str) -> Option<&PerformanceBaseline> {
        self.baselines.get(operation)
    }

    /// Update regression thresholds
    pub fn update_thresholds(&mut self, thresholds: RegressionThresholds) {
        self.thresholds = thresholds;
    }

    /// Calculate average duration from a list of durations
    fn calculate_average_duration(durations: &[Duration]) -> Duration {
        let total_nanos: u128 = durations.iter().map(|d| d.as_nanos()).sum();
        Duration::from_nanos((total_nanos / durations.len() as u128) as u64)
    }

    /// Calculate percentile duration from a list of durations
    fn calculate_percentile_duration(durations: &[Duration], percentile: u8) -> Duration {
        let mut sorted_durations = durations.to_vec();
        sorted_durations.sort();

        let index = ((f64::from(percentile) / 100.0) * (sorted_durations.len() - 1) as f64).round()
            as usize;
        sorted_durations[index.min(sorted_durations.len() - 1)]
    }

    /// Calculate percentage change between two values
    fn calculate_percentage_change(baseline: f64, current: f64) -> f64 {
        if baseline == 0.0 {
            return 0.0;
        }
        ((current - baseline) / baseline * 100.0).abs()
    }

    /// Determine regression severity based on degradation percentage
    fn determine_severity(degradation: f64) -> RegressionSeverity {
        match degradation {
            d if d >= 100.0 => RegressionSeverity::Critical,
            d if d >= 50.0 => RegressionSeverity::High,
            d if d >= 25.0 => RegressionSeverity::Medium,
            _ => RegressionSeverity::Low,
        }
    }
}

/// Performance measurement helper
pub struct PerformanceTimer {
    start_time: Instant,
    operation: String,
    memory_start: u64,
}

impl PerformanceTimer {
    /// Start timing an operation
    pub fn start(operation: String) -> Self {
        Self {
            start_time: Instant::now(),
            operation,
            memory_start: Self::get_memory_usage(),
        }
    }

    /// Finish timing and return metrics
    pub fn finish(self) -> PerformanceMetrics {
        let duration = self.start_time.elapsed();
        let memory_usage = Self::get_memory_usage() - self.memory_start;
        let ops_per_second = 1.0 / duration.as_secs_f64();

        PerformanceMetrics {
            operation: self.operation,
            duration,
            memory_usage,
            cpu_usage: Self::get_cpu_usage(),
            ops_per_second,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get current memory usage (simplified implementation)
    fn get_memory_usage() -> u64 {
        // This is a simplified implementation
        // In a real implementation, you would use system-specific APIs
        // or libraries like `sysinfo` to get actual memory usage
        0
    }

    /// Get current CPU usage (simplified implementation)
    fn get_cpu_usage() -> f64 {
        // This is a simplified implementation
        // In a real implementation, you would use system-specific APIs
        // or libraries like `sysinfo` to get actual CPU usage
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new(100);
        assert_eq!(monitor.max_samples, 100);
    }

    #[test]
    fn test_performance_metrics_recording() {
        let mut monitor = PerformanceMonitor::new(10);

        let metrics = PerformanceMetrics {
            operation: "test_operation".to_string(),
            duration: Duration::from_millis(100),
            memory_usage: 1024,
            cpu_usage: 50.0,
            ops_per_second: 10.0,
            timestamp: chrono::Utc::now(),
        };

        monitor.record_metrics(metrics);

        let recorded_metrics = monitor.get_current_metrics("test_operation");
        assert!(recorded_metrics.is_some());
        assert_eq!(recorded_metrics.unwrap().len(), 1);
    }

    #[test]
    fn test_baseline_creation() {
        let mut monitor = PerformanceMonitor::new(10);

        // Record some metrics
        for i in 0..5 {
            let metrics = PerformanceMetrics {
                operation: "test_operation".to_string(),
                duration: Duration::from_millis(100 + i * 10),
                memory_usage: 1024 + i * 100,
                cpu_usage: 50.0 + i as f64,
                ops_per_second: 10.0 - i as f64,
                timestamp: chrono::Utc::now(),
            };
            monitor.record_metrics(metrics);
        }

        let baseline = monitor.create_baseline("test_operation");
        assert!(baseline.is_ok());

        let baseline = baseline.unwrap();
        assert_eq!(baseline.operation, "test_operation");
        assert_eq!(baseline.sample_count, 5);
    }

    #[test]
    fn test_regression_detection() {
        let mut monitor = PerformanceMonitor::new(10);

        // Create baseline
        for _i in 0..5 {
            let metrics = PerformanceMetrics {
                operation: "test_operation".to_string(),
                duration: Duration::from_millis(100),
                memory_usage: 1024,
                cpu_usage: 50.0,
                ops_per_second: 10.0,
                timestamp: chrono::Utc::now(),
            };
            monitor.record_metrics(metrics);
        }

        let _baseline = monitor.create_baseline("test_operation").unwrap();

        // Test with regression
        let current_metrics = PerformanceMetrics {
            operation: "test_operation".to_string(),
            duration: Duration::from_millis(150), // 50% slower
            memory_usage: 2048,                   // 100% more memory
            cpu_usage: 75.0,                      // 50% more CPU
            ops_per_second: 5.0,                  // 50% fewer OPS
            timestamp: chrono::Utc::now(),
        };

        let regression = monitor.check_regression("test_operation", &current_metrics);
        assert!(regression.is_some());

        let regression = regression.unwrap();
        assert!(regression.is_regression);
        assert!(regression.degradation_percentage > 0.0);
    }

    #[test]
    fn test_performance_timer() {
        let timer = PerformanceTimer::start("test_operation".to_string());
        std::thread::sleep(Duration::from_millis(10));
        let metrics = timer.finish();

        assert_eq!(metrics.operation, "test_operation");
        assert!(metrics.duration >= Duration::from_millis(10));
    }
}
