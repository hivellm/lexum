//! Index management handlers

use crate::error::{ApiError, ApiResult};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use lexum_core::{FieldConfig, FieldType, IndexManager, IndexSettings, SchemaBuilder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Application state
#[derive(Clone)]
pub struct AppState {
    /// Index manager
    pub index_manager: Arc<IndexManager>,
}

/// Field definition in schema
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    /// Index name
    pub name: String,
    /// Number of documents
    pub num_docs: u64,
}

/// List indices response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListIndicesResponse {
    /// Indices
    pub indices: Vec<IndexInfo>,
}

/// Create index handler
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
pub async fn delete_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    state.index_manager.delete_index(&name).await?;
    Ok(StatusCode::NO_CONTENT)
}
