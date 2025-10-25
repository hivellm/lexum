//! Snapshot manager for handling multiple repositories

use crate::config::{Config, SnapshotRepositoryConfig};
use crate::error::{Error, Result};
use crate::snapshot::repository::{FsSnapshotRepository, SnapshotRepository};
use crate::snapshot::types::*;
use crate::types::{RepositoryName, SnapshotName};
use std::collections::HashMap;
use std::sync::Arc;

/// Snapshot manager for handling multiple repositories
pub struct SnapshotManager {
    repositories: HashMap<RepositoryName, Arc<dyn SnapshotRepository>>,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(config: &Config) -> Result<Self> {
        let mut repositories = HashMap::new();

        for repo_config in &config.snapshots.repositories {
            let repository = Self::create_repository(repo_config)?;
            repositories.insert(
                RepositoryName::new(repo_config.name.clone()),
                Arc::new(repository) as Arc<dyn SnapshotRepository>,
            );
        }

        Ok(Self { repositories })
    }

    /// Create a repository based on configuration
    fn create_repository(config: &SnapshotRepositoryConfig) -> Result<FsSnapshotRepository> {
        match config.repository_type.as_str() {
            "fs" => {
                let repo = FsSnapshotRepository::new(config.clone())?;
                Ok(repo)
            }
            _ => Err(Error::Validation(format!(
                "Unsupported repository type: {}",
                config.repository_type
            ))),
        }
    }

