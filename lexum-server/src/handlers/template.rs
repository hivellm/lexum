//! Template management handlers

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::Json;
use axum::extract::{Path, State};
use lexum_core::index::template::{IndexTemplate, TemplateName};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Create or update template request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PutTemplateRequest {
    /// Index patterns this template applies to
    pub index_patterns: Vec<String>,
    /// Template priority (higher number = higher priority)
    #[serde(default = "default_priority")]
    pub priority: i32,
    /// Template version
    #[serde(default = "default_version")]
    pub version: u32,
    /// Index settings to apply
    pub settings: TemplateSettingsRequest,
    /// Schema fields to apply
    pub mappings: TemplateMappingsRequest,
    /// Template order (for ordering when priority is equal)
    #[serde(default = "default_order")]
    pub order: i32,
}

fn default_priority() -> i32 {
    0
}

fn default_version() -> u32 {
    1
}

fn default_order() -> i32 {
    0
}

/// Template settings request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TemplateSettingsRequest {
    /// Number of shards
    #[serde(default = "default_shards")]
    pub number_of_shards: usize,
    /// Number of replicas
    #[serde(default = "default_replicas")]
    pub number_of_replicas: usize,
    /// Refresh interval in seconds
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,
    /// Additional custom settings
    #[serde(default)]
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

fn default_shards() -> usize {
    1
}

fn default_replicas() -> usize {
    0
}

fn default_refresh_interval() -> u64 {
    1
}

/// Template mappings request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct TemplateMappingsRequest {
    /// Field mappings
    #[serde(default)]
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

/// Template response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TemplateResponse {
    /// Template name
    pub name: String,
    /// Whether the operation was acknowledged
    pub acknowledged: bool,
}

/// Create or update template handler
#[utoipa::path(
    put,
    path = "/_template/{name}",
    params(
        ("name" = String, Path, description = "Template name")
    ),
    request_body = PutTemplateRequest,
    responses(
        (status = 200, description = "Template created or updated successfully", body = TemplateResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Templates"
)]
pub async fn put_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
    request: Result<Json<PutTemplateRequest>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<Json<TemplateResponse>> {
    // Convert JsonRejection to ApiError if JSON parsing failed
    let Json(request) = request.map_err(ApiError::from)?;
    // Convert request to IndexTemplate
    let template = IndexTemplate {
        name: TemplateName::from(name.clone()),
        index_patterns: request
            .index_patterns
            .into_iter()
            .map(lexum_core::index::template::IndexPattern::from)
            .collect(),
        priority: request.priority,
        version: request.version,
        settings: lexum_core::index::template::TemplateSettings {
            number_of_shards: request.settings.number_of_shards,
            number_of_replicas: request.settings.number_of_replicas,
            refresh_interval: request.settings.refresh_interval,
            custom: request.settings.custom,
        },
        mappings: lexum_core::index::template::TemplateMappings {
            properties: request.mappings.properties,
        },
        order: request.order,
    };

    // Validate template before storing
    template
        .validate()
        .map_err(|e| ApiError::InvalidRequest(format!("Template validation failed: {e}")))?;

    // Store the template
    state
        .template_manager
        .put_template(template)
        .map_err(|e| ApiError::Internal(format!("Failed to store template: {e}")))?;

    Ok(Json(TemplateResponse {
        name,
        acknowledged: true,
    }))
}

/// Get template handler
#[utoipa::path(
    get,
    path = "/_template/{name}",
    params(
        ("name" = String, Path, description = "Template name")
    ),
    responses(
        (status = 200, description = "Template retrieved successfully", body = IndexTemplate),
        (status = 404, description = "Template not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Templates"
)]
pub async fn get_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<Json<IndexTemplate>> {
    let template = state
        .template_manager
        .get_template(&name)?
        .ok_or(ApiError::TemplateNotFound(name))?;

    Ok(Json(template))
}

/// Delete template handler
#[utoipa::path(
    delete,
    path = "/_template/{name}",
    params(
        ("name" = String, Path, description = "Template name")
    ),
    responses(
        (status = 200, description = "Template deleted successfully", body = TemplateResponse),
        (status = 404, description = "Template not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Templates"
)]
pub async fn delete_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<Json<TemplateResponse>> {
    let deleted = state.template_manager.delete_template(&name)?;

    if !deleted {
        return Err(ApiError::TemplateNotFound(name));
    }

    Ok(Json(TemplateResponse {
        name,
        acknowledged: true,
    }))
}

/// List templates handler
#[utoipa::path(
    get,
    path = "/_template",
    responses(
        (status = 200, description = "Templates retrieved successfully", body = ListTemplatesResponse),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Templates"
)]
pub async fn list_templates(
    State(state): State<AppState>,
) -> ApiResult<Json<ListTemplatesResponse>> {
    let templates = state.template_manager.list_templates();

    Ok(Json(ListTemplatesResponse { templates }))
}

/// List templates response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListTemplatesResponse {
    /// List of templates
    pub templates: Vec<IndexTemplate>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::index::AppState;
    use lexum_core::TemplateManager;
    use std::sync::Arc;

    fn create_test_app_state() -> AppState {
        AppState {
            index_manager: Arc::new(lexum_core::IndexManager::new(std::env::temp_dir())),
            snapshot_manager: Arc::new(tokio::sync::RwLock::new(
                lexum_core::SnapshotManager::new(&lexum_core::config::Config::default()).unwrap(),
            )),
            template_manager: Arc::new(TemplateManager::new()),
            task_manager: Arc::new(crate::handlers::reindex::TaskManager::new()),
            progress_tracker: Arc::new(lexum_core::ProgressTracker::new()),
            auth_state: crate::middleware::auth::AuthState::new(
                crate::middleware::auth::AuthConfig::default(),
            ),
            query_complexity_config:
                crate::middleware::query_complexity::QueryComplexityLimitConfig::default(),
            metrics: Arc::new(crate::handlers::metrics::PrometheusMetrics::new()),
        }
    }

    #[lexum_macros::tokio_test]
    async fn test_put_template_success() {
        let state = create_test_app_state();
        let request = PutTemplateRequest {
            index_patterns: vec!["test-*".to_string()],
            priority: 1,
            version: 1,
            settings: TemplateSettingsRequest {
                number_of_shards: 1,
                number_of_replicas: 0,
                refresh_interval: 1,
                custom: std::collections::HashMap::new(),
            },
            mappings: TemplateMappingsRequest::default(),
            order: 0,
        };

        let result = put_template(
            State(state),
            Path("test-template".to_string()),
            Json(request),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.0.name, "test-template");
        assert!(response.0.acknowledged);
    }

    #[lexum_macros::tokio_test]
    async fn test_put_template_invalid_patterns() {
        let state = create_test_app_state();
        let request = PutTemplateRequest {
            index_patterns: vec![], // Empty patterns should fail validation
            priority: 1,
            version: 1,
            settings: TemplateSettingsRequest {
                number_of_shards: 1,
                number_of_replicas: 0,
                refresh_interval: 1,
                custom: std::collections::HashMap::new(),
            },
            mappings: TemplateMappingsRequest::default(),
            order: 0,
        };

        let result = put_template(
            State(state),
            Path("test-template".to_string()),
            Json(request),
        )
        .await;

        assert!(result.is_err());
    }
}
