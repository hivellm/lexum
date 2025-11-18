//! Query-based document operations (Update by Query, Delete by Query)

use crate::error::{Error, Result};
use crate::index::Index;
use crate::query::Query;
use crate::search::executor::SearchExecutor;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use utoipa::ToSchema;

/// Update by Query request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateByQueryRequest {
    /// Query to match documents
    pub query: Query,
    /// Document updates (fields to update)
    #[serde(default)]
    pub doc: Option<JsonValue>,
    /// Script for updating documents
    #[serde(default)]
    pub script: Option<UpdateScript>,
    /// Batch size for processing
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Whether to refresh index after update
    #[serde(default = "default_refresh")]
    pub refresh: bool,
    /// Maximum number of documents to update
    #[serde(default)]
    pub max_docs: Option<usize>,
}

fn default_batch_size() -> usize {
    1000
}

fn default_refresh() -> bool {
    true
}

/// Update script
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateScript {
    /// Script source
    pub source: String,
    /// Script language
    #[serde(default = "default_lang")]
    pub lang: String,
    /// Script parameters
    #[serde(default)]
    pub params: Option<JsonValue>,
}

fn default_lang() -> String {
    "painless".to_string()
}

/// Update by Query response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateByQueryResponse {
    /// Number of documents updated
    pub updated: usize,
    /// Number of batches processed
    pub batches: usize,
    /// Time taken in milliseconds
    pub took_ms: u64,
    /// Number of errors
    pub errors: usize,
    /// Error details
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub error_details: Vec<String>,
}

/// Delete by Query request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeleteByQueryRequest {
    /// Query to match documents
    pub query: Query,
    /// Batch size for processing
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Whether to refresh index after deletion
    #[serde(default = "default_refresh")]
    pub refresh: bool,
    /// Maximum number of documents to delete
    #[serde(default)]
    pub max_docs: Option<usize>,
}

/// Delete by Query response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeleteByQueryResponse {
    /// Number of documents deleted
    pub deleted: usize,
    /// Number of batches processed
    pub batches: usize,
    /// Time taken in milliseconds
    pub took_ms: u64,
    /// Number of errors
    pub errors: usize,
    /// Error details
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub error_details: Vec<String>,
}

/// Query-based operations for documents
pub struct QueryOperations {
    #[allow(dead_code)]
    index: Arc<Index>,
    store: Arc<crate::document::store::DocumentStore>,
    executor: Arc<SearchExecutor>,
}

impl QueryOperations {
    /// Create new query operations handler
    pub fn new(index: Arc<Index>) -> Self {
        let store = Arc::new(crate::document::store::DocumentStore::new(index.clone()));
        let executor = Arc::new(SearchExecutor::new(index.clone()));
        Self {
            index,
            store,
            executor,
        }
    }

    /// Update documents matching a query
    pub async fn update_by_query(
        &self,
        request: UpdateByQueryRequest,
    ) -> Result<UpdateByQueryResponse> {
        let start_time = std::time::Instant::now();
        let mut updated = 0;
        let mut batches = 0;
        let mut errors = 0;
        let mut error_details = Vec::new();

        // Search for matching documents
        let mut offset = 0;
        let batch_size = request.batch_size;
        let max_docs = request.max_docs.unwrap_or(usize::MAX);

        loop {
            // Search for documents matching the query
            let search_result = self
                .executor
                .search(request.query.clone(), batch_size, offset, None)
                .await?;

            if search_result.hits.is_empty() {
                break;
            }

            // Process each document
            for hit in &search_result.hits {
                if updated >= max_docs {
                    break;
                }

                match self.update_document_from_hit(hit, &request).await {
                    Ok(_) => updated += 1,
                    Err(e) => {
                        errors += 1;
                        error_details.push(format!("Failed to update document {}: {}", hit.id, e));
                    }
                }
            }

            batches += 1;
            offset += batch_size;

            // Check if we've processed all documents or reached max_docs
            if search_result.hits.len() < batch_size || updated >= max_docs {
                break;
            }
        }

        // Refresh index if requested
        // Note: Index refresh is handled by IndexManager, not directly on Index
        // For now, we'll skip refresh here as it should be handled at a higher level
        if request.refresh {
            tracing::debug!("Refresh requested after update_by_query (handled by IndexManager)");
        }

        Ok(UpdateByQueryResponse {
            updated,
            batches,
            took_ms: start_time.elapsed().as_millis() as u64,
            errors,
            error_details,
        })
    }

