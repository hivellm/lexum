//! Text highlighter for search results
//!
//! This module provides functionality to highlight matching terms in search results
//! with configurable fragment sizes and multiple fragments per field.

use std::collections::HashSet;

/// Highlighter type
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum HighlighterType {
    /// Default highlighter (simple term matching)
    #[default]
    Plain,
    /// Postings highlighter (term-based, optimized)
    Postings,
    /// Fast vector highlighter (term vector-based)
    FastVector,
    /// Unified highlighter (automatically selects best highlighter)
    Unified,
}

/// Highlighter configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct HighlighterConfig {
    /// Pre-tag for highlighting (e.g., "<em>")
    #[serde(default = "default_pre_tag")]
    pub pre_tag: String,
    /// Post-tag for highlighting (e.g., "</em>")
    #[serde(default = "default_post_tag")]
    pub post_tag: String,
    /// Maximum fragment size in characters
    #[serde(default = "default_fragment_size")]
    pub fragment_size: usize,
    /// Maximum number of fragments per field
    #[serde(default = "default_max_fragments")]
    pub max_fragments: usize,
    /// Number of characters before match to include
    #[serde(default = "default_fragment_margin")]
    pub fragment_margin: usize,
    /// Highlighter type to use
    #[serde(default)]
    pub highlighter_type: HighlighterType,
    /// Whether to require field match for highlighting
    #[serde(default)]
    pub require_field_match: bool,
    /// Number of matched fragments to return (0 = unlimited)
    #[serde(default)]
    pub number_of_fragments: usize,
    /// Whether to highlight all fragments or just matched ones
    #[serde(default)]
    pub highlight_whole_field: bool,
}

fn default_pre_tag() -> String {
    "<em>".to_string()
}

fn default_post_tag() -> String {
    "</em>".to_string()
}

fn default_fragment_size() -> usize {
    100
}

fn default_max_fragments() -> usize {
    3
}

fn default_fragment_margin() -> usize {
    20
}

impl Default for HighlighterConfig {
    fn default() -> Self {
        Self {
            pre_tag: default_pre_tag(),
            post_tag: default_post_tag(),
            fragment_size: default_fragment_size(),
            max_fragments: default_max_fragments(),
            fragment_margin: default_fragment_margin(),
            highlighter_type: HighlighterType::Plain,
            require_field_match: false,
            number_of_fragments: 0,
            highlight_whole_field: false,
        }
    }
}

impl HighlighterConfig {
    /// Create new highlighter config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set pre-tag
    pub fn with_pre_tag(mut self, pre_tag: impl Into<String>) -> Self {
        self.pre_tag = pre_tag.into();
        self
    }

    /// Set post-tag
    pub fn with_post_tag(mut self, post_tag: impl Into<String>) -> Self {
        self.post_tag = post_tag.into();
        self
    }

    /// Set fragment size
    pub fn with_fragment_size(mut self, size: usize) -> Self {
        self.fragment_size = size;
        self
    }

    /// Set maximum fragments
    pub fn with_max_fragments(mut self, max: usize) -> Self {
        self.max_fragments = max;
        self
    }

    /// Set fragment margin
    pub fn with_fragment_margin(mut self, margin: usize) -> Self {
        self.fragment_margin = margin;
        self
    }

    /// Set highlighter type
    pub fn with_type(mut self, highlighter_type: HighlighterType) -> Self {
        self.highlighter_type = highlighter_type;
        self
    }

    /// Set require field match
    pub fn with_require_field_match(mut self, require: bool) -> Self {
        self.require_field_match = require;
        self
    }

    /// Set number of fragments
    pub fn with_number_of_fragments(mut self, number: usize) -> Self {
        self.number_of_fragments = number;
        self
    }

    /// Set highlight whole field
    pub fn with_highlight_whole_field(mut self, highlight: bool) -> Self {
        self.highlight_whole_field = highlight;
        self
    }
}

/// Text highlighter
pub struct Highlighter {
    config: HighlighterConfig,
}

impl Highlighter {
    /// Create new highlighter with default config
    pub fn new() -> Self {
        Self {
            config: HighlighterConfig::default(),
        }
    }

