//! Scroll API handler for efficient pagination of large result sets

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use crate::handlers::search::SearchRequest;
use axum::Json;
use axum::extract::{Path, State};
use lexum_core::Query as CoreQuery;
use lexum_core::search::{SearchExecutor, SearchResult};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Scroll context stores search state for pagination
#[derive(Debug, Clone)]
struct ScrollContext {
    /// Index name
    index_name: String,
    /// Original query
    query: CoreQuery,
    /// Filter queries
    filter: Option<Vec<CoreQuery>>,
    /// Sort option
    sort: Option<lexum_core::search::SortOption>,
    /// Fields to return
    #[allow(dead_code)] // Will be used when implementing field filtering in scroll
    fields: Option<Vec<String>>,
    /// Highlight config
    #[allow(dead_code)] // Will be used when implementing highlighting in scroll
    highlight: Option<Value>,
    /// Aggregations
    aggregations: Option<HashMap<String, lexum_core::aggregation::AggregationSpec>>,
    /// Last document offset
    last_offset: usize,
    /// Batch size
    batch_size: usize,
    /// Created timestamp
    created_at: Instant,
    /// Keep-alive duration
    keep_alive: Duration,
}

/// Scroll context manager
struct ScrollContextManager {
    contexts: Arc<RwLock<HashMap<String, ScrollContext>>>,
}

impl ScrollContextManager {
    fn new() -> Self {
        Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn create_context(
        &self,
        scroll_id: String,
        context: ScrollContext,
    ) -> Result<(), ApiError> {
        let mut contexts = self.contexts.write().await;
        contexts.insert(scroll_id, context);
        Ok(())
    }

    async fn get_context(&self, scroll_id: &str) -> Result<ScrollContext, ApiError> {
        let contexts = self.contexts.read().await;
        contexts.get(scroll_id).cloned().ok_or_else(|| {
            ApiError::InvalidRequest(format!("Scroll context not found: {scroll_id}"))
        })
    }

    async fn update_context(&self, scroll_id: &str, last_offset: usize) -> Result<(), ApiError> {
        let mut contexts = self.contexts.write().await;
        if let Some(context) = contexts.get_mut(scroll_id) {
            context.last_offset = last_offset;
            Ok(())
        } else {
            Err(ApiError::InvalidRequest(format!(
                "Scroll context not found: {scroll_id}"
            )))
        }
    }

    async fn delete_context(&self, scroll_id: &str) -> Result<(), ApiError> {
        let mut contexts = self.contexts.write().await;
        contexts.remove(scroll_id);
        Ok(())
    }

    /// Clean up expired contexts
    async fn cleanup_expired(&self) {
        let mut contexts = self.contexts.write().await;
        let now = Instant::now();
        contexts.retain(|_, context| now.duration_since(context.created_at) < context.keep_alive);
    }
}

/// Global scroll context manager
static SCROLL_MANAGER: std::sync::OnceLock<ScrollContextManager> = std::sync::OnceLock::new();

fn get_scroll_manager() -> &'static ScrollContextManager {
    SCROLL_MANAGER.get_or_init(ScrollContextManager::new)
}

/// Parse duration string (e.g., "5m", "1h", "30s")
fn parse_duration(duration_str: &str) -> Result<Duration, ApiError> {
    let duration_str = duration_str.trim();
    if duration_str.is_empty() {
        return Ok(Duration::from_secs(60)); // Default 1 minute
    }

    let (num_str, unit) = if let Some(pos) = duration_str
        .char_indices()
        .find(|(_, c)| c.is_alphabetic())
        .map(|(pos, _)| pos)
    {
        (&duration_str[..pos], &duration_str[pos..])
    } else {
        return Err(ApiError::InvalidRequest(format!(
            "Invalid duration format: {duration_str}"
        )));
    };

    let num: u64 = num_str
        .parse()
        .map_err(|_| ApiError::InvalidRequest(format!("Invalid duration number: {num_str}")))?;

    let duration = match unit {
        "s" | "S" => Duration::from_secs(num),
        "m" | "M" => Duration::from_secs(num * 60),
        "h" | "H" => Duration::from_secs(num * 3600),
        "d" | "D" => Duration::from_secs(num * 86400),
        _ => {
            return Err(ApiError::InvalidRequest(format!(
                "Invalid duration unit: {unit}. Use s, m, h, or d"
            )));
        }
    };

    Ok(duration)
}

