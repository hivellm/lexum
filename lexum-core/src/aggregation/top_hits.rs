//! Top Hits Aggregation
//!
//! Retrieves the top matching documents within aggregation buckets.

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::{SearchHit, SortOption, SortOrder};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Top Hits Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopHitsAggregation {
    /// Maximum number of hits to return (default: 10)
    #[serde(default = "default_size")]
    pub size: usize,
    /// Offset for pagination (default: 0)
    #[serde(default)]
    pub from: usize,
    /// Sort options (default: sort by score descending)
    #[serde(default)]
    pub sort: Vec<SortOption>,
    /// Fields to include in the response (if None, include all)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    /// Highlight configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight: Option<TopHitsHighlight>,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

fn default_size() -> usize {
    10
}

/// Top Hits Highlight configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopHitsHighlight {
    /// Fields to highlight
    #[serde(default)]
    pub fields: Vec<String>,
    /// Pre-tag for highlighting (default: "<em>")
    #[serde(default = "default_pre_tag")]
    pub pre_tag: String,
    /// Post-tag for highlighting (default: "</em>")
    #[serde(default = "default_post_tag")]
    pub post_tag: String,
    /// Fragment size in characters (default: 100)
    #[serde(default = "default_fragment_size")]
    pub fragment_size: usize,
    /// Maximum number of fragments (default: 3)
    #[serde(default = "default_max_fragments")]
    pub max_fragments: usize,
}

fn default_pre_tag() -> String {
    "<em>".to_string()
}

fn default_post_tag() -> String {
    "</em>".to_string()
}

fn default_fragment_size() -> usize {
    100
}

fn default_max_fragments() -> usize {
    3
}

impl TopHitsAggregation {
    /// Create new top hits aggregation
    pub fn new() -> Self {
        Self {
            size: 10,
            from: 0,
            sort: Vec::new(),
            fields: None,
            highlight: None,
            aggs: HashMap::new(),
        }
    }

    /// Set maximum number of hits to return
    pub fn size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Set offset for pagination
    pub fn from(mut self, from: usize) -> Self {
        self.from = from;
        self
    }

    /// Add sort option
    pub fn sort(mut self, sort: SortOption) -> Self {
        self.sort.push(sort);
        self
    }

    /// Set fields to include in response
    pub fn fields(mut self, fields: Vec<String>) -> Self {
        self.fields = Some(fields);
        self
    }

    /// Set highlight configuration
    pub fn highlight(mut self, highlight: TopHitsHighlight) -> Self {
        self.highlight = Some(highlight);
        self
    }

    /// Add sub-aggregation
    pub fn agg(
        mut self,
        name: impl Into<String>,
        agg: crate::aggregation::AggregationSpec,
    ) -> Self {
        self.aggs.insert(name.into(), agg);
        self
    }
}

impl Default for TopHitsAggregation {
    fn default() -> Self {
        Self::new()
    }
}

/// Top Hits result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopHitsResult {
    /// Total number of hits
    pub total: usize,
    /// Maximum score
    pub max_score: Option<f32>,
    /// Hits (documents)
    pub hits: Vec<SearchHit>,
}

