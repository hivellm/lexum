//! LQL (Lexum Query Language) parser and executor

use anyhow::{Result, anyhow};
use lexum_core::Query;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

/// LQL parser for converting LQL strings to Lexum queries
pub struct LqlParser;

/// Simple query cache for parsed LQL queries
use std::sync::LazyLock;
static QUERY_CACHE: LazyLock<Mutex<HashMap<String, Query>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

impl LqlParser {
    /// Parse an LQL string into a Lexum Query with caching
    pub fn parse(lql: &str) -> Result<Query> {
        let lql = lql.trim();

        // Check cache first
        if let Ok(cache) = QUERY_CACHE.lock() {
            if let Some(cached_query) = cache.get(lql) {
                return Ok(cached_query.clone());
            }
        }

        // Parse the query
        let query = if lql.starts_with("FROM") {
            Self::parse_from_query(lql)
        } else if lql.starts_with("SELECT") {
            Self::parse_select_query(lql)
        } else if lql.starts_with("MATCH") {
            Self::parse_match_query(lql)
        } else {
            // Try to parse as a simple search query
            Self::parse_simple_query(lql)
        }?;

        // Cache the result (limit cache size to prevent memory issues)
        if let Ok(mut cache) = QUERY_CACHE.lock() {
            if cache.len() < 1000 {
                // Limit cache size
                cache.insert(lql.to_string(), query.clone());
            }
        }

        Ok(query)
    }

    /// Parse FROM query: FROM index WHERE field:value
    fn parse_from_query(lql: &str) -> Result<Query> {
        let parts: Vec<&str> = lql.split_whitespace().collect();

        if parts.len() < 2 {
            return Err(anyhow!("FROM query requires an index name"));
        }

        let _index = parts[1];

        // Look for WHERE clause
        if let Some(where_pos) = parts.iter().position(|&p| p == "WHERE") {
            if where_pos + 1 < parts.len() {
                let where_clause = parts[where_pos + 1..].join(" ");
                return Self::parse_where_clause(&where_clause);
            }
        }

        // No WHERE clause, return match_all
        Ok(Query::MatchAll)
    }

    /// Parse SELECT query: SELECT * FROM index WHERE field:value
    fn parse_select_query(lql: &str) -> Result<Query> {
        // For now, treat SELECT queries the same as FROM queries
        Self::parse_from_query(lql)
    }

    /// Parse MATCH query: MATCH field:value
    fn parse_match_query(lql: &str) -> Result<Query> {
        let parts: Vec<&str> = lql.split_whitespace().collect();

        if parts.len() < 2 {
            return Err(anyhow!("MATCH query requires field:value"));
        }

        let match_clause = parts[1..].join(" ");
        Self::parse_where_clause(&match_clause)
    }

    /// Parse simple query patterns
    fn parse_simple_query(lql: &str) -> Result<Query> {
        // Try to parse as field:value pattern
        if lql.contains(':') {
            Self::parse_where_clause(lql)
        } else {
            // Treat as match query
            Ok(Query::Match(lexum_core::query::types::MatchQuery {
                field: "*".to_string(),
                query: lql.to_string(),
            }))
        }
    }

    /// Parse WHERE clause: field:value, field:"phrase", field:[min,max]
    fn parse_where_clause(where_clause: &str) -> Result<Query> {
        let where_clause = where_clause.trim();

        // Handle multiple conditions with AND/OR
        if where_clause.contains(" AND ") {
            return Self::parse_boolean_query(where_clause, "AND");
        }

        if where_clause.contains(" OR ") {
            return Self::parse_boolean_query(where_clause, "OR");
        }

        // Single condition
        Self::parse_single_condition(where_clause)
    }

    /// Parse boolean query with AND/OR
    fn parse_boolean_query(where_clause: &str, operator: &str) -> Result<Query> {
        let conditions: Vec<&str> = where_clause.split(&format!(" {operator} ")).collect();
        let mut queries = Vec::new();

        for condition in conditions {
            let query = Self::parse_single_condition(condition.trim())?;
            queries.push(query);
        }

        let bool_query = if operator == "AND" {
            lexum_core::query::types::BoolQuery {
                must: queries,
                should: vec![],
                must_not: vec![],
                filter: vec![],
            }
        } else {
            lexum_core::query::types::BoolQuery {
                must: vec![],
                should: queries,
                must_not: vec![],
                filter: vec![],
            }
        };

        Ok(Query::Bool(bool_query))
    }

    /// Parse single condition: field:value, field:"phrase", field:[min,max]
    fn parse_single_condition(condition: &str) -> Result<Query> {
        let condition = condition.trim();

        // Range query: field:[min,max]
        if condition.contains('[') && condition.contains(']') {
            return Self::parse_range_query(condition);
        }

        // Phrase query: field:"phrase"
        if condition.contains('"') {
            return Self::parse_phrase_query(condition);
        }

        // Fuzzy query: field:~value
        if condition.contains(":~") {
            return Self::parse_fuzzy_query(condition);
        }

        // Term query: field:value
        if condition.contains(':') {
            return Self::parse_term_query(condition);
        }

        // Default to match query
        Ok(Query::Match(lexum_core::query::types::MatchQuery {
            field: "*".to_string(),
            query: condition.to_string(),
        }))
    }

