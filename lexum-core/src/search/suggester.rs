//! Search suggestion module
//!
//! Provides functionality for generating search suggestions including:
//! - Completion suggester (prefix-based autocomplete)
//! - Fuzzy suggester (typo-tolerant suggestions)
//! - Phrase suggester (complete phrase autocomplete)

use crate::error::{Error, Result};
use crate::index::Index;
use std::collections::HashSet;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::{FuzzyTermQuery, QueryParser, TermQuery};
use tantivy::schema::{IndexRecordOption, Term, Value};
use tantivy::{Searcher, TantivyDocument};

/// Configuration for suggestion generation
#[derive(Debug, Clone)]
pub struct SuggesterConfig {
    /// Maximum number of suggestions to return
    pub max_suggestions: usize,
    /// Minimum prefix length for completion suggester
    pub min_prefix_length: usize,
    /// Fuzziness level for fuzzy suggester (0-2)
    pub fuzziness: u8,
    /// Whether to include phrase suggestions
    pub include_phrases: bool,
    /// Maximum phrase length for phrase suggester
    pub max_phrase_length: usize,
}

impl Default for SuggesterConfig {
    fn default() -> Self {
        Self {
            max_suggestions: 10,
            min_prefix_length: 2,
            fuzziness: 1,
            include_phrases: true,
            max_phrase_length: 5,
        }
    }
}

impl SuggesterConfig {
    /// Create new suggester config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum number of suggestions
    pub fn with_max_suggestions(mut self, max: usize) -> Self {
        self.max_suggestions = max;
        self
    }

    /// Set minimum prefix length
    pub fn with_min_prefix_length(mut self, min: usize) -> Self {
        self.min_prefix_length = min;
        self
    }

    /// Set fuzziness level
    pub fn with_fuzziness(mut self, fuzziness: u8) -> Self {
        self.fuzziness = fuzziness.min(2);
        self
    }

    /// Set whether to include phrase suggestions
    pub fn with_include_phrases(mut self, include: bool) -> Self {
        self.include_phrases = include;
        self
    }

    /// Set maximum phrase length
    pub fn with_max_phrase_length(mut self, max: usize) -> Self {
        self.max_phrase_length = max;
        self
    }
}

/// A single search suggestion
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// The suggested text
    pub text: String,
    /// The score/relevance of this suggestion
    pub score: f32,
    /// The type of suggestion (completion, fuzzy, phrase)
    pub suggestion_type: SuggestionType,
}

impl PartialEq for Suggestion {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text && self.suggestion_type == other.suggestion_type
    }
}

impl Eq for Suggestion {}

/// Type of suggestion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuggestionType {
    /// Prefix-based completion
    Completion,
    /// Fuzzy/typo-tolerant suggestion
    Fuzzy,
    /// Complete phrase suggestion
    Phrase,
}

/// Search suggester
pub struct Suggester {
    index: Arc<Index>,
    config: SuggesterConfig,
}

impl Suggester {
    /// Create a new suggester with default configuration
    pub fn new(index: Arc<Index>) -> Self {
        Self {
            index,
            config: SuggesterConfig::default(),
        }
    }

    /// Create a new suggester with custom configuration
    pub fn with_config(index: Arc<Index>, config: SuggesterConfig) -> Self {
        Self { index, config }
    }

    /// Generate suggestions for a given query text
    ///
    /// This method combines completion, fuzzy, and phrase suggestions
    /// and returns them sorted by relevance.
    pub fn suggest(&self, query: &str, fields: &[String]) -> Result<Vec<Suggestion>> {
        if query.len() < self.config.min_prefix_length {
            return Ok(vec![]);
        }

        let reader = self.index.reader()?;
        let searcher = reader.searcher();

        let mut all_suggestions = Vec::new();
        let mut seen_texts = HashSet::new();

        // Get completion suggestions
        let completion_suggestions = self.completion_suggest(&searcher, query, fields)?;
        for suggestion in completion_suggestions {
            let text_lower = suggestion.text.to_lowercase();
            if !seen_texts.contains(&text_lower) {
                seen_texts.insert(text_lower);
                all_suggestions.push(suggestion);
            }
        }

        // Get fuzzy suggestions if query is long enough
        if query.len() >= 3 {
            let fuzzy_suggestions = self.fuzzy_suggest(&searcher, query, fields)?;
            for suggestion in fuzzy_suggestions {
                let text_lower = suggestion.text.to_lowercase();
                if !seen_texts.contains(&text_lower) {
                    seen_texts.insert(text_lower);
                    all_suggestions.push(suggestion);
                }
            }
        }

        // Get phrase suggestions if enabled
        if self.config.include_phrases {
            let phrase_suggestions = self.phrase_suggest(&searcher, query, fields)?;
            for suggestion in phrase_suggestions {
                let text_lower = suggestion.text.to_lowercase();
                if !seen_texts.contains(&text_lower) {
                    seen_texts.insert(text_lower);
                    all_suggestions.push(suggestion);
                }
            }
        }

        // Sort by score (descending)
        let mut suggestions = all_suggestions;
        suggestions.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        suggestions.truncate(self.config.max_suggestions);

        Ok(suggestions)
    }

