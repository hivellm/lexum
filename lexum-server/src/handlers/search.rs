//! Search handler

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use crate::middleware::query_complexity::QueryComplexityLimitLayer;
use axum::Json;
use axum::extract::{Path, State};
use lexum_core::search::{Highlighter, HighlighterConfig};
use lexum_core::{Query, SearchExecutor, SearchResult, SortOption};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;

/// Search request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchRequest {
    /// Query
    pub query: Query,
    /// Filter queries (must match but don't affect score)
    #[serde(default)]
    pub filter: Option<Vec<Query>>,
    /// Limit (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Offset (default: 0)
    #[serde(default)]
    pub offset: usize,
    /// Optional sort specification
    #[serde(default)]
    pub sort: Option<SortOption>,
    /// Fields to return in results (source filtering)
    #[serde(default)]
    pub fields: Option<Vec<String>>,
    /// Highlight search terms in results
    #[serde(default)]
    pub highlight: Option<HighlightConfig>,
    /// Explain query execution
    #[serde(default)]
    pub explain: bool,
    /// Minimum score threshold
    #[serde(default)]
    pub min_score: Option<f32>,
    /// Query string for simple text search
    #[serde(default)]
    pub q: Option<String>,
}

fn default_limit() -> usize {
    10
}

/// Highlight configuration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HighlightConfig {
    /// Fields to highlight
    pub fields: Vec<String>,
    /// Pre-tag for highlighting
    #[serde(default = "default_pre_tag")]
    pub pre_tag: String,
    /// Post-tag for highlighting
    #[serde(default = "default_post_tag")]
    pub post_tag: String,
    /// Fragment size in characters (default: 100)
    #[serde(default = "default_fragment_size")]
    pub fragment_size: usize,
    /// Maximum number of fragments per field (default: 3)
    #[serde(default = "default_max_fragments")]
    pub max_fragments: usize,
}

fn default_fragment_size() -> usize {
    100
}

fn default_max_fragments() -> usize {
    3
}

fn default_pre_tag() -> String {
    "<em>".to_string()
}

fn default_post_tag() -> String {
    "</em>".to_string()
}

/// Extract query terms from a query for highlighting
fn extract_query_terms(query: &Query, query_string: Option<&str>) -> HashSet<String> {
    let mut terms = HashSet::new();

    // If query string is provided, use it
    if let Some(q) = query_string {
        for word in q.split_whitespace() {
            terms.insert(word.to_lowercase());
        }
        return terms;
    }

    // Extract terms from query structure
    match query {
        Query::Match(m) => {
            for word in m.query.split_whitespace() {
                terms.insert(word.to_lowercase());
            }
        }
        Query::Term(t) => {
            terms.insert(t.value.to_lowercase());
        }
        Query::Fuzzy(f) => {
            terms.insert(f.value.to_lowercase());
        }
        Query::Phrase(p) => {
            for word in p.phrase.split_whitespace() {
                terms.insert(word.to_lowercase());
            }
        }
        Query::Wildcard(w) => {
            // Extract base pattern (remove wildcards)
            let pattern = w.pattern.replace('*', "").replace('?', "");
            if !pattern.is_empty() {
                terms.insert(pattern.to_lowercase());
            }
        }
        Query::Bool(b) => {
            // Extract terms from all clauses
            for must in &b.must {
                terms.extend(extract_query_terms(must, None));
            }
            for should in &b.should {
                terms.extend(extract_query_terms(should, None));
            }
        }
        _ => {
            // For other query types, no terms extracted
        }
    }

    terms
}

