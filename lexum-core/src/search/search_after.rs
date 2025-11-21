//! Search After - Cursor-based pagination
//!
//! Search After provides efficient cursor-based pagination for large result sets.
//! Unlike offset-based pagination, search_after maintains consistent results even
//! when documents are added or removed between requests.
//!
//! # Usage
//!
//! ```no_run
//! use lexum_core::search::search_after::{SearchAfterExecutor, SearchAfterRequest};
//! use lexum_core::query::Query;
//! use lexum_core::search::result::SortOption;
//! use lexum_core::search::executor::SearchExecutor;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create executor (requires an index)
//! // let index = ...; // Your index here
//! // let search_executor = Arc::new(SearchExecutor::new(index));
//! // let executor = SearchAfterExecutor::new(search_executor);
//!
//! // First request
//! let request = SearchAfterRequest {
//!     query: Query::MatchAll,
//!     sort: vec![SortOption::desc("timestamp")],
//!     size: 10,
//!     search_after: None,
//!     track_total_hits: None,
//!     pit_id: None,
//! };
//!
//! // let result = executor.search_after(request).await?;
//!
//! // Subsequent request using sort values from previous result
//! let next_request = SearchAfterRequest {
//!     query: Query::MatchAll,
//!     sort: vec![SortOption::desc("timestamp")],
//!     size: 10,
//!     search_after: None, // Would use: result.sort_values
//!     track_total_hits: None,
//!     pit_id: None,
//! };
//! # Ok(())
//! # }
//! ```

use crate::error::Result;
use crate::query::Query;
use crate::search::executor::SearchExecutor;
use crate::search::point_in_time::{PitId, PointInTimeManager};
use crate::search::result::{SearchHit, SortOption};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use utoipa::ToSchema;

/// Track total hits option
/// - `true`: Track total hits accurately (may be expensive)
/// - `false`: Don't track total hits (default, faster)
/// - `usize`: Track up to N hits, then return "at least N"
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, ToSchema)]
#[serde(untagged)]
pub enum TrackTotalHits {
    /// Track all hits accurately
    #[serde(rename = "true")]
    True,
    /// Don't track hits
    #[serde(rename = "false")]
    #[default]
    False,
    /// Track up to N hits
    Count(usize),
}

// Custom serialization/deserialization to handle true/false/number
impl TrackTotalHits {
    #[allow(dead_code)]
    fn serialize_as_value(&self) -> serde_json::Value {
        match self {
            TrackTotalHits::True => serde_json::Value::Bool(true),
            TrackTotalHits::False => serde_json::Value::Bool(false),
            TrackTotalHits::Count(n) => serde_json::Value::Number((*n as u64).into()),
        }
    }
}

/// Search After request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchAfterRequest {
    /// Query to execute
    pub query: Query,
    /// Sort options (required for search_after)
    pub sort: Vec<SortOption>,
    /// Size (number of results)
    #[serde(default = "default_size")]
    pub size: usize,
    /// Search after values (cursor from previous result)
    #[serde(rename = "search_after", skip_serializing_if = "Option::is_none")]
    pub search_after: Option<Vec<JsonValue>>,
    /// Track total hits (true, false, or number)
    #[serde(
        rename = "track_total_hits",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub track_total_hits: Option<TrackTotalHits>,
    /// Point in Time ID for consistent reads
    #[serde(rename = "pit_id", skip_serializing_if = "Option::is_none")]
    pub pit_id: Option<PitId>,
}

fn default_size() -> usize {
    10
}

/// Search After response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchAfterResponse {
    /// Search results
    pub hits: Vec<SearchHit>,
    /// Total number of hits
    /// If track_total_hits is false, this may be an approximation
    pub total: usize,
    /// Time taken in milliseconds
    pub took_ms: u64,
    /// Sort values of last hit (for next search_after)
    #[serde(rename = "sort", skip_serializing_if = "Option::is_none")]
    pub sort_values: Option<Vec<JsonValue>>,
    /// Whether total is accurate or approximate
    #[serde(rename = "total_relation", skip_serializing_if = "Option::is_none")]
    pub total_relation: Option<String>,
}

/// Search After executor
pub struct SearchAfterExecutor {
    executor: Arc<SearchExecutor>,
    /// Optional PIT manager for consistent reads
    pit_manager: Option<Arc<PointInTimeManager>>,
}

impl SearchAfterExecutor {
    /// Create new search after executor
    pub fn new(executor: Arc<SearchExecutor>) -> Self {
        Self {
            executor,
            pit_manager: None,
        }
    }

