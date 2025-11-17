//! Parent Aggregation
//!
//! Aggregates parent documents that have child documents.

use super::AggregationTrait;
use super::result::{AggregationResult, BucketAggregationResult};
use crate::error::Result;
use crate::query::Query;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Parent Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParentAggregation {
    /// Join field type (e.g., "question_answer")
    pub r#type: String,
    /// Query to filter parent documents (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<Query>,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

impl ParentAggregation {
    /// Create new parent aggregation
    pub fn new(r#type: impl Into<String>) -> Self {
        Self {
            r#type: r#type.into(),
            query: None,
            aggs: HashMap::new(),
        }
    }

    /// Set query to filter parent documents
    pub fn query(mut self, query: Query) -> Self {
        self.query = Some(query);
        self
    }

    /// Add sub-aggregation
    pub fn sub_aggregation(
        mut self,
        name: impl Into<String>,
        agg: crate::aggregation::AggregationSpec,
    ) -> Self {
        self.aggs.insert(name.into(), agg);
        self
    }
}

impl AggregationTrait for ParentAggregation {
    fn name(&self) -> &str {
        "parent"
    }

    fn execute(&self, hits: &[SearchHit], field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Parent Aggregation requires join field support in Tantivy
        // This is a placeholder implementation
        // Full implementation would:
        // 1. Identify child documents from hits
        // 2. Find parent documents for each child using join field
        // 3. Apply query filter if specified
        // 4. Execute sub-aggregations on parent documents
        // 5. Return aggregated results

        // For now, return empty buckets as placeholder
        let buckets = vec![];
        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            buckets,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge parent aggregation results
        // In a full implementation, this would merge parent document aggregations
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            use crate::error::Error;
            Err(Error::Config("No results to merge".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::TermQuery;

    #[test]
    fn test_parent_aggregation() {
        let agg = ParentAggregation::new("question_answer");

        assert_eq!(agg.r#type, "question_answer");
    }

    #[test]
    fn test_parent_aggregation_with_query() {
        let query = Query::Term(TermQuery::new("status", "active"));
        let agg = ParentAggregation::new("question_answer").query(query);

        assert!(agg.query.is_some());
    }

    #[test]
    fn test_parent_aggregation_serialization() {
        let agg = ParentAggregation::new("question_answer");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("type"));
        assert!(json.contains("question_answer"));

        let deserialized: ParentAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.r#type, "question_answer");
    }
}