/// Search handler
#[utoipa::path(
    post,
    path = "/api/v1/indices/{index_name}/search",
    params(
        ("index_name" = String, Path, description = "Index name")
    ),
    request_body = SearchRequest,
    responses(
        (status = 200, description = "Search completed successfully", body = SearchResult),
        (status = 404, description = "Index not found"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Search"
)]
pub async fn search(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
    Json(request): Json<SearchRequest>,
) -> ApiResult<Json<SearchResult>> {
    // Resolve alias to actual index names
    let target_indices = state
        .index_manager
        .resolve_name(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    // Handle simple query string if provided
    let query = if let Some(ref q) = request.q {
        // Get index to find text fields
        let index = state
            .index_manager
            .get_index(target_indices[0].as_str())
            .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

        let text_fields = index.get_text_field_names();

        if text_fields.is_empty() {
            // No text fields found, use MatchAll
            lexum_core::Query::MatchAll
        } else if text_fields.len() == 1 {
            // Single text field, use simple match
            lexum_core::Query::Match(lexum_core::MatchQuery::new(&text_fields[0], q.clone()))
        } else {
            // Multiple text fields, create bool query with should clauses
            let mut bool_query = lexum_core::BoolQuery::new();
            for field in text_fields {
                bool_query = bool_query.should(lexum_core::Query::Match(
                    lexum_core::MatchQuery::new(&field, q.clone()),
                ));
            }
            lexum_core::Query::Bool(bool_query)
        }
    } else {
        request.query
    };

    // Validate query complexity
    if state.query_complexity_config.enabled {
        let query_json: Value = serde_json::to_value(&query)
            .map_err(|e| ApiError::InvalidRequest(format!("Failed to serialize query: {e}")))?;
        let complexity_layer =
            QueryComplexityLimitLayer::new(state.query_complexity_config.clone());
        if let Err(e) = complexity_layer.analyze_query_json(&query_json) {
            return Err(ApiError::InvalidRequest(format!(
                "Query complexity limit exceeded: {}",
                e.message()
            )));
        }

        // Validate filters if present
        if let Some(ref filters) = request.filter {
            for filter in filters {
                let filter_json: Value = serde_json::to_value(filter).map_err(|e| {
                    ApiError::InvalidRequest(format!("Failed to serialize filter: {e}"))
                })?;
                if let Err(e) = complexity_layer.analyze_query_json(&filter_json) {
                    return Err(ApiError::InvalidRequest(format!(
                        "Filter complexity limit exceeded: {}",
                        e.message()
                    )));
                }
            }
        }
    }

    // Apply filters if provided (wrap query in bool query with filter clause)
    let final_query = if let Some(ref filters) = request.filter {
        if !filters.is_empty() {
            // Wrap the main query in a bool query with filter clauses
            let mut bool_query = lexum_core::BoolQuery::new();
            bool_query = bool_query.must(query.clone());
            for filter in filters {
                bool_query = bool_query.filter(filter.clone());
            }
            lexum_core::Query::Bool(bool_query)
        } else {
            query
        }
    } else {
        query
    };

    // Clone final_query for highlighting (before it's moved)
    let final_query_for_highlighting = final_query.clone();

    // Use single index search for now (multi-index search not implemented yet)
    let mut result = if target_indices.len() > 1 {
        // For now, just search the first index
        let index = state
            .index_manager
            .get_index(target_indices[0].as_str())
            .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

        let executor = SearchExecutor::new(Arc::new(index));
        executor
            .search(
                final_query.clone(),
                request.limit,
                request.offset,
                request.sort,
            )
            .await?
    } else {
        // Single index search
        let index = state
            .index_manager
            .get_index(target_indices[0].as_str())
            .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

        let executor = SearchExecutor::new(Arc::new(index));
        executor
            .search(final_query, request.limit, request.offset, request.sort)
            .await?
    };

    // Apply minimum score filtering
    if let Some(min_score) = request.min_score {
        result.hits.retain(|hit| hit.score.value() >= min_score);
    }

    // Apply field filtering (source filtering)
    if let Some(fields) = request.fields {
        for hit in &mut result.hits {
            if let serde_json::Value::Object(ref mut source) = hit.source {
                let filtered: serde_json::Map<String, serde_json::Value> = source
                    .iter()
                    .filter(|(key, _)| fields.contains(key))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                hit.source = serde_json::Value::Object(filtered);
            }
        }
    }

    // Apply highlighting if requested
    if let Some(highlight) = request.highlight {
        // Extract query terms for highlighting
        let query_terms = extract_query_terms(&final_query_for_highlighting, request.q.as_deref());

        // Create highlighter with config
        let highlighter_config = HighlighterConfig::new()
            .with_pre_tag(highlight.pre_tag.clone())
            .with_post_tag(highlight.post_tag.clone())
            .with_fragment_size(highlight.fragment_size)
            .with_max_fragments(highlight.max_fragments);
        let highlighter = Highlighter::with_config(highlighter_config);

        for hit in &mut result.hits {
            if let serde_json::Value::Object(ref mut source) = hit.source {
                let mut highlighted_fields = std::collections::HashMap::new();

                for field in &highlight.fields {
                    if field == "_all" {
                        // Highlight all text fields
                        let keys: Vec<String> = source.keys().cloned().collect();
                        for key in keys {
                            if let Some(serde_json::Value::String(text)) = source.get(&key) {
                                let fragments = highlighter.highlight(text, &query_terms);
                                if !fragments.is_empty() {
                                    if fragments.len() == 1 {
                                        highlighted_fields.insert(
                                            format!("{key}_highlighted"),
                                            serde_json::Value::String(fragments[0].clone()),
                                        );
                                    } else {
                                        highlighted_fields.insert(
                                            format!("{key}_highlighted"),
                                            serde_json::Value::Array(
                                                fragments
                                                    .iter()
                                                    .map(|f| serde_json::Value::String(f.clone()))
                                                    .collect(),
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    } else if let Some(serde_json::Value::String(text)) = source.get(field) {
                        let fragments = highlighter.highlight(text, &query_terms);
                        if !fragments.is_empty() {
                            if fragments.len() == 1 {
                                highlighted_fields.insert(
                                    format!("{field}_highlighted"),
                                    serde_json::Value::String(fragments[0].clone()),
                                );
                            } else {
                                highlighted_fields.insert(
                                    format!("{field}_highlighted"),
                                    serde_json::Value::Array(
                                        fragments
                                            .iter()
                                            .map(|f| serde_json::Value::String(f.clone()))
                                            .collect(),
                                    ),
                                );
                            }
                        }
                    }
                }

                // Add highlighted fields to source
                for (key, value) in highlighted_fields {
                    source.insert(key, value);
                }
            }
        }
    }

    // Add explain information if requested
    if request.explain {
        // In a real implementation, this would include query execution details
        result.hits.iter_mut().for_each(|hit| {
            if let serde_json::Value::Object(ref mut source) = hit.source {
                source.insert(
                    "_explanation".to_string(),
                    serde_json::json!({
                        "value": hit.score.value(),
                        "description": "score computed from query"
                    }),
                );
            }
        });
    }

    Ok(Json(result))
}

/// Explain why a document matches or doesn't match a query
#[utoipa::path(
    get,
    path = "/api/v1/indices/{index_name}/_explain/{id}",
    params(
        ("index_name" = String, Path, description = "Index name"),
        ("id" = String, Path, description = "Document ID"),
        ("q" = Option<String>, Query, description = "Query string"),
    ),
    responses(
        (status = 200, description = "Explanation generated", body = ExplainResult),
        (status = 404, description = "Index or document not found"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Search"
)]
pub async fn explain(
    State(state): State<AppState>,
    Path((index_name, doc_id)): Path<(String, String)>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<ExplainResult>> {
    // Resolve alias to actual index names
    let target_indices = state
        .index_manager
        .resolve_name(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    if target_indices.is_empty() {
        return Err(ApiError::IndexNotFound(index_name));
    }

    // Get the first index (for now, single index only)
    let index = state
        .index_manager
        .get_index(target_indices[0].as_str())
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    // Parse query from query parameter
    let query_str = params
        .get("q")
        .ok_or_else(|| ApiError::InvalidRequest("Query parameter 'q' is required".to_string()))?;

    // Build query from query string
    // Get text field names from index
    let text_fields = index.get_text_field_names();

    if text_fields.is_empty() {
        return Err(ApiError::InvalidRequest(
            "No text fields found in index".to_string(),
        ));
    }

    // Use first text field for simple query string
    let query = lexum_core::Query::Match(lexum_core::MatchQuery::new(
        text_fields[0].clone(),
        query_str.clone(),
    ));

    // Create search executor
    let executor = SearchExecutor::new(Arc::new(index));

    // Execute search with explain enabled
    let search_result = executor
        .search(query.clone(), 100, 0, None)
        .await
        .map_err(|e| ApiError::InvalidRequest(format!("Search failed: {e}")))?;

    // Find the document in results
    let hit = search_result
        .hits
        .iter()
        .find(|h| h.id.as_str() == doc_id.as_str());

    let matched = hit.is_some();
    let score = hit.map(|h| h.score.value()).unwrap_or(0.0);

    // Build explanation
    let explanation = ExplainExplanation {
        value: score,
        description: if matched {
            format!("Document matches query with score {score}")
        } else {
            "Document does not match query".to_string()
        },
        details: vec![ExplainDetail {
            value: score,
            description: format!("Query: {}", query_str),
        }],
    };

    let result = ExplainResult {
        index: index_name,
        id: doc_id,
        matched,
        explanation,
    };

    Ok(Json(result))
}

/// Explain result
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExplainResult {
    /// Index name
    pub index: String,
    /// Document ID
    pub id: String,
    /// Whether document matches the query
    pub matched: bool,
    /// Explanation details
    pub explanation: ExplainExplanation,
}

/// Explanation details
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExplainExplanation {
    /// Score value
    pub value: f32,
    /// Description
    pub description: String,
    /// Additional details
    pub details: Vec<ExplainDetail>,
}

/// Explanation detail
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExplainDetail {
    /// Value
    pub value: f32,
    /// Description
    pub description: String,
}

/// Simple search handler with query parameters
#[utoipa::path(
    get,
    path = "/api/v1/indices/{index_name}/search",
    params(
        ("index_name" = String, Path, description = "Index name"),
        ("q" = Option<String>, Query, description = "Query string"),
        ("filter" = Option<String>, Query, description = "JSON array of filter queries (don't affect score)"),
        ("limit" = Option<usize>, Query, description = "Number of results"),
        ("offset" = Option<usize>, Query, description = "Result offset"),
        ("sort" = Option<String>, Query, description = "Sort field:order"),
        ("fields" = Option<String>, Query, description = "Comma-separated fields to return"),
        ("highlight" = Option<bool>, Query, description = "Enable highlighting"),
        ("explain" = Option<bool>, Query, description = "Include explanation"),
        ("min_score" = Option<f32>, Query, description = "Minimum score threshold")
    ),
    responses(
        (status = 200, description = "Search completed successfully", body = SearchResult),
        (status = 404, description = "Index not found"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Search"
)]
pub async fn search_get(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
    axum::extract::Query(params): axum::extract::Query<SearchParams>,
) -> ApiResult<Json<SearchResult>> {
    // Resolve alias to actual index names
    let target_indices = state
        .index_manager
        .resolve_name(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    // Build query from parameters
    let base_query = if let Some(ref q) = params.q {
        // Get index to find text fields
        let index = state
            .index_manager
            .get_index(target_indices[0].as_str())
            .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

        let text_fields = index.get_text_field_names();

        if text_fields.is_empty() {
            // No text fields found, use MatchAll
            lexum_core::Query::MatchAll
        } else if text_fields.len() == 1 {
            // Single text field, use simple match
            lexum_core::Query::Match(lexum_core::MatchQuery::new(&text_fields[0], q.clone()))
        } else {
            // Multiple text fields, create bool query with should clauses
            let mut bool_query = lexum_core::BoolQuery::new();
            for field in text_fields {
                bool_query = bool_query.should(lexum_core::Query::Match(
                    lexum_core::MatchQuery::new(&field, q.clone()),
                ));
            }
            lexum_core::Query::Bool(bool_query)
        }
    } else {
        lexum_core::Query::MatchAll
    };

    // Parse filters
    let filters = params
        .filter
        .and_then(|f| serde_json::from_str::<Vec<Query>>(&f).ok());

    // Apply filters if provided
    let query = if let Some(ref filters) = filters {
        if !filters.is_empty() {
            let mut bool_query = lexum_core::BoolQuery::new();
            bool_query = bool_query.must(base_query);
            for filter in filters {
                bool_query = bool_query.filter(filter.clone());
            }
            lexum_core::Query::Bool(bool_query)
        } else {
            base_query
        }
    } else {
        base_query
    };

    // Parse sort option
    let sort = params.sort.map(|s| {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 2 {
            let order = match parts[1] {
                "asc" => lexum_core::SortOrder::Asc,
                "desc" => lexum_core::SortOrder::Desc,
                _ => lexum_core::SortOrder::Desc,
            };
            lexum_core::SortOption::new(parts[0], order)
        } else {
            lexum_core::SortOption::new(s, lexum_core::SortOrder::Desc)
        }
    });

    // Parse fields
    let fields = params
        .fields
        .map(|f| f.split(',').map(|s| s.trim().to_string()).collect());

    // Build highlight config
    let highlight = if params.highlight.unwrap_or(false) {
        Some(HighlightConfig {
            fields: vec!["_all".to_string()],
            pre_tag: "<em>".to_string(),
            post_tag: "</em>".to_string(),
            fragment_size: 100,
            max_fragments: 3,
        })
    } else {
        None
    };

    let request = SearchRequest {
        query: query.clone(),
        filter: filters,
        limit: params.limit.unwrap_or(10),
        offset: params.offset.unwrap_or(0),
        sort,
        fields,
        highlight,
        explain: params.explain.unwrap_or(false),
        min_score: params.min_score,
        q: params.q,
    };

    // Use single index search for now (multi-index search not implemented yet)
    if target_indices.len() > 1 {
        // For now, just search the first index
        let index = state
            .index_manager
            .get_index(target_indices[0].as_str())
            .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

        let executor = SearchExecutor::new(Arc::new(index));
        let mut result = executor
            .search(query.clone(), request.limit, request.offset, request.sort)
            .await?;

        // Apply minimum score filtering
        if let Some(min_score) = request.min_score {
            result.hits.retain(|hit| hit.score.value() >= min_score);
        }

        // Apply field filtering (source filtering)
        if let Some(fields) = request.fields {
            for hit in &mut result.hits {
                if let serde_json::Value::Object(ref mut source) = hit.source {
                    let filtered: serde_json::Map<String, serde_json::Value> = source
                        .iter()
                        .filter(|(key, _)| fields.contains(key))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    hit.source = serde_json::Value::Object(filtered);
                }
            }
        }

        // Apply highlighting if requested
        if let Some(highlight) = request.highlight {
            // Extract query terms for highlighting
            let query_terms = extract_query_terms(&query, request.q.as_deref());

            // Create highlighter with config
            let highlighter_config = HighlighterConfig::new()
                .with_pre_tag(highlight.pre_tag.clone())
                .with_post_tag(highlight.post_tag.clone())
                .with_fragment_size(highlight.fragment_size)
                .with_max_fragments(highlight.max_fragments);
            let highlighter = Highlighter::with_config(highlighter_config);

            for hit in &mut result.hits {
                if let serde_json::Value::Object(ref mut source) = hit.source {
                    let mut highlighted_fields = std::collections::HashMap::new();

                    for field in &highlight.fields {
                        if field == "_all" {
                            // Highlight all text fields
                            let keys: Vec<String> = source.keys().cloned().collect();
                            for key in keys {
                                if let Some(serde_json::Value::String(text)) = source.get(&key) {
                                    let fragments = highlighter.highlight(text, &query_terms);
                                    if !fragments.is_empty() {
                                        if fragments.len() == 1 {
                                            highlighted_fields.insert(
                                                format!("{key}_highlighted"),
                                                serde_json::Value::String(fragments[0].clone()),
                                            );
                                        } else {
                                            highlighted_fields.insert(
                                                format!("{key}_highlighted"),
                                                serde_json::Value::Array(
                                                    fragments
                                                        .iter()
                                                        .map(|f| {
                                                            serde_json::Value::String(f.clone())
                                                        })
                                                        .collect(),
                                                ),
                                            );
                                        }
                                    }
                                }
                            }
                        } else if let Some(serde_json::Value::String(text)) = source.get(field) {
                            let fragments = highlighter.highlight(text, &query_terms);
                            if !fragments.is_empty() {
                                if fragments.len() == 1 {
                                    highlighted_fields.insert(
                                        format!("{field}_highlighted"),
                                        serde_json::Value::String(fragments[0].clone()),
                                    );
                                } else {
                                    highlighted_fields.insert(
                                        format!("{field}_highlighted"),
                                        serde_json::Value::Array(
                                            fragments
                                                .iter()
                                                .map(|f| serde_json::Value::String(f.clone()))
                                                .collect(),
                                        ),
                                    );
                                }
                            }
                        }
                    }

                    // Add highlighted fields to source
                    for (key, value) in highlighted_fields {
                        source.insert(key, value);
                    }
                }
            }
        }

        Ok(Json(result))
    } else {
        // Single index search - delegate to the main search function
        search(State(state), Path(index_name), Json(request)).await
    }
}

/// Search parameters for GET request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchParams {
    /// Query string
    pub q: Option<String>,
    /// Filter queries (JSON array of queries, must match but don't affect score)
    pub filter: Option<String>,
    /// Number of results
    pub limit: Option<usize>,
    /// Result offset
    pub offset: Option<usize>,
    /// Sort field:order
    pub sort: Option<String>,
    /// Comma-separated fields to return
    pub fields: Option<String>,
    /// Enable highlighting
    pub highlight: Option<bool>,
    /// Include explanation
    pub explain: Option<bool>,
    /// Minimum score threshold
    pub min_score: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexum_core::{MatchQuery, Query, SortOrder};

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), 10);
    }

    #[test]
    fn test_search_request_serialization() {
        let query = Query::Match(MatchQuery::new(
            "title".to_string(),
            "test query".to_string(),
        ));
        let request = SearchRequest {
            query: query.clone(),
            filter: None,
            limit: 20,
            offset: 5,
            sort: Some(SortOption::new("title", SortOrder::Asc)),
            fields: None,
            highlight: None,
            explain: false,
            min_score: None,
            q: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: SearchRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.limit, deserialized.limit);
        assert_eq!(request.offset, deserialized.offset);
        assert!(request.sort.is_some());
        assert!(deserialized.sort.is_some());
    }

    #[test]
    fn test_search_request_defaults() {
        let query = Query::Match(MatchQuery::new(
            "title".to_string(),
            "test query".to_string(),
        ));
        let request = SearchRequest {
            query,
            filter: None,
            limit: 10,  // default
            offset: 0,  // default
            sort: None, // default
            fields: None,
            highlight: None,
            explain: false,
            min_score: None,
            q: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: SearchRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.limit, 10);
        assert_eq!(deserialized.offset, 0);
        assert!(deserialized.sort.is_none());
    }

    #[test]
    fn test_search_request_with_custom_limits() {
        let query = Query::Match(MatchQuery::new(
            "title".to_string(),
            "test query".to_string(),
        ));
        let request = SearchRequest {
            query,
            filter: None,
            limit: 50,
            offset: 100,
            sort: None,
            fields: None,
            highlight: None,
            explain: false,
            min_score: None,
            q: None,
        };

        assert_eq!(request.limit, 50);
        assert_eq!(request.offset, 100);
    }

    #[test]
    fn test_search_request_with_filters() {
        use lexum_core::{RangeQuery, TermQuery};

        let query = Query::Match(MatchQuery::new(
            "title".to_string(),
            "test query".to_string(),
        ));

        let filters = vec![
            Query::Term(TermQuery::new("status", "active")),
            Query::Range(RangeQuery::new("age").gte(serde_json::json!(18))),
        ];

        let request = SearchRequest {
            query,
            filter: Some(filters.clone()),
            limit: 10,
            offset: 0,
            sort: None,
            fields: None,
            highlight: None,
            explain: false,
            min_score: None,
            q: None,
        };

        assert!(request.filter.is_some());
        assert_eq!(request.filter.as_ref().unwrap().len(), 2);

        // Test serialization
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: SearchRequest = serde_json::from_str(&json).unwrap();
        assert!(deserialized.filter.is_some());
        assert_eq!(deserialized.filter.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_search_request_filter_serialization() {
        use lexum_core::TermQuery;

        let query = Query::Match(MatchQuery::new("content".to_string(), "search".to_string()));

        let filter = vec![Query::Term(TermQuery::new("category", "tech"))];

        let request = SearchRequest {
            query: query.clone(),
            filter: Some(filter),
            limit: 20,
            offset: 0,
            sort: None,
            fields: None,
            highlight: None,
            explain: false,
            min_score: None,
            q: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("filter"));
        assert!(json.contains("category"));

        let deserialized: SearchRequest = serde_json::from_str(&json).unwrap();
        assert!(deserialized.filter.is_some());
    }

    #[test]
    fn test_search_request_without_filters() {
        let query = Query::Match(MatchQuery::new("title".to_string(), "test".to_string()));

        let request = SearchRequest {
            query,
            filter: None,
            limit: 10,
            offset: 0,
            sort: None,
            fields: None,
            highlight: None,
            explain: false,
            min_score: None,
            q: None,
        };

        assert!(request.filter.is_none());

        // Test serialization without filters
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: SearchRequest = serde_json::from_str(&json).unwrap();
        assert!(deserialized.filter.is_none());
    }
}