impl AggregationTrait for TopHitsAggregation {
    fn name(&self) -> &str {
        "top_hits"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Start with all hits
        let mut top_hits = hits.to_vec();

        // Apply sorting
        if !self.sort.is_empty() {
            // Sort by multiple fields (first field has highest priority)
            top_hits.sort_by(|a, b| {
                for sort_opt in &self.sort {
                    let comparison = match compare_hits_by_field(a, b, &sort_opt.field) {
                        Some(ord) => ord,
                        None => continue,
                    };

                    let result = match sort_opt.order {
                        SortOrder::Asc => comparison,
                        SortOrder::Desc => comparison.reverse(),
                    };

                    if result != std::cmp::Ordering::Equal {
                        return result;
                    }
                }
                // If all sort fields are equal, maintain original order (by score)
                a.score.value().partial_cmp(&b.score.value()).unwrap()
            });
        } else {
            // Default: sort by score descending
            top_hits.sort_by(|a, b| {
                b.score
                    .value()
                    .partial_cmp(&a.score.value())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        // Apply pagination
        let total = top_hits.len();
        let paginated_hits: Vec<SearchHit> = top_hits
            .into_iter()
            .skip(self.from)
            .take(self.size)
            .collect();

        // Apply field filtering if specified
        let filtered_hits: Vec<SearchHit> = if let Some(ref fields) = self.fields {
            paginated_hits
                .into_iter()
                .map(|hit| {
                    // Filter source to only include specified fields
                    let mut filtered_source = serde_json::Map::new();
                    if let Some(obj) = hit.source.as_object() {
                        for field in fields {
                            if let Some(value) = obj.get(field) {
                                filtered_source.insert(field.clone(), value.clone());
                            }
                        }
                    }
                    SearchHit {
                        id: hit.id,
                        score: hit.score,
                        source: JsonValue::Object(filtered_source),
                    }
                })
                .collect()
        } else {
            paginated_hits
        };

        // Apply highlighting if specified
        let highlighted_hits: Vec<SearchHit> = if let Some(ref highlight_config) = self.highlight {
            filtered_hits
                .into_iter()
                .map(|hit| apply_highlighting(hit, highlight_config))
                .collect()
        } else {
            filtered_hits
        };

        // Calculate max score
        let max_score = highlighted_hits
            .iter()
            .map(|h| h.score.value())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let result = TopHitsResult {
            total,
            max_score,
            hits: highlighted_hits,
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(result)?,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge top hits results by combining all hits and re-sorting
        let mut all_hits: Vec<SearchHit> = Vec::new();

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Ok(top_hits_result) =
                    serde_json::from_value::<TopHitsResult>(metric_result.value.clone())
                {
                    all_hits.extend(top_hits_result.hits);
                }
            }
        }

        // Re-sort and paginate
        if !self.sort.is_empty() {
            all_hits.sort_by(|a, b| {
                for sort_opt in &self.sort {
                    let comparison = match compare_hits_by_field(a, b, &sort_opt.field) {
                        Some(ord) => ord,
                        None => continue,
                    };

                    let result = match sort_opt.order {
                        SortOrder::Asc => comparison,
                        SortOrder::Desc => comparison.reverse(),
                    };

                    if result != std::cmp::Ordering::Equal {
                        return result;
                    }
                }
                b.score
                    .value()
                    .partial_cmp(&a.score.value())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            all_hits.sort_by(|a, b| {
                b.score
                    .value()
                    .partial_cmp(&a.score.value())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        let total = all_hits.len();
        let paginated_hits: Vec<SearchHit> = all_hits
            .into_iter()
            .skip(self.from)
            .take(self.size)
            .collect();

        let max_score = paginated_hits
            .iter()
            .map(|h| h.score.value())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let merged_result = TopHitsResult {
            total,
            max_score,
            hits: paginated_hits,
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(merged_result)?,
        )))
    }
}

/// Compare two hits by a field
fn compare_hits_by_field(a: &SearchHit, b: &SearchHit, field: &str) -> Option<std::cmp::Ordering> {
    let value_a = a.source.get(field)?;
    let value_b = b.source.get(field)?;

    match (value_a, value_b) {
        (JsonValue::String(s1), JsonValue::String(s2)) => Some(s1.cmp(s2)),
        (JsonValue::Number(n1), JsonValue::Number(n2)) => {
            let f1 = n1.as_f64()?;
            let f2 = n2.as_f64()?;
            f1.partial_cmp(&f2)
        }
        (JsonValue::Bool(b1), JsonValue::Bool(b2)) => Some(b1.cmp(b2)),
        _ => None,
    }
}

/// Apply highlighting to a hit
/// Note: This is a simplified implementation. Full highlighting would require
/// query context and text analysis to find matching terms.
fn apply_highlighting(hit: SearchHit, config: &TopHitsHighlight) -> SearchHit {
    // Simplified highlighting: just wrap field values with tags
    // Full implementation would require:
    // 1. Query context to identify matching terms
    // 2. Text analysis to find term positions
    // 3. Fragment extraction around matches
    // 4. Proper HTML/XML escaping

    let mut highlighted_source = hit.source.clone();

    if let Some(obj) = highlighted_source.as_object_mut() {
        for field in &config.fields {
            if let Some(value) = obj.get(field) {
                if let Some(text) = value.as_str() {
                    // Simplified: just wrap the entire text
                    // Full implementation would extract fragments around matches
                    let highlighted = format!("{}{}{}", config.pre_tag, text, config.post_tag);
                    obj.insert(field.clone(), JsonValue::String(highlighted));
                }
            }
        }
    }

    SearchHit {
        id: hit.id,
        score: hit.score,
        source: highlighted_source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DocumentId, Score};

    #[test]
    fn test_top_hits_aggregation() {
        let agg = TopHitsAggregation::new();

        assert_eq!(agg.size, 10);
        assert_eq!(agg.from, 0);
        assert!(agg.sort.is_empty());
    }

    #[test]
    fn test_top_hits_aggregation_with_size() {
        let agg = TopHitsAggregation::new().size(5);

        assert_eq!(agg.size, 5);
    }

    #[test]
    fn test_top_hits_aggregation_with_from() {
        let agg = TopHitsAggregation::new().from(10);

        assert_eq!(agg.from, 10);
    }

    #[test]
    fn test_top_hits_aggregation_with_sort() {
        let agg = TopHitsAggregation::new().sort(SortOption::desc("title"));

        assert_eq!(agg.sort.len(), 1);
        assert_eq!(agg.sort[0].field, "title");
        assert_eq!(agg.sort[0].order, SortOrder::Desc);
    }

    #[test]
    fn test_top_hits_aggregation_empty() {
        let agg = TopHitsAggregation::new();
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache).unwrap();
        if let AggregationResult::Metric(metric_result) = result {
            let top_hits: TopHitsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(top_hits.total, 0);
            assert_eq!(top_hits.hits.len(), 0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_top_hits_aggregation_basic() {
        let agg = TopHitsAggregation::new().size(2);
        let mut hits = vec![];

        hits.push(SearchHit {
            id: DocumentId::new("1"),
            score: Score::new(0.9),
            source: serde_json::json!({ "title": "First" }),
        });
        hits.push(SearchHit {
            id: DocumentId::new("2"),
            score: Score::new(0.8),
            source: serde_json::json!({ "title": "Second" }),
        });
        hits.push(SearchHit {
            id: DocumentId::new("3"),
            score: Score::new(0.7),
            source: serde_json::json!({ "title": "Third" }),
        });

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let top_hits: TopHitsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(top_hits.total, 3);
            assert_eq!(top_hits.hits.len(), 2); // Limited by size
            assert_eq!(top_hits.hits[0].id.as_str(), "1"); // Highest score first
            assert_eq!(top_hits.hits[1].id.as_str(), "2");
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_top_hits_aggregation_with_pagination() {
        let agg = TopHitsAggregation::new().size(2).from(1);
        let mut hits = vec![];

        for i in 1..=5 {
            hits.push(SearchHit {
                id: DocumentId::new(&i.to_string()),
                score: Score::new(i as f32 * 0.1),
                source: serde_json::json!({ "title": format!("Title {}", i) }),
            });
        }

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let top_hits: TopHitsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(top_hits.total, 5);
            assert_eq!(top_hits.hits.len(), 2); // Size = 2
            assert_eq!(top_hits.hits[0].id.as_str(), "4"); // From = 1, so starts at index 1 (second highest)
            assert_eq!(top_hits.hits[1].id.as_str(), "3");
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_top_hits_aggregation_with_sort() {
        let agg = TopHitsAggregation::new()
            .size(3)
            .sort(SortOption::asc("title"));
        let mut hits = vec![];

        hits.push(SearchHit {
            id: DocumentId::new("1"),
            score: Score::new(0.5),
            source: serde_json::json!({ "title": "Zebra" }),
        });
        hits.push(SearchHit {
            id: DocumentId::new("2"),
            score: Score::new(0.9),
            source: serde_json::json!({ "title": "Apple" }),
        });
        hits.push(SearchHit {
            id: DocumentId::new("3"),
            score: Score::new(0.7),
            source: serde_json::json!({ "title": "Banana" }),
        });

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let top_hits: TopHitsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(top_hits.hits.len(), 3);
            assert_eq!(top_hits.hits[0].id.as_str(), "2"); // Apple (A comes first)
            assert_eq!(top_hits.hits[1].id.as_str(), "3"); // Banana
            assert_eq!(top_hits.hits[2].id.as_str(), "1"); // Zebra
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_top_hits_aggregation_with_fields() {
        let agg = TopHitsAggregation::new()
            .size(1)
            .fields(vec!["title".to_string()]);
        let mut hits = vec![];

        hits.push(SearchHit {
            id: DocumentId::new("1"),
            score: Score::new(0.9),
            source: serde_json::json!({
                "title": "Test",
                "description": "Should be filtered out",
                "author": "Also filtered"
            }),
        });

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let top_hits: TopHitsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(top_hits.hits.len(), 1);
            let source = &top_hits.hits[0].source;
            assert!(source.get("title").is_some());
            assert!(source.get("description").is_none());
            assert!(source.get("author").is_none());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_top_hits_aggregation_merge() {
        let agg = TopHitsAggregation::new().size(2);

        let mut hits1 = vec![];
        hits1.push(SearchHit {
            id: DocumentId::new("1"),
            score: Score::new(0.9),
            source: serde_json::json!({ "title": "First" }),
        });

        let mut hits2 = vec![];
        hits2.push(SearchHit {
            id: DocumentId::new("2"),
            score: Score::new(0.8),
            source: serde_json::json!({ "title": "Second" }),
        });

        let field_cache = FieldCache::new();
        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            let top_hits: TopHitsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(top_hits.total, 2);
            assert_eq!(top_hits.hits.len(), 2);
            assert_eq!(top_hits.hits[0].id.as_str(), "1"); // Highest score first
            assert_eq!(top_hits.hits[1].id.as_str(), "2");
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_top_hits_aggregation_serialization() {
        let highlight = TopHitsHighlight {
            fields: vec!["title".to_string()],
            pre_tag: "<mark>".to_string(),
            post_tag: "</mark>".to_string(),
            fragment_size: 150,
            max_fragments: 5,
        };

        let agg = TopHitsAggregation::new()
            .size(5)
            .from(10)
            .sort(SortOption::desc("title"))
            .highlight(highlight);

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("size"));
        assert!(json.contains("from"));
        assert!(json.contains("sort"));
        assert!(json.contains("highlight"));

        let deserialized: TopHitsAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.size, 5);
        assert_eq!(deserialized.from, 10);
        assert_eq!(deserialized.sort.len(), 1);
        assert!(deserialized.highlight.is_some());
    }
}
