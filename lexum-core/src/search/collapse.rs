//! Collapse - Field collapsing for search results

use crate::error::Result;
use crate::query::Query;
use crate::search::executor::SearchExecutor;
use crate::search::inner_hits::{InnerHitsConfig, InnerHitsResult};
use crate::search::result::{SearchHit, SortOption};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

/// Collapse configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CollapseConfig {
    /// Field to collapse on
    pub field: String,
    /// Inner hits configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_hits: Option<InnerHitsConfig>,
    /// Maximum number of inner hits per collapsed group
    #[serde(default = "default_max_inner_hits")]
    pub max_concurrent_group_searches: usize,
}

fn default_max_inner_hits() -> usize {
    3
}

/// Collapsed hit with inner hits
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CollapsedHit {
    /// Main hit (first document in collapsed group)
    #[serde(flatten)]
    pub hit: SearchHit,
    /// Inner hits (other documents in collapsed group)
    #[serde(rename = "inner_hits", skip_serializing_if = "Option::is_none")]
    pub inner_hits: Option<HashMap<String, InnerHitsResult>>,
}

/// Collapse request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CollapseRequest {
    /// Query to execute
    pub query: Query,
    /// Collapse configuration
    pub collapse: CollapseConfig,
    /// Size (number of collapsed groups)
    #[serde(default = "default_size")]
    pub size: usize,
    /// Offset
    #[serde(default)]
    pub from: usize,
    /// Sort options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<SortOption>,
    /// Expand collapse results (return all documents in collapsed groups)
    #[serde(default)]
    pub expand: bool,
    /// Expand specific group by collapse field value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand_group: Option<String>,
}

fn default_size() -> usize {
    10
}

/// Collapse response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CollapseResponse {
    /// Collapsed hits (or expanded hits if expand=true)
    pub hits: Vec<CollapsedHit>,
    /// Total number of collapsed groups
    pub total: usize,
    /// Time taken in milliseconds
    pub took_ms: u64,
    /// Expanded hits (if expand=true, contains all documents from collapsed groups)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded_hits: Option<Vec<SearchHit>>,
}

/// Collapse executor
pub struct CollapseExecutor {
    executor: Arc<SearchExecutor>,
}

impl CollapseExecutor {
    /// Create new collapse executor
    pub fn new(executor: Arc<SearchExecutor>) -> Self {
        Self { executor }
    }