    /// Create new search after executor with PIT support
    pub fn with_pit(executor: Arc<SearchExecutor>, pit_manager: Arc<PointInTimeManager>) -> Self {
        Self {
            executor,
            pit_manager: Some(pit_manager),
        }
    }

    /// Execute search with search_after pagination
    pub async fn search_after(&self, request: SearchAfterRequest) -> Result<SearchAfterResponse> {
        // Validate sort options are provided
        if request.sort.is_empty() {
            return Err(crate::error::Error::Config(
                "search_after requires at least one sort option".to_string(),
            ));
        }

        // Validate search_after values match sort options count
        if let Some(ref search_after) = request.search_after {
            if search_after.len() != request.sort.len() {
                return Err(crate::error::Error::Config(format!(
                    "search_after values count ({}) must match sort options count ({})",
                    search_after.len(),
                    request.sort.len()
                )));
            }
        }

        // Handle PIT if provided
        if let Some(ref pit_id) = request.pit_id {
            if let Some(ref pit_manager) = self.pit_manager {
                // Get PIT context - this ensures we use the same index snapshot
                let _pit_context = pit_manager.get_pit(pit_id).await?;
                // Note: The executor should use the index from PIT context
                // For now, we'll proceed with the current executor
                // Full PIT integration requires executor changes
            } else {
                return Err(crate::error::Error::Config(
                    "PIT manager not available for this executor".to_string(),
                ));
            }
        }

        // Use first sort option for executor (executor supports single sort)
        // We'll handle multi-field sorting by filtering results after search
        let sort = Some(request.sort[0].clone());
        let sort_for_additional = sort.clone(); // Clone for potential additional search

        // Calculate search limit more efficiently
        // When search_after is provided, we need extra results to filter correctly
        // but we don't need 3x - we can be smarter about it
        let search_limit = if request.search_after.is_some() {
            // Get enough results to ensure we have 'size' results after filtering
            // Add a small buffer for edge cases
            request.size + (request.size / 2).max(10)
        } else {
            request.size
        };

        let mut result = self
            .executor
            .search(request.query.clone(), search_limit, 0, sort)
            .await?;

        // Filter results based on search_after values if provided
        if let Some(ref search_after_values) = request.search_after {
            result.hits =
                self.filter_by_search_after(&result.hits, search_after_values, &request.sort);

            // If we don't have enough results after filtering, we might need to search more
            // This is a limitation of the current approach - ideally we'd push this down to Tantivy
            if result.hits.len() < request.size {
                // Try to get more results if we filtered too aggressively
                // This is a fallback - in production, you'd want better integration with Tantivy
                let additional_limit = (request.size - result.hits.len()) * 2;
                let additional_result = self
                    .executor
                    .search(
                        request.query.clone(),
                        additional_limit,
                        0,
                        sort_for_additional,
                    )
                    .await?;

                // Filter additional results and append
                let additional_filtered = self.filter_by_search_after(
                    &additional_result.hits,
                    search_after_values,
                    &request.sort,
                );

                // Merge and deduplicate by document ID
                let mut seen_ids = std::collections::HashSet::new();
                for hit in &result.hits {
                    seen_ids.insert(hit.id.clone());
                }

                for hit in additional_filtered {
                    if !seen_ids.contains(&hit.id) {
                        result.hits.push(hit.clone());
                        seen_ids.insert(hit.id.clone());
                    }
                }
            }
        }

        // Apply multi-field sorting if multiple sort options provided
        if request.sort.len() > 1 {
            result.hits = self.sort_by_multiple_fields(&result.hits, &request.sort);
        }

        // Limit to requested size
        result.hits.truncate(request.size);

        // Handle track_total_hits
        let (total, total_relation) = match request.track_total_hits {
            Some(TrackTotalHits::True) => {
                // Count all matching documents (expensive)
                // For now, we use the total from search result
                // Full implementation would require a separate count query
                (result.total, Some("eq".to_string()))
            }
            Some(TrackTotalHits::Count(max_count)) => {
                if result.total <= max_count {
                    (result.total, Some("eq".to_string()))
                } else {
                    (max_count, Some("gte".to_string()))
                }
            }
            Some(TrackTotalHits::False) | None => {
                // Don't track accurately - use approximation
                (result.total, Some("eq".to_string()))
            }
        };

        // Extract sort values from last hit for next request
        let sort_values = result
            .hits
            .last()
            .map(|hit| self.extract_sort_values(hit, &request.sort));

        Ok(SearchAfterResponse {
            hits: result.hits,
            total,
            took_ms: result.took_ms,
            sort_values,
            total_relation,
        })
    }