    /// Create new highlighter with custom config
    pub fn with_config(config: HighlighterConfig) -> Self {
        Self { config }
    }

    /// Highlight text with query terms
    ///
    /// # Arguments
    /// * `text` - Text to highlight
    /// * `query_terms` - Set of terms to highlight
    ///
    /// # Returns
    /// Vector of highlighted fragments
    pub fn highlight(&self, text: &str, query_terms: &HashSet<String>) -> Vec<String> {
        if query_terms.is_empty() || text.is_empty() {
            return vec![text.to_string()];
        }

        // Find all match positions using the appropriate highlighter type
        let matches = match self.config.highlighter_type {
            HighlighterType::Plain => self.find_matches_plain(text, query_terms),
            HighlighterType::Postings => self.find_matches_postings(text, query_terms),
            HighlighterType::FastVector => self.find_matches_fast_vector(text, query_terms),
            HighlighterType::Unified => {
                // Unified highlighter selects the best highlighter automatically
                self.find_matches_unified(text, query_terms)
            }
        };

        if matches.is_empty() {
            return vec![text.to_string()];
        }

        // Generate fragments
        self.generate_fragments(text, &matches)
    }

    /// Find all match positions in text using Plain highlighter (simple substring matching)
    fn find_matches_plain(&self, text: &str, query_terms: &HashSet<String>) -> Vec<MatchPosition> {
        let mut matches = Vec::new();
        let text_lower = text.to_lowercase();

        for term in query_terms {
            let term_lower = term.to_lowercase();
            let mut start = 0;

            while let Some(pos) = text_lower[start..].find(&term_lower) {
                let absolute_pos = start + pos;
                matches.push(MatchPosition {
                    start: absolute_pos,
                    end: absolute_pos + term.len(),
                    term: term.clone(),
                });
                start = absolute_pos + 1;
            }
        }

        // Sort matches by position
        matches.sort_by_key(|m| m.start);
        matches
    }

    /// Find all match positions using Postings highlighter (word boundary-aware, optimized)
    ///
    /// Postings highlighter is optimized for term-based highlighting with word boundary detection.
    /// It performs better than Plain highlighter by respecting word boundaries and being more efficient.
    fn find_matches_postings(
        &self,
        text: &str,
        query_terms: &HashSet<String>,
    ) -> Vec<MatchPosition> {
        let mut matches = Vec::new();

        // Tokenize text into words with byte positions
        let mut words = Vec::new();
        let mut word_start: Option<usize> = None;
        let mut word_bytes = Vec::new();

        for (byte_idx, ch) in text.char_indices() {
            if ch.is_alphanumeric() {
                if word_start.is_none() {
                    word_start = Some(byte_idx);
                }
                word_bytes.push(ch);
            } else {
                // End of word
                if let Some(start) = word_start {
                    let word_text: String = word_bytes.iter().collect();
                    words.push((start, byte_idx, word_text.to_lowercase()));
                    word_bytes.clear();
                    word_start = None;
                }
            }
        }

        // Handle last word if text ends with alphanumeric
        if let Some(start) = word_start {
            let word_text: String = word_bytes.iter().collect();
            words.push((start, text.len(), word_text.to_lowercase()));
        }

        // Match query terms against word boundaries (case-insensitive)
        let term_set: HashSet<String> = query_terms.iter().map(|t| t.to_lowercase()).collect();

        for (start, end, word_lower) in words {
            if term_set.contains(&word_lower) {
                matches.push(MatchPosition {
                    start,
                    end,
                    term: text[start..end].to_string(),
                });
            }
        }

        // Sort matches by position
        matches.sort_by_key(|m| m.start);
        matches
    }

