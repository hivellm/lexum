//! Cluster management endpoints

use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::index::AppState;

/// Cluster information response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusterInfo {
    /// Cluster name
    pub name: String,
    /// Cluster UUID
    pub cluster_uuid: String,
    /// Version information
    pub version: VersionInfo,
    /// Tagline
    pub tagline: String,
}

/// Version information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VersionInfo {
    /// Version number
    pub number: String,
    /// Build hash
    pub build_hash: String,
    /// Build date
    pub build_date: String,
    /// Build snapshot
    pub build_snapshot: bool,
    /// Lucene version
    pub lucene_version: String,
    /// Minimum wire compatibility version
    pub minimum_wire_compatibility_version: String,
    /// Minimum index compatibility version
    pub minimum_index_compatibility_version: String,
}

/// Cluster health response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusterHealth {
    /// Cluster name
    pub cluster_name: String,
    /// Health status (green, yellow, red)
    pub status: String,
    /// Timed out
    pub timed_out: bool,
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
    /// Delayed unassigned shards
    pub delayed_unassigned_shards: u32,
    /// Number of pending tasks
    pub number_of_pending_tasks: u32,
    /// Number of in-flight fetch
    pub number_of_in_flight_fetch: u32,
    /// Task max waiting in queue
    pub task_max_waiting_in_queue_millis: u32,
    /// Active shards percent as number
    pub active_shards_percent_as_number: f64,
}

/// Cluster statistics response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusterStats {
    /// Cluster name
    pub cluster_name: String,
    /// Cluster UUID
    pub cluster_uuid: String,
    /// Timestamp
    pub timestamp: u64,
    /// Status
    pub status: String,
    /// Indices statistics
    pub indices: IndicesStats,
    /// Nodes statistics
    pub nodes: NodesStats,
}

/// Indices statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IndicesStats {
    /// Count of indices
    pub count: u32,
    /// Shards statistics
    pub shards: ShardsStats,
    /// Documents statistics
    pub docs: DocsStats,
    /// Store statistics
    pub store: StoreStats,
    /// Field data statistics
    pub fielddata: FieldDataStats,
    /// Query cache statistics
    pub query_cache: QueryCacheStats,
    /// Completion statistics
    pub completion: CompletionStats,
    /// Segments statistics
    pub segments: SegmentsStats,
    /// Translog statistics
    pub translog: TranslogStats,
    /// Request cache statistics
    pub request_cache: RequestCacheStats,
    /// Recovery statistics
    pub recovery: RecoveryStats,
}

/// Shards statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ShardsStats {
    /// Total number of shards
    pub total: u32,
    /// Primary shards
    pub primaries: u32,
    /// Replication shards
    pub replication: u32,
    /// Unassigned shards
    pub unassigned: u32,
}

/// Documents statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocsStats {
    /// Total number of documents
    pub count: u64,
    /// Deleted documents
    pub deleted: u64,
}

/// Store statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StoreStats {
    /// Size in bytes
    pub size_in_bytes: u64,
    /// Reserved size in bytes
    pub reserved_in_bytes: u64,
}

/// Field data statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FieldDataStats {
    /// Memory size in bytes
    pub memory_size_in_bytes: u64,
    /// Evictions
    pub evictions: u64,
}

/// Query cache statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QueryCacheStats {
    /// Memory size in bytes
    pub memory_size_in_bytes: u64,
    /// Total count
    pub total_count: u64,
    /// Hit count
    pub hit_count: u64,
    /// Miss count
    pub miss_count: u64,
    /// Cache count
    pub cache_count: u64,
    /// Cache size
    pub cache_size: u64,
    /// Evictions
    pub evictions: u64,
}

/// Completion statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CompletionStats {
    /// Size in bytes
    pub size_in_bytes: u64,
}

