//! Search suggestion handler

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::Json;
use axum::extract::{Path, State};
use lexum_core::search::{Suggester, SuggesterConfig, Suggestion, SuggestionType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

/// Suggest request parameters
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SuggestParams {
    /// Query text to get suggestions for
    pub q: String,
    /// Fields to search for suggestions (default: all text fields)
    #[serde(default)]
    pub fields: Option<Vec<String>>,
    /// Maximum number of suggestions (default: 10)
    #[serde(default = "default_max_suggestions")]
    pub size: usize,
    /// Minimum prefix length (default: 2)
    #[serde(default = "default_min_prefix")]
    pub min_prefix_length: usize,
    /// Fuzziness level for fuzzy suggestions (0-2, default: 1)
    #[serde(default = "default_fuzziness")]
    pub fuzziness: u8,
    /// Whether to include phrase suggestions (default: true)
    #[serde(default = "default_include_phrases")]
    pub include_phrases: bool,
    /// Maximum phrase length (default: 5)
    #[serde(default = "default_max_phrase_length")]
    pub max_phrase_length: usize,
}

fn default_max_suggestions() -> usize {
    10
}

fn default_min_prefix() -> usize {
    2
}

fn default_fuzziness() -> u8 {
    1
}

fn default_include_phrases() -> bool {
    true
}

fn default_max_phrase_length() -> usize {
    5
}

/// Suggest response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SuggestResponse {
    /// List of suggestions
    pub suggestions: Vec<SuggestionResponseItem>,
}

/// Individual suggestion item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SuggestionResponseItem {
    /// Suggested text
    pub text: String,
    /// Relevance score
    pub score: f32,
    /// Type of suggestion
    #[serde(rename = "type")]
    pub suggestion_type: String,
}

impl From<Suggestion> for SuggestionResponseItem {
    fn from(suggestion: Suggestion) -> Self {
        let suggestion_type_str = match suggestion.suggestion_type {
            SuggestionType::Completion => "completion",
            SuggestionType::Fuzzy => "fuzzy",
            SuggestionType::Phrase => "phrase",
        };

        Self {
            text: suggestion.text,
            score: suggestion.score,
            suggestion_type: suggestion_type_str.to_string(),
        }
    }
}

