//! Enhanced incremental snapshot management for Phase 3

use crate::error::Result;
use crate::snapshot::compression::{CompressionConfig, SnapshotCompressor, ContentDeduplicator};
use crate::snapshot::parallel::{ParallelDeltaProcessor, SnapshotChainOptimizer};
use crate::types::{IndexName, RepositoryName, SnapshotName};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;

/// Enhanced incremental snapshot manager for Phase 3
pub struct IncrementalSnapshotManager {
    /// Compression configuration
    compression_config: CompressionConfig,
    /// Parallel processor
    parallel_processor: ParallelDeltaProcessor,
    /// Chain optimizer
    chain_optimizer: SnapshotChainOptimizer,
    /// Content deduplicator
    deduplicator: ContentDeduplicator,
    /// Statistics tracker
    stats: IncrementalStats,
}

impl IncrementalSnapshotManager {
    /// Create a new enhanced incremental snapshot manager
    pub fn new() -> Self {
        Self {
            compression_config: CompressionConfig::default(),
            parallel_processor: ParallelDeltaProcessor::new(4), // 4 parallel workers
            chain_optimizer: SnapshotChainOptimizer::new(10, 0.3), // Max 10 depth, 30% compression threshold
            deduplicator: ContentDeduplicator::new(),
            stats: IncrementalStats::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        compression_config: CompressionConfig,
        max_workers: usize,
        max_chain_depth: usize,
    ) -> Self {
        Self {
            compression_config,
            parallel_processor: ParallelDeltaProcessor::new(max_workers),
            chain_optimizer: SnapshotChainOptimizer::new(max_chain_depth, 0.3),
            deduplicator: ContentDeduplicator::new(),
            stats: IncrementalStats::new(),
        }
    }

    /// Create an enhanced incremental snapshot
    pub async fn create_enhanced_incremental_snapshot(
        &mut self,
        repository_name: &RepositoryName,
        snapshot_name: &SnapshotName,
        indices: &[IndexName],
        parent_snapshot: Option<&SnapshotName>,
    ) -> Result<EnhancedSnapshotResult> {
        let start_time = std::time::Instant::now();
        let snapshot_start = Utc::now();

        // Find the best parent snapshot if not provided
        let parent = if let Some(parent) = parent_snapshot {
            parent.clone()
        } else {
            self.find_best_parent_snapshot(indices).await?
        };

        // Process indices in parallel
        let delta_results = self.parallel_processor
            .process_indices_parallel(indices, &parent, &format!("./snapshots/{}", repository_name.as_str()))
            .await?;

        // Apply content deduplication
        let deduplication_result = self.apply_deduplication(&delta_results).await?;

        // Create compressed delta files
        let compression_result = self.create_compressed_deltas(&delta_results).await?;

        // Create enhanced snapshot metadata
        let snapshot_info = self.create_enhanced_metadata(
            snapshot_name,
            repository_name,
            indices,
            &parent,
            &delta_results,
            &deduplication_result,
            &compression_result,
            snapshot_start,
        ).await?;

        // Update statistics
        self.stats.record_snapshot_creation(&snapshot_info);

        let processing_time = start_time.elapsed();

        Ok(EnhancedSnapshotResult {
            snapshot_info,
            delta_results,
            deduplication_result,
            compression_result,
            processing_time,
        })
    }

    /// Find the best parent snapshot for incremental creation
    async fn find_best_parent_snapshot(&self, indices: &[IndexName]) -> Result<SnapshotName> {
        // This is a simplified implementation
        // In a real implementation, this would:
        // 1. Query all existing snapshots
        // 2. Find the most recent snapshot that contains all requested indices
        // 3. Consider chain depth and compression ratios
        // 4. Return the optimal parent

        // For now, return a placeholder
        Ok(SnapshotName::new("latest_snapshot"))
    }

    /// Apply content deduplication to delta results
    async fn apply_deduplication(
        &mut self,
        delta_results: &[crate::snapshot::parallel::IndexDeltaResult],
    ) -> Result<DeduplicationResult> {
        let mut total_files = 0;
        let mut duplicate_files = 0;
        let mut space_saved = 0;

        for delta_result in delta_results {
            for file_path in &delta_result.delta.added_files {
                total_files += 1;
                
                // Read file content
                if let Ok(content) = fs::read(&file_path).await {
                    if let Some(_existing_path) = self.deduplicator.check_duplicate(&content) {
                        // File is a duplicate
                        duplicate_files += 1;
                        space_saved += content.len();
                        
                        // Create a reference instead of storing the file
                        // This would be implemented in the actual file storage
                    } else {
                        // Add new unique content
                        self.deduplicator.add_content(&content, file_path.clone());
                    }
                }
            }
        }

        Ok(DeduplicationResult {
            total_files,
            duplicate_files,
            space_saved,
            deduplication_ratio: if total_files > 0 {
                duplicate_files as f64 / total_files as f64
            } else {
                0.0
            },
        })
    }

    /// Create compressed delta files
    async fn create_compressed_deltas(
        &self,
        delta_results: &[crate::snapshot::parallel::IndexDeltaResult],
    ) -> Result<CompressionResult> {
        let compressor = SnapshotCompressor::new(self.compression_config.clone());
        let mut total_original_size = 0;
        let mut total_compressed_size = 0;
        let mut compression_stats = Vec::new();

        for delta_result in delta_results {
            for file_path in &delta_result.delta.added_files {
                if let Ok(content) = fs::read(file_path).await {
                    let original_size = content.len();
                    total_original_size += original_size;

                    // Compress the content
                    match compressor.compress(&content) {
                        Ok(compressed) => {
                            let compressed_size = compressed.len();
                            total_compressed_size += compressed_size;

                            // Store compressed content (simplified)
                            let compressed_path = format!("{}.compressed", file_path);
                            fs::write(&compressed_path, compressed).await?;

                            // Record compression stats
                            let stats = compressor.compression_stats(original_size, compressed_size);
                            compression_stats.push(stats);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to compress file {}: {}", file_path, e);
                            total_compressed_size += original_size; // Fallback to uncompressed
                        }
                    }
                }
            }
        }

        Ok(CompressionResult {
            total_original_size,
            total_compressed_size,
            compression_ratio: if total_original_size > 0 {
                total_compressed_size as f64 / total_original_size as f64
            } else {
                0.0
            },
            space_saved: total_original_size.saturating_sub(total_compressed_size),
            compression_stats,
        })
    }

    /// Create enhanced snapshot metadata
    async fn create_enhanced_metadata(
        &self,
        snapshot_name: &SnapshotName,
        repository_name: &RepositoryName,
        indices: &[IndexName],
        parent_snapshot: &SnapshotName,
        delta_results: &[crate::snapshot::parallel::IndexDeltaResult],
        deduplication_result: &DeduplicationResult,
        compression_result: &CompressionResult,
        start_time: DateTime<Utc>,
    ) -> Result<EnhancedSnapshotInfo> {
        let end_time = Utc::now();
        let duration = end_time.signed_duration_since(start_time);

        // Calculate total statistics
        let total_documents = delta_results.iter()
            .map(|r| r.delta.added_files.len() as u64)
            .sum();

        let total_size = compression_result.total_compressed_size as u64;

        Ok(EnhancedSnapshotInfo {
            name: snapshot_name.clone(),
            repository: repository_name.clone(),
            indices: indices.to_vec(),
            parent_snapshot: Some(parent_snapshot.clone()),
            snapshot_type: crate::snapshot::types::SnapshotType::Incremental,
            state: crate::snapshot::types::SnapshotState::Success,
            start_time,
            end_time: Some(end_time),
            duration_in_millis: Some(duration.num_milliseconds() as u64),
            failures: 0,
            shards: crate::snapshot::types::ShardInfo::default(),
            metadata: crate::snapshot::types::SnapshotMetadata::default(),
            chain_depth: 1, // Simplified
            size_bytes: total_size,
            document_count: total_documents,
            // Phase 3 enhancements
            compression_info: Some(CompressionInfo {
                algorithm: self.compression_config.algorithm,
                compression_ratio: compression_result.compression_ratio,
                space_saved: compression_result.space_saved as u64,
            }),
            deduplication_info: Some(DeduplicationInfo {
                duplicate_files: deduplication_result.duplicate_files as u64,
                deduplication_ratio: deduplication_result.deduplication_ratio,
                space_saved: deduplication_result.space_saved as u64,
            }),
            parallel_processing_info: Some(ParallelProcessingInfo {
                workers_used: self.parallel_processor.max_workers,
                total_processing_time: delta_results.iter()
                    .map(|r| r.processing_time)
                    .sum::<std::time::Duration>()
                    .as_millis() as u64,
            }),
        })
    }

    /// Optimize snapshot chains
    pub async fn optimize_snapshot_chains(
        &self,
        _repository_name: &RepositoryName,
    ) -> Result<Vec<OptimizationResult>> {
        // This would query all snapshot chains and optimize them
        // For now, return empty result
        Ok(vec![])
    }

    /// Get incremental snapshot statistics
    pub fn get_stats(&self) -> &IncrementalStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = IncrementalStats::new();
    }
}

