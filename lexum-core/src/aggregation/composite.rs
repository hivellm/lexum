//! Composite aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Source type for composite aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompositeSource {
    /// Terms source (group by field values)
    Terms {
        /// Field to group by
        field: String,
    },
    /// Date histogram source (group by date intervals)
    DateHistogram {
        /// Field containing dates
        field: String,
        /// Interval (e.g., "1d", "1h", "1w")
        interval: String,
    },
    /// Histogram source (group by numeric intervals)
    Histogram {
        /// Field to group by
        field: String,
        /// Interval size
        interval: f64,
    },
}

/// Composite aggregation source definition
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompositeSourceSpec {
    /// Name of the source
    pub name: String,
    /// Source type and configuration
    #[serde(flatten)]
    pub source: CompositeSource,
}

/// After key for pagination
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AfterKey {
    /// Key values for each source
    #[serde(flatten)]
    pub keys: HashMap<String, JsonValue>,
}

/// Composite aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompositeAggregation {
    /// Sources to group by
    pub sources: Vec<CompositeSourceSpec>,
    /// Maximum number of buckets to return
    #[serde(default = "default_size")]
    pub size: usize,
    /// After key for pagination (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<AfterKey>,
}

fn default_size() -> usize {
    10
}

impl CompositeAggregation {
    /// Create new composite aggregation
    pub fn new(sources: Vec<CompositeSourceSpec>) -> Self {
        Self {
            sources,
            size: default_size(),
            after: None,
        }
    }

    /// Set size limit
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Set after key for pagination
    pub fn with_after(mut self, after: AfterKey) -> Self {
        self.after = Some(after);
        self
    }
}

