//! Query-based operations: Update by Query, Delete by Query, Multi-Get, Multi-Search

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use crate::handlers::search::SearchRequest;
use axum::Json;
use axum::extract::{Path, State};
use lexum_core::search::{SearchExecutor, SearchResult};
use lexum_core::{DocumentStore, Query as CoreQuery};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Update by Query request
#[derive(Debug, Deserialize)]
pub struct UpdateByQueryRequest {
    /// Query to match documents
    #[serde(flatten)]
    pub query: SearchRequest,
    /// Script to update documents (optional)
    #[serde(default)]
    pub script: Option<Script>,
    /// Fields to update (if script not provided)
    #[serde(default)]
    pub doc: Option<JsonValue>,
    /// Batch size for processing
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Refresh index after update
    #[serde(default = "default_refresh")]
    pub refresh: bool,
}

fn default_batch_size() -> usize {
    1000
}

fn default_refresh() -> bool {
    true
}

/// Script for updating documents
#[derive(Debug, Deserialize)]
pub struct Script {
    /// Script source
    pub source: String,
    /// Script parameters
    #[serde(default)]
    pub params: Option<JsonValue>,
}

/// Update by Query response
#[derive(Debug, Serialize)]
pub struct UpdateByQueryResponse {
    /// Number of documents updated
    pub updated: usize,
    /// Number of batches processed
    pub batches: usize,
    /// Time taken in milliseconds
    pub took_ms: u64,
}

