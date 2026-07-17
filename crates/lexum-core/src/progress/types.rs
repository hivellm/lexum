//! Progress tracking types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a progress tracking session
pub type ProgressId = String;

/// Status of a progress tracking session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum ProgressStatus {
    /// Operation is pending/queued
    Pending,
    /// Operation is currently running
    Running,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed,
    /// Operation was cancelled
    Cancelled,
    /// Operation is paused
    Paused,
}

/// Progress information for a single operation
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProgressInfo {
    /// Unique identifier for this progress session
    pub id: ProgressId,
    /// Type of operation being tracked
    pub operation_type: OperationType,
    /// Current status
    pub status: ProgressStatus,
    /// Human-readable description
    pub description: String,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time (if completed/failed/cancelled)
    pub end_time: Option<DateTime<Utc>>,
    /// Progress metrics
    pub metrics: ProgressMetrics,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Type of operation being tracked
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
pub enum OperationType {
    /// Bulk document operations
    BulkOperation,
    /// Reindexing operation
    Reindex,
    /// Snapshot creation
    SnapshotCreate,
    /// Snapshot restoration
    SnapshotRestore,
    /// Index creation
    IndexCreate,
    /// Index deletion
    IndexDelete,
    /// Index optimization
    IndexOptimize,
    /// Search operation
    Search,
    /// Template operations
    TemplateOperation,
    /// Alias operations
    AliasOperation,
    /// Custom operation
    Custom(String),
}

/// Progress metrics for tracking completion
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProgressMetrics {
    /// Total number of items to process
    pub total: u64,
    /// Number of items completed
    pub completed: u64,
    /// Number of items failed
    pub failed: u64,
    /// Number of items skipped
    pub skipped: u64,
    /// Current processing rate (items per second)
    pub rate: f64,
    /// Estimated time remaining (seconds)
    pub estimated_remaining: Option<u64>,
    /// Current phase or step
    pub current_phase: Option<String>,
    /// Additional custom metrics
    pub custom: HashMap<String, f64>,
}

impl Default for ProgressMetrics {
    fn default() -> Self {
        Self {
            total: 0,
            completed: 0,
            failed: 0,
            skipped: 0,
            rate: 0.0,
            estimated_remaining: None,
            current_phase: None,
            custom: HashMap::new(),
        }
    }
}

impl ProgressMetrics {
    /// Calculate completion percentage
    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.completed as f64 / self.total as f64) * 100.0
        }
    }

    /// Update the processing rate based on elapsed time
    pub fn update_rate(&mut self, elapsed_seconds: f64) {
        if elapsed_seconds > 0.0 {
            self.rate = self.completed as f64 / elapsed_seconds;

            // Calculate estimated remaining time
            if self.rate > 0.0 && self.completed < self.total {
                let remaining = self.total - self.completed;
                self.estimated_remaining = Some((remaining as f64 / self.rate) as u64);
            }
        }
    }

    /// Add custom metric
    pub fn set_custom(&mut self, key: String, value: f64) {
        self.custom.insert(key, value);
    }

    /// Get custom metric
    pub fn get_custom(&self, key: &str) -> Option<f64> {
        self.custom.get(key).copied()
    }
}

/// Progress update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    /// Progress ID
    pub id: ProgressId,
    /// Updated metrics
    pub metrics: ProgressMetrics,
    /// Current phase
    pub phase: Option<String>,
    /// Status change
    pub status: Option<ProgressStatus>,
    /// Error message if any
    pub error: Option<String>,
    /// Timestamp of the update
    pub timestamp: DateTime<Utc>,
}

/// Progress tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressConfig {
    /// Maximum number of progress sessions to keep in memory
    pub max_sessions: usize,
    /// How often to emit progress updates (milliseconds)
    pub update_interval_ms: u64,
    /// Whether to persist progress to disk
    pub persist_to_disk: bool,
    /// Whether to emit real-time updates via WebSocket
    pub real_time_updates: bool,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            max_sessions: 1000,
            update_interval_ms: 1000, // 1 second
            persist_to_disk: false,
            real_time_updates: false,
        }
    }
}

/// Progress tracking filter for querying sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressFilter {
    /// Filter by operation type
    pub operation_type: Option<OperationType>,
    /// Filter by status
    pub status: Option<ProgressStatus>,
    /// Filter by date range
    pub start_time_after: Option<DateTime<Utc>>,
    /// Filter by date range
    pub start_time_before: Option<DateTime<Utc>>,
    /// Limit number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Progress tracking statistics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProgressStats {
    /// Total number of sessions
    pub total_sessions: u64,
    /// Active sessions
    pub active_sessions: u64,
    /// Completed sessions
    pub completed_sessions: u64,
    /// Failed sessions
    pub failed_sessions: u64,
    /// Average completion time (seconds)
    pub avg_completion_time: Option<f64>,
    /// Most common operation type
    pub most_common_operation: Option<OperationType>,
}
