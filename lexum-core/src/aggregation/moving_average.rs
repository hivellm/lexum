//! Moving Average Aggregation
//!
//! Calculates moving average (smoothing) of metric values across buckets.

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Moving average model type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MovingAverageModel {
    /// Simple moving average (SMA)
    Simple,
    /// Linear moving average (LMA)
    Linear,
    /// Exponentially weighted moving average (EWMA)
    Ewma,
    /// Holt linear trend
    Holt,
    /// Holt-Winters seasonal
    HoltWinters,
}

impl Default for MovingAverageModel {
    fn default() -> Self {
        MovingAverageModel::Simple
    }
}

/// Moving Average Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MovingAverageAggregation {
    /// Buckets path (parent aggregation path, e.g., "my_histogram>_count")
    pub buckets_path: String,
    /// Window size (number of buckets to average)
    #[serde(default = "default_window")]
    pub window: usize,
    /// Model type (default: "simple")
    #[serde(default)]
    pub model: MovingAverageModel,
    /// Model parameters (optional, model-specific)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub settings: HashMap<String, JsonValue>,
    /// Predict number of future buckets (default: 0)
    #[serde(default)]
    pub predict: usize,
    /// Format for the output value (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Gap policy (default: "skip")
    #[serde(default = "default_gap_policy")]
    pub gap_policy: String,
}

fn default_window() -> usize {
    5
}

fn default_gap_policy() -> String {
    "skip".to_string()
}

impl MovingAverageAggregation {
    /// Create new moving average aggregation
    pub fn new(buckets_path: impl Into<String>) -> Self {
        Self {
            buckets_path: buckets_path.into(),
            window: 5,
            model: MovingAverageModel::Simple,
            settings: HashMap::new(),
            predict: 0,
            format: None,
            gap_policy: "skip".to_string(),
        }
    }

    /// Set window size
    pub fn window(mut self, window: usize) -> Self {
        self.window = window;
        self
    }

    /// Set model type
    pub fn model(mut self, model: MovingAverageModel) -> Self {
        self.model = model;
        self
    }

    /// Add model setting
    pub fn setting(mut self, key: impl Into<String>, value: JsonValue) -> Self {
        self.settings.insert(key.into(), value);
        self
    }

