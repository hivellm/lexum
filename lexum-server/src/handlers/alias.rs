//! Index alias management endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use lexum_core::index::alias::{
    AliasAction as CoreAliasAction, AliasConfig as CoreAliasConfig,
    AliasOperationsRequest as CoreAliasOperationsRequest,
    AliasOperationsResponse as CoreAliasOperationsResponse, IndexAlias as CoreIndexAlias,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use crate::handlers::index::AppState;

/// Index alias information (API representation)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexAlias {
    /// Alias name
    pub name: String,
    /// List of indices this alias points to
    pub indices: Vec<String>,
    /// Alias filter (optional)
    pub filter: Option<serde_json::Value>,
    /// Routing information
    pub routing: Option<String>,
    /// Search routing information
    pub search_routing: Option<String>,
    /// Index routing information
    pub index_routing: Option<String>,
    /// Whether this is a write alias
    pub is_write_index: Option<bool>,
}

/// Alias action for atomic operations (API representation)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AliasAction {
    /// Action type (add, remove)
    pub action: String,
    /// Index name
    pub index: String,
    /// Alias name
    pub alias: String,
    /// Alias filter (optional)
    pub filter: Option<serde_json::Value>,
    /// Routing information (optional)
    pub routing: Option<String>,
    /// Search routing information (optional)
    pub search_routing: Option<String>,
    /// Index routing information (optional)
    pub index_routing: Option<String>,
    /// Whether this is a write alias
    pub is_write_index: Option<bool>,
}

/// Alias operations request (API representation)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AliasOperationsRequest {
    /// List of alias actions
    pub actions: Vec<AliasAction>,
}

/// Alias operations response (API representation)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AliasOperationsResponse {
    /// Whether all operations succeeded
    pub acknowledged: bool,
    /// Error message if any operation failed
    pub error: Option<String>,
    /// Number of operations executed
    pub executed_operations: Option<usize>,
    /// Whether the operation was atomic (all succeeded or all failed)
    pub atomic: Option<bool>,
}

impl From<CoreIndexAlias> for IndexAlias {
    fn from(core_alias: CoreIndexAlias) -> Self {
        Self {
            name: core_alias.name.as_str().to_string(),
            indices: core_alias
                .indices
                .into_iter()
                .map(|i| i.as_str().to_string())
                .collect(),
            filter: core_alias.config.filter,
            routing: core_alias.config.routing,
            search_routing: core_alias.config.search_routing,
            index_routing: core_alias.config.index_routing,
            is_write_index: core_alias.config.is_write_index,
        }
    }
}

impl From<AliasAction> for CoreAliasAction {
    fn from(api_action: AliasAction) -> Self {
        let alias_name = api_action.alias.into();
        let indices = vec![api_action.index.into()];
        let config = CoreAliasConfig {
            filter: api_action.filter,
            routing: api_action.routing,
            search_routing: api_action.search_routing,
            index_routing: api_action.index_routing,
            is_write_index: api_action.is_write_index,
        };

        match api_action.action.as_str() {
            "add" => CoreAliasAction::Add {
                alias: alias_name,
                indices,
                config: Some(config),
            },
            "remove" => CoreAliasAction::Remove {
                alias: alias_name,
                indices,
            },
            "remove_index" => CoreAliasAction::RemoveIndex { alias: alias_name },
            _ => panic!("Invalid action: {}", api_action.action),
        }
    }
}

