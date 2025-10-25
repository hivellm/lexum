//! Index management handlers

use crate::error::{ApiError, ApiResult};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use lexum_core::{
    FieldConfig, FieldType, IndexManager, IndexSettings, SchemaBuilder, SnapshotManager,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

/// Application state
#[derive(Clone)]
pub struct AppState {
    /// Index manager
    pub index_manager: Arc<IndexManager>,
    /// Snapshot manager
    pub snapshot_manager: Arc<SnapshotManager>,
}

impl Default for AppState {
    fn default() -> Self {
        // Create a temporary directory for testing purposes
        let temp_dir = std::env::temp_dir().join("lexum_test");
        std::fs::create_dir_all(&temp_dir).ok();

        // Create default config for snapshot manager
        let config = lexum_core::config::Config::default();
        let snapshot_manager = Arc::new(SnapshotManager::new(&config).unwrap_or_else(|_| {
            // Fallback to a minimal config if default fails
            let mut fallback_config = config;
            fallback_config.snapshots.repositories =
                vec![lexum_core::config::SnapshotRepositoryConfig {
                    name: "default".to_string(),
                    repository_type: "fs".to_string(),
                    settings: lexum_core::config::SnapshotRepositorySettings {
                        location: temp_dir.join("snapshots").to_string_lossy().to_string(),
                        ..Default::default()
                    },
                }];
            SnapshotManager::new(&fallback_config).unwrap()
        }));

        Self {
            index_manager: Arc::new(IndexManager::new(&temp_dir)),
            snapshot_manager,
        }
    }
}

/// Field definition in schema
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FieldDefinition {
    /// Field name
    pub name: String,
    /// Field type
    #[serde(rename = "type")]
    pub field_type: String,
    /// Is stored
    #[serde(default)]
    pub stored: bool,
    /// Is indexed
    #[serde(default = "default_true")]
    pub indexed: bool,
    /// Is fast (column-oriented)
    #[serde(default)]
    pub fast: bool,
}

fn default_true() -> bool {
    true
}

/// Create index request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateIndexRequest {
    /// Index name
    pub name: String,
    /// Schema fields
    pub fields: Vec<FieldDefinition>,
    /// Index settings
    #[serde(default)]
    pub settings: IndexSettings,
}

/// Index info response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexInfo {
    /// Index name
    pub name: String,
    /// Number of documents
    pub num_docs: u64,
}

/// List indices response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListIndicesResponse {
    /// Indices
    pub indices: Vec<IndexInfo>,
}

/// Create index handler
#[utoipa::path(
    post,
    path = "/api/v1/indices",
    request_body = CreateIndexRequest,
    responses(
        (status = 201, description = "Index created successfully", body = IndexInfo),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 409, description = "Index already exists", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Indices"
)]
pub async fn create_index(
    State(state): State<AppState>,
    Json(request): Json<CreateIndexRequest>,
) -> ApiResult<(StatusCode, Json<IndexInfo>)> {
    // Build schema
    let mut builder = SchemaBuilder::new();

    for field in &request.fields {
        let field_type = match field.field_type.as_str() {
            "text" => FieldType::Text,
            "keyword" => FieldType::Keyword,
            "i64" => FieldType::I64,
            "f64" => FieldType::F64,
            "date" => FieldType::Date,
            "boolean" => FieldType::Boolean,
            _ => {
                return Err(ApiError::InvalidRequest(format!(
                    "Unknown field type: {}",
                    field.field_type
                )));
            }
        };

        let mut field_config = FieldConfig::new(&field.name, field_type);

        if field.stored {
            field_config = field_config.stored(true);
        }
        if field.indexed {
            field_config = field_config.indexed(true);
        }
        if field.fast {
            field_config = field_config.fast(true);
        }

        builder = builder.add_field(field_config);
    }

    let (schema, _) = builder.build()?;

    // Create index
    let index = state
        .index_manager
        .create_index(&request.name, schema, request.settings)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(IndexInfo {
            name: index.name().to_string(),
            num_docs: 0,
        }),
    ))
}

/// Get index handler
#[utoipa::path(
    get,
    path = "/api/v1/indices/{name}",
    params(
        ("name" = String, Path, description = "Index name")
    ),
    responses(
        (status = 200, description = "Index information", body = IndexInfo),
        (status = 404, description = "Index not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Indices"
)]
pub async fn get_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<Json<IndexInfo>> {
    let stats = state
        .index_manager
        .get_index_stats(&name)
        .map_err(|_| ApiError::IndexNotFound(name.clone()))?;

    Ok(Json(IndexInfo {
        name: stats.name,
        num_docs: stats.num_docs,
    }))
}

/// List indices handler
#[utoipa::path(
    get,
    path = "/api/v1/indices",
    responses(
        (status = 200, description = "List of indices", body = ListIndicesResponse),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Indices"
)]
pub async fn list_indices(State(state): State<AppState>) -> ApiResult<Json<ListIndicesResponse>> {
    let index_names = state.index_manager.list_indices();

    let mut index_infos = Vec::new();
    for name in index_names {
        if let Ok(stats) = state.index_manager.get_index_stats(&name) {
            index_infos.push(IndexInfo {
                name: stats.name,
                num_docs: stats.num_docs,
            });
        }
    }

    Ok(Json(ListIndicesResponse {
        indices: index_infos,
    }))
}

/// Delete index handler
#[utoipa::path(
    delete,
    path = "/api/v1/indices/{name}",
    params(
        ("name" = String, Path, description = "Index name")
    ),
    responses(
        (status = 204, description = "Index deleted successfully"),
        (status = 404, description = "Index not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Indices"
)]
pub async fn delete_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    state.index_manager.delete_index(&name).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexStats {
    /// Index name
    pub name: String,
    /// Number of documents
    pub num_docs: u64,
    /// Number of segments
    pub num_segments: usize,
}

/// Get index statistics
#[utoipa::path(
    get,
    path = "/api/v1/indices/{name}/stats",
    params(
        ("name" = String, Path, description = "Index name")
    ),
    responses(
        (status = 200, description = "Index statistics retrieved successfully", body = IndexStats),
        (status = 404, description = "Index not found")
    ),
    tag = "Indices"
)]
pub async fn get_index_stats(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<Json<IndexStats>> {
    let core_stats = state.index_manager.get_index_stats(&name)?;
    let stats = IndexStats {
        name: core_stats.name,
        num_docs: core_stats.num_docs,
        num_segments: core_stats.num_segments,
    };
    Ok(Json(stats))
}

/// Refresh index
#[utoipa::path(
    post,
    path = "/api/v1/indices/{name}/refresh",
    params(
        ("name" = String, Path, description = "Index name")
    ),
    responses(
        (status = 200, description = "Index refreshed successfully"),
        (status = 404, description = "Index not found")
    ),
    tag = "Indices"
)]
pub async fn refresh_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    // Check if index exists
    state.index_manager.get_index(&name)?;
    // TODO: Implement actual refresh logic
    Ok(StatusCode::OK)
}

/// Flush index
#[utoipa::path(
    post,
    path = "/api/v1/indices/{name}/flush",
    params(
        ("name" = String, Path, description = "Index name")
    ),
    responses(
        (status = 200, description = "Index flushed successfully"),
        (status = 404, description = "Index not found")
    ),
    tag = "Indices"
)]
pub async fn flush_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    // Check if index exists
    state.index_manager.get_index(&name)?;
    // TODO: Implement actual flush logic
    Ok(StatusCode::OK)
}
