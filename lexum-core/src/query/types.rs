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
    /// Multi-match query (search across multiple fields)
    MultiMatch(MultiMatchQuery),
    /// Constant score query (fixed score for all matches)
    ConstantScore(ConstantScoreQuery),
    /// Dis Max query (best matching query with tie breaker)
    DisMax(DisMaxQuery),
    /// Common terms query (separates low/high frequency terms)
    CommonTerms(CommonTermsQuery),
}

/// Match query for full-text search
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MatchQuery {
    /// Field to search
    pub field: String,
    /// Query text
    pub query: String,
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

fn default_boost() -> f32 {
    1.0
}

impl MatchQuery {
    /// Create new match query
    pub fn new(field: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            query: query.into(),
            boost: 1.0,
        }
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }
}

/// Term query for exact matching
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TermQuery {
    /// Field to search
    pub field: String,
    /// Term value
    pub value: String,
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

impl TermQuery {
    /// Create new term query
    pub fn new(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
            boost: 1.0,
        }
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
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
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
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
            boost: 1.0,
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

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
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
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
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
            boost: 1.0,
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

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
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
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

impl PhraseQuery {
    /// Create new phrase query with no slop (exact phrase)
    pub fn new(field: impl Into<String>, phrase: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            phrase: phrase.into(),
            slop: 0,
            boost: 1.0,
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

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
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
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

impl WildcardQuery {
    /// Create new wildcard query
    pub fn new(field: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            pattern: pattern.into(),
            boost: 1.0,
        }
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
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
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

impl RegexQuery {
    /// Create new regex query
    pub fn new(field: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            pattern: pattern.into(),
            case_sensitive: false,
            boost: 1.0,
        }
    }

    /// Set case sensitivity
    pub fn case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
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

fn default_min_term_freq() -> u32 {
    1
}
fn default_max_query_terms() -> u32 {
    25
}
fn default_min_doc_freq() -> u32 {
    5
}
fn default_max_doc_freq() -> u32 {
    0
}
fn default_min_word_length() -> u32 {
    0
}
fn default_max_word_length() -> u32 {
    0
}

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

/// Score mode for nested queries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum NestedScoreMode {
    /// Average score of all matching nested objects
    #[default]
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

/// Score mode for function score queries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum FunctionScoreMode {
    /// Multiply function scores
    #[default]
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

/// Boost mode for function score queries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum FunctionBoostMode {
    /// Multiply boost with query score
    #[default]
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

/// Score functions for function score queries
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

/// Field value factor for scoring
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

fn default_factor() -> f32 {
    1.0
}

/// Field value modifiers for scoring
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum FieldModifier {
    /// No modification
    #[default]
    None,
    /// Logarithmic modification
    Log,
    /// Logarithmic modification (log(1 + value))
    Log1p,
    /// Logarithmic modification (log(2 + value))
    Log2p,
    /// Natural logarithm
    Ln,
    /// Natural logarithm (ln(1 + value))
    Ln1p,
    /// Natural logarithm (ln(2 + value))
    Ln2p,
    /// Square the value
    Square,
    /// Square root of the value
    Sqrt,
    /// Reciprocal of the value
    Reciprocal,
}

/// Decay function for scoring based on distance
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

fn default_decay() -> f32 {
    0.5
}
fn default_max_boost() -> f32 {
    3.4028235e38
}

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

/// Geographic point with latitude and longitude
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

/// Multi-match query for searching across multiple fields
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MultiMatchQuery {
    /// Fields to search across
    pub fields: Vec<String>,
    /// Query text to search for
    pub query: String,
    /// Type of multi-match query
    #[serde(default)]
    pub r#type: MultiMatchType,
    /// Tie breaker for best_fields type (0.0 to 1.0, default: 0.0)
    #[serde(default)]
    pub tie_breaker: f32,
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
    /// Operator for boolean queries (AND/OR, default: OR)
    #[serde(default)]
    pub operator: MultiMatchOperator,
    /// Minimum should match for should clauses
    #[serde(default)]
    pub minimum_should_match: Option<String>,
    /// Analyzer to use for query parsing
    #[serde(default)]
    pub analyzer: Option<String>,
    /// Field-specific boosts (field^boost format)
    #[serde(default)]
    pub field_boosts: std::collections::HashMap<String, f32>,
}

/// Type of multi-match query
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum MultiMatchType {
    /// Best matching field score (default)
    #[default]
    BestFields,
    /// Sum of scores from all matching fields
    MostFields,
    /// Cross-fields matching (treats fields as one big field)
    CrossFields,
    /// Phrase matching across fields
    Phrase,
    /// Phrase prefix matching across fields
    PhrasePrefix,
}

/// Operator for multi-match queries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum MultiMatchOperator {
    /// OR operator (default)
    #[default]
    Or,
    /// AND operator
    And,
}

impl MultiMatchQuery {
    /// Create new multi-match query
    pub fn new(fields: Vec<String>, query: impl Into<String>) -> Self {
        Self {
            fields,
            query: query.into(),
            r#type: MultiMatchType::BestFields,
            tie_breaker: 0.0,
            boost: 1.0,
            operator: MultiMatchOperator::Or,
            minimum_should_match: None,
            analyzer: None,
            field_boosts: std::collections::HashMap::new(),
        }
    }

    /// Set the type of multi-match query
    pub fn r#type(mut self, r#type: MultiMatchType) -> Self {
        self.r#type = r#type;
        self
    }

    /// Set tie breaker for best_fields type
    pub fn tie_breaker(mut self, tie_breaker: f32) -> Self {
        self.tie_breaker = tie_breaker.clamp(0.0, 1.0);
        self
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }

    /// Set operator (AND/OR)
    pub fn operator(mut self, operator: MultiMatchOperator) -> Self {
        self.operator = operator;
        self
    }

    /// Set minimum should match
    pub fn minimum_should_match(mut self, msm: impl Into<String>) -> Self {
        self.minimum_should_match = Some(msm.into());
        self
    }

    /// Set analyzer
    pub fn analyzer(mut self, analyzer: impl Into<String>) -> Self {
        self.analyzer = Some(analyzer.into());
        self
    }

    /// Add field boost
    pub fn field_boost(mut self, field: impl Into<String>, boost: f32) -> Self {
        self.field_boosts.insert(field.into(), boost);
        self
    }

    /// Parse fields with boosts (e.g., "title^2.0,content^1.5")
    pub fn parse_fields(fields_str: &str) -> (Vec<String>, std::collections::HashMap<String, f32>) {
        let mut fields = Vec::new();
        let mut boosts = std::collections::HashMap::new();

        for field_part in fields_str.split(',') {
            let field_part = field_part.trim();
            if let Some((field, boost_str)) = field_part.split_once('^') {
                let field = field.trim().to_string();
                if let Ok(boost) = boost_str.trim().parse::<f32>() {
                    boosts.insert(field.clone(), boost);
                    fields.push(field);
                } else {
                    fields.push(field_part.to_string());
                }
            } else {
                fields.push(field_part.to_string());
            }
        }

        (fields, boosts)
    }
}

/// Constant score query for applying a fixed score to all matching documents
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConstantScoreQuery {
    /// Filter query (matches get constant score)
    pub filter: Box<Query>,
    /// Constant score value (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

impl ConstantScoreQuery {
    /// Create new constant score query with default boost of 1.0
    pub fn new(filter: Query) -> Self {
        Self {
            filter: Box::new(filter),
            boost: 1.0,
        }
    }

    /// Set the constant score/boost value
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }
}

/// Dis Max query for selecting the best matching query from multiple queries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DisMaxQuery {
    /// Queries to evaluate (best match wins)
    pub queries: Vec<Query>,
    /// Tie breaker for adding scores from other queries (0.0 to 1.0, default: 0.0)
    #[serde(default)]
    pub tie_breaker: f32,
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

impl DisMaxQuery {
    /// Create new dis max query with default tie breaker of 0.0
    pub fn new(queries: Vec<Query>) -> Self {
        Self {
            queries,
            tie_breaker: 0.0,
            boost: 1.0,
        }
    }

    /// Set the tie breaker value (clamped to 0.0-1.0)
    pub fn tie_breaker(mut self, tie_breaker: f32) -> Self {
        self.tie_breaker = tie_breaker.clamp(0.0, 1.0);
        self
    }

    /// Set the boost value
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }
}

/// Common terms query for handling low/high frequency terms separately
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CommonTermsQuery {
    /// Field to search
    pub field: String,
    /// Query text to search for
    pub query: String,
    /// Cutoff frequency threshold (0.0 to 1.0, default: 0.001)
    /// Terms appearing in more than this fraction of documents are considered high-frequency
    #[serde(default = "default_cutoff_frequency")]
    pub cutoff_frequency: f32,
    /// Operator for low-frequency terms (default: OR)
    #[serde(default)]
    pub low_freq_operator: CommonTermsOperator,
    /// Operator for high-frequency terms (default: OR)
    #[serde(default)]
    pub high_freq_operator: CommonTermsOperator,
    /// Minimum should match for low-frequency terms
    #[serde(default)]
    pub minimum_should_match: Option<String>,
    /// Analyzer to use for query parsing
    #[serde(default)]
    pub analyzer: Option<String>,
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

fn default_cutoff_frequency() -> f32 {
    0.001
}

/// Operator for common terms query
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum CommonTermsOperator {
    /// OR operator (default)
    #[default]
    Or,
    /// AND operator
    And,
}

impl CommonTermsQuery {
    /// Create new common terms query
    pub fn new(field: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            query: query.into(),
            cutoff_frequency: 0.001,
            low_freq_operator: CommonTermsOperator::Or,
            high_freq_operator: CommonTermsOperator::Or,
            minimum_should_match: None,
            analyzer: None,
            boost: 1.0,
        }
    }

    /// Set cutoff frequency (clamped to 0.0-1.0)
    pub fn cutoff_frequency(mut self, cutoff: f32) -> Self {
        self.cutoff_frequency = cutoff.clamp(0.0, 1.0);
        self
    }

    /// Set operator for low-frequency terms
    pub fn low_freq_operator(mut self, operator: CommonTermsOperator) -> Self {
        self.low_freq_operator = operator;
        self
    }

    /// Set operator for high-frequency terms
    pub fn high_freq_operator(mut self, operator: CommonTermsOperator) -> Self {
        self.high_freq_operator = operator;
        self
    }

    /// Set minimum should match
    pub fn minimum_should_match(mut self, msm: impl Into<String>) -> Self {
        self.minimum_should_match = Some(msm.into());
        self
    }

    /// Set analyzer
    pub fn analyzer(mut self, analyzer: impl Into<String>) -> Self {
        self.analyzer = Some(analyzer.into());
        self
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
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
        assert_eq!(query.boost, 1.0);
    }

    #[test]
    fn test_match_query_boost() {
        let query = MatchQuery::new("title", "search").boost(2.5);
        assert_eq!(query.boost, 2.5);
    }

    #[test]
    fn test_term_query() {
        let query = TermQuery::new("status", "active");
        assert_eq!(query.field, "status");
        assert_eq!(query.value, "active");
        assert_eq!(query.boost, 1.0);
    }

    #[test]
    fn test_term_query_boost() {
        let query = TermQuery::new("status", "active").boost(3.0);
        assert_eq!(query.boost, 3.0);
    }

    #[test]
    fn test_range_query() {
        let query = RangeQuery::new("age")
            .gte(serde_json::json!(18))
            .lte(serde_json::json!(65));

        assert_eq!(query.field, "age");
        assert!(query.gte.is_some());
        assert!(query.lte.is_some());
        assert_eq!(query.boost, 1.0);
    }

    #[test]
    fn test_range_query_boost() {
        let query = RangeQuery::new("age").boost(1.5);
        assert_eq!(query.boost, 1.5);
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
        assert_eq!(query.boost, 1.0);
    }

    #[test]
    fn test_fuzzy_query_boost() {
        let query = FuzzyQuery::new("name", "test").boost(2.0);
        assert_eq!(query.boost, 2.0);
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
        assert_eq!(query.boost, 1.0);
    }

    #[test]
    fn test_phrase_query_boost() {
        let query = PhraseQuery::new("content", "test").boost(1.5);
        assert_eq!(query.boost, 1.5);
    }

    #[test]
    fn test_wildcard_query() {
        let query = WildcardQuery::new("name", "test*");
        assert_eq!(query.field, "name");
        assert_eq!(query.pattern, "test*");
        assert_eq!(query.boost, 1.0);
    }

    #[test]
    fn test_wildcard_query_boost() {
        let query = WildcardQuery::new("name", "test*").boost(2.0);
        assert_eq!(query.boost, 2.0);
    }

    #[test]
    fn test_regex_query() {
        let query = RegexQuery::new("content", "[A-Z][a-z]+").case_sensitive(true);

        assert_eq!(query.field, "content");
        assert_eq!(query.pattern, "[A-Z][a-z]+");
        assert!(query.case_sensitive);
        assert_eq!(query.boost, 1.0);
    }

    #[test]
    fn test_regex_query_boost() {
        let query = RegexQuery::new("content", "test").boost(1.8);
        assert_eq!(query.boost, 1.8);
    }

    #[test]
    fn test_phrase_query_with_slop() {
        let query = PhraseQuery::new("content", "quick fox").slop(2);
        assert_eq!(query.slop, 2);
    }

    #[test]
    fn test_more_like_this_query() {
        let query = MoreLikeThisQuery::new(
            vec!["title".to_string(), "content".to_string()],
            "sample text",
        );

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
        assert!(matches!(
            function_score.score_mode,
            FunctionScoreMode::Multiply
        ));
        assert!(matches!(
            function_score.boost_mode,
            FunctionBoostMode::Multiply
        ));
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

    #[test]
    fn test_range_query_bounds() {
        let query = RangeQuery::new("age")
            .gte(serde_json::json!(18))
            .lt(serde_json::json!(65));

        assert_eq!(query.field, "age");
        assert!(query.gte.is_some());
        assert!(query.lt.is_some());
        assert!(query.lte.is_none());
        assert!(query.gt.is_none());
    }

    #[test]
    fn test_range_query_all_bounds() {
        let query = RangeQuery::new("price")
            .gt(serde_json::json!(10))
            .lt(serde_json::json!(100))
            .gte(serde_json::json!(20))
            .lte(serde_json::json!(80));

        assert_eq!(query.field, "price");
        assert!(query.gt.is_some());
        assert!(query.lt.is_some());
        assert!(query.gte.is_some());
        assert!(query.lte.is_some());
    }

    #[test]
    fn test_bool_query_all_clauses() {
        let bool_query = BoolQuery::new()
            .must(Query::Term(TermQuery::new("status", "active")))
            .should(Query::Match(MatchQuery::new("title", "test")))
            .must_not(Query::Term(TermQuery::new("deleted", "true")))
            .filter(Query::Range(
                RangeQuery::new("age").gte(serde_json::json!(18)),
            ));

        assert_eq!(bool_query.must.len(), 1);
        assert_eq!(bool_query.should.len(), 1);
        assert_eq!(bool_query.must_not.len(), 1);
        assert_eq!(bool_query.filter.len(), 1);
    }

    #[test]
    fn test_bool_query_multiple_clauses() {
        let bool_query = BoolQuery::new()
            .must(Query::Term(TermQuery::new("status", "active")))
            .must(Query::Match(MatchQuery::new("title", "test")));

        assert_eq!(bool_query.must.len(), 2);
    }

    #[test]
    fn test_fuzzy_query_transpositions() {
        let query = FuzzyQuery::new("name", "test").transpositions(false);
        assert!(!query.transpositions);
    }

    #[test]
    fn test_fuzzy_query_prefix_length() {
        let query = FuzzyQuery::new("name", "test").prefix_length(5);
        assert_eq!(query.prefix_length, 5);
    }

    #[test]
    fn test_phrase_query_default_slop() {
        let query = PhraseQuery::new("content", "quick brown fox");
        assert_eq!(query.slop, 0);
    }

    #[test]
    fn test_regex_query_case_insensitive() {
        let query = RegexQuery::new("content", "[A-Z]+").case_sensitive(false);
        assert!(!query.case_sensitive);
    }

    #[test]
    fn test_more_like_this_query_with_options() {
        let mut query = MoreLikeThisQuery::new(vec!["title".to_string()], "sample");
        query.min_term_freq = 2;
        query.max_query_terms = 50;
        query.min_doc_freq = 1;

        assert_eq!(query.min_term_freq, 2);
        assert_eq!(query.max_query_terms, 50);
        assert_eq!(query.min_doc_freq, 1);
    }

    #[test]
    fn test_nested_query_score_modes() {
        let inner_query = Query::Term(TermQuery::new("nested.field", "value"));

        let mut avg_query = NestedQuery::new("nested", inner_query.clone());
        avg_query.score_mode = NestedScoreMode::Avg;
        assert!(matches!(avg_query.score_mode, NestedScoreMode::Avg));

        let mut max_query = NestedQuery::new("nested", inner_query.clone());
        max_query.score_mode = NestedScoreMode::Max;
        assert!(matches!(max_query.score_mode, NestedScoreMode::Max));

        let mut sum_query = NestedQuery::new("nested", inner_query.clone());
        sum_query.score_mode = NestedScoreMode::Sum;
        assert!(matches!(sum_query.score_mode, NestedScoreMode::Sum));

        let mut none_query = NestedQuery::new("nested", inner_query);
        none_query.score_mode = NestedScoreMode::None;
        assert!(matches!(none_query.score_mode, NestedScoreMode::None));
    }

    #[test]
    fn test_function_score_query_modes() {
        let base_query = Query::Match(MatchQuery::new("title", "test"));

        let mut multiply_query = FunctionScoreQuery::new(base_query.clone());
        multiply_query.score_mode = FunctionScoreMode::Multiply;
        assert!(matches!(
            multiply_query.score_mode,
            FunctionScoreMode::Multiply
        ));

        let mut sum_query = FunctionScoreQuery::new(base_query.clone());
        sum_query.score_mode = FunctionScoreMode::Sum;
        assert!(matches!(sum_query.score_mode, FunctionScoreMode::Sum));

        let mut avg_query = FunctionScoreQuery::new(base_query.clone());
        avg_query.score_mode = FunctionScoreMode::Avg;
        assert!(matches!(avg_query.score_mode, FunctionScoreMode::Avg));

        let mut max_query = FunctionScoreQuery::new(base_query.clone());
        max_query.score_mode = FunctionScoreMode::Max;
        assert!(matches!(max_query.score_mode, FunctionScoreMode::Max));

        let mut min_query = FunctionScoreQuery::new(base_query);
        min_query.score_mode = FunctionScoreMode::Min;
        assert!(matches!(min_query.score_mode, FunctionScoreMode::Min));
    }

    #[test]
    fn test_function_score_query_boost_modes() {
        let base_query = Query::Match(MatchQuery::new("title", "test"));

        let mut multiply_boost = FunctionScoreQuery::new(base_query.clone());
        multiply_boost.boost_mode = FunctionBoostMode::Multiply;
        assert!(matches!(
            multiply_boost.boost_mode,
            FunctionBoostMode::Multiply
        ));

        let mut replace_boost = FunctionScoreQuery::new(base_query.clone());
        replace_boost.boost_mode = FunctionBoostMode::Replace;
        assert!(matches!(
            replace_boost.boost_mode,
            FunctionBoostMode::Replace
        ));

        let mut sum_boost = FunctionScoreQuery::new(base_query.clone());
        sum_boost.boost_mode = FunctionBoostMode::Sum;
        assert!(matches!(sum_boost.boost_mode, FunctionBoostMode::Sum));

        let mut avg_boost = FunctionScoreQuery::new(base_query.clone());
        avg_boost.boost_mode = FunctionBoostMode::Avg;
        assert!(matches!(avg_boost.boost_mode, FunctionBoostMode::Avg));

        let mut max_boost = FunctionScoreQuery::new(base_query.clone());
        max_boost.boost_mode = FunctionBoostMode::Max;
        assert!(matches!(max_boost.boost_mode, FunctionBoostMode::Max));

        let mut min_boost = FunctionScoreQuery::new(base_query);
        min_boost.boost_mode = FunctionBoostMode::Min;
        assert!(matches!(min_boost.boost_mode, FunctionBoostMode::Min));
    }

    #[test]
    fn test_function_score_query_with_limits() {
        let base_query = Query::Match(MatchQuery::new("title", "test"));
        let mut query = FunctionScoreQuery::new(base_query);
        query.max_boost = 2.0;
        query.min_score = Some(0.5);

        assert_eq!(query.max_boost, 2.0);
        assert_eq!(query.min_score, Some(0.5));
    }

    #[test]
    fn test_script_query_with_params() {
        let mut script_query = ScriptQuery::new("doc['field'].value > param");
        script_query
            .params
            .insert("param".to_string(), serde_json::json!(10));

        assert_eq!(script_query.source, "doc['field'].value > param");
        assert_eq!(script_query.params.len(), 1);
        assert_eq!(
            script_query.params.get("param"),
            Some(&serde_json::json!(10))
        );
    }

    #[test]
    fn test_query_serialization() {
        let query = Query::Match(MatchQuery::new("title", "test"));
        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("match")); // snake_case serialization
        assert!(json.contains("title"));
        assert!(json.contains("test"));

        let deserialized: Query = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Query::Match(_)));
    }

    #[test]
    fn test_match_query_serialization() {
        let query = MatchQuery::new("title", "test");
        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("title"));
        assert!(json.contains("test"));

        let deserialized: MatchQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "title");
        assert_eq!(deserialized.query, "test");
    }

    #[test]
    fn test_term_query_serialization() {
        let query = TermQuery::new("status", "active");
        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("status"));
        assert!(json.contains("active"));

        let deserialized: TermQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "status");
        assert_eq!(deserialized.value, "active");
    }

    #[test]
    fn test_range_query_serialization() {
        let query = RangeQuery::new("age")
            .gte(serde_json::json!(18))
            .lte(serde_json::json!(65));

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("age"));
        assert!(json.contains("gte"));
        assert!(json.contains("lte"));

        let deserialized: RangeQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "age");
        assert!(deserialized.gte.is_some());
        assert!(deserialized.lte.is_some());
    }

    #[test]
    fn test_bool_query_serialization() {
        let bool_query = BoolQuery::new().must(Query::Term(TermQuery::new("status", "active")));

        let json = serde_json::to_string(&bool_query).unwrap();
        assert!(json.contains("must"));

        let deserialized: BoolQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.must.len(), 1);
    }

    #[test]
    fn test_fuzzy_query_serialization() {
        let query = FuzzyQuery::new("name", "test")
            .fuzziness(1)
            .prefix_length(2)
            .transpositions(false);

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("name"));
        assert!(json.contains("test"));

        let deserialized: FuzzyQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "name");
        assert_eq!(deserialized.value, "test");
        assert_eq!(deserialized.fuzziness, 1);
        assert_eq!(deserialized.prefix_length, 2);
        assert!(!deserialized.transpositions);
    }

    #[test]
    fn test_phrase_query_serialization() {
        let query = PhraseQuery::new("content", "quick brown fox").slop(2);

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("content"));
        assert!(json.contains("quick brown fox"));

        let deserialized: PhraseQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "content");
        assert_eq!(deserialized.phrase, "quick brown fox");
        assert_eq!(deserialized.slop, 2);
    }

    #[test]
    fn test_wildcard_query_serialization() {
        let query = WildcardQuery::new("name", "test*");

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("name"));
        assert!(json.contains("test*"));

        let deserialized: WildcardQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "name");
        assert_eq!(deserialized.pattern, "test*");
    }

    #[test]
    fn test_regex_query_serialization() {
        let query = RegexQuery::new("content", "[A-Z]+").case_sensitive(true);

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("content"));
        assert!(json.contains("[A-Z]+"));

        let deserialized: RegexQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "content");
        assert_eq!(deserialized.pattern, "[A-Z]+");
        assert!(deserialized.case_sensitive);
    }

    #[test]
    fn test_multi_match_query() {
        let query = MultiMatchQuery::new(
            vec!["title".to_string(), "content".to_string()],
            "search terms",
        );

        assert_eq!(query.fields.len(), 2);
        assert_eq!(query.query, "search terms");
        assert!(matches!(query.r#type, MultiMatchType::BestFields));
        assert_eq!(query.tie_breaker, 0.0);
        assert_eq!(query.boost, 1.0);
    }

    #[test]
    fn test_multi_match_query_with_type() {
        let query = MultiMatchQuery::new(vec!["title".to_string()], "test")
            .r#type(MultiMatchType::MostFields)
            .tie_breaker(0.3)
            .boost(2.0);

        assert!(matches!(query.r#type, MultiMatchType::MostFields));
        assert_eq!(query.tie_breaker, 0.3);
        assert_eq!(query.boost, 2.0);
    }

    #[test]
    fn test_multi_match_query_operator() {
        let query = MultiMatchQuery::new(vec!["title".to_string()], "test")
            .operator(MultiMatchOperator::And);

        assert!(matches!(query.operator, MultiMatchOperator::And));
    }

    #[test]
    fn test_multi_match_query_field_boost() {
        let query = MultiMatchQuery::new(vec!["title".to_string(), "content".to_string()], "test")
            .field_boost("title", 2.0)
            .field_boost("content", 1.5);

        assert_eq!(query.field_boosts.get("title"), Some(&2.0));
        assert_eq!(query.field_boosts.get("content"), Some(&1.5));
    }

    #[test]
    fn test_multi_match_query_parse_fields() {
        let (fields, boosts) = MultiMatchQuery::parse_fields("title^2.0,content^1.5,description");

        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], "title");
        assert_eq!(fields[1], "content");
        assert_eq!(fields[2], "description");
        assert_eq!(boosts.get("title"), Some(&2.0));
        assert_eq!(boosts.get("content"), Some(&1.5));
        assert_eq!(boosts.get("description"), None);
    }

    #[test]
    fn test_multi_match_query_tie_breaker_clamping() {
        let query1 = MultiMatchQuery::new(vec!["title".to_string()], "test").tie_breaker(-0.5);
        assert_eq!(query1.tie_breaker, 0.0);

        let query2 = MultiMatchQuery::new(vec!["title".to_string()], "test").tie_breaker(1.5);
        assert_eq!(query2.tie_breaker, 1.0);
    }

    #[test]
    fn test_multi_match_query_serialization() {
        let query =
            MultiMatchQuery::new(vec!["title".to_string(), "content".to_string()], "search")
                .r#type(MultiMatchType::CrossFields)
                .tie_breaker(0.3)
                .boost(2.0);

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("title"));
        assert!(json.contains("content"));
        assert!(json.contains("search"));

        let deserialized: MultiMatchQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.fields.len(), 2);
        assert_eq!(deserialized.query, "search");
        assert!(matches!(deserialized.r#type, MultiMatchType::CrossFields));
    }

    #[test]
    fn test_multi_match_query_all_types() {
        let base_fields = vec!["title".to_string()];
        let base_query = "test";

        let best_fields = MultiMatchQuery::new(base_fields.clone(), base_query)
            .r#type(MultiMatchType::BestFields);
        assert!(matches!(best_fields.r#type, MultiMatchType::BestFields));

        let most_fields = MultiMatchQuery::new(base_fields.clone(), base_query)
            .r#type(MultiMatchType::MostFields);
        assert!(matches!(most_fields.r#type, MultiMatchType::MostFields));

        let cross_fields = MultiMatchQuery::new(base_fields.clone(), base_query)
            .r#type(MultiMatchType::CrossFields);
        assert!(matches!(cross_fields.r#type, MultiMatchType::CrossFields));

        let phrase =
            MultiMatchQuery::new(base_fields.clone(), base_query).r#type(MultiMatchType::Phrase);
        assert!(matches!(phrase.r#type, MultiMatchType::Phrase));

        let phrase_prefix =
            MultiMatchQuery::new(base_fields, base_query).r#type(MultiMatchType::PhrasePrefix);
        assert!(matches!(phrase_prefix.r#type, MultiMatchType::PhrasePrefix));
    }

    #[test]
    fn test_constant_score_query() {
        let filter_query = Query::Term(TermQuery::new("status", "active"));
        let constant_score = ConstantScoreQuery::new(filter_query);

        assert!(matches!(*constant_score.filter, Query::Term(_)));
        assert_eq!(constant_score.boost, 1.0);
    }

    #[test]
    fn test_constant_score_query_with_boost() {
        let filter_query = Query::Match(MatchQuery::new("title", "test"));
        let constant_score = ConstantScoreQuery::new(filter_query).boost(2.5);

        assert!(matches!(*constant_score.filter, Query::Match(_)));
        assert_eq!(constant_score.boost, 2.5);
    }

    #[test]
    fn test_constant_score_query_serialization() {
        let filter_query = Query::Range(RangeQuery::new("age").gte(serde_json::json!(18)));
        let constant_score = ConstantScoreQuery::new(filter_query).boost(1.5);

        let json = serde_json::to_string(&constant_score).unwrap();
        assert!(json.contains("filter"));
        assert!(json.contains("boost"));

        let deserialized: ConstantScoreQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.boost, 1.5);
        assert!(matches!(*deserialized.filter, Query::Range(_)));
    }

    #[test]
    fn test_dis_max_query() {
        let queries = vec![
            Query::Match(MatchQuery::new("title", "test")),
            Query::Match(MatchQuery::new("content", "test")),
        ];
        let dis_max = DisMaxQuery::new(queries);

        assert_eq!(dis_max.queries.len(), 2);
        assert_eq!(dis_max.tie_breaker, 0.0);
        assert_eq!(dis_max.boost, 1.0);
    }

    #[test]
    fn test_dis_max_query_with_tie_breaker() {
        let queries = vec![Query::Term(TermQuery::new("status", "active"))];
        let dis_max = DisMaxQuery::new(queries).tie_breaker(0.3);

        assert_eq!(dis_max.tie_breaker, 0.3);
    }

    #[test]
    fn test_dis_max_query_tie_breaker_clamping() {
        let queries = vec![Query::Match(MatchQuery::new("title", "test"))];
        let dis_max_high = DisMaxQuery::new(queries.clone()).tie_breaker(2.0);
        let dis_max_low = DisMaxQuery::new(queries).tie_breaker(-1.0);

        assert_eq!(dis_max_high.tie_breaker, 1.0);
        assert_eq!(dis_max_low.tie_breaker, 0.0);
    }

    #[test]
    fn test_dis_max_query_with_boost() {
        let queries = vec![Query::Match(MatchQuery::new("title", "test"))];
        let dis_max = DisMaxQuery::new(queries).boost(2.5);

        assert_eq!(dis_max.boost, 2.5);
    }

    #[test]
    fn test_dis_max_query_serialization() {
        let queries = vec![
            Query::Match(MatchQuery::new("title", "test")),
            Query::Term(TermQuery::new("status", "active")),
        ];
        let dis_max = DisMaxQuery::new(queries).tie_breaker(0.5).boost(1.5);

        let json = serde_json::to_string(&dis_max).unwrap();
        assert!(json.contains("queries"));
        assert!(json.contains("tie_breaker"));
        assert!(json.contains("boost"));

        let deserialized: DisMaxQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tie_breaker, 0.5);
        assert_eq!(deserialized.boost, 1.5);
        assert_eq!(deserialized.queries.len(), 2);
    }

    #[test]
    fn test_common_terms_query() {
        let query = CommonTermsQuery::new("body", "bonsai cool");
        assert_eq!(query.field, "body");
        assert_eq!(query.query, "bonsai cool");
        assert_eq!(query.cutoff_frequency, 0.001);
        assert!(matches!(query.low_freq_operator, CommonTermsOperator::Or));
        assert!(matches!(query.high_freq_operator, CommonTermsOperator::Or));
        assert_eq!(query.boost, 1.0);
    }

    #[test]
    fn test_common_terms_query_with_cutoff_frequency() {
        let query = CommonTermsQuery::new("body", "test query").cutoff_frequency(0.01);
        assert_eq!(query.cutoff_frequency, 0.01);
    }

    #[test]
    fn test_common_terms_query_cutoff_frequency_clamping() {
        let query_high = CommonTermsQuery::new("body", "test").cutoff_frequency(2.0);
        let query_low = CommonTermsQuery::new("body", "test").cutoff_frequency(-1.0);

        assert_eq!(query_high.cutoff_frequency, 1.0);
        assert_eq!(query_low.cutoff_frequency, 0.0);
    }

    #[test]
    fn test_common_terms_query_operators() {
        let query = CommonTermsQuery::new("body", "test")
            .low_freq_operator(CommonTermsOperator::And)
            .high_freq_operator(CommonTermsOperator::And);

        assert!(matches!(query.low_freq_operator, CommonTermsOperator::And));
        assert!(matches!(query.high_freq_operator, CommonTermsOperator::And));
    }

    #[test]
    fn test_common_terms_query_with_minimum_should_match() {
        let query = CommonTermsQuery::new("body", "test query").minimum_should_match("2");

        assert_eq!(query.minimum_should_match, Some("2".to_string()));
    }

    #[test]
    fn test_common_terms_query_with_analyzer() {
        let query = CommonTermsQuery::new("body", "test query").analyzer("standard");

        assert_eq!(query.analyzer, Some("standard".to_string()));
    }

    #[test]
    fn test_common_terms_query_with_boost() {
        let query = CommonTermsQuery::new("body", "test").boost(2.5);

        assert_eq!(query.boost, 2.5);
    }

    #[test]
    fn test_common_terms_query_serialization() {
        let query = CommonTermsQuery::new("body", "bonsai cool")
            .cutoff_frequency(0.001)
            .low_freq_operator(CommonTermsOperator::And)
            .high_freq_operator(CommonTermsOperator::Or)
            .minimum_should_match("2")
            .boost(1.5);

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("field"));
        assert!(json.contains("body"));
        assert!(json.contains("query"));
        assert!(json.contains("cutoff_frequency"));
        assert!(json.contains("low_freq_operator"));
        assert!(json.contains("high_freq_operator"));
        assert!(json.contains("boost"));

        let deserialized: CommonTermsQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "body");
        assert_eq!(deserialized.query, "bonsai cool");
        assert_eq!(deserialized.cutoff_frequency, 0.001);
        assert!(matches!(
            deserialized.low_freq_operator,
            CommonTermsOperator::And
        ));
        assert!(matches!(
            deserialized.high_freq_operator,
            CommonTermsOperator::Or
        ));
        assert_eq!(deserialized.boost, 1.5);
    }
}
