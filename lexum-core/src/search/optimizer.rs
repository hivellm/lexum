//! Query optimization and analysis

use crate::error::Result;
use crate::query::Query;
use std::collections::HashSet;

/// Query optimizer for improving search performance
pub struct QueryOptimizer {
    /// Maximum query depth to prevent infinite recursion
    max_depth: usize,
    /// Whether to enable query caching
    #[allow(dead_code)]
    enable_caching: bool,
}

impl QueryOptimizer {
    /// Create new query optimizer with default settings
    pub fn new() -> Self {
        Self {
            max_depth: 10,
            enable_caching: true,
        }
    }

    /// Create new query optimizer with custom settings
    pub fn with_settings(max_depth: usize, enable_caching: bool) -> Self {
        Self {
            max_depth,
            enable_caching,
        }
    }

    /// Optimize a query for better performance
    pub fn optimize(&self, query: Query) -> Result<Query> {
        self.optimize_recursive(query, 0)
    }

    /// Recursively optimize query with depth tracking
    fn optimize_recursive(&self, query: Query, depth: usize) -> Result<Query> {
        if depth > self.max_depth {
            return Err(crate::error::Error::Config(
                "Query too deep for optimization".to_string(),
            ));
        }

        match query {
            Query::Bool(mut bool_query) => {
                // Optimize boolean query sub-clauses
                bool_query.must = bool_query
                    .must
                    .into_iter()
                    .map(|q| self.optimize_recursive(q, depth + 1))
                    .collect::<Result<Vec<_>>>()?;

                bool_query.should = bool_query
                    .should
                    .into_iter()
                    .map(|q| self.optimize_recursive(q, depth + 1))
                    .collect::<Result<Vec<_>>>()?;

                bool_query.must_not = bool_query
                    .must_not
                    .into_iter()
                    .map(|q| self.optimize_recursive(q, depth + 1))
                    .collect::<Result<Vec<_>>>()?;

                bool_query.filter = bool_query
                    .filter
                    .into_iter()
                    .map(|q| self.optimize_recursive(q, depth + 1))
                    .collect::<Result<Vec<_>>>()?;

                // Remove empty boolean queries
                if bool_query.must.is_empty()
                    && bool_query.should.is_empty()
                    && bool_query.must_not.is_empty()
                    && bool_query.filter.is_empty()
                {
                    return Ok(Query::MatchAll);
                }

                // Optimize single-clause boolean queries
                if bool_query.must.len() == 1
                    && bool_query.should.is_empty()
                    && bool_query.must_not.is_empty()
                    && bool_query.filter.is_empty()
                {
                    return Ok(bool_query.must.into_iter().next().unwrap());
                }

                Ok(Query::Bool(bool_query))
            }

            Query::Nested(nested_query) => {
                let optimized_query = self.optimize_recursive(*nested_query.query, depth + 1)?;
                Ok(Query::Nested(crate::query::types::NestedQuery {
                    path: nested_query.path,
                    query: Box::new(optimized_query),
                    score_mode: nested_query.score_mode,
                }))
            }

            Query::FunctionScore(func_score_query) => {
                let optimized_query =
                    self.optimize_recursive(*func_score_query.query, depth + 1)?;
                Ok(Query::FunctionScore(
                    crate::query::types::FunctionScoreQuery {
                        query: Box::new(optimized_query),
                        functions: func_score_query.functions,
                        score_mode: func_score_query.score_mode,
                        boost_mode: func_score_query.boost_mode,
                        max_boost: func_score_query.max_boost,
                        min_score: func_score_query.min_score,
                    },
                ))
            }

            // For other query types, return as-is
            other => Ok(other),
        }
    }

    /// Analyze query complexity and provide recommendations
    pub fn analyze(&self, query: &Query) -> QueryAnalysis {
        let mut analysis = QueryAnalysis::new();
        self.analyze_recursive(query, &mut analysis, 0);
        analysis
    }