/// Segments statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SegmentsStats {
    /// Count of segments
    pub count: u32,
    /// Memory in bytes
    pub memory_in_bytes: u64,
    /// Terms memory in bytes
    pub terms_memory_in_bytes: u64,
    /// Stored fields memory in bytes
    pub stored_fields_memory_in_bytes: u64,
    /// Term vectors memory in bytes
    pub term_vectors_memory_in_bytes: u64,
    /// Norms memory in bytes
    pub norms_memory_in_bytes: u64,
    /// Points memory in bytes
    pub points_memory_in_bytes: u64,
    /// Doc values memory in bytes
    pub doc_values_memory_in_bytes: u64,
    /// Index writer memory in bytes
    pub index_writer_memory_in_bytes: u64,
    /// Version map memory in bytes
    pub version_map_memory_in_bytes: u64,
    /// Fixed bit set memory in bytes
    pub fixed_bit_set_memory_in_bytes: u64,
}

/// Translog statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TranslogStats {
    /// Operations
    pub operations: u64,
    /// Size in bytes
    pub size_in_bytes: u64,
    /// Uncommitted operations
    pub uncommitted_operations: u64,
    /// Uncommitted size in bytes
    pub uncommitted_size_in_bytes: u64,
    /// Earliest last modified age
    pub earliest_last_modified_age: u64,
}

/// Request cache statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestCacheStats {
    /// Memory size in bytes
    pub memory_size_in_bytes: u64,
    /// Evictions
    pub evictions: u64,
    /// Hit count
    pub hit_count: u64,
    /// Miss count
    pub miss_count: u64,
}

/// Recovery statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecoveryStats {
    /// Current as source
    pub current_as_source: u32,
    /// Current as target
    pub current_as_target: u32,
    /// Throttled until
    pub throttled_until: Option<u64>,
}

/// Nodes statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NodesStats {
    /// Count of nodes
    pub count: NodesCount,
    /// Versions
    pub versions: Vec<String>,
    /// Operating systems
    pub os: OsStats,
    /// Processes
    pub process: ProcessStats,
    /// JVM statistics
    pub jvm: JvmStats,
    /// Thread pool statistics
    pub thread_pool: ThreadPoolStats,
    /// File system statistics
    pub fs: FsStats,
    /// Network statistics
    pub network: NetworkStats,
    /// Transport statistics
    pub transport: TransportStats,
    /// HTTP statistics
    pub http: HttpStats,
    /// Breakers statistics
    pub breakers: BreakersStats,
    /// Script statistics
    pub script: ScriptStats,
    /// Discovery statistics
    pub discovery: DiscoveryStats,
    /// Ingest statistics
    pub ingest: IngestStats,
    /// Adaptive selection statistics
    pub adaptive_selection: AdaptiveSelectionStats,
}

/// Nodes count
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NodesCount {
    /// Total nodes
    pub total: u32,
    /// Master nodes
    pub master_only: u32,
    /// Data nodes
    pub data: u32,
    /// Coordinating only nodes
    pub coordinating_only: u32,
    /// Ingest nodes
    pub ingest: u32,
    /// Machine learning nodes
    pub ml: u32,
    /// Remote cluster client nodes
    pub remote_cluster_client: u32,
}

/// Operating system statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OsStats {
    /// Available processors
    pub available_processors: u32,
    /// Allocated processors
    pub allocated_processors: u32,
    /// Names
    pub names: Vec<OsName>,
    /// Pretty names
    pub pretty_names: Vec<OsPrettyName>,
    /// Architectures
    pub architectures: Vec<OsArchitecture>,
}

/// OS name
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OsName {
    /// Count
    pub count: u32,
    /// Name
    pub name: String,
}

/// OS pretty name
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OsPrettyName {
    /// Count
    pub count: u32,
    /// Pretty name
    pub pretty_name: String,
}

/// OS architecture
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OsArchitecture {
    /// Count
    pub count: u32,
    /// Architecture
    pub arch: String,
}

/// Process statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProcessStats {
    /// Open file descriptors
    pub open_file_descriptors: u32,
    /// Max file descriptors
    pub max_file_descriptors: u32,
    /// CPU percentage
    pub cpu: CpuStats,
    /// Memory statistics
    pub mem: MemoryStats,
}

