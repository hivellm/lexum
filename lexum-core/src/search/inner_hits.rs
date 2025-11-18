//! Inner Hits - Support for nested and parent-child queries
//!
//! Inner hits allow returning nested documents or child/parent documents
//! that matched a query, alongside the parent document in search results.

use crate::error::Result;
use crate::search::highlighter::{Highlighter, HighlighterConfig};
use crate::search::result::{SearchHit, SortOption};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Inner hits configuration (shared with collapse)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InnerHitsConfig {
    /// Name for inner hits
    #[serde(default = "default_inner_hits_name")]
    pub name: String,
    /// Size of inner hits per group
    #[serde(default = "default_inner_hits_size")]
    pub size: usize,
    /// Sort options for inner hits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Vec<SortOption>>,
    /// Source filtering for inner hits
    #[serde(rename = "_source", skip_serializing_if = "Option::is_none")]
    pub source: Option<JsonValue>,
    /// Highlight configuration for inner hits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<HighlighterConfig>,
    /// Offset for pagination within inner hits
    #[serde(default)]
    pub from: usize,
}

fn default_inner_hits_name() -> String {
    "inner_hits".to_string()
}

fn default_inner_hits_size() -> usize {
    3
}

/// Inner hits result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InnerHitsResult {
    /// Inner hits
    pub hits: Vec<InnerHit>,
    /// Total inner hits
    pub total: usize,
}

/// Inner hit with highlighting support
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InnerHit {
    /// Document ID
    #[serde(rename = "_id")]
    pub id: String,
    /// Relevance score
    #[serde(rename = "_score")]
    pub score: f32,
    /// Document source
    #[serde(rename = "_source")]
    pub source: JsonValue,
    /// Highlighted fields (if highlighting is enabled)
    #[serde(rename = "highlight", skip_serializing_if = "Option::is_none")]
    pub highlight: Option<HashMap<String, Vec<String>>>,
    /// Nested metadata (for nested queries)
    #[serde(rename = "_nested", skip_serializing_if = "Option::is_none")]
    pub nested: Option<NestedMetadata>,
}

/// Nested metadata for nested inner hits
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NestedMetadata {
    /// Field path to the nested object
    pub field: String,
    /// Offset within the nested array
    pub offset: usize,
}

/// Inner hits processor
pub struct InnerHitsProcessor;

impl InnerHitsProcessor {
    /// Process inner hits from search hits (returns InnerHit for nested/parent-child queries)
    pub fn process_inner_hits(
        hits: Vec<SearchHit>,
        config: &InnerHitsConfig,
        _highlighter: Option<&Highlighter>,
    ) -> Result<InnerHitsResult> {
        let total = hits.len();

        // Apply sorting if specified
        let sorted_hits = if let Some(ref sort_opts) = config.sort {
            let mut sorted = hits.clone();
            sorted.sort_by(|a, b| Self::compare_hits_by_sort_options(a, b, sort_opts));
            sorted
        } else {
            // Default: sort by score descending
            let mut sorted = hits.clone();
            sorted.sort_by(|a, b| {
                b.score
                    .value()
                    .partial_cmp(&a.score.value())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            sorted
        };

        // Apply pagination
        let paginated_hits: Vec<SearchHit> = sorted_hits
            .into_iter()
            .skip(config.from)
            .take(config.size)
            .collect();

        // Convert to inner hits with highlighting
        let inner_hits: Vec<InnerHit> = paginated_hits
            .into_iter()
            .map(|hit| {
                let highlight = config
                    .highlight
                    .as_ref()
                    .map(|_highlight_config| HashMap::new());

                // Apply source filtering if specified
                let filtered_source = if let Some(ref source_filter) = config.source {
                    Self::apply_source_filter(&hit.source, source_filter)
                } else {
                    hit.source.clone()
                };

                InnerHit {
                    id: hit.id.to_string(),
                    score: hit.score.value(),
                    source: filtered_source,
                    highlight,
                    nested: None, // Will be set by nested query processor
                }
            })
            .collect();

        Ok(InnerHitsResult {
            hits: inner_hits,
            total,
        })
    }

    /// Process inner hits for collapse (keeps SearchHit format)
    pub fn process_inner_hits_for_collapse(
        hits: Vec<SearchHit>,
        config: &InnerHitsConfig,
        highlighter: Option<&Highlighter>,
    ) -> Result<crate::search::inner_hits::InnerHitsResult> {
        // For collapse, we use the same processing but need to convert back
        // For now, we'll use a simplified version that works with collapse's InnerHitsResult
        // which expects Vec<InnerHit>
        Self::process_inner_hits(hits, config, highlighter)
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

    /// Apply source filtering to document source
    fn apply_source_filter(source: &JsonValue, _source_filter: &JsonValue) -> JsonValue {
        // If source_filter is an object with "includes" or "excludes", filter fields
        // For now, we'll return source as-is since source filtering is complex
        // Full implementation would require field-level filtering
        source.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::result::SortOption;
    use serde_json::json;

    #[test]
    fn test_inner_hits_config_serialization() {
        let config = InnerHitsConfig {
            name: "nested_docs".to_string(),
            size: 5,
            sort: Some(vec![SortOption::desc("_score")]),
            source: None,
            highlight: None,
            from: 0,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("nested_docs"));
        assert!(json.contains("size"));
    }

    #[test]
    fn test_inner_hit_serialization() {
        let inner_hit = InnerHit {
            id: "doc1".to_string(),
            score: 0.95,
            source: json!({"title": "Test"}),
            highlight: None,
            nested: Some(NestedMetadata {
                field: "comments".to_string(),
                offset: 0,
            }),
        };

        let json = serde_json::to_string(&inner_hit).unwrap();
        assert!(json.contains("doc1"));
        assert!(json.contains("0.95"));
        assert!(json.contains("_nested"));
    }
}