    /// Recursively analyze query complexity
    fn analyze_recursive(&self, query: &Query, analysis: &mut QueryAnalysis, depth: usize) {
        analysis.max_depth = analysis.max_depth.max(depth);
        analysis.total_clauses += 1;

        match query {
            Query::Bool(bool_query) => {
                analysis.boolean_clauses += bool_query.must.len()
                    + bool_query.should.len()
                    + bool_query.must_not.len()
                    + bool_query.filter.len();

                for clause in &bool_query.must {
                    self.analyze_recursive(clause, analysis, depth + 1);
                }
                for clause in &bool_query.should {
                    self.analyze_recursive(clause, analysis, depth + 1);
                }
                for clause in &bool_query.must_not {
                    self.analyze_recursive(clause, analysis, depth + 1);
                }
                for clause in &bool_query.filter {
                    self.analyze_recursive(clause, analysis, depth + 1);
                }
            }

            Query::Fuzzy(fuzzy_query) => {
                analysis.fuzzy_queries += 1;
                analysis.unique_fields.insert(fuzzy_query.field.clone());
            }

            Query::Phrase(phrase_query) => {
                analysis.phrase_queries += 1;
                analysis.unique_fields.insert(phrase_query.field.clone());
            }

            Query::Regex(regex_query) => {
                analysis.regex_queries += 1;
                analysis.unique_fields.insert(regex_query.field.clone());
            }

            Query::Wildcard(wildcard_query) => {
                analysis.wildcard_queries += 1;
                analysis.unique_fields.insert(wildcard_query.field.clone());
            }

            Query::MoreLikeThis(mlt_query) => {
                analysis.more_like_this_queries += 1;
                for field in &mlt_query.fields {
                    analysis.unique_fields.insert(field.clone());
                }
            }

            Query::Nested(nested_query) => {
                analysis.nested_queries += 1;
                self.analyze_recursive(nested_query.query.as_ref(), analysis, depth + 1);
            }

            Query::FunctionScore(func_score_query) => {
                analysis.function_score_queries += 1;
                self.analyze_recursive(func_score_query.query.as_ref(), analysis, depth + 1);
            }

            _ => {
                if let Some(field) = Self::get_query_field(query) {
                    analysis.unique_fields.insert(field);
                }
            }
        }
    }

    /// Extract field name from query if possible
    fn get_query_field(query: &Query) -> Option<String> {
        match query {
            Query::Match(match_query) => Some(match_query.field.clone()),
            Query::Term(term_query) => Some(term_query.field.clone()),
            Query::Range(range_query) => Some(range_query.field.clone()),
            _ => None,
        }
    }
}

impl Default for QueryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Analysis results for query optimization
#[derive(Debug, Clone)]
pub struct QueryAnalysis {
    /// Maximum query depth
    pub max_depth: usize,
    /// Total number of query clauses
    pub total_clauses: usize,
    /// Number of boolean clauses
    pub boolean_clauses: usize,
    /// Number of fuzzy queries
    pub fuzzy_queries: usize,
    /// Number of phrase queries
    pub phrase_queries: usize,
    /// Number of regex queries
    pub regex_queries: usize,
    /// Number of wildcard queries
    pub wildcard_queries: usize,
    /// Number of More Like This queries
    pub more_like_this_queries: usize,
    /// Number of nested queries
    pub nested_queries: usize,
    /// Number of function score queries
    pub function_score_queries: usize,
    /// Unique fields referenced in the query
    pub unique_fields: HashSet<String>,
}

impl QueryAnalysis {
    /// Create new empty analysis
    pub fn new() -> Self {
        Self {
            max_depth: 0,
            total_clauses: 0,
            boolean_clauses: 0,
            fuzzy_queries: 0,
            phrase_queries: 0,
            regex_queries: 0,
            wildcard_queries: 0,
            more_like_this_queries: 0,
            nested_queries: 0,
            function_score_queries: 0,
            unique_fields: HashSet::new(),
        }
    }

    /// Get query complexity score (higher = more complex)
    pub fn complexity_score(&self) -> usize {
        self.total_clauses * 2
            + self.boolean_clauses
            + self.fuzzy_queries * 3
            + self.phrase_queries * 2
            + self.regex_queries * 4
            + self.wildcard_queries * 3
            + self.more_like_this_queries * 5
            + self.nested_queries * 3
            + self.function_score_queries * 4
    }

    /// Check if query is complex (score > 20)
    pub fn is_complex(&self) -> bool {
        self.complexity_score() > 20
    }

    /// Get optimization recommendations
    pub fn recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        if self.max_depth > 5 {
            recommendations
                .push("Consider reducing query depth to improve performance".to_string());
        }

        if self.boolean_clauses > 10 {
            recommendations.push("Consider simplifying boolean query structure".to_string());
        }