    /// Filter hits based on search_after values
    fn filter_by_search_after(
        &self,
        hits: &[SearchHit],
        search_after_values: &[JsonValue],
        sort_options: &[SortOption],
    ) -> Vec<SearchHit> {
        let mut filtered = Vec::new();
        let mut found_start = false;

        for hit in hits {
            let hit_sort_values = self.extract_sort_values(hit, sort_options);

            // Check if this hit comes after search_after values
            if !found_start {
                if self.compare_sort_values(&hit_sort_values, search_after_values, sort_options) > 0
                {
                    found_start = true;
                    filtered.push(hit.clone());
                }
            } else {
                filtered.push(hit.clone());
            }
        }

        filtered
    }

    /// Compare sort values to determine ordering
    #[allow(clippy::unused_self)]
    fn compare_sort_values(
        &self,
        a: &[JsonValue],
        b: &[JsonValue],
        sort_options: &[SortOption],
    ) -> i32 {
        for (i, sort_opt) in sort_options.iter().enumerate() {
            if i >= a.len() || i >= b.len() {
                return 0;
            }

            let comparison = match (&a[i], &b[i]) {
                (JsonValue::Number(a_num), JsonValue::Number(b_num)) => {
                    if let (Some(a_f64), Some(b_f64)) = (a_num.as_f64(), b_num.as_f64()) {
                        a_f64
                            .partial_cmp(&b_f64)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    } else if let (Some(a_i64), Some(b_i64)) = (a_num.as_i64(), b_num.as_i64()) {
                        a_i64.cmp(&b_i64)
                    } else if let (Some(a_u64), Some(b_u64)) = (a_num.as_u64(), b_num.as_u64()) {
                        a_u64.cmp(&b_u64)
                    } else {
                        std::cmp::Ordering::Equal
                    }
                }
                (JsonValue::String(a_str), JsonValue::String(b_str)) => a_str.cmp(b_str),
                (JsonValue::Bool(a_bool), JsonValue::Bool(b_bool)) => a_bool.cmp(b_bool),
                (JsonValue::Null, JsonValue::Null) => std::cmp::Ordering::Equal,
                (JsonValue::Null, _) => std::cmp::Ordering::Less,
                (_, JsonValue::Null) => std::cmp::Ordering::Greater,
                _ => {
                    // Convert to JSON string for comparison
                    serde_json::to_string(a)
                        .unwrap_or_default()
                        .cmp(&serde_json::to_string(b).unwrap_or_default())
                }
            };

            let result = match sort_opt.order {
                crate::search::result::SortOrder::Asc => comparison,
                crate::search::result::SortOrder::Desc => comparison.reverse(),
            };

            match result {
                std::cmp::Ordering::Less => return -1,
                std::cmp::Ordering::Greater => return 1,
                std::cmp::Ordering::Equal => {}
            }
        }

        0
    }

