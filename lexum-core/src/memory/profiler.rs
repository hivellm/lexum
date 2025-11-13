//! Memory profiler for tracking and analyzing memory usage
//!
//! This module provides memory profiling capabilities to track:
//! - Memory usage by component
//! - Allocation patterns
//! - Memory leaks detection
//! - Memory usage reports

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Memory usage snapshot
#[derive(Debug, Clone, Serialize)]
pub struct MemorySnapshot {
    /// Timestamp of the snapshot
    #[serde(skip)]
    pub timestamp: Instant,
    /// Timestamp as seconds since epoch (for serialization)
    pub timestamp_secs: u64,
    /// Total memory usage in bytes
    pub total_memory: u64,
    /// Memory usage by component
    pub component_memory: HashMap<String, u64>,
    /// Number of allocations
    pub allocation_count: u64,
    /// Number of deallocations
    pub deallocation_count: u64,
}

impl MemorySnapshot {
    /// Create a new memory snapshot
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            timestamp: now,
            timestamp_secs: now.elapsed().as_secs(),
            total_memory: 0,
            component_memory: HashMap::new(),
            allocation_count: 0,
            deallocation_count: 0,
        }
    }

    /// Calculate memory delta from another snapshot
    pub fn delta(&self, other: &MemorySnapshot) -> i64 {
        self.total_memory as i64 - other.total_memory as i64
    }
}

impl Default for MemorySnapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory profiler for tracking memory usage
#[derive(Debug, Clone)]
pub struct MemoryProfiler {
    /// Snapshots over time
    snapshots: Arc<Mutex<Vec<MemorySnapshot>>>,
    /// Current memory usage by component
    component_usage: Arc<Mutex<HashMap<String, u64>>>,
    /// Allocation counters
    allocation_counters: Arc<Mutex<AllocationCounters>>,
    /// Whether profiling is enabled
    enabled: bool,
    /// Maximum number of snapshots to keep
    max_snapshots: usize,
}

/// Allocation counters
#[derive(Debug, Clone, Default)]
struct AllocationCounters {
    /// Total allocations
    total_allocations: u64,
    /// Total deallocations
    total_deallocations: u64,
    /// Allocations by component
    component_allocations: HashMap<String, u64>,
    /// Deallocations by component
    component_deallocations: HashMap<String, u64>,
}

