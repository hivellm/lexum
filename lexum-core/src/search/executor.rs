//! Search execution engine

use crate::error::{Error, Result};
use crate::index::Index;
use crate::query::Query;
use crate::search::result::{SearchHit, SearchResult};
use crate::types::{DocumentId, Score};
use std::sync::Arc;
use std::time::Instant;
use tantivy::TantivyDocument;
use tantivy::query::{AllQuery, BooleanQuery, Occur, QueryParser, RangeQuery, TermQuery};
use tantivy::schema::*;

/// Search executor for running queries
pub struct SearchExecutor {
    index: Arc<Index>,
}

impl SearchExecutor {
    /// Create new search executor
    pub fn new(index: Arc<Index>) -> Self {
        Self { index }
    }

    /// Execute a search query
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lexum_core::{IndexManager, SchemaBuilder, SearchExecutor, QueryBuilder};
    /// use std::sync::Arc;
    ///
    /// # tokio_test::block_on(async {
    /// # let manager = IndexManager::new("./data");
    /// # let (schema, _) = SchemaBuilder::new().add_text_field("title").build().unwrap();
    /// # let index = manager.create_index("test", schema, Default::default()).await.unwrap();
    /// let executor = SearchExecutor::new(Arc::new(index));
    ///
    /// let query = QueryBuilder::match_query("title", "search terms");
    /// let result = executor.search(query, 10, 0).await.unwrap();
    ///
    /// println!("Found {} results", result.total);
    /// # });
    /// ```
    pub async fn search(&self, query: Query, limit: usize, offset: usize) -> Result<SearchResult> {
        let start = Instant::now();

        let schema = self.index.schema();
        let index = self.index.clone();

        let result = tokio::task::spawn_blocking(move || {
            let reader = index.reader()?;
            let searcher = reader.searcher();

            // Convert our query to Tantivy query
            let tantivy_query = Self::build_tantivy_query(&index.inner, &query)?;

            // Execute search
            let top_docs = searcher
                .search(
                    &tantivy_query,
                    &tantivy::collector::TopDocs::with_limit(limit),
                )
                .map_err(|e| Error::Config(format!("Search failed: {e}")))?;

            // Convert results
            let mut hits = Vec::new();
            for (score, doc_address) in top_docs.iter().skip(offset) {
                let doc: TantivyDocument = searcher
                    .doc(*doc_address)
                    .map_err(|e| Error::Config(format!("Failed to retrieve document: {e}")))?;

                let source = serde_json::from_str(&doc.to_json(&schema))
                    .map_err(|e| Error::Config(format!("Failed to parse document JSON: {e}")))?;

                hits.push(SearchHit {
                    id: DocumentId::new(format!("doc_{}", doc_address.segment_ord)),
                    score: Score::new(*score),
                    source,
                });
            }

            let total = hits.len();
            Ok::<SearchResult, Error>(SearchResult::new(hits, total, 0))
        })
        .await
        .map_err(|e| Error::Config(format!("Task join error: {e}")))?;

        let mut result = result?;
        result.took_ms = start.elapsed().as_millis() as u64;

        Ok(result)
    }

    /// Build Tantivy query from our Query type
    fn build_tantivy_query(
        tantivy_index: &tantivy::Index,
        query: &Query,
    ) -> Result<Box<dyn tantivy::query::Query>> {
        let schema = tantivy_index.schema();

        match query {
            Query::MatchAll => Ok(Box::new(AllQuery)),

            Query::Match(match_query) => {
                let field = schema
                    .get_field(&match_query.field)
                    .map_err(|e| Error::Config(format!("Field not found: {e}")))?;

                let query_parser = QueryParser::for_index(tantivy_index, vec![field]);
                query_parser
                    .parse_query(&match_query.query)
                    .map_err(|e| Error::Config(format!("Failed to parse query: {e}")))
            }

            Query::Term(term_query) => {
                let field = schema
                    .get_field(&term_query.field)
                    .map_err(|e| Error::Config(format!("Field not found: {e}")))?;

                let term = tantivy::Term::from_field_text(field, &term_query.value);
                Ok(Box::new(TermQuery::new(term, IndexRecordOption::Basic)))
            }

            Query::Range(range_query) => {
                let field = schema
                    .get_field(&range_query.field)
                    .map_err(|e| Error::Config(format!("Field not found: {e}")))?;

                // For now, only support i64 ranges (will expand later)
                if let (Some(gte_val), Some(lte_val)) = (&range_query.gte, &range_query.lte) {
                    let gte = gte_val
                        .as_i64()
                        .ok_or_else(|| Error::Config("Range value must be i64".to_string()))?;
                    let lte = lte_val
                        .as_i64()
                        .ok_or_else(|| Error::Config("Range value must be i64".to_string()))?;

                    let lower_bound =
                        std::ops::Bound::Included(tantivy::Term::from_field_i64(field, gte));
                    let upper_bound =
                        std::ops::Bound::Included(tantivy::Term::from_field_i64(field, lte));

                    Ok(Box::new(RangeQuery::new(lower_bound, upper_bound)))
                } else {
                    Err(Error::Config(
                        "Range query requires both gte and lte".to_string(),
                    ))
                }
            }

            Query::Bool(bool_query) => {
                let mut clauses = Vec::new();

                // Add must clauses
                for must in &bool_query.must {
                    let sub_query = Self::build_tantivy_query(tantivy_index, must)?;
                    clauses.push((Occur::Must, sub_query));
                }

                // Add should clauses
                for should in &bool_query.should {
                    let sub_query = Self::build_tantivy_query(tantivy_index, should)?;
                    clauses.push((Occur::Should, sub_query));
                }

                // Add must_not clauses
                for must_not in &bool_query.must_not {
                    let sub_query = Self::build_tantivy_query(tantivy_index, must_not)?;
                    clauses.push((Occur::MustNot, sub_query));
                }

                // Filter clauses (treat as must for now)
                for filter in &bool_query.filter {
                    let sub_query = Self::build_tantivy_query(tantivy_index, filter)?;
                    clauses.push((Occur::Must, sub_query));
                }

                Ok(Box::new(BooleanQuery::from(clauses)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{QueryBuilder, TermQuery};
    use crate::schema::SchemaBuilder;

    #[tokio::test]
    async fn test_search_executor() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        let executor = SearchExecutor::new(Arc::new(index));

        let query = QueryBuilder::match_all();
        let result = executor.search(query, 10, 0).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let query = QueryBuilder::term_query("title", "test");
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query);
        assert!(result.is_ok());
    }
}
