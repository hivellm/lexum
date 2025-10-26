//! Query type definitions

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Main query enum
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Query {
    /// Match query (full-text search)
    Match(MatchQuery),
    /// Term query (exact match)
    Term(TermQuery),
    /// Range query (numeric/date ranges)
    Range(RangeQuery),
    /// Boolean query (combinations)
    Bool(BoolQuery),
    /// Fuzzy query (approximate matching)
    Fuzzy(FuzzyQuery),
    /// Phrase query (exact phrase matching)
    Phrase(PhraseQuery),
    /// Wildcard query (prefix, suffix, contains)
    Wildcard(WildcardQuery),
    /// Regex query (regular expression matching)
    Regex(RegexQuery),
    /// Match all documents
    MatchAll,
    /// More Like This query (find similar documents)
    MoreLikeThis(MoreLikeThisQuery),
    /// Nested query (for nested object fields)
    Nested(NestedQuery),
    /// Function score query (custom scoring)
    FunctionScore(FunctionScoreQuery),
    /// Geo distance query (for geographic data)
    GeoDistance(GeoDistanceQuery),
    /// Script query (custom script evaluation)
    Script(ScriptQuery),
}

/// Match query for full-text search
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MatchQuery {
    /// Field to search
    pub field: String,
    /// Query text
    pub query: String,
}

impl MatchQuery {
    /// Create new match query
    pub fn new(field: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            query: query.into(),
        }
    }
}

/// Term query for exact matching
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TermQuery {
    /// Field to search
    pub field: String,
    /// Term value
    pub value: String,
}

impl TermQuery {
    /// Create new term query
    pub fn new(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
        }
    }
}

/// Range query for numeric/date ranges
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RangeQuery {
    /// Field to search
    pub field: String,
    /// Greater than or equal
    pub gte: Option<serde_json::Value>,
    /// Less than or equal
    pub lte: Option<serde_json::Value>,
    /// Greater than
    pub gt: Option<serde_json::Value>,
    /// Less than
    pub lt: Option<serde_json::Value>,
}

impl RangeQuery {
    /// Create new range query
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            gte: None,
            lte: None,
            gt: None,
            lt: None,
        }
    }

    /// Set greater than or equal
    pub fn gte(mut self, value: serde_json::Value) -> Self {
        self.gte = Some(value);
        self
    }

    /// Set less than or equal
    pub fn lte(mut self, value: serde_json::Value) -> Self {
        self.lte = Some(value);
        self
    }

    /// Set greater than
    pub fn gt(mut self, value: serde_json::Value) -> Self {
        self.gt = Some(value);
        self
    }

    /// Set less than
    pub fn lt(mut self, value: serde_json::Value) -> Self {
        self.lt = Some(value);
        self
    }
}

/// Boolean query for combining queries
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct BoolQuery {
    /// Must clauses (all required)
    #[serde(default)]
    pub must: Vec<Query>,
    /// Should clauses (at least one should match)
    #[serde(default)]
    pub should: Vec<Query>,
    /// Must not clauses (exclude)
    #[serde(default)]
    pub must_not: Vec<Query>,
    /// Filter clauses (must match, but don't affect score)
    #[serde(default)]
    pub filter: Vec<Query>,
}

impl BoolQuery {
    /// Create new boolean query
    pub fn new() -> Self {
        Self::default()
    }

    /// Add must clause
    pub fn must(mut self, query: Query) -> Self {
        self.must.push(query);
        self
    }

    /// Add should clause
    pub fn should(mut self, query: Query) -> Self {
        self.should.push(query);
        self
    }

    /// Add must_not clause
    pub fn must_not(mut self, query: Query) -> Self {
        self.must_not.push(query);
        self
    }

    /// Add filter clause
    pub fn filter(mut self, query: Query) -> Self {
        self.filter.push(query);
        self
    }
}

/// Fuzzy query for approximate matching with edit distance
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FuzzyQuery {
    /// Field to search
    pub field: String,
    /// Term to match fuzzily
    pub value: String,
    /// Maximum edit distance (0-2, default: 2)
    #[serde(default = "default_fuzzy_distance")]
    pub fuzziness: u8,
    /// Whether to include transpositions in edit distance
    #[serde(default = "default_true")]
    pub transpositions: bool,
    /// Minimum prefix length that must match exactly
    #[serde(default)]
    pub prefix_length: u32,
}

fn default_fuzzy_distance() -> u8 {
    2
}

