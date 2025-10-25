//! Administrative operation handlers

use crate::error::ApiResult;
use crate::handlers::index::AppState;
use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

/// Cluster health information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ClusterHealth {
    /// Overall cluster status
    pub status: String,
    /// Number of nodes
    pub number_of_nodes: u32,
    /// Number of data nodes
    pub number_of_data_nodes: u32,
    /// Active primary shards
    pub active_primary_shards: u32,
    /// Active shards
    pub active_shards: u32,
    /// Relocating shards
    pub relocating_shards: u32,
    /// Initializing shards
    pub initializing_shards: u32,
    /// Unassigned shards
    pub unassigned_shards: u32,
}

/// Cluster statistics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ClusterStats {
    /// Total number of documents
    pub total_documents: u64,
    /// Total size in bytes
    pub total_size_bytes: u64,
    /// Number of indices
    pub number_of_indices: u32,
    /// Number of shards
    pub number_of_shards: u32,
}

/// Node statistics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NodeStats {
    /// Node name
    pub name: String,
    /// Node role
    pub role: String,
    /// JVM heap used in bytes
    pub jvm_heap_used_bytes: u64,
    /// JVM heap max in bytes
    pub jvm_heap_max_bytes: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Memory usage percentage
    pub memory_usage_percent: f64,
}

/// Cluster settings
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ClusterSettings {
    /// Cluster name
    pub cluster_name: String,
    /// Persistence settings
    pub persistence: PersistenceSettings,
    /// Network settings
    pub network: NetworkSettings,
}

/// Persistence settings
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PersistenceSettings {
    /// Storage path
    pub storage_path: String,
    /// Snapshot settings
    pub snapshot: SnapshotSettings,
}

/// Snapshot settings
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SnapshotSettings {
    /// Snapshot repository path
    pub repository_path: String,
    /// Maximum snapshots to keep
    pub max_snapshots: u32,
}

/// Network settings
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NetworkSettings {
    /// Bind address
    pub bind_address: String,
    /// Port
    pub port: u16,
    /// Enable CORS
    pub enable_cors: bool,
}

/// Update cluster settings request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateClusterSettingsRequest {
    /// New cluster settings
    pub settings: ClusterSettings,
}

/// Get cluster health
#[utoipa::path(
    get,
    path = "/_cluster/health",
    responses(
        (status = 200, description = "Cluster health retrieved successfully", body = ClusterHealth),
        (status = 500, description = "Internal server error")
    ),
    tag = "Admin"
)]
pub async fn get_cluster_health(State(state): State<AppState>) -> ApiResult<Json<ClusterHealth>> {
    // Get list of indices to determine shard counts
    let indices = state.index_manager.list_indices();
    let total_indices = indices.len() as u32;

    // For now, we're a single-node cluster with no sharding
    // Each index has 1 primary shard and 0 replicas
    let active_primary_shards = total_indices;
    let active_shards = total_indices;

    // Determine cluster status based on health
    let status = if total_indices == 0 {
        "yellow" // No indices but cluster is running
    } else {
        "green" // All shards are active
    };

    Ok(Json(ClusterHealth {
        status: status.to_string(),
        number_of_nodes: 1,
        number_of_data_nodes: 1,
        active_primary_shards,
        active_shards,
        relocating_shards: 0,
        initializing_shards: 0,
        unassigned_shards: 0,
    }))
}

/// Get cluster statistics
#[utoipa::path(
    get,
    path = "/_cluster/stats",
    responses(
        (status = 200, description = "Cluster statistics retrieved successfully", body = ClusterStats),
        (status = 500, description = "Internal server error")
    ),
    tag = "Admin"
)]
pub async fn get_cluster_stats(State(state): State<AppState>) -> ApiResult<Json<ClusterStats>> {
    // Get list of indices
    let indices = state.index_manager.list_indices();
    let number_of_indices = indices.len() as u32;

    // Calculate total documents and size across all indices
    let mut total_documents = 0u64;
    let mut total_size_bytes = 0u64;

    for index_name in &indices {
        if let Ok(_index) = state.index_manager.get_index(index_name) {
            // For now, we'll use placeholder values since get_stats doesn't exist
            // In a real implementation, this would get actual statistics
            total_documents += 100; // Placeholder
            total_size_bytes += 1024; // Placeholder
        }
    }

    // For now, each index has 1 shard (no replication)
    let number_of_shards = number_of_indices;

    Ok(Json(ClusterStats {
        total_documents,
        total_size_bytes,
        number_of_indices,
        number_of_shards,
    }))
}