/// Enhanced snapshot information with Phase 3 features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSnapshotInfo {
    /// Basic snapshot information
    pub name: SnapshotName,
    pub repository: RepositoryName,
    pub indices: Vec<IndexName>,
    pub parent_snapshot: Option<SnapshotName>,
    pub snapshot_type: crate::snapshot::types::SnapshotType,
    pub state: crate::snapshot::types::SnapshotState,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_in_millis: Option<u64>,
    pub failures: u32,
    pub shards: crate::snapshot::types::ShardInfo,
    pub metadata: crate::snapshot::types::SnapshotMetadata,
    pub chain_depth: u32,
    pub size_bytes: u64,
    pub document_count: u64,
    
    // Phase 3 enhancements
    pub compression_info: Option<CompressionInfo>,
    pub deduplication_info: Option<DeduplicationInfo>,
    pub parallel_processing_info: Option<ParallelProcessingInfo>,
}

/// Compression information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionInfo {
    pub algorithm: crate::snapshot::compression::CompressionType,
    pub compression_ratio: f64,
    pub space_saved: u64,
}

/// Deduplication information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationInfo {
    pub duplicate_files: u64,
    pub deduplication_ratio: f64,
    pub space_saved: u64,
}

/// Parallel processing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelProcessingInfo {
    pub workers_used: usize,
    pub total_processing_time: u64, // milliseconds
}

