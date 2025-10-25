//! Query type definitions

use serde::{Deserialize, Serialize};

/// Main query enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Query {
    /// Match query (full-text search)
    Match(MatchQuery),
    /// Term query (exact match)
    Term(TermQuery),
    /// Range query (numeric/date ranges)
    Range(RangeQuery),
    /// Boolean query (combinations)
    Bool(BoolQuery),
    /// Match all documents
    MatchAll,
}

/// Match query for full-text search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchQuery {
    /// Field to search
    pub field: String,
    /// Query text
    pub query: String,
}

impl MatchQuery {
    /// Create new match query
    pub fn new(field: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            query: query.into(),
        }
    }
}

/// Term query for exact matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermQuery {
    /// Field to search
    pub field: String,
    /// Term value
    pub value: String,
}

impl TermQuery {
    /// Create new term query
    pub fn new(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
        }
    }
}

/// Range query for numeric/date ranges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeQuery {
    /// Field to search
    pub field: String,
    /// Greater than or equal
    pub gte: Option<serde_json::Value>,
    /// Less than or equal
    pub lte: Option<serde_json::Value>,
    /// Greater than
    pub gt: Option<serde_json::Value>,
    /// Less than
    pub lt: Option<serde_json::Value>,
}

impl RangeQuery {
    /// Create new range query
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            gte: None,
            lte: None,
            gt: None,
            lt: None,
        }
    }

    /// Set greater than or equal
    pub fn gte(mut self, value: serde_json::Value) -> Self {
        self.gte = Some(value);
        self
    }

    /// Set less than or equal
    pub fn lte(mut self, value: serde_json::Value) -> Self {
        self.lte = Some(value);
        self
    }

    /// Set greater than
    pub fn gt(mut self, value: serde_json::Value) -> Self {
        self.gt = Some(value);
        self
    }

    /// Set less than
    pub fn lt(mut self, value: serde_json::Value) -> Self {
        self.lt = Some(value);
        self
    }
}

/// Boolean query for combining queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BoolQuery {
    /// Must clauses (all required)
    #[serde(default)]
    pub must: Vec<Query>,
    /// Should clauses (at least one should match)
    #[serde(default)]
    pub should: Vec<Query>,
    /// Must not clauses (exclude)
    #[serde(default)]
    pub must_not: Vec<Query>,
    /// Filter clauses (must match, but don't affect score)
    #[serde(default)]
    pub filter: Vec<Query>,
}

impl BoolQuery {
    /// Create new boolean query
    pub fn new() -> Self {
        Self::default()
    }

    /// Add must clause
    pub fn must(mut self, query: Query) -> Self {
        self.must.push(query);
        self
    }

    /// Add should clause
    pub fn should(mut self, query: Query) -> Self {
        self.should.push(query);
        self
    }

    /// Add must_not clause
    pub fn must_not(mut self, query: Query) -> Self {
        self.must_not.push(query);
        self
    }

    /// Add filter clause
    pub fn filter(mut self, query: Query) -> Self {
        self.filter.push(query);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_query() {
        let query = MatchQuery::new("title", "search terms");
        assert_eq!(query.field, "title");
        assert_eq!(query.query, "search terms");
    }

    #[test]
    fn test_term_query() {
        let query = TermQuery::new("status", "active");
        assert_eq!(query.field, "status");
        assert_eq!(query.value, "active");
    }

    #[test]
    fn test_range_query() {
        let query = RangeQuery::new("age")
            .gte(serde_json::json!(18))
            .lte(serde_json::json!(65));

        assert_eq!(query.field, "age");
        assert!(query.gte.is_some());
        assert!(query.lte.is_some());
    }

    #[test]
    fn test_bool_query() {
        let bool_query = BoolQuery::new()
            .must(Query::Term(TermQuery::new("status", "active")))
            .should(Query::Match(MatchQuery::new("title", "test")));

        assert_eq!(bool_query.must.len(), 1);
        assert_eq!(bool_query.should.len(), 1);
    }
}