/// Suggest handler
#[utoipa::path(
    get,
    path = "/api/v1/indices/{index_name}/_suggest",
    params(
        ("index_name" = String, Path, description = "Index name"),
        ("q" = String, Query, description = "Query text"),
        ("fields" = Option<Vec<String>>, Query, description = "Fields to search"),
        ("size" = Option<usize>, Query, description = "Maximum number of suggestions"),
        ("min_prefix_length" = Option<usize>, Query, description = "Minimum prefix length"),
        ("fuzziness" = Option<u8>, Query, description = "Fuzziness level (0-2)"),
        ("include_phrases" = Option<bool>, Query, description = "Include phrase suggestions"),
        ("max_phrase_length" = Option<usize>, Query, description = "Maximum phrase length")
    ),
    responses(
        (status = 200, description = "Suggestions generated successfully", body = SuggestResponse),
        (status = 404, description = "Index not found"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Search"
)]
pub async fn suggest(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
    axum::extract::Query(params): axum::extract::Query<SuggestParams>,
) -> ApiResult<Json<SuggestResponse>> {
    // Resolve alias to actual index names
    let target_indices = state.index_manager.resolve_name(&index_name).map_err(|e| {
        // Convert Validation error for "not found" to IndexNotFound
        if let lexum_core::Error::Validation(ref msg) = e {
            if msg.contains("not found") || msg.contains("does not exist") {
                return ApiError::IndexNotFound(index_name.clone());
            }
        }
        let error_msg = e.to_string();
        if error_msg.contains("not found") || error_msg.contains("does not exist") {
            ApiError::IndexNotFound(index_name.clone())
        } else {
            tracing::error!("Failed to resolve name '{}': {}", index_name, error_msg);
            ApiError::Core(e)
        }
    })?;

    // Get the first index (for now, we only support single index suggestions)
    let index = state
        .index_manager
        .get_index(target_indices[0].as_str())
        .map_err(|e| {
            // Convert Validation error for "not found" to IndexNotFound
            if let lexum_core::Error::Validation(ref msg) = e {
                if msg.contains("not found") || msg.contains("does not exist") {
                    return ApiError::IndexNotFound(index_name.clone());
                }
            }
            let error_msg = e.to_string();
            if error_msg.contains("not found") || error_msg.contains("does not exist") {
                ApiError::IndexNotFound(index_name.clone())
            } else {
                tracing::error!("Failed to get index '{}': {}", target_indices[0], error_msg);
                ApiError::Core(e)
            }
        })?;

    let index_arc = Arc::new(index);

    // Determine fields to search
    let fields = if let Some(ref requested_fields) = params.fields {
        requested_fields.clone()
    } else {
        // Use all text fields if not specified
        index_arc.get_text_field_names()
    };

    if fields.is_empty() {
        return Err(ApiError::InvalidRequest(
            "No searchable text fields found in index".to_string(),
        ));
    }

    // Create suggester configuration
    let suggester_config = SuggesterConfig::new()
        .with_max_suggestions(params.size)
        .with_min_prefix_length(params.min_prefix_length)
        .with_fuzziness(params.fuzziness)
        .with_include_phrases(params.include_phrases)
        .with_max_phrase_length(params.max_phrase_length);

    // Create suggester
    let suggester = Suggester::with_config(index_arc, suggester_config);

    // Generate suggestions
    let suggestions = suggester
        .suggest(&params.q, &fields)
        .map_err(|e| ApiError::InvalidRequest(format!("Failed to generate suggestions: {e}")))?;

    // Convert to response format
    let response_items: Vec<SuggestionResponseItem> = suggestions
        .into_iter()
        .map(SuggestionResponseItem::from)
        .collect();

    Ok(Json(SuggestResponse {
        suggestions: response_items,
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
    use axum::extract::{Path, Query, State};
    use lexum_core::ProgressTracker;
    use lexum_core::{IndexManager, SnapshotManager, TemplateManager};
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::RwLock;

    fn create_test_app_state() -> (TempDir, AppState) {
        let temp_dir = TempDir::new().unwrap();
        let index_manager = Arc::new(IndexManager::new(temp_dir.path()));
        let config = lexum_core::config::Config::default();
        let snapshot_manager = Arc::new(RwLock::new(SnapshotManager::new(&config).unwrap_or_else(
            |_| {
                let mut fallback_config = config;
                fallback_config.snapshots.repositories =
                    vec![lexum_core::config::SnapshotRepositoryConfig {
                        name: "default".to_string(),
                        repository_type: "fs".to_string(),
                        settings: lexum_core::config::SnapshotRepositorySettings {
                            location: temp_dir
                                .path()
                                .join("snapshots")
                                .to_string_lossy()
                                .to_string(),
                            ..Default::default()
                        },
                    }];
                SnapshotManager::new(&fallback_config).unwrap()
            },
        )));

        let state = AppState {
            index_manager,
            snapshot_manager,
            template_manager: Arc::new(TemplateManager::new()),
            task_manager: Arc::new(TaskManager::new()),
            progress_tracker: Arc::new(ProgressTracker::new()),
            auth_state: AuthState::new(crate::middleware::auth::AuthConfig::default()),
            query_complexity_config: QueryComplexityLimitConfig::default(),
            metrics: Arc::new(PrometheusMetrics::new()),
        };

        (temp_dir, state)
    }

    #[lexum_macros::tokio_test]
    async fn test_suggest_index_not_found() {
        let (_temp_dir, state) = create_test_app_state();

        // Test suggest with non-existent index
        let params = SuggestParams {
            q: "test".to_string(),
            fields: None,
            size: 10,
            min_prefix_length: 2,
            fuzziness: 1,
            include_phrases: true,
            max_phrase_length: 5,
        };

        let result = suggest(
            State(state),
            Path("non-existent-index".to_string()),
            Query(params),
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ApiError::IndexNotFound(_) => {
                // Expected - index doesn't exist
            }
            e => panic!("Expected IndexNotFound error, got: {e:?}"),
        }
    }

    #[test]
    fn test_suggest_params_defaults() {
        let params = SuggestParams {
            q: "test".to_string(),
            fields: None,
            size: default_max_suggestions(),
            min_prefix_length: default_min_prefix(),
            fuzziness: default_fuzziness(),
            include_phrases: default_include_phrases(),
            max_phrase_length: default_max_phrase_length(),
        };

        assert_eq!(params.q, "test");
        assert!(params.fields.is_none());
        assert_eq!(params.size, 10);
        assert_eq!(params.min_prefix_length, 2);
        assert_eq!(params.fuzziness, 1);
        assert_eq!(params.include_phrases, true);
        assert_eq!(params.max_phrase_length, 5);
    }

    #[test]
    fn test_suggest_params_with_all_fields() {
        let params = SuggestParams {
            q: "search term".to_string(),
            fields: Some(vec!["title".to_string(), "content".to_string()]),
            size: 20,
            min_prefix_length: 3,
            fuzziness: 2,
            include_phrases: false,
            max_phrase_length: 10,
        };

        assert_eq!(params.q, "search term");
        assert_eq!(params.fields.as_ref().unwrap().len(), 2);
        assert_eq!(params.size, 20);
        assert_eq!(params.min_prefix_length, 3);
        assert_eq!(params.fuzziness, 2);
        assert_eq!(params.include_phrases, false);
        assert_eq!(params.max_phrase_length, 10);
    }
}

/// POST suggest handler (for more complex requests)
#[utoipa::path(
    post,
    path = "/api/v1/indices/{index_name}/_suggest",
    params(
        ("index_name" = String, Path, description = "Index name")
    ),
    request_body = SuggestParams,
    responses(
        (status = 200, description = "Suggestions generated successfully", body = SuggestResponse),
        (status = 404, description = "Index not found"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Search"
)]
pub async fn suggest_post(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
    params: Result<Json<SuggestParams>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<Json<SuggestResponse>> {
    // Convert JsonRejection to ApiError if JSON parsing failed
    let Json(params) = params.map_err(ApiError::from)?;
    // Resolve alias to actual index names
    let target_indices = state
        .index_manager
        .resolve_name(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    // Get the first index
    let index = state
        .index_manager
        .get_index(target_indices[0].as_str())
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    let index_arc = Arc::new(index);

    // Determine fields to search
    let fields = if let Some(ref requested_fields) = params.fields {
        requested_fields.clone()
    } else {
        index_arc.get_text_field_names()
    };

    if fields.is_empty() {
        return Err(ApiError::InvalidRequest(
            "No searchable text fields found in index".to_string(),
        ));
    }

    // Create suggester configuration
    let suggester_config = SuggesterConfig::new()
        .with_max_suggestions(params.size)
        .with_min_prefix_length(params.min_prefix_length)
        .with_fuzziness(params.fuzziness)
        .with_include_phrases(params.include_phrases)
        .with_max_phrase_length(params.max_phrase_length);

    // Create suggester
    let suggester = Suggester::with_config(index_arc, suggester_config);

    // Generate suggestions
    let suggestions = suggester
        .suggest(&params.q, &fields)
        .map_err(|e| ApiError::InvalidRequest(format!("Failed to generate suggestions: {e}")))?;

    // Convert to response format
    let response_items: Vec<SuggestionResponseItem> = suggestions
        .into_iter()
        .map(SuggestionResponseItem::from)
        .collect();

    Ok(Json(SuggestResponse {
        suggestions: response_items,
    }))
}
