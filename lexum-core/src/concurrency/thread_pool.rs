//! Thread pool configuration and optimization
//!
//! This module provides configuration for optimizing thread pool sizing
//! based on workload characteristics and system resources.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Thread pool configuration
#[derive(Debug, Clone)]
pub struct ThreadPoolConfig {
    /// Number of worker threads for CPU-bound tasks
    pub cpu_threads: usize,
    /// Number of worker threads for I/O-bound tasks
    pub io_threads: usize,
    /// Stack size per thread (bytes)
    pub stack_size: usize,
    /// Thread name prefix
    pub thread_name_prefix: String,
    /// Enable thread affinity (pin threads to CPU cores)
    pub enable_thread_affinity: bool,
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            cpu_threads: cpu_count,
            io_threads: cpu_count * 2,
            stack_size: 2 * 1024 * 1024, // 2MB
            thread_name_prefix: "lexum-worker".to_string(),
            enable_thread_affinity: false,
        }
    }
}

impl ThreadPoolConfig {
    /// Create a new thread pool configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set number of CPU threads
    pub fn with_cpu_threads(mut self, threads: usize) -> Self {
        self.cpu_threads = threads;
        self
    }

    /// Set number of I/O threads
    pub fn with_io_threads(mut self, threads: usize) -> Self {
        self.io_threads = threads;
        self
    }

    /// Set stack size per thread
    pub fn with_stack_size(mut self, size: usize) -> Self {
        self.stack_size = size;
        self
    }

    /// Set thread name prefix
    pub fn with_thread_name_prefix(mut self, prefix: String) -> Self {
        self.thread_name_prefix = prefix;
        self
    }

    /// Enable thread affinity
    pub fn with_thread_affinity(mut self, enabled: bool) -> Self {
        self.enable_thread_affinity = enabled;
        self
    }

    /// Calculate optimal thread count based on workload
    ///
    /// Formula: threads = CPU_COUNT * (1 + BLOCKING_TIME / COMPUTE_TIME)
    pub fn calculate_optimal_threads(
        blocking_time_ratio: f64,
        min_threads: usize,
        max_threads: usize,
    ) -> usize {
        let cpu_count = num_cpus::get();
        let optimal = (cpu_count as f64 * (1.0 + blocking_time_ratio)) as usize;
        optimal.clamp(min_threads, max_threads)
    }

    /// Create configuration optimized for CPU-bound workloads
    pub fn for_cpu_bound() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            cpu_threads: cpu_count,
            io_threads: cpu_count,
            ..Default::default()
        }
    }

    /// Create configuration optimized for I/O-bound workloads
    pub fn for_io_bound() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            cpu_threads: cpu_count,
            io_threads: cpu_count * 4,
            ..Default::default()
        }
    }

    /// Create configuration optimized for mixed workloads
    pub fn for_mixed() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            cpu_threads: cpu_count,
            io_threads: cpu_count * 2,
            ..Default::default()
        }
    }
}

/// Thread pool statistics
#[derive(Debug, Clone, Default)]
pub struct ThreadPoolStats {
    /// Total tasks executed
    pub total_tasks: u64,
    /// Tasks currently in queue
    pub queued_tasks: u64,
    /// Tasks currently executing
    pub active_tasks: u64,
    /// Tasks completed successfully
    pub completed_tasks: u64,
    /// Tasks that failed
    pub failed_tasks: u64,
    /// Average task execution time (microseconds)
    pub avg_execution_time_us: u64,
    /// Maximum task execution time (microseconds)
    pub max_execution_time_us: u64,
}

impl ThreadPoolStats {
    /// Create new statistics tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a task start
    pub fn record_task_start(&mut self) {
        self.total_tasks += 1;
        self.queued_tasks += 1;
    }

    /// Record a task execution start
    pub fn record_task_execute(&mut self) {
        self.queued_tasks = self.queued_tasks.saturating_sub(1);
        self.active_tasks += 1;
    }

    /// Record a task completion
    pub fn record_task_complete(&mut self, execution_time_us: u64) {
        self.active_tasks = self.active_tasks.saturating_sub(1);
        self.completed_tasks += 1;
        self.update_avg_time(execution_time_us);
        if execution_time_us > self.max_execution_time_us {
            self.max_execution_time_us = execution_time_us;
        }
    }

    /// Record a task failure
    pub fn record_task_fail(&mut self) {
        self.active_tasks = self.active_tasks.saturating_sub(1);
        self.failed_tasks += 1;
    }