fn default_true() -> bool {
    true
}

impl FuzzyQuery {
    /// Create new fuzzy query with default fuzziness of 2
    pub fn new(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
            fuzziness: 2,
            transpositions: true,
            prefix_length: 0,
        }
    }

    /// Set fuzziness (maximum edit distance 0-2)
    pub fn fuzziness(mut self, distance: u8) -> Self {
        self.fuzziness = distance.min(2);
        self
    }

    /// Set whether to include transpositions
    pub fn transpositions(mut self, enabled: bool) -> Self {
        self.transpositions = enabled;
        self
    }

    /// Set prefix length that must match exactly
    pub fn prefix_length(mut self, length: u32) -> Self {
        self.prefix_length = length;
        self
    }
}

/// Phrase query for exact phrase matching
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PhraseQuery {
    /// Field to search
    pub field: String,
    /// Phrase to match (terms in exact order)
    pub phrase: String,
    /// Maximum allowed distance between terms (slop)
    #[serde(default)]
    pub slop: u32,
}

impl PhraseQuery {
    /// Create new phrase query with no slop (exact phrase)
    pub fn new(field: impl Into<String>, phrase: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            phrase: phrase.into(),
            slop: 0,
        }
    }

    /// Set slop (maximum distance between terms)
    ///
    /// Slop allows terms to be in different positions.
    /// For example, with slop=1, "quick fox" will match "quick brown fox"
    pub fn slop(mut self, slop: u32) -> Self {
        self.slop = slop;
        self
    }
}

/// Wildcard query for pattern matching
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WildcardQuery {
    /// Field to search
    pub field: String,
    /// Wildcard pattern (* for any characters, ? for single character)
    pub pattern: String,
}

impl WildcardQuery {
    /// Create new wildcard query
    pub fn new(field: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            pattern: pattern.into(),
        }
    }
}

/// Regex query for regular expression matching
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegexQuery {
    /// Field to search
    pub field: String,
    /// Regular expression pattern
    pub pattern: String,
    /// Case sensitivity
    #[serde(default)]
    pub case_sensitive: bool,
}

impl RegexQuery {
    /// Create new regex query
    pub fn new(field: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            pattern: pattern.into(),
            case_sensitive: false,
        }
    }

    /// Set case sensitivity
    pub fn case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }
}

/// More Like This query for finding similar documents
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MoreLikeThisQuery {
    /// Fields to analyze for similarity
    pub fields: Vec<String>,
    /// Like text (document content to find similar to)
    pub like: String,
    /// Minimum term frequency
    #[serde(default = "default_min_term_freq")]
    pub min_term_freq: u32,
    /// Maximum query terms
    #[serde(default = "default_max_query_terms")]
    pub max_query_terms: u32,
    /// Minimum document frequency
    #[serde(default = "default_min_doc_freq")]
    pub min_doc_freq: u32,
    /// Maximum document frequency
    #[serde(default = "default_max_doc_freq")]
    pub max_doc_freq: u32,
    /// Minimum word length
    #[serde(default = "default_min_word_length")]
    pub min_word_length: u32,
    /// Maximum word length
    #[serde(default = "default_max_word_length")]
    pub max_word_length: u32,
}

fn default_min_term_freq() -> u32 { 1 }
fn default_max_query_terms() -> u32 { 25 }
fn default_min_doc_freq() -> u32 { 5 }
fn default_max_doc_freq() -> u32 { 0 }
fn default_min_word_length() -> u32 { 0 }
fn default_max_word_length() -> u32 { 0 }

impl MoreLikeThisQuery {
    /// Create new More Like This query
    pub fn new(fields: Vec<String>, like: impl Into<String>) -> Self {
        Self {
            fields,
            like: like.into(),
            min_term_freq: 1,
            max_query_terms: 25,
            min_doc_freq: 5,
            max_doc_freq: 0,
            min_word_length: 0,
            max_word_length: 0,
        }
    }
}

