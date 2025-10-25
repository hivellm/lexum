//! Snapshot-related type definitions

use crate::types::{IndexName, RepositoryName, SnapshotName};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Snapshot state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, ToSchema)]
pub enum SnapshotState {
    /// Snapshot is being created
    #[default]
    InProgress,
    /// Snapshot completed successfully
    Success,
    /// Snapshot failed
    Failed,
    /// Snapshot was partially created
    Partial,
}

/// Snapshot information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SnapshotInfo {
    /// Snapshot name
    pub name: SnapshotName,

    /// Repository name
    pub repository: RepositoryName,

    /// Snapshot state
    pub state: SnapshotState,

    /// Indices included in snapshot
    pub indices: Vec<IndexName>,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time (if completed)
    pub end_time: Option<DateTime<Utc>>,

    /// Duration in milliseconds
    pub duration_in_millis: Option<u64>,

    /// Number of failures
    pub failures: u32,

    /// Number of shards
    pub shards: ShardInfo,

    /// Snapshot metadata
    pub metadata: SnapshotMetadata,
}

/// Shard information
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct ShardInfo {
    /// Total number of shards
    pub total: u32,

    /// Number of successful shards
    pub successful: u32,

    /// Number of failed shards
    pub failed: u32,
}

/// Snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SnapshotMetadata {
    /// User-provided metadata
    pub user_metadata: HashMap<String, String>,

    /// Snapshot version
    pub version: String,

    /// Creation timestamp
    pub creation_time: DateTime<Utc>,
}

impl Default for SnapshotMetadata {
    fn default() -> Self {
        Self {
            user_metadata: HashMap::new(),
            version: "1.0".to_string(),
            creation_time: Utc::now(),
        }
    }
}

/// Snapshot repository information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RepositoryInfo {
    /// Repository name
    pub name: RepositoryName,

    /// Repository type
    pub repository_type: String,

    /// Repository settings
    pub settings: HashMap<String, String>,

    /// Number of snapshots in repository
    pub snapshot_count: u32,

    /// Total size of snapshots in bytes
    pub total_size: u64,
}

/// Snapshot creation request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSnapshotRequest {
    /// Indices to include in snapshot
    pub indices: Vec<IndexName>,

    /// Snapshot metadata
    pub metadata: Option<SnapshotMetadata>,

    /// Wait for completion
    pub wait_for_completion: bool,

    /// Ignore unavailable indices
    pub ignore_unavailable: bool,

    /// Include global state
    pub include_global_state: bool,
}

impl Default for CreateSnapshotRequest {
    fn default() -> Self {
        Self {
            indices: vec![],
            metadata: None,
            wait_for_completion: false,
            ignore_unavailable: false,
            include_global_state: true,
        }
    }
}

/// Snapshot restore request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RestoreSnapshotRequest {
    /// Indices to restore
    pub indices: Vec<IndexName>,

    /// Rename pattern for indices
    pub rename_pattern: Option<String>,

    /// Rename replacement
    pub rename_replacement: Option<String>,

    /// Wait for completion
    pub wait_for_completion: bool,

    /// Ignore unavailable indices
    pub ignore_unavailable: bool,

    /// Include global state
    pub include_global_state: bool,

    /// Include aliases
    pub include_aliases: bool,
}

impl Default for RestoreSnapshotRequest {
    fn default() -> Self {
        Self {
            indices: vec![],
            rename_pattern: None,
            rename_replacement: None,
            wait_for_completion: false,
            ignore_unavailable: false,
            include_global_state: true,
            include_aliases: true,
        }
    }
}

/// Snapshot statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct SnapshotStats {
    /// Total number of snapshots
    pub total_snapshots: u32,

    /// Total size of all snapshots in bytes
    pub total_size: u64,

    /// Number of successful snapshots
    pub successful_snapshots: u32,

    /// Number of failed snapshots
    pub failed_snapshots: u32,

    /// Number of in-progress snapshots
    pub in_progress_snapshots: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_state_default() {
        let state = SnapshotState::default();
        assert_eq!(state, SnapshotState::InProgress);
    }

    #[test]
    fn test_shard_info_default() {
        let shards = ShardInfo::default();
        assert_eq!(shards.total, 0);
        assert_eq!(shards.successful, 0);
        assert_eq!(shards.failed, 0);
    }

    #[test]
    fn test_create_snapshot_request_default() {
        let request = CreateSnapshotRequest::default();
        assert!(request.indices.is_empty());
        assert!(request.metadata.is_none());
        assert!(!request.wait_for_completion);
        assert!(!request.ignore_unavailable);
        assert!(request.include_global_state);
    }

    #[test]
    fn test_restore_snapshot_request_default() {
        let request = RestoreSnapshotRequest::default();
        assert!(request.indices.is_empty());
        assert!(request.rename_pattern.is_none());
        assert!(request.rename_replacement.is_none());
        assert!(!request.wait_for_completion);
        assert!(!request.ignore_unavailable);
        assert!(request.include_global_state);
        assert!(request.include_aliases);
    }

    #[test]
    fn test_snapshot_stats_default() {
        let stats = SnapshotStats::default();
        assert_eq!(stats.total_snapshots, 0);
        assert_eq!(stats.total_size, 0);
        assert_eq!(stats.successful_snapshots, 0);
        assert_eq!(stats.failed_snapshots, 0);
        assert_eq!(stats.in_progress_snapshots, 0);
    }
}
