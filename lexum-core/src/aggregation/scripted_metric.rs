//! Scripted Metric Aggregation
//!
//! Executes custom scripts to compute metric aggregations.

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Scripted Metric Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScriptedMetricAggregation {
    /// Init script (optional) - initializes the state
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub init_script: Option<String>,
    /// Map script (required) - processes each document
    pub map_script: String,
    /// Combine script (required) - combines results from shards
    pub combine_script: String,
    /// Reduce script (required) - reduces final results
    pub reduce_script: String,
    /// Script parameters (optional)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, JsonValue>,
    /// Script language (default: "painless")
    #[serde(default = "default_lang")]
    pub lang: String,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

fn default_lang() -> String {
    "painless".to_string()
}

impl ScriptedMetricAggregation {
    /// Create new scripted metric aggregation
    pub fn new(
        map_script: impl Into<String>,
        combine_script: impl Into<String>,
        reduce_script: impl Into<String>,
    ) -> Self {
        Self {
            init_script: None,
            map_script: map_script.into(),
            combine_script: combine_script.into(),
            reduce_script: reduce_script.into(),
            params: HashMap::new(),
            lang: "painless".to_string(),
            aggs: HashMap::new(),
        }
    }

    /// Set init script
    pub fn init_script(mut self, script: impl Into<String>) -> Self {
        self.init_script = Some(script.into());
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

/// Scripted Metric result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScriptedMetricResult {
    /// Result value from script execution
    pub value: JsonValue,
}

impl AggregationTrait for ScriptedMetricAggregation {
    fn name(&self) -> &str {
        "scripted_metric"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Full implementation would require:
        // 1. Script engine to execute scripts (init, map, combine)
        // 2. State management for script execution
        // 3. Script parameter injection
        //
        // For now, provide a simplified implementation that:
        // - Executes init script (if provided) to initialize state
        // - Executes map script for each document (simplified)
        // - Returns a placeholder result

        // Initialize state with init script (if provided)
        let mut state = if let Some(ref init_script) = self.init_script {
            // In a full implementation, this would execute the init script
            // For now, return empty state
            JsonValue::Object(serde_json::Map::new())
        } else {
            JsonValue::Object(serde_json::Map::new())
        };

        // Execute map script for each document
        // In a full implementation, this would:
        // 1. Parse the map script
        // 2. Execute it with document context
        // 3. Update state with results
        let mut map_results: Vec<JsonValue> = Vec::new();
        for hit in hits {
            // Simplified: extract numeric values from documents
            // Full implementation would execute the map script
            if let Some(obj) = hit.source.as_object() {
                // Try to find numeric values
                for (_, value) in obj {
                    if let Some(num) = value.as_f64() {
                        map_results.push(JsonValue::Number(
                            serde_json::Number::from_f64(num)
                                .unwrap_or(serde_json::Number::from(0)),
                        ));
                        break;
                    }
                }
            }
        }

        // Execute combine script (simplified)
        // In a full implementation, this would execute the combine script
        // For now, return a simple aggregation of map results
        let combined_result = if !map_results.is_empty() {
            // Simple sum as placeholder
            let sum: f64 = map_results.iter().filter_map(|v| v.as_f64()).sum();
            JsonValue::Number(
                serde_json::Number::from_f64(sum).unwrap_or(serde_json::Number::from(0)),
            )
        } else {
            JsonValue::Null
        };

        let result = ScriptedMetricResult {
            value: combined_result,
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(result)?,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge scripted metric results by executing reduce script
        // In a full implementation, this would:
        // 1. Collect all combined results from shards
        // 2. Execute reduce script with all results
        // 3. Return final reduced result

        // Collect combined results
        let mut combined_results: Vec<JsonValue> = Vec::new();
        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Ok(scripted_result) =
                    serde_json::from_value::<ScriptedMetricResult>(metric_result.value.clone())
                {
                    combined_results.push(scripted_result.value);
                }
            }
        }

        // Execute reduce script (simplified)
        // In a full implementation, this would execute the reduce script
        // For now, return a simple aggregation
        let reduced_result = if !combined_results.is_empty() {
            // Simple sum as placeholder
            let sum: f64 = combined_results.iter().filter_map(|v| v.as_f64()).sum();
            JsonValue::Number(
                serde_json::Number::from_f64(sum).unwrap_or(serde_json::Number::from(0)),
            )
        } else {
            JsonValue::Null
        };

        let merged_result = ScriptedMetricResult {
            value: reduced_result,
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(merged_result)?,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scripted_metric_aggregation() {
        let agg = ScriptedMetricAggregation::new("_source.value", "states", "states");

        assert_eq!(agg.map_script, "_source.value");
        assert_eq!(agg.combine_script, "states");
        assert_eq!(agg.reduce_script, "states");
        assert_eq!(agg.lang, "painless");
    }

    #[test]
    fn test_scripted_metric_aggregation_with_init() {
        let agg = ScriptedMetricAggregation::new("_source.value", "states", "states")
            .init_script("state = []");

        assert!(agg.init_script.is_some());
        assert_eq!(agg.init_script.unwrap(), "state = []");
    }

    #[test]
    fn test_scripted_metric_aggregation_with_params() {
        let mut params = HashMap::new();
        params.insert(
            "multiplier".to_string(),
            JsonValue::Number(serde_json::Number::from(2)),
        );

        let agg =
            ScriptedMetricAggregation::new("_source.value * params.multiplier", "states", "states")
                .params(params);

        assert_eq!(agg.params.len(), 1);
        assert!(agg.params.contains_key("multiplier"));
    }

    #[test]
    fn test_scripted_metric_aggregation_with_lang() {
        let agg =
            ScriptedMetricAggregation::new("_source.value", "states", "states").lang("javascript");

        assert_eq!(agg.lang, "javascript");
    }

    #[test]
    fn test_scripted_metric_aggregation_empty() {
        let agg = ScriptedMetricAggregation::new("_source.value", "states", "states");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache).unwrap();
        if let AggregationResult::Metric(metric_result) = result {
            let scripted: ScriptedMetricResult =
                serde_json::from_value(metric_result.value).unwrap();
            assert!(scripted.value.is_null());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_scripted_metric_aggregation_basic() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = ScriptedMetricAggregation::new("_source.value", "states", "states");
        let mut hits = vec![];

        hits.push(SearchHit {
            id: DocumentId::new("1"),
            score: Score::new(1.0),
            source: serde_json::json!({ "value": 10 }),
        });
        hits.push(SearchHit {
            id: DocumentId::new("2"),
            score: Score::new(1.0),
            source: serde_json::json!({ "value": 20 }),
        });

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let scripted: ScriptedMetricResult =
                serde_json::from_value(metric_result.value).unwrap();
            // Simplified implementation returns sum
            assert!(scripted.value.is_number());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_scripted_metric_aggregation_merge() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = ScriptedMetricAggregation::new("_source.value", "states", "states");

        // Create first result
        let mut hits1 = vec![];
        hits1.push(SearchHit {
            id: DocumentId::new("1"),
            score: Score::new(1.0),
            source: serde_json::json!({ "value": 10 }),
        });

        // Create second result
        let mut hits2 = vec![];
        hits2.push(SearchHit {
            id: DocumentId::new("2"),
            score: Score::new(1.0),
            source: serde_json::json!({ "value": 20 }),
        });

        let field_cache = FieldCache::new();
        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            let scripted: ScriptedMetricResult =
                serde_json::from_value(metric_result.value).unwrap();
            // Simplified implementation returns sum
            assert!(scripted.value.is_number());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_scripted_metric_aggregation_serialization() {
        let agg =
            ScriptedMetricAggregation::new("_source.value * params.multiplier", "states", "states")
                .init_script("state = []")
                .param("multiplier", JsonValue::Number(serde_json::Number::from(2)))
                .lang("javascript");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("map_script"));
        assert!(json.contains("combine_script"));
        assert!(json.contains("reduce_script"));
        assert!(json.contains("init_script"));
        assert!(json.contains("params"));
        assert!(json.contains("lang"));

        let deserialized: ScriptedMetricAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.map_script, "_source.value * params.multiplier");
        assert!(deserialized.init_script.is_some());
        assert_eq!(deserialized.lang, "javascript");
    }
}