    /// Delete documents matching a query
    pub async fn delete_by_query(
        &self,
        request: DeleteByQueryRequest,
    ) -> Result<DeleteByQueryResponse> {
        let start_time = std::time::Instant::now();
        let mut deleted = 0;
        let mut batches = 0;
        let mut errors = 0;
        let mut error_details = Vec::new();

        // Search for matching documents
        let mut offset = 0;
        let batch_size = request.batch_size;
        let max_docs = request.max_docs.unwrap_or(usize::MAX);

        loop {
            // Search for documents matching the query
            let search_result = self
                .executor
                .search(request.query.clone(), batch_size, offset, None)
                .await?;

            if search_result.hits.is_empty() {
                break;
            }

            // Delete each document
            for hit in &search_result.hits {
                if deleted >= max_docs {
                    break;
                }

                match self.store.delete_document(&hit.id).await {
                    Ok(_) => deleted += 1,
                    Err(e) => {
                        errors += 1;
                        error_details.push(format!("Failed to delete document {}: {}", hit.id, e));
                    }
                }
            }

            batches += 1;
            offset += batch_size;

            // Check if we've processed all documents or reached max_docs
            if search_result.hits.len() < batch_size || deleted >= max_docs {
                break;
            }
        }

        // Refresh index if requested
        // Note: Index refresh is handled by IndexManager, not directly on Index
        // For now, we'll skip refresh here as it should be handled at a higher level
        if request.refresh {
            tracing::debug!("Refresh requested after delete_by_query (handled by IndexManager)");
        }

        Ok(DeleteByQueryResponse {
            deleted,
            batches,
            took_ms: start_time.elapsed().as_millis() as u64,
            errors,
            error_details,
        })
    }

    /// Update a single document from a search hit
    async fn update_document_from_hit(
        &self,
        hit: &crate::search::result::SearchHit,
        request: &UpdateByQueryRequest,
    ) -> Result<()> {
        // Get current document
        let current_doc = hit.source.clone();

        // Apply updates
        let updated_doc = if let Some(ref _script) = request.script {
            // Script-based update
            // Note: Full implementation requires script engine integration
            // For now, return error indicating script support is not yet available
            return Err(Error::Config(
                "Script-based updates require script engine integration".to_string(),
            ));
        } else if let Some(ref doc_updates) = request.doc {
            // Field-based update: merge doc_updates into current_doc
            Self::merge_documents(&current_doc, doc_updates)
        } else {
            return Err(Error::Config(
                "Either 'doc' or 'script' must be provided for update".to_string(),
            ));
        };

        // Update the document
        self.store.update_document(&hit.id, updated_doc).await?;

        Ok(())
    }

    /// Merge update document into current document
    fn merge_documents(current: &JsonValue, updates: &JsonValue) -> JsonValue {
        match (current, updates) {
            (JsonValue::Object(current_map), JsonValue::Object(updates_map)) => {
                let mut merged = current_map.clone();
                for (key, value) in updates_map {
                    // Recursively merge nested objects
                    if let (Some(JsonValue::Object(current_obj)), JsonValue::Object(updates_obj)) =
                        (merged.get(key), value)
                    {
                        let nested_merged = Self::merge_documents(
                            &JsonValue::Object(current_obj.clone()),
                            &JsonValue::Object(updates_obj.clone()),
                        );
                        merged.insert(key.clone(), nested_merged);
                    } else {
                        // Replace or add field
                        merged.insert(key.clone(), value.clone());
                    }
                }
                JsonValue::Object(merged)
            }
            _ => updates.clone(), // If types don't match, use updates
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::MatchQuery;

    #[test]
    fn test_merge_documents() {
        let current = serde_json::json!({
            "title": "Original Title",
            "content": "Original Content",
            "metadata": {
                "author": "John",
                "views": 10
            }
        });

        let updates = serde_json::json!({
            "title": "Updated Title",
            "metadata": {
                "views": 20
            },
            "new_field": "new_value"
        });

        let merged = QueryOperations::merge_documents(&current, &updates);

        assert_eq!(merged["title"], "Updated Title");
        assert_eq!(merged["content"], "Original Content");
        assert_eq!(merged["metadata"]["author"], "John");
        assert_eq!(merged["metadata"]["views"], 20);
        assert_eq!(merged["new_field"], "new_value");
    }

    #[test]
    fn test_update_by_query_request_serialization() {
        let request = UpdateByQueryRequest {
            query: Query::Match(MatchQuery::new("field", "value")),
            doc: Some(serde_json::json!({"status": "updated"})),
            script: None,
            batch_size: 100,
            refresh: true,
            max_docs: Some(1000),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("query"));
        assert!(json.contains("batch_size"));
    }

    #[test]
    fn test_delete_by_query_request_serialization() {
        let request = DeleteByQueryRequest {
            query: Query::Match(MatchQuery::new("field", "value")),
            batch_size: 100,
            refresh: true,
            max_docs: Some(1000),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("query"));
        assert!(json.contains("batch_size"));
    }
}
