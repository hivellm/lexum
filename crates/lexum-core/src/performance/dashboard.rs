//! Real-time performance monitoring dashboard

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::performance::{Metrics, MetricsCollector, Profile, Profiler};

/// Real-time performance dashboard
#[derive(Debug, Clone)]
pub struct PerformanceDashboard {
    metrics_collector: Arc<MetricsCollector>,
    profiler: Arc<Profiler>,
    dashboard_data: Arc<RwLock<DashboardData>>,
    update_interval: Duration,
    is_running: Arc<RwLock<bool>>,
}

/// Dashboard data containing current performance metrics and status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    /// Current metrics
    pub metrics: Metrics,
    /// Current profiles
    pub profiles: HashMap<String, Profile>,
    /// System health status
    pub health_status: HealthStatus,
    /// Performance alerts
    pub alerts: Vec<PerformanceAlert>,
    /// Last update time (nanoseconds since epoch)
    pub last_update: u64,
    /// Dashboard statistics
    pub stats: DashboardStats,
}

/// System health status with various health indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall health score (0-100)
    pub overall_score: u8,
    /// Search performance health
    pub search_health: u8,
    /// Memory health
    pub memory_health: u8,
    /// CPU health
    pub cpu_health: u8,
    /// Disk health
    pub disk_health: u8,
    /// Status message
    pub status: String,
}

/// Performance alert with level and message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    /// Alert level
    pub level: AlertLevel,
    /// Alert message
    pub message: String,
    /// Timestamp (nanoseconds since epoch)
    pub timestamp: u64,
    /// Related operation
    pub operation: Option<String>,
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    /// Informational alert
    Info,
    /// Warning alert
    Warning,
    /// Error alert
    Error,
    /// Critical alert
    Critical,
}

/// Dashboard statistics and performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    /// Total operations performed
    pub total_operations: u64,
    /// Operations per second
    pub ops_per_second: f64,
    /// Average response time
    pub avg_response_time: Duration,
    /// Error rate (percentage)
    pub error_rate: f64,
    /// Cache hit rate (percentage)
    pub cache_hit_rate: f64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// CPU usage (percentage)
    pub cpu_usage: f64,
}

impl PerformanceDashboard {
    /// Create new performance dashboard
    pub fn new(update_interval: Duration) -> Self {
        Self {
            metrics_collector: Arc::new(MetricsCollector::new()),
            profiler: Arc::new(Profiler::new()),
            dashboard_data: Arc::new(RwLock::new(DashboardData::new())),
            update_interval,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the dashboard
    pub async fn start(&self) {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return; // Already running
        }
        *is_running = true;
        drop(is_running);

        // Start update loop
        let dashboard_data = self.dashboard_data.clone();
        let metrics_collector = self.metrics_collector.clone();
        let profiler = self.profiler.clone();
        let update_interval = self.update_interval;
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            while *is_running.read().await {
                // Update dashboard data
                let metrics = metrics_collector.get_metrics().await;
                let profiles = profiler.get_all_profiles().await;

                let mut data = dashboard_data.write().await;
                data.update(metrics, profiles).await;
                drop(data);

                tokio::time::sleep(update_interval).await;
            }
        });
    }

    /// Stop the dashboard
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
    }

    /// Get current dashboard data
    pub async fn get_data(&self) -> DashboardData {
        self.dashboard_data.read().await.clone()
    }

    /// Get metrics collector
    pub fn metrics_collector(&self) -> Arc<MetricsCollector> {
        self.metrics_collector.clone()
    }

    /// Get profiler
    pub fn profiler(&self) -> Arc<Profiler> {
        self.profiler.clone()
    }

    /// Add a performance alert
    pub async fn add_alert(&self, level: AlertLevel, message: String, operation: Option<String>) {
        let mut data = self.dashboard_data.write().await;
        data.add_alert(level, message, operation);
    }

    /// Get performance alerts
    pub async fn get_alerts(&self) -> Vec<PerformanceAlert> {
        self.dashboard_data.read().await.alerts.clone()
    }

    /// Clear old alerts
    pub async fn clear_old_alerts(&self, older_than: Duration) {
        let mut data = self.dashboard_data.write().await;
        data.clear_old_alerts(older_than);
    }
}

