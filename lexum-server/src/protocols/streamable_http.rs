//! StreamableHTTP protocol handler - streaming search results over HTTP with chunked transfer encoding

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use crate::handlers::search::SearchRequest;
use axum::Json;
use axum::extract::{Path, State};
use axum::response::Response;
use futures::StreamExt;
use futures::stream as futures_stream;
use lexum_core::{Query, SearchExecutor};
use serde_json::json;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

/// StreamableHTTP search endpoint - streams results as they are found
/// Uses chunked transfer encoding for progressive result delivery
pub async fn stream_search(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
    Json(request): Json<SearchRequest>,
) -> ApiResult<Response> {
    // Resolve alias to actual index names
    let target_indices = state
        .index_manager
        .resolve_name(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    if target_indices.is_empty() {
        return Err(ApiError::IndexNotFound(index_name));
    }

    // Get index
    let index = state
        .index_manager
        .get_index(target_indices[0].as_str())
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    // Build query (same logic as regular search)
    let query = if let Some(ref q) = request.q {
        let text_fields = index.get_text_field_names();
        if text_fields.is_empty() {
            Query::MatchAll
        } else if text_fields.len() == 1 {
            Query::Match(lexum_core::MatchQuery::new(&text_fields[0], q.clone()))
        } else {
            let mut bool_query = lexum_core::BoolQuery::new();
            for field in text_fields {
                bool_query =
                    bool_query.should(Query::Match(lexum_core::MatchQuery::new(&field, q.clone())));
            }
            Query::Bool(bool_query)
        }
    } else if let Some(ref query) = request.query {
        query.clone()
    } else {
        return Err(ApiError::InvalidRequest(
            "Either 'query' or 'q' parameter is required".to_string(),
        ));
    };

    // Apply filters if provided
    let final_query = if let Some(ref filters) = request.filter {
        if !filters.is_empty() {
            let mut bool_query = lexum_core::BoolQuery::new();
            bool_query = bool_query.must(query.clone());
            for filter in filters {
                bool_query = bool_query.filter(filter.clone());
            }
            Query::Bool(bool_query)
        } else {
            query
        }
    } else {
        query
    };

    // Create executor
    let executor = SearchExecutor::new(Arc::new(index));

    // Stream results in chunks using a custom stream implementation
    let limit = request.limit;
    let offset = request.offset;
    let sort = request.sort.clone();
    let query = final_query.clone();

    let stream = futures_stream::unfold(
        (executor, query, limit, offset, sort, 0usize),
        |(executor, query, limit, offset, sort, current_offset)| async move {
            // Check if we've reached the limit
            if current_offset >= limit {
                return None;
            }

            // Calculate batch size (stream in chunks of 10)
            let batch_size = std::cmp::min(10, limit - current_offset);

            // Execute search for this batch
            match executor
                .search(
                    query.clone(),
                    batch_size,
                    offset + current_offset,
                    sort.clone(),
                )
                .await
            {
                Ok(result) => {
                    if result.hits.is_empty() {
                        return None;
                    }

                    // Serialize each hit as a separate chunk
                    let chunks: Vec<Result<String, std::io::Error>> = result
                        .hits
                        .iter()
                        .enumerate()
                        .map(|(idx, hit)| {
                            serde_json::to_string(&json!({
                                "hit": hit,
                                "offset": current_offset + idx
                            }))
                            .map_err(std::io::Error::other)
                        })
                        .collect();

                    // Add small delay for backpressure handling
                    sleep(Duration::from_millis(10)).await;

                    Some((
                        futures_stream::iter(chunks),
                        (
                            executor,
                            query,
                            limit,
                            offset,
                            sort,
                            current_offset + result.hits.len(),
                        ),
                    ))
                }
                Err(_) => None,
            }
        },
    )
    .flatten();

    // Create streaming response with chunked transfer encoding
    use axum::body::Body;
    use axum::body::Bytes;
    use std::convert::Infallible;

    let body_stream: futures_stream::Map<_, _> =
        stream.map(|chunk: Result<String, std::io::Error>| {
            chunk.map(|s| {
                // Format as NDJSON (newline-delimited JSON) for streaming
                Bytes::from(format!("{s}\n"))
            })
        });

    let body =
        Body::from_stream(body_stream.map(|r: Result<Bytes, std::io::Error>| {
            r.map_err(|_| -> Infallible { unreachable!() })
        }));

    let response = Response::builder()
        .status(200)
        .header("Content-Type", "application/x-ndjson")
        .header("Transfer-Encoding", "chunked")
        .header("Connection", "keep-alive")
        .body(body)
        .map_err(|e| ApiError::Internal(format!("Failed to create response: {e}")))?;

    Ok(response)
}