impl From<AliasOperationsRequest> for CoreAliasOperationsRequest {
    fn from(api_request: AliasOperationsRequest) -> Self {
        Self {
            actions: api_request.actions.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<CoreAliasOperationsResponse> for AliasOperationsResponse {
    fn from(core_response: CoreAliasOperationsResponse) -> Self {
        Self {
            acknowledged: core_response.acknowledged,
            error: None, // Core response doesn't have error field
            executed_operations: Some(core_response.executed_operations),
            atomic: Some(core_response.atomic),
        }
    }
}

/// Get all aliases
#[utoipa::path(
    get,
    path = "/_aliases",
    responses(
        (status = 200, description = "Aliases retrieved successfully"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Aliases"
)]
pub async fn get_aliases(
    State(state): State<AppState>,
) -> Result<Json<HashMap<String, serde_json::Value>>, StatusCode> {
    let aliases = state.index_manager.list_aliases();
    let mut result = HashMap::new();
    for alias in aliases {
        let alias_name = alias.name.as_str().to_string();
        let alias_data = IndexAlias::from(alias);
        result.insert(
            alias_name,
            serde_json::to_value(alias_data).unwrap_or_default(),
        );
    }
    Ok(Json(result))
}

/// Get aliases for a specific index
#[utoipa::path(
    get,
    path = "/{index}/_alias",
    responses(
        (status = 200, description = "Index aliases retrieved successfully"),
        (status = 404, description = "Index not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Aliases"
)]
pub async fn get_index_aliases(
    State(state): State<AppState>,
    Path(index): Path<String>,
) -> Result<Json<HashMap<String, serde_json::Value>>, StatusCode> {
    let aliases = state.index_manager.get_aliases_for_index(&index);
    let mut result = HashMap::new();
    for alias in aliases {
        let alias_name = alias.name.as_str().to_string();
        let alias_data = IndexAlias::from(alias);
        result.insert(
            alias_name,
            serde_json::to_value(alias_data).unwrap_or_default(),
        );
    }
    Ok(Json(result))
}

/// Perform alias operations (add, remove)
#[utoipa::path(
    post,
    path = "/_aliases",
    request_body = AliasOperationsRequest,
    responses(
        (status = 200, description = "Alias operations completed successfully", body = AliasOperationsResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Aliases"
)]
pub async fn perform_alias_operations(
    State(state): State<AppState>,
    Json(request): Json<AliasOperationsRequest>,
) -> Result<Json<AliasOperationsResponse>, StatusCode> {
    tracing::info!("Performing alias operations: {:?}", request.actions);

    // Convert API request to core request
    let core_request: CoreAliasOperationsRequest = request.into();

    // Execute operations using the core alias manager
    match state.index_manager.execute_alias_operations(core_request) {
        Ok(response) => {
            tracing::info!("Alias operations completed successfully");
            Ok(Json(AliasOperationsResponse::from(response)))
        }
        Err(e) => {
            tracing::error!("Failed to execute alias operations: {}", e);
            Ok(Json(AliasOperationsResponse {
                acknowledged: false,
                error: Some(e.to_string()),
                executed_operations: None,
                atomic: None,
            }))
        }
    }
}

/// Perform atomic alias operations with transaction support
#[utoipa::path(
    post,
    path = "/_aliases/atomic",
    request_body = AliasOperationsRequest,
    responses(
        (status = 200, description = "Atomic alias operations completed successfully", body = AliasOperationsResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Aliases"
)]
pub async fn perform_atomic_alias_operations(
    State(state): State<AppState>,
    Json(request): Json<AliasOperationsRequest>,
) -> Result<Json<AliasOperationsResponse>, StatusCode> {
    tracing::info!("Performing atomic alias operations: {:?}", request.actions);

    // Convert API request to core request
    let core_request: CoreAliasOperationsRequest = request.into();

    // Execute atomic operations using the core alias manager
    match state
        .index_manager
        .execute_atomic_alias_operations(core_request)
    {
        Ok(response) => {
            tracing::info!("Atomic alias operations completed successfully");
            Ok(Json(AliasOperationsResponse::from(response)))
        }
        Err(e) => {
            tracing::error!("Failed to execute atomic alias operations: {}", e);
            Ok(Json(AliasOperationsResponse {
                acknowledged: false,
                error: Some(e.to_string()),
                executed_operations: None,
                atomic: Some(false),
            }))
        }
    }
}

/// Add alias to index
#[utoipa::path(
    put,
    path = "/{index}/_alias/{alias}",
    request_body = Option<serde_json::Value>,
    responses(
        (status = 200, description = "Alias added successfully"),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Index not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Aliases"
)]
pub async fn add_alias(
    State(state): State<AppState>,
    Path((index, alias)): Path<(String, String)>,
    Json(body): Json<Option<serde_json::Value>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!(
        "Adding alias '{}' to index '{}' with body: {:?}",
        alias,
        index,
        body
    );

    // Parse alias configuration from body (currently not used in core API)
    let _config = if let Some(body) = body {
        serde_json::from_value::<CoreAliasConfig>(body).unwrap_or_default()
    } else {
        CoreAliasConfig::default()
    };

    // Create alias using the core manager
    match state
        .index_manager
        .create_alias(alias.as_str(), vec![index.as_str().into()])
    {
        Ok(_) => {
            tracing::info!("Alias '{}' created successfully", alias);
            Ok(Json(serde_json::json!({
                "acknowledged": true
            })))
        }
        Err(e) => {
            tracing::error!("Failed to create alias '{}': {}", alias, e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Remove alias from index
#[utoipa::path(
    delete,
    path = "/{index}/_alias/{alias}",
    responses(
        (status = 200, description = "Alias removed successfully"),
        (status = 404, description = "Index or alias not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Aliases"
)]
pub async fn remove_alias(
    State(state): State<AppState>,
    Path((index, alias)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!("Removing alias '{}' from index '{}'", alias, index);

    // Remove indices from alias using the core manager
    match state
        .index_manager
        .remove_indices_from_alias(&alias, vec![index.as_str().into()])
    {
        Ok(_) => {
            tracing::info!("Alias '{}' updated successfully", alias);
            Ok(Json(serde_json::json!({
                "acknowledged": true
            })))
        }
        Err(e) => {
            // If the alias doesn't exist or has no indices, try to delete it completely
            if e.to_string().contains("not found") || e.to_string().contains("no indices") {
                match state.index_manager.delete_alias(&alias) {
                    Ok(_) => {
                        tracing::info!("Alias '{}' deleted successfully", alias);
                        Ok(Json(serde_json::json!({
                            "acknowledged": true
                        })))
                    }
                    Err(_) => {
                        tracing::error!("Failed to delete alias '{}': {}", alias, e);
                        Err(StatusCode::NOT_FOUND)
                    }
                }
            } else {
                tracing::error!("Failed to remove indices from alias '{}': {}", alias, e);
                Err(StatusCode::BAD_REQUEST)
            }
        }
    }
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
            .route("/_aliases", axum::routing::get(get_aliases))
            .route("/{index}/_alias", axum::routing::get(get_index_aliases))
            .route("/_aliases", axum::routing::post(perform_alias_operations))
            .route(
                "/_aliases/atomic",
                axum::routing::post(perform_atomic_alias_operations),
            )
            .route("/{index}/_alias/{alias}", axum::routing::put(add_alias))
            .route(
                "/{index}/_alias/{alias}",
                axum::routing::delete(remove_alias),
            )
            .with_state(state)
    }

    #[tokio::test]
    async fn test_get_aliases() {
        let app = create_test_app();
        let request = Request::builder()
            .uri("/_aliases")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_index_aliases() {
        let app = create_test_app();
        let request = Request::builder()
            .uri("/test_index/_alias")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_perform_alias_operations() {
        let app = create_test_app();
        let request_body = AliasOperationsRequest {
            actions: vec![AliasAction {
                action: "add".to_string(),
                index: "test_index".to_string(),
                alias: "test_alias".to_string(),
                filter: None,
                routing: None,
                search_routing: None,
                index_routing: None,
                is_write_index: None,
            }],
        };
        let request = Request::builder()
            .uri("/_aliases")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_add_alias() {
        let app = create_test_app();
        let request = Request::builder()
            .uri("/test_index/_alias/test_alias")
            .method("PUT")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        // Expect 400 because the index doesn't exist
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_remove_alias() {
        let app = create_test_app();
        let request = Request::builder()
            .uri("/test_index/_alias/test_alias")
            .method("DELETE")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        // Expect 404 because the alias doesn't exist
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_perform_atomic_alias_operations() {
        let app = create_test_app();
        let request_body = AliasOperationsRequest {
            actions: vec![
                AliasAction {
                    action: "add".to_string(),
                    index: "test_index1".to_string(),
                    alias: "test_alias1".to_string(),
                    filter: None,
                    routing: None,
                    search_routing: None,
                    index_routing: None,
                    is_write_index: None,
                },
                AliasAction {
                    action: "add".to_string(),
                    index: "test_index2".to_string(),
                    alias: "test_alias2".to_string(),
                    filter: None,
                    routing: None,
                    search_routing: None,
                    index_routing: None,
                    is_write_index: None,
                },
            ],
        };
        let request = Request::builder()
            .uri("/_aliases/atomic")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_perform_atomic_alias_operations_failure() {
        let app = create_test_app();
        let request_body = AliasOperationsRequest {
            actions: vec![
                AliasAction {
                    action: "add".to_string(),
                    index: "test_index1".to_string(),
                    alias: "test_alias1".to_string(),
                    filter: None,
                    routing: None,
                    search_routing: None,
                    index_routing: None,
                    is_write_index: None,
                },
                AliasAction {
                    action: "add".to_string(),
                    index: "test_index2".to_string(),
                    alias: "test_alias1".to_string(), // Duplicate alias name - should fail
                    filter: None,
                    routing: None,
                    search_routing: None,
                    index_routing: None,
                    is_write_index: None,
                },
            ],
        };
        let request = Request::builder()
            .uri("/_aliases/atomic")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        // Should fail due to duplicate alias
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // ============================================================================
    // Enhanced Server-Side Alias API Tests
    // ============================================================================

    #[tokio::test]
    async fn test_get_aliases_with_data() {
        let app = create_test_app();
        
        // First create some aliases
        let create_request = AliasOperationsRequest {
            actions: vec![
                AliasAction {
                    action: "add".to_string(),
                    index: "index1".to_string(),
                    alias: "alias1".to_string(),
                    filter: None,
                    routing: None,
                    search_routing: None,
                    index_routing: None,
                    is_write_index: None,
                },
                AliasAction {
                    action: "add".to_string(),
                    index: "index2".to_string(),
                    alias: "alias2".to_string(),
                    filter: None,
                    routing: None,
                    search_routing: None,
                    index_routing: None,
                    is_write_index: None,
                },
            ],
        };
        
        let create_req = Request::builder()
            .uri("/_aliases")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&create_request).unwrap()))
            .unwrap();
        let _create_response = app.clone().oneshot(create_req).await.unwrap();

        // Now test getting aliases
        let request = Request::builder()
            .uri("/_aliases")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_index_aliases_with_data() {
        let app = create_test_app();
        
        // First create an alias for an index
        let create_request = AliasOperationsRequest {
            actions: vec![AliasAction {
                action: "add".to_string(),
                index: "test_index".to_string(),
                alias: "test_alias".to_string(),
                filter: None,
                routing: None,
                search_routing: None,
                index_routing: None,
                is_write_index: None,
            }],
        };
        
        let create_req = Request::builder()
            .uri("/_aliases")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&create_request).unwrap()))
            .unwrap();
        let _create_response = app.clone().oneshot(create_req).await.unwrap();

        // Now test getting aliases for the index
        let request = Request::builder()
            .uri("/test_index/_alias")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_add_alias_with_config() {
        let app = create_test_app();
        let request_body = serde_json::json!({
            "filter": {
                "term": {
                    "status": "active"
                }
            },
            "routing": "user1",
            "search_routing": "user1",
            "index_routing": "user1",
            "is_write_index": true
        });
        
        let request = Request::builder()
            .uri("/test_index/_alias/test_alias")
            .method("PUT")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        // Expect 400 because the index doesn't exist
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_add_alias_invalid_json() {
        let app = create_test_app();
        let request = Request::builder()
            .uri("/test_index/_alias/test_alias")
            .method("PUT")
            .header("content-type", "application/json")
            .body(Body::from("invalid json"))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_remove_alias_success() {
        let app = create_test_app();
        
        // First create an alias
        let create_request = AliasOperationsRequest {
            actions: vec![AliasAction {
                action: "add".to_string(),
                index: "test_index".to_string(),
                alias: "test_alias".to_string(),
                filter: None,
                routing: None,
                search_routing: None,
                index_routing: None,
                is_write_index: None,
            }],
        };
        
        let create_req = Request::builder()
            .uri("/_aliases")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&create_request).unwrap()))
            .unwrap();
        let _create_response = app.clone().oneshot(create_req).await.unwrap();

        // Now test removing the alias
        let request = Request::builder()
            .uri("/test_index/_alias/test_alias")
            .method("DELETE")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_perform_alias_operations_invalid_json() {
        let app = create_test_app();
        let request = Request::builder()
            .uri("/_aliases")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from("invalid json"))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_perform_alias_operations_empty_actions() {
        let app = create_test_app();
        let request_body = AliasOperationsRequest {
            actions: vec![],
        };
        let request = Request::builder()
            .uri("/_aliases")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_perform_alias_operations_invalid_action() {
        let app = create_test_app();
        let request_body = AliasOperationsRequest {
            actions: vec![AliasAction {
                action: "invalid_action".to_string(),
                index: "test_index".to_string(),
                alias: "test_alias".to_string(),
                filter: None,
                routing: None,
                search_routing: None,
                index_routing: None,
                is_write_index: None,
            }],
        };
        let request = Request::builder()
            .uri("/_aliases")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_perform_atomic_alias_operations_invalid_json() {
        let app = create_test_app();
        let request = Request::builder()
            .uri("/_aliases/atomic")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from("invalid json"))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_perform_atomic_alias_operations_empty_actions() {
        let app = create_test_app();
        let request_body = AliasOperationsRequest {
            actions: vec![],
        };
        let request = Request::builder()
            .uri("/_aliases/atomic")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_perform_atomic_alias_operations_remove_action() {
        let app = create_test_app();
        
        // First create an alias
        let create_request = AliasOperationsRequest {
            actions: vec![AliasAction {
                action: "add".to_string(),
                index: "test_index1".to_string(),
                alias: "test_alias".to_string(),
                filter: None,
                routing: None,
                search_routing: None,
                index_routing: None,
                is_write_index: None,
            }],
        };
        
        let create_req = Request::builder()
            .uri("/_aliases/atomic")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&create_request).unwrap()))
            .unwrap();
        let _create_response = app.clone().oneshot(create_req).await.unwrap();

        // Now test removing indices from the alias
        let remove_request = AliasOperationsRequest {
            actions: vec![AliasAction {
                action: "remove".to_string(),
                index: "test_index1".to_string(),
                alias: "test_alias".to_string(),
                filter: None,
                routing: None,
                search_routing: None,
                index_routing: None,
                is_write_index: None,
            }],
        };
        
        let request = Request::builder()
            .uri("/_aliases/atomic")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&remove_request).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_perform_atomic_alias_operations_remove_index_action() {
        let app = create_test_app();
        
        // First create an alias
        let create_request = AliasOperationsRequest {
            actions: vec![AliasAction {
                action: "add".to_string(),
                index: "test_index1".to_string(),
                alias: "test_alias".to_string(),
                filter: None,
                routing: None,
                search_routing: None,
                index_routing: None,
                is_write_index: None,
            }],
        };
        
        let create_req = Request::builder()
            .uri("/_aliases/atomic")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&create_request).unwrap()))
            .unwrap();
        let _create_response = app.clone().oneshot(create_req).await.unwrap();

        // Now test removing the entire alias
        let remove_request = AliasOperationsRequest {
            actions: vec![AliasAction {
                action: "remove_index".to_string(),
                index: "".to_string(), // Not used for remove_index
                alias: "test_alias".to_string(),
                filter: None,
                routing: None,
                search_routing: None,
                index_routing: None,
                is_write_index: None,
            }],
        };
        
        let request = Request::builder()
            .uri("/_aliases/atomic")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&remove_request).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_alias_operations_with_complex_config() {
        let app = create_test_app();
        let request_body = AliasOperationsRequest {
            actions: vec![AliasAction {
                action: "add".to_string(),
                index: "test_index".to_string(),
                alias: "complex_alias".to_string(),
                filter: Some(serde_json::json!({
                    "bool": {
                        "must": [
                            {"term": {"status": "active"}},
                            {"range": {"created_at": {"gte": "2023-01-01"}}}
                        ]
                    }
                })),
                routing: Some("user123".to_string()),
                search_routing: Some("user123".to_string()),
                index_routing: Some("user123".to_string()),
                is_write_index: Some(true),
            }],
        };
        
        let request = Request::builder()
            .uri("/_aliases/atomic")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        // Should succeed even though index doesn't exist (alias operations are independent)
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_alias_operations_missing_content_type() {
        let app = create_test_app();
        let request_body = AliasOperationsRequest {
            actions: vec![AliasAction {
                action: "add".to_string(),
                index: "test_index".to_string(),
                alias: "test_alias".to_string(),
                filter: None,
                routing: None,
                search_routing: None,
                index_routing: None,
                is_write_index: None,
            }],
        };
        
        let request = Request::builder()
            .uri("/_aliases")
            .method("POST")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        // Should still work without content-type header
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_alias_operations_large_request() {
        let app = create_test_app();
        
        // Create a large request with many operations
        let mut actions = Vec::new();
        for i in 0..100 {
            actions.push(AliasAction {
                action: "add".to_string(),
                index: format!("index_{}", i),
                alias: format!("alias_{}", i),
                filter: None,
                routing: None,
                search_routing: None,
                index_routing: None,
                is_write_index: None,
            });
        }
        
        let request_body = AliasOperationsRequest { actions };
        let request = Request::builder()
            .uri("/_aliases/atomic")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_alias_operations_concurrent_requests() {
        use std::sync::Arc;
        use tokio::task;
        
        let app = Arc::new(create_test_app());
        let mut handles = vec![];
        
        // Spawn multiple concurrent requests
        for i in 0..10 {
            let app_clone = app.clone();
            let handle = task::spawn(async move {
                let request_body = AliasOperationsRequest {
                    actions: vec![AliasAction {
                        action: "add".to_string(),
                        index: format!("index_{}", i),
                        alias: format!("alias_{}", i),
                        filter: None,
                        routing: None,
                        search_routing: None,
                        index_routing: None,
                        is_write_index: None,
                    }],
                };
                
                let request = Request::builder()
                    .uri("/_aliases/atomic")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap();
                <Router as Clone>::clone(&app_clone).oneshot(request).await.unwrap()
            });
            handles.push(handle);
        }
        
        // Wait for all requests to complete
        for handle in handles {
            let response = handle.await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}
