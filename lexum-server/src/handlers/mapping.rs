//! Elasticsearch mapping handlers

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use lexum_core::schema::{ElasticsearchMapping, schema_to_mapping};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Get mapping response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetMappingResponse {
    /// Index name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<String>,
    /// Mapping data (serialized as JSON)
    #[serde(flatten)]
    pub mappings: serde_json::Value,
}

/// Update mapping request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateMappingRequest {
    /// Mapping data (as JSON)
    pub mappings: serde_json::Value,
}

/// Get index mapping handler
#[utoipa::path(
    get,
    path = "/api/v1/indices/{index}/_mapping",
    params(
        ("index" = String, Path, description = "Index name")
    ),
    responses(
        (status = 200, description = "Mapping retrieved successfully", body = GetMappingResponse),
        (status = 404, description = "Index not found", body = ApiError)
    ),
    tag = "Mappings"
)]
pub async fn get_mapping(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
) -> ApiResult<Json<GetMappingResponse>> {
    // Resolve index alias if needed
    let resolved_index = state
        .index_manager
        .resolve_alias(&index_name)
        .ok()
        .and_then(|indices| indices.first().map(|idx| idx.to_string()))
        .unwrap_or_else(|| index_name.clone());

    let index = state
        .index_manager
        .get_index(&resolved_index)
        .map_err(|e| {
            // Convert Validation error for "not found" to IndexNotFound
            if let lexum_core::Error::Validation(ref msg) = e {
                if msg.contains("not found") || msg.contains("does not exist") {
                    return ApiError::IndexNotFound(resolved_index);
                }
            }
            let error_msg = e.to_string();
            if error_msg.contains("not found") || error_msg.contains("does not exist") {
                ApiError::IndexNotFound(resolved_index)
            } else {
                tracing::error!("Failed to get index '{}': {}", resolved_index, error_msg);
                ApiError::Core(e)
            }
        })?;

    // Convert schema to Elasticsearch mapping
    let schema = index.schema();
    let mapping = schema_to_mapping(&schema)
        .map_err(|e| ApiError::Internal(format!("Failed to convert schema to mapping: {e}")))?;

    // Serialize to JSON value for response
    let mappings_json = serde_json::to_value(&mapping)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize mapping: {e}")))?;

    Ok(Json(GetMappingResponse {
        index: Some(index_name),
        mappings: mappings_json,
    }))
}

/// Update index mapping handler
#[utoipa::path(
    put,
    path = "/api/v1/indices/{index}/_mapping",
    params(
        ("index" = String, Path, description = "Index name")
    ),
    request_body = UpdateMappingRequest,
    responses(
        (status = 200, description = "Mapping updated successfully"),
        (status = 400, description = "Invalid mapping", body = ApiError),
        (status = 404, description = "Index not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Mappings"
)]
pub async fn update_mapping(
    State(_state): State<AppState>,
    Path(_index_name): Path<String>,
    Json(request): Json<UpdateMappingRequest>,
) -> ApiResult<StatusCode> {
    // Parse mapping from JSON
    let mapping: ElasticsearchMapping = serde_json::from_value(request.mappings)
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid mapping format: {e}")))?;

    // Validate mapping
    mapping
        .validate()
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid mapping: {e}")))?;

    // TODO: Implement mapping update
    // This requires schema modification which is not directly supported in Tantivy
    // For now, we'll return a not implemented error
    Err(ApiError::Internal(
        "Mapping updates are not yet supported. Index schemas cannot be modified after creation."
            .to_string(),
    ))
}

/// Get field mapping response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetFieldMappingResponse {
    /// Index name
    pub index: String,
    /// Field name
    pub field: String,
    /// Field mapping data (serialized as JSON)
    #[serde(flatten)]
    pub mapping: serde_json::Value,
}

/// Get all mappings response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetAllMappingsResponse {
    /// Mappings by index name
    pub mappings: std::collections::HashMap<String, serde_json::Value>,
}