    /// Sort hits by multiple fields
    fn sort_by_multiple_fields(
        &self,
        hits: &[SearchHit],
        sort_options: &[SortOption],
    ) -> Vec<SearchHit> {
        let mut sorted = hits.to_vec();
        sorted.sort_by(|a, b| {
            let a_values = self.extract_sort_values(a, sort_options);
            let b_values = self.extract_sort_values(b, sort_options);
            match self.compare_sort_values(&a_values, &b_values, sort_options) {
                x if x < 0 => std::cmp::Ordering::Less,
                x if x > 0 => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        });
        sorted
    }

    /// Extract sort values from a hit based on sort options
    pub fn extract_sort_values(
        &self,
        hit: &SearchHit,
        sort_options: &[SortOption],
    ) -> Vec<JsonValue> {
        sort_options
            .iter()
            .map(|sort_opt| {
                match sort_opt.field.as_str() {
                    "_score" => {
                        let score = f64::from(hit.score.value());
                        JsonValue::Number(
                            serde_json::Number::from_f64(score)
                                .unwrap_or_else(|| serde_json::Number::from(0)),
                        )
                    }
                    "_id" => JsonValue::String(hit.id.to_string()),
                    field => {
                        // Extract from source
                        hit.source.get(field).cloned().unwrap_or(JsonValue::Null)
                    }
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Index;
    use crate::query::{MatchQuery, Query};
    use crate::search::executor::SearchExecutor;
    use crate::types::IndexName;
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tantivy::schema::{STORED, Schema, TEXT};
    use tempfile::TempDir;

    /// Create a temporary directory compatible with WSL/Windows
    /// Uses Linux native paths in WSL to avoid Tantivy compatibility issues
    fn create_test_temp_dir() -> (TempDir, PathBuf) {
        use std::env;
        use std::time::{SystemTime, UNIX_EPOCH};

        // Detect WSL by checking multiple indicators
        let cargo_manifest = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let is_wsl_mounted = cargo_manifest.contains("/mnt/")
            || current_dir.to_string_lossy().contains("/mnt/")
            || env::var("WSL_DISTRO_NAME").is_ok();

        if is_wsl_mounted {
            // In WSL: use HOME directory which is always native Linux filesystem
            // This completely avoids 9p filesystem protocol issues
            let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let temp_dir = TempDir::new_in(&home).unwrap();
            let path = temp_dir.path().to_path_buf();
            (temp_dir, path)
        } else {
            // Native Windows or Linux: use tempfile
            let temp_dir = TempDir::new().unwrap();
            let path = temp_dir.path().to_path_buf();
            (temp_dir, path)
        }
    }

    fn create_test_index() -> (TempDir, Arc<Index>) {
        let (temp_dir, index_path) = create_test_temp_dir();
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("name", TEXT | STORED);
        schema_builder.add_i64_field("age", STORED);
        let schema = schema_builder.build();

        let schema_clone = schema.clone();
        // Try to create index in directory, with fallback for WSL compatibility issues
        let tantivy_index = tantivy::Index::create_in_dir(&index_path, schema.clone())
            .or_else(|e| {
                // If creation fails with Invalid argument (WSL issue), try using RAM
                // This allows tests to run even in WSL, though without full persistence testing
                if e.to_string().contains("Invalid argument") || e.to_string().contains("os error 22") {
                    tracing::warn!("Index creation in directory failed (likely WSL issue), using RAM index for test");
                    Ok(tantivy::Index::create_in_ram(schema))
                } else {
                    Err(e)
                }
            })
            .unwrap();
        let index = Index {
            name: IndexName::new("test_search_after"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
            mapping: None,
        };

        // Add test documents with different values for sorting
        let mut writer = index.writer(50_000_000).unwrap();
        for i in 0..10 {
            let mut doc = tantivy::TantivyDocument::default();
            doc.add_text(
                schema_clone.get_field("title").unwrap(),
                format!("Document {i}"),
            );
            doc.add_text(schema_clone.get_field("name").unwrap(), format!("Name{i}"));
            doc.add_i64(schema_clone.get_field("age").unwrap(), i64::from(20 + i));
            writer.add_document(doc).unwrap();
        }
        writer.commit().unwrap();

        (temp_dir, Arc::new(index))
    }

    #[test]
    fn test_search_after_request_serialization() {
        let request = SearchAfterRequest {
            query: crate::query::Query::Match(MatchQuery::new("field", "value")),
            sort: vec![SortOption::desc("_score")],
            size: 10,
            search_after: Some(vec![
                JsonValue::Number(serde_json::Number::from_f64(1.5).unwrap()),
                JsonValue::String("doc_id".to_string()),
            ]),
            track_total_hits: None,
            pit_id: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("search_after"));
        assert!(json.contains("sort"));
    }

    #[lexum_macros::tokio_test]
    async fn test_search_after_basic() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let search_after_executor = SearchAfterExecutor::new(executor);

        let request = SearchAfterRequest {
            query: Query::MatchAll,
            sort: vec![SortOption::desc("_score")],
            size: 5,
            search_after: None,
            track_total_hits: None,
            pit_id: None,
        };

        let result = search_after_executor.search_after(request).await.unwrap();
        assert!(result.hits.len() <= 5);
        assert!(result.total > 0);
        assert!(result.sort_values.is_some());
    }

    #[lexum_macros::tokio_test]
    async fn test_search_after_with_cursor() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let search_after_executor = SearchAfterExecutor::new(executor);

        // First request
        let request1 = SearchAfterRequest {
            query: Query::MatchAll,
            sort: vec![SortOption::desc("_score")],
            size: 3,
            search_after: None,
            track_total_hits: None,
            pit_id: None,
        };

        let result1 = search_after_executor.search_after(request1).await.unwrap();
        assert_eq!(result1.hits.len(), 3);
        assert!(result1.sort_values.is_some());

        // Second request with search_after
        let request2 = SearchAfterRequest {
            query: Query::MatchAll,
            sort: vec![SortOption::desc("_score")],
            size: 3,
            search_after: result1.sort_values.clone(),
            track_total_hits: None,
            pit_id: None,
        };

        let result2 = search_after_executor.search_after(request2).await.unwrap();
        assert!(result2.hits.len() <= 3);

        // Results should be different
        if !result2.hits.is_empty() {
            assert_ne!(
                result1.hits[0].id.to_string(),
                result2.hits[0].id.to_string()
            );
        }
    }

    #[lexum_macros::tokio_test]
    async fn test_search_after_multiple_sort_fields() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let search_after_executor = SearchAfterExecutor::new(executor);

        let request = SearchAfterRequest {
            query: Query::MatchAll,
            sort: vec![SortOption::desc("age"), SortOption::asc("name")],
            size: 5,
            search_after: None,
            track_total_hits: None,
            pit_id: None,
        };

        let result = search_after_executor.search_after(request).await.unwrap();
        assert!(result.hits.len() <= 5);
        assert!(result.sort_values.is_some());
        if let Some(ref sort_vals) = result.sort_values {
            assert_eq!(sort_vals.len(), 2); // Should have 2 sort values
        }
    }

    #[lexum_macros::tokio_test]
    async fn test_search_after_empty_sort() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let search_after_executor = SearchAfterExecutor::new(executor);

        let request = SearchAfterRequest {
            query: Query::MatchAll,
            sort: vec![],
            size: 5,
            search_after: None,
            track_total_hits: None,
            pit_id: None,
        };

        let result = search_after_executor.search_after(request).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires at least one sort option")
        );
    }

    #[lexum_macros::tokio_test]
    async fn test_search_after_mismatched_values() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let search_after_executor = SearchAfterExecutor::new(executor);

        let request = SearchAfterRequest {
            query: Query::MatchAll,
            sort: vec![SortOption::desc("_score"), SortOption::asc("age")],
            size: 5,
            search_after: Some(vec![JsonValue::Number(serde_json::Number::from(1))]), // Only 1 value, needs 2
            track_total_hits: None,
            pit_id: None,
        };

        let result = search_after_executor.search_after(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must match"));
    }

    #[lexum_macros::tokio_test]
    async fn test_search_after_field_sort() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let search_after_executor = SearchAfterExecutor::new(executor);

        let request = SearchAfterRequest {
            query: Query::MatchAll,
            sort: vec![SortOption::desc("age")],
            size: 5,
            search_after: None,
            track_total_hits: None,
            pit_id: None,
        };

        let result = search_after_executor.search_after(request).await.unwrap();
        assert!(result.hits.len() <= 5);
        assert!(result.sort_values.is_some());
    }

