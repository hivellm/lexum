//! Aggregation result types

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Aggregation result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum AggregationResult {
    /// Bucket-based aggregation (terms, histogram, etc.)
    Buckets(BucketAggregationResult),
    /// Metric aggregation (stats, percentile, cardinality)
    Metric(MetricAggregationResult),
    /// Single bucket aggregation (filter, etc.)
    SingleBucket(SingleBucketAggregationResult),
}

/// Bucket aggregation result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum BucketAggregationResult {
    /// Array format (default)
    Array {
        /// Buckets
        buckets: Vec<Bucket>,
        /// Total number of documents
        #[serde(skip_serializing_if = "Option::is_none")]
        doc_count: Option<usize>,
    },
    /// Keyed format (key: bucket)
    Keyed {
        /// Buckets keyed by their key
        #[serde(flatten)]
        buckets: HashMap<String, Bucket>,
    },
}

impl BucketAggregationResult {
    /// Create new bucket aggregation result (array format)
    pub fn new(buckets: Vec<Bucket>) -> Self {
        Self::Array {
            buckets,
            doc_count: None,
        }
    }

    /// Create with doc count
    pub fn with_doc_count(buckets: Vec<Bucket>, doc_count: usize) -> Self {
        Self::Array {
            buckets,
            doc_count: Some(doc_count),
        }
    }

    /// Create keyed format
    pub fn new_keyed(buckets: HashMap<String, Bucket>) -> Self {
        Self::Keyed { buckets }
    }

    /// Get buckets as vector (for merging)
    pub fn buckets(&self) -> Vec<&Bucket> {
        match self {
            BucketAggregationResult::Array { buckets, .. } => buckets.iter().collect(),
            BucketAggregationResult::Keyed { buckets } => buckets.values().collect(),
        }
    }

    /// Get buckets as mutable vector (for tests)
    #[cfg(test)]
    pub fn buckets_mut(&mut self) -> &mut Vec<Bucket> {
        match self {
            BucketAggregationResult::Array { buckets, .. } => buckets,
            BucketAggregationResult::Keyed { .. } => {
                panic!("Cannot get mutable buckets from keyed format")
            }
        }
    }

    /// Get buckets as vector (for tests - converts keyed to array)
    #[cfg(test)]
    pub fn buckets_vec(&self) -> Vec<Bucket> {
        match self {
            BucketAggregationResult::Array { buckets, .. } => buckets.clone(),
            BucketAggregationResult::Keyed { buckets } => buckets.values().cloned().collect(),
        }
    }

    /// Check if this is a keyed format
    pub fn is_keyed(&self) -> bool {
        matches!(self, BucketAggregationResult::Keyed { .. })
    }
}

/// Single bucket in an aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Bucket {
    /// Bucket key
    pub key: JsonValue,
    /// Number of documents in this bucket
    pub doc_count: usize,
    /// Sub-aggregations (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregations: Option<HashMap<String, AggregationResult>>,
}

impl Bucket {
    /// Create new bucket
    pub fn new(key: JsonValue, doc_count: usize) -> Self {
        Self {
            key,
            doc_count,
            aggregations: None,
        }
    }

    /// Add sub-aggregation
    pub fn with_aggregation(mut self, name: String, result: AggregationResult) -> Self {
        if self.aggregations.is_none() {
            self.aggregations = Some(HashMap::new());
        }
        if let Some(ref mut aggs) = self.aggregations {
            aggs.insert(name, result);
        }
        self
    }
}

/// Metric aggregation result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricAggregationResult {
    /// Metric value(s)
    pub value: JsonValue,
}

impl MetricAggregationResult {
    /// Create new metric aggregation result
    pub fn new(value: JsonValue) -> Self {
        Self { value }
    }

    /// Create from f64
    pub fn from_f64(value: f64) -> Self {
        Self {
            value: JsonValue::Number(
                serde_json::Number::from_f64(value).unwrap_or_else(|| serde_json::Number::from(0)),
            ),
        }
    }

    /// Create from usize
    pub fn from_usize(value: usize) -> Self {
        Self {
            value: JsonValue::Number(serde_json::Number::from(value)),
        }
    }
}

/// Single bucket aggregation result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SingleBucketAggregationResult {
    /// Number of documents
    pub doc_count: usize,
    /// Sub-aggregations (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregations: Option<HashMap<String, AggregationResult>>,
}

impl SingleBucketAggregationResult {
    /// Create new single bucket aggregation result
    pub fn new(doc_count: usize) -> Self {
        Self {
            doc_count,
            aggregations: None,
        }
    }

    /// Add sub-aggregation
    pub fn with_aggregation(mut self, name: String, result: AggregationResult) -> Self {
        if self.aggregations.is_none() {
            self.aggregations = Some(HashMap::new());
        }
        if let Some(ref mut aggs) = self.aggregations {
            aggs.insert(name, result);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_creation() {
        let bucket = Bucket::new(JsonValue::String("test".to_string()), 10);
        assert_eq!(bucket.doc_count, 10);
        assert_eq!(bucket.key, JsonValue::String("test".to_string()));
    }

    #[test]
    fn test_metric_result() {
        let result = MetricAggregationResult::from_f64(42.5);
        assert!(result.value.is_number());
    }

    #[test]
    fn test_single_bucket_result() {
        let result = SingleBucketAggregationResult::new(5);
        assert_eq!(result.doc_count, 5);
    }
}