    /// Parse range query: field:[min,max]
    fn parse_range_query(condition: &str) -> Result<Query> {
        let parts: Vec<&str> = condition.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid range query format"));
        }

        let field = parts[0].trim();
        let range_str = parts[1].trim();

        if !range_str.starts_with('[') || !range_str.ends_with(']') {
            return Err(anyhow!("Range query must be in format [min,max]"));
        }

        let range_content = &range_str[1..range_str.len() - 1];
        let range_parts: Vec<&str> = range_content.split(',').collect();

        if range_parts.len() != 2 {
            return Err(anyhow!("Range query must have min and max values"));
        }

        let min_val = range_parts[0].trim();
        let max_val = range_parts[1].trim();

        Ok(Query::Range(lexum_core::query::types::RangeQuery {
            field: field.to_string(),
            gte: Some(Value::String(min_val.to_string())),
            lte: Some(Value::String(max_val.to_string())),
            gt: None,
            lt: None,
        }))
    }

    /// Parse phrase query: field:"phrase"
    fn parse_phrase_query(condition: &str) -> Result<Query> {
        let parts: Vec<&str> = condition.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid phrase query format"));
        }

        let field = parts[0].trim();
        let phrase_str = parts[1].trim();

        if !phrase_str.starts_with('"') || !phrase_str.ends_with('"') {
            return Err(anyhow!("Phrase query must be in format field:\"phrase\""));
        }

        let phrase = &phrase_str[1..phrase_str.len() - 1];

        Ok(Query::Phrase(lexum_core::query::types::PhraseQuery {
            field: field.to_string(),
            phrase: phrase.to_string(),
            slop: 0,
        }))
    }

    /// Parse fuzzy query: field:~value
    fn parse_fuzzy_query(condition: &str) -> Result<Query> {
        let parts: Vec<&str> = condition.split(":~").collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid fuzzy query format"));
        }

        let field = parts[0].trim();
        let value = parts[1].trim();

        Ok(Query::Fuzzy(lexum_core::query::types::FuzzyQuery {
            field: field.to_string(),
            value: value.to_string(),
            fuzziness: 1,
            prefix_length: 0,
            transpositions: true,
        }))
    }

    /// Parse term query: field:value
    fn parse_term_query(condition: &str) -> Result<Query> {
        let parts: Vec<&str> = condition.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid term query format"));
        }

        let field = parts[0].trim();
        let value = parts[1].trim();

        Ok(Query::Term(lexum_core::query::types::TermQuery {
            field: field.to_string(),
            value: value.to_string(),
        }))
    }
}

/// LQL query executor
pub struct LqlExecutor {
    base_url: String,
}

impl LqlExecutor {
    /// Create a new LQL executor with the given base URL
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    /// Execute an LQL query against an index
    pub async fn execute(&self, index: &str, lql: &str) -> Result<Value> {
        let query = LqlParser::parse(lql)?;

        // Convert query to JSON for API call
        let query_json = serde_json::to_value(&query)?;

        // Create search request
        let search_request = serde_json::json!({
            "query": query_json,
            "limit": 10,
            "offset": 0
        });

        // Make API call
        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/indices/{}/search", self.base_url, index);

        let response = client.post(&url).json(&search_request).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Search request failed: {}", response.status()));
        }

        let result: Value = response.json().await?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_query() {
        let query = LqlParser::parse("title:rust").unwrap();
        match query {
            Query::Term(term) => {
                assert_eq!(term.field, "title");
                assert_eq!(term.value, "rust");
            }
            _ => panic!("Expected Term query"),
        }
    }

    #[test]
    fn test_parse_phrase_query() {
        let query = LqlParser::parse("title:\"rust programming\"").unwrap();
        match query {
            Query::Phrase(phrase) => {
                assert_eq!(phrase.field, "title");
                assert_eq!(phrase.phrase, "rust programming");
            }
            _ => panic!("Expected Phrase query"),
        }
    }

    #[test]
    fn test_parse_range_query() {
        let query = LqlParser::parse("age:[18,65]").unwrap();
        match query {
            Query::Range(range) => {
                assert_eq!(range.field, "age");
                assert_eq!(range.gte, Some(Value::String("18".to_string())));
                assert_eq!(range.lte, Some(Value::String("65".to_string())));
            }
            _ => panic!("Expected Range query"),
        }
    }

    #[test]
    fn test_parse_fuzzy_query() {
        let query = LqlParser::parse("title:~rust").unwrap();
        match query {
            Query::Fuzzy(fuzzy) => {
                assert_eq!(fuzzy.field, "title");
                assert_eq!(fuzzy.value, "rust");
            }
            _ => panic!("Expected Fuzzy query"),
        }
    }

    #[test]
    fn test_parse_boolean_query() {
        let query = LqlParser::parse("title:rust AND age:[18,65]").unwrap();
        match query {
            Query::Bool(bool_query) => {
                assert_eq!(bool_query.must.len(), 2);
                assert_eq!(bool_query.should.len(), 0);
            }
            _ => panic!("Expected Bool query"),
        }
    }

    #[test]
    fn test_parse_from_query() {
        let query = LqlParser::parse("FROM users WHERE name:john").unwrap();
        match query {
            Query::Term(term) => {
                assert_eq!(term.field, "name");
                assert_eq!(term.value, "john");
            }
            _ => panic!("Expected Term query"),
        }
    }
}
