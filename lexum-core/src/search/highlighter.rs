//! Text highlighter for search results
//!
//! This module provides functionality to highlight matching terms in search results
//! with configurable fragment sizes and multiple fragments per field.

use std::collections::HashSet;

/// Highlighter configuration
#[derive(Debug, Clone)]
pub struct HighlighterConfig {
    /// Pre-tag for highlighting (e.g., "<em>")
    pub pre_tag: String,
    /// Post-tag for highlighting (e.g., "</em>")
    pub post_tag: String,
    /// Maximum fragment size in characters
    pub fragment_size: usize,
    /// Maximum number of fragments per field
    pub max_fragments: usize,
    /// Number of characters before match to include
    pub fragment_margin: usize,
}

impl Default for HighlighterConfig {
    fn default() -> Self {
        Self {
            pre_tag: "<em>".to_string(),
            post_tag: "</em>".to_string(),
            fragment_size: 100,
            max_fragments: 3,
            fragment_margin: 20,
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

        // Find all match positions
        let matches = self.find_matches(text, query_terms);

        if matches.is_empty() {
            return vec![text.to_string()];
        }

        // Generate fragments
        self.generate_fragments(text, &matches)
    }

    /// Find all match positions in text
    fn find_matches(&self, text: &str, query_terms: &HashSet<String>) -> Vec<MatchPosition> {
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

    /// Generate fragments from matches
    fn generate_fragments(&self, text: &str, matches: &[MatchPosition]) -> Vec<String> {
        if matches.is_empty() {
            return vec![text.to_string()];
        }

        let mut fragments = Vec::new();
        let mut used_ranges = Vec::new();

        for (idx, mat) in matches.iter().enumerate() {
            if fragments.len() >= self.config.max_fragments {
                break;
            }

            // Calculate fragment boundaries
            let fragment_start = mat
                .start
                .saturating_sub(self.config.fragment_margin)
                .max(0);
            let fragment_end = (mat.end + self.config.fragment_margin).min(text.len());

            // Check if this fragment overlaps with already used ranges
            let overlaps = used_ranges.iter().any(|(start, end)| {
                fragment_start < *end && fragment_end > *start
            });

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
            .filter(|m| {
                m.start >= fragment_offset
                    && m.end <= fragment_offset + fragment.len()
            })
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

        let matches = self.find_matches(text, query_terms);
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
}