    /// Execute search with field collapsing
    pub async fn collapse(&self, request: CollapseRequest) -> Result<CollapseResponse> {
        // Execute initial search with higher limit to get more results for collapsing
        let search_size = request.size * 2; // Get more results to account for collapsing
        let result = self
            .executor
            .search(
                request.query.clone(),
                search_size,
                request.from,
                request.sort.clone(),
            )
            .await?;

        // Group hits by collapse field
        let mut groups: HashMap<String, Vec<SearchHit>> = HashMap::new();

        for hit in result.hits {
            let collapse_value = Self::extract_field_value(&hit.source, &request.collapse.field);
            let key = collapse_value.unwrap_or_else(|| "null".to_string());
            groups.entry(key).or_default().push(hit);
        }

        // Store total groups count before consuming groups
        let total_groups = groups.len();

        // Create collapsed hits
        let mut collapsed_hits = Vec::new();
        for (_, mut group_hits) in groups {
            if group_hits.is_empty() {
                continue;
            }

            // Sort group hits by score (descending)
            group_hits.sort_by(|a, b| {
                b.score
                    .value()
                    .partial_cmp(&a.score.value())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // First hit is the main hit
            let main_hit = group_hits.remove(0);

            // Store total inner hits count before consuming group_hits
            let total_inner_hits = group_hits.len();

            // Remaining hits are inner hits
            let inner_hits = if let Some(ref inner_config) = request.collapse.inner_hits {
                let inner_hits_size = inner_config.size.min(total_inner_hits);

                // Apply sorting to inner hits if specified
                let inner_hits_list = if let Some(ref sort_opts) = inner_config.sort {
                    // Sort inner hits according to configuration
                    let mut sorted = group_hits.clone();
                    sorted.sort_by(|a, b| Self::compare_hits_by_sort_options(a, b, sort_opts));
                    sorted.into_iter().take(inner_hits_size).collect()
                } else {
                    // Default: keep original order (already sorted by score)
                    group_hits.into_iter().take(inner_hits_size).collect()
                };

                // Convert SearchHit to InnerHit using InnerHitsProcessor
                use crate::search::inner_hits::InnerHitsProcessor;
                let inner_hits_result = InnerHitsProcessor::process_inner_hits(
                    inner_hits_list,
                    inner_config,
                    None, // No highlighter for now
                )?;

                if !inner_hits_result.hits.is_empty() {
                    let mut inner_map = HashMap::new();
                    inner_map.insert(inner_config.name.clone(), inner_hits_result);
                    Some(inner_map)
                } else {
                    None
                }
            } else {
                None
            };

            collapsed_hits.push(CollapsedHit {
                hit: main_hit,
                inner_hits,
            });
        }

        // Sort collapsed hits by main hit score
        collapsed_hits.sort_by(|a, b| {
            b.hit
                .score
                .value()
                .partial_cmp(&a.hit.score.value())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply size limit
        collapsed_hits.truncate(request.size);

        // Handle expand functionality
        let expanded_hits = if request.expand {
            // Return all documents from collapsed groups as a flat list
            Some(
                collapsed_hits
                    .iter()
                    .flat_map(|collapsed| {
                        let mut hits = vec![collapsed.hit.clone()];
                        if let Some(ref inner_hits_map) = collapsed.inner_hits {
                            for inner_hits_result in inner_hits_map.values() {
                                // Convert InnerHit back to SearchHit for expanded results
                                for inner_hit in &inner_hits_result.hits {
                                    hits.push(SearchHit::new(
                                        crate::types::DocumentId::new(&inner_hit.id),
                                        crate::types::Score::new(inner_hit.score),
                                        inner_hit.source.clone(),
                                    ));
                                }
                            }
                        }
                        hits
                    })
                    .collect(),
            )
        } else {
            request.expand_group.as_ref().map(|expand_group_value| {
                // Expand only specific group
                collapsed_hits
                    .iter()
                    .filter_map(|collapsed| {
                        let collapse_value = Self::extract_field_value(
                            &collapsed.hit.source,
                            &request.collapse.field,
                        );
                        if collapse_value
                            .as_ref()
                            .map(|v| v == expand_group_value)
                            .unwrap_or(false)
                        {
                            let mut hits = vec![collapsed.hit.clone()];
                            if let Some(ref inner_hits_map) = collapsed.inner_hits {
                                for inner_hits_result in inner_hits_map.values() {
                                    // Convert InnerHit back to SearchHit for expanded results
                                    for inner_hit in &inner_hits_result.hits {
                                        hits.push(SearchHit::new(
                                            crate::types::DocumentId::new(&inner_hit.id),
                                            crate::types::Score::new(inner_hit.score),
                                            inner_hit.source.clone(),
                                        ));
                                    }
                                }
                            }
                            Some(hits)
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect()
            })
        };

        Ok(CollapseResponse {
            hits: collapsed_hits,
            total: total_groups,
            took_ms: result.took_ms,
            expanded_hits,
        })
    }

    /// Extract field value from document source
    fn extract_field_value(source: &JsonValue, field: &str) -> Option<String> {
        if let JsonValue::Object(map) = source {
            if let Some(value) = map.get(field) {
                return Some(value.to_string().trim_matches('"').to_string());
            }
        }
        None
    }

    /// Compare hits by sort options
    fn compare_hits_by_sort_options(
        a: &SearchHit,
        b: &SearchHit,
        sort_options: &[SortOption],
    ) -> std::cmp::Ordering {
        for sort_opt in sort_options {
            let comparison = match sort_opt.field.as_str() {
                "_score" => a
                    .score
                    .value()
                    .partial_cmp(&b.score.value())
                    .unwrap_or(std::cmp::Ordering::Equal),
                "_id" => a.id.to_string().cmp(&b.id.to_string()),
                field => {
                    let a_val = a.source.get(field);
                    let b_val = b.source.get(field);
                    match (a_val, b_val) {
                        (Some(a), Some(b)) => {
                            // Try numeric comparison first
                            if let (Some(a_num), Some(b_num)) = (a.as_i64(), b.as_i64()) {
                                a_num.cmp(&b_num)
                            } else if let (Some(a_num), Some(b_num)) = (a.as_f64(), b.as_f64()) {
                                a_num
                                    .partial_cmp(&b_num)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            } else {
                                // String comparison
                                a.to_string().cmp(&b.to_string())
                            }
                        }
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                }
            };

            let result = match sort_opt.order {
                crate::search::result::SortOrder::Asc => comparison,
                crate::search::result::SortOrder::Desc => comparison.reverse(),
            };

            if result != std::cmp::Ordering::Equal {
                return result;
            }
        }
        std::cmp::Ordering::Equal
    }

    /// Apply source filtering to hits
    #[allow(dead_code)]
    fn apply_source_filter(hits: Vec<SearchHit>, _source_filter: &JsonValue) -> Vec<SearchHit> {
        // If source_filter is an object with "includes" or "excludes", filter fields
        // For now, we'll return hits as-is since source filtering is complex
        // Full implementation would require field-level filtering
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Index;
    use crate::query::Query;
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
        schema_builder.add_text_field("category", TEXT | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_i64_field("price", STORED);
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
            name: IndexName::new("test_collapse"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
            mapping: None,
        };

        // Add test documents with same category for collapsing
        let mut writer = index.writer(50_000_000).unwrap();
        for i in 0..10 {
            let mut doc = tantivy::TantivyDocument::default();
            doc.add_text(
                schema_clone.get_field("category").unwrap(),
                if i < 5 { "electronics" } else { "books" },
            );
            doc.add_text(
                schema_clone.get_field("title").unwrap(),
                format!("Item {i}"),
            );
            doc.add_i64(
                schema_clone.get_field("price").unwrap(),
                i64::from(100 + i * 10),
            );
            writer.add_document(doc).unwrap();
        }
        writer.commit().unwrap();

        (temp_dir, Arc::new(index))
    }

    #[test]
    fn test_collapse_config_serialization() {
        let config = CollapseConfig {
            field: "category".to_string(),
            inner_hits: Some(InnerHitsConfig {
                name: "variants".to_string(),
                size: 5,
                sort: None,
                source: None,
                highlight: None,
                from: 0,
            }),
            max_concurrent_group_searches: 10,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("field"));
        assert!(json.contains("inner_hits"));
    }

    #[test]
    fn test_extract_field_value() {
        let source = json!({
            "category": "electronics",
            "price": 100
        });

        assert_eq!(
            CollapseExecutor::extract_field_value(&source, "category"),
            Some("electronics".to_string())
        );
        assert_eq!(
            CollapseExecutor::extract_field_value(&source, "price"),
            Some("100".to_string())
        );
        assert_eq!(
            CollapseExecutor::extract_field_value(&source, "missing"),
            None
        );
    }

    #[lexum_macros::tokio_test]
    async fn test_collapse_basic() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let collapse_executor = CollapseExecutor::new(executor);

        let request = CollapseRequest {
            query: Query::MatchAll,
            collapse: CollapseConfig {
                field: "category".to_string(),
                inner_hits: None,
                max_concurrent_group_searches: 10,
            },
            size: 10,
            from: 0,
            sort: None,
            expand: false,
            expand_group: None,
        };

        let result = collapse_executor.collapse(request).await.unwrap();
        // Should have 2 collapsed groups (electronics and books)
        assert_eq!(result.total, 2);
        assert_eq!(result.hits.len(), 2);
    }

    #[lexum_macros::tokio_test]
    async fn test_collapse_with_inner_hits() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let collapse_executor = CollapseExecutor::new(executor);

        let request = CollapseRequest {
            query: Query::MatchAll,
            collapse: CollapseConfig {
                field: "category".to_string(),
                inner_hits: Some(InnerHitsConfig {
                    name: "variants".to_string(),
                    size: 3,
                    sort: None,
                    source: None,
                    highlight: None,
                    from: 0,
                }),
                max_concurrent_group_searches: 10,
            },
            size: 10,
            from: 0,
            sort: None,
            expand: false,
            expand_group: None,
        };

        let result = collapse_executor.collapse(request).await.unwrap();
        assert_eq!(result.total, 2);
        assert!(result.hits.iter().any(|h| h.inner_hits.is_some()));
    }

    #[lexum_macros::tokio_test]
    async fn test_collapse_expand() {
        let (_temp_dir, index) = create_test_index();
        let executor = Arc::new(SearchExecutor::new(index));
        let collapse_executor = CollapseExecutor::new(executor);

        let request = CollapseRequest {
            query: Query::MatchAll,
            collapse: CollapseConfig {
                field: "category".to_string(),
                inner_hits: Some(InnerHitsConfig {
                    name: "variants".to_string(),
                    size: 10, // Large enough to include all documents in each group
                    sort: None,
                    source: None,
                    highlight: None,
                    from: 0,
                }),
                max_concurrent_group_searches: 10,
            },
            size: 10,
            from: 0,
            sort: None,
            expand: true,
            expand_group: None,
        };

        let result = collapse_executor.collapse(request).await.unwrap();
        assert!(result.expanded_hits.is_some());
        if let Some(ref expanded) = result.expanded_hits {
            // Should have all 10 documents expanded
            assert_eq!(expanded.len(), 10);
        }
    }
}