/// CPU statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CpuStats {
    /// Percent
    pub percent: u32,
    /// Total in milliseconds
    pub total_in_millis: u64,
}

/// Memory statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MemoryStats {
    /// Total virtual in bytes
    pub total_virtual_in_bytes: u64,
}

/// JVM statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JvmStats {
    /// Max uptime in milliseconds
    pub max_uptime_in_millis: u64,
    /// Versions
    pub versions: Vec<JvmVersion>,
    /// Memory statistics
    pub mem: JvmMemoryStats,
    /// Threads
    pub threads: JvmThreadsStats,
    /// GC statistics
    pub gc: JvmGcStats,
    /// Buffer pools
    pub buffer_pools: Vec<JvmBufferPool>,
    /// Classes
    pub classes: JvmClassesStats,
}

/// JVM version
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JvmVersion {
    /// Version
    pub version: String,
    /// VM name
    pub vm_name: String,
    /// VM version
    pub vm_version: String,
    /// VM vendor
    pub vm_vendor: String,
    /// Count
    pub count: u32,
}

/// JVM memory statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JvmMemoryStats {
    /// Heap used in bytes
    pub heap_used_in_bytes: u64,
    /// Heap used percent
    pub heap_used_percent: u32,
    /// Heap max in bytes
    pub heap_max_in_bytes: u64,
    /// Non heap used in bytes
    pub non_heap_used_in_bytes: u64,
    /// Non heap committed in bytes
    pub non_heap_committed_in_bytes: u64,
    /// Pools
    pub pools: Vec<JvmMemoryPool>,
}

/// JVM memory pool
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JvmMemoryPool {
    /// Name
    pub name: String,
    /// Used in bytes
    pub used_in_bytes: u64,
    /// Max in bytes
    pub max_in_bytes: u64,
    /// Peak used in bytes
    pub peak_used_in_bytes: u64,
    /// Peak max in bytes
    pub peak_max_in_bytes: u64,
}

/// JVM threads statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JvmThreadsStats {
    /// Count
    pub count: u32,
    /// Peak count
    pub peak_count: u32,
}

/// JVM GC statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JvmGcStats {
    /// Collectors
    pub collectors: Vec<JvmGcCollector>,
}

/// JVM GC collector
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JvmGcCollector {
    /// Name
    pub name: String,
    /// Collection count
    pub collection_count: u64,
    /// Collection time in milliseconds
    pub collection_time_in_millis: u64,
}

/// JVM buffer pool
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JvmBufferPool {
    /// Name
    pub name: String,
    /// Count
    pub count: u64,
    /// Used in bytes
    pub used_in_bytes: u64,
    /// Total capacity in bytes
    pub total_capacity_in_bytes: u64,
}

/// JVM classes statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JvmClassesStats {
    /// Current loaded class count
    pub current_loaded_class_count: u32,
    /// Total loaded class count
    pub total_loaded_class_count: u64,
    /// Total unloaded class count
    pub total_unloaded_class_count: u64,
}

/// Thread pool statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ThreadPoolStats {
    /// Thread pools
    pub thread_pools: Vec<ThreadPoolInfo>,
}

/// Thread pool information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ThreadPoolInfo {
    /// Name
    pub name: String,
    /// Type
    pub r#type: String,
    /// Active
    pub active: u32,
    /// Pool size
    pub pool_size: u32,
    /// Queue size
    pub queue_size: u32,
    /// Rejected
    pub rejected: u64,
    /// Largest
    pub largest: u32,
    /// Completed
    pub completed: u64,
    /// Threads
    pub threads: u32,
    /// Queue
    pub queue: u32,
}

/// File system statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FsStats {
    /// Total
    pub total: FsTotal,
    /// Data
    pub data: Vec<FsData>,
}

/// File system total
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FsTotal {
    /// Total in bytes
    pub total_in_bytes: u64,
    /// Free in bytes
    pub free_in_bytes: u64,
    /// Available in bytes
    pub available_in_bytes: u64,
}

