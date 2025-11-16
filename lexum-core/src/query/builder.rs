//! Query builder for constructing queries

use super::types::{CommonTermsQuery, ConstantScoreQuery, DisMaxQuery, MultiMatchQuery, *};

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

    /// Create a wildcard query
    pub fn wildcard_query(field: impl Into<String>, pattern: impl Into<String>) -> Query {
        Query::Wildcard(WildcardQuery::new(field, pattern))
    }

    /// Create a regex query
    pub fn regex_query(field: impl Into<String>, pattern: impl Into<String>) -> Query {
        Query::Regex(RegexQuery::new(field, pattern))
    }

    /// Create a match all query
    pub fn match_all() -> Query {
        Query::MatchAll
    }

    /// Create a multi-match query
    pub fn multi_match_query(fields: Vec<String>, query: impl Into<String>) -> Query {
        Query::MultiMatch(MultiMatchQuery::new(fields, query))
    }

    /// Create a constant score query
    pub fn constant_score_query(filter: Query) -> Query {
        Query::ConstantScore(ConstantScoreQuery::new(filter))
    }

    /// Create a dis max query
    pub fn dis_max_query(queries: Vec<Query>) -> Query {
        Query::DisMax(DisMaxQuery::new(queries))
    }

    /// Create a common terms query
    pub fn common_terms_query(field: impl Into<String>, query: impl Into<String>) -> Query {
        Query::CommonTerms(CommonTermsQuery::new(field, query))
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

    #[test]
    fn test_wildcard_query_builder() {
        let query = QueryBuilder::wildcard_query("name", "test*");
        assert!(matches!(query, Query::Wildcard(_)));
    }

    #[test]
    fn test_regex_query_builder() {
        let query = QueryBuilder::regex_query("content", "[A-Z][a-z]+");
        assert!(matches!(query, Query::Regex(_)));
    }

    #[test]
    fn test_multi_match_query_builder() {
        let query = QueryBuilder::multi_match_query(
            vec!["title".to_string(), "content".to_string()],
            "search terms",
        );
        assert!(matches!(query, Query::MultiMatch(_)));
    }

    #[test]
    fn test_constant_score_query_builder() {
        let filter = QueryBuilder::term_query("status", "active");
        let query = QueryBuilder::constant_score_query(filter);
        assert!(matches!(query, Query::ConstantScore(_)));
    }

    #[test]
    fn test_dis_max_query_builder() {
        let queries = vec![
            QueryBuilder::match_query("title", "test"),
            QueryBuilder::term_query("status", "active"),
        ];
        let query = QueryBuilder::dis_max_query(queries);
        assert!(matches!(query, Query::DisMax(_)));
    }

    #[test]
    fn test_common_terms_query_builder() {
        let query = QueryBuilder::common_terms_query("body", "bonsai cool");
        assert!(matches!(query, Query::CommonTerms(_)));
    }
}