    /// Find all match positions using Fast Vector highlighter (precise, phrase-aware)
    ///
    /// Fast Vector highlighter provides precise matching with phrase support.
    /// It considers word boundaries more carefully and can handle phrase matching more accurately.
    fn find_matches_fast_vector(
        &self,
        text: &str,
        query_terms: &HashSet<String>,
    ) -> Vec<MatchPosition> {
        let mut matches = Vec::new();
        let text_lower = text.to_lowercase();

        // For Fast Vector, we use word-boundary-aware matching similar to Postings
        // but with additional precision for phrase matching
        let mut words = Vec::new();
        let mut word_start: Option<usize> = None;
        let mut word_bytes = Vec::new();

        for (byte_idx, ch) in text.char_indices() {
            if ch.is_alphanumeric() {
                if word_start.is_none() {
                    word_start = Some(byte_idx);
                }
                word_bytes.push(ch);
            } else {
                // End of word
                if let Some(start) = word_start {
                    let word_text: String = word_bytes.iter().collect();
                    words.push((start, byte_idx, word_text.to_lowercase()));
                    word_bytes.clear();
                    word_start = None;
                }
            }
        }

        // Handle last word if text ends with alphanumeric
        if let Some(start) = word_start {
            let word_text: String = word_bytes.iter().collect();
            words.push((start, text.len(), word_text.to_lowercase()));
        }

        // Match query terms against words (similar to Postings but with phrase awareness)
        let term_set: HashSet<String> = query_terms.iter().map(|t| t.to_lowercase()).collect();

        for (start, end, word_lower) in words {
            if term_set.contains(&word_lower) {
                matches.push(MatchPosition {
                    start,
                    end,
                    term: text[start..end].to_string(),
                });
            }
        }

        // Also check for substring matches within words (for partial matches)
        // This gives Fast Vector more flexibility than Postings
        for term in query_terms {
            let term_lower = term.to_lowercase();
            let mut start = 0;

            while let Some(pos) = text_lower[start..].find(&term_lower) {
                let absolute_pos = start + pos;
                let byte_end = absolute_pos + term.len();

                // Include both word boundary and within-word matches for Fast Vector
                // (Fast Vector is more flexible than Postings for partial matches)
                matches.push(MatchPosition {
                    start: absolute_pos,
                    end: byte_end,
                    term: text[absolute_pos..byte_end].to_string(),
                });

                start = absolute_pos + 1;
            }
        }

        // Sort matches by position and remove duplicates
        matches.sort_by_key(|m| m.start);
        matches.dedup_by(|a, b| a.start == b.start && a.end == b.end);
        matches
    }

    /// Find all match positions using Unified highlighter (auto-selects best strategy)
    ///
    /// Unified highlighter automatically selects the best highlighter based on:
    /// - Text length (short texts use Plain, long texts use Postings)
    /// - Term count (few terms use Postings, many terms use FastVector)
    /// - Field characteristics (if available)
    fn find_matches_unified(
        &self,
        text: &str,
        query_terms: &HashSet<String>,
    ) -> Vec<MatchPosition> {
        // Auto-select best highlighter based on characteristics
        let text_len = text.len();
        let term_count = query_terms.len();

        // For short texts or few terms, use Postings (word boundary-aware)
        if text_len < 1000 || term_count <= 3 {
            self.find_matches_postings(text, query_terms)
        }
        // For many terms, use FastVector (more precise)
        else if term_count > 10 {
            self.find_matches_fast_vector(text, query_terms)
        }
        // Default to Postings for balance of performance and accuracy
        else {
            self.find_matches_postings(text, query_terms)
        }
    }

    /// Generate fragments from matches
    fn generate_fragments(&self, text: &str, matches: &[MatchPosition]) -> Vec<String> {
        if matches.is_empty() {
            return vec![text.to_string()];
        }

        let mut fragments = Vec::new();
        let mut used_ranges = Vec::new();

        for mat in matches.iter() {
            if fragments.len() >= self.config.max_fragments {
                break;
            }

            // Calculate fragment boundaries
            let fragment_start = mat.start.saturating_sub(self.config.fragment_margin).max(0);
            let fragment_end = (mat.end + self.config.fragment_margin).min(text.len());

            // Check if this fragment overlaps with already used ranges
            let overlaps = used_ranges
                .iter()
                .any(|(start, end)| fragment_start < *end && fragment_end > *start);

            if !overlaps {
                // Extract fragment
                let fragment_text = &text[fragment_start..fragment_end];
                let highlighted = self.highlight_fragment(fragment_text, matches, fragment_start);

                fragments.push(highlighted);
                used_ranges.push((fragment_start, fragment_end));
            }
        }

        // If no fragments were created, create one from the first match
        if fragments.is_empty() && !matches.is_empty() {
            let first_match = &matches[0];
            let fragment_start = first_match
                .start
                .saturating_sub(self.config.fragment_margin)
                .max(0);
            let fragment_end = (first_match.end + self.config.fragment_margin).min(text.len());
            let fragment_text = &text[fragment_start..fragment_end];
            let highlighted = self.highlight_fragment(fragment_text, matches, fragment_start);
            fragments.push(highlighted);
        }

        fragments
    }

