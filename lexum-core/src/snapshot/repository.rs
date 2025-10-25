//! Snapshot repository implementation

use crate::config::SnapshotRepositoryConfig;
use crate::error::{Error, Result};
use crate::snapshot::types::*;
use crate::types::{RepositoryName, SnapshotName};
use chrono::Utc;
use std::collections::HashMap;
use tokio::fs;

/// Snapshot repository trait
#[async_trait::async_trait]
pub trait SnapshotRepository: Send + Sync {
    /// Get repository information
    async fn get_info(&self) -> Result<RepositoryInfo>;

    /// Create a snapshot
    async fn create_snapshot(
        &self,
        snapshot_name: SnapshotName,
        request: CreateSnapshotRequest,
    ) -> Result<SnapshotInfo>;

    /// Get snapshot information
    async fn get_snapshot(&self, snapshot_name: SnapshotName) -> Result<SnapshotInfo>;

    /// List all snapshots
    async fn list_snapshots(&self) -> Result<Vec<SnapshotInfo>>;

    /// Delete a snapshot
    async fn delete_snapshot(&self, snapshot_name: SnapshotName) -> Result<()>;

    /// Restore from snapshot
    async fn restore_snapshot(
        &self,
        snapshot_name: SnapshotName,
        request: RestoreSnapshotRequest,
    ) -> Result<()>;

    /// Get repository statistics
    async fn get_stats(&self) -> Result<SnapshotStats>;
}

/// Filesystem-based snapshot repository
pub struct FsSnapshotRepository {
    name: RepositoryName,
    path: String,
    settings: HashMap<String, String>,
}

impl FsSnapshotRepository {
    /// Create a new filesystem snapshot repository
    pub fn new(config: SnapshotRepositoryConfig) -> Result<Self> {
        let path = config.settings.location.clone();

        // Validate path
        if path.is_empty() {
            return Err(Error::Validation(
                "Repository location cannot be empty".to_string(),
            ));
        }

        let mut settings = HashMap::new();
        settings.insert("location".to_string(), path.clone());
        settings.insert("compress".to_string(), config.settings.compress.to_string());
        settings.insert("chunk_size".to_string(), config.settings.chunk_size.clone());
        settings.insert(
            "max_restore_bytes_per_sec".to_string(),
            config.settings.max_restore_bytes_per_sec.clone(),
        );
        settings.insert(
            "max_snapshot_bytes_per_sec".to_string(),
            config.settings.max_snapshot_bytes_per_sec.clone(),
        );
        settings.insert("readonly".to_string(), config.settings.readonly.to_string());

        Ok(Self {
            name: RepositoryName::new(config.name),
            path,
            settings,
        })
    }

    /// Ensure repository directory exists
    async fn ensure_directory(&self) -> Result<()> {
        fs::create_dir_all(&self.path).await?;
        Ok(())
    }

    /// Get snapshot directory path
    fn get_snapshot_path(&self, snapshot_name: &SnapshotName) -> String {
        format!("{}/{}", self.path, snapshot_name.as_str())
    }

    /// Get snapshots metadata file path
    fn get_metadata_path(&self) -> String {
        format!("{}/snapshots.json", self.path)
    }
}

#[async_trait::async_trait]
impl SnapshotRepository for FsSnapshotRepository {
    async fn get_info(&self) -> Result<RepositoryInfo> {
        self.ensure_directory().await?;

        let snapshot_count = self.count_snapshots().await?;
        let total_size = self.calculate_total_size().await?;

        Ok(RepositoryInfo {
            name: self.name.clone(),
            repository_type: "fs".to_string(),
            settings: self.settings.clone(),
            snapshot_count,
            total_size,
        })
    }