impl DashboardData {
    /// Create new dashboard data
    pub fn new() -> Self {
        Self {
            metrics: Metrics::new(),
            profiles: HashMap::new(),
            health_status: HealthStatus::new(),
            alerts: Vec::new(),
            last_update: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            stats: DashboardStats::new(),
        }
    }
}

impl Default for DashboardData {
    fn default() -> Self {
        Self::new()
    }
}

impl DashboardData {
    /// Update dashboard data
    pub async fn update(&mut self, metrics: Metrics, profiles: HashMap<String, Profile>) {
        self.metrics = metrics;
        self.profiles = profiles;
        self.last_update = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        // Update health status
        self.update_health_status();

        // Update statistics
        self.update_stats();

        // Check for alerts
        self.check_alerts();
    }

    /// Update health status
    fn update_health_status(&mut self) {
        let mut health = HealthStatus::new();

        // Calculate search health based on response times
        if let Some(search_timing) = self.metrics.get_timing_stats("search") {
            let avg_ms = search_timing.avg.as_millis() as u64;
            health.search_health = if avg_ms < 10 {
                100
            } else if avg_ms < 50 {
                80
            } else if avg_ms < 100 {
                60
            } else if avg_ms < 500 {
                40
            } else {
                20
            };
        }

        // Calculate memory health
        let memory_mb = self.metrics.system.memory_usage / 1024 / 1024;
        health.memory_health = if memory_mb < 100 {
            100
        } else if memory_mb < 500 {
            80
        } else if memory_mb < 1000 {
            60
        } else if memory_mb < 2000 {
            40
        } else {
            20
        };

        // Calculate CPU health
        health.cpu_health = if self.metrics.system.cpu_usage < 50.0 {
            100
        } else if self.metrics.system.cpu_usage < 70.0 {
            80
        } else if self.metrics.system.cpu_usage < 85.0 {
            60
        } else if self.metrics.system.cpu_usage < 95.0 {
            40
        } else {
            20
        };

        // Calculate disk health
        let disk_gb = self.metrics.system.disk_usage / 1024 / 1024 / 1024;
        health.disk_health = if disk_gb < 1 {
            100
        } else if disk_gb < 5 {
            80
        } else if disk_gb < 10 {
            60
        } else if disk_gb < 20 {
            40
        } else {
            20
        };

        // Calculate overall health
        health.overall_score = ((u16::from(health.search_health)
            + u16::from(health.memory_health)
            + u16::from(health.cpu_health)
            + u16::from(health.disk_health))
            / 4) as u8;

        // Set status message
        health.status = match health.overall_score {
            90..=100 => "Excellent".to_string(),
            70..=89 => "Good".to_string(),
            50..=69 => "Fair".to_string(),
            30..=49 => "Poor".to_string(),
            _ => "Critical".to_string(),
        };

        self.health_status = health;
    }

    /// Update statistics
    fn update_stats(&mut self) {
        let mut stats = DashboardStats::new();

        // Calculate total operations
        stats.total_operations = self.profiles.values().map(|p| p.count).sum();

        // Calculate operations per second
        if let Some(search_timing) = self.metrics.get_timing_stats("search") {
            stats.ops_per_second = if search_timing.avg.is_zero() {
                0.0
            } else {
                1.0 / search_timing.avg.as_secs_f64()
            };
            stats.avg_response_time = search_timing.avg;
        }

        // Calculate error rate (placeholder)
        stats.error_rate = 0.0; // Would be calculated from error counters

        // Calculate cache hit rate
        let cache_hits = self.metrics.get_counter("cache_hits").unwrap_or(0);
        let cache_misses = self.metrics.get_counter("cache_misses").unwrap_or(0);
        let total_cache_ops = cache_hits + cache_misses;

        if total_cache_ops > 0 {
            stats.cache_hit_rate = (cache_hits as f64 / total_cache_ops as f64) * 100.0;
        }

        // System metrics
        stats.memory_usage = self.metrics.system.memory_usage;
        stats.cpu_usage = self.metrics.system.cpu_usage;

        self.stats = stats;
    }

