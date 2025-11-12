//! Search handler

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::Json;
use axum::extract::{Path, State};
use lexum_core::{Query, SearchExecutor, SearchResult, SortOption};
use serde::{Deserialize, Serialize};
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
}

fn default_pre_tag() -> String {
    "<em>".to_string()
}

fn default_post_tag() -> String {
    "</em>".to_string()
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
        // Convert simple query string to match query
        lexum_core::Query::Match(lexum_core::MatchQuery::new("_all", q.clone()))
    } else {
        request.query
    };

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
        for hit in &mut result.hits {
            if let serde_json::Value::Object(ref mut source) = hit.source {
                for field in &highlight.fields {
                    if let Some(serde_json::Value::String(text)) = source.get(field) {
                        // Simple highlighting - in production this would be more sophisticated
                        let query_text = request.q.as_deref().unwrap_or("");
                        let highlighted = text.replace(
                            query_text,
                            &format!("{}{}{}", highlight.pre_tag, query_text, highlight.post_tag),
                        );
                        source.insert(
                            format!("{field}_highlighted"),
                            serde_json::Value::String(highlighted),
                        );
                    }
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
        lexum_core::Query::Match(lexum_core::MatchQuery::new("_all", q.clone()))
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
            for hit in &mut result.hits {
                if let serde_json::Value::Object(ref mut source) = hit.source {
                    let mut highlighted_fields = std::collections::HashMap::new();

                    for field in &highlight.fields {
                        if let Some(value) = source.get(field) {
                            if let Some(text) = value.as_str() {
                                // Simple highlighting - wrap matched terms
                                let highlighted = text
                                    .split_whitespace()
                                    .map(|word| {
                                        if word.to_lowercase().contains(
                                            &request
                                                .q
                                                .as_ref()
                                                .unwrap_or(&String::new())
                                                .to_lowercase(),
                                        ) {
                                            format!(
                                                "{}{}{}",
                                                highlight.pre_tag, word, highlight.post_tag
                                            )
                                        } else {
                                            word.to_string()
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join(" ");

                                highlighted_fields.insert(
                                    format!("{field}_highlighted"),
                                    serde_json::Value::String(highlighted),
                                );
                            }
                        }
                    }

                    source.extend(highlighted_fields);
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