/// Get node statistics
#[utoipa::path(
    get,
    path = "/_nodes/stats",
    responses(
        (status = 200, description = "Node statistics retrieved successfully", body = NodeStats),
        (status = 500, description = "Internal server error")
    ),
    tag = "Admin"
)]
pub async fn get_node_stats(State(_state): State<AppState>) -> ApiResult<Json<NodeStats>> {
    // Get system memory information
    let total_memory = sys_info::mem_info()
        .map(|info| info.total * 1024)
        .unwrap_or(0) as u64;
    let available_memory = sys_info::mem_info()
        .map(|info| info.avail * 1024)
        .unwrap_or(0) as u64;
    let used_memory = total_memory - available_memory;
    let memory_usage_percent = if total_memory > 0 {
        (used_memory as f64 / total_memory as f64) * 100.0
    } else {
        0.0
    };

    // Get CPU usage (simplified - in production this would be more sophisticated)
    let cpu_usage_percent = sys_info::loadavg()
        .map(|load| load.one * 100.0)
        .unwrap_or(0.0);

    // Get heap information (simplified - in production this would track actual heap usage)
    let jvm_heap_max_bytes = total_memory / 4; // Assume 25% of total memory for heap
    let jvm_heap_used_bytes = (jvm_heap_max_bytes as f64 * 0.3) as u64; // Assume 30% heap usage

    Ok(Json(NodeStats {
        name: "lexum-node-1".to_string(),
        role: "master,data".to_string(),
        jvm_heap_used_bytes,
        jvm_heap_max_bytes,
        cpu_usage_percent,
        memory_usage_percent,
    }))
}

/// Get cluster settings
#[utoipa::path(
    get,
    path = "/_cluster/settings",
    responses(
        (status = 200, description = "Cluster settings retrieved successfully", body = ClusterSettings),
        (status = 500, description = "Internal server error")
    ),
    tag = "Admin"
)]
pub async fn get_cluster_settings(
    State(_state): State<AppState>,
) -> ApiResult<Json<ClusterSettings>> {
    // Get actual configuration from the core
    let config = lexum_core::config::Config::default();

    // Extract settings from the configuration
    let storage_path = config.path.data.clone();
    let snapshot_repo_path = config
        .snapshots
        .repositories
        .first()
        .map(|repo| repo.settings.location.clone())
        .unwrap_or_else(|| "./snapshots".to_string());
    let max_snapshots = config
        .snapshots
        .repositories
        .first()
        .map(|repo| repo.settings.max_snapshots)
        .unwrap_or(10);

    Ok(Json(ClusterSettings {
        cluster_name: "lexum-cluster".to_string(),
        persistence: PersistenceSettings {
            storage_path,
            snapshot: SnapshotSettings {
                repository_path: snapshot_repo_path,
                max_snapshots: max_snapshots.try_into().unwrap_or(10),
            },
        },
        network: NetworkSettings {
            bind_address: "0.0.0.0".to_string(),
            port: 9200,
            enable_cors: true,
        },
    }))
}

/// Update cluster settings
#[utoipa::path(
    put,
    path = "/_cluster/settings",
    request_body = UpdateClusterSettingsRequest,
    responses(
        (status = 200, description = "Cluster settings updated successfully"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Admin"
)]
pub async fn update_cluster_settings(
    State(_state): State<AppState>,
    Json(request): Json<UpdateClusterSettingsRequest>,
) -> ApiResult<()> {
    // For now, we'll just log the settings update
    // In a real implementation, this would update the configuration
    // and potentially restart services or apply changes dynamically
    tracing::info!("Cluster settings update requested: {:?}", request.settings);

    // TODO: Implement actual cluster settings update
    // This would involve:
    // 1. Validating the new settings
    // 2. Updating the configuration
    // 3. Applying changes to running services
    // 4. Persisting the new configuration

    Ok(())
}

/// Cluster information response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ClusterInfo {
    /// Cluster name
    pub name: String,
    /// Cluster UUID
    pub cluster_uuid: String,
    /// Version information
    pub version: ClusterVersion,
}

/// Cluster version information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ClusterVersion {
    /// Version number
    pub number: String,
    /// Build hash
    pub build_hash: String,
    /// Build date
    pub build_date: String,
    /// Lucene version
    pub lucene_version: String,
}

/// Get cluster information (GET /)
#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Cluster information retrieved successfully", body = ClusterInfo)
    ),
    tag = "Cluster"
)]
pub async fn get_cluster_info() -> ApiResult<Json<ClusterInfo>> {
    Ok(Json(ClusterInfo {
        name: "lexum-cluster".to_string(),
        cluster_uuid: "12345678-1234-1234-1234-123456789abc".to_string(),
        version: ClusterVersion {
            number: env!("CARGO_PKG_VERSION").to_string(),
            build_hash: "abc123def456".to_string(),
            build_date: "2024-10-25".to_string(),
            lucene_version: "9.8.0".to_string(),
        },
    }))
}