/// File system data
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FsData {
    /// Path
    pub path: String,
    /// Mount
    pub mount: String,
    /// Device
    pub device: String,
    /// Total in bytes
    pub total_in_bytes: u64,
    /// Free in bytes
    pub free_in_bytes: u64,
    /// Available in bytes
    pub available_in_bytes: u64,
}

/// Network statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NetworkStats {
    /// Transport
    pub transport: TransportStats,
    /// HTTP
    pub http: HttpStats,
}

/// Transport statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TransportStats {
    /// Server open
    pub server_open: u32,
    /// Total inbound bytes
    pub rx_count: u64,
    /// Total outbound bytes
    pub tx_count: u64,
    /// Total inbound size in bytes
    pub rx_size_in_bytes: u64,
    /// Total outbound size in bytes
    pub tx_size_in_bytes: u64,
}

/// HTTP statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HttpStats {
    /// Current open
    pub current_open: u32,
    /// Total opened
    pub total_opened: u64,
}

/// Breakers statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BreakersStats {
    /// Breakers
    pub breakers: Vec<BreakerInfo>,
}

/// Breaker information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BreakerInfo {
    /// Name
    pub name: String,
    /// Limit size in bytes
    pub limit_size_in_bytes: u64,
    /// Limit size as percentage
    pub limit_size_as_percentage: f64,
    /// Estimated size in bytes
    pub estimated_size_in_bytes: u64,
    /// Estimated size as percentage
    pub estimated_size_as_percentage: f64,
    /// Overhead
    pub overhead: f64,
    /// Tripped
    pub tripped: u64,
}

/// Script statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ScriptStats {
    /// Compilations
    pub compilations: u64,
    /// Cache evictions
    pub cache_evictions: u64,
    /// Compilation limit triggered
    pub compilation_limit_triggered: u64,
}

/// Discovery statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DiscoveryStats {
    /// Cluster state queue
    pub cluster_state_queue: ClusterStateQueue,
}

/// Cluster state queue
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClusterStateQueue {
    /// Total
    pub total: u32,
    /// Pending
    pub pending: u32,
    /// Committed
    pub committed: u32,
}

/// Ingest statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IngestStats {
    /// Total
    pub total: IngestTotal,
    /// Pipelines
    pub pipelines: Vec<IngestPipeline>,
}

/// Ingest total
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IngestTotal {
    /// Count
    pub count: u64,
    /// Time in milliseconds
    pub time_in_millis: u64,
    /// Current
    pub current: u64,
    /// Failed
    pub failed: u64,
}

/// Ingest pipeline
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IngestPipeline {
    /// ID
    pub id: String,
    /// Count
    pub count: u64,
    /// Time in milliseconds
    pub time_in_millis: u64,
    /// Current
    pub current: u64,
    /// Failed
    pub failed: u64,
}

/// Adaptive selection statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AdaptiveSelectionStats {
    /// Adaptive selections
    pub adaptive_selections: Vec<AdaptiveSelection>,
}

/// Adaptive selection
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AdaptiveSelection {
    /// Node ID
    pub node_id: String,
    /// Outgoing searches
    pub outgoing_searches: u64,
    /// Rank
    pub rank: String,
}

/// Get cluster information
#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Cluster information retrieved successfully", body = ClusterInfo),
        (status = 500, description = "Internal server error")
    ),
    tag = "Cluster"
)]
pub async fn get_cluster_info(
    State(_state): State<AppState>,
) -> Result<Json<ClusterInfo>, StatusCode> {
    let cluster_info = ClusterInfo {
        name: "lexum-cluster".to_string(),
        cluster_uuid: "lexum-cluster-uuid".to_string(),
        version: VersionInfo {
            number: "0.1.0-alpha".to_string(),
            build_hash: "lexum-build-hash".to_string(),
            build_date: "2025-10-25T00:00:00Z".to_string(),
            build_snapshot: false,
            lucene_version: "9.0.0".to_string(),
            minimum_wire_compatibility_version: "0.1.0".to_string(),
            minimum_index_compatibility_version: "0.1.0".to_string(),
        },
        tagline: "You Know, for Search".to_string(),
    };

    Ok(Json(cluster_info))
}