    async fn create_snapshot(
        &self,
        snapshot_name: SnapshotName,
        request: CreateSnapshotRequest,
    ) -> Result<SnapshotInfo> {
        self.ensure_directory().await?;

        let snapshot_path = self.get_snapshot_path(&snapshot_name);

        // Check if snapshot already exists
        if fs::metadata(&snapshot_path).await.is_ok() {
            return Err(Error::Validation(format!(
                "Snapshot '{}' already exists",
                snapshot_name.as_str()
            )));
        }

        // Create snapshot directory
        fs::create_dir_all(&snapshot_path).await?;

        let start_time = Utc::now();
        let mut failures = 0;
        let mut shards = ShardInfo::default();

        // Create snapshot metadata file
        let metadata_file = format!("{}/snapshot.json", snapshot_path);
        let mut snapshot_info = SnapshotInfo {
            name: snapshot_name.clone(),
            repository: self.name.clone(),
            state: SnapshotState::InProgress,
            indices: request.indices.clone(),
            start_time,
            end_time: None,
            duration_in_millis: None,
            failures: 0,
            shards: ShardInfo::default(),
            metadata: request.metadata.unwrap_or_default(),
        };

        // Save initial metadata
        self.save_snapshot_metadata(&snapshot_info).await?;

        // Create index snapshots
        for index_name in &request.indices {
            let index_snapshot_path = format!("{}/{}", snapshot_path, index_name.as_str());
            fs::create_dir_all(&index_snapshot_path).await?;

            // Create a simple index snapshot file
            // In a real implementation, this would copy actual index data
            let index_metadata = serde_json::json!({
                "name": index_name.as_str(),
                "created_at": start_time,
                "version": "1.0"
            });

            let index_metadata_file = format!("{}/index.json", index_snapshot_path);
            fs::write(&index_metadata_file, serde_json::to_string_pretty(&index_metadata)?).await?;

            // Create a placeholder data file
            let data_file = format!("{}/data.bin", index_snapshot_path);
            fs::write(&data_file, b"snapshot_data_placeholder").await?;

            shards.total += 1;
            shards.successful += 1;
        }

        let end_time = Utc::now();
        let duration = end_time.signed_duration_since(start_time);

        // Update snapshot info with completion data
        snapshot_info.state = SnapshotState::Success;
        snapshot_info.end_time = Some(end_time);
        snapshot_info.duration_in_millis = Some(duration.num_milliseconds() as u64);
        snapshot_info.failures = failures;
        snapshot_info.shards = shards;

        // Save final metadata
        self.save_snapshot_metadata(&snapshot_info).await?;

        Ok(snapshot_info)
    }

    async fn get_snapshot(&self, snapshot_name: SnapshotName) -> Result<SnapshotInfo> {
        let metadata_path = self.get_metadata_path();

        if fs::metadata(&metadata_path).await.is_err() {
            return Err(Error::NotFound(format!(
                "Snapshot '{}' not found",
                snapshot_name.as_str()
            )));
        }

        let content = fs::read_to_string(&metadata_path).await?;
        let snapshots: HashMap<String, SnapshotInfo> = serde_json::from_str(&content)?;

        snapshots
            .get(snapshot_name.as_str())
            .cloned()
            .ok_or_else(|| {
                Error::NotFound(format!("Snapshot '{}' not found", snapshot_name.as_str()))
            })
    }

    async fn list_snapshots(&self) -> Result<Vec<SnapshotInfo>> {
        let metadata_path = self.get_metadata_path();

        if fs::metadata(&metadata_path).await.is_err() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&metadata_path).await?;
        let snapshots: HashMap<String, SnapshotInfo> = serde_json::from_str(&content)?;

