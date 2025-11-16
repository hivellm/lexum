//! Significant terms aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::Query;
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Scoring method for significant terms
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SignificantTermsScoring {
    /// Mutual Information scoring
    #[default]
    MutualInformation,
    /// Chi-square scoring
    ChiSquare,
    /// G-test scoring
    GTest,
    /// Percentage scoring (simple ratio)
    Percentage,
}

/// Significant terms aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SignificantTermsAggregation {
    /// Field to aggregate on
    pub field: String,
    /// Maximum number of buckets to return
    #[serde(default = "default_size")]
    pub size: usize,
    /// Background filter (optional query to define background set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_filter: Option<Query>,
    /// Scoring method
    #[serde(default)]
    pub scoring: SignificantTermsScoring,
    /// Minimum document count for a term to be included
    #[serde(default)]
    pub min_doc_count: usize,
}

fn default_size() -> usize {
    10
}

impl AggregationTrait for SignificantTermsAggregation {
    fn name(&self) -> &str {
        "significant_terms"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Count terms in foreground (current hits)
        let mut foreground_counts: HashMap<String, usize> = HashMap::new();
        let foreground_total = hits.len();

        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                let key = extract_term_key(field_value);
                *foreground_counts.entry(key).or_insert(0) += 1;
            }
        }

        // For background, we'll use all hits as background if no filter is specified
        // In a real implementation, background_filter would be evaluated against the full index
        // For now, we'll use the same hits as background (simplified)
        let background_counts = foreground_counts.clone();
        let background_total = foreground_total;

        // Calculate significance scores
        let mut scored_terms: Vec<(String, usize, f64)> = foreground_counts
            .into_iter()
            .filter(|(_, count)| *count >= self.min_doc_count)
            .map(|(term, foreground_count)| {
                let background_count = background_counts.get(&term).copied().unwrap_or(0);
                let score = calculate_significance(
                    foreground_count,
                    foreground_total,
                    background_count,
                    background_total,
                    self.scoring,
                );
                (term, foreground_count, score)
            })
            .collect();

        // Sort by significance score (descending)
        scored_terms.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        // Limit to size
        scored_terms.truncate(self.size);

        // Convert to buckets
        let buckets: Vec<Bucket> = scored_terms
            .into_iter()
            .map(|(key, count, _score)| {
                // Score is used for sorting, but not stored in bucket
                // In a full implementation, score would be stored as metadata
                Bucket::new(JsonValue::String(key), count)
            })
            .collect();

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            buckets,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge significant terms results
        // This is simplified - proper implementation would merge foreground/background counts
        // and recalculate significance scores
        let mut merged_counts: HashMap<String, usize> = HashMap::new();

        for result in results {
            if let AggregationResult::Buckets(bucket_result) = result {
                for bucket in bucket_result.buckets() {
                    if let JsonValue::String(key) = &bucket.key {
                        *merged_counts.entry(key.clone()).or_insert(0) += bucket.doc_count;
                    }
                }
            }
        }

        // Convert to buckets
        let mut buckets: Vec<Bucket> = merged_counts
            .into_iter()
            .filter(|(_, count)| *count >= self.min_doc_count)
            .map(|(key, count)| Bucket::new(JsonValue::String(key), count))
            .collect();

        // Sort by count (descending) - simplified merge
        buckets.sort_by(|a, b| b.doc_count.cmp(&a.doc_count));
        buckets.truncate(self.size);

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            buckets,
        )))
    }
}

impl SignificantTermsAggregation {
    /// Create new significant terms aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            size: default_size(),
            background_filter: None,
            scoring: SignificantTermsScoring::default(),
            min_doc_count: 1,
        }
    }

    /// Set size
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Set background filter
    pub fn with_background_filter(mut self, filter: Query) -> Self {
        self.background_filter = Some(filter);
        self
    }

    /// Set scoring method
    pub fn with_scoring(mut self, scoring: SignificantTermsScoring) -> Self {
        self.scoring = scoring;
        self
    }

    /// Set minimum document count
    pub fn with_min_doc_count(mut self, min_doc_count: usize) -> Self {
        self.min_doc_count = min_doc_count;
        self
    }
}

/// Extract term key from JSON value
fn extract_term_key(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        _ => value.to_string(),
    }
}

