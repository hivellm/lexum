//! Query object pool for reusing query objects
//!
//! This module provides a query pool that allows reusing query objects
//! to reduce memory allocations in hot paths, particularly for frequently
//! used query types like MatchQuery and TermQuery.

use crate::query::{BoolQuery, MatchQuery, TermQuery};
use std::sync::{Arc, Mutex};

/// Thread-safe query pool for reusing query objects
#[derive(Debug, Clone)]
pub struct QueryPool {
    /// Pool of available MatchQuery objects
    match_query_pool: Arc<Mutex<Vec<MatchQuery>>>,
    /// Pool of available TermQuery objects
    term_query_pool: Arc<Mutex<Vec<TermQuery>>>,
    /// Pool of available BoolQuery objects
    bool_query_pool: Arc<Mutex<Vec<BoolQuery>>>,
    /// Maximum number of queries to keep in each pool
    max_pool_size: usize,
}

impl QueryPool {
    /// Create new query pool with default settings
    ///
    /// Defaults:
    /// - Max pool size: 50 queries per type
    pub fn new() -> Self {
        Self::with_settings(50)
    }

    /// Create query pool with custom settings
    ///
    /// # Arguments
    /// * `max_pool_size` - Maximum number of queries to keep in each pool
    pub fn with_settings(max_pool_size: usize) -> Self {
        Self {
            match_query_pool: Arc::new(Mutex::new(Vec::new())),
            term_query_pool: Arc::new(Mutex::new(Vec::new())),
            bool_query_pool: Arc::new(Mutex::new(Vec::new())),
            max_pool_size,
        }
    }

    /// Get a MatchQuery from the pool or create a new one
    ///
    /// # Arguments
    /// * `field` - Field name
    /// * `query` - Query text
    ///
    /// # Returns
    /// A MatchQuery (may be reused from pool or newly allocated)
    pub fn get_match_query(
        &self,
        field: impl Into<String>,
        query: impl Into<String>,
    ) -> MatchQuery {
        let mut pool = self.match_query_pool.lock().unwrap();
        if let Some(mut q) = pool.pop() {
            // Reuse existing query object
            q.field = field.into();
            q.query = query.into();
            q
        } else {
            // Create new query
            MatchQuery::new(field, query)
        }
    }

    /// Return a MatchQuery to the pool for reuse
    ///
    /// If the pool is full, the query will be dropped.
    ///
    /// # Arguments
    /// * `query` - Query to return to pool
    pub fn put_match_query(&self, query: MatchQuery) {
        let mut pool = self.match_query_pool.lock().unwrap();
        if pool.len() < self.max_pool_size {
            pool.push(query);
        }
    }

    /// Get a TermQuery from the pool or create a new one
    ///
    /// # Arguments
    /// * `field` - Field name
    /// * `value` - Term value
    ///
    /// # Returns
    /// A TermQuery (may be reused from pool or newly allocated)
    pub fn get_term_query(&self, field: impl Into<String>, value: impl Into<String>) -> TermQuery {
        let mut pool = self.term_query_pool.lock().unwrap();
        if let Some(mut q) = pool.pop() {
            // Reuse existing query object
            q.field = field.into();
            q.value = value.into();
            q
        } else {
            // Create new query
            TermQuery::new(field, value)
        }
    }

    /// Return a TermQuery to the pool for reuse
    ///
    /// If the pool is full, the query will be dropped.
    ///
    /// # Arguments
    /// * `query` - Query to return to pool
    pub fn put_term_query(&self, query: TermQuery) {
        let mut pool = self.term_query_pool.lock().unwrap();
        if pool.len() < self.max_pool_size {
            pool.push(query);
        }
    }

    /// Get a BoolQuery from the pool or create a new one
    ///
    /// # Returns
    /// A BoolQuery (may be reused from pool or newly allocated)
    pub fn get_bool_query(&self) -> BoolQuery {
        let mut pool = self.bool_query_pool.lock().unwrap();
        if let Some(mut q) = pool.pop() {
            // Clear the query for reuse
            q.must.clear();
            q.should.clear();
            q.must_not.clear();
            q.filter.clear();
            q
        } else {
            // Create new query
            BoolQuery::new()
        }
    }

    /// Return a BoolQuery to the pool for reuse
    ///
    /// The query will be cleared before being added to the pool.
    /// If the pool is full, the query will be dropped.
    ///
    /// # Arguments
    /// * `query` - Query to return to pool
    pub fn put_bool_query(&self, mut query: BoolQuery) {
        let mut pool = self.bool_query_pool.lock().unwrap();
        if pool.len() < self.max_pool_size {
            // Clear the query
            query.must.clear();
            query.should.clear();
            query.must_not.clear();
            query.filter.clear();
            pool.push(query);
        }
    }