/// Get cluster health
#[utoipa::path(
    get,
    path = "/_cluster/health",
    responses(
        (status = 200, description = "Cluster health retrieved successfully", body = ClusterHealth),
        (status = 500, description = "Internal server error")
    ),
    tag = "Cluster"
)]
pub async fn get_cluster_health(
    State(_state): State<AppState>,
) -> Result<Json<ClusterHealth>, StatusCode> {
    let cluster_health = ClusterHealth {
        cluster_name: "lexum-cluster".to_string(),
        status: "green".to_string(),
        timed_out: false,
        number_of_nodes: 1,
        number_of_data_nodes: 1,
        active_primary_shards: 0,
        active_shards: 0,
        relocating_shards: 0,
        initializing_shards: 0,
        unassigned_shards: 0,
        delayed_unassigned_shards: 0,
        number_of_pending_tasks: 0,
        number_of_in_flight_fetch: 0,
        task_max_waiting_in_queue_millis: 0,
        active_shards_percent_as_number: 100.0,
    };

    Ok(Json(cluster_health))
}

/// Get cluster statistics
#[utoipa::path(
    get,
    path = "/_cluster/stats",
    responses(
        (status = 200, description = "Cluster statistics retrieved successfully", body = ClusterStats),
        (status = 500, description = "Internal server error")
    ),
    tag = "Cluster"
)]
pub async fn get_cluster_stats(
    State(_state): State<AppState>,
) -> Result<Json<ClusterStats>, StatusCode> {
    let cluster_stats = ClusterStats {
        cluster_name: "lexum-cluster".to_string(),
        cluster_uuid: "lexum-cluster-uuid".to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        status: "green".to_string(),
        indices: IndicesStats {
            count: 0,
            shards: ShardsStats {
                total: 0,
                primaries: 0,
                replication: 0,
                unassigned: 0,
            },
            docs: DocsStats {
                count: 0,
                deleted: 0,
            },
            store: StoreStats {
                size_in_bytes: 0,
                reserved_in_bytes: 0,
            },
            fielddata: FieldDataStats {
                memory_size_in_bytes: 0,
                evictions: 0,
            },
            query_cache: QueryCacheStats {
                memory_size_in_bytes: 0,
                total_count: 0,
                hit_count: 0,
                miss_count: 0,
                cache_count: 0,
                cache_size: 0,
                evictions: 0,
            },
            completion: CompletionStats { size_in_bytes: 0 },
            segments: SegmentsStats {
                count: 0,
                memory_in_bytes: 0,
                terms_memory_in_bytes: 0,
                stored_fields_memory_in_bytes: 0,
                term_vectors_memory_in_bytes: 0,
                norms_memory_in_bytes: 0,
                points_memory_in_bytes: 0,
                doc_values_memory_in_bytes: 0,
                index_writer_memory_in_bytes: 0,
                version_map_memory_in_bytes: 0,
                fixed_bit_set_memory_in_bytes: 0,
            },
            translog: TranslogStats {
                operations: 0,
                size_in_bytes: 0,
                uncommitted_operations: 0,
                uncommitted_size_in_bytes: 0,
                earliest_last_modified_age: 0,
            },
            request_cache: RequestCacheStats {
                memory_size_in_bytes: 0,
                evictions: 0,
                hit_count: 0,
                miss_count: 0,
            },
            recovery: RecoveryStats {
                current_as_source: 0,
                current_as_target: 0,
                throttled_until: None,
            },
        },
        nodes: NodesStats {
            count: NodesCount {
                total: 1,
                master_only: 1,
                data: 1,
                coordinating_only: 0,
                ingest: 1,
                ml: 0,
                remote_cluster_client: 0,
            },
            versions: vec!["0.1.0-alpha".to_string()],
            os: OsStats {
                available_processors: 4,
                allocated_processors: 4,
                names: vec![OsName {
                    count: 1,
                    name: "Linux".to_string(),
                }],
                pretty_names: vec![OsPrettyName {
                    count: 1,
                    pretty_name: "Ubuntu 24.04".to_string(),
                }],
                architectures: vec![OsArchitecture {
                    count: 1,
                    arch: "amd64".to_string(),
                }],
            },
            process: ProcessStats {
                open_file_descriptors: 0,
                max_file_descriptors: 65536,
                cpu: CpuStats {
                    percent: 0,
                    total_in_millis: 0,
                },
                mem: MemoryStats {
                    total_virtual_in_bytes: 0,
                },
            },
            jvm: JvmStats {
                max_uptime_in_millis: 0,
                versions: vec![JvmVersion {
                    version: "21.0.0".to_string(),
                    vm_name: "OpenJDK 64-Bit Server VM".to_string(),
                    vm_version: "21.0.0+35".to_string(),
                    vm_vendor: "Eclipse Adoptium".to_string(),
                    count: 1,
                }],
                mem: JvmMemoryStats {
                    heap_used_in_bytes: 0,
                    heap_used_percent: 0,
                    heap_max_in_bytes: 0,
                    non_heap_used_in_bytes: 0,
                    non_heap_committed_in_bytes: 0,
                    pools: vec![],
                },
                threads: JvmThreadsStats {
                    count: 0,
                    peak_count: 0,
                },
                gc: JvmGcStats { collectors: vec![] },
                buffer_pools: vec![],
                classes: JvmClassesStats {
                    current_loaded_class_count: 0,
                    total_loaded_class_count: 0,
                    total_unloaded_class_count: 0,
                },
            },
            thread_pool: ThreadPoolStats {
                thread_pools: vec![],
            },
            fs: FsStats {
                total: FsTotal {
                    total_in_bytes: 0,
                    free_in_bytes: 0,
                    available_in_bytes: 0,
                },
                data: vec![],
            },
            network: NetworkStats {
                transport: TransportStats {
                    server_open: 0,
                    rx_count: 0,
                    tx_count: 0,
                    rx_size_in_bytes: 0,
                    tx_size_in_bytes: 0,
                },
                http: HttpStats {
                    current_open: 0,
                    total_opened: 0,
                },
            },
            transport: TransportStats {
                server_open: 0,
                rx_count: 0,
                tx_count: 0,
                rx_size_in_bytes: 0,
                tx_size_in_bytes: 0,
            },
            http: HttpStats {
                current_open: 0,
                total_opened: 0,
            },
            breakers: BreakersStats { breakers: vec![] },
            script: ScriptStats {
                compilations: 0,
                cache_evictions: 0,
                compilation_limit_triggered: 0,
            },
            discovery: DiscoveryStats {
                cluster_state_queue: ClusterStateQueue {
                    total: 0,
                    pending: 0,
                    committed: 0,
                },
            },
            ingest: IngestStats {
                total: IngestTotal {
                    count: 0,
                    time_in_millis: 0,
                    current: 0,
                    failed: 0,
                },
                pipelines: vec![],
            },
            adaptive_selection: AdaptiveSelectionStats {
                adaptive_selections: vec![],
            },
        },
    };

    Ok(Json(cluster_stats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::index::AppState;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    fn create_test_app() -> Router {
        let state = AppState::default();
        Router::new()
            .route("/", axum::routing::get(get_cluster_info))
            .route("/_cluster/health", axum::routing::get(get_cluster_health))
            .route("/_cluster/stats", axum::routing::get(get_cluster_stats))
            .with_state(state)
    }

    #[lexum_macros::tokio_test]
    async fn test_get_cluster_info() {
        let app = create_test_app();
        let request = Request::builder()
            .uri("/")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[lexum_macros::tokio_test]
    async fn test_get_cluster_health() {
        let app = create_test_app();
        let request = Request::builder()
            .uri("/_cluster/health")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[lexum_macros::tokio_test]
    async fn test_get_cluster_stats() {
        let app = create_test_app();
        let request = Request::builder()
            .uri("/_cluster/stats")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
