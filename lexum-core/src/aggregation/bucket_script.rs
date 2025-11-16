//! Bucket Script Aggregation
//!
//! Executes a script per bucket with access to sibling aggregation results.

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Bucket Script Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BucketScriptAggregation {
    /// Script to execute per bucket
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
    /// Format for the output value (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

fn default_lang() -> String {
    "painless".to_string()
}

fn default_gap_policy() -> String {
    "skip".to_string()
}

impl BucketScriptAggregation {
    /// Create new bucket script aggregation
    pub fn new(script: impl Into<String>, buckets_path: impl Into<String>) -> Self {
        Self {
            script: script.into(),
            buckets_path: buckets_path.into(),
            params: HashMap::new(),
            lang: "painless".to_string(),
            gap_policy: "skip".to_string(),
            format: None,
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

    /// Set format for output value
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }
}

impl AggregationTrait for BucketScriptAggregation {
    fn name(&self) -> &str {
        "bucket_script"
    }

    fn execute(&self, _hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Bucket Script Aggregation operates on parent aggregation results
        // It should be executed as a pipeline aggregation after the parent aggregation
        // This execute method is a placeholder - actual execution happens in pipeline processing
        use crate::error::Error;
        Err(Error::Config(
            "Bucket Script Aggregation must be executed as a pipeline aggregation on parent results".to_string(),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge bucket script results by applying script to merged buckets
        // In a full implementation, this would:
        // 1. Extract buckets from parent aggregation results
        // 2. Execute script for each bucket with access to sibling aggregations
        // 3. Return buckets with computed values

        // For now, return the first result as placeholder
        // Full implementation would process buckets and execute scripts
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            use crate::error::Error;
            Err(Error::Config("No results to merge".to_string()))
        }
    }
}

/// Execute bucket script on buckets
/// This is a helper function that would be called during pipeline aggregation processing
pub fn execute_bucket_script(
    buckets: &[Bucket],
    script: &str,
    sibling_aggs: &HashMap<String, AggregationResult>,
    params: &HashMap<String, JsonValue>,
) -> Result<Vec<Bucket>> {
    // Note: Full implementation would:
    // 1. Parse and execute the script for each bucket
    // 2. Access sibling aggregation values (e.g., params.sibling_agg.value)
    // 3. Compute new value for each bucket
    // 4. Handle gap policy (skip or insert zeros)
    //
    // For now, return buckets as-is (placeholder)
    Ok(buckets.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_script_aggregation() {
        let agg = BucketScriptAggregation::new("params.multiplier * _value", "my_histogram");

        assert_eq!(agg.script, "params.multiplier * _value");
        assert_eq!(agg.buckets_path, "my_histogram");
        assert_eq!(agg.lang, "painless");
        assert_eq!(agg.gap_policy, "skip");
    }

    #[test]
    fn test_bucket_script_aggregation_with_params() {
        let mut params = HashMap::new();
        params.insert(
            "multiplier".to_string(),
            JsonValue::Number(serde_json::Number::from(2)),
        );

        let agg = BucketScriptAggregation::new("params.multiplier * _value", "my_histogram")
            .params(params);

        assert_eq!(agg.params.len(), 1);
        assert!(agg.params.contains_key("multiplier"));
    }

    #[test]
    fn test_bucket_script_aggregation_with_gap_policy() {
        let agg = BucketScriptAggregation::new("_value", "my_histogram").gap_policy("insert_zeros");

        assert_eq!(agg.gap_policy, "insert_zeros");
    }

    #[test]
    fn test_bucket_script_aggregation_with_format() {
        let agg = BucketScriptAggregation::new("_value", "my_histogram").format("0.00");

        assert_eq!(agg.format, Some("0.00".to_string()));
    }

    #[test]
    fn test_bucket_script_aggregation_execute_error() {
        let agg = BucketScriptAggregation::new("_value", "my_histogram");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache);

        // Should return error since bucket script operates on parent results
        assert!(result.is_err());
    }

    #[test]
    fn test_bucket_script_aggregation_serialization() {
        let mut params = HashMap::new();
        params.insert(
            "multiplier".to_string(),
            JsonValue::Number(serde_json::Number::from(2)),
        );

        let agg = BucketScriptAggregation::new("params.multiplier * _value", "my_histogram")
            .params(params)
            .gap_policy("insert_zeros")
            .format("0.00");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("script"));
        assert!(json.contains("buckets_path"));
        assert!(json.contains("params"));
        assert!(json.contains("gap_policy"));
        assert!(json.contains("format"));

        let deserialized: BucketScriptAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.script, "params.multiplier * _value");
        assert_eq!(deserialized.buckets_path, "my_histogram");
        assert_eq!(deserialized.gap_policy, "insert_zeros");
        assert_eq!(deserialized.format, Some("0.00".to_string()));
    }
}