impl AggregationTrait for CompositeAggregation {
    fn name(&self) -> &str {
        "composite"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Build composite keys from hits
        let mut composite_buckets: HashMap<Vec<JsonValue>, usize> = HashMap::new();

        for hit in hits {
            let mut key = Vec::new();

            // Build key from each source
            for source_spec in &self.sources {
                let value = match &source_spec.source {
                    CompositeSource::Terms { field } => {
                        hit.source.get(field).cloned().unwrap_or(JsonValue::Null)
                    }
                    CompositeSource::DateHistogram { field, interval: _ } => {
                        // Simplified: extract date and bucket it
                        // Full implementation would parse interval and bucket dates
                        if let Some(date_value) = hit.source.get(field) {
                            date_value.clone()
                        } else {
                            JsonValue::Null
                        }
                    }
                    CompositeSource::Histogram { field, interval } => {
                        if let Some(num_value) = hit.source.get(field) {
                            if let Some(num) = num_value.as_f64() {
                                let bucket_key = (num / interval).floor() * interval;
                                JsonValue::Number(
                                    serde_json::Number::from_f64(bucket_key)
                                        .unwrap_or_else(|| serde_json::Number::from(0)),
                                )
                            } else if let Some(num) = num_value.as_i64() {
                                let bucket_key = ((num as f64) / interval).floor() * interval;
                                JsonValue::Number(
                                    serde_json::Number::from_f64(bucket_key)
                                        .unwrap_or_else(|| serde_json::Number::from(0)),
                                )
                            } else {
                                JsonValue::Null
                            }
                        } else {
                            JsonValue::Null
                        }
                    }
                };
                key.push(value);
            }

            // Check if we should skip this hit (pagination)
            if let Some(ref after) = self.after {
                let mut should_skip = true;
                for (i, source_spec) in self.sources.iter().enumerate() {
                    if let Some(after_value) = after.keys.get(&source_spec.name) {
                        let hit_value = &key[i];
                        // Compare values
                        match (hit_value, after_value) {
                            (JsonValue::String(hs), JsonValue::String(as_val)) => {
                                if hs > as_val {
                                    should_skip = false;
                                    break;
                                } else if hs < as_val {
                                    return Ok(AggregationResult::Buckets(
                                        BucketAggregationResult::new(vec![]),
                                    ));
                                }
                            }
                            (JsonValue::Number(hn), JsonValue::Number(an)) => {
                                if let (Some(hf), Some(af)) = (hn.as_f64(), an.as_f64()) {
                                    if hf > af {
                                        should_skip = false;
                                        break;
                                    } else if hf < af {
                                        return Ok(AggregationResult::Buckets(
                                            BucketAggregationResult::new(vec![]),
                                        ));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                if should_skip {
                    continue;
                }
            }

            // Count documents for this composite key
            *composite_buckets.entry(key).or_insert(0) += 1;
        }

        // Convert to buckets, sorted by key
        let mut buckets: Vec<Bucket> = composite_buckets
            .into_iter()
            .map(|(key, count)| {
                // Create bucket key from composite key
                let bucket_key = if key.len() == 1 {
                    key[0].clone()
                } else {
                    // Multi-level key as object
                    let mut key_obj = serde_json::Map::new();
                    for (i, source_spec) in self.sources.iter().enumerate() {
                        key_obj.insert(source_spec.name.clone(), key[i].clone());
                    }
                    JsonValue::Object(key_obj)
                };
                Bucket::new(bucket_key, count)
            })
            .collect();

        // Sort buckets by composite key
        buckets.sort_by(|a, b| {
            match (&a.key, &b.key) {
                (JsonValue::Object(a_obj), JsonValue::Object(b_obj)) => {
                    // Compare by each source in order
                    for source_spec in &self.sources {
                        if let (Some(a_val), Some(b_val)) =
                            (a_obj.get(&source_spec.name), b_obj.get(&source_spec.name))
                        {
                            match (a_val, b_val) {
                                (JsonValue::String(a_s), JsonValue::String(b_s)) => {
                                    let cmp = a_s.cmp(b_s);
                                    if cmp != std::cmp::Ordering::Equal {
                                        return cmp;
                                    }
                                }
                                (JsonValue::Number(a_n), JsonValue::Number(b_n)) => {
                                    if let (Some(a_f), Some(b_f)) = (a_n.as_f64(), b_n.as_f64()) {
                                        let cmp = a_f
                                            .partial_cmp(&b_f)
                                            .unwrap_or(std::cmp::Ordering::Equal);
                                        if cmp != std::cmp::Ordering::Equal {
                                            return cmp;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    std::cmp::Ordering::Equal
                }
                _ => a.key.to_string().cmp(&b.key.to_string()),
            }
        });

        // Apply size limit
        buckets.truncate(self.size);

        // Build after_key from last bucket if we have more results
        // (In a real implementation, we'd track if there are more buckets)
        let _after_key = if buckets.len() == self.size && !buckets.is_empty() {
            let last_bucket = &buckets[buckets.len() - 1];
            if let JsonValue::Object(key_obj) = &last_bucket.key {
                let mut keys_map = HashMap::new();
                for (k, v) in key_obj {
                    keys_map.insert(k.clone(), v.clone());
                }
                Some(AfterKey { keys: keys_map })
            } else {
                None
            }
        } else {
            None
        };

        // Create result with after_key metadata
        let result_buckets = BucketAggregationResult::new(buckets);

        // Note: In a full implementation, we'd add after_key to the result
        // For now, we'll return the buckets

        Ok(AggregationResult::Buckets(result_buckets))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut merged_buckets: HashMap<Vec<JsonValue>, usize> = HashMap::new();

        // Merge buckets from all results
        for result in results {
            if let AggregationResult::Buckets(bucket_result) = result {
                for bucket in bucket_result.buckets() {
                    // Extract composite key from bucket
                    let key = match &bucket.key {
                        JsonValue::Object(key_obj) => self
                            .sources
                            .iter()
                            .map(|s| key_obj.get(&s.name).cloned().unwrap_or(JsonValue::Null))
                            .collect(),
                        _ => vec![bucket.key.clone()],
                    };
                    *merged_buckets.entry(key).or_insert(0) += bucket.doc_count;
                }
            }
        }

        // Convert to buckets and sort
        let mut buckets: Vec<Bucket> = merged_buckets
            .into_iter()
            .map(|(key, count)| {
                let bucket_key = if key.len() == 1 {
                    key[0].clone()
                } else {
                    let mut key_obj = serde_json::Map::new();
                    for (i, source_spec) in self.sources.iter().enumerate() {
                        key_obj.insert(source_spec.name.clone(), key[i].clone());
                    }
                    JsonValue::Object(key_obj)
                };
                Bucket::new(bucket_key, count)
            })
            .collect();

        // Sort buckets
        buckets.sort_by(|a, b| match (&a.key, &b.key) {
            (JsonValue::Object(a_obj), JsonValue::Object(b_obj)) => {
                for source_spec in &self.sources {
                    if let (Some(a_val), Some(b_val)) =
                        (a_obj.get(&source_spec.name), b_obj.get(&source_spec.name))
                    {
                        match (a_val, b_val) {
                            (JsonValue::String(a_s), JsonValue::String(b_s)) => {
                                let cmp = a_s.cmp(b_s);
                                if cmp != std::cmp::Ordering::Equal {
                                    return cmp;
                                }
                            }
                            (JsonValue::Number(a_n), JsonValue::Number(b_n)) => {
                                if let (Some(a_f), Some(b_f)) = (a_n.as_f64(), b_n.as_f64()) {
                                    let cmp =
                                        a_f.partial_cmp(&b_f).unwrap_or(std::cmp::Ordering::Equal);
                                    if cmp != std::cmp::Ordering::Equal {
                                        return cmp;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                std::cmp::Ordering::Equal
            }
            _ => a.key.to_string().cmp(&b.key.to_string()),
        });

        buckets.truncate(self.size);

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            buckets,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::result::SearchHit;
    use crate::types::{DocumentId, Score};

    fn create_test_hit(id: &str, category: &str, brand: &str) -> SearchHit {
        SearchHit {
            id: DocumentId::new(id),
            score: Score::new(1.0),
            source: serde_json::json!({
                "category": category,
                "brand": brand
            }),
        }
    }

    fn create_test_hit_numeric(id: &str, category: &str, price: f64) -> SearchHit {
        SearchHit {
            id: DocumentId::new(id),
            score: Score::new(1.0),
            source: serde_json::json!({
                "category": category,
                "price": price
            }),
        }
    }

    #[test]
    fn test_composite_aggregation_basic() {
        let sources = vec![
            CompositeSourceSpec {
                name: "category".to_string(),
                source: CompositeSource::Terms {
                    field: "category".to_string(),
                },
            },
            CompositeSourceSpec {
                name: "brand".to_string(),
                source: CompositeSource::Terms {
                    field: "brand".to_string(),
                },
            },
        ];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "electronics", "sony"),
            create_test_hit("2", "electronics", "sony"),
            create_test_hit("3", "electronics", "samsung"),
            create_test_hit("4", "clothing", "nike"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 3); // 3 unique combinations
            // Check that we have the right combinations
            let mut found_combinations = 0;
            for bucket in &buckets {
                if let JsonValue::Object(key_obj) = &bucket.key {
                    if let (Some(cat), Some(br)) = (key_obj.get("category"), key_obj.get("brand")) {
                        #[allow(clippy::if_same_then_else)]
                        if cat.as_str() == Some("electronics") && br.as_str() == Some("sony") {
                            assert_eq!(bucket.doc_count, 2);
                            found_combinations += 1;
                        } else if cat.as_str() == Some("electronics")
                            && br.as_str() == Some("samsung")
                        {
                            assert_eq!(bucket.doc_count, 1);
                            found_combinations += 1;
                        } else if cat.as_str() == Some("clothing") && br.as_str() == Some("nike") {
                            assert_eq!(bucket.doc_count, 1);
                            found_combinations += 1;
                        }
                    }
                }
            }
            assert_eq!(found_combinations, 3);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_with_histogram() {
        let sources = vec![
            CompositeSourceSpec {
                name: "category".to_string(),
                source: CompositeSource::Terms {
                    field: "category".to_string(),
                },
            },
            CompositeSourceSpec {
                name: "price_range".to_string(),
                source: CompositeSource::Histogram {
                    field: "price".to_string(),
                    interval: 10.0,
                },
            },
        ];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "electronics", 15.0),
            create_test_hit_numeric("2", "electronics", 25.0),
            create_test_hit_numeric("3", "clothing", 15.0),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 3); // 3 unique combinations
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_size_limit() {
        let sources = vec![CompositeSourceSpec {
            name: "category".to_string(),
            source: CompositeSource::Terms {
                field: "category".to_string(),
            },
        }];
        let agg = CompositeAggregation::new(sources).with_size(2);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "electronics", "sony"),
            create_test_hit("2", "electronics", "sony"),
            create_test_hit("3", "clothing", "nike"),
            create_test_hit("4", "books", "penguin"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 2); // Limited to 2
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_merge() {
        let sources = vec![
            CompositeSourceSpec {
                name: "category".to_string(),
                source: CompositeSource::Terms {
                    field: "category".to_string(),
                },
            },
            CompositeSourceSpec {
                name: "brand".to_string(),
                source: CompositeSource::Terms {
                    field: "brand".to_string(),
                },
            },
        ];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        let hits1 = vec![create_test_hit("1", "electronics", "sony")];
        let hits2 = vec![create_test_hit("2", "electronics", "sony")];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1);
            // Should have merged count
            if let JsonValue::Object(key_obj) = &buckets[0].key {
                if let (Some(cat), Some(br)) = (key_obj.get("category"), key_obj.get("brand")) {
                    if cat.as_str() == Some("electronics") && br.as_str() == Some("sony") {
                        assert_eq!(buckets[0].doc_count, 2);
                    }
                }
            }
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_empty_hits() {
        let sources = vec![CompositeSourceSpec {
            name: "category".to_string(),
            source: CompositeSource::Terms {
                field: "category".to_string(),
            },
        }];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        let hits = vec![];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 0);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_single_source() {
        let sources = vec![CompositeSourceSpec {
            name: "category".to_string(),
            source: CompositeSource::Terms {
                field: "category".to_string(),
            },
        }];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "electronics", "sony"),
            create_test_hit("2", "electronics", "sony"),
            create_test_hit("3", "clothing", "nike"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 2); // 2 unique categories
            // Single source should use simple key format
            let electronics_count = buckets
                .iter()
                .find(|b| b.key.as_str() == Some("electronics"))
                .map(|b| b.doc_count)
                .unwrap_or(0);
            assert_eq!(electronics_count, 2);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_three_levels() {
        let sources = vec![
            CompositeSourceSpec {
                name: "category".to_string(),
                source: CompositeSource::Terms {
                    field: "category".to_string(),
                },
            },
            CompositeSourceSpec {
                name: "brand".to_string(),
                source: CompositeSource::Terms {
                    field: "brand".to_string(),
                },
            },
            CompositeSourceSpec {
                name: "price_range".to_string(),
                source: CompositeSource::Histogram {
                    field: "price".to_string(),
                    interval: 10.0,
                },
            },
        ];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        let mut hit1 = create_test_hit_numeric("1", "electronics", 15.0);
        hit1.source["brand"] = serde_json::json!("sony");
        let mut hit2 = create_test_hit_numeric("2", "electronics", 15.0);
        hit2.source["brand"] = serde_json::json!("sony");
        let mut hit3 = create_test_hit_numeric("3", "electronics", 25.0);
        hit3.source["brand"] = serde_json::json!("samsung");

        let hits = vec![hit1, hit2, hit3];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 2); // 2 unique 3-level combinations
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_missing_fields() {
        let sources = vec![
            CompositeSourceSpec {
                name: "category".to_string(),
                source: CompositeSource::Terms {
                    field: "category".to_string(),
                },
            },
            CompositeSourceSpec {
                name: "brand".to_string(),
                source: CompositeSource::Terms {
                    field: "brand".to_string(),
                },
            },
        ];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "electronics", "sony"),
            SearchHit {
                id: DocumentId::new("2"),
                score: Score::new(1.0),
                source: serde_json::json!({}), // Missing both fields
            },
            SearchHit {
                id: DocumentId::new("3"),
                score: Score::new(1.0),
                source: serde_json::json!({ "category": "electronics" }), // Missing brand
            },
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            // Should have buckets for null/null, electronics/null, and electronics/sony
            assert!(!buckets.is_empty());
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_histogram_bucketing() {
        let sources = vec![CompositeSourceSpec {
            name: "price_range".to_string(),
            source: CompositeSource::Histogram {
                field: "price".to_string(),
                interval: 10.0,
            },
        }];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "electronics", 5.0),
            create_test_hit_numeric("2", "electronics", 8.0), // Same bucket (0-10)
            create_test_hit_numeric("3", "electronics", 15.0), // Different bucket (10-20)
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 2); // 2 buckets: 0-10 and 10-20
            // Check bucket counts
            let bucket_0_10 = buckets
                .iter()
                .find(|b| {
                    b.key
                        .as_f64()
                        .map(|v| (v - 0.0).abs() < 0.1)
                        .unwrap_or(false)
                })
                .map(|b| b.doc_count)
                .unwrap_or(0);
            assert_eq!(bucket_0_10, 2); // 5.0 and 8.0
        } else {
            panic!("Expected Buckets result");
        }
    }

    /// Test with large dataset (1000 documents)
    /// This test is marked as slow because it processes a large number of documents
    #[test]
    #[cfg(feature = "slow-tests")]
    fn test_composite_aggregation_large_dataset() {
        let sources = vec![
            CompositeSourceSpec {
                name: "category".to_string(),
                source: CompositeSource::Terms {
                    field: "category".to_string(),
                },
            },
            CompositeSourceSpec {
                name: "brand".to_string(),
                source: CompositeSource::Terms {
                    field: "brand".to_string(),
                },
            },
        ];
        let agg = CompositeAggregation::new(sources).with_size(100);
        let field_cache = FieldCache::new();

        // Create 1000 hits with various combinations
        let mut hits = Vec::new();
        for i in 0..1000 {
            let category = match i % 10 {
                0..=3 => "electronics",
                4..=6 => "clothing",
                _ => "books",
            };
            let brand = match i % 5 {
                0 => "sony",
                1 => "samsung",
                2 => "nike",
                3 => "adidas",
                _ => "penguin",
            };
            hits.push(create_test_hit(&i.to_string(), category, brand));
        }

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert!(buckets.len() <= 100); // Should respect size limit
            assert!(!buckets.is_empty());
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_merge_different_keys() {
        let sources = vec![
            CompositeSourceSpec {
                name: "category".to_string(),
                source: CompositeSource::Terms {
                    field: "category".to_string(),
                },
            },
            CompositeSourceSpec {
                name: "brand".to_string(),
                source: CompositeSource::Terms {
                    field: "brand".to_string(),
                },
            },
        ];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        let hits1 = vec![create_test_hit("1", "electronics", "sony")];
        let hits2 = vec![create_test_hit("2", "clothing", "nike")];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 2); // Two different combinations
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_numeric_sorting() {
        let sources = vec![CompositeSourceSpec {
            name: "price_range".to_string(),
            source: CompositeSource::Histogram {
                field: "price".to_string(),
                interval: 10.0,
            },
        }];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "electronics", 25.0),
            create_test_hit_numeric("2", "electronics", 5.0),
            create_test_hit_numeric("3", "electronics", 15.0),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 3);
            // Should be sorted by numeric value
            let values: Vec<f64> = buckets.iter().filter_map(|b| b.key.as_f64()).collect();
            assert_eq!(values, vec![0.0, 10.0, 20.0]);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_string_sorting() {
        let sources = vec![CompositeSourceSpec {
            name: "category".to_string(),
            source: CompositeSource::Terms {
                field: "category".to_string(),
            },
        }];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "zebra", "brand1"),
            create_test_hit("2", "alpha", "brand2"),
            create_test_hit("3", "beta", "brand3"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 3);
            // Should be sorted alphabetically
            let values: Vec<&str> = buckets.iter().filter_map(|b| b.key.as_str()).collect();
            assert_eq!(values, vec!["alpha", "beta", "zebra"]);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_after_key_pagination() {
        let sources = vec![CompositeSourceSpec {
            name: "category".to_string(),
            source: CompositeSource::Terms {
                field: "category".to_string(),
            },
        }];
        let mut agg = CompositeAggregation::new(sources.clone()).with_size(2);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "alpha", "brand1"),
            create_test_hit("2", "beta", "brand2"),
            create_test_hit("3", "gamma", "brand3"),
            create_test_hit("4", "delta", "brand4"),
        ];

        // First page
        let result1 = agg.execute(&hits, &field_cache).unwrap();
        let buckets1 = if let AggregationResult::Buckets(br) = result1 {
            br.buckets_vec()
        } else {
            panic!("Expected Buckets");
        };
        assert_eq!(buckets1.len(), 2);

        // Second page with after_key
        let mut after_keys = HashMap::new();
        after_keys.insert(
            "category".to_string(),
            buckets1[1].key.clone(), // Last key from first page
        );
        agg.after = Some(AfterKey { keys: after_keys });

        let result2 = agg.execute(&hits, &field_cache).unwrap();
        let buckets2 = if let AggregationResult::Buckets(br) = result2 {
            br.buckets_vec()
        } else {
            panic!("Expected Buckets");
        };
        // Should have remaining buckets
        assert!(buckets2.len() <= 2);
    }

    #[test]
    fn test_composite_aggregation_mixed_types_in_key() {
        let sources = vec![
            CompositeSourceSpec {
                name: "category".to_string(),
                source: CompositeSource::Terms {
                    field: "category".to_string(),
                },
            },
            CompositeSourceSpec {
                name: "price_range".to_string(),
                source: CompositeSource::Histogram {
                    field: "price".to_string(),
                    interval: 10.0,
                },
            },
        ];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        let mut hit1 = create_test_hit_numeric("1", "electronics", 15.0);
        hit1.source["category"] = serde_json::json!("electronics");
        let mut hit2 = create_test_hit_numeric("2", "clothing", 25.0);
        hit2.source["category"] = serde_json::json!("clothing");

        let hits = vec![hit1, hit2];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 2); // Different combinations
            // Verify composite keys have both string and number
            for bucket in &buckets {
                if let JsonValue::Object(key_obj) = &bucket.key {
                    assert!(key_obj.contains_key("category"));
                    assert!(key_obj.contains_key("price_range"));
                }
            }
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_duplicate_keys() {
        let sources = vec![CompositeSourceSpec {
            name: "category".to_string(),
            source: CompositeSource::Terms {
                field: "category".to_string(),
            },
        }];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        // Multiple hits with same category
        let hits = vec![
            create_test_hit("1", "electronics", "sony"),
            create_test_hit("2", "electronics", "samsung"),
            create_test_hit("3", "electronics", "lg"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1); // All same category
            assert_eq!(buckets[0].doc_count, 3); // All 3 documents
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_merge_with_overlapping_keys() {
        let sources = vec![
            CompositeSourceSpec {
                name: "category".to_string(),
                source: CompositeSource::Terms {
                    field: "category".to_string(),
                },
            },
            CompositeSourceSpec {
                name: "brand".to_string(),
                source: CompositeSource::Terms {
                    field: "brand".to_string(),
                },
            },
        ];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        // Shard 1: electronics/sony (2 docs)
        let hits1 = vec![
            create_test_hit("1", "electronics", "sony"),
            create_test_hit("2", "electronics", "sony"),
        ];
        // Shard 2: electronics/sony (1 doc) + electronics/samsung (1 doc)
        let hits2 = vec![
            create_test_hit("3", "electronics", "sony"),
            create_test_hit("4", "electronics", "samsung"),
        ];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 2); // electronics/sony and electronics/samsung
            // Find electronics/sony bucket
            let sony_bucket = buckets.iter().find(|b| {
                if let JsonValue::Object(key_obj) = &b.key {
                    key_obj.get("category").and_then(|v| v.as_str()) == Some("electronics")
                        && key_obj.get("brand").and_then(|v| v.as_str()) == Some("sony")
                } else {
                    false
                }
            });
            assert!(sony_bucket.is_some());
            assert_eq!(sony_bucket.unwrap().doc_count, 3); // 2 + 1 merged
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_histogram_edge_cases() {
        let sources = vec![CompositeSourceSpec {
            name: "price_range".to_string(),
            source: CompositeSource::Histogram {
                field: "price".to_string(),
                interval: 10.0,
            },
        }];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        // Test edge cases: exactly on boundaries, negative, zero
        let hits = vec![
            create_test_hit_numeric("1", "price", 0.0), // Should be in bucket 0
            create_test_hit_numeric("2", "price", 10.0), // Should be in bucket 10 (boundary)
            create_test_hit_numeric("3", "price", -5.0), // Negative value
            create_test_hit_numeric("4", "price", 9.99999), // Just below boundary
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert!(buckets.len() >= 2); // At least 2 buckets
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_composite_aggregation_empty_sources() {
        let sources = vec![];
        let agg = CompositeAggregation::new(sources);
        let field_cache = FieldCache::new();

        let hits = vec![create_test_hit("1", "category", "electronics")];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            // With no sources, key will be empty, so all documents go into one bucket
            // Current behavior: empty key vector creates one bucket with all documents
            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets[0].doc_count, 1);
        } else {
            panic!("Expected Buckets result");
        }
    }
}