    /// Generate completion suggestions (prefix-based)
    fn completion_suggest(
        &self,
        searcher: &Searcher,
        query: &str,
        fields: &[String],
    ) -> Result<Vec<Suggestion>> {
        let schema = self.index.schema();
        let mut suggestions = Vec::new();
        let query_lower = query.to_lowercase();

        for field_name in fields {
            let Ok(field) = schema.get_field(field_name) else {
                continue;
            };

            // Use a query parser to search for documents with prefix
            let query_parser = QueryParser::for_index(searcher.index(), vec![field]);
            let search_query = format!("{query}*");

            if let Ok(parsed_query) = query_parser.parse_query(&search_query) {
                let top_docs = searcher
                    .search(
                        &parsed_query,
                        &TopDocs::with_limit(self.config.max_suggestions * 3),
                    )
                    .map_err(|e| Error::Config(format!("Completion search failed: {e}")))?;

                // Extract unique terms from matching documents
                let mut seen_terms = HashSet::new();
                for (score, doc_address) in top_docs {
                    if let Ok(doc) = searcher.doc::<TantivyDocument>(doc_address) {
                        // Extract text from field
                        if let Some(field_value) = doc.get_first(field) {
                            if let Some(text) = field_value.as_str() {
                                let text_lower = text.to_lowercase();
                                // Check if text starts with query (for prefix matching)
                                if text_lower.starts_with(&query_lower)
                                    && !seen_terms.contains(&text_lower)
                                {
                                    seen_terms.insert(text_lower.clone());
                                    suggestions.push(Suggestion {
                                        text: text.to_string(),
                                        score: score.max(0.1), // Ensure positive score
                                        suggestion_type: SuggestionType::Completion,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // Also try exact term match for single-word queries
            if !query.contains(' ') {
                let term = Term::from_field_text(field, &query_lower);
                let term_query = TermQuery::new(term, IndexRecordOption::Basic);

                let top_docs = searcher
                    .search(
                        &term_query,
                        &TopDocs::with_limit(self.config.max_suggestions),
                    )
                    .map_err(|e| Error::Config(format!("Term search failed: {e}")))?;

                let mut seen_terms = HashSet::new();
                for (score, doc_address) in top_docs {
                    if let Ok(doc) = searcher.doc::<TantivyDocument>(doc_address) {
                        if let Some(field_value) = doc.get_first(field) {
                            if let Some(text) = field_value.as_str() {
                                let text_lower = text.to_lowercase();
                                if text_lower.starts_with(&query_lower)
                                    && !seen_terms.contains(&text_lower)
                                {
                                    seen_terms.insert(text_lower.clone());
                                    suggestions.push(Suggestion {
                                        text: text.to_string(),
                                        score: score.max(0.1),
                                        suggestion_type: SuggestionType::Completion,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(suggestions)
    }

    /// Generate fuzzy suggestions (typo-tolerant)
    fn fuzzy_suggest(
        &self,
        searcher: &Searcher,
        query: &str,
        fields: &[String],
    ) -> Result<Vec<Suggestion>> {
        let schema = self.index.schema();
        let mut suggestions = Vec::new();

        for field_name in fields {
            let Ok(field) = schema.get_field(field_name) else {
                continue;
            };

            // Create fuzzy query
            let term = Term::from_field_text(field, query);
            let fuzzy_query = FuzzyTermQuery::new(term, self.config.fuzziness, true);

            let top_docs = searcher
                .search(
                    &fuzzy_query,
                    &TopDocs::with_limit(self.config.max_suggestions),
                )
                .map_err(|e| Error::Config(format!("Fuzzy search failed: {e}")))?;

            // Extract unique terms from matching documents
            let mut seen_terms = HashSet::new();
            for (score, doc_address) in top_docs {
                if let Ok(doc) = searcher.doc::<TantivyDocument>(doc_address) {
                    if let Some(field_value) = doc.get_first(field) {
                        if let Some(text) = field_value.as_str() {
                            let text_lower = text.to_lowercase();
                            if text_lower != query.to_lowercase()
                                && !seen_terms.contains(&text_lower)
                            {
                                seen_terms.insert(text_lower.clone());
                                suggestions.push(Suggestion {
                                    text: text.to_string(),
                                    score,
                                    suggestion_type: SuggestionType::Fuzzy,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(suggestions)
    }

    /// Generate phrase suggestions (complete phrases)
    fn phrase_suggest(
        &self,
        searcher: &Searcher,
        query: &str,
        fields: &[String],
    ) -> Result<Vec<Suggestion>> {
        let schema = self.index.schema();
        let mut suggestions = Vec::new();

        // Split query into words
        let query_words: Vec<&str> = query.split_whitespace().collect();
        if query_words.is_empty() || query_words.len() > self.config.max_phrase_length {
            return Ok(suggestions);
        }

        for field_name in fields {
            let Ok(field) = schema.get_field(field_name) else {
                continue;
            };

            // Create a phrase query parser
            let query_parser = QueryParser::for_index(searcher.index(), vec![field]);

            // Try to parse as phrase query
            if let Ok(parsed_query) = query_parser.parse_query(query) {
                let top_docs = searcher
                    .search(
                        &parsed_query,
                        &TopDocs::with_limit(self.config.max_suggestions),
                    )
                    .map_err(|e| Error::Config(format!("Phrase search failed: {e}")))?;

                // Extract phrases from matching documents
                let mut seen_phrases = HashSet::new();
                for (score, doc_address) in top_docs {
                    if let Ok(doc) = searcher.doc::<TantivyDocument>(doc_address) {
                        if let Some(field_value) = doc.get_first(field) {
                            if let Some(text) = field_value.as_str() {
                                let text_lower = text.to_lowercase();
                                // Check if this text contains the query as a phrase
                                if text_lower.contains(&query.to_lowercase())
                                    && !seen_phrases.contains(&text_lower)
                                    && text.split_whitespace().count()
                                        <= self.config.max_phrase_length
                                {
                                    seen_phrases.insert(text_lower.clone());
                                    suggestions.push(Suggestion {
                                        text: text.to_string(),
                                        score,
                                        suggestion_type: SuggestionType::Phrase,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(suggestions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Index;
    use crate::types::IndexName;
    use tantivy::schema::{STORED, Schema, TEXT};
    use tempfile::TempDir;

    fn create_test_index() -> (TempDir, Arc<Index>) {
        let temp_dir = TempDir::new().unwrap();
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("content", TEXT | STORED);
        let schema = schema_builder.build();

        let schema_clone = schema.clone();
        let tantivy_index = tantivy::Index::create_in_dir(temp_dir.path(), schema).unwrap();
        let index = Index {
            name: IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        // Add some test documents
        let mut writer = index.writer(50_000_000).unwrap();
        for i in 0..10 {
            let mut doc = tantivy::TantivyDocument::default();
            doc.add_text(
                schema_clone.get_field("title").unwrap(),
                format!("Test Document {i}"),
            );
            doc.add_text(
                schema_clone.get_field("content").unwrap(),
                format!("This is test content {i}"),
            );
            writer.add_document(doc).unwrap();
        }
        writer.commit().unwrap();

        (temp_dir, Arc::new(index))
    }

    #[test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    fn test_completion_suggest() {
        let (_temp_dir, index) = create_test_index();
        let suggester = Suggester::new(index);

        let fields = vec!["title".to_string()];
        let suggestions = suggester
            .completion_suggest(
                &suggester.index.reader().unwrap().searcher(),
                "Test",
                &fields,
            )
            .unwrap();

        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.text.contains("Test")));
    }

    #[test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    fn test_fuzzy_suggest() {
        let (_temp_dir, index) = create_test_index();
        let suggester = Suggester::new(index);

        let fields = vec!["title".to_string()];
        let suggestions = suggester
            .fuzzy_suggest(
                &suggester.index.reader().unwrap().searcher(),
                "Tset", // typo for "Test"
                &fields,
            )
            .unwrap();

        // Should find "Test" documents despite typo
        assert!(!suggestions.is_empty());
    }

    #[test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    fn test_suggest_min_prefix_length() {
        let (_temp_dir, index) = create_test_index();
        let config = SuggesterConfig::new().with_min_prefix_length(3);
        let suggester = Suggester::with_config(index, config);

        let fields = vec!["title".to_string()];
        let suggestions = suggester.suggest("Te", &fields).unwrap();

        // Should return empty because "Te" is shorter than min_prefix_length (3)
        assert!(suggestions.is_empty());
    }
}