/// Get field mapping handler
#[utoipa::path(
    get,
    path = "/api/v1/indices/{index}/_mapping/{field}",
    params(
        ("index" = String, Path, description = "Index name"),
        ("field" = String, Path, description = "Field name")
    ),
    responses(
        (status = 200, description = "Field mapping retrieved successfully", body = GetFieldMappingResponse),
        (status = 404, description = "Index or field not found", body = ApiError)
    ),
    tag = "Mappings"
)]
pub async fn get_field_mapping(
    State(state): State<AppState>,
    Path((index_name, field_name)): Path<(String, String)>,
) -> ApiResult<Json<GetFieldMappingResponse>> {
    // Resolve index alias if needed
    let resolved_index = state
        .index_manager
        .resolve_alias(&index_name)
        .ok()
        .and_then(|indices| indices.first().map(|idx| idx.to_string()))
        .unwrap_or_else(|| index_name.clone());

    let index = state
        .index_manager
        .get_index(&resolved_index)
        .map_err(|e| {
            // Convert Validation error for "not found" to IndexNotFound
            if let lexum_core::Error::Validation(ref msg) = e {
                if msg.contains("not found") || msg.contains("does not exist") {
                    return ApiError::IndexNotFound(resolved_index);
                }
            }
            let error_msg = e.to_string();
            if error_msg.contains("not found") || error_msg.contains("does not exist") {
                ApiError::IndexNotFound(resolved_index)
            } else {
                tracing::error!("Failed to get index '{}': {}", resolved_index, error_msg);
                ApiError::Core(e)
            }
        })?;

    // Convert schema to Elasticsearch mapping
    let schema = index.schema();
    let mapping = schema_to_mapping(&schema)
        .map_err(|e| ApiError::Internal(format!("Failed to convert schema to mapping: {e}")))?;

    // Extract field mapping
    let field_mapping = mapping
        .properties
        .as_ref()
        .and_then(|props| props.get(&field_name))
        .ok_or_else(|| ApiError::InvalidRequest(format!("Field '{field_name}' not found")))?;

    // Serialize field mapping to JSON
    let field_mapping_json = serde_json::to_value(field_mapping)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize field mapping: {e}")))?;

    Ok(Json(GetFieldMappingResponse {
        index: index_name,
        field: field_name,
        mapping: field_mapping_json,
    }))
}

