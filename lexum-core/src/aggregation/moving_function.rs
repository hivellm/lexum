//! Moving Function Aggregation
//!
//! Applies a custom window function to metric values across buckets.

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Moving Function Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MovingFunctionAggregation {
    /// Buckets path (parent aggregation path, e.g., "my_histogram>_count")
    pub buckets_path: String,
    /// Script to execute on the window (required)
    pub script: String,
    /// Window size (number of buckets in the window)
    #[serde(default = "default_window")]
    pub window: usize,
    /// Script parameters (optional)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, JsonValue>,
    /// Script language (default: "painless")
    #[serde(default = "default_lang")]
    pub lang: String,
    /// Shift window (default: 0, negative values shift backward)
    #[serde(default)]
    pub shift: i32,
    /// Format for the output value (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Gap policy (default: "skip")
    #[serde(default = "default_gap_policy")]
    pub gap_policy: String,
}

fn default_window() -> usize {
    10
}

fn default_lang() -> String {
    "painless".to_string()
}

fn default_gap_policy() -> String {
    "skip".to_string()
}

impl MovingFunctionAggregation {
    /// Create new moving function aggregation
    pub fn new(script: impl Into<String>, buckets_path: impl Into<String>) -> Self {
        Self {
            script: script.into(),
            buckets_path: buckets_path.into(),
            window: 10,
            params: HashMap::new(),
            lang: "painless".to_string(),
            shift: 0,
            format: None,
            gap_policy: "skip".to_string(),
        }
    }

    /// Set window size
    pub fn window(mut self, window: usize) -> Self {
        self.window = window;
        self
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

    /// Set window shift (negative values shift backward)
    pub fn shift(mut self, shift: i32) -> Self {
        self.shift = shift;
        self
    }

    /// Set format for output value
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Set gap policy ("skip" or "insert_zeros")
    pub fn gap_policy(mut self, gap_policy: impl Into<String>) -> Self {
        self.gap_policy = gap_policy.into();
        self
    }
}

impl AggregationTrait for MovingFunctionAggregation {
    fn name(&self) -> &str {
        "moving_function"
    }

    fn execute(&self, _hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Moving Function Aggregation operates on parent aggregation results
        // It should be executed as a pipeline aggregation after the parent aggregation
        // This execute method is a placeholder - actual execution happens in pipeline processing
        use crate::error::Error;
        Err(Error::Config(
            "Moving Function Aggregation must be executed as a pipeline aggregation on parent results".to_string(),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge moving function results by applying script to merged buckets
        // In a full implementation, this would:
        // 1. Extract buckets from parent aggregation results
        // 2. Extract metric values from buckets (based on buckets_path)
        // 3. For each bucket, create a window of size `window` (shifted by `shift`)
        // 4. Execute script on the window values
        // 5. Apply gap policy
        // 6. Return buckets with computed values

        // For now, return the first result as placeholder
        // Full implementation would execute script on windows
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            use crate::error::Error;
            Err(Error::Config("No results to merge".to_string()))
        }
    }
}

/// Apply moving function to buckets
/// This is a helper function that would be called during pipeline aggregation processing
pub fn apply_moving_function(
    buckets: &[Bucket],
    _buckets_path: &str,
    _script: &str,
    _window: usize,
    _shift: i32,
    _params: &HashMap<String, JsonValue>,
) -> Result<Vec<Bucket>> {
    // Note: Full implementation would:
    // 1. Parse buckets_path to extract metric values
    // 2. For each bucket i:
    //    - Create window: buckets[i+shift-window+1..i+shift+1]
    //    - Extract values from window buckets
    //    - Execute script with window values and params
    //    - Store result in bucket
    // 3. Handle gap policy (skip or insert zeros for missing buckets)
    // 4. Add moving_function value to each bucket
    //
    // For now, return buckets as-is (placeholder)
    Ok(buckets.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moving_function_aggregation() {
        let agg = MovingFunctionAggregation::new("Math.max(values)", "my_histogram>_count");

        assert_eq!(agg.script, "Math.max(values)");
        assert_eq!(agg.buckets_path, "my_histogram>_count");
        assert_eq!(agg.window, 10);
        assert_eq!(agg.shift, 0);
        assert_eq!(agg.gap_policy, "skip");
    }

    #[test]
    fn test_moving_function_aggregation_with_window() {
        let agg =
            MovingFunctionAggregation::new("Math.max(values)", "my_histogram>_count").window(5);

        assert_eq!(agg.window, 5);
    }

    #[test]
    fn test_moving_function_aggregation_with_shift() {
        let agg =
            MovingFunctionAggregation::new("Math.max(values)", "my_histogram>_count").shift(-2);

        assert_eq!(agg.shift, -2);
    }

    #[test]
    fn test_moving_function_aggregation_with_params() {
        let mut params = HashMap::new();
        params.insert(
            "multiplier".to_string(),
            JsonValue::Number(serde_json::Number::from(2)),
        );

        let agg = MovingFunctionAggregation::new(
            "values.sum() * params.multiplier",
            "my_histogram>_count",
        )
        .params(params);

        assert_eq!(agg.params.len(), 1);
        assert!(agg.params.contains_key("multiplier"));
    }

    #[test]
    fn test_moving_function_aggregation_with_format() {
        let agg = MovingFunctionAggregation::new("Math.max(values)", "my_histogram>_count")
            .format("0.00");

        assert_eq!(agg.format, Some("0.00".to_string()));
    }

    #[test]
    fn test_moving_function_aggregation_with_gap_policy() {
        let agg = MovingFunctionAggregation::new("Math.max(values)", "my_histogram>_count")
            .gap_policy("insert_zeros");

        assert_eq!(agg.gap_policy, "insert_zeros");
    }

    #[test]
    fn test_moving_function_aggregation_execute_error() {
        let agg = MovingFunctionAggregation::new("Math.max(values)", "my_histogram>_count");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache);

        // Should return error since moving function operates on parent results
        assert!(result.is_err());
    }

    #[test]
    fn test_moving_function_aggregation_serialization() {
        let mut params = HashMap::new();
        params.insert(
            "multiplier".to_string(),
            JsonValue::Number(serde_json::Number::from(2)),
        );

        let agg = MovingFunctionAggregation::new(
            "values.sum() * params.multiplier",
            "my_histogram>_count",
        )
        .window(5)
        .shift(-1)
        .params(params)
        .lang("javascript")
        .format("0.00")
        .gap_policy("insert_zeros");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("script"));
        assert!(json.contains("buckets_path"));
        assert!(json.contains("window"));
        assert!(json.contains("shift"));
        assert!(json.contains("params"));
        assert!(json.contains("lang"));
        assert!(json.contains("format"));
        assert!(json.contains("gap_policy"));

        let deserialized: MovingFunctionAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.script, "values.sum() * params.multiplier");
        assert_eq!(deserialized.buckets_path, "my_histogram>_count");
        assert_eq!(deserialized.window, 5);
        assert_eq!(deserialized.shift, -1);
        assert_eq!(deserialized.lang, "javascript");
        assert_eq!(deserialized.format, Some("0.00".to_string()));
        assert_eq!(deserialized.gap_policy, "insert_zeros");
    }
}