/// Create scroll context from search request
#[derive(Debug, Deserialize)]
pub struct CreateScrollRequest {
    /// Scroll duration (e.g., "5m", "1h")
    #[serde(default = "default_scroll")]
    pub scroll: String,
    /// Search request
    #[serde(flatten)]
    pub search: SearchRequest,
}

fn default_scroll() -> String {
    "1m".to_string()
}

/// Create scroll context response
#[derive(Debug, Serialize)]
pub struct CreateScrollResponse {
    /// Scroll ID for subsequent requests
    pub scroll_id: String,
    /// Initial search results
    pub hits: SearchResult,
}

/// Create scroll context handler
/// POST /api/v1/indices/{index}/_search/scroll
pub async fn create_scroll(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
    Json(request): Json<CreateScrollRequest>,
) -> ApiResult<Json<CreateScrollResponse>> {
    // Clean up expired contexts periodically
    get_scroll_manager().cleanup_expired().await;

    let index = state
        .index_manager
        .get_index(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    let keep_alive = parse_duration(&request.scroll)?;
    let batch_size = request.search.limit.clamp(1, 10000); // Max 10k per batch

    // Convert search request to core query
    let query = if let Some(q) = &request.search.q {
        // Parse query string (simple match query)
        let text_fields = index.get_text_field_names();
        if text_fields.is_empty() {
            CoreQuery::MatchAll
        } else if text_fields.len() == 1 {
            CoreQuery::Match(lexum_core::MatchQuery::new(&text_fields[0], q.clone()))
        } else {
            let mut bool_query = lexum_core::BoolQuery::new();
            for field in text_fields {
                bool_query = bool_query.should(CoreQuery::Match(lexum_core::MatchQuery::new(
                    &field,
                    q.clone(),
                )));
            }
            CoreQuery::Bool(bool_query)
        }
    } else if let Some(ref query) = request.search.query {
        query.clone()
    } else {
        CoreQuery::MatchAll
    };

    // Build final query with filters
    let final_query = if let Some(ref filters) = request.search.filter {
        if !filters.is_empty() {
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

    // Clone values needed for context before moving them
    let query_for_context = final_query.clone();
    let sort_for_context = request.search.sort.clone();
    let filter_for_context = request.search.filter.clone();
    let fields_for_context = request.search.fields.clone();
    let highlight_for_context = request
        .search
        .highlight
        .as_ref()
        .map(|h| serde_json::to_value(h).unwrap());
    let aggregations_for_context = request.search.aggregations.clone();

    // Prepare aggregations slice if needed
    let aggregations_slice = if let Some(aggs) = &aggregations_for_context {
        let aggs_vec: Vec<_> = aggs.values().cloned().collect();
        Some(aggs_vec)
    } else {
        None
    };

    // Execute initial search
    let executor = SearchExecutor::new(Arc::new(index.clone()));
    let search_result = if let Some(ref aggs) = aggregations_slice {
        executor
            .search_with_aggregations(
                final_query,
                0,
                batch_size,
                sort_for_context.clone(),
                Some(aggs.as_slice()),
            )
            .await
            .map_err(|e| ApiError::Internal(format!("Search failed: {e}")))?
    } else {
        executor
            .search(final_query, batch_size, 0, sort_for_context.clone())
            .await
            .map_err(|e| ApiError::Internal(format!("Search failed: {e}")))?
    };

    // Apply highlighting if requested (simplified - full highlighting would require more work)
    // For now, we'll return the basic search result
    let total = search_result.total;
    let search_response = search_result;

    // Generate scroll ID
    let scroll_id = format!("scroll_{}", Uuid::new_v4());

    // Create scroll context
    let context = ScrollContext {
        index_name: index_name.clone(),
        query: query_for_context,
        filter: filter_for_context,
        sort: sort_for_context,
        fields: fields_for_context,
        highlight: highlight_for_context,
        aggregations: aggregations_for_context,
        last_offset: batch_size.min(total),
        batch_size,
        created_at: Instant::now(),
        keep_alive,
    };

    get_scroll_manager()
        .create_context(scroll_id.clone(), context)
        .await?;

    Ok(Json(CreateScrollResponse {
        scroll_id,
        hits: search_response,
    }))
}

/// Scroll request parameters
#[derive(Debug, Deserialize)]
pub struct ScrollRequest {
    /// Scroll ID from previous request
    pub scroll_id: String,
    /// Scroll duration to extend keep-alive (optional)
    #[serde(default)]
    pub scroll: Option<String>,
}

/// Continue scroll and get next batch
/// POST /api/v1/_search/scroll
pub async fn scroll(
    State(state): State<AppState>,
    Json(request): Json<ScrollRequest>,
) -> ApiResult<Json<SearchResult>> {
    // Clean up expired contexts
    get_scroll_manager().cleanup_expired().await;

    let manager = get_scroll_manager();
    let mut context = manager.get_context(&request.scroll_id).await?;

    // Check if context expired
    if Instant::now().duration_since(context.created_at) >= context.keep_alive {
        manager.delete_context(&request.scroll_id).await?;
        return Err(ApiError::InvalidRequest(
            "Scroll context expired".to_string(),
        ));
    }

    // Extend keep-alive if requested
    if let Some(scroll_duration) = request.scroll {
        let keep_alive = parse_duration(&scroll_duration)?;
        context.keep_alive = keep_alive;
        context.created_at = Instant::now(); // Reset timer
    }

    let index = state
        .index_manager
        .get_index(&context.index_name)
        .map_err(|_| ApiError::IndexNotFound(context.index_name.clone()))?;

    // Build final query with filters
    let final_query = if let Some(ref filters) = context.filter {
        if !filters.is_empty() {
            let mut bool_query = lexum_core::BoolQuery::new();
            bool_query = bool_query.must(context.query.clone());
            for filter in filters {
                bool_query = bool_query.filter(filter.clone());
            }
            lexum_core::Query::Bool(bool_query)
        } else {
            context.query.clone()
        }
    } else {
        context.query.clone()
    };

    // Prepare aggregations slice if needed
    let aggregations_slice = if let Some(aggs) = &context.aggregations {
        let aggs_vec: Vec<_> = aggs.values().cloned().collect();
        Some(aggs_vec)
    } else {
        None
    };

    // Execute search with offset
    let executor = SearchExecutor::new(Arc::new(index.clone()));
    let search_result = if let Some(ref aggs) = aggregations_slice {
        executor
            .search_with_aggregations(
                final_query,
                context.last_offset,
                context.batch_size,
                context.sort.clone(),
                Some(aggs.as_slice()),
            )
            .await
            .map_err(|e| ApiError::Internal(format!("Search failed: {e}")))?
    } else {
        executor
            .search(
                final_query,
                context.batch_size,
                context.last_offset,
                context.sort.clone(),
            )
            .await
            .map_err(|e| ApiError::Internal(format!("Search failed: {e}")))?
    };

    // Get hits count and check if empty before moving search_result
    let hits_count = search_result.hits.len();
    let is_empty = hits_count == 0;

    // Return search result (highlighting would require additional processing)
    let search_response = search_result;

    // Update context with new offset
    let new_offset = context.last_offset + hits_count;
    manager
        .update_context(&request.scroll_id, new_offset)
        .await?;

    // If no more results, delete context
    if is_empty {
        manager.delete_context(&request.scroll_id).await?;
    }

    Ok(Json(search_response))
}

/// Clear scroll context
/// DELETE /api/v1/_search/scroll/{scroll_id}
pub async fn clear_scroll(Path(scroll_id): Path<String>) -> ApiResult<Json<Value>> {
    get_scroll_manager().delete_context(&scroll_id).await?;

    Ok(Json(json!({
        "acknowledged": true
    })))
}

/// Clear all scroll contexts
/// DELETE /api/v1/_search/scroll/_all
pub async fn clear_all_scrolls() -> ApiResult<Json<Value>> {
    let manager = get_scroll_manager();
    let contexts = manager.contexts.read().await;
    let count = contexts.len();
    drop(contexts);

    // Clear all contexts
    let mut contexts = manager.contexts.write().await;
    contexts.clear();

    Ok(Json(json!({
        "acknowledged": true,
        "num_freed": count
    })))
}