        if self.fuzzy_queries > 3 {
            recommendations.push("Consider reducing number of fuzzy queries".to_string());
        }

        if self.regex_queries > 2 {
            recommendations.push("Regex queries are expensive, consider alternatives".to_string());
        }

        if self.wildcard_queries > 5 {
            recommendations.push("Consider using more specific wildcard patterns".to_string());
        }

        if self.unique_fields.len() > 10 {
            recommendations
                .push("Query spans many fields, consider field-specific queries".to_string());
        }

        if self.more_like_this_queries > 0 {
            recommendations
                .push("More Like This queries are expensive, consider caching".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Query is well-optimized".to_string());
        }

        recommendations
    }
}

impl Default for QueryAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::types::*;

    #[test]
    fn test_query_optimizer_creation() {
        let optimizer = QueryOptimizer::new();
        assert_eq!(optimizer.max_depth, 10);
        assert!(optimizer.enable_caching);
    }

    #[test]
    fn test_simple_query_optimization() {
        let optimizer = QueryOptimizer::new();
        let query = Query::Match(MatchQuery::new("title", "test"));
        let optimized = optimizer.optimize(query.clone()).unwrap();

        // Simple queries should remain unchanged
        assert!(matches!(optimized, Query::Match(_)));
    }

    #[test]
    fn test_boolean_query_optimization() {
        let optimizer = QueryOptimizer::new();
        let query = Query::Bool(
            BoolQuery::new()
                .must(Query::Match(MatchQuery::new("title", "test")))
                .should(Query::Term(TermQuery::new("status", "active"))),
        );

        let optimized = optimizer.optimize(query).unwrap();
        assert!(matches!(optimized, Query::Bool(_)));
    }

    #[test]
    fn test_empty_boolean_query_optimization() {
        let optimizer = QueryOptimizer::new();
        let query = Query::Bool(BoolQuery::new());
        let optimized = optimizer.optimize(query).unwrap();

        // Empty boolean queries should become MatchAll
        assert!(matches!(optimized, Query::MatchAll));
    }

    #[test]
    fn test_single_clause_boolean_optimization() {
        let optimizer = QueryOptimizer::new();
        let inner_query = Query::Match(MatchQuery::new("title", "test"));
        let query = Query::Bool(BoolQuery::new().must(inner_query));
        let optimized = optimizer.optimize(query).unwrap();

        // Single-clause boolean queries should be simplified
        assert!(matches!(optimized, Query::Match(_)));
    }

    #[test]
    fn test_query_analysis() {
        let optimizer = QueryOptimizer::new();
        let query = Query::Bool(
            BoolQuery::new()
                .must(Query::Match(MatchQuery::new("title", "test")))
                .should(Query::Fuzzy(FuzzyQuery::new("name", "john")))
                .must_not(Query::Regex(RegexQuery::new("content", "spam"))),
        );

        let analysis = optimizer.analyze(&query);

        assert_eq!(analysis.total_clauses, 4); // Bool + 3 sub-clauses
        assert_eq!(analysis.boolean_clauses, 3);
        assert_eq!(analysis.fuzzy_queries, 1);
        assert_eq!(analysis.regex_queries, 1);
        assert_eq!(analysis.unique_fields.len(), 3);
        assert!(analysis.unique_fields.contains("title"));
        assert!(analysis.unique_fields.contains("name"));
        assert!(analysis.unique_fields.contains("content"));
    }

    #[test]
    fn test_complexity_score() {
        let mut analysis = QueryAnalysis::new();
        analysis.total_clauses = 10;
        analysis.boolean_clauses = 5;
        analysis.fuzzy_queries = 2;
        analysis.regex_queries = 1;

        let score = analysis.complexity_score();
        assert!(score > 0);
        assert!(analysis.is_complex());
    }

    #[test]
    fn test_recommendations() {
        let mut analysis = QueryAnalysis::new();
        analysis.max_depth = 8;
        analysis.boolean_clauses = 15;
        analysis.fuzzy_queries = 5;
        analysis.regex_queries = 3;

        let recommendations = analysis.recommendations();
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.contains("depth")));
        assert!(recommendations.iter().any(|r| r.contains("boolean")));
        assert!(recommendations.iter().any(|r| r.contains("fuzzy")));
        assert!(recommendations.iter().any(|r| r.contains("Regex")));
    }
}