/// Result of creating an enhanced incremental snapshot
#[derive(Debug, Clone)]
pub struct EnhancedSnapshotResult {
    pub snapshot_info: EnhancedSnapshotInfo,
    pub delta_results: Vec<crate::snapshot::parallel::IndexDeltaResult>,
    pub deduplication_result: DeduplicationResult,
    pub compression_result: CompressionResult,
    pub processing_time: std::time::Duration,
}

/// Deduplication result
#[derive(Debug, Clone)]
pub struct DeduplicationResult {
    pub total_files: usize,
    pub duplicate_files: usize,
    pub space_saved: usize,
    pub deduplication_ratio: f64,
}

/// Compression result
#[derive(Debug, Clone)]
pub struct CompressionResult {
    pub total_original_size: usize,
    pub total_compressed_size: usize,
    pub compression_ratio: f64,
    pub space_saved: usize,
    pub compression_stats: Vec<crate::snapshot::compression::CompressionStats>,
}

/// Optimization result
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub chain_id: String,
    pub original_depth: usize,
    pub optimized_depth: usize,
    pub space_saved: u64,
    pub processing_time: std::time::Duration,
}

/// Incremental snapshot statistics
#[derive(Debug, Clone)]
pub struct IncrementalStats {
    pub total_snapshots_created: u64,
    pub total_compression_ratio: f64,
    pub total_deduplication_ratio: f64,
    pub total_space_saved: u64,
    pub average_processing_time: std::time::Duration,
    pub parallel_efficiency: f64,
}

impl IncrementalStats {
    fn new() -> Self {
        Self {
            total_snapshots_created: 0,
            total_compression_ratio: 0.0,
            total_deduplication_ratio: 0.0,
            total_space_saved: 0,
            average_processing_time: std::time::Duration::ZERO,
            parallel_efficiency: 0.0,
        }
    }

    fn record_snapshot_creation(&mut self, snapshot_info: &EnhancedSnapshotInfo) {
        self.total_snapshots_created += 1;
        
        if let Some(compression) = &snapshot_info.compression_info {
            self.total_compression_ratio = (self.total_compression_ratio + compression.compression_ratio) / 2.0;
            self.total_space_saved += compression.space_saved;
        }
        
        if let Some(deduplication) = &snapshot_info.deduplication_info {
            self.total_deduplication_ratio = (self.total_deduplication_ratio + deduplication.deduplication_ratio) / 2.0;
            self.total_space_saved += deduplication.space_saved;
        }
    }
}

impl Default for IncrementalSnapshotManager {
    fn default() -> Self {
        Self::new()
    }
}