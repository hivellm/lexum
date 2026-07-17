//! Performance profiler for detailed analysis

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Performance profiler for detailed analysis
#[derive(Debug, Clone)]
pub struct Profiler {
    profiles: Arc<RwLock<HashMap<String, Profile>>>,
    enabled: bool,
}

impl Profiler {
    /// Create new profiler
    pub fn new() -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            enabled: true,
        }
    }

    /// Create profiler with enabled/disabled state
    pub fn with_enabled(enabled: bool) -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            enabled,
        }
    }

    /// Start profiling an operation
    pub async fn start_profile(&self, name: &str) -> Option<ProfileHandle> {
        if !self.enabled {
            return None;
        }

        let mut profiles = self.profiles.write().await;
        let profile = profiles
            .entry(name.to_string())
            .or_insert_with(Profile::new);
        profile.start();

        Some(ProfileHandle {
            name: name.to_string(),
            start_time: Instant::now(),
            profiler: self.clone(),
        })
    }

    /// Get profile for an operation
    pub async fn get_profile(&self, name: &str) -> Option<Profile> {
        let profiles = self.profiles.read().await;
        profiles.get(name).cloned()
    }

    /// Get all profiles
    pub async fn get_all_profiles(&self) -> HashMap<String, Profile> {
        self.profiles.read().await.clone()
    }

    /// Clear all profiles
    pub async fn clear(&self) {
        let mut profiles = self.profiles.write().await;
        profiles.clear();
    }

    /// Enable/disable profiling
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if profiling is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Profile handle for tracking a single operation
#[derive(Debug)]
pub struct ProfileHandle {
    name: String,
    start_time: Instant,
    profiler: Profiler,
}

impl ProfileHandle {
    /// Finish profiling and record the measurement
    pub async fn finish(self) {
        let duration = self.start_time.elapsed();
        let mut profiles = self.profiler.profiles.write().await;
        if let Some(profile) = profiles.get_mut(&self.name) {
            profile.add_measurement(duration);
        }
    }
}

/// Detailed profile for an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Operation name
    pub name: String,
    /// Number of measurements
    pub count: u64,
    /// Total time
    pub total_time: Duration,
    /// Average time
    pub avg_time: Duration,
    /// Minimum time
    pub min_time: Duration,
    /// Maximum time
    pub max_time: Duration,
    /// Standard deviation
    pub std_dev: Duration,
    /// All measurements (for detailed analysis)
    pub measurements: Vec<Duration>,
    /// Currently running
    pub is_running: bool,
}

impl Profile {
    /// Create new profile
    pub fn new() -> Self {
        Self {
            name: String::new(),
            count: 0,
            total_time: Duration::ZERO,
            avg_time: Duration::ZERO,
            min_time: Duration::MAX,
            max_time: Duration::ZERO,
            std_dev: Duration::ZERO,
            measurements: Vec::new(),
            is_running: false,
        }
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self::new()
    }
}

impl Profile {
    /// Start timing
    pub fn start(&mut self) {
        self.is_running = true;
    }

    /// Add a measurement
    pub fn add_measurement(&mut self, duration: Duration) {
        self.count += 1;
        self.total_time += duration;

        if duration < self.min_time {
            self.min_time = duration;
        }
        if duration > self.max_time {
            self.max_time = duration;
        }

        self.avg_time =
            Duration::from_nanos((self.total_time.as_nanos() / u128::from(self.count)) as u64);

        // Keep measurements for analysis
        self.measurements.push(duration);
        if self.measurements.len() > 10000 {
            self.measurements.drain(0..1000); // Keep only last 9000
        }

        self.calculate_std_dev();
        self.is_running = false;
    }