        Ok(snapshots.into_values().collect())
    }

    async fn delete_snapshot(&self, snapshot_name: SnapshotName) -> Result<()> {
        let snapshot_path = self.get_snapshot_path(&snapshot_name);

        if fs::metadata(&snapshot_path).await.is_err() {
            return Err(Error::NotFound(format!(
                "Snapshot '{}' not found",
                snapshot_name.as_str()
            )));
        }

        // Remove snapshot directory
        fs::remove_dir_all(&snapshot_path).await?;

        // Remove from metadata
        self.remove_snapshot_from_metadata(&snapshot_name).await?;

        Ok(())
    }

    async fn restore_snapshot(
        &self,
        snapshot_name: SnapshotName,
        _request: RestoreSnapshotRequest,
    ) -> Result<()> {
        let snapshot_info = self.get_snapshot(snapshot_name).await?;

        if snapshot_info.state != SnapshotState::Success {
            return Err(Error::Validation(format!(
                "Cannot restore snapshot '{}' in state {:?}",
                snapshot_info.name.as_str(),
                snapshot_info.state
            )));
        }

        // TODO: Implement actual restore logic
        // This would involve copying data from snapshot back to indices

        Ok(())
    }

    async fn get_stats(&self) -> Result<SnapshotStats> {
        let snapshots = self.list_snapshots().await?;

        let mut total_size = 0;
        let mut successful_snapshots = 0;
        let mut failed_snapshots = 0;
        let mut in_progress_snapshots = 0;

        for snapshot in &snapshots {
            total_size += 0; // TODO: Calculate actual size

            match snapshot.state {
                SnapshotState::Success => successful_snapshots += 1,
                SnapshotState::Failed => failed_snapshots += 1,
                SnapshotState::InProgress => in_progress_snapshots += 1,
                SnapshotState::Partial => failed_snapshots += 1,
            }
        }

        let stats = SnapshotStats {
            total_snapshots: snapshots.len() as u32,
            total_size,
            successful_snapshots,
            failed_snapshots,
            in_progress_snapshots,
        };

        Ok(stats)
    }
}

impl FsSnapshotRepository {
    /// Count snapshots in repository
    async fn count_snapshots(&self) -> Result<u32> {
        let snapshots = self.list_snapshots().await?;
        Ok(snapshots.len() as u32)
    }

    /// Calculate total size of all snapshots
    async fn calculate_total_size(&self) -> Result<u64> {
        // TODO: Implement actual size calculation
        Ok(0)
    }

    /// Save snapshot metadata
    async fn save_snapshot_metadata(&self, snapshot_info: &SnapshotInfo) -> Result<()> {
        let metadata_path = self.get_metadata_path();

        let mut snapshots = if fs::metadata(&metadata_path).await.is_ok() {
            let content = fs::read_to_string(&metadata_path).await?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        snapshots.insert(
            snapshot_info.name.as_str().to_string(),
            snapshot_info.clone(),
        );

        let content = serde_json::to_string_pretty(&snapshots)?;
        fs::write(&metadata_path, content).await?;

        Ok(())
    }

    /// Remove snapshot from metadata
    async fn remove_snapshot_from_metadata(&self, snapshot_name: &SnapshotName) -> Result<()> {
        let metadata_path = self.get_metadata_path();

        if fs::metadata(&metadata_path).await.is_err() {
            return Ok(());
        }

        let content = fs::read_to_string(&metadata_path).await?;
        let mut snapshots: HashMap<String, SnapshotInfo> = serde_json::from_str(&content)?;

        snapshots.remove(snapshot_name.as_str());

        let content = serde_json::to_string_pretty(&snapshots)?;
        fs::write(&metadata_path, content).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_fs_repository_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = SnapshotRepositoryConfig {
            name: "test_repo".to_string(),
            repository_type: "fs".to_string(),
            settings: crate::config::SnapshotRepositorySettings {
                location: temp_dir.path().to_string_lossy().to_string(),
                ..Default::default()
            },
        };

        let repo = FsSnapshotRepository::new(config).unwrap();
        assert_eq!(repo.name.as_str(), "test_repo");
    }

    #[tokio::test]
    async fn test_fs_repository_info() {
        let temp_dir = TempDir::new().unwrap();
        let config = SnapshotRepositoryConfig {
            name: "test_repo".to_string(),
            repository_type: "fs".to_string(),
            settings: crate::config::SnapshotRepositorySettings {
                location: temp_dir.path().to_string_lossy().to_string(),
                ..Default::default()
            },
        };

        let repo = FsSnapshotRepository::new(config).unwrap();
        let info = repo.get_info().await.unwrap();

        assert_eq!(info.name.as_str(), "test_repo");
        assert_eq!(info.repository_type, "fs");
        assert_eq!(info.snapshot_count, 0);
    }

    #[tokio::test]
    async fn test_create_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let config = SnapshotRepositoryConfig {
            name: "test_repo".to_string(),
            repository_type: "fs".to_string(),
            settings: crate::config::SnapshotRepositorySettings {
                location: temp_dir.path().to_string_lossy().to_string(),
                ..Default::default()
            },
        };

        let repo = FsSnapshotRepository::new(config).unwrap();
        let snapshot_name = SnapshotName::new("test_snapshot");
        let request = CreateSnapshotRequest {
            indices: vec![
                crate::types::IndexName::new("index1"),
                crate::types::IndexName::new("index2"),
            ],
            metadata: Some(SnapshotMetadata {
                user_metadata: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("description".to_string(), "Test snapshot".to_string());
                    map
                },
                version: "1.0".to_string(),
                creation_time: Utc::now(),
            }),
            wait_for_completion: true,
            ignore_unavailable: false,
            include_global_state: true,
        };