/// Calculate significance score
fn calculate_significance(
    foreground_count: usize,
    foreground_total: usize,
    background_count: usize,
    background_total: usize,
    scoring: SignificantTermsScoring,
) -> f64 {
    if foreground_total == 0 || background_total == 0 {
        return 0.0;
    }

    match scoring {
        SignificantTermsScoring::MutualInformation => calculate_mutual_information(
            foreground_count,
            foreground_total,
            background_count,
            background_total,
        ),
        SignificantTermsScoring::ChiSquare => calculate_chi_square(
            foreground_count,
            foreground_total,
            background_count,
            background_total,
        ),
        SignificantTermsScoring::GTest => calculate_g_test(
            foreground_count,
            foreground_total,
            background_count,
            background_total,
        ),
        SignificantTermsScoring::Percentage => calculate_percentage(
            foreground_count,
            foreground_total,
            background_count,
            background_total,
        ),
    }
}

/// Calculate Mutual Information score
fn calculate_mutual_information(
    foreground_count: usize,
    foreground_total: usize,
    background_count: usize,
    background_total: usize,
) -> f64 {
    let fg_count = foreground_count as f64;
    let fg_total = foreground_total as f64;
    let bg_count = background_count as f64;
    let bg_total = background_total as f64;
    let total = fg_total + bg_total;

    if total == 0.0 || fg_count == 0.0 {
        return 0.0;
    }

    // P(term | foreground)
    let p_term_fg = fg_count / fg_total;
    // P(foreground)
    let p_fg = fg_total / total;
    // P(term)
    let p_term = (fg_count + bg_count) / total;

    if p_term == 0.0 || p_fg == 0.0 {
        return 0.0;
    }

    // Mutual Information: log2(P(term|fg) / P(term))
    if p_term_fg > 0.0 && p_term > 0.0 {
        (p_term_fg / p_term).log2() * p_term_fg * p_fg
    } else {
        0.0
    }
}

/// Calculate Chi-square score
fn calculate_chi_square(
    foreground_count: usize,
    foreground_total: usize,
    background_count: usize,
    background_total: usize,
) -> f64 {
    let fg_count = foreground_count as f64;
    let fg_total = foreground_total as f64;
    let bg_count = background_count as f64;
    let bg_total = background_total as f64;
    let total = fg_total + bg_total;

    if total == 0.0 {
        return 0.0;
    }

    // Expected counts
    let expected_fg = (fg_count + bg_count) * (fg_total / total);
    let expected_bg = (fg_count + bg_count) * (bg_total / total);

    if expected_fg == 0.0 || expected_bg == 0.0 {
        return 0.0;
    }

    // Chi-square: sum of (observed - expected)^2 / expected
    let chi_fg = ((fg_count - expected_fg).powi(2)) / expected_fg;
    let chi_bg = ((bg_count - expected_bg).powi(2)) / expected_bg;

    chi_fg + chi_bg
}

/// Calculate G-test score (likelihood ratio test)
fn calculate_g_test(
    foreground_count: usize,
    foreground_total: usize,
    background_count: usize,
    background_total: usize,
) -> f64 {
    let fg_count = foreground_count as f64;
    let fg_total = foreground_total as f64;
    let bg_count = background_count as f64;
    let bg_total = background_total as f64;
    let total = fg_total + bg_total;

    if total == 0.0 || fg_count == 0.0 || bg_count == 0.0 {
        return 0.0;
    }

    // Expected proportions
    let p_fg = fg_total / total;
    let p_bg = bg_total / total;
    let p_term = (fg_count + bg_count) / total;

    if p_term == 0.0 {
        return 0.0;
    }

    // Observed proportions
    let p_fg_term = fg_count / fg_total;
    let p_bg_term = bg_count / bg_total;

    // G-test: 2 * sum(observed * ln(observed / expected))
    let g_fg = if p_fg_term > 0.0 {
        fg_count * (p_fg_term / (p_term * p_fg)).ln()
    } else {
        0.0
    };

    let g_bg = if p_bg_term > 0.0 {
        bg_count * (p_bg_term / (p_term * p_bg)).ln()
    } else {
        0.0
    };

    2.0 * (g_fg + g_bg)
}

