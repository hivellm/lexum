//! Search result types

use crate::aggregation::AggregationResult;
use crate::types::{DocumentId, Score};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Sort order for search results
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// Ascending order
    Asc,
    /// Descending order
    #[default]
    Desc,
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortOrder::Asc => write!(f, "asc"),
            SortOrder::Desc => write!(f, "desc"),
        }
    }
}

/// Sort option for a field
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SortOption {
    /// Field to sort by
    pub field: String,
    /// Sort order
    #[serde(default)]
    pub order: SortOrder,
}

impl SortOption {
    /// Create new sort option
    pub fn new(field: impl Into<String>, order: SortOrder) -> Self {
        Self {
            field: field.into(),
            order,
        }
    }

    /// Create ascending sort
    pub fn asc(field: impl Into<String>) -> Self {
        Self::new(field, SortOrder::Asc)
    }

    /// Create descending sort
    pub fn desc(field: impl Into<String>) -> Self {
        Self::new(field, SortOrder::Desc)
    }
}

/// Search hit containing a document and metadata
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchHit {
    /// Document ID
    pub id: DocumentId,
    /// Relevance score
    pub score: Score,
    /// Document source
    pub source: JsonValue,
    /// Inner hits (for nested/parent-child queries)
    #[serde(rename = "inner_hits", skip_serializing_if = "Option::is_none")]
    pub inner_hits: Option<HashMap<String, crate::search::inner_hits::InnerHitsResult>>,
}

impl SearchHit {
    /// Create new search hit
    pub fn new(id: DocumentId, score: Score, source: JsonValue) -> Self {
        Self {
            id,
            score,
            source,
            inner_hits: None,
        }
    }

    /// Create new search hit with inner hits
    pub fn with_inner_hits(
        id: DocumentId,
        score: Score,
        source: JsonValue,
        inner_hits: HashMap<String, crate::search::inner_hits::InnerHitsResult>,
    ) -> Self {
        Self {
            id,
            score,
            source,
            inner_hits: Some(inner_hits),
        }
    }
}

/// Search result containing hits and metadata
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchResult {
    /// Hits (matching documents)
    pub hits: Vec<SearchHit>,
    /// Total number of hits
    pub total: usize,
    /// Time taken in milliseconds
    pub took_ms: u64,
    /// Aggregation results (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregations: Option<HashMap<String, AggregationResult>>,
}

impl SearchResult {
    /// Create new search result
    pub fn new(hits: Vec<SearchHit>, total: usize, took_ms: u64) -> Self {
        Self {
            hits,
            total,
            took_ms,
            aggregations: None,
        }
    }

    /// Create empty result
    pub fn empty() -> Self {
        Self {
            hits: Vec::new(),
            total: 0,
            took_ms: 0,
            aggregations: None,
        }
    }