        let snapshot_info = repo.create_snapshot(snapshot_name.clone(), request).await.unwrap();

        assert_eq!(snapshot_info.name, snapshot_name);
        assert_eq!(snapshot_info.repository.as_str(), "test_repo");
        assert_eq!(snapshot_info.state, SnapshotState::Success);
        assert_eq!(snapshot_info.indices.len(), 2);
        assert!(snapshot_info.end_time.is_some());
        assert!(snapshot_info.duration_in_millis.is_some());
        assert_eq!(snapshot_info.failures, 0);
        assert_eq!(snapshot_info.shards.total, 2);
        assert_eq!(snapshot_info.shards.successful, 2);
        assert_eq!(snapshot_info.shards.failed, 0);

        // Verify snapshot directory was created
        let snapshot_path = temp_dir.path().join("test_snapshot");
        assert!(snapshot_path.exists());
        assert!(snapshot_path.is_dir());

        // Verify index directories were created
        let index1_path = snapshot_path.join("index1");
        let index2_path = snapshot_path.join("index2");
        assert!(index1_path.exists());
        assert!(index2_path.exists());

        // Verify metadata files exist
        assert!(index1_path.join("index.json").exists());
        assert!(index1_path.join("data.bin").exists());
        assert!(index2_path.join("index.json").exists());
        assert!(index2_path.join("data.bin").exists());
    }

    #[tokio::test]
    async fn test_create_duplicate_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let config = SnapshotRepositoryConfig {
            name: "test_repo".to_string(),
            repository_type: "fs".to_string(),
            settings: crate::config::SnapshotRepositorySettings {
                location: temp_dir.path().to_string_lossy().to_string(),
                ..Default::default()
            },
        };

        let repo = FsSnapshotRepository::new(config).unwrap();
        let snapshot_name = SnapshotName::new("test_snapshot");
        let request = CreateSnapshotRequest::default();

        // Create first snapshot
        repo.create_snapshot(snapshot_name.clone(), request.clone()).await.unwrap();

        // Try to create duplicate snapshot
        let result = repo.create_snapshot(snapshot_name, request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_get_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let config = SnapshotRepositoryConfig {
            name: "test_repo".to_string(),
            repository_type: "fs".to_string(),
            settings: crate::config::SnapshotRepositorySettings {
                location: temp_dir.path().to_string_lossy().to_string(),
                ..Default::default()
            },
        };

        let repo = FsSnapshotRepository::new(config).unwrap();
        let snapshot_name = SnapshotName::new("test_snapshot");
        let request = CreateSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            ..Default::default()
        };

        // Create snapshot
        let created_snapshot = repo.create_snapshot(snapshot_name.clone(), request).await.unwrap();

        // Get snapshot
        let retrieved_snapshot = repo.get_snapshot(snapshot_name).await.unwrap();

        assert_eq!(created_snapshot.name, retrieved_snapshot.name);
        assert_eq!(created_snapshot.repository, retrieved_snapshot.repository);
        assert_eq!(created_snapshot.state, retrieved_snapshot.state);
        assert_eq!(created_snapshot.indices, retrieved_snapshot.indices);
    }

    #[tokio::test]
    async fn test_list_snapshots() {
        let temp_dir = TempDir::new().unwrap();
        let config = SnapshotRepositoryConfig {
            name: "test_repo".to_string(),
            repository_type: "fs".to_string(),
            settings: crate::config::SnapshotRepositorySettings {
                location: temp_dir.path().to_string_lossy().to_string(),
                ..Default::default()
            },
        };

        let repo = FsSnapshotRepository::new(config).unwrap();

        // Initially no snapshots
        let snapshots = repo.list_snapshots().await.unwrap();
        assert_eq!(snapshots.len(), 0);

        // Create snapshots
        let snapshot1 = SnapshotName::new("snapshot1");
        let snapshot2 = SnapshotName::new("snapshot2");
        let request = CreateSnapshotRequest::default();

        repo.create_snapshot(snapshot1, request.clone()).await.unwrap();
        repo.create_snapshot(snapshot2, request).await.unwrap();

        // List snapshots
        let snapshots = repo.list_snapshots().await.unwrap();
        assert_eq!(snapshots.len(), 2);

        let snapshot_names: Vec<String> = snapshots.iter()
            .map(|s| s.name.as_str().to_string())
            .collect();
        assert!(snapshot_names.contains(&"snapshot1".to_string()));
        assert!(snapshot_names.contains(&"snapshot2".to_string()));
    }

    #[tokio::test]
    async fn test_delete_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let config = SnapshotRepositoryConfig {
            name: "test_repo".to_string(),
            repository_type: "fs".to_string(),
            settings: crate::config::SnapshotRepositorySettings {
                location: temp_dir.path().to_string_lossy().to_string(),
                ..Default::default()
            },
        };

        let repo = FsSnapshotRepository::new(config).unwrap();
        let snapshot_name = SnapshotName::new("test_snapshot");
        let request = CreateSnapshotRequest::default();

        // Create snapshot
        repo.create_snapshot(snapshot_name.clone(), request).await.unwrap();

        // Verify snapshot exists
        let snapshots = repo.list_snapshots().await.unwrap();
        assert_eq!(snapshots.len(), 1);

        // Delete snapshot
        repo.delete_snapshot(snapshot_name).await.unwrap();

        // Verify snapshot is deleted
        let snapshots = repo.list_snapshots().await.unwrap();
        assert_eq!(snapshots.len(), 0);
    }

    #[tokio::test]
    async fn test_snapshot_stats() {
        let temp_dir = TempDir::new().unwrap();
        let config = SnapshotRepositoryConfig {
            name: "test_repo".to_string(),
            repository_type: "fs".to_string(),
            settings: crate::config::SnapshotRepositorySettings {
                location: temp_dir.path().to_string_lossy().to_string(),
                ..Default::default()
            },
        };

        let repo = FsSnapshotRepository::new(config).unwrap();

        // Initially no snapshots
        let stats = repo.get_stats().await.unwrap();
        assert_eq!(stats.total_snapshots, 0);
        assert_eq!(stats.successful_snapshots, 0);
        assert_eq!(stats.failed_snapshots, 0);
        assert_eq!(stats.in_progress_snapshots, 0);

        // Create snapshots
        let snapshot1 = SnapshotName::new("snapshot1");
        let snapshot2 = SnapshotName::new("snapshot2");
        let request = CreateSnapshotRequest::default();

        repo.create_snapshot(snapshot1, request.clone()).await.unwrap();
        repo.create_snapshot(snapshot2, request).await.unwrap();

        // Check stats
        let stats = repo.get_stats().await.unwrap();
        assert_eq!(stats.total_snapshots, 2);
        assert_eq!(stats.successful_snapshots, 2);
        assert_eq!(stats.failed_snapshots, 0);
        assert_eq!(stats.in_progress_snapshots, 0);
    }
}