    /// Calculate standard deviation
    fn calculate_std_dev(&mut self) {
        if self.measurements.len() < 2 {
            self.std_dev = Duration::ZERO;
            return;
        }

        let avg_nanos = self.avg_time.as_nanos() as f64;
        let variance = self
            .measurements
            .iter()
            .map(|&d| {
                let diff = d.as_nanos() as f64 - avg_nanos;
                diff * diff
            })
            .sum::<f64>()
            / (self.measurements.len() - 1) as f64;

        let std_dev_nanos = variance.sqrt() as u64;
        self.std_dev = Duration::from_nanos(std_dev_nanos);
    }

    /// Get percentile value
    pub fn get_percentile(&self, percentile: f64) -> Duration {
        if self.measurements.is_empty() {
            return Duration::ZERO;
        }

        let mut sorted = self.measurements.clone();
        sorted.sort();

        let index = ((percentile / 100.0) * (sorted.len() - 1) as f64) as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    /// Get P50 (median)
    pub fn get_p50(&self) -> Duration {
        self.get_percentile(50.0)
    }

    /// Get P95
    pub fn get_p95(&self) -> Duration {
        self.get_percentile(95.0)
    }

    /// Get P99
    pub fn get_p99(&self) -> Duration {
        self.get_percentile(99.0)
    }

    /// Get throughput (operations per second)
    pub fn get_throughput(&self) -> f64 {
        if self.avg_time.is_zero() {
            0.0
        } else {
            1.0 / self.avg_time.as_secs_f64()
        }
    }
}

/// Profiler context for automatic profiling
pub struct ProfilerContext {
    profiler: Profiler,
    operation: String,
}

impl ProfilerContext {
    /// Create new profiler context
    pub fn new(profiler: Profiler, operation: &str) -> Self {
        Self {
            profiler,
            operation: operation.to_string(),
        }
    }

    /// Start profiling
    pub async fn start(self) -> Option<ProfileHandle> {
        self.profiler.start_profile(&self.operation).await
    }
}

/// Macro for easy profiling
#[macro_export]
macro_rules! profile_operation {
    ($profiler:expr, $operation:expr, $code:block) => {{
        if let Some(mut handle) = $profiler.start_profile($operation).await {
            let result = $code;
            handle.finish().await;
            result
        } else {
            $code
        }
    }};
}

/// Performance analysis tools
pub struct PerformanceAnalyzer;

impl PerformanceAnalyzer {
    /// Analyze profiles and generate recommendations
    pub fn analyze_profiles(profiles: &HashMap<String, Profile>) -> AnalysisReport {
        let mut report = AnalysisReport::new();

        for (name, profile) in profiles {
            if profile.count < 10 {
                continue; // Skip profiles with too few samples
            }

            // Check for performance issues
            if profile.avg_time > Duration::from_millis(100) {
                report.slow_operations.push(SlowOperation {
                    name: name.clone(),
                    avg_time: profile.avg_time,
                    count: profile.count,
                    recommendation: "Consider optimizing this operation".to_string(),
                });
            }

            // Check for high variance
            let coefficient_of_variation = if profile.avg_time.is_zero() {
                0.0
            } else {
                profile.std_dev.as_secs_f64() / profile.avg_time.as_secs_f64()
            };

            if coefficient_of_variation > 0.5 {
                report.inconsistent_operations.push(InconsistentOperation {
                    name: name.clone(),
                    std_dev: profile.std_dev,
                    coefficient_of_variation,
                    recommendation: "High variance detected, investigate for bottlenecks"
                        .to_string(),
                });
            }

            // Check for memory usage patterns
            if profile.count > 1000 && profile.avg_time > Duration::from_millis(10) {
                report
                    .high_frequency_operations
                    .push(HighFrequencyOperation {
                        name: name.clone(),
                        count: profile.count,
                        avg_time: profile.avg_time,
                        total_time: profile.total_time,
                        recommendation: "High frequency operation, consider caching or batching"
                            .to_string(),
                    });
            }
        }

        report
    }
}

/// Analysis report containing performance analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    /// Operations that are running slowly
    pub slow_operations: Vec<SlowOperation>,
    /// Operations with high variance in execution time
    pub inconsistent_operations: Vec<InconsistentOperation>,
    /// Operations that are called very frequently
    pub high_frequency_operations: Vec<HighFrequencyOperation>,
}