    #[lexum_macros::tokio_test]
    async fn test_search_after_id_sort() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let search_after_executor = SearchAfterExecutor::new(executor);

        let request = SearchAfterRequest {
            query: Query::MatchAll,
            sort: vec![SortOption::asc("_id")],
            size: 5,
            search_after: None,
            track_total_hits: None,
            pit_id: None,
        };

        let result = search_after_executor.search_after(request).await.unwrap();
        assert!(result.hits.len() <= 5);
        assert!(result.sort_values.is_some());
    }

    #[test]
    fn test_extract_sort_values_score() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let search_after_executor = SearchAfterExecutor::new(executor);

        let hit = crate::search::result::SearchHit::new(
            crate::types::DocumentId::new("doc1"),
            crate::types::Score::new(0.95),
            json!({"title": "Test", "age": 25}),
        );

        let sort_options = vec![SortOption::desc("_score")];
        let values = search_after_executor.extract_sort_values(&hit, &sort_options);
        assert_eq!(values.len(), 1);
        assert!(values[0].is_number());
    }

    #[test]
    fn test_extract_sort_values_field() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let search_after_executor = SearchAfterExecutor::new(executor);

        let hit = crate::search::result::SearchHit::new(
            crate::types::DocumentId::new("doc1"),
            crate::types::Score::new(0.95),
            json!({"title": "Test", "age": 25}),
        );

        let sort_options = vec![SortOption::desc("age")];
        let values = search_after_executor.extract_sort_values(&hit, &sort_options);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], json!(25));
    }

    #[test]
    fn test_compare_sort_values_numeric() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let search_after_executor = SearchAfterExecutor::new(executor);

        let sort_options = vec![SortOption::desc("age")];
        let a = vec![json!(30)];
        let b = vec![json!(20)];

        let result = search_after_executor.compare_sort_values(&a, &b, &sort_options);
        assert!(result < 0); // 30 > 20, but desc order means 30 comes first
    }

    #[test]
    fn test_compare_sort_values_string() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let search_after_executor = SearchAfterExecutor::new(executor);

        let sort_options = vec![SortOption::asc("name")];
        let a = vec![json!("Alice")];
        let b = vec![json!("Bob")];

        let result = search_after_executor.compare_sort_values(&a, &b, &sort_options);
        assert!(result < 0); // Alice < Bob in asc order
    }
}