    /// Get a repository by name
    pub fn get_repository(&self, name: &RepositoryName) -> Result<Arc<dyn SnapshotRepository>> {
        self.repositories
            .get(name)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("Repository '{}' not found", name.as_str())))
    }

    /// List all repositories
    pub fn list_repositories(&self) -> Vec<RepositoryName> {
        self.repositories.keys().cloned().collect()
    }

    /// Get repository information
    pub async fn get_repository_info(&self, name: &RepositoryName) -> Result<RepositoryInfo> {
        let repository = self.get_repository(name)?;
        repository.get_info().await
    }

    /// List all repositories with their information
    pub async fn list_repositories_info(&self) -> Result<Vec<RepositoryInfo>> {
        let mut infos = Vec::new();

        for name in self.repositories.keys() {
            let info = self.get_repository_info(name).await?;
            infos.push(info);
        }

        Ok(infos)
    }

    /// Create a snapshot
    pub async fn create_snapshot(
        &self,
        repository_name: &RepositoryName,
        snapshot_name: SnapshotName,
        request: CreateSnapshotRequest,
    ) -> Result<SnapshotInfo> {
        let repository = self.get_repository(repository_name)?;
        repository.create_snapshot(snapshot_name, request).await
    }

    /// Get snapshot information
    pub async fn get_snapshot(
        &self,
        repository_name: &RepositoryName,
        snapshot_name: SnapshotName,
    ) -> Result<SnapshotInfo> {
        let repository = self.get_repository(repository_name)?;
        repository.get_snapshot(snapshot_name).await
    }

    /// List snapshots in a repository
    pub async fn list_snapshots(
        &self,
        repository_name: &RepositoryName,
    ) -> Result<Vec<SnapshotInfo>> {
        let repository = self.get_repository(repository_name)?;
        repository.list_snapshots().await
    }

    /// Delete a snapshot
    pub async fn delete_snapshot(
        &self,
        repository_name: &RepositoryName,
        snapshot_name: SnapshotName,
    ) -> Result<()> {
        let repository = self.get_repository(repository_name)?;
        repository.delete_snapshot(snapshot_name).await
    }

    /// Restore from snapshot
    pub async fn restore_snapshot(
        &self,
        repository_name: &RepositoryName,
        snapshot_name: SnapshotName,
        request: RestoreSnapshotRequest,
    ) -> Result<()> {
        let repository = self.get_repository(repository_name)?;
        repository.restore_snapshot(snapshot_name, request).await
    }

    /// Get snapshot statistics for a repository
    pub async fn get_repository_stats(
        &self,
        repository_name: &RepositoryName,
    ) -> Result<SnapshotStats> {
        let repository = self.get_repository(repository_name)?;
        repository.get_stats().await
    }

    /// Get global snapshot statistics across all repositories
    pub async fn get_global_stats(&self) -> Result<SnapshotStats> {
        let mut global_stats = SnapshotStats::default();

        for repository in self.repositories.values() {
            let stats = repository.get_stats().await?;
            global_stats.total_snapshots += stats.total_snapshots;
            global_stats.total_size += stats.total_size;
            global_stats.successful_snapshots += stats.successful_snapshots;
            global_stats.failed_snapshots += stats.failed_snapshots;
            global_stats.in_progress_snapshots += stats.in_progress_snapshots;
        }

        Ok(global_stats)
    }

    /// Check if a repository exists
    pub fn repository_exists(&self, name: &RepositoryName) -> bool {
        self.repositories.contains_key(name)
    }

    /// Get repository count
    pub fn repository_count(&self) -> usize {
        self.repositories.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tempfile::TempDir;

    fn create_test_config() -> Config {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();

        config.snapshots.repositories = vec![SnapshotRepositoryConfig {
            name: "test_repo".to_string(),
            repository_type: "fs".to_string(),
            settings: crate::config::SnapshotRepositorySettings {
                location: temp_dir.path().to_string_lossy().to_string(),
                ..Default::default()
            },
        }];

        config
    }

    #[tokio::test]
    async fn test_snapshot_manager_creation() {
        let config = create_test_config();
        let manager = SnapshotManager::new(&config).unwrap();

        assert_eq!(manager.repository_count(), 1);
        assert!(manager.repository_exists(&RepositoryName::new("test_repo")));
    }

    #[tokio::test]
    async fn test_get_repository() {
        let config = create_test_config();
        let manager = SnapshotManager::new(&config).unwrap();

        let repo = manager.get_repository(&RepositoryName::new("test_repo"));
        assert!(repo.is_ok());

        let repo = manager.get_repository(&RepositoryName::new("nonexistent"));
        assert!(repo.is_err());
    }

    #[tokio::test]
    async fn test_list_repositories() {
        let config = create_test_config();
        let manager = SnapshotManager::new(&config).unwrap();

        let repos = manager.list_repositories();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].as_str(), "test_repo");
    }

    #[tokio::test]
    async fn test_get_repository_info() {
        let config = create_test_config();
        let manager = SnapshotManager::new(&config).unwrap();

        let info = manager
            .get_repository_info(&RepositoryName::new("test_repo"))
            .await
            .unwrap();
        assert_eq!(info.name.as_str(), "test_repo");
        assert_eq!(info.repository_type, "fs");
    }

    #[tokio::test]
    async fn test_get_global_stats() {
        let config = create_test_config();
        let manager = SnapshotManager::new(&config).unwrap();

        let stats = manager.get_global_stats().await.unwrap();
        assert_eq!(stats.total_snapshots, 0);
        assert_eq!(stats.total_size, 0);
    }

    #[tokio::test]
    async fn test_create_snapshot() {
        let config = create_test_config();
        let manager = SnapshotManager::new(&config).unwrap();

        let repo_name = RepositoryName::new("test_repo");
        let snapshot_name = SnapshotName::new("test_snapshot");
        let request = CreateSnapshotRequest {
            indices: vec![
                crate::types::IndexName::new("index1"),
                crate::types::IndexName::new("index2"),
            ],
            ..Default::default()
        };

        let snapshot_info = manager
            .create_snapshot(&repo_name, snapshot_name.clone(), request)
            .await
            .unwrap();

        assert_eq!(snapshot_info.name, snapshot_name);
        assert_eq!(snapshot_info.repository, repo_name);
        assert_eq!(snapshot_info.state, crate::snapshot::SnapshotState::Success);
        assert_eq!(snapshot_info.indices.len(), 2);
    }

    #[tokio::test]
    async fn test_get_snapshot() {
        let config = create_test_config();
        let manager = SnapshotManager::new(&config).unwrap();

        let repo_name = RepositoryName::new("test_repo");
        let snapshot_name = SnapshotName::new("test_snapshot");
        let request = CreateSnapshotRequest::default();

        // Create snapshot
        manager
            .create_snapshot(&repo_name, snapshot_name.clone(), request)
            .await
            .unwrap();

        // Get snapshot
        let snapshot_info = manager
            .get_snapshot(&repo_name, snapshot_name)
            .await
            .unwrap();

        assert_eq!(snapshot_info.name.as_str(), "test_snapshot");
        assert_eq!(snapshot_info.repository, repo_name);
    }

    #[tokio::test]
    async fn test_list_snapshots() {
        let config = create_test_config();
        let manager = SnapshotManager::new(&config).unwrap();

        let repo_name = RepositoryName::new("test_repo");

        // Initially no snapshots
        let snapshots = manager.list_snapshots(&repo_name).await.unwrap();
        assert_eq!(snapshots.len(), 0);

        // Create snapshots
        let snapshot1 = SnapshotName::new("snapshot1");
        let snapshot2 = SnapshotName::new("snapshot2");
        let request = CreateSnapshotRequest::default();

        manager
            .create_snapshot(&repo_name, snapshot1, request.clone())
            .await
            .unwrap();
        manager
            .create_snapshot(&repo_name, snapshot2, request)
            .await
            .unwrap();

        // List snapshots
        let snapshots = manager.list_snapshots(&repo_name).await.unwrap();
        assert_eq!(snapshots.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_snapshot() {
        let config = create_test_config();
        let manager = SnapshotManager::new(&config).unwrap();

        let repo_name = RepositoryName::new("test_repo");
        let snapshot_name = SnapshotName::new("test_snapshot");
        let request = CreateSnapshotRequest::default();

        // Create snapshot
        manager
            .create_snapshot(&repo_name, snapshot_name.clone(), request)
            .await
            .unwrap();

        // Verify snapshot exists
        let snapshots = manager.list_snapshots(&repo_name).await.unwrap();
        assert_eq!(snapshots.len(), 1);

        // Delete snapshot
        manager
            .delete_snapshot(&repo_name, snapshot_name)
            .await
            .unwrap();

        // Verify snapshot is deleted
        let snapshots = manager.list_snapshots(&repo_name).await.unwrap();
        assert_eq!(snapshots.len(), 0);
    }

    #[tokio::test]
    async fn test_repository_not_found() {
        let config = create_test_config();
        let manager = SnapshotManager::new(&config).unwrap();

        let repo_name = RepositoryName::new("nonexistent_repo");
        let snapshot_name = SnapshotName::new("test_snapshot");
        let request = CreateSnapshotRequest::default();

        let result = manager
            .create_snapshot(&repo_name, snapshot_name, request)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