    /// Add aggregation results
    pub fn with_aggregations(mut self, aggregations: HashMap<String, AggregationResult>) -> Self {
        self.aggregations = Some(aggregations);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_hit() {
        let hit = SearchHit {
            id: DocumentId::new("doc1"),
            score: Score::new(0.95_f32),
            source: serde_json::json!({"title": "Test"}),
            inner_hits: None,
        };

        assert_eq!(hit.id.as_str(), "doc1");
        assert_eq!(hit.score.value(), 0.95_f32);
    }

    #[test]
    fn test_search_result() {
        let result = SearchResult::new(vec![], 0, 10);
        assert_eq!(result.total, 0);
        assert_eq!(result.took_ms, 10);

        let empty = SearchResult::empty();
        assert_eq!(empty.total, 0);
    }

    #[test]
    fn test_search_result_with_hits() {
        let hits = vec![
            SearchHit::new(
                DocumentId::new("doc1"),
                Score::new(0.95_f32),
                serde_json::json!({"title": "Test 1"}),
            ),
            SearchHit::new(
                DocumentId::new("doc2"),
                Score::new(0.85_f32),
                serde_json::json!({"title": "Test 2"}),
            ),
        ];

        let result = SearchResult::new(hits.clone(), 2, 15);
        assert_eq!(result.hits.len(), 2);
        assert_eq!(result.total, 2);
        assert_eq!(result.took_ms, 15);
        assert_eq!(result.hits[0].id.as_str(), "doc1");
        assert_eq!(result.hits[1].id.as_str(), "doc2");
    }

    #[test]
    fn test_sort_order() {
        assert_eq!(SortOrder::Asc, SortOrder::Asc);
        assert_eq!(SortOrder::Desc, SortOrder::Desc);
        assert_ne!(SortOrder::Asc, SortOrder::Desc);
    }

    #[test]
    fn test_sort_order_display() {
        assert_eq!(format!("{}", SortOrder::Asc), "asc");
        assert_eq!(format!("{}", SortOrder::Desc), "desc");
    }

    #[test]
    fn test_sort_order_default() {
        let default: SortOrder = Default::default();
        assert_eq!(default, SortOrder::Desc);
    }

    #[test]
    fn test_sort_order_serialization() {
        let asc_json = serde_json::to_string(&SortOrder::Asc).unwrap();
        assert_eq!(asc_json, "\"asc\"");

        let desc_json = serde_json::to_string(&SortOrder::Desc).unwrap();
        assert_eq!(desc_json, "\"desc\"");

        let deserialized_asc: SortOrder = serde_json::from_str(&asc_json).unwrap();
        assert_eq!(deserialized_asc, SortOrder::Asc);

        let deserialized_desc: SortOrder = serde_json::from_str(&desc_json).unwrap();
        assert_eq!(deserialized_desc, SortOrder::Desc);
    }

    #[test]
    fn test_sort_option_new() {
        let option = SortOption::new("title", SortOrder::Asc);
        assert_eq!(option.field, "title");
        assert_eq!(option.order, SortOrder::Asc);
    }

    #[test]
    fn test_sort_option_asc() {
        let option = SortOption::asc("title");
        assert_eq!(option.field, "title");
        assert_eq!(option.order, SortOrder::Asc);
    }

    #[test]
    fn test_sort_option_desc() {
        let option = SortOption::desc("title");
        assert_eq!(option.field, "title");
        assert_eq!(option.order, SortOrder::Desc);
    }

    #[test]
    fn test_sort_option_default_order() {
        let option = SortOption {
            field: "title".to_string(),
            order: Default::default(),
        };
        assert_eq!(option.order, SortOrder::Desc);
    }

    #[test]
    fn test_sort_option_serialization() {
        let option = SortOption::new("title", SortOrder::Asc);
        let json = serde_json::to_string(&option).unwrap();
        assert!(json.contains("title"));
        assert!(json.contains("asc"));

        let deserialized: SortOption = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "title");
        assert_eq!(deserialized.order, SortOrder::Asc);
    }

    #[test]
    fn test_search_hit_serialization() {
        let hit = SearchHit {
            id: DocumentId::new("doc1"),
            score: Score::new(0.95_f32),
            source: serde_json::json!({"title": "Test"}),
            inner_hits: None,
        };

        let json = serde_json::to_string(&hit).unwrap();
        assert!(json.contains("doc1"));
        assert!(json.contains("0.95"));
        assert!(json.contains("title"));

        let deserialized: SearchHit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id.as_str(), "doc1");
        assert_eq!(deserialized.score.value(), 0.95_f32);
    }

    #[test]
    fn test_search_result_serialization() {
        let hits = vec![SearchHit::new(
            DocumentId::new("doc1"),
            Score::new(0.95_f32),
            serde_json::json!({"title": "Test"}),
        )];

        let result = SearchResult::new(hits, 1, 10);
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("hits"));
        assert!(json.contains("total"));
        assert!(json.contains("took_ms"));

        let deserialized: SearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total, 1);
        assert_eq!(deserialized.took_ms, 10);
        assert_eq!(deserialized.hits.len(), 1);
    }
}