/// Get all mappings handler
#[utoipa::path(
    get,
    path = "/api/v1/_mapping",
    responses(
        (status = 200, description = "All mappings retrieved successfully", body = GetAllMappingsResponse)
    ),
    tag = "Mappings"
)]
pub async fn get_all_mappings(
    State(state): State<AppState>,
) -> ApiResult<Json<GetAllMappingsResponse>> {
    let index_names = state.index_manager.list_indices();
    let mut all_mappings = std::collections::HashMap::new();

    for index_name in index_names {
        if let Ok(index) = state.index_manager.get_index(&index_name) {
            let schema = index.schema();
            if let Ok(mapping) = schema_to_mapping(&schema) {
                if let Ok(mapping_json) = serde_json::to_value(&mapping) {
                    all_mappings.insert(index_name, mapping_json);
                }
            }
        }
    }

    Ok(Json(GetAllMappingsResponse {
        mappings: all_mappings,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::index::AppState;
    use crate::handlers::metrics::PrometheusMetrics;
    use crate::handlers::reindex::TaskManager;
    use crate::middleware::auth::AuthState;
    use crate::middleware::query_complexity::QueryComplexityLimitConfig;
    use lexum_core::{ProgressTracker, SnapshotManager, TemplateManager, index::IndexManager};
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn create_test_app_state() -> (tempfile::TempDir, AppState) {
        use tempfile::TempDir;
        // Create a proper temporary directory in Linux native path to avoid WSL filesystem issues
        // Use /tmp instead of Windows-mounted drives for better Tantivy compatibility
        let temp_dir = TempDir::new_in("/tmp").unwrap_or_else(|_| {
            // Fallback to default tempdir if /tmp is not available
            TempDir::new().expect("Failed to create temp dir")
        });
        let temp_path = temp_dir.path();
        let index_manager = Arc::new(IndexManager::new(temp_path));
        // Create default config for snapshot manager
        let config = lexum_core::config::Config::default();
        let snapshot_manager = Arc::new(RwLock::new(SnapshotManager::new(&config).unwrap_or_else(
            |_| {
                // Fallback to a minimal config if default fails
                let mut fallback_config = config;
                fallback_config.snapshots.repositories =
                    vec![lexum_core::config::SnapshotRepositoryConfig {
                        name: "default".to_string(),
                        repository_type: "fs".to_string(),
                        settings: lexum_core::config::SnapshotRepositorySettings {
                            location: temp_path.join("snapshots").to_string_lossy().to_string(),
                            ..Default::default()
                        },
                    }];
                SnapshotManager::new(&fallback_config).unwrap()
            },
        )));
        let template_manager = Arc::new(TemplateManager::new());
        let task_manager = Arc::new(TaskManager::new());
        let progress_tracker = Arc::new(ProgressTracker::new());
        let auth_state = AuthState::new(crate::middleware::auth::AuthConfig::default());
        let query_complexity_config = QueryComplexityLimitConfig::default();
        let metrics = Arc::new(PrometheusMetrics::new());

        let app_state = AppState {
            index_manager,
            snapshot_manager,
            template_manager,
            task_manager,
            progress_tracker,
            auth_state,
            query_complexity_config,
            metrics,
        };

        (temp_dir, app_state)
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy filesystem compatibility issue - use Windows native or Linux native paths"]
    async fn test_get_mapping() {
        let (_temp_dir, state) = create_test_app_state();

        // Create a test index using lexum-core's SchemaBuilder
        let schema = lexum_core::SchemaBuilder::new()
            .add_text_field("title")
            .add_i64_field("price")
            .build()
            .unwrap()
            .0;

        state
            .index_manager
            .create_index(
                "test_index",
                schema,
                lexum_core::index::IndexSettings {
                    storage: lexum_core::index::StorageSettings {
                        enable_memory_mapped_storage: false,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Get mapping
        let result = get_mapping(State(state), Path("test_index".to_string())).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        // Check that mappings is a JSON object with properties
        if let serde_json::Value::Object(ref obj) = response.0.mappings {
            if let Some(serde_json::Value::Object(properties)) = obj.get("properties") {
                assert!(properties.contains_key("title"));
                assert!(properties.contains_key("price"));
            } else {
                panic!("Expected properties field in mappings");
            }
        } else {
            panic!("Expected mappings to be a JSON object");
        }
    }

    #[tokio::test]
    async fn test_get_mapping_not_found() {
        let (_temp_dir, state) = create_test_app_state();

        let result = get_mapping(State(state), Path("nonexistent".to_string())).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ApiError::IndexNotFound(_) => {}
            _ => panic!("Expected IndexNotFound error"),
        }
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy filesystem compatibility issue - use Windows native or Linux native paths"]
    async fn test_create_index_with_copy_to() {
        use crate::handlers::index::create_index;
        use lexum_core::document::store::DocumentStore;
        use lexum_core::query::QueryBuilder;
        use lexum_core::search::executor::SearchExecutor;
        use std::sync::Arc;

        // Keep TempDir alive during the test
        let (_temp_dir, state) = create_test_app_state();

        // Create index with copy_to mapping
        let create_request = crate::handlers::index::CreateIndexRequest {
            name: "test_copy_to_index".to_string(),
            fields: vec![],
            mappings: Some(json!({
                "properties": {
                    "title": {
                        "type": "text",
                        "copy_to": "full_text"
                    },
                    "content": {
                        "type": "text",
                        "copy_to": "full_text"
                    },
                    "full_text": {
                        "type": "text"
                    }
                }
            })),
            settings: lexum_core::index::IndexSettings {
                storage: lexum_core::index::StorageSettings {
                    enable_memory_mapped_storage: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        };

        let result = create_index(State(state.clone()), Json(create_request)).await;

        assert!(
            result.is_ok(),
            "Index creation should succeed: {:?}",
            result.err()
        );

        // Add a document
        let index = state.index_manager.get_index("test_copy_to_index").unwrap();
        let store = DocumentStore::new(Arc::new(index));

        let doc = json!({
            "title": "Test Title",
            "content": "Test Content"
        });

        let _doc_id = store.add_document(doc).await.unwrap();

        // Verify that copy_to was applied by searching in full_text field
        let index_for_search = state.index_manager.get_index("test_copy_to_index").unwrap();
        let search_executor = SearchExecutor::new(Arc::new(index_for_search));

        // Search for "Test Title" in full_text field (should find it because copy_to was applied)
        let query = QueryBuilder::match_query("full_text", "Test Title");
        let results = search_executor.search(query, 10, 0, None).await.unwrap();

        assert!(
            !results.hits.is_empty(),
            "Document should be found via copy_to field"
        );
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy filesystem compatibility issue - use Windows native or Linux native paths"]
    async fn test_dynamic_mapping_strict_validation() {
        use crate::handlers::index::create_index;
        use lexum_core::document::store::DocumentStore;
        use std::sync::Arc;

        // Keep TempDir alive during the test
        let (_temp_dir, state) = create_test_app_state();

        // Create index with strict dynamic mapping
        let create_request = crate::handlers::index::CreateIndexRequest {
            name: "test_strict_index".to_string(),
            fields: vec![],
            mappings: Some(json!({
                "dynamic": "strict",
                "properties": {
                    "title": {
                        "type": "text"
                    }
                }
            })),
            settings: lexum_core::index::IndexSettings {
                storage: lexum_core::index::StorageSettings {
                    enable_memory_mapped_storage: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        };

        let result = create_index(State(state.clone()), Json(create_request)).await;

        assert!(
            result.is_ok(),
            "Index creation should succeed: {:?}",
            result.err()
        );

        // Try to add a document with unknown field (should fail)
        let index = state.index_manager.get_index("test_strict_index").unwrap();
        let store = DocumentStore::new(Arc::new(index));

        let invalid_doc = json!({
            "title": "Test Title",
            "unknown_field": "value"
        });

        let result = store.add_document(invalid_doc).await;
        assert!(
            result.is_err(),
            "Document with unknown field should be rejected in strict mode"
        );
        assert!(result.unwrap_err().to_string().contains("unmapped field"));

        // Try to add a document with only known fields (should succeed)
        let valid_doc = json!({
            "title": "Test Title"
        });

        let result = store.add_document(valid_doc).await;
        assert!(
            result.is_ok(),
            "Document with only known fields should be accepted"
        );
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy filesystem compatibility issue - use Windows native or Linux native paths"]
    async fn test_dynamic_mapping_false_allows_unknown() {
        use crate::handlers::index::create_index;
        use lexum_core::document::store::DocumentStore;
        use std::sync::Arc;

        // Keep TempDir alive during the test
        let (_temp_dir, state) = create_test_app_state();

        // Create index with dynamic=false
        let create_request = crate::handlers::index::CreateIndexRequest {
            name: "test_dynamic_false_index".to_string(),
            fields: vec![],
            mappings: Some(json!({
                "dynamic": false,
                "properties": {
                    "title": {
                        "type": "text"
                    }
                }
            })),
            settings: lexum_core::index::IndexSettings {
                storage: lexum_core::index::StorageSettings {
                    enable_memory_mapped_storage: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        };

        let result = create_index(State(state.clone()), Json(create_request)).await;

        assert!(
            result.is_ok(),
            "Index creation should succeed: {:?}",
            result.err()
        );

        // Try to add a document with unknown field (should succeed, unknown fields ignored)
        let index = state
            .index_manager
            .get_index("test_dynamic_false_index")
            .unwrap();
        let store = DocumentStore::new(Arc::new(index));

        let doc = json!({
            "title": "Test Title",
            "unknown_field": "value"
        });

        let result = store.add_document(doc).await;
        assert!(
            result.is_ok(),
            "Document with unknown field should be accepted in false mode"
        );
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy filesystem compatibility issue - use Windows native or Linux native paths"]
    async fn test_create_index_with_template_mapping() {
        use crate::handlers::index::create_index;
        use crate::handlers::template::{
            PutTemplateRequest, TemplateMappingsRequest, TemplateSettingsRequest, put_template,
        };
        use std::collections::HashMap;

        // Keep TempDir alive during the test
        let (_temp_dir, state) = create_test_app_state();

        // Create a template with mappings
        let mut template_mappings = HashMap::new();
        template_mappings.insert(
            "template_field".to_string(),
            json!({
                "type": "text"
            }),
        );

        let template_request = PutTemplateRequest {
            index_patterns: vec!["test_template_*".to_string()],
            priority: 0,
            version: 1,
            settings: TemplateSettingsRequest {
                number_of_shards: 1,
                number_of_replicas: 0,
                refresh_interval: 1,
                custom: HashMap::new(),
            },
            mappings: TemplateMappingsRequest {
                properties: template_mappings,
            },
            order: 0,
        };

        let template_result = put_template(
            State(state.clone()),
            Path("test_template".to_string()),
            Json(template_request),
        )
        .await;

        assert!(template_result.is_ok(), "Template creation should succeed");

        // Create index matching template pattern (should apply template mappings)
        let create_request = crate::handlers::index::CreateIndexRequest {
            name: "test_template_index".to_string(),
            fields: vec![],
            mappings: Some(json!({
                "properties": {
                    "request_field": {
                        "type": "text"
                    }
                }
            })),
            settings: lexum_core::index::IndexSettings {
                storage: lexum_core::index::StorageSettings {
                    enable_memory_mapped_storage: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        };

        let result: ApiResult<(StatusCode, Json<crate::handlers::index::IndexInfo>)> =
            create_index(State(state.clone()), Json(create_request)).await;

        assert!(
            result.is_ok(),
            "Index creation should succeed: {:?}",
            result.err()
        );

        // Verify that template mappings were merged with request mappings
        let index = state
            .index_manager
            .get_index("test_template_index")
            .unwrap();

        // Check that mapping exists (template field + request field)
        if let Some(mapping) = index.mapping() {
            if let Some(properties) = &mapping.properties {
                assert!(
                    properties.contains_key("template_field"),
                    "Template field should be present"
                );
                assert!(
                    properties.contains_key("request_field"),
                    "Request field should be present"
                );
            }
        }
    }
}