/// Update documents matching a query
/// POST /api/v1/indices/{index}/_update_by_query
pub async fn update_by_query(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
    Json(request): Json<UpdateByQueryRequest>,
) -> ApiResult<Json<UpdateByQueryResponse>> {
    let start = std::time::Instant::now();

    let index = state
        .index_manager
        .get_index(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    // Build query from search request
    let query = if let Some(q) = &request.query.q {
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
    } else if let Some(ref query) = request.query.query {
        query.clone()
    } else {
        CoreQuery::MatchAll
    };

    // Build final query with filters
    let final_query = if let Some(ref filters) = request.query.filter {
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

    // Execute search to find matching documents
    let executor = SearchExecutor::new(Arc::new(index.clone()));
    let mut updated = 0;
    let mut batches = 0;
    let mut offset = 0;
    let batch_size = request.batch_size.min(10000); // Max 10k per batch

    loop {
        let search_result = executor
            .search(final_query.clone(), batch_size, offset, None)
            .await
            .map_err(|e| ApiError::Internal(format!("Search failed: {e}")))?;

        if search_result.hits.is_empty() {
            break;
        }

        // Update each document
        let store = DocumentStore::new(Arc::new(index.clone()));
        for hit in &search_result.hits {
            if let Some(ref _script) = request.script {
                // Script-based update (simplified - would need script engine)
                // For now, we'll skip script updates
                tracing::warn!("Script-based updates not yet implemented");
            } else if let Some(ref doc) = request.doc {
                // Merge document update
                let mut current_doc = store
                    .get_document(&hit.id)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Failed to get document: {e}")))?;

                if let (Some(current_obj), Some(update_obj)) =
                    (current_doc.as_object_mut(), doc.as_object())
                {
                    // Merge update fields into current document
                    for (key, value) in update_obj {
                        current_obj.insert(key.clone(), value.clone());
                    }
                    store
                        .update_document(&hit.id, current_doc.clone())
                        .await
                        .map_err(|e| {
                            ApiError::Internal(format!("Failed to update document: {e}"))
                        })?;
                    updated += 1;
                }
            }
        }

        batches += 1;
        offset += search_result.hits.len();

        // Break if we've processed all results
        if search_result.hits.len() < batch_size {
            break;
        }
    }

    // Refresh index if requested
    if request.refresh {
        state
            .index_manager
            .refresh_index(&index_name)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to refresh index: {e}")))?;
    }

    let took_ms = start.elapsed().as_millis() as u64;

    Ok(Json(UpdateByQueryResponse {
        updated,
        batches,
        took_ms,
    }))
}

/// Delete by Query request
#[derive(Debug, Deserialize)]
pub struct DeleteByQueryRequest {
    /// Query to match documents
    #[serde(flatten)]
    pub query: SearchRequest,
    /// Batch size for processing
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Refresh index after deletion
    #[serde(default = "default_refresh")]
    pub refresh: bool,
}

/// Delete by Query response
#[derive(Debug, Serialize)]
pub struct DeleteByQueryResponse {
    /// Number of documents deleted
    pub deleted: usize,
    /// Number of batches processed
    pub batches: usize,
    /// Time taken in milliseconds
    pub took_ms: u64,
}

/// Delete documents matching a query
/// POST /api/v1/indices/{index}/_delete_by_query
pub async fn delete_by_query(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
    Json(request): Json<DeleteByQueryRequest>,
) -> ApiResult<Json<DeleteByQueryResponse>> {
    let start = std::time::Instant::now();

    let index = state
        .index_manager
        .get_index(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    // Build query from search request
    let query = if let Some(q) = &request.query.q {
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
    } else if let Some(ref query) = request.query.query {
        query.clone()
    } else {
        return Err(ApiError::InvalidRequest(
            "Query is required for delete by query".to_string(),
        ));
    };

    // Build final query with filters
    let final_query = if let Some(ref filters) = request.query.filter {
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

    // Execute search to find matching documents
    let executor = SearchExecutor::new(Arc::new(index.clone()));
    let mut deleted = 0;
    let mut batches = 0;
    let mut offset = 0;
    let batch_size = request.batch_size.min(10000); // Max 10k per batch

    loop {
        let search_result = executor
            .search(final_query.clone(), batch_size, offset, None)
            .await
            .map_err(|e| ApiError::Internal(format!("Search failed: {e}")))?;

        if search_result.hits.is_empty() {
            break;
        }

        // Delete each document
        let store = DocumentStore::new(Arc::new(index.clone()));
        for hit in &search_result.hits {
            store
                .delete_document(&hit.id)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to delete document: {e}")))?;
            deleted += 1;
        }

        batches += 1;
        offset += search_result.hits.len();

        // Break if we've processed all results
        if search_result.hits.len() < batch_size {
            break;
        }
    }

    // Refresh index if requested
    if request.refresh {
        state
            .index_manager
            .refresh_index(&index_name)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to refresh index: {e}")))?;
    }

    let took_ms = start.elapsed().as_millis() as u64;

    Ok(Json(DeleteByQueryResponse {
        deleted,
        batches,
        took_ms,
    }))
}

/// Multi-Get request item
#[derive(Debug, Deserialize)]
pub struct MultiGetItem {
    /// Index name
    #[serde(default)]
    pub index: Option<String>,
    /// Document ID
    pub id: String,
    /// Fields to return
    #[serde(default)]
    pub fields: Option<Vec<String>>,
}

/// Multi-Get request
#[derive(Debug, Deserialize)]
pub struct MultiGetRequest {
    /// Documents to retrieve
    pub docs: Vec<MultiGetItem>,
}

/// Multi-Get response item
#[derive(Debug, Serialize)]
pub struct MultiGetResponseItem {
    /// Index name
    pub index: String,
    /// Document ID
    pub id: String,
    /// Document found
    pub found: bool,
    /// Document source (if found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<JsonValue>,
}

/// Multi-Get response
#[derive(Debug, Serialize)]
pub struct MultiGetResponse {
    /// Retrieved documents
    pub docs: Vec<MultiGetResponseItem>,
}

/// Retrieve multiple documents by ID
/// POST /api/v1/_mget
pub async fn multi_get(
    State(state): State<AppState>,
    Json(request): Json<MultiGetRequest>,
) -> ApiResult<Json<MultiGetResponse>> {
    let mut docs = Vec::new();

    for item in request.docs {
        let index_name = item.index.as_deref().unwrap_or("");
        if index_name.is_empty() {
            return Err(ApiError::InvalidRequest(
                "Index name is required for each document".to_string(),
            ));
        }

        let index = state
            .index_manager
            .get_index(index_name)
            .map_err(|_| ApiError::IndexNotFound(index_name.to_string()))?;

        let store = DocumentStore::new(Arc::new(index));
        let doc_id = lexum_core::types::DocumentId::new(item.id.clone());

        match store.get_document(&doc_id).await {
            Ok(mut doc) => {
                // Apply field filtering if requested
                if let Some(fields) = item.fields {
                    if let Some(doc_obj) = doc.as_object_mut() {
                        let filtered: serde_json::Map<String, JsonValue> = doc_obj
                            .iter()
                            .filter(|(key, _)| fields.contains(key))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        doc = JsonValue::Object(filtered);
                    }
                }

                docs.push(MultiGetResponseItem {
                    index: index_name.to_string(),
                    id: item.id,
                    found: true,
                    source: Some(doc),
                });
            }
            Err(_) => {
                docs.push(MultiGetResponseItem {
                    index: index_name.to_string(),
                    id: item.id,
                    found: false,
                    source: None,
                });
            }
        }
    }

    Ok(Json(MultiGetResponse { docs }))
}

/// Multi-Search request item
#[derive(Debug, Deserialize)]
pub struct MultiSearchItem {
    /// Index name
    #[serde(default)]
    pub index: Option<String>,
    /// Search request
    #[serde(flatten)]
    pub search: SearchRequest,
}

/// Multi-Search request
#[derive(Debug, Deserialize)]
pub struct MultiSearchRequest {
    /// Search requests to execute
    pub searches: Vec<MultiSearchItem>,
}

/// Multi-Search response
#[derive(Debug, Serialize)]
pub struct MultiSearchResponse {
    /// Search results
    pub responses: Vec<SearchResult>,
}

/// Execute multiple search requests
/// POST /api/v1/_msearch
pub async fn multi_search(
    State(state): State<AppState>,
    Json(request): Json<MultiSearchRequest>,
) -> ApiResult<Json<MultiSearchResponse>> {
    let mut responses = Vec::new();

    for item in request.searches {
        let index_name = item.index.as_deref().unwrap_or("");
        if index_name.is_empty() {
            return Err(ApiError::InvalidRequest(
                "Index name is required for each search".to_string(),
            ));
        }

        let index = state
            .index_manager
            .get_index(index_name)
            .map_err(|_| ApiError::IndexNotFound(index_name.to_string()))?;

        // Build query from search request
        let query = if let Some(q) = &item.search.q {
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
        } else if let Some(ref query) = item.search.query {
            query.clone()
        } else {
            CoreQuery::MatchAll
        };

        // Build final query with filters
        let final_query = if let Some(ref filters) = item.search.filter {
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

        // Prepare aggregations slice if needed
        let aggregations_slice = if let Some(aggs) = &item.search.aggregations {
            let aggs_vec: Vec<_> = aggs.values().cloned().collect();
            Some(aggs_vec)
        } else {
            None
        };

        // Execute search
        let executor = SearchExecutor::new(Arc::new(index));
        let result = if let Some(ref aggs) = aggregations_slice {
            executor
                .search_with_aggregations(
                    final_query,
                    item.search.limit,
                    item.search.offset,
                    item.search.sort,
                    Some(aggs.as_slice()),
                )
                .await
                .map_err(|e| ApiError::Internal(format!("Search failed: {e}")))?
        } else {
            executor
                .search(
                    final_query,
                    item.search.limit,
                    item.search.offset,
                    item.search.sort,
                )
                .await
                .map_err(|e| ApiError::Internal(format!("Search failed: {e}")))?
        };

        responses.push(result);
    }

    Ok(Json(MultiSearchResponse { responses }))
}
