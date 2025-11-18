//! Bucket Selector Aggregation
//!
//! Filters buckets based on a script condition.

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Bucket Selector Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BucketSelectorAggregation {
    /// Script condition to evaluate (must return boolean)
    pub script: String,
    /// Buckets path (parent aggregation path)
    pub buckets_path: String,
    /// Script parameters (optional)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, JsonValue>,
    /// Script language (default: "painless")
    #[serde(default = "default_lang")]
    pub lang: String,
    /// Gap policy (default: "skip")
    #[serde(default = "default_gap_policy")]
    pub gap_policy: String,
}

fn default_lang() -> String {
    "painless".to_string()
}

fn default_gap_policy() -> String {
    "skip".to_string()
}

impl BucketSelectorAggregation {
    /// Create new bucket selector aggregation
    pub fn new(script: impl Into<String>, buckets_path: impl Into<String>) -> Self {
        Self {
            script: script.into(),
            buckets_path: buckets_path.into(),
            params: HashMap::new(),
            lang: "painless".to_string(),
            gap_policy: "skip".to_string(),
        }
    }

    /// Set script parameters
    pub fn params(mut self, params: HashMap<String, JsonValue>) -> Self {
        self.params = params;
        self
    }

    /// Add a parameter
    pub fn param(mut self, key: impl Into<String>, value: JsonValue) -> Self {
        self.params.insert(key.into(), value);
        self
    }

    /// Set script language
    pub fn lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = lang.into();
        self
    }

    /// Set gap policy ("skip" or "insert_zeros")
    pub fn gap_policy(mut self, gap_policy: impl Into<String>) -> Self {
        self.gap_policy = gap_policy.into();
        self
    }
}

impl AggregationTrait for BucketSelectorAggregation {
    fn name(&self) -> &str {
        "bucket_selector"
    }

    fn execute(&self, _hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Bucket Selector Aggregation operates on parent aggregation results
        // It should be executed as a pipeline aggregation after the parent aggregation
        // This execute method is a placeholder - actual execution happens in pipeline processing
        use crate::error::Error;
        Err(Error::Config(
            "Bucket Selector Aggregation must be executed as a pipeline aggregation on parent results".to_string(),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge bucket selector results by filtering buckets based on script condition
        // In a full implementation, this would:
        // 1. Extract buckets from parent aggregation results
        // 2. Execute script condition for each bucket
        // 3. Filter out buckets where condition returns false
        // 4. Return filtered buckets

        // For now, return the first result as placeholder
        // Full implementation would filter buckets based on script condition
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            use crate::error::Error;
            Err(Error::Config("No results to merge".to_string()))
        }
    }
}

/// Filter buckets based on script condition
/// This is a helper function that would be called during pipeline aggregation processing
pub fn filter_buckets_by_script(
    buckets: &[Bucket],
    _script: &str,
    _sibling_aggs: &HashMap<String, AggregationResult>,
    _params: &HashMap<String, JsonValue>,
) -> Result<Vec<Bucket>> {
    // Note: Full implementation would:
    // 1. Parse and execute the script condition for each bucket
    // 2. Access bucket values and sibling aggregation values
    // 3. Evaluate condition (must return boolean)
    // 4. Filter out buckets where condition is false
    //
    // For now, return all buckets (placeholder)
    Ok(buckets.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_selector_aggregation() {
        let agg = BucketSelectorAggregation::new("params.min_value > 0", "my_histogram");

        assert_eq!(agg.script, "params.min_value > 0");
        assert_eq!(agg.buckets_path, "my_histogram");
        assert_eq!(agg.lang, "painless");
        assert_eq!(agg.gap_policy, "skip");
    }

    #[test]
    fn test_bucket_selector_aggregation_with_params() {
        let mut params = HashMap::new();
        params.insert(
            "min_value".to_string(),
            JsonValue::Number(serde_json::Number::from(10)),
        );

        let agg =
            BucketSelectorAggregation::new("params.min_value > 0", "my_histogram").params(params);

        assert_eq!(agg.params.len(), 1);
        assert!(agg.params.contains_key("min_value"));
    }

    #[test]
    fn test_bucket_selector_aggregation_with_gap_policy() {
        let agg =
            BucketSelectorAggregation::new("_value > 0", "my_histogram").gap_policy("insert_zeros");

        assert_eq!(agg.gap_policy, "insert_zeros");
    }

    #[test]
    fn test_bucket_selector_aggregation_execute_error() {
        let agg = BucketSelectorAggregation::new("_value > 0", "my_histogram");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache);

        // Should return error since bucket selector operates on parent results
        assert!(result.is_err());
    }

    #[test]
    fn test_bucket_selector_aggregation_serialization() {
        let mut params = HashMap::new();
        params.insert(
            "min_value".to_string(),
            JsonValue::Number(serde_json::Number::from(10)),
        );

        let agg =
            BucketSelectorAggregation::new("params.min_value > 0 && _value > 0", "my_histogram")
                .params(params)
                .gap_policy("insert_zeros")
                .lang("javascript");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("script"));
        assert!(json.contains("buckets_path"));
        assert!(json.contains("params"));
        assert!(json.contains("gap_policy"));
        assert!(json.contains("lang"));

        let deserialized: BucketSelectorAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.script, "params.min_value > 0 && _value > 0");
        assert_eq!(deserialized.buckets_path, "my_histogram");
        assert_eq!(deserialized.gap_policy, "insert_zeros");
        assert_eq!(deserialized.lang, "javascript");
    }
}
