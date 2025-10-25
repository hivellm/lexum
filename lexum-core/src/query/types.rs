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
    /// Fuzzy query (approximate matching)
    Fuzzy(FuzzyQuery),
    /// Phrase query (exact phrase matching)
    Phrase(PhraseQuery),
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

/// Fuzzy query for approximate matching with edit distance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzyQuery {
    /// Field to search
    pub field: String,
    /// Term to match fuzzily
    pub value: String,
    /// Maximum edit distance (0-2, default: 2)
    #[serde(default = "default_fuzzy_distance")]
    pub fuzziness: u8,
    /// Whether to include transpositions in edit distance
    #[serde(default = "default_true")]
    pub transpositions: bool,
    /// Minimum prefix length that must match exactly
    #[serde(default)]
    pub prefix_length: u32,
}

fn default_fuzzy_distance() -> u8 {
    2
}

fn default_true() -> bool {
    true
}

impl FuzzyQuery {
    /// Create new fuzzy query with default fuzziness of 2
    pub fn new(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
            fuzziness: 2,
            transpositions: true,
            prefix_length: 0,
        }
    }

    /// Set fuzziness (maximum edit distance 0-2)
    pub fn fuzziness(mut self, distance: u8) -> Self {
        self.fuzziness = distance.min(2);
        self
    }

    /// Set whether to include transpositions
    pub fn transpositions(mut self, enabled: bool) -> Self {
        self.transpositions = enabled;
        self
    }

    /// Set prefix length that must match exactly
    pub fn prefix_length(mut self, length: u32) -> Self {
        self.prefix_length = length;
        self
    }
}

/// Phrase query for exact phrase matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseQuery {
    /// Field to search
    pub field: String,
    /// Phrase to match (terms in exact order)
    pub phrase: String,
    /// Maximum allowed distance between terms (slop)
    #[serde(default)]
    pub slop: u32,
}

impl PhraseQuery {
    /// Create new phrase query with no slop (exact phrase)
    pub fn new(field: impl Into<String>, phrase: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            phrase: phrase.into(),
            slop: 0,
        }
    }

    /// Set slop (maximum distance between terms)
    /// 
    /// Slop allows terms to be in different positions.
    /// For example, with slop=1, "quick fox" will match "quick brown fox"
    pub fn slop(mut self, slop: u32) -> Self {
        self.slop = slop;
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

    #[test]
    fn test_fuzzy_query() {
        let query = FuzzyQuery::new("name", "jhon")
            .fuzziness(1)
            .prefix_length(2);

        assert_eq!(query.field, "name");
        assert_eq!(query.value, "jhon");
        assert_eq!(query.fuzziness, 1);
        assert_eq!(query.prefix_length, 2);
        assert!(query.transpositions);
    }

    #[test]
    fn test_fuzzy_query_max_distance() {
        let query = FuzzyQuery::new("name", "test").fuzziness(10);
        // Should cap at 2
        assert_eq!(query.fuzziness, 2);
    }

    #[test]
    fn test_phrase_query() {
        let query = PhraseQuery::new("content", "quick brown fox");
        assert_eq!(query.field, "content");
        assert_eq!(query.phrase, "quick brown fox");
        assert_eq!(query.slop, 0);
    }

    #[test]
    fn test_phrase_query_with_slop() {
        let query = PhraseQuery::new("content", "quick fox").slop(2);
        assert_eq!(query.slop, 2);
    }
}