/// Calculate percentage score (simple ratio)
fn calculate_percentage(
    foreground_count: usize,
    foreground_total: usize,
    background_count: usize,
    background_total: usize,
) -> f64 {
    let fg_count = foreground_count as f64;
    let fg_total = foreground_total as f64;
    let bg_count = background_count as f64;
    let bg_total = background_total as f64;

    if fg_total == 0.0 || bg_total == 0.0 {
        return 0.0;
    }

    let fg_percentage = fg_count / fg_total;
    let bg_percentage = bg_count / bg_total;

    if bg_percentage == 0.0 {
        return fg_percentage;
    }

    // Ratio of foreground to background percentage
    fg_percentage / bg_percentage
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::field_cache::FieldCache;
    use crate::search::result::SearchHit;
    use crate::types::{DocumentId, Score};

    fn create_test_hit(id: &str, field: &str, value: &str) -> SearchHit {
        SearchHit {
            id: DocumentId::new(id),
            score: Score::new(1.0),
            source: serde_json::json!({ field: value }),
        }
    }

    #[test]
    fn test_significant_terms_aggregation_basic() {
        let agg = SignificantTermsAggregation::new("category");
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "category", "electronics"),
            create_test_hit("2", "category", "electronics"),
            create_test_hit("3", "category", "clothing"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert!(!bucket_result.buckets().is_empty());
            // Electronics should appear more frequently
            let buckets = bucket_result.buckets();
            assert!(buckets[0].doc_count >= buckets[1].doc_count);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_significant_terms_aggregation_size_limit() {
        let agg = SignificantTermsAggregation::new("category").with_size(2);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "category", "a"),
            create_test_hit("2", "category", "b"),
            create_test_hit("3", "category", "c"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert!(bucket_result.buckets().len() <= 2);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_significant_terms_aggregation_min_doc_count() {
        let agg = SignificantTermsAggregation::new("category").with_min_doc_count(2);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "category", "electronics"),
            create_test_hit("2", "category", "electronics"),
            create_test_hit("3", "category", "clothing"), // Only 1 occurrence
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            // Should only include electronics (appears 2 times)
            let buckets = bucket_result.buckets();
            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets[0].doc_count, 2);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_significant_terms_aggregation_different_scoring() {
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "category", "electronics"),
            create_test_hit("2", "category", "electronics"),
            create_test_hit("3", "category", "clothing"),
        ];

        // Test different scoring methods
        for scoring in [
            SignificantTermsScoring::MutualInformation,
            SignificantTermsScoring::ChiSquare,
            SignificantTermsScoring::GTest,
            SignificantTermsScoring::Percentage,
        ] {
            let agg = SignificantTermsAggregation::new("category").with_scoring(scoring);
            let result = agg.execute(&hits, &field_cache).unwrap();
            assert!(matches!(result, AggregationResult::Buckets(_)));
        }
    }

    #[test]
    fn test_significant_terms_aggregation_merge() {
        let agg = SignificantTermsAggregation::new("category");
        let field_cache = FieldCache::new();

        let hits1 = vec![create_test_hit("1", "category", "electronics")];
        let hits2 = vec![create_test_hit("2", "category", "electronics")];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            assert_eq!(bucket_result.buckets().len(), 1);
            assert_eq!(bucket_result.buckets()[0].doc_count, 2);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_significant_terms_aggregation_empty_hits() {
        let agg = SignificantTermsAggregation::new("category");
        let field_cache = FieldCache::new();
        let hits = vec![];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert!(bucket_result.buckets().is_empty());
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_calculate_mutual_information() {
        // Test with clear difference
        let score = calculate_mutual_information(10, 20, 2, 100);
        assert!(score > 0.0);

        // Test with equal distributions
        let score_equal = calculate_mutual_information(5, 10, 5, 10);
        assert!(score_equal >= 0.0);
    }

    #[test]
    fn test_calculate_chi_square() {
        // Test with clear difference
        let score = calculate_chi_square(10, 20, 2, 100);
        assert!(score > 0.0);

        // Test with equal distributions
        let score_equal = calculate_chi_square(5, 10, 5, 10);
        assert!(score_equal >= 0.0);
    }

    #[test]
    fn test_calculate_percentage() {
        // Test with clear difference
        let score = calculate_percentage(10, 20, 2, 100);
        assert!(score > 1.0); // Should be higher than background

        // Test with equal distributions
        let score_equal = calculate_percentage(5, 10, 5, 10);
        assert!((score_equal - 1.0).abs() < 0.01);
    }
}
