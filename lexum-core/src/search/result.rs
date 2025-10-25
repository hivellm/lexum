//! Search result types

use crate::types::{DocumentId, Score};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

/// Sort order for search results
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// Ascending order
    Asc,
    /// Descending order
    #[default]
    Desc,
}

/// Sort option for a field
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl SearchResult {
    /// Create new search result
    pub fn new(hits: Vec<SearchHit>, total: usize, took_ms: u64) -> Self {
        Self {
            hits,
            total,
            took_ms,
        }
    }

    /// Create empty result
    pub fn empty() -> Self {
        Self {
            hits: Vec::new(),
            total: 0,
            took_ms: 0,
        }
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
}
