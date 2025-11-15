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
    let target_indices = state
        .index_manager
        .resolve_name(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    // Get the first index (for now, we only support single index suggestions)
    let index = state
        .index_manager
        .get_index(target_indices[0].as_str())
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

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
    Json(params): Json<SuggestParams>,
) -> ApiResult<Json<SuggestResponse>> {
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