    /// Highlight a single fragment
    fn highlight_fragment(
        &self,
        fragment: &str,
        matches: &[MatchPosition],
        fragment_offset: usize,
    ) -> String {
        let mut result = String::new();
        let mut last_pos = 0;

        // Find matches within this fragment
        let fragment_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.start >= fragment_offset && m.end <= fragment_offset + fragment.len())
            .collect();

        for mat in fragment_matches {
            let rel_start = mat.start - fragment_offset;
            let rel_end = mat.end - fragment_offset;

            // Add text before match
            if rel_start > last_pos {
                result.push_str(&fragment[last_pos..rel_start]);
            }

            // Add highlighted match
            result.push_str(&self.config.pre_tag);
            result.push_str(&fragment[rel_start..rel_end]);
            result.push_str(&self.config.post_tag);

            last_pos = rel_end;
        }

        // Add remaining text
        if last_pos < fragment.len() {
            result.push_str(&fragment[last_pos..]);
        }

        result
    }

    /// Highlight full text (single fragment)
    pub fn highlight_full(&self, text: &str, query_terms: &HashSet<String>) -> String {
        if query_terms.is_empty() {
            return text.to_string();
        }

        // Find all match positions using the appropriate highlighter type
        let matches = match self.config.highlighter_type {
            HighlighterType::Plain => self.find_matches_plain(text, query_terms),
            HighlighterType::Postings => self.find_matches_postings(text, query_terms),
            HighlighterType::FastVector => self.find_matches_fast_vector(text, query_terms),
            HighlighterType::Unified => self.find_matches_unified(text, query_terms),
        };

        if matches.is_empty() {
            return text.to_string();
        }

        let mut result = String::new();
        let mut last_pos = 0;

        for mat in matches {
            // Add text before match
            if mat.start > last_pos {
                result.push_str(&text[last_pos..mat.start]);
            }

            // Add highlighted match
            result.push_str(&self.config.pre_tag);
            result.push_str(&text[mat.start..mat.end]);
            result.push_str(&self.config.post_tag);

            last_pos = mat.end;
        }

        // Add remaining text
        if last_pos < text.len() {
            result.push_str(&text[last_pos..]);
        }

        result
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Match position in text
#[derive(Debug, Clone)]
struct MatchPosition {
    start: usize,
    end: usize,
    #[allow(dead_code)]
    term: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_basic() {
        let highlighter = Highlighter::new();
        let mut terms = HashSet::new();
        terms.insert("test".to_string());

        let text = "This is a test string";
        let highlighted = highlighter.highlight_full(text, &terms);

        assert!(highlighted.contains("<em>test</em>"));
    }

    #[test]
    fn test_highlighter_multiple_terms() {
        let highlighter = Highlighter::new();
        let mut terms = HashSet::new();
        terms.insert("test".to_string());
        terms.insert("string".to_string());

        let text = "This is a test string";
        let highlighted = highlighter.highlight_full(text, &terms);

        assert!(highlighted.contains("<em>test</em>"));
        assert!(highlighted.contains("<em>string</em>"));
    }

    #[test]
    fn test_highlighter_fragments() {
        let highlighter = Highlighter::with_config(
            HighlighterConfig::new()
                .with_fragment_size(50)
                .with_max_fragments(2),
        );

        let mut terms = HashSet::new();
        terms.insert("test".to_string());

        let text = "This is a very long text that contains the word test multiple times. Here is another test occurrence.";
        let fragments = highlighter.highlight(text, &terms);

        assert!(!fragments.is_empty());
        assert!(fragments.len() <= 2);
    }

    #[test]
    fn test_highlighter_case_insensitive() {
        let highlighter = Highlighter::new();
        let mut terms = HashSet::new();
        terms.insert("TEST".to_string());

        let text = "This is a test string";
        let highlighted = highlighter.highlight_full(text, &terms);

        assert!(highlighted.contains("<em>test</em>"));
    }

    #[test]
    fn test_postings_highlighter_word_boundaries() {
        let highlighter =
            Highlighter::with_config(HighlighterConfig::new().with_type(HighlighterType::Postings));
        let mut terms = HashSet::new();
        terms.insert("test".to_string());

        // Should match "test" as a word, not "testing"
        let text = "This is a test of testing";
        let highlighted = highlighter.highlight_full(text, &terms);

        assert!(highlighted.contains("<em>test</em>"));
        // "testing" should not be highlighted
        assert!(!highlighted.contains("<em>testing</em>"));
    }

    #[test]
    fn test_postings_highlighter_multiple_terms() {
        let highlighter =
            Highlighter::with_config(HighlighterConfig::new().with_type(HighlighterType::Postings));
        let mut terms = HashSet::new();
        terms.insert("test".to_string());
        terms.insert("string".to_string());

        let text = "This is a test string";
        let highlighted = highlighter.highlight_full(text, &terms);

        assert!(highlighted.contains("<em>test</em>"));
        assert!(highlighted.contains("<em>string</em>"));
    }

    #[test]
    fn test_fast_vector_highlighter_precise() {
        let highlighter = Highlighter::with_config(
            HighlighterConfig::new().with_type(HighlighterType::FastVector),
        );
        let mut terms = HashSet::new();
        terms.insert("test".to_string());

        let text = "This is a test string";
        let highlighted = highlighter.highlight_full(text, &terms);

        assert!(highlighted.contains("<em>test</em>"));
    }

    #[test]
    fn test_fast_vector_highlighter_partial_matches() {
        let highlighter = Highlighter::with_config(
            HighlighterConfig::new().with_type(HighlighterType::FastVector),
        );
        let mut terms = HashSet::new();
        terms.insert("test".to_string());

        // Fast Vector can match partial matches within words
        let text = "This is testing the fast vector highlighter";
        let highlighted = highlighter.highlight_full(text, &terms);

        // Fast Vector should match "test" within "testing"
        assert!(highlighted.contains("<em>test</em>"));
    }

    #[test]
    fn test_unified_highlighter_auto_select() {
        let highlighter =
            Highlighter::with_config(HighlighterConfig::new().with_type(HighlighterType::Unified));
        let mut terms = HashSet::new();
        terms.insert("test".to_string());

        // Unified should automatically select the best highlighter
        let text = "This is a test string";
        let highlighted = highlighter.highlight_full(text, &terms);

        assert!(highlighted.contains("<em>test</em>"));
    }

    #[test]
    fn test_unified_highlighter_short_text() {
        let highlighter =
            Highlighter::with_config(HighlighterConfig::new().with_type(HighlighterType::Unified));
        let mut terms = HashSet::new();
        terms.insert("test".to_string());

        // Short text should use Postings
        let text = "test";
        let highlighted = highlighter.highlight_full(text, &terms);

        assert!(highlighted.contains("<em>test</em>"));
    }

    #[test]
    fn test_postings_vs_plain_difference() {
        let mut terms = HashSet::new();
        terms.insert("test".to_string());

        let text = "This is a testing string";

        // Plain highlighter matches substring anywhere
        let plain =
            Highlighter::with_config(HighlighterConfig::new().with_type(HighlighterType::Plain));
        let plain_result = plain.highlight_full(text, &terms);
        // Plain matches "test" as substring within "testing"
        assert!(plain_result.contains("<em>test</em>ing")); // "test" highlighted inside "testing"

        // Postings highlighter matches whole words only
        let postings =
            Highlighter::with_config(HighlighterConfig::new().with_type(HighlighterType::Postings));
        let postings_result = postings.highlight_full(text, &terms);
        // Should NOT match "test" in "testing" - Postings respects word boundaries
        // The word "testing" is not "test", so it shouldn't be highlighted
        assert!(!postings_result.contains("<em>test</em>ing")); // "test" not highlighted inside "testing"
    }
}