impl AnalysisReport {
    /// Create new empty analysis report
    pub fn new() -> Self {
        Self {
            slow_operations: Vec::new(),
            inconsistent_operations: Vec::new(),
            high_frequency_operations: Vec::new(),
        }
    }
}

impl Default for AnalysisReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a slow operation identified during performance analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowOperation {
    /// Name of the operation
    pub name: String,
    /// Average execution time
    pub avg_time: Duration,
    /// Number of times this operation was called
    pub count: u64,
    /// Recommendation for improving performance
    pub recommendation: String,
}

/// Represents an operation with inconsistent execution times
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InconsistentOperation {
    /// Name of the operation
    pub name: String,
    /// Standard deviation of execution times
    pub std_dev: Duration,
    /// Coefficient of variation (std_dev / mean)
    pub coefficient_of_variation: f64,
    /// Recommendation for improving consistency
    pub recommendation: String,
}

/// Represents a high-frequency operation that may benefit from optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighFrequencyOperation {
    /// Name of the operation
    pub name: String,
    /// Number of times this operation was called
    pub count: u64,
    /// Average execution time
    pub avg_time: Duration,
    /// Total time spent in this operation
    pub total_time: Duration,
    /// Recommendation for optimization
    pub recommendation: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[lexum_macros::tokio_test]
    async fn test_profiler() {
        let profiler = Profiler::new();

        // Profile an operation
        if let Some(handle) = profiler.start_profile("test_operation").await {
            tokio::time::sleep(Duration::from_millis(50)).await;
            handle.finish().await;
        }

        let profile = profiler.get_profile("test_operation").await.unwrap();
        assert_eq!(profile.count, 1);
        assert!(profile.avg_time >= Duration::from_millis(50));
    }

    #[lexum_macros::tokio_test]
    #[ignore = "Slow test - performs multiple long-running operations"]
    async fn test_profile_analysis() {
        let profiler = Profiler::new();

        // Add multiple measurements with longer durations to trigger slow operation detection
        for i in 0..20 {
            if let Some(handle) = profiler.start_profile("test_analysis").await {
                tokio::time::sleep(Duration::from_millis(150 + i * 10)).await; // 150ms+ to trigger slow operation
                handle.finish().await;
            }
        }

        let profiles = profiler.get_all_profiles().await;
        let report = PerformanceAnalyzer::analyze_profiles(&profiles);

        // Should have some analysis results
        assert!(!report.slow_operations.is_empty() || !report.inconsistent_operations.is_empty());
    }

    #[test]
    fn test_profile_calculations() {
        let mut profile = Profile::new();

        profile.add_measurement(Duration::from_millis(100));
        profile.add_measurement(Duration::from_millis(200));
        profile.add_measurement(Duration::from_millis(300));

        assert_eq!(profile.count, 3);
        assert_eq!(profile.total_time, Duration::from_millis(600));
        assert_eq!(profile.avg_time, Duration::from_millis(200));
        assert_eq!(profile.min_time, Duration::from_millis(100));
        assert_eq!(profile.max_time, Duration::from_millis(300));

        let p50 = profile.get_p50();
        assert!(p50 >= Duration::from_millis(100) && p50 <= Duration::from_millis(300));
    }

    #[test]
    fn test_profiler_with_enabled() {
        let profiler = Profiler::with_enabled(false);
        assert!(!profiler.is_enabled());

        let profiler = Profiler::with_enabled(true);
        assert!(profiler.is_enabled());
    }

    #[test]
    fn test_profiler_set_enabled() {
        let mut profiler = Profiler::new();
        assert!(profiler.is_enabled());

        profiler.set_enabled(false);
        assert!(!profiler.is_enabled());

        profiler.set_enabled(true);
        assert!(profiler.is_enabled());
    }

    #[lexum_macros::tokio_test]
    async fn test_profiler_disabled() {
        let profiler = Profiler::with_enabled(false);
        let handle = profiler.start_profile("test").await;
        assert!(handle.is_none());
    }

    #[lexum_macros::tokio_test]
    async fn test_profiler_get_profile_not_found() {
        let profiler = Profiler::new();
        let profile = profiler.get_profile("nonexistent").await;
        assert!(profile.is_none());
    }

    #[lexum_macros::tokio_test]
    async fn test_profiler_clear() {
        let profiler = Profiler::new();

        if let Some(handle) = profiler.start_profile("test").await {
            handle.finish().await;
        }

        assert!(profiler.get_profile("test").await.is_some());

        profiler.clear().await;

        assert!(profiler.get_profile("test").await.is_none());
        let profiles = profiler.get_all_profiles().await;
        assert!(profiles.is_empty());
    }

    #[test]
    fn test_profile_start() {
        let mut profile = Profile::new();
        assert!(!profile.is_running);

        profile.start();
        assert!(profile.is_running);
    }

    #[test]
    fn test_profile_add_measurement_updates_running() {
        let mut profile = Profile::new();
        profile.start();
        assert!(profile.is_running);

        profile.add_measurement(Duration::from_millis(100));
        assert!(!profile.is_running);
    }

    #[test]
    fn test_profile_get_percentile() {
        let mut profile = Profile::new();

        // Empty profile
        assert_eq!(profile.get_percentile(50.0), Duration::ZERO);

        // Add measurements
        for i in 1..=10 {
            profile.add_measurement(Duration::from_millis(i * 10));
        }

        let p50 = profile.get_p50();
        assert!(p50 >= Duration::from_millis(50) && p50 <= Duration::from_millis(60));

        let p95 = profile.get_p95();
        assert!(p95 >= Duration::from_millis(90) && p95 <= Duration::from_millis(100));

        let p99 = profile.get_p99();
        assert!(p99 >= Duration::from_millis(90) && p99 <= Duration::from_millis(100));
    }

    #[test]
    fn test_profile_get_throughput() {
        let mut profile = Profile::new();

        // Zero avg_time
        assert_eq!(profile.get_throughput(), 0.0);

        // Add measurement
        profile.add_measurement(Duration::from_millis(100));

        // Throughput should be approximately 10 ops/sec (1 / 0.1s)
        let throughput = profile.get_throughput();
        assert!(throughput > 9.0 && throughput < 11.0);
    }

    #[test]
    fn test_profile_std_dev_calculation() {
        let mut profile = Profile::new();

        // Single measurement - std dev should be zero
        profile.add_measurement(Duration::from_millis(100));
        assert_eq!(profile.std_dev, Duration::ZERO);

        // Multiple measurements - should have std dev
        profile.add_measurement(Duration::from_millis(200));
        assert!(profile.std_dev > Duration::ZERO);
    }

    #[test]
    fn test_profile_measurements_limit() {
        let mut profile = Profile::new();

        // Add more than 10000 measurements
        for _ in 0..11000 {
            profile.add_measurement(Duration::from_millis(100));
        }

        // Should keep only last 9000 (after draining first 1000 when exceeding 10000)
        // After 10000, it drains 1000, leaving 9000
        // After 11000, it should have drained multiple times
        assert!(profile.measurements.len() <= 10000);
    }

    #[lexum_macros::tokio_test]
    async fn test_profiler_context() {
        let profiler = Profiler::new();
        let context = ProfilerContext::new(profiler, "test_operation");

        let handle = context.start().await;
        assert!(handle.is_some());

        if let Some(h) = handle {
            h.finish().await;
        }
    }

    #[test]
    fn test_performance_analyzer_with_few_samples() {
        let mut profiles = HashMap::new();
        let mut profile = Profile::new();
        profile.count = 5; // Less than 10
        profiles.insert("test".to_string(), profile);

        let report = PerformanceAnalyzer::analyze_profiles(&profiles);

        // Should skip profiles with < 10 samples
        assert!(report.slow_operations.is_empty());
        assert!(report.inconsistent_operations.is_empty());
        assert!(report.high_frequency_operations.is_empty());
    }

    #[test]
    fn test_performance_analyzer_slow_operation() {
        let mut profiles = HashMap::new();
        let mut profile = Profile::new();
        profile.count = 20;
        profile.avg_time = Duration::from_millis(200); // > 100ms
        profiles.insert("slow_op".to_string(), profile);

        let report = PerformanceAnalyzer::analyze_profiles(&profiles);

        assert_eq!(report.slow_operations.len(), 1);
        assert_eq!(report.slow_operations[0].name, "slow_op");
    }

    #[test]
    fn test_performance_analyzer_inconsistent_operation() {
        let mut profiles = HashMap::new();
        let mut profile = Profile::new();
        profile.count = 20;
        profile.avg_time = Duration::from_millis(100);
        profile.std_dev = Duration::from_millis(60); // CV = 0.6 > 0.5
        profiles.insert("inconsistent_op".to_string(), profile);

        let report = PerformanceAnalyzer::analyze_profiles(&profiles);

        assert_eq!(report.inconsistent_operations.len(), 1);
        assert_eq!(report.inconsistent_operations[0].name, "inconsistent_op");
    }

    #[test]
    fn test_performance_analyzer_high_frequency_operation() {
        let mut profiles = HashMap::new();
        let mut profile = Profile::new();
        profile.count = 2000; // > 1000
        profile.avg_time = Duration::from_millis(20); // > 10ms
        profile.total_time = Duration::from_secs(40);
        profiles.insert("frequent_op".to_string(), profile);

        let report = PerformanceAnalyzer::analyze_profiles(&profiles);

        assert_eq!(report.high_frequency_operations.len(), 1);
        assert_eq!(report.high_frequency_operations[0].name, "frequent_op");
    }

    #[test]
    fn test_analysis_report_new() {
        let report = AnalysisReport::new();
        assert!(report.slow_operations.is_empty());
        assert!(report.inconsistent_operations.is_empty());
        assert!(report.high_frequency_operations.is_empty());
    }

    #[test]
    fn test_slow_operation_fields() {
        let op = SlowOperation {
            name: "test".to_string(),
            avg_time: Duration::from_millis(100),
            count: 10,
            recommendation: "Optimize".to_string(),
        };

        assert_eq!(op.name, "test");
        assert_eq!(op.avg_time, Duration::from_millis(100));
        assert_eq!(op.count, 10);
        assert_eq!(op.recommendation, "Optimize");
    }

    #[test]
    fn test_inconsistent_operation_fields() {
        let op = InconsistentOperation {
            name: "test".to_string(),
            std_dev: Duration::from_millis(50),
            coefficient_of_variation: 0.5,
            recommendation: "Investigate".to_string(),
        };

        assert_eq!(op.name, "test");
        assert_eq!(op.std_dev, Duration::from_millis(50));
        assert_eq!(op.coefficient_of_variation, 0.5);
        assert_eq!(op.recommendation, "Investigate");
    }

    #[test]
    fn test_high_frequency_operation_fields() {
        let op = HighFrequencyOperation {
            name: "test".to_string(),
            count: 1000,
            avg_time: Duration::from_millis(5),
            total_time: Duration::from_secs(5),
            recommendation: "Cache".to_string(),
        };

        assert_eq!(op.name, "test");
        assert_eq!(op.count, 1000);
        assert_eq!(op.avg_time, Duration::from_millis(5));
        assert_eq!(op.total_time, Duration::from_secs(5));
        assert_eq!(op.recommendation, "Cache");
    }
}