/// Nested query for searching within nested objects
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NestedQuery {
    /// Path to the nested field
    pub path: String,
    /// Query to execute within the nested context
    pub query: Box<Query>,
    /// Score mode for nested queries
    #[serde(default)]
    pub score_mode: NestedScoreMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NestedScoreMode {
    /// Average score of all matching nested objects
    Avg,
    /// Sum of all matching nested object scores
    Sum,
    /// Maximum score among matching nested objects
    Max,
    /// Minimum score among matching nested objects
    Min,
    /// No scoring (filter only)
    None,
}

impl Default for NestedScoreMode {
    fn default() -> Self {
        Self::Avg
    }
}

impl NestedQuery {
    /// Create new nested query
    pub fn new(path: impl Into<String>, query: Query) -> Self {
        Self {
            path: path.into(),
            query: Box::new(query),
            score_mode: NestedScoreMode::Avg,
        }
    }
}

/// Function score query for custom scoring
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FunctionScoreQuery {
    /// Base query to score
    pub query: Box<Query>,
    /// Functions to apply for scoring
    pub functions: Vec<ScoreFunction>,
    /// Score mode
    #[serde(default)]
    pub score_mode: FunctionScoreMode,
    /// Boost mode
    #[serde(default)]
    pub boost_mode: FunctionBoostMode,
    /// Maximum boost value
    #[serde(default = "default_max_boost")]
    pub max_boost: f32,
    /// Minimum score threshold
    #[serde(default)]
    pub min_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FunctionScoreMode {
    /// Multiply function scores
    Multiply,
    /// Sum function scores
    Sum,
    /// Average function scores
    Avg,
    /// First function score
    First,
    /// Maximum function score
    Max,
    /// Minimum function score
    Min,
}

impl Default for FunctionScoreMode {
    fn default() -> Self {
        Self::Multiply
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FunctionBoostMode {
    /// Multiply boost with query score
    Multiply,
    /// Replace query score with boost
    Replace,
    /// Sum boost with query score
    Sum,
    /// Average boost with query score
    Avg,
    /// Maximum of boost and query score
    Max,
    /// Minimum of boost and query score
    Min,
}

impl Default for FunctionBoostMode {
    fn default() -> Self {
        Self::Multiply
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScoreFunction {
    /// Field value factor
    FieldValueFactor(FieldValueFactor),
    /// Linear decay function
    Linear(DecayFunction),
    /// Exponential decay function
    Exp(DecayFunction),
    /// Gaussian decay function
    Gauss(DecayFunction),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FieldValueFactor {
    /// Field to use for scoring
    pub field: String,
    /// Factor to multiply field value by
    #[serde(default = "default_factor")]
    pub factor: f32,
    /// Modifier to apply to field value
    #[serde(default)]
    pub modifier: FieldModifier,
    /// Missing value to use if field is missing
    #[serde(default)]
    pub missing: Option<f32>,
}

fn default_factor() -> f32 { 1.0 }

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldModifier {
    None,
    Log,
    Log1p,
    Log2p,
    Ln,
    Ln1p,
    Ln2p,
    Square,
    Sqrt,
    Reciprocal,
}

impl Default for FieldModifier {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DecayFunction {
    /// Field to use for decay
    pub field: String,
    /// Origin point for decay
    pub origin: serde_json::Value,
    /// Scale for decay
    pub scale: serde_json::Value,
    /// Decay factor
    #[serde(default = "default_decay")]
    pub decay: f32,
    /// Offset
    #[serde(default)]
    pub offset: Option<serde_json::Value>,
}

fn default_decay() -> f32 { 0.5 }
fn default_max_boost() -> f32 { 3.4028235e38 }

impl FunctionScoreQuery {
    /// Create new function score query
    pub fn new(query: Query) -> Self {
        Self {
            query: Box::new(query),
            functions: Vec::new(),
            score_mode: FunctionScoreMode::Multiply,
            boost_mode: FunctionBoostMode::Multiply,
            max_boost: 3.4028235e38,
            min_score: None,
        }
    }
}

/// Geo distance query for geographic searches
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoDistanceQuery {
    /// Field containing geo point
    pub field: String,
    /// Distance from point
    pub distance: String,
    /// Center point (lat, lon)
    pub location: GeoPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoPoint {
    /// Latitude
    pub lat: f64,
    /// Longitude
    pub lon: f64,
}

impl GeoDistanceQuery {
    /// Create new geo distance query
    pub fn new(field: impl Into<String>, distance: impl Into<String>, lat: f64, lon: f64) -> Self {
        Self {
            field: field.into(),
            distance: distance.into(),
            location: GeoPoint { lat, lon },
        }
    }
}

/// Script query for custom script evaluation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScriptQuery {
    /// Script source code
    pub source: String,
    /// Script parameters
    #[serde(default)]
    pub params: std::collections::HashMap<String, serde_json::Value>,
}

impl ScriptQuery {
    /// Create new script query
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            params: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_query() {
        let query = MatchQuery::new("title", "search terms");
        assert_eq!(query.field, "title");
        assert_eq!(query.query, "search terms");
    }

    #[test]
    fn test_term_query() {
        let query = TermQuery::new("status", "active");
        assert_eq!(query.field, "status");
        assert_eq!(query.value, "active");
    }

    #[test]
    fn test_range_query() {
        let query = RangeQuery::new("age")
            .gte(serde_json::json!(18))
            .lte(serde_json::json!(65));

        assert_eq!(query.field, "age");
        assert!(query.gte.is_some());
        assert!(query.lte.is_some());
    }

    #[test]
    fn test_bool_query() {
        let bool_query = BoolQuery::new()
            .must(Query::Term(TermQuery::new("status", "active")))
            .should(Query::Match(MatchQuery::new("title", "test")));

        assert_eq!(bool_query.must.len(), 1);
        assert_eq!(bool_query.should.len(), 1);
    }

    #[test]
    fn test_fuzzy_query() {
        let query = FuzzyQuery::new("name", "jhon")
            .fuzziness(1)
            .prefix_length(2);

        assert_eq!(query.field, "name");
        assert_eq!(query.value, "jhon");
        assert_eq!(query.fuzziness, 1);
        assert_eq!(query.prefix_length, 2);
        assert!(query.transpositions);
    }

    #[test]
    fn test_fuzzy_query_max_distance() {
        let query = FuzzyQuery::new("name", "test").fuzziness(10);
        assert_eq!(query.fuzziness, 2); // Should be capped at 2
    }

    #[test]
    fn test_phrase_query() {
        let query = PhraseQuery::new("content", "quick brown fox").slop(1);

        assert_eq!(query.field, "content");
        assert_eq!(query.phrase, "quick brown fox");
        assert_eq!(query.slop, 1);
    }

    #[test]
    fn test_wildcard_query() {
        let query = WildcardQuery::new("name", "test*");
        assert_eq!(query.field, "name");
        assert_eq!(query.pattern, "test*");
    }

    #[test]
    fn test_regex_query() {
        let query = RegexQuery::new("content", "[A-Z][a-z]+").case_sensitive(true);

        assert_eq!(query.field, "content");
        assert_eq!(query.pattern, "[A-Z][a-z]+");
        assert!(query.case_sensitive);
    }

    #[test]
    fn test_phrase_query_with_slop() {
        let query = PhraseQuery::new("content", "quick fox").slop(2);
        assert_eq!(query.slop, 2);
    }

    #[test]
    fn test_more_like_this_query() {
        let query = MoreLikeThisQuery::new(vec!["title".to_string(), "content".to_string()], "sample text");

        assert_eq!(query.fields, vec!["title", "content"]);
        assert_eq!(query.like, "sample text");
        assert_eq!(query.min_term_freq, 1);
        assert_eq!(query.max_query_terms, 25);
    }

    #[test]
    fn test_nested_query() {
        let inner_query = Query::Term(TermQuery::new("nested.field", "value"));
        let nested_query = NestedQuery::new("nested", inner_query);

        assert_eq!(nested_query.path, "nested");
        assert!(matches!(nested_query.query.as_ref(), Query::Term(_)));
        assert!(matches!(nested_query.score_mode, NestedScoreMode::Avg));
    }

    #[test]
    fn test_function_score_query() {
        let base_query = Query::Match(MatchQuery::new("title", "test"));
        let function_score = FunctionScoreQuery::new(base_query);

        assert!(matches!(function_score.query.as_ref(), Query::Match(_)));
        assert!(function_score.functions.is_empty());
        assert!(matches!(function_score.score_mode, FunctionScoreMode::Multiply));
        assert!(matches!(function_score.boost_mode, FunctionBoostMode::Multiply));
    }

    #[test]
    fn test_geo_distance_query() {
        let geo_query = GeoDistanceQuery::new("location", "10km", 40.7128, -74.0060);

        assert_eq!(geo_query.field, "location");
        assert_eq!(geo_query.distance, "10km");
        assert_eq!(geo_query.location.lat, 40.7128);
        assert_eq!(geo_query.location.lon, -74.0060);
    }

    #[test]
    fn test_script_query() {
        let script_query = ScriptQuery::new("doc['field'].value > 10");

        assert_eq!(script_query.source, "doc['field'].value > 10");
        assert!(script_query.params.is_empty());
    }
}
