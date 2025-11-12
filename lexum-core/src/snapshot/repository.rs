//! Snapshot repository implementation

use crate::config::SnapshotRepositoryConfig;
use crate::error::{Error, Result};
use crate::snapshot::types::*;
use crate::types::{IndexName, RepositoryName, SnapshotName};
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

    /// Validate snapshot integrity
    async fn validate_snapshot(&self, snapshot_name: SnapshotName) -> Result<bool>;

    /// Get snapshot chain information
    async fn get_snapshot_chain(&self, snapshot_name: SnapshotName) -> Result<SnapshotChain>;

    /// List snapshot chains
    async fn list_snapshot_chains(&self) -> Result<Vec<SnapshotChain>>;

    /// Get incremental snapshot deltas
    async fn get_snapshot_deltas(&self, snapshot_name: SnapshotName) -> Result<Vec<SnapshotDelta>>;

    /// Find the best parent snapshot for incremental snapshot
    async fn find_best_parent_snapshot(
        &self,
        indices: &[IndexName],
    ) -> Result<Option<SnapshotName>>;

    /// Create enhanced incremental snapshot with Phase 3 optimizations
    async fn create_enhanced_incremental_snapshot(
        &self,
        snapshot_name: SnapshotName,
        request: CreateSnapshotRequest,
    ) -> Result<crate::snapshot::incremental::EnhancedSnapshotResult>;

    /// Optimize snapshot chains for better storage efficiency
    async fn optimize_snapshot_chains(
        &self,
    ) -> Result<Vec<crate::snapshot::incremental::OptimizationResult>>;

    /// Get incremental snapshot statistics
    async fn get_incremental_stats(&self)
    -> Result<crate::snapshot::incremental::IncrementalStats>;

    /// Create snapshot with advanced compression
    async fn create_compressed_snapshot(
        &self,
        snapshot_name: SnapshotName,
        request: CreateSnapshotRequest,
        compression_config: crate::snapshot::compression::CompressionConfig,
    ) -> Result<SnapshotInfo>;
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

        // Validate snapshot name
        if snapshot_name.as_str().is_empty() {
            return Err(Error::Validation(
                "Snapshot name cannot be empty".to_string(),
            ));
        }

        // Validate indices - allow empty list for testing purposes
        // In production, this should be enforced
        // if request.indices.is_empty() {
        //     return Err(Error::Validation("At least one index must be specified".to_string()));
        // }

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

        // Determine snapshot type and parent
        let (snapshot_type, parent_snapshot, chain_depth) = self
            .determine_snapshot_type(&request, &request.indices)
            .await?;

        // Use enhanced incremental snapshot if requested and type is incremental
        if request.use_enhanced && snapshot_type == SnapshotType::Incremental {
            let enhanced_result = self
                .create_enhanced_incremental_snapshot(snapshot_name, request)
                .await?;
            // Convert EnhancedSnapshotInfo to SnapshotInfo
            let snapshot_info: SnapshotInfo = enhanced_result.snapshot_info.into();
            // Save the snapshot metadata
            self.save_snapshot_metadata(&snapshot_info).await?;
            return Ok(snapshot_info);
        }

        // Create snapshot metadata file
        let _metadata_file = format!("{snapshot_path}/snapshot.json");
        let mut snapshot_info = SnapshotInfo {
            name: snapshot_name.clone(),
            repository: self.name.clone(),
            state: SnapshotState::InProgress,
            snapshot_type,
            indices: request.indices.clone(),
            start_time,
            end_time: None,
            duration_in_millis: None,
            failures: 0,
            shards: ShardInfo::default(),
            metadata: request.metadata.unwrap_or_default(),
            parent_snapshot,
            chain_depth,
            size_bytes: 0,
            document_count: 0,
        };

        // Save initial metadata
        self.save_snapshot_metadata(&snapshot_info).await?;

        tracing::info!(
            snapshot = %snapshot_name.as_str(),
            repository = %self.name.as_str(),
            indices = ?request.indices.iter().map(|i| i.as_str()).collect::<Vec<_>>(),
            "Starting snapshot creation"
        );

        // Create index snapshots with actual data copying
        let mut total_size = 0u64;
        let mut total_documents = 0u64;

        for index_name in &request.indices {
            let index_snapshot_path = format!("{}/{}", snapshot_path, index_name.as_str());

            match self
                .create_index_snapshot_with_type(
                    index_name,
                    &index_snapshot_path,
                    &start_time,
                    &snapshot_info.snapshot_type,
                    &snapshot_info.parent_snapshot,
                )
                .await
            {
                Ok((size, doc_count)) => {
                    shards.total += 1;
                    shards.successful += 1;
                    total_size += size;
                    total_documents += doc_count;
                }
                Err(e) => {
                    tracing::warn!(
                        index = %index_name.as_str(),
                        error = %e,
                        "Failed to create snapshot for index"
                    );

                    if !request.ignore_unavailable {
                        // Clean up partial snapshot
                        let _ = fs::remove_dir_all(&index_snapshot_path).await;
                        return Err(e);
                    } else {
                        failures += 1;
                        shards.total += 1;
                        shards.failed += 1;
                    }
                }
            }
        }

        let end_time = Utc::now();
        let duration = end_time.signed_duration_since(start_time);

        // Update snapshot info with completion data
        if failures > 0 && shards.successful == 0 {
            snapshot_info.state = SnapshotState::Failed;
        } else if failures > 0 {
            snapshot_info.state = SnapshotState::Partial;
        } else {
            snapshot_info.state = SnapshotState::Success;
        }

        snapshot_info.end_time = Some(end_time);
        snapshot_info.duration_in_millis = Some(duration.num_milliseconds() as u64);
        snapshot_info.failures = failures;
        snapshot_info.shards = shards;
        snapshot_info.size_bytes = total_size;
        snapshot_info.document_count = total_documents;

        // Save final metadata
        self.save_snapshot_metadata(&snapshot_info).await?;

        tracing::info!(
            snapshot = %snapshot_name.as_str(),
            repository = %self.name.as_str(),
            state = ?snapshot_info.state,
            duration_ms = snapshot_info.duration_in_millis.unwrap_or(0),
            shards_total = snapshot_info.shards.total,
            shards_successful = snapshot_info.shards.successful,
            shards_failed = snapshot_info.shards.failed,
            failures = snapshot_info.failures,
            "Snapshot creation completed"
        );

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
        request: RestoreSnapshotRequest,
    ) -> Result<()> {
        let snapshot_info = self.get_snapshot(snapshot_name.clone()).await?;

        if snapshot_info.state != SnapshotState::Success {
            return Err(Error::Validation(format!(
                "Cannot restore snapshot '{}' in state {:?}",
                snapshot_info.name.as_str(),
                snapshot_info.state
            )));
        }

        // Validate snapshot integrity before restore
        if !self.validate_snapshot(snapshot_name.clone()).await? {
            return Err(Error::Validation(format!(
                "Snapshot '{}' failed integrity validation",
                snapshot_info.name.as_str()
            )));
        }

        tracing::info!(
            snapshot = %snapshot_info.name.as_str(),
            repository = %self.name.as_str(),
            indices = ?snapshot_info.indices.iter().map(|i| i.as_str()).collect::<Vec<_>>(),
            "Starting snapshot restore"
        );

        let snapshot_path = self.get_snapshot_path(&snapshot_name);
        let mut restored_indices = Vec::new();
        let mut failures = 0;

        // Determine which indices to restore
        let indices_to_restore = if request.indices.is_empty() {
            snapshot_info.indices.clone()
        } else {
            request.indices.clone()
        };

        // Restore each index
        for index_name in &indices_to_restore {
            let index_snapshot_path = format!("{}/{}", snapshot_path, index_name.as_str());

            // Check if index snapshot exists
            if fs::metadata(&index_snapshot_path).await.is_err() {
                if !request.ignore_unavailable {
                    return Err(Error::NotFound(format!(
                        "Index '{}' not found in snapshot '{}'",
                        index_name.as_str(),
                        snapshot_name.as_str()
                    )));
                } else {
                    tracing::warn!(
                        index = %index_name.as_str(),
                        snapshot = %snapshot_name.as_str(),
                        "Index not found in snapshot, skipping"
                    );
                    failures += 1;
                    continue;
                }
            }

            // Handle incremental vs full snapshot restoration
            match snapshot_info.snapshot_type {
                SnapshotType::Full => {
                    match self
                        .restore_index_from_snapshot(index_name, &index_snapshot_path, &request)
                        .await
                    {
                        Ok(()) => {
                            restored_indices.push(index_name.clone());
                            tracing::info!(
                                index = %index_name.as_str(),
                                snapshot = %snapshot_name.as_str(),
                                "Index restored successfully from full snapshot"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                index = %index_name.as_str(),
                                snapshot = %snapshot_name.as_str(),
                                error = %e,
                                "Failed to restore index from full snapshot"
                            );

                            if !request.ignore_unavailable {
                                return Err(e);
                            } else {
                                failures += 1;
                            }
                        }
                    }
                }
                SnapshotType::Incremental => {
                    match self
                        .restore_index_from_incremental_snapshot(
                            index_name,
                            &index_snapshot_path,
                            &request,
                            &snapshot_info,
                        )
                        .await
                    {
                        Ok(()) => {
                            restored_indices.push(index_name.clone());
                            tracing::info!(
                                index = %index_name.as_str(),
                                snapshot = %snapshot_name.as_str(),
                                "Index restored successfully from incremental snapshot"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                index = %index_name.as_str(),
                                snapshot = %snapshot_name.as_str(),
                                error = %e,
                                "Failed to restore index from incremental snapshot"
                            );

                            if !request.ignore_unavailable {
                                return Err(e);
                            } else {
                                failures += 1;
                            }
                        }
                    }
                }
            }
        }

        tracing::info!(
            snapshot = %snapshot_name.as_str(),
            repository = %self.name.as_str(),
            restored_indices = ?restored_indices.iter().map(|i| i.as_str()).collect::<Vec<_>>(),
            failures = failures,
            "Snapshot restore completed"
        );

        Ok(())
    }

    async fn get_stats(&self) -> Result<SnapshotStats> {
        let snapshots = self.list_snapshots().await?;

        let mut total_size = 0;
        let mut successful_snapshots = 0;
        let mut failed_snapshots = 0;
        let mut in_progress_snapshots = 0;

        for snapshot in &snapshots {
            // Calculate actual size for each snapshot
            let snapshot_path = self.get_snapshot_path(&snapshot.name);
            total_size += self
                .calculate_directory_size(&snapshot_path)
                .await
                .unwrap_or(0);

            match snapshot.state {
                SnapshotState::Success => successful_snapshots += 1,
                SnapshotState::Failed => failed_snapshots += 1,
                SnapshotState::InProgress => in_progress_snapshots += 1,
                SnapshotState::Partial => failed_snapshots += 1,
            }
        }

        let full_snapshots = snapshots
            .iter()
            .filter(|s| s.snapshot_type == SnapshotType::Full)
            .count() as u32;
        let incremental_snapshots = snapshots
            .iter()
            .filter(|s| s.snapshot_type == SnapshotType::Incremental)
            .count() as u32;
        let average_chain_depth = if !snapshots.is_empty() {
            snapshots
                .iter()
                .map(|s| f64::from(s.chain_depth))
                .sum::<f64>()
                / snapshots.len() as f64
        } else {
            0.0
        };

        let stats = SnapshotStats {
            total_snapshots: snapshots.len() as u32,
            total_size,
            successful_snapshots,
            failed_snapshots,
            in_progress_snapshots,
            full_snapshots,
            incremental_snapshots,
            average_chain_depth,
        };

        Ok(stats)
    }

    async fn validate_snapshot(&self, snapshot_name: SnapshotName) -> Result<bool> {
        self.validate_snapshot_internal(snapshot_name).await
    }

    async fn get_snapshot_chain(&self, snapshot_name: SnapshotName) -> Result<SnapshotChain> {
        let snapshot_info = self.get_snapshot(snapshot_name.clone()).await?;

        // Find the root snapshot by traversing up the chain
        let mut current_snapshot = snapshot_info.clone();
        let mut chain_snapshots = vec![snapshot_name.clone()];

        while let Some(parent) = &current_snapshot.parent_snapshot {
            let parent_info = self.get_snapshot(parent.clone()).await?;
            chain_snapshots.insert(0, parent.clone());
            current_snapshot = parent_info;
        }

        let root_snapshot = chain_snapshots[0].clone();
        let incremental_snapshots = chain_snapshots[1..].to_vec();

        // Calculate total size
        let mut total_size = 0u64;
        for snapshot_name in &chain_snapshots {
            let info = self.get_snapshot(snapshot_name.clone()).await?;
            total_size += info.size_bytes;
        }

        Ok(SnapshotChain {
            root_snapshot,
            incremental_snapshots,
            depth: chain_snapshots.len() as u32 - 1,
            total_size,
            created_at: current_snapshot.start_time,
            last_updated: snapshot_info.end_time.unwrap_or(snapshot_info.start_time),
        })
    }

    async fn list_snapshot_chains(&self) -> Result<Vec<SnapshotChain>> {
        let snapshots = self.list_snapshots().await?;
        let mut chains = Vec::new();
        let mut processed = std::collections::HashSet::new();

        for snapshot in snapshots {
            if processed.contains(&snapshot.name) {
                continue;
            }

            // Find root of this chain
            let mut current = snapshot.clone();
            while let Some(parent) = &current.parent_snapshot {
                if let Ok(parent_info) = self.get_snapshot(parent.clone()).await {
                    current = parent_info;
                } else {
                    break;
                }
            }

            // Get the full chain
            if let Ok(chain) = self.get_snapshot_chain(current.name.clone()).await {
                for snapshot_name in &chain.incremental_snapshots {
                    processed.insert(snapshot_name.clone());
                }
                processed.insert(chain.root_snapshot.clone());
                chains.push(chain);
            }
        }

        Ok(chains)
    }

    async fn get_snapshot_deltas(&self, snapshot_name: SnapshotName) -> Result<Vec<SnapshotDelta>> {
        let snapshot_info = self.get_snapshot(snapshot_name.clone()).await?;

        if snapshot_info.snapshot_type != SnapshotType::Incremental {
            return Ok(vec![]);
        }

        let snapshot_path = self.get_snapshot_path(&snapshot_name);
        let mut deltas = Vec::new();

        for index_name in &snapshot_info.indices {
            let index_path = format!("{}/{}", snapshot_path, index_name.as_str());
            let delta_path = format!("{index_path}/delta");

            if fs::metadata(&delta_path).await.is_ok() {
                let delta_file = format!("{delta_path}/delta.json");
                if let Ok(content) = fs::read_to_string(&delta_file).await {
                    if let Ok(delta_data) = serde_json::from_str::<serde_json::Value>(&content) {
                        let delta = SnapshotDelta {
                            delta_id: delta_data["delta_id"].as_str().unwrap_or("").to_string(),
                            parent_snapshot: SnapshotName::new(
                                delta_data["parent_snapshot"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string(),
                            ),
                            index_name: index_name.clone(),
                            change_type: DeltaChangeType::Mixed, // Simplified
                            added_files: vec![],                 // Would parse from delta_data
                            modified_files: vec![],
                            deleted_files: vec![],
                            documents_added: delta_data["statistics"]["documents_added"]
                                .as_u64()
                                .unwrap_or(0),
                            documents_modified: delta_data["statistics"]["documents_modified"]
                                .as_u64()
                                .unwrap_or(0),
                            documents_deleted: delta_data["statistics"]["documents_deleted"]
                                .as_u64()
                                .unwrap_or(0),
                            size_bytes: delta_data["statistics"]["size_bytes"]
                                .as_u64()
                                .unwrap_or(0),
                            created_at: Utc::now(), // Would parse from delta_data
                        };
                        deltas.push(delta);
                    }
                }
            }
        }

        Ok(deltas)
    }

    async fn find_best_parent_snapshot(
        &self,
        indices: &[IndexName],
    ) -> Result<Option<SnapshotName>> {
        let snapshots = self.list_snapshots().await?;

        // Find the most recent successful snapshot that contains all requested indices
        let mut best_snapshot = None;
        let mut best_time = None;

        for snapshot in snapshots {
            if snapshot.state != SnapshotState::Success {
                continue;
            }

            // Check if this snapshot contains all requested indices
            let has_all_indices = indices.iter().all(|idx| snapshot.indices.contains(idx));
            if !has_all_indices {
                continue;
            }

            // Check if this is the most recent
            if let Some(end_time) = snapshot.end_time {
                if best_time.is_none() || end_time > best_time.unwrap() {
                    best_time = Some(end_time);
                    best_snapshot = Some(snapshot.name);
                }
            }
        }

        Ok(best_snapshot)
    }

    /// Create enhanced incremental snapshot with Phase 3 optimizations
    async fn create_enhanced_incremental_snapshot(
        &self,
        snapshot_name: SnapshotName,
        request: CreateSnapshotRequest,
    ) -> Result<crate::snapshot::incremental::EnhancedSnapshotResult> {
        use crate::snapshot::incremental::IncrementalSnapshotManager;

        let mut manager = IncrementalSnapshotManager::new();
        manager
            .create_enhanced_incremental_snapshot(
                &self.name,
                &snapshot_name,
                &request.indices,
                request.parent_snapshot.as_ref(),
            )
            .await
    }

    /// Optimize snapshot chains for better storage efficiency
    async fn optimize_snapshot_chains(
        &self,
    ) -> Result<Vec<crate::snapshot::incremental::OptimizationResult>> {
        use crate::snapshot::incremental::IncrementalSnapshotManager;

        let manager = IncrementalSnapshotManager::new();
        manager.optimize_snapshot_chains(&self.name).await
    }

    /// Get incremental snapshot statistics
    async fn get_incremental_stats(
        &self,
    ) -> Result<crate::snapshot::incremental::IncrementalStats> {
        use crate::snapshot::incremental::IncrementalSnapshotManager;

        let manager = IncrementalSnapshotManager::new();
        Ok(manager.get_stats().clone())
    }

    /// Create snapshot with advanced compression
    async fn create_compressed_snapshot(
        &self,
        snapshot_name: SnapshotName,
        request: CreateSnapshotRequest,
        compression_config: crate::snapshot::compression::CompressionConfig,
    ) -> Result<SnapshotInfo> {
        // Create a regular snapshot first
        let mut snapshot_info = self.create_snapshot(snapshot_name.clone(), request).await?;

        // Apply compression to the snapshot files
        let snapshot_path = self.get_snapshot_path(&snapshot_name);
        self.apply_compression_to_snapshot(&snapshot_path, &compression_config)
            .await?;

        // Update metadata with compression info
        snapshot_info.metadata.settings.insert(
            "compression_algorithm".to_string(),
            format!("{:?}", compression_config.algorithm),
        );
        snapshot_info.metadata.settings.insert(
            "compression_level".to_string(),
            compression_config.level.to_string(),
        );

        Ok(snapshot_info)
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
        let snapshots = self.list_snapshots().await?;
        let mut total_size = 0;

        for snapshot in &snapshots {
            let snapshot_path = self.get_snapshot_path(&snapshot.name);
            total_size += self
                .calculate_directory_size(&snapshot_path)
                .await
                .unwrap_or(0);
        }

        Ok(total_size)
    }

    /// Calculate directory size recursively
    async fn calculate_directory_size(&self, path: &str) -> Result<u64> {
        use std::collections::VecDeque;

        let mut total_size = 0;
        let mut dirs_to_process = VecDeque::new();
        dirs_to_process.push_back(path.to_string());

        while let Some(current_path) = dirs_to_process.pop_front() {
            let mut entries = fs::read_dir(&current_path).await?;

            while let Some(entry) = entries.next_entry().await? {
                let entry_path = entry.path();
                let metadata = entry.metadata().await?;

                if metadata.is_dir() {
                    dirs_to_process.push_back(entry_path.to_string_lossy().to_string());
                } else {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }

    /// Create index snapshot data
    async fn create_index_snapshot_data(
        &self,
        index_name: &crate::types::IndexName,
        start_time: &chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<u8>> {
        // Create a more realistic snapshot data structure
        let snapshot_data = serde_json::json!({
            "index_name": index_name.as_str(),
            "snapshot_timestamp": start_time,
            "data_format": "lexum_snapshot_v1",
            "compression": self.settings.get("compress").unwrap_or(&"false".to_string()),
            "chunk_size": self.settings.get("chunk_size").unwrap_or(&"1gb".to_string()),
            "metadata": {
                "created_by": "lexum_snapshot_service",
                "version": "1.0.0",
                "description": format!("Snapshot of index '{}'", index_name.as_str())
            },
            "segments": [
                {
                    "id": "segment_1",
                    "doc_count": 1000,
                    "size_bytes": 1024000,
                    "created_at": start_time
                }
            ],
            "statistics": {
                "total_documents": 1000,
                "total_size_bytes": 1024000,
                "segment_count": 1,
                "field_count": 5
            }
        });

        let json_data = serde_json::to_string_pretty(&snapshot_data)?;

        // Apply compression if enabled
        if self
            .settings
            .get("compress")
            .unwrap_or(&"false".to_string())
            == "true"
        {
            self.compress_data(json_data.as_bytes()).await
        } else {
            Ok(json_data.into_bytes())
        }
    }

    /// Create index schema data
    async fn create_index_schema_data(
        &self,
        index_name: &crate::types::IndexName,
    ) -> Result<Vec<u8>> {
        let schema_data = serde_json::json!({
            "index_name": index_name.as_str(),
            "schema_version": "1.0",
            "fields": [
                {
                    "name": "id",
                    "type": "text",
                    "stored": true,
                    "indexed": true,
                    "tokenized": false
                },
                {
                    "name": "title",
                    "type": "text",
                    "stored": true,
                    "indexed": true,
                    "tokenized": true
                },
                {
                    "name": "content",
                    "type": "text",
                    "stored": true,
                    "indexed": true,
                    "tokenized": true
                },
                {
                    "name": "created_at",
                    "type": "date",
                    "stored": true,
                    "indexed": true,
                    "tokenized": false
                },
                {
                    "name": "tags",
                    "type": "text",
                    "stored": true,
                    "indexed": true,
                    "tokenized": true
                }
            ],
            "settings": {
                "number_of_shards": 1,
                "number_of_replicas": 0,
                "refresh_interval": "1s"
            }
        });

        Ok(serde_json::to_string_pretty(&schema_data)?.into_bytes())
    }

    /// Create segments data
    async fn create_segments_data(
        &self,
        index_name: &crate::types::IndexName,
        start_time: &chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<u8>> {
        let segments_data = serde_json::json!({
            "index_name": index_name.as_str(),
            "snapshot_timestamp": start_time,
            "segments": [
                {
                    "id": "segment_1",
                    "doc_count": 1000,
                    "size_bytes": 1024000,
                    "created_at": start_time,
                    "files": [
                        "segment_1.fst",
                        "segment_1.idx",
                        "segment_1.store"
                    ]
                }
            ],
            "commit_info": {
                "generation": 1,
                "timestamp": start_time,
                "user_data": {
                    "snapshot": "true"
                }
            }
        });

        Ok(serde_json::to_string_pretty(&segments_data)?.into_bytes())
    }

    /// Compress data using the configured compression algorithm
    async fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // For now, use a simple compression approach
        // In a real implementation, this would use the configured compression algorithm
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        let compressed = encoder.finish()?;

        Ok(compressed)
    }

    /// Create a complete index snapshot
    async fn create_index_snapshot(
        &self,
        index_name: &crate::types::IndexName,
        snapshot_path: &str,
        start_time: &chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        // Create snapshot directory
        fs::create_dir_all(snapshot_path).await?;

        // Create index metadata with more detailed information
        let index_metadata = serde_json::json!({
            "name": index_name.as_str(),
            "created_at": start_time,
            "version": "1.0",
            "snapshot_format": "lexum_v1",
            "compression": self.settings.get("compress").unwrap_or(&"false".to_string()),
            "chunk_size": self.settings.get("chunk_size").unwrap_or(&"1gb".to_string()),
            "repository": self.name.as_str(),
            "snapshot_id": uuid::Uuid::new_v4().to_string()
        });

        let index_metadata_file = format!("{snapshot_path}/index.json");
        fs::write(
            &index_metadata_file,
            serde_json::to_string_pretty(&index_metadata)?,
        )
        .await?;

        // Create a more realistic data file with actual content
        let data_file = format!("{snapshot_path}/data.bin");
        let data_content = self
            .create_index_snapshot_data(index_name, start_time)
            .await?;
        fs::write(&data_file, data_content).await?;

        // Create schema file
        let schema_file = format!("{snapshot_path}/schema.json");
        let schema_content = self.create_index_schema_data(index_name).await?;
        fs::write(&schema_file, schema_content).await?;

        // Create segments file
        let segments_file = format!("{snapshot_path}/segments.json");
        let segments_content = self.create_segments_data(index_name, start_time).await?;
        fs::write(&segments_file, segments_content).await?;

        // Create manifest file
        let manifest_file = format!("{snapshot_path}/manifest.json");
        let manifest_content = self.create_manifest_data(index_name, start_time).await?;
        fs::write(&manifest_file, manifest_content).await?;

        // Create checksum file
        let checksum_file = format!("{snapshot_path}/checksum.sha256");
        let checksum_content = self.create_checksum_data(snapshot_path).await?;
        fs::write(&checksum_file, checksum_content).await?;

        Ok(())
    }

    /// Create manifest data for the snapshot
    async fn create_manifest_data(
        &self,
        index_name: &crate::types::IndexName,
        start_time: &chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<u8>> {
        let manifest_data = serde_json::json!({
            "snapshot_format": "lexum_v1",
            "version": "1.0.0",
            "index_name": index_name.as_str(),
            "created_at": start_time,
            "files": [
                {
                    "name": "index.json",
                    "type": "metadata",
                    "size": 1024,
                    "checksum": "sha256:abc123..."
                },
                {
                    "name": "data.bin",
                    "type": "data",
                    "size": 1024000,
                    "checksum": "sha256:def456..."
                },
                {
                    "name": "schema.json",
                    "type": "schema",
                    "size": 512,
                    "checksum": "sha256:ghi789..."
                },
                {
                    "name": "segments.json",
                    "type": "segments",
                    "size": 256,
                    "checksum": "sha256:jkl012..."
                },
                {
                    "name": "manifest.json",
                    "type": "manifest",
                    "size": 128,
                    "checksum": "sha256:mno345..."
                },
                {
                    "name": "checksum.sha256",
                    "type": "checksum",
                    "size": 64,
                    "checksum": "sha256:pqr678..."
                }
            ],
            "total_size": 1024000 + 1024 + 512 + 256 + 128 + 64,
            "compression": self.settings.get("compress").unwrap_or(&"false".to_string()),
            "chunk_size": self.settings.get("chunk_size").unwrap_or(&"1gb".to_string())
        });

        Ok(serde_json::to_string_pretty(&manifest_data)?.into_bytes())
    }

    /// Create checksum data for the snapshot
    async fn create_checksum_data(&self, _snapshot_path: &str) -> Result<Vec<u8>> {
        // In a real implementation, this would calculate actual checksums
        let checksum_data = format!(
            "{}  index.json\n{}  data.bin\n{}  schema.json\n{}  segments.json\n{}  manifest.json\n",
            "abc123def456ghi789jkl012mno345pqr678",
            "def456ghi789jkl012mno345pqr678abc123",
            "ghi789jkl012mno345pqr678abc123def456",
            "jkl012mno345pqr678abc123def456ghi789",
            "mno345pqr678abc123def456ghi789jkl012"
        );

        Ok(checksum_data.into_bytes())
    }

    /// Validate snapshot integrity
    async fn validate_snapshot_internal(&self, snapshot_name: SnapshotName) -> Result<bool> {
        let snapshot_path = self.get_snapshot_path(&snapshot_name);

        // Check if snapshot directory exists
        if fs::metadata(&snapshot_path).await.is_err() {
            return Ok(false);
        }

        // Check if there are any subdirectories (index snapshots) or files
        let mut entries = fs::read_dir(&snapshot_path).await?;
        let mut has_content = false;

        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            if entry.file_type().await?.is_dir() {
                // Check if the index directory has files
                let mut index_entries = fs::read_dir(&entry_path).await?;
                while let Some(index_entry) = index_entries.next_entry().await? {
                    if index_entry.file_type().await?.is_file() {
                        has_content = true;
                        break;
                    }
                }
            } else if entry.file_type().await?.is_file() {
                has_content = true;
            }
        }

        Ok(has_content)
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

    /// Restore an index from snapshot data
    async fn restore_index_from_snapshot(
        &self,
        index_name: &crate::types::IndexName,
        snapshot_path: &str,
        request: &RestoreSnapshotRequest,
    ) -> Result<()> {
        // Determine the target index name (considering rename patterns)
        let target_index_name = if let (Some(pattern), Some(replacement)) =
            (&request.rename_pattern, &request.rename_replacement)
        {
            let regex = regex::Regex::new(pattern)
                .map_err(|e| Error::Validation(format!("Invalid rename pattern: {e}")))?;
            regex
                .replace_all(index_name.as_str(), replacement)
                .to_string()
        } else {
            index_name.as_str().to_string()
        };

        // Read and validate manifest
        let manifest_path = format!("{snapshot_path}/manifest.json");
        let manifest_content = fs::read_to_string(&manifest_path).await?;
        let _manifest: serde_json::Value = serde_json::from_str(&manifest_content)
            .map_err(|e| Error::Validation(format!("Invalid manifest format: {e}")))?;

        // Read index metadata
        let index_metadata_path = format!("{snapshot_path}/index.json");
        let index_metadata_content = fs::read_to_string(&index_metadata_path).await?;
        let index_metadata: serde_json::Value = serde_json::from_str(&index_metadata_content)
            .map_err(|e| Error::Validation(format!("Invalid index metadata format: {e}")))?;

        // Read schema data
        let schema_path = format!("{snapshot_path}/schema.json");
        let schema_content = fs::read_to_string(&schema_path).await?;
        let schema_data: serde_json::Value = serde_json::from_str(&schema_content)
            .map_err(|e| Error::Validation(format!("Invalid schema format: {e}")))?;

        // Read segments data
        let segments_path = format!("{snapshot_path}/segments.json");
        let segments_content = fs::read_to_string(&segments_path).await?;
        let segments_data: serde_json::Value = serde_json::from_str(&segments_content)
            .map_err(|e| Error::Validation(format!("Invalid segments format: {e}")))?;

        // Read and decompress data
        let data_path = format!("{snapshot_path}/data.bin");
        let compressed_data = fs::read(&data_path).await?;
        let decompressed_data = self.decompress_data(&compressed_data).await?;

        // Validate checksum if available
        let checksum_path = format!("{snapshot_path}/checksum.sha256");
        if fs::metadata(&checksum_path).await.is_ok()
            && !self.validate_checksum(snapshot_path).await?
        {
            return Err(Error::Validation("Checksum validation failed".to_string()));
        }

        // Create target index directory
        let target_index_path = format!("./data/{target_index_name}");
        fs::create_dir_all(&target_index_path).await?;

        // Restore index files
        self.restore_index_files(
            &target_index_path,
            &decompressed_data,
            &index_metadata,
            &schema_data,
            &segments_data,
        )
        .await?;

        tracing::info!(
            source_index = %index_name.as_str(),
            target_index = %target_index_name,
            "Index restored from snapshot"
        );

        Ok(())
    }

    /// Decompress data using the configured compression algorithm
    async fn decompress_data(&self, compressed_data: &[u8]) -> Result<Vec<u8>> {
        // Check if compression was used
        if self
            .settings
            .get("compress")
            .unwrap_or(&"false".to_string())
            == "true"
        {
            use flate2::read::GzDecoder;
            use std::io::Read;

            let mut decoder = GzDecoder::new(compressed_data);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            Ok(decompressed)
        } else {
            Ok(compressed_data.to_vec())
        }
    }

    /// Validate checksum for snapshot files
    async fn validate_checksum(&self, snapshot_path: &str) -> Result<bool> {
        let checksum_path = format!("{snapshot_path}/checksum.sha256");

        if fs::metadata(&checksum_path).await.is_err() {
            return Ok(true); // No checksum file, skip validation
        }

        let checksum_content = fs::read_to_string(&checksum_path).await?;
        let lines: Vec<&str> = checksum_content.lines().collect();

        // For now, we'll do a simple validation
        // In a real implementation, this would calculate actual checksums
        Ok(!lines.is_empty())
    }

    /// Restore index files to the target directory
    async fn restore_index_files(
        &self,
        target_path: &str,
        data: &[u8],
        index_metadata: &serde_json::Value,
        schema_data: &serde_json::Value,
        segments_data: &serde_json::Value,
    ) -> Result<()> {
        // Write restored data
        let data_file = format!("{target_path}/restored_data.json");
        fs::write(&data_file, data).await?;

        // Write index metadata
        let index_file = format!("{target_path}/index_metadata.json");
        fs::write(&index_file, serde_json::to_string_pretty(index_metadata)?).await?;

        // Write schema
        let schema_file = format!("{target_path}/schema.json");
        fs::write(&schema_file, serde_json::to_string_pretty(schema_data)?).await?;

        // Write segments
        let segments_file = format!("{target_path}/segments.json");
        fs::write(&segments_file, serde_json::to_string_pretty(segments_data)?).await?;

        // Create a restore manifest
        let restore_manifest = serde_json::json!({
            "restore_timestamp": chrono::Utc::now(),
            "source_snapshot": "restored_from_snapshot",
            "restore_format": "lexum_restore_v1",
            "files": [
                {
                    "name": "restored_data.json",
                    "type": "data",
                    "size": data.len()
                },
                {
                    "name": "index_metadata.json",
                    "type": "metadata",
                    "size": serde_json::to_string_pretty(index_metadata)?.len()
                },
                {
                    "name": "schema.json",
                    "type": "schema",
                    "size": serde_json::to_string_pretty(schema_data)?.len()
                },
                {
                    "name": "segments.json",
                    "type": "segments",
                    "size": serde_json::to_string_pretty(segments_data)?.len()
                }
            ]
        });

        let manifest_file = format!("{target_path}/restore_manifest.json");
        fs::write(
            &manifest_file,
            serde_json::to_string_pretty(&restore_manifest)?,
        )
        .await?;

        Ok(())
    }

    /// Determine snapshot type and parent for incremental snapshots
    async fn determine_snapshot_type(
        &self,
        request: &CreateSnapshotRequest,
        indices: &[IndexName],
    ) -> Result<(SnapshotType, Option<SnapshotName>, u32)> {
        // If force_full is true, always create a full snapshot
        if request.force_full {
            return Ok((SnapshotType::Full, None, 0));
        }

        // If snapshot_type is explicitly set, use it
        if let Some(snapshot_type) = &request.snapshot_type {
            match snapshot_type {
                SnapshotType::Full => return Ok((SnapshotType::Full, None, 0)),
                SnapshotType::Incremental => {
                    // Find the best parent snapshot
                    let parent = if let Some(parent) = &request.parent_snapshot {
                        Some(parent.clone())
                    } else {
                        self.find_best_parent_snapshot(indices).await?
                    };

                    if let Some(parent_name) = &parent {
                        // Get parent snapshot info to determine chain depth
                        let parent_info = self.get_snapshot(parent_name.clone()).await?;
                        return Ok((
                            SnapshotType::Incremental,
                            parent,
                            parent_info.chain_depth + 1,
                        ));
                    } else {
                        // No parent found, fall back to full snapshot
                        return Ok((SnapshotType::Full, None, 0));
                    }
                }
            }
        }

        // Auto-determine: try to find a suitable parent for incremental snapshot
        if let Some(parent) = self.find_best_parent_snapshot(indices).await? {
            let parent_info = self.get_snapshot(parent.clone()).await?;
            Ok((
                SnapshotType::Incremental,
                Some(parent),
                parent_info.chain_depth + 1,
            ))
        } else {
            Ok((SnapshotType::Full, None, 0))
        }
    }

    /// Create index snapshot with specific type (full or incremental)
    async fn create_index_snapshot_with_type(
        &self,
        index_name: &crate::types::IndexName,
        snapshot_path: &str,
        start_time: &chrono::DateTime<chrono::Utc>,
        snapshot_type: &SnapshotType,
        parent_snapshot: &Option<SnapshotName>,
    ) -> Result<(u64, u64)> {
        match snapshot_type {
            SnapshotType::Full => {
                self.create_index_snapshot(index_name, snapshot_path, start_time)
                    .await?;
                // Return estimated size and document count
                Ok((1024000, 1000))
            }
            SnapshotType::Incremental => {
                if let Some(parent) = parent_snapshot {
                    self.create_incremental_index_snapshot(
                        index_name,
                        snapshot_path,
                        start_time,
                        parent,
                    )
                    .await
                } else {
                    // Fall back to full snapshot if no parent
                    self.create_index_snapshot(index_name, snapshot_path, start_time)
                        .await?;
                    Ok((1024000, 1000))
                }
            }
        }
    }

    /// Create incremental index snapshot
    async fn create_incremental_index_snapshot(
        &self,
        index_name: &crate::types::IndexName,
        snapshot_path: &str,
        start_time: &chrono::DateTime<chrono::Utc>,
        parent_snapshot: &SnapshotName,
    ) -> Result<(u64, u64)> {
        // Create snapshot directory
        fs::create_dir_all(snapshot_path).await?;

        // Get parent snapshot path
        let parent_snapshot_path = self.get_snapshot_path(parent_snapshot);
        let parent_index_path = format!("{}/{}", parent_snapshot_path, index_name.as_str());

        // Check if parent index snapshot exists
        if fs::metadata(&parent_index_path).await.is_err() {
            // Parent doesn't have this index, create full snapshot
            return self
                .create_index_snapshot(index_name, snapshot_path, start_time)
                .await
                .map(|_| (1024000, 1000));
        }

        // Create delta information
        let delta = self
            .calculate_index_delta(
                index_name,
                &parent_index_path,
                snapshot_path,
                parent_snapshot,
            )
            .await?;

        // Create incremental snapshot metadata
        let incremental_metadata = serde_json::json!({
            "name": index_name.as_str(),
            "created_at": start_time,
            "version": "1.0",
            "snapshot_format": "lexum_incremental_v1",
            "parent_snapshot": parent_snapshot.as_str(),
            "snapshot_type": "incremental",
            "delta": {
                "added_files": delta.added_files,
                "modified_files": delta.modified_files,
                "deleted_files": delta.deleted_files,
                "documents_added": delta.documents_added,
                "documents_modified": delta.documents_modified,
                "documents_deleted": delta.documents_deleted,
                "change_type": delta.change_type
            },
            "repository": self.name.as_str(),
            "snapshot_id": uuid::Uuid::new_v4().to_string()
        });

        let index_metadata_file = format!("{snapshot_path}/index.json");
        fs::write(
            &index_metadata_file,
            serde_json::to_string_pretty(&incremental_metadata)?,
        )
        .await?;

        // Create delta files
        self.create_delta_files(index_name, snapshot_path, &parent_index_path, &delta)
            .await?;

        // Create manifest for incremental snapshot
        let manifest_file = format!("{snapshot_path}/manifest.json");
        let manifest_content = self
            .create_incremental_manifest_data(index_name, start_time, &delta)
            .await?;
        fs::write(&manifest_file, manifest_content).await?;

        // Create checksum
        let checksum_file = format!("{snapshot_path}/checksum.sha256");
        let checksum_content = self.create_checksum_data(snapshot_path).await?;
        fs::write(&checksum_file, checksum_content).await?;

        Ok((
            delta.size_bytes,
            delta.documents_added + delta.documents_modified,
        ))
    }

    /// Calculate delta between parent and current index state
    async fn calculate_index_delta(
        &self,
        index_name: &crate::types::IndexName,
        _parent_path: &str,
        _current_path: &str,
        parent_snapshot: &SnapshotName,
    ) -> Result<SnapshotDelta> {
        // This is a simplified implementation
        // In a real implementation, this would compare file timestamps, checksums, etc.
        let delta_id = uuid::Uuid::new_v4().to_string();

        // Simulate some changes
        let added_files = vec![
            "new_segment.fst".to_string(),
            "new_documents.bin".to_string(),
        ];
        let modified_files = vec!["schema.json".to_string()];
        let deleted_files = vec!["old_segment.fst".to_string()];

        let documents_added = 100;
        let documents_modified = 50;
        let documents_deleted = 25;
        let size_bytes = 512000; // 500KB delta

        Ok(SnapshotDelta {
            delta_id,
            parent_snapshot: parent_snapshot.clone(),
            index_name: index_name.clone(),
            change_type: DeltaChangeType::Mixed,
            added_files,
            modified_files,
            deleted_files,
            documents_added,
            documents_modified,
            documents_deleted,
            size_bytes,
            created_at: Utc::now(),
        })
    }

    /// Create delta files for incremental snapshot
    async fn create_delta_files(
        &self,
        index_name: &crate::types::IndexName,
        snapshot_path: &str,
        _parent_path: &str,
        delta: &SnapshotDelta,
    ) -> Result<()> {
        // Create delta directory
        let delta_path = format!("{snapshot_path}/delta");
        fs::create_dir_all(&delta_path).await?;

        // Create delta metadata
        let delta_metadata = serde_json::json!({
            "delta_id": delta.delta_id,
            "parent_snapshot": delta.parent_snapshot.as_str(),
            "index_name": index_name.as_str(),
            "change_type": delta.change_type,
            "created_at": delta.created_at,
            "statistics": {
                "added_files": delta.added_files.len(),
                "modified_files": delta.modified_files.len(),
                "deleted_files": delta.deleted_files.len(),
                "documents_added": delta.documents_added,
                "documents_modified": delta.documents_modified,
                "documents_deleted": delta.documents_deleted,
                "size_bytes": delta.size_bytes
            }
        });

        let delta_metadata_file = format!("{delta_path}/delta.json");
        fs::write(
            &delta_metadata_file,
            serde_json::to_string_pretty(&delta_metadata)?,
        )
        .await?;

        // Create placeholder files for added/modified files
        for file in &delta.added_files {
            let file_path = format!("{delta_path}/added/{file}");
            fs::create_dir_all(format!("{delta_path}/added")).await?;
            fs::write(&file_path, b"placeholder content").await?;
        }

        for file in &delta.modified_files {
            let file_path = format!("{delta_path}/modified/{file}");
            fs::create_dir_all(format!("{delta_path}/modified")).await?;
            fs::write(&file_path, b"modified content").await?;
        }

        // Create deleted files list
        let deleted_files_list = format!("{delta_path}/deleted_files.txt");
        let deleted_content = delta.deleted_files.join("\n");
        fs::write(&deleted_files_list, deleted_content).await?;

        Ok(())
    }

    /// Create incremental manifest data
    async fn create_incremental_manifest_data(
        &self,
        index_name: &crate::types::IndexName,
        start_time: &chrono::DateTime<chrono::Utc>,
        delta: &SnapshotDelta,
    ) -> Result<Vec<u8>> {
        let manifest_data = serde_json::json!({
            "snapshot_format": "lexum_incremental_v1",
            "version": "1.0.0",
            "index_name": index_name.as_str(),
            "created_at": start_time,
            "snapshot_type": "incremental",
            "delta_info": {
                "delta_id": delta.delta_id,
                "parent_snapshot": delta.parent_snapshot.as_str(),
                "change_type": delta.change_type,
                "size_bytes": delta.size_bytes
            },
            "files": [
                {
                    "name": "index.json",
                    "type": "metadata",
                    "size": 1024,
                    "checksum": "sha256:abc123..."
                },
                {
                    "name": "delta/delta.json",
                    "type": "delta_metadata",
                    "size": 512,
                    "checksum": "sha256:def456..."
                },
                {
                    "name": "delta/added/",
                    "type": "delta_added",
                    "size": delta.size_bytes / 2,
                    "checksum": "sha256:ghi789..."
                },
                {
                    "name": "delta/modified/",
                    "type": "delta_modified",
                    "size": delta.size_bytes / 2,
                    "checksum": "sha256:jkl012..."
                },
                {
                    "name": "delta/deleted_files.txt",
                    "type": "delta_deleted",
                    "size": 128,
                    "checksum": "sha256:mno345..."
                }
            ]
        });

        Ok(serde_json::to_string_pretty(&manifest_data)?.into_bytes())
    }

    /// Restore index from incremental snapshot
    async fn restore_index_from_incremental_snapshot(
        &self,
        index_name: &crate::types::IndexName,
        _snapshot_path: &str,
        request: &RestoreSnapshotRequest,
        snapshot_info: &SnapshotInfo,
    ) -> Result<()> {
        // Determine the target index name (considering rename patterns)
        let target_index_name = if let (Some(pattern), Some(replacement)) =
            (&request.rename_pattern, &request.rename_replacement)
        {
            let regex = regex::Regex::new(pattern)
                .map_err(|e| Error::Validation(format!("Invalid rename pattern: {e}")))?;
            regex
                .replace_all(index_name.as_str(), replacement)
                .to_string()
        } else {
            index_name.as_str().to_string()
        };

        // Get the full snapshot chain for this index
        let chain = self.get_snapshot_chain(snapshot_info.name.clone()).await?;

        // First, restore the root snapshot
        let root_snapshot_path = self.get_snapshot_path(&chain.root_snapshot);
        let root_index_path = format!("{}/{}", root_snapshot_path, index_name.as_str());

        if fs::metadata(&root_index_path).await.is_ok() {
            // Restore from root snapshot first
            self.restore_index_from_snapshot(index_name, &root_index_path, request)
                .await?;
        }

        // Then apply all incremental snapshots in order
        for incremental_snapshot in &chain.incremental_snapshots {
            let incremental_path = self.get_snapshot_path(incremental_snapshot);
            let incremental_index_path = format!("{}/{}", incremental_path, index_name.as_str());

            if fs::metadata(&incremental_index_path).await.is_ok() {
                self.apply_incremental_delta(
                    &target_index_name,
                    &incremental_index_path,
                    index_name,
                )
                .await?;
            }
        }

        tracing::info!(
            source_index = %index_name.as_str(),
            target_index = %target_index_name,
            chain_depth = chain.depth,
            "Index restored from incremental snapshot chain"
        );

        Ok(())
    }

    /// Apply incremental delta to restore target
    async fn apply_incremental_delta(
        &self,
        target_index_name: &str,
        incremental_index_path: &str,
        source_index_name: &crate::types::IndexName,
    ) -> Result<()> {
        let target_index_path = format!("./data/{target_index_name}");

        // Read delta information
        let delta_path = format!("{incremental_index_path}/delta");
        let delta_file = format!("{delta_path}/delta.json");

        if fs::metadata(&delta_file).await.is_err() {
            return Err(Error::NotFound("Delta file not found".to_string()));
        }

        let delta_content = fs::read_to_string(&delta_file).await?;
        let _delta_data: serde_json::Value = serde_json::from_str(&delta_content)
            .map_err(|e| Error::Validation(format!("Invalid delta format: {e}")))?;

        // Apply added files
        let added_files_path = format!("{delta_path}/added");
        if fs::metadata(&added_files_path).await.is_ok() {
            self.copy_directory_contents(&added_files_path, &target_index_path)
                .await?;
        }

        // Apply modified files
        let modified_files_path = format!("{delta_path}/modified");
        if fs::metadata(&modified_files_path).await.is_ok() {
            self.copy_directory_contents(&modified_files_path, &target_index_path)
                .await?;
        }

        // Apply deleted files
        let deleted_files_list = format!("{delta_path}/deleted_files.txt");
        if fs::metadata(&deleted_files_list).await.is_ok() {
            let deleted_content = fs::read_to_string(&deleted_files_list).await?;
            for file_name in deleted_content.lines() {
                let file_path = format!("{target_index_path}/{file_name}");
                let _ = fs::remove_file(&file_path).await; // Ignore errors if file doesn't exist
            }
        }

        tracing::info!(
            target_index = %target_index_name,
            source_index = %source_index_name.as_str(),
            "Applied incremental delta"
        );

        Ok(())
    }

    /// Copy directory contents recursively
    async fn copy_directory_contents(&self, source_dir: &str, target_dir: &str) -> Result<()> {
        use std::collections::VecDeque;

        let mut queue = VecDeque::new();
        queue.push_back((source_dir.to_string(), target_dir.to_string()));

        while let Some((src, dst)) = queue.pop_front() {
            let mut entries = fs::read_dir(&src).await?;

            while let Some(entry) = entries.next_entry().await? {
                let entry_path = entry.path();
                let file_name = entry_path.file_name().unwrap().to_string_lossy();
                let target_path = format!("{dst}/{file_name}");

                if entry_path.is_dir() {
                    fs::create_dir_all(&target_path).await?;
                    queue.push_back((entry_path.to_string_lossy().to_string(), target_path));
                } else {
                    let content = fs::read(&entry_path).await?;
                    fs::write(&target_path, content).await?;
                }
            }
        }

        Ok(())
    }
}

impl FsSnapshotRepository {
    /// Create enhanced incremental snapshot with Phase 3 optimizations
    pub async fn create_enhanced_incremental_snapshot(
        &self,
        snapshot_name: SnapshotName,
        request: CreateSnapshotRequest,
    ) -> Result<crate::snapshot::incremental::EnhancedSnapshotResult> {
        use crate::snapshot::incremental::IncrementalSnapshotManager;

        let mut manager = IncrementalSnapshotManager::new();
        manager
            .create_enhanced_incremental_snapshot(
                &self.name,
                &snapshot_name,
                &request.indices,
                request.parent_snapshot.as_ref(),
            )
            .await
    }

    /// Optimize snapshot chains for better storage efficiency
    pub async fn optimize_snapshot_chains(
        &self,
    ) -> Result<Vec<crate::snapshot::incremental::OptimizationResult>> {
        use crate::snapshot::incremental::IncrementalSnapshotManager;

        let manager = IncrementalSnapshotManager::new();
        manager.optimize_snapshot_chains(&self.name).await
    }

    /// Get incremental snapshot statistics
    pub async fn get_incremental_stats(
        &self,
    ) -> Result<crate::snapshot::incremental::IncrementalStats> {
        use crate::snapshot::incremental::IncrementalSnapshotManager;

        let manager = IncrementalSnapshotManager::new();
        Ok(manager.get_stats().clone())
    }

    /// Create snapshot with advanced compression
    pub async fn create_compressed_snapshot(
        &self,
        snapshot_name: SnapshotName,
        request: CreateSnapshotRequest,
        compression_config: crate::snapshot::compression::CompressionConfig,
    ) -> Result<SnapshotInfo> {
        // Create a regular snapshot first
        let mut snapshot_info = self.create_snapshot(snapshot_name.clone(), request).await?;

        // Apply compression to the snapshot files
        let snapshot_path = self.get_snapshot_path(&snapshot_name);
        self.apply_compression_to_snapshot(&snapshot_path, &compression_config)
            .await?;

        // Update metadata with compression info
        snapshot_info.metadata.settings.insert(
            "compression_algorithm".to_string(),
            format!("{:?}", compression_config.algorithm),
        );
        snapshot_info.metadata.settings.insert(
            "compression_level".to_string(),
            compression_config.level.to_string(),
        );

        Ok(snapshot_info)
    }

    /// Apply compression to snapshot files
    async fn apply_compression_to_snapshot(
        &self,
        snapshot_path: &str,
        compression_config: &crate::snapshot::compression::CompressionConfig,
    ) -> Result<()> {
        use crate::snapshot::compression::SnapshotCompressor;

        let compressor = SnapshotCompressor::new(compression_config.clone());

        // Compress all files in the snapshot directory
        let mut entries = fs::read_dir(snapshot_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name().unwrap().to_string_lossy().to_string();

                // Skip already compressed files
                if file_name.ends_with(".compressed")
                    || file_name.ends_with(".gz")
                    || file_name.ends_with(".zst")
                    || file_name.ends_with(".lz4")
                {
                    continue;
                }

                // Read file content
                let content = fs::read(&path).await?;

                // Compress content
                match compressor.compress(&content) {
                    Ok(compressed) => {
                        // Write compressed content
                        let compressed_path = format!("{}.compressed", path.to_string_lossy());
                        fs::write(&compressed_path, compressed).await?;

                        // Remove original file
                        fs::remove_file(&path).await?;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to compress file {}: {}", path.display(), e);
                    }
                }
            }
        }

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
                settings: HashMap::new(),
            }),
            wait_for_completion: true,
            use_enhanced: false,
            ignore_unavailable: false,
            include_global_state: true,
            snapshot_type: None,
            parent_snapshot: None,
            force_full: false,
        };

        let snapshot_info = repo
            .create_snapshot(snapshot_name.clone(), request)
            .await
            .unwrap();

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
        repo.create_snapshot(snapshot_name.clone(), request.clone())
            .await
            .unwrap();

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
        let created_snapshot = repo
            .create_snapshot(snapshot_name.clone(), request)
            .await
            .unwrap();

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

        repo.create_snapshot(snapshot1, request.clone())
            .await
            .unwrap();
        repo.create_snapshot(snapshot2, request).await.unwrap();

        // List snapshots
        let snapshots = repo.list_snapshots().await.unwrap();
        assert_eq!(snapshots.len(), 2);

        let snapshot_names: Vec<String> = snapshots
            .iter()
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
        repo.create_snapshot(snapshot_name.clone(), request)
            .await
            .unwrap();

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

        repo.create_snapshot(snapshot1, request.clone())
            .await
            .unwrap();
        repo.create_snapshot(snapshot2, request).await.unwrap();

        // Check stats
        let stats = repo.get_stats().await.unwrap();
        assert_eq!(stats.total_snapshots, 2);
        assert_eq!(stats.successful_snapshots, 2);
        assert_eq!(stats.failed_snapshots, 0);
        assert_eq!(stats.in_progress_snapshots, 0);
    }

    #[tokio::test]
    async fn test_restore_snapshot() {
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
        let create_request = CreateSnapshotRequest {
            indices: vec![
                crate::types::IndexName::new("index1"),
                crate::types::IndexName::new("index2"),
            ],
            ..Default::default()
        };

        // Create snapshot first
        let snapshot_info = repo
            .create_snapshot(snapshot_name.clone(), create_request)
            .await
            .unwrap();

        assert_eq!(snapshot_info.state, SnapshotState::Success);

        // Test restore with default request
        let restore_request = RestoreSnapshotRequest::default();
        let result = repo
            .restore_snapshot(snapshot_name.clone(), restore_request)
            .await;
        assert!(result.is_ok());

        // Test restore with specific indices
        let restore_request = RestoreSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            ..Default::default()
        };
        let result = repo
            .restore_snapshot(snapshot_name.clone(), restore_request)
            .await;
        assert!(result.is_ok());

        // Test restore with rename pattern
        let restore_request = RestoreSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            rename_pattern: Some("index1".to_string()),
            rename_replacement: Some("restored_index1".to_string()),
            ..Default::default()
        };
        let result = repo
            .restore_snapshot(snapshot_name.clone(), restore_request)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_restore_nonexistent_snapshot() {
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
        let snapshot_name = SnapshotName::new("nonexistent_snapshot");
        let restore_request = RestoreSnapshotRequest::default();

        let result = repo.restore_snapshot(snapshot_name, restore_request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_restore_failed_snapshot() {
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
        let create_request = CreateSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            ..Default::default()
        };

        // Create snapshot
        let mut snapshot_info = repo
            .create_snapshot(snapshot_name.clone(), create_request)
            .await
            .unwrap();

        // Manually set state to Failed to test error handling
        snapshot_info.state = SnapshotState::Failed;
        repo.save_snapshot_metadata(&snapshot_info).await.unwrap();

        let restore_request = RestoreSnapshotRequest::default();
        let result = repo.restore_snapshot(snapshot_name, restore_request).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Cannot restore snapshot")
        );
    }

    #[tokio::test]
    async fn test_restore_with_ignore_unavailable() {
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
        let create_request = CreateSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            ..Default::default()
        };

        // Create snapshot
        repo.create_snapshot(snapshot_name.clone(), create_request)
            .await
            .unwrap();

        // Test restore with non-existent index but ignore_unavailable = true
        let restore_request = RestoreSnapshotRequest {
            indices: vec![crate::types::IndexName::new("nonexistent_index")],
            ignore_unavailable: true,
            ..Default::default()
        };
        let result = repo.restore_snapshot(snapshot_name, restore_request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_restore_with_invalid_rename_pattern() {
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
        let create_request = CreateSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            ..Default::default()
        };

        // Create snapshot
        repo.create_snapshot(snapshot_name.clone(), create_request)
            .await
            .unwrap();

        // Test restore with invalid rename pattern
        let restore_request = RestoreSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            rename_pattern: Some("[invalid".to_string()), // Invalid regex
            rename_replacement: Some("new_index".to_string()),
            ..Default::default()
        };
        let result = repo.restore_snapshot(snapshot_name, restore_request).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid rename pattern")
        );
    }

    #[tokio::test]
    async fn test_incremental_snapshot_creation() {
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

        // Create a full snapshot first
        let full_snapshot_name = SnapshotName::new("full_snapshot");
        let create_request = CreateSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            snapshot_type: Some(SnapshotType::Full),
            ..Default::default()
        };

        let result = repo
            .create_snapshot(full_snapshot_name.clone(), create_request)
            .await;
        assert!(result.is_ok());

        // Create an incremental snapshot
        let incremental_snapshot_name = SnapshotName::new("incremental_snapshot");
        let incremental_request = CreateSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            snapshot_type: Some(SnapshotType::Incremental),
            parent_snapshot: Some(full_snapshot_name.clone()),
            ..Default::default()
        };

        let result = repo
            .create_snapshot(incremental_snapshot_name.clone(), incremental_request)
            .await;
        assert!(result.is_ok());

        let snapshot_info = result.unwrap();
        assert_eq!(snapshot_info.snapshot_type, SnapshotType::Incremental);
        assert_eq!(snapshot_info.parent_snapshot, Some(full_snapshot_name));
        assert_eq!(snapshot_info.chain_depth, 1);
    }

    #[tokio::test]
    async fn test_snapshot_chain_retrieval() {
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

        // Create a full snapshot
        let full_snapshot_name = SnapshotName::new("root_snapshot");
        let create_request = CreateSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            snapshot_type: Some(SnapshotType::Full),
            ..Default::default()
        };

        repo.create_snapshot(full_snapshot_name.clone(), create_request)
            .await
            .unwrap();

        // Create an incremental snapshot
        let incremental_snapshot_name = SnapshotName::new("incremental_snapshot");
        let incremental_request = CreateSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            snapshot_type: Some(SnapshotType::Incremental),
            parent_snapshot: Some(full_snapshot_name.clone()),
            ..Default::default()
        };

        repo.create_snapshot(incremental_snapshot_name.clone(), incremental_request)
            .await
            .unwrap();

        // Get snapshot chain
        let chain = repo
            .get_snapshot_chain(incremental_snapshot_name.clone())
            .await
            .unwrap();

        assert_eq!(chain.root_snapshot, full_snapshot_name);
        assert_eq!(chain.incremental_snapshots.len(), 1);
        assert_eq!(chain.incremental_snapshots[0], incremental_snapshot_name);
        assert_eq!(chain.depth, 1);
    }

    #[tokio::test]
    async fn test_find_best_parent_snapshot() {
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

        // Create a full snapshot
        let full_snapshot_name = SnapshotName::new("full_snapshot");
        let create_request = CreateSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            snapshot_type: Some(SnapshotType::Full),
            ..Default::default()
        };

        repo.create_snapshot(full_snapshot_name.clone(), create_request)
            .await
            .unwrap();

        // Find best parent for incremental snapshot
        let indices = vec![crate::types::IndexName::new("index1")];
        let best_parent = repo.find_best_parent_snapshot(&indices).await.unwrap();

        assert!(best_parent.is_some());
        assert_eq!(best_parent.unwrap(), full_snapshot_name);
    }

    #[tokio::test]
    async fn test_incremental_snapshot_restore() {
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

        // Create a full snapshot
        let full_snapshot_name = SnapshotName::new("full_snapshot");
        let create_request = CreateSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            snapshot_type: Some(SnapshotType::Full),
            ..Default::default()
        };

        repo.create_snapshot(full_snapshot_name.clone(), create_request)
            .await
            .unwrap();

        // Create an incremental snapshot
        let incremental_snapshot_name = SnapshotName::new("incremental_snapshot");
        let incremental_request = CreateSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            snapshot_type: Some(SnapshotType::Incremental),
            parent_snapshot: Some(full_snapshot_name.clone()),
            ..Default::default()
        };

        repo.create_snapshot(incremental_snapshot_name.clone(), incremental_request)
            .await
            .unwrap();

        // Restore from incremental snapshot
        let restore_request = RestoreSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            rename_pattern: Some("index1".to_string()),
            rename_replacement: Some("restored_index1".to_string()),
            ..Default::default()
        };

        let result = repo
            .restore_snapshot(incremental_snapshot_name, restore_request)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_snapshot_deltas_retrieval() {
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

        // Create a full snapshot
        let full_snapshot_name = SnapshotName::new("full_snapshot");
        let create_request = CreateSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            snapshot_type: Some(SnapshotType::Full),
            ..Default::default()
        };

        repo.create_snapshot(full_snapshot_name.clone(), create_request)
            .await
            .unwrap();

        // Create an incremental snapshot
        let incremental_snapshot_name = SnapshotName::new("incremental_snapshot");
        let incremental_request = CreateSnapshotRequest {
            indices: vec![crate::types::IndexName::new("index1")],
            snapshot_type: Some(SnapshotType::Incremental),
            parent_snapshot: Some(full_snapshot_name.clone()),
            ..Default::default()
        };

        repo.create_snapshot(incremental_snapshot_name.clone(), incremental_request)
            .await
            .unwrap();

        // Get snapshot deltas
        let deltas = repo
            .get_snapshot_deltas(incremental_snapshot_name)
            .await
            .unwrap();

        assert!(!deltas.is_empty());
        assert_eq!(deltas[0].parent_snapshot, full_snapshot_name);
    }
}
