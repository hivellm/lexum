//! Regex query cache for compiled regex patterns
//!
//! This module provides caching for compiled regex queries to avoid
//! recompiling the same patterns repeatedly.

use crate::error::{Error, Result};
use lru::LruCache;
use parking_lot::Mutex;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Arc;
use tantivy::query::RegexQuery as TantivyRegexQuery;
use tantivy::schema::Field;

/// Cache key for regex queries
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RegexCacheKey {
    /// Pattern string
    pattern: String,
    /// Field ID
    field_id: u32,
    /// Case sensitivity flag
    case_sensitive: bool,
}

impl RegexCacheKey {
    /// Create a new cache key
    fn new(pattern: String, field: Field, case_sensitive: bool) -> Self {
        Self {
            pattern,
            field_id: field.field_id(),
            case_sensitive,
        }
    }
}

/// Regex query cache
#[derive(Debug, Clone)]
pub struct RegexCache {
    /// LRU cache for compiled regex queries
    cache: Arc<Mutex<LruCache<u64, Arc<TantivyRegexQuery>>>>,
    /// Maximum pattern length (safety limit)
    max_pattern_length: usize,
    /// Maximum number of alternations (safety limit)
    max_alternations: usize,
    /// Whether caching is enabled
    enabled: bool,
}

impl RegexCache {
    /// Create new regex cache with default settings
    ///
    /// Defaults:
    /// - Max size: 500 entries
    /// - Max pattern length: 1000 characters
    /// - Max alternations: 100
    pub fn new() -> Self {
        Self::with_capacity(500)
    }

    /// Create regex cache with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity.max(1)).unwrap_or(NonZeroUsize::new(1).unwrap());
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
            max_pattern_length: 1000,
            max_alternations: 100,
            enabled: true,
        }
    }

    /// Create disabled regex cache
    pub fn disabled() -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(1).unwrap()))),
            max_pattern_length: 1000,
            max_alternations: 100,
            enabled: false,
        }
    }

    /// Set maximum pattern length (safety limit)
    pub fn with_max_pattern_length(mut self, max_length: usize) -> Self {
        self.max_pattern_length = max_length;
        self
    }

    /// Set maximum alternations (safety limit)
    pub fn with_max_alternations(mut self, max_alternations: usize) -> Self {
        self.max_alternations = max_alternations;
        self
    }

    /// Validate regex pattern for safety
    pub fn validate_pattern(&self, pattern: &str) -> Result<()> {
        // Check pattern length
        if pattern.len() > self.max_pattern_length {
            return Err(Error::Config(format!(
                "Regex pattern too long: {} characters (max: {})",
                pattern.len(),
                self.max_pattern_length
            )));
        }

        // Check for excessive alternations (simple heuristic)
        let alternation_count = pattern.matches('|').count();
        if alternation_count > self.max_alternations {
            return Err(Error::Config(format!(
                "Regex pattern has too many alternations: {} (max: {})",
                alternation_count, self.max_alternations
            )));
        }

        // Check for potentially dangerous patterns (ReDoS protection)
        // Look for nested quantifiers like (a+)+ or (a*)*
        if self.detect_redos_pattern(pattern) {
            return Err(Error::Config(
                "Regex pattern may be vulnerable to ReDoS attacks".to_string(),
            ));
        }

        Ok(())
    }

    /// Detect potentially dangerous ReDoS patterns
    fn detect_redos_pattern(&self, pattern: &str) -> bool {
        // Simple heuristic: look for nested quantifiers
        // This is a basic check - a full implementation would use a proper regex parser
        let chars: Vec<char> = pattern.chars().collect();
        for i in 0..chars.len().saturating_sub(3) {
            // Look for patterns like (a+)+ or (a*)*
            if chars[i] == '(' && i + 3 < chars.len() {
                if (chars[i + 1] == 'a' || chars[i + 1] == '.')
                    && (chars[i + 2] == '+' || chars[i + 2] == '*')
                    && chars[i + 3] == ')'
                {
                    // Check if followed by another quantifier
                    if i + 4 < chars.len() && (chars[i + 4] == '+' || chars[i + 4] == '*') {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Get or compile regex query
    pub fn get_or_compile(
        &self,
        pattern: &str,
        field: Field,
        case_sensitive: bool,
    ) -> Result<Arc<TantivyRegexQuery>> {
        // Validate pattern for safety
        self.validate_pattern(pattern)?;

        if !self.enabled {
            // If caching is disabled, compile directly
            let final_pattern = if case_sensitive {
                pattern.to_string()
            } else {
                format!("(?i){}", pattern)
            };
            return TantivyRegexQuery::from_pattern(&final_pattern, field)
                .map_err(|e| Error::Config(format!("Invalid regex pattern: {e}")))
                .map(Arc::new);
        }

        // Create cache key
        let key = RegexCacheKey::new(pattern.to_string(), field, case_sensitive);

        // Hash the key for cache lookup
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();

        // Check cache
        {
            let mut cache = self.cache.lock();
            if let Some(cached_query) = cache.get(&hash) {
                return Ok(cached_query.clone());
            }
        }

        // Compile regex query
        let final_pattern = if case_sensitive {
            pattern.to_string()
        } else {
            format!("(?i){}", pattern)
        };

        let compiled_query = TantivyRegexQuery::from_pattern(&final_pattern, field)
            .map_err(|e| Error::Config(format!("Invalid regex pattern: {e}")))?;

        let compiled_query = Arc::new(compiled_query);

        // Store in cache
        {
            let mut cache = self.cache.lock();
            cache.put(hash, compiled_query.clone());
        }

        Ok(compiled_query)
    }

    /// Clear the regex cache
    pub fn clear(&self) {
        let mut cache = self.cache.lock();
        cache.clear();
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        let cache = self.cache.lock();
        cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        let cache = self.cache.lock();
        cache.is_empty()
    }
}

impl Default for RegexCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::schema::*;

    #[test]
    fn test_regex_cache_creation() {
        let cache = RegexCache::new();
        assert!(cache.enabled);
        assert_eq!(cache.max_pattern_length, 1000);
    }

    #[test]
    fn test_regex_cache_validation() {
        let cache = RegexCache::new().with_max_pattern_length(10);

        // Should fail - pattern too long
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("test", TEXT);
        let schema = schema_builder.build();
        let field = schema.get_field("test").unwrap();
        assert!(cache
            .get_or_compile("a".repeat(20).as_str(), field, false)
            .is_err());

        // Should succeed - pattern within limit
        assert!(cache.get_or_compile("test", field, false).is_ok());
    }

    #[test]
    fn test_regex_cache_caching() {
        let cache = RegexCache::new();
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("test", TEXT | STORED);
        let schema = schema_builder.build();
        let field = schema.get_field("test").unwrap();

        // First call - should compile
        let query1 = cache.get_or_compile("test.*", field, false).unwrap();
        assert_eq!(cache.len(), 1);

        // Second call - should use cache
        let query2 = cache.get_or_compile("test.*", field, false).unwrap();
        assert_eq!(cache.len(), 1);
        assert!(Arc::ptr_eq(&query1, &query2));
    }
}