    /// Check for performance alerts
    fn check_alerts(&mut self) {
        // Check for slow operations
        let slow_operations: Vec<_> = self
            .profiles
            .iter()
            .filter(|(_, profile)| profile.avg_time > Duration::from_millis(1000))
            .map(|(name, profile)| (name.clone(), profile.avg_time))
            .collect();

        for (name, avg_time) in slow_operations {
            self.add_alert(
                AlertLevel::Warning,
                format!(
                    "Slow operation detected: {} (avg: {:.2}ms)",
                    name,
                    avg_time.as_secs_f64() * 1000.0
                ),
                Some(name),
            );
        }

        // Check for high memory usage
        if self.metrics.system.memory_usage > 1024 * 1024 * 1024 {
            // 1GB
            self.add_alert(
                AlertLevel::Warning,
                format!(
                    "High memory usage: {} MB",
                    self.metrics.system.memory_usage / 1024 / 1024
                ),
                None,
            );
        }

        // Check for high CPU usage
        if self.metrics.system.cpu_usage > 90.0 {
            self.add_alert(
                AlertLevel::Error,
                format!("High CPU usage: {:.1}%", self.metrics.system.cpu_usage),
                None,
            );
        }

        // Check for low health score
        if self.health_status.overall_score < 50 {
            self.add_alert(
                AlertLevel::Critical,
                format!(
                    "System health critical: {}%",
                    self.health_status.overall_score
                ),
                None,
            );
        }
    }

    /// Add an alert
    fn add_alert(&mut self, level: AlertLevel, message: String, operation: Option<String>) {
        let alert = PerformanceAlert {
            level,
            message,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            operation,
        };

        self.alerts.push(alert);

        // Keep only last 100 alerts
        if self.alerts.len() > 100 {
            self.alerts.drain(0..self.alerts.len() - 100);
        }
    }

    /// Clear old alerts
    fn clear_old_alerts(&mut self, older_than: Duration) {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
            - older_than.as_nanos() as u64;
        self.alerts.retain(|alert| alert.timestamp > cutoff);
    }
}

impl HealthStatus {
    /// Create new health status
    pub fn new() -> Self {
        Self {
            overall_score: 100,
            search_health: 100,
            memory_health: 100,
            cpu_health: 100,
            disk_health: 100,
            status: "Excellent".to_string(),
        }
    }
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::new()
    }
}

impl DashboardStats {
    /// Create new dashboard stats
    pub fn new() -> Self {
        Self {
            total_operations: 0,
            ops_per_second: 0.0,
            avg_response_time: Duration::ZERO,
            error_rate: 0.0,
            cache_hit_rate: 0.0,
            memory_usage: 0,
            cpu_usage: 0.0,
        }
    }
}

impl Default for DashboardStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance monitoring service
pub struct PerformanceMonitoringService {
    dashboard: PerformanceDashboard,
}

impl PerformanceMonitoringService {
    /// Create new monitoring service
    pub fn new() -> Self {
        Self {
            dashboard: PerformanceDashboard::new(Duration::from_secs(1)),
        }
    }

    /// Start monitoring
    pub async fn start(&self) {
        self.dashboard.start().await;
    }

    /// Stop monitoring
    pub async fn stop(&self) {
        self.dashboard.stop().await;
    }

    /// Get dashboard
    pub fn dashboard(&self) -> &PerformanceDashboard {
        &self.dashboard
    }

    /// Get metrics collector
    pub fn metrics_collector(&self) -> Arc<MetricsCollector> {
        self.dashboard.metrics_collector()
    }

    /// Get profiler
    pub fn profiler(&self) -> Arc<Profiler> {
        self.dashboard.profiler()
    }
}