    /// Update average execution time
    fn update_avg_time(&mut self, new_time_us: u64) {
        if self.completed_tasks == 0 {
            self.avg_execution_time_us = new_time_us;
        } else {
            // Running average
            let completed = self.completed_tasks;
            self.avg_execution_time_us =
                (self.avg_execution_time_us * (completed - 1) + new_time_us) / completed;
        }
    }
}

/// Thread-safe statistics tracker
#[derive(Debug, Clone)]
pub struct AtomicThreadPoolStats {
    total_tasks: Arc<AtomicU64>,
    queued_tasks: Arc<AtomicU64>,
    active_tasks: Arc<AtomicU64>,
    completed_tasks: Arc<AtomicU64>,
    failed_tasks: Arc<AtomicU64>,
}

impl AtomicThreadPoolStats {
    /// Create new atomic statistics tracker
    pub fn new() -> Self {
        Self {
            total_tasks: Arc::new(AtomicU64::new(0)),
            queued_tasks: Arc::new(AtomicU64::new(0)),
            active_tasks: Arc::new(AtomicU64::new(0)),
            completed_tasks: Arc::new(AtomicU64::new(0)),
            failed_tasks: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a task start
    pub fn record_task_start(&self) {
        self.total_tasks.fetch_add(1, Ordering::Relaxed);
        self.queued_tasks.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a task execution start
    pub fn record_task_execute(&self) {
        self.queued_tasks.fetch_sub(1, Ordering::Relaxed);
        self.active_tasks.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a task completion
    pub fn record_task_complete(&self) {
        self.active_tasks.fetch_sub(1, Ordering::Relaxed);
        self.completed_tasks.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a task failure
    pub fn record_task_fail(&self) {
        self.active_tasks.fetch_sub(1, Ordering::Relaxed);
        self.failed_tasks.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current statistics snapshot
    pub fn snapshot(&self) -> ThreadPoolStats {
        ThreadPoolStats {
            total_tasks: self.total_tasks.load(Ordering::Relaxed),
            queued_tasks: self.queued_tasks.load(Ordering::Relaxed),
            active_tasks: self.active_tasks.load(Ordering::Relaxed),
            completed_tasks: self.completed_tasks.load(Ordering::Relaxed),
            failed_tasks: self.failed_tasks.load(Ordering::Relaxed),
            avg_execution_time_us: 0, // Would need additional tracking
            max_execution_time_us: 0, // Would need additional tracking
        }
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.total_tasks.store(0, Ordering::Relaxed);
        self.queued_tasks.store(0, Ordering::Relaxed);
        self.active_tasks.store(0, Ordering::Relaxed);
        self.completed_tasks.store(0, Ordering::Relaxed);
        self.failed_tasks.store(0, Ordering::Relaxed);
    }
}

impl Default for AtomicThreadPoolStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_pool_config_default() {
        let config = ThreadPoolConfig::default();
        assert!(config.cpu_threads > 0);
        assert!(config.io_threads > 0);
    }

    #[test]
    fn test_thread_pool_config_builder() {
        let config = ThreadPoolConfig::new()
            .with_cpu_threads(4)
            .with_io_threads(8)
            .with_stack_size(4 * 1024 * 1024);

        assert_eq!(config.cpu_threads, 4);
        assert_eq!(config.io_threads, 8);
        assert_eq!(config.stack_size, 4 * 1024 * 1024);
    }

    #[test]
    fn test_calculate_optimal_threads() {
        let threads = ThreadPoolConfig::calculate_optimal_threads(0.5, 1, 100);
        assert!((1..=100).contains(&threads));
    }

    #[test]
    fn test_thread_pool_stats() {
        let mut stats = ThreadPoolStats::new();
        stats.record_task_start();
        stats.record_task_execute();
        stats.record_task_complete(1000);
        stats.record_task_fail();

        assert_eq!(stats.total_tasks, 1);
        assert_eq!(stats.completed_tasks, 1);
        assert_eq!(stats.failed_tasks, 1);
    }

    #[test]
    fn test_atomic_thread_pool_stats() {
        let stats = AtomicThreadPoolStats::new();
        stats.record_task_start();
        stats.record_task_execute();
        stats.record_task_complete();
        stats.record_task_fail();

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.total_tasks, 1);
        assert_eq!(snapshot.completed_tasks, 1);
        assert_eq!(snapshot.failed_tasks, 1);
    }
}