    /// Get pool statistics
    ///
    /// # Returns
    /// Tuple of (match_queries, term_queries, bool_queries, max_pool_size)
    pub fn stats(&self) -> (usize, usize, usize, usize) {
        let match_pool = self.match_query_pool.lock().unwrap();
        let term_pool = self.term_query_pool.lock().unwrap();
        let bool_pool = self.bool_query_pool.lock().unwrap();
        (
            match_pool.len(),
            term_pool.len(),
            bool_pool.len(),
            self.max_pool_size,
        )
    }

    /// Clear all queries from the pool
    pub fn clear(&self) {
        let mut match_pool = self.match_query_pool.lock().unwrap();
        let mut term_pool = self.term_query_pool.lock().unwrap();
        let mut bool_pool = self.bool_query_pool.lock().unwrap();
        match_pool.clear();
        term_pool.clear();
        bool_pool.clear();
    }
}

impl Default for QueryPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::Query;

    #[test]
    fn test_query_pool_match_query() {
        let pool = QueryPool::new();
        let query = pool.get_match_query("field", "value");
        assert_eq!(query.field, "field");
        assert_eq!(query.query, "value");

        pool.put_match_query(query);
        let stats = pool.stats();
        assert_eq!(stats.0, 1); // match_query pool has 1 item
    }

    #[test]
    fn test_query_pool_match_query_reuse() {
        let pool = QueryPool::new();
        let query1 = pool.get_match_query("field1", "value1");
        pool.put_match_query(query1);

        let query2 = pool.get_match_query("field2", "value2");
        assert_eq!(query2.field, "field2");
        assert_eq!(query2.query, "value2");
    }

    #[test]
    fn test_query_pool_term_query() {
        let pool = QueryPool::new();
        let query = pool.get_term_query("field", "value");
        assert_eq!(query.field, "field");
        assert_eq!(query.value, "value");

        pool.put_term_query(query);
        let stats = pool.stats();
        assert_eq!(stats.1, 1); // term_query pool has 1 item
    }

    #[test]
    fn test_query_pool_bool_query() {
        let pool = QueryPool::new();
        let mut query = pool.get_bool_query();
        query
            .must
            .push(Query::Match(MatchQuery::new("field", "value")));

        pool.put_bool_query(query);
        let stats = pool.stats();
        assert_eq!(stats.2, 1); // bool_query pool has 1 item
    }

    #[test]
    fn test_query_pool_bool_query_reuse() {
        let pool = QueryPool::new();
        let mut query1 = pool.get_bool_query();
        query1
            .must
            .push(Query::Match(MatchQuery::new("field1", "value1")));
        pool.put_bool_query(query1);

        let query2 = pool.get_bool_query();
        assert!(query2.must.is_empty());
        assert!(query2.should.is_empty());
        assert!(query2.must_not.is_empty());
        assert!(query2.filter.is_empty());
    }

    #[test]
    fn test_query_pool_max_size() {
        let pool = QueryPool::with_settings(2);
        pool.put_match_query(MatchQuery::new("field1", "value1"));
        pool.put_match_query(MatchQuery::new("field2", "value2"));
        pool.put_match_query(MatchQuery::new("field3", "value3")); // Should be dropped

        let stats = pool.stats();
        assert_eq!(stats.0, 2);
    }

    #[test]
    fn test_query_pool_clear() {
        let pool = QueryPool::new();
        pool.put_match_query(MatchQuery::new("field", "value"));
        pool.put_term_query(TermQuery::new("field", "value"));

        pool.clear();
        let stats = pool.stats();
        assert_eq!(stats.0, 0);
        assert_eq!(stats.1, 0);
        assert_eq!(stats.2, 0);
    }

    #[test]
    fn test_query_pool_stats() {
        let pool = QueryPool::new();
        pool.put_match_query(MatchQuery::new("field1", "value1"));
        pool.put_match_query(MatchQuery::new("field2", "value2"));
        pool.put_term_query(TermQuery::new("field", "value"));
        pool.put_bool_query(BoolQuery::new());

        let stats = pool.stats();
        assert_eq!(stats.0, 2); // match queries
        assert_eq!(stats.1, 1); // term queries
        assert_eq!(stats.2, 1); // bool queries
        assert_eq!(stats.3, 50); // max pool size
    }
}