impl Default for PerformanceMonitoringService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[lexum_macros::tokio_test]
    async fn test_performance_dashboard() {
        let dashboard = PerformanceDashboard::new(Duration::from_millis(100));

        // Start dashboard
        dashboard.start().await;

        // Wait a bit for updates
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Get data
        let data = dashboard.get_data().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        assert!(data.last_update > now - 1_000_000_000); // 1 second in nanoseconds

        // Stop dashboard
        dashboard.stop().await;
    }

    #[lexum_macros::tokio_test]
    async fn test_health_status_calculation() {
        let mut data = DashboardData::new();

        // Test with good metrics
        data.metrics.system.memory_usage = 50 * 1024 * 1024; // 50MB
        data.metrics.system.cpu_usage = 30.0;

        data.update_health_status();

        assert!(data.health_status.overall_score > 70);
        assert_eq!(data.health_status.memory_health, 100);
        assert_eq!(data.health_status.cpu_health, 100);
    }

    #[lexum_macros::tokio_test]
    async fn test_alert_system() {
        let dashboard = PerformanceDashboard::new(Duration::from_secs(1));

        // Add an alert
        dashboard
            .add_alert(
                AlertLevel::Warning,
                "Test alert".to_string(),
                Some("test_operation".to_string()),
            )
            .await;

        let alerts = dashboard.get_alerts().await;
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].level, AlertLevel::Warning);
        assert_eq!(alerts[0].message, "Test alert");
    }

    #[lexum_macros::tokio_test]
    async fn test_dashboard_stop_start() {
        let dashboard = PerformanceDashboard::new(Duration::from_millis(50));

        // Start dashboard
        dashboard.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Stop dashboard
        dashboard.stop().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Start again
        dashboard.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        dashboard.stop().await;
    }

    #[lexum_macros::tokio_test]
    async fn test_dashboard_get_metrics_collector() {
        let dashboard = PerformanceDashboard::new(Duration::from_secs(1));
        let collector = dashboard.metrics_collector();
        assert!(collector.get_metrics().await.timings.is_empty());
    }

    #[lexum_macros::tokio_test]
    async fn test_dashboard_get_profiler() {
        let dashboard = PerformanceDashboard::new(Duration::from_secs(1));
        let profiler = dashboard.profiler();
        let profiles = profiler.get_all_profiles().await;
        assert!(profiles.is_empty());
    }

    #[lexum_macros::tokio_test]
    async fn test_dashboard_clear_old_alerts() {
        let dashboard = PerformanceDashboard::new(Duration::from_secs(1));

        // Add an alert
        dashboard
            .add_alert(AlertLevel::Info, "Old alert".to_string(), None)
            .await;

        // Clear alerts older than 1 second
        tokio::time::sleep(Duration::from_millis(1100)).await;
        dashboard.clear_old_alerts(Duration::from_secs(1)).await;

        let alerts = dashboard.get_alerts().await;
        assert_eq!(alerts.len(), 0);
    }

    #[test]
    fn test_health_status_new() {
        let health = HealthStatus::new();
        assert_eq!(health.overall_score, 100);
        assert_eq!(health.search_health, 100);
        assert_eq!(health.memory_health, 100);
        assert_eq!(health.cpu_health, 100);
        assert_eq!(health.disk_health, 100);
        assert_eq!(health.status, "Excellent");
    }

    #[test]
    fn test_dashboard_stats_new() {
        let stats = DashboardStats::new();
        assert_eq!(stats.total_operations, 0);
        assert_eq!(stats.ops_per_second, 0.0);
        assert_eq!(stats.avg_response_time, Duration::ZERO);
        assert_eq!(stats.error_rate, 0.0);
        assert_eq!(stats.cache_hit_rate, 0.0);
        assert_eq!(stats.memory_usage, 0);
        assert_eq!(stats.cpu_usage, 0.0);
    }

    #[lexum_macros::tokio_test]
    async fn test_dashboard_data_update_with_profiles() {
        let mut data = DashboardData::new();
        let mut metrics = Metrics::new();
        metrics.system.memory_usage = 200 * 1024 * 1024; // 200MB
        metrics.system.cpu_usage = 60.0;

        let mut profiles = HashMap::new();
        let mut profile = Profile::new();
        profile.name = "search".to_string();
        profile.add_measurement(Duration::from_millis(50));
        profiles.insert("search".to_string(), profile);

        data.update(metrics, profiles).await;

        assert_eq!(data.profiles.len(), 1);
        assert!(data.last_update > 0);
    }

    #[lexum_macros::tokio_test]
    async fn test_dashboard_data_update_health_status_poor() {
        let mut data = DashboardData::new();
        let mut metrics = Metrics::new();
        metrics.system.memory_usage = 2 * 1024 * 1024 * 1024; // 2GB
        metrics.system.cpu_usage = 95.0;
        metrics.system.disk_usage = 25 * 1024 * 1024 * 1024; // 25GB

        // Add slow search timing
        metrics.record_timing("search", Duration::from_millis(500));

        let profiles = HashMap::new();
        data.update(metrics, profiles).await;

        assert!(data.health_status.overall_score < 50);
        assert!(data.health_status.memory_health < 50);
        assert!(data.health_status.cpu_health < 50);
    }

    #[lexum_macros::tokio_test]
    async fn test_dashboard_data_check_alerts_slow_operation() {
        let mut data = DashboardData::new();
        let mut profile = Profile::new();
        profile.name = "slow_op".to_string();
        profile.add_measurement(Duration::from_millis(2000)); // 2 seconds
        data.profiles.insert("slow_op".to_string(), profile);

        data.check_alerts();

        assert!(!data.alerts.is_empty());
        assert!(data.alerts.iter().any(|a| a.message.contains("slow_op")));
    }

    #[lexum_macros::tokio_test]
    async fn test_dashboard_data_check_alerts_high_memory() {
        let mut data = DashboardData::new();
        data.metrics.system.memory_usage = 2 * 1024 * 1024 * 1024; // 2GB

        data.check_alerts();

        assert!(!data.alerts.is_empty());
        assert!(data.alerts.iter().any(|a| a.message.contains("memory")));
    }

    #[lexum_macros::tokio_test]
    async fn test_dashboard_data_check_alerts_high_cpu() {
        let mut data = DashboardData::new();
        data.metrics.system.cpu_usage = 95.0;

        data.check_alerts();

        // Should have CPU alert when usage > 90%
        assert!(data.alerts.iter().any(|a| a.message.contains("CPU")));
        assert!(data.alerts.iter().any(|a| a.level == AlertLevel::Error));
    }

    #[lexum_macros::tokio_test]
    async fn test_dashboard_data_check_alerts_critical_health() {
        let mut data = DashboardData::new();
        data.health_status.overall_score = 30;

        data.check_alerts();

        assert!(!data.alerts.is_empty());
        assert!(data.alerts.iter().any(|a| a.level == AlertLevel::Critical));
    }

    #[lexum_macros::tokio_test]
    async fn test_dashboard_data_update_stats_with_cache() {
        let mut data = DashboardData::new();
        let mut metrics = Metrics::new();
        metrics.increment_counter("cache_hits", 80);
        metrics.increment_counter("cache_misses", 20);

        let profiles = HashMap::new();
        data.update(metrics, profiles).await;

        assert_eq!(data.stats.cache_hit_rate, 80.0); // 80/(80+20) * 100
    }

    #[lexum_macros::tokio_test]
    async fn test_dashboard_data_add_alert_limit() {
        let mut data = DashboardData::new();

        // Add more than 100 alerts
        for i in 0..150 {
            data.add_alert(AlertLevel::Info, format!("Alert {i}"), None);
        }

        // Should keep only last 100
        assert_eq!(data.alerts.len(), 100);
    }

    #[lexum_macros::tokio_test]
    async fn test_performance_monitoring_service() {
        let service = PerformanceMonitoringService::new();

        service.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let dashboard = service.dashboard();
        let data = dashboard.get_data().await;
        assert!(data.last_update > 0);

        service.stop().await;
    }

    #[test]
    fn test_alert_level_equality() {
        assert_eq!(AlertLevel::Info, AlertLevel::Info);
        assert_eq!(AlertLevel::Warning, AlertLevel::Warning);
        assert_eq!(AlertLevel::Error, AlertLevel::Error);
        assert_eq!(AlertLevel::Critical, AlertLevel::Critical);
        assert_ne!(AlertLevel::Info, AlertLevel::Warning);
    }
}