impl MemoryProfiler {
    /// Create a new memory profiler
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(Mutex::new(Vec::new())),
            component_usage: Arc::new(Mutex::new(HashMap::new())),
            allocation_counters: Arc::new(Mutex::new(AllocationCounters::default())),
            enabled: true,
            max_snapshots: 1000,
        }
    }

    /// Create profiler with custom settings
    pub fn with_settings(max_snapshots: usize, enabled: bool) -> Self {
        Self {
            snapshots: Arc::new(Mutex::new(Vec::new())),
            component_usage: Arc::new(Mutex::new(HashMap::new())),
            allocation_counters: Arc::new(Mutex::new(AllocationCounters::default())),
            enabled,
            max_snapshots,
        }
    }

    /// Record memory usage for a component
    pub fn record_component_usage(&self, component: &str, bytes: u64) {
        if !self.enabled {
            return;
        }

        let mut usage = self.component_usage.lock().unwrap();
        usage.insert(component.to_string(), bytes);
    }

    /// Record an allocation
    pub fn record_allocation(&self, component: &str, bytes: u64) {
        if !self.enabled {
            return;
        }

        let mut counters = self.allocation_counters.lock().unwrap();
        counters.total_allocations += 1;
        *counters
            .component_allocations
            .entry(component.to_string())
            .or_insert(0) += 1;

        let mut usage = self.component_usage.lock().unwrap();
        *usage.entry(component.to_string()).or_insert(0) += bytes;
    }

    /// Record a deallocation
    pub fn record_deallocation(&self, component: &str, bytes: u64) {
        if !self.enabled {
            return;
        }

        let mut counters = self.allocation_counters.lock().unwrap();
        counters.total_deallocations += 1;
        *counters
            .component_deallocations
            .entry(component.to_string())
            .or_insert(0) += 1;

        let mut usage = self.component_usage.lock().unwrap();
        if let Some(current) = usage.get_mut(component) {
            if *current >= bytes {
                *current -= bytes;
            } else {
                *current = 0;
            }
        }
    }

    /// Take a memory snapshot
    pub fn take_snapshot(&self) -> MemorySnapshot {
        let usage = self.component_usage.lock().unwrap();
        let counters = self.allocation_counters.lock().unwrap();

        let total_memory: u64 = usage.values().sum();

        let now = Instant::now();
        let snapshot = MemorySnapshot {
            timestamp: now,
            timestamp_secs: now.elapsed().as_secs(),
            total_memory,
            component_memory: usage.clone(),
            allocation_count: counters.total_allocations,
            deallocation_count: counters.total_deallocations,
        };

        // Store snapshot
        let mut snapshots = self.snapshots.lock().unwrap();
        snapshots.push(snapshot.clone());

        // Limit snapshot history
        if snapshots.len() > self.max_snapshots {
            let excess = snapshots.len() - self.max_snapshots;
            snapshots.drain(0..excess);
        }

        snapshot
    }

    /// Get current memory usage
    pub fn current_usage(&self) -> HashMap<String, u64> {
        let usage = self.component_usage.lock().unwrap();
        usage.clone()
    }

    /// Get total memory usage
    pub fn total_usage(&self) -> u64 {
        let usage = self.component_usage.lock().unwrap();
        usage.values().sum()
    }

    /// Get allocation statistics
    pub fn allocation_stats(&self) -> AllocationStats {
        let counters = self.allocation_counters.lock().unwrap();
        let usage = self.component_usage.lock().unwrap();

        AllocationStats {
            total_allocations: counters.total_allocations,
            total_deallocations: counters.total_deallocations,
            net_allocations: counters.total_allocations as i64
                - counters.total_deallocations as i64,
            component_allocations: counters.component_allocations.clone(),
            component_deallocations: counters.component_deallocations.clone(),
            current_usage: usage.clone(),
        }
    }

    /// Get memory usage report
    pub fn generate_report(&self) -> MemoryReport {
        let snapshots = self.snapshots.lock().unwrap();
        let usage = self.component_usage.lock().unwrap();
        let counters = self.allocation_counters.lock().unwrap();

        let total_memory: u64 = usage.values().sum();
        let peak_memory = snapshots.iter().map(|s| s.total_memory).max().unwrap_or(0);

        let memory_growth = if snapshots.len() >= 2 {
            let first = &snapshots[0];
            let last = snapshots.last().unwrap();
            last.total_memory as i64 - first.total_memory as i64
        } else {
            0
        };

        MemoryReport {
            total_memory,
            peak_memory,
            memory_growth,
            component_usage: usage.clone(),
            allocation_stats: AllocationStats {
                total_allocations: counters.total_allocations,
                total_deallocations: counters.total_deallocations,
                net_allocations: counters.total_allocations as i64
                    - counters.total_deallocations as i64,
                component_allocations: counters.component_allocations.clone(),
                component_deallocations: counters.component_deallocations.clone(),
                current_usage: usage.clone(),
            },
            snapshot_count: snapshots.len(),
        }
    }

    /// Clear all profiling data
    pub fn clear(&self) {
        let mut snapshots = self.snapshots.lock().unwrap();
        let mut usage = self.component_usage.lock().unwrap();
        let mut counters = self.allocation_counters.lock().unwrap();

        snapshots.clear();
        usage.clear();
        *counters = AllocationCounters::default();
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

impl Default for MemoryProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Allocation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationStats {
    /// Total number of allocations
    pub total_allocations: u64,
    /// Total number of deallocations
    pub total_deallocations: u64,
    /// Net allocations (allocations - deallocations)
    pub net_allocations: i64,
    /// Allocations by component
    pub component_allocations: HashMap<String, u64>,
    /// Deallocations by component
    pub component_deallocations: HashMap<String, u64>,
    /// Current memory usage by component
    pub current_usage: HashMap<String, u64>,
}

/// Memory usage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryReport {
    /// Total memory usage in bytes
    pub total_memory: u64,
    /// Peak memory usage in bytes
    pub peak_memory: u64,
    /// Memory growth since profiling started (bytes)
    pub memory_growth: i64,
    /// Memory usage by component
    pub component_usage: HashMap<String, u64>,
    /// Allocation statistics
    pub allocation_stats: AllocationStats,
    /// Number of snapshots taken
    pub snapshot_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_profiler_creation() {
        let profiler = MemoryProfiler::new();
        assert!(profiler.is_enabled());
        assert_eq!(profiler.total_usage(), 0);
    }

    #[test]
    fn test_record_component_usage() {
        let profiler = MemoryProfiler::new();
        profiler.record_component_usage("cache", 1024);
        profiler.record_component_usage("index", 2048);

        let usage = profiler.current_usage();
        assert_eq!(usage.get("cache"), Some(&1024));
        assert_eq!(usage.get("index"), Some(&2048));
        assert_eq!(profiler.total_usage(), 3072);
    }

    #[test]
    fn test_record_allocation_deallocation() {
        let profiler = MemoryProfiler::new();
        profiler.record_allocation("cache", 512);
        profiler.record_allocation("cache", 256);
        profiler.record_deallocation("cache", 256);

        let usage = profiler.current_usage();
        assert_eq!(usage.get("cache"), Some(&512));

        let stats = profiler.allocation_stats();
        assert_eq!(stats.total_allocations, 2);
        assert_eq!(stats.total_deallocations, 1);
        assert_eq!(stats.net_allocations, 1);
    }

    #[test]
    fn test_take_snapshot() {
        let profiler = MemoryProfiler::new();
        profiler.record_component_usage("cache", 1024);
        profiler.record_component_usage("index", 2048);

        let snapshot = profiler.take_snapshot();
        assert_eq!(snapshot.total_memory, 3072);
        assert_eq!(snapshot.component_memory.len(), 2);
    }

    #[test]
    fn test_generate_report() {
        let profiler = MemoryProfiler::new();
        profiler.record_component_usage("cache", 1024);
        profiler.record_allocation("cache", 512);
        profiler.take_snapshot();

        let report = profiler.generate_report();
        assert_eq!(report.total_memory, 1536); // 1024 + 512
        assert_eq!(report.snapshot_count, 1);
        assert!(report.component_usage.contains_key("cache"));
    }

    #[test]
    fn test_clear() {
        let profiler = MemoryProfiler::new();
        profiler.record_component_usage("cache", 1024);
        profiler.take_snapshot();

        profiler.clear();
        assert_eq!(profiler.total_usage(), 0);
        let report = profiler.generate_report();
        assert_eq!(report.snapshot_count, 0);
    }

    #[test]
    fn test_memory_snapshot_delta() {
        let mut snapshot1 = MemorySnapshot::new();
        snapshot1.total_memory = 1000;

        let mut snapshot2 = MemorySnapshot::new();
        snapshot2.total_memory = 1500;

        assert_eq!(snapshot2.delta(&snapshot1), 500);
        assert_eq!(snapshot1.delta(&snapshot2), -500);
    }
}