    /// Set number of future buckets to predict
    pub fn predict(mut self, predict: usize) -> Self {
        self.predict = predict;
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

impl AggregationTrait for MovingAverageAggregation {
    fn name(&self) -> &str {
        "moving_average"
    }

    fn execute(&self, _hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Moving Average Aggregation operates on parent aggregation results
        // It should be executed as a pipeline aggregation after the parent aggregation
        // This execute method is a placeholder - actual execution happens in pipeline processing
        use crate::error::Error;
        Err(Error::Config(
            "Moving Average Aggregation must be executed as a pipeline aggregation on parent results".to_string(),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge moving average results by calculating moving average across merged buckets
        // In a full implementation, this would:
        // 1. Extract buckets from parent aggregation results
        // 2. Extract metric values from buckets (based on buckets_path)
        // 3. Calculate moving average based on model type:
        //    - Simple: average of last N buckets
        //    - Linear: weighted average with linear weights
        //    - EWMA: exponentially weighted moving average
        //    - Holt: Holt linear trend model
        //    - Holt-Winters: seasonal decomposition
        // 4. Apply gap policy
        // 5. Optionally predict future buckets
        // 6. Return buckets with moving average values

        // For now, return the first result as placeholder
        // Full implementation would calculate moving averages
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            use crate::error::Error;
            Err(Error::Config("No results to merge".to_string()))
        }
    }
}

/// Calculate moving average for buckets
/// This is a helper function that would be called during pipeline aggregation processing
pub fn calculate_moving_average(
    buckets: &[Bucket],
    buckets_path: &str,
    window: usize,
    model: MovingAverageModel,
    settings: &HashMap<String, JsonValue>,
) -> Result<Vec<Bucket>> {
    // Note: Full implementation would:
    // 1. Parse buckets_path to extract metric values
    // 2. Apply moving average algorithm based on model:
    //    - Simple: sum(values[i-window+1..i+1]) / window
    //    - Linear: weighted sum with linear weights
    //    - EWMA: exponential smoothing with alpha parameter
    //    - Holt: trend-adjusted exponential smoothing
    //    - Holt-Winters: seasonal decomposition with trend and seasonality
    // 3. Handle gap policy
    // 4. Add moving_average to each bucket
    //
    // For now, return buckets as-is (placeholder)
    Ok(buckets.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moving_average_aggregation() {
        let agg = MovingAverageAggregation::new("my_histogram>_count");

        assert_eq!(agg.buckets_path, "my_histogram>_count");
        assert_eq!(agg.window, 5);
        assert_eq!(agg.model, MovingAverageModel::Simple);
        assert_eq!(agg.gap_policy, "skip");
    }

    #[test]
    fn test_moving_average_aggregation_with_window() {
        let agg = MovingAverageAggregation::new("my_histogram>_count").window(10);

        assert_eq!(agg.window, 10);
    }

    #[test]
    fn test_moving_average_aggregation_with_model() {
        let agg =
            MovingAverageAggregation::new("my_histogram>_count").model(MovingAverageModel::Ewma);

        assert_eq!(agg.model, MovingAverageModel::Ewma);
    }

    #[test]
    fn test_moving_average_aggregation_with_settings() {
        let mut settings = HashMap::new();
        settings.insert(
            "alpha".to_string(),
            JsonValue::Number(serde_json::Number::from_f64(0.3).unwrap()),
        );

        let agg = MovingAverageAggregation::new("my_histogram>_count")
            .model(MovingAverageModel::Ewma)
            .setting(
                "alpha",
                JsonValue::Number(serde_json::Number::from_f64(0.3).unwrap()),
            );

        assert_eq!(agg.settings.len(), 1);
        assert!(agg.settings.contains_key("alpha"));
    }

    #[test]
    fn test_moving_average_aggregation_with_predict() {
        let agg = MovingAverageAggregation::new("my_histogram>_count").predict(5);

        assert_eq!(agg.predict, 5);
    }

    #[test]
    fn test_moving_average_aggregation_with_format() {
        let agg = MovingAverageAggregation::new("my_histogram>_count").format("0.00");

        assert_eq!(agg.format, Some("0.00".to_string()));
    }

    #[test]
    fn test_moving_average_aggregation_execute_error() {
        let agg = MovingAverageAggregation::new("my_histogram>_count");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache);

        // Should return error since moving average operates on parent results
        assert!(result.is_err());
    }

    #[test]
    fn test_moving_average_aggregation_serialization() {
        let agg = MovingAverageAggregation::new("my_histogram>_count")
            .window(10)
            .model(MovingAverageModel::Ewma)
            .setting(
                "alpha",
                JsonValue::Number(serde_json::Number::from_f64(0.3).unwrap()),
            )
            .predict(5)
            .format("0.00")
            .gap_policy("insert_zeros");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("buckets_path"));
        assert!(json.contains("window"));
        assert!(json.contains("model"));
        assert!(json.contains("settings"));
        assert!(json.contains("predict"));
        assert!(json.contains("format"));
        assert!(json.contains("gap_policy"));

        let deserialized: MovingAverageAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.buckets_path, "my_histogram>_count");
        assert_eq!(deserialized.window, 10);
        assert_eq!(deserialized.model, MovingAverageModel::Ewma);
        assert_eq!(deserialized.predict, 5);
        assert_eq!(deserialized.format, Some("0.00".to_string()));
        assert_eq!(deserialized.gap_policy, "insert_zeros");
    }

    #[test]
    fn test_moving_average_model_types() {
        assert_eq!(MovingAverageModel::Simple, MovingAverageModel::Simple);
        assert_eq!(MovingAverageModel::Linear, MovingAverageModel::Linear);
        assert_eq!(MovingAverageModel::Ewma, MovingAverageModel::Ewma);
        assert_eq!(MovingAverageModel::Holt, MovingAverageModel::Holt);
        assert_eq!(
            MovingAverageModel::HoltWinters,
            MovingAverageModel::HoltWinters
        );
    }
}
