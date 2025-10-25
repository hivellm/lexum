//! Query builder for constructing queries

use super::types::*;

/// Builder for creating queries
pub struct QueryBuilder;

impl QueryBuilder {
    /// Create a match query
    pub fn match_query(field: impl Into<String>, query: impl Into<String>) -> Query {
        Query::Match(MatchQuery::new(field, query))
    }

    /// Create a term query
    pub fn term_query(field: impl Into<String>, value: impl Into<String>) -> Query {
        Query::Term(TermQuery::new(field, value))
    }

    /// Create a range query
    pub fn range_query(field: impl Into<String>) -> RangeQuery {
        RangeQuery::new(field)
    }

    /// Create a boolean query
    pub fn bool_query() -> BoolQuery {
        BoolQuery::new()
    }

    /// Create a fuzzy query
    pub fn fuzzy_query(field: impl Into<String>, value: impl Into<String>) -> Query {
        Query::Fuzzy(FuzzyQuery::new(field, value))
    }

    /// Create a phrase query
    pub fn phrase_query(field: impl Into<String>, phrase: impl Into<String>) -> Query {
        Query::Phrase(PhraseQuery::new(field, phrase))
    }

    /// Create a match all query
    pub fn match_all() -> Query {
        Query::MatchAll
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::match_query("title", "search");
        assert!(matches!(query, Query::Match(_)));

        let query = QueryBuilder::term_query("status", "active");
        assert!(matches!(query, Query::Term(_)));

        let query = QueryBuilder::match_all();
        assert!(matches!(query, Query::MatchAll));
    }

    #[test]
    fn test_fuzzy_query_builder() {
        let query = QueryBuilder::fuzzy_query("name", "jhon");
        assert!(matches!(query, Query::Fuzzy(_)));
    }

    #[test]
    fn test_phrase_query_builder() {
        let query = QueryBuilder::phrase_query("content", "quick brown fox");
        assert!(matches!(query, Query::Phrase(_)));
    }
}
