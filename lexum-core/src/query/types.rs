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
    /// Wrapper query (accepts serialized queries)
    Wrapper(WrapperQuery),
    /// Pinned query (promotes specific documents)
    Pinned(PinnedQuery),
    /// Has child query (find parent documents with matching children)
    HasChild(HasChildQuery),
    /// Has parent query (find child documents with matching parents)
    HasParent(HasParentQuery),
    /// Geo bounding box query (search within bounding box)
    GeoBoundingBox(GeoBoundingBoxQuery),
    /// Geo polygon query (search within polygon)
    GeoPolygon(GeoPolygonQuery),
    /// Geo shape query (search with geographic shapes)
    GeoShape(GeoShapeQuery),
    /// Percolate query (reverse search - match stored queries against document)
    Percolate(PercolateQuery),
    /// Simple query string query (simplified syntax for end users)
    SimpleQueryString(SimpleQueryStringQuery),
    /// Query string query (advanced syntax with field groups, proximity, boosting, etc.)
    QueryString(QueryStringQuery),
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

    /// Set minimum term frequency
    pub fn min_term_freq(mut self, freq: u32) -> Self {
        self.min_term_freq = freq;
        self
    }

    /// Set maximum query terms
    pub fn max_query_terms(mut self, terms: u32) -> Self {
        self.max_query_terms = terms;
        self
    }

    /// Set minimum document frequency
    pub fn min_doc_freq(mut self, freq: u32) -> Self {
        self.min_doc_freq = freq;
        self
    }

    /// Set maximum document frequency (0 = unlimited)
    pub fn max_doc_freq(mut self, freq: u32) -> Self {
        self.max_doc_freq = freq;
        self
    }

    /// Set minimum word length
    pub fn min_word_length(mut self, length: u32) -> Self {
        self.min_word_length = length;
        self
    }

    /// Set maximum word length (0 = unlimited)
    pub fn max_word_length(mut self, length: u32) -> Self {
        self.max_word_length = length;
        self
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

    /// Set score mode
    pub fn score_mode(mut self, mode: NestedScoreMode) -> Self {
        self.score_mode = mode;
        self
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
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
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
            boost: 1.0,
        }
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }
}

/// Geo bounding box query for searching within a bounding box
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoBoundingBoxQuery {
    /// Field containing geo point
    pub field: String,
    /// Top-left corner of bounding box
    pub top_left: GeoPoint,
    /// Bottom-right corner of bounding box
    pub bottom_right: GeoPoint,
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

impl GeoBoundingBoxQuery {
    /// Create new geo bounding box query
    pub fn new(field: impl Into<String>, top_left: GeoPoint, bottom_right: GeoPoint) -> Self {
        Self {
            field: field.into(),
            top_left,
            bottom_right,
            boost: 1.0,
        }
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }
}

/// Geo polygon query for searching within a polygon
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoPolygonQuery {
    /// Field containing geo point
    pub field: String,
    /// Polygon points (must form a closed polygon)
    pub points: Vec<GeoPoint>,
    /// Holes in the polygon (optional)
    #[serde(default)]
    pub holes: Vec<Vec<GeoPoint>>,
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

impl GeoPolygonQuery {
    /// Create new geo polygon query
    pub fn new(field: impl Into<String>, points: Vec<GeoPoint>) -> Self {
        Self {
            field: field.into(),
            points,
            holes: Vec::new(),
            boost: 1.0,
        }
    }

    /// Add a hole to the polygon
    pub fn add_hole(mut self, hole: Vec<GeoPoint>) -> Self {
        self.holes.push(hole);
        self
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }
}

/// Geo shape query for searching with geographic shapes
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoShapeQuery {
    /// Field containing geo shape
    pub field: String,
    /// Shape to match against
    pub shape: GeoShape,
    /// Spatial relationship type
    #[serde(default)]
    pub relation: GeoShapeRelation,
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

/// Geographic shape for geo shape queries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum GeoShape {
    /// Point shape
    Point {
        /// Point coordinates
        coordinates: GeoPoint,
    },
    /// LineString shape
    LineString {
        /// LineString coordinates
        coordinates: Vec<GeoPoint>,
    },
    /// Polygon shape
    Polygon {
        /// Polygon coordinates (outer ring and holes)
        coordinates: Vec<Vec<GeoPoint>>,
    },
    /// Circle shape
    Circle {
        /// Center point of the circle
        coordinates: GeoPoint,
        /// Radius of the circle (e.g., \"10km\")
        radius: String,
    },
    /// Envelope (bounding box) shape
    Envelope {
        /// Coordinates tuple (top_left, bottom_right)
        coordinates: (GeoPoint, GeoPoint),
    },
}

/// Spatial relationship for geo shape queries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum GeoShapeRelation {
    /// Shapes intersect
    #[default]
    Intersects,
    /// Query shape contains indexed shape
    Contains,
    /// Query shape is within indexed shape
    Within,
    /// Shapes are disjoint
    Disjoint,
}

impl GeoShapeQuery {
    /// Create new geo shape query
    pub fn new(field: impl Into<String>, shape: GeoShape) -> Self {
        Self {
            field: field.into(),
            shape,
            relation: GeoShapeRelation::Intersects,
            boost: 1.0,
        }
    }

    /// Set spatial relationship
    pub fn relation(mut self, relation: GeoShapeRelation) -> Self {
        self.relation = relation;
        self
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }
}

/// Percolate query for reverse search (match stored queries against a document)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PercolateQuery {
    /// Field containing the document to percolate
    pub field: String,
    /// Document to match against stored queries (as JSON object)
    pub document: serde_json::Value,
    /// Index name where percolator queries are stored (optional)
    #[serde(default)]
    pub index: Option<String>,
    /// Document type for percolator queries (optional, deprecated in ES but kept for compatibility)
    #[serde(default)]
    pub document_type: Option<String>,
    /// Preferred document source (optional)
    #[serde(default)]
    pub preferred_sources: Option<Vec<String>>,
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

impl PercolateQuery {
    /// Create new percolate query
    pub fn new(field: impl Into<String>, document: serde_json::Value) -> Self {
        Self {
            field: field.into(),
            document,
            index: None,
            document_type: None,
            preferred_sources: None,
            boost: 1.0,
        }
    }

    /// Set the index name where percolator queries are stored
    pub fn index(mut self, index: impl Into<String>) -> Self {
        self.index = Some(index.into());
        self
    }

    /// Set the document type for percolator queries
    pub fn document_type(mut self, doc_type: impl Into<String>) -> Self {
        self.document_type = Some(doc_type.into());
        self
    }

    /// Set preferred document sources
    pub fn preferred_sources(mut self, sources: Vec<String>) -> Self {
        self.preferred_sources = Some(sources);
        self
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }
}

/// Simple query string query for simplified syntax
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SimpleQueryStringQuery {
    /// Query string to parse
    pub query: String,
    /// Fields to search (defaults to all fields if empty)
    #[serde(default)]
    pub fields: Vec<String>,
    /// Default operator (AND/OR)
    #[serde(default)]
    pub default_operator: SimpleQueryStringOperator,
    /// Analyze wildcard (whether to analyze wildcard terms)
    #[serde(default)]
    pub analyze_wildcard: bool,
    /// Auto generate synonyms phrase query
    #[serde(default)]
    pub auto_generate_synonyms_phrase_query: bool,
    /// Flags to control which features are enabled
    #[serde(default = "default_simple_query_string_flags")]
    pub flags: SimpleQueryStringFlags,
    /// Fuzzy max expansions
    #[serde(default = "default_fuzzy_max_expansions")]
    pub fuzzy_max_expansions: u32,
    /// Fuzzy prefix length
    #[serde(default = "default_fuzzy_prefix_length")]
    pub fuzzy_prefix_length: u32,
    /// Fuzzy transpositions
    #[serde(default)]
    pub fuzzy_transpositions: bool,
    /// Lenient (ignore format-based errors)
    #[serde(default)]
    pub lenient: bool,
    /// Minimum should match
    #[serde(default)]
    pub minimum_should_match: Option<String>,
    /// Quote field suffix
    #[serde(default)]
    pub quote_field_suffix: Option<String>,
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

/// Default operator for simple query string
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum SimpleQueryStringOperator {
    /// AND operator (all terms must match)
    #[default]
    And,
    /// OR operator (any term can match)
    Or,
}

/// Flags for simple query string features
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct SimpleQueryStringFlags {
    /// Enable AND operator
    #[serde(default = "default_true")]
    pub and: bool,
    /// Enable OR operator
    #[serde(default = "default_true")]
    pub or: bool,
    /// Enable NOT operator
    #[serde(default = "default_true")]
    pub not: bool,
    /// Enable prefix matching
    #[serde(default = "default_true")]
    pub prefix: bool,
    /// Enable phrase matching
    #[serde(default = "default_true")]
    pub phrase: bool,
    /// Enable precedence
    #[serde(default = "default_true")]
    pub precedence: bool,
    /// Enable escape
    #[serde(default = "default_true")]
    pub escape: bool,
    /// Enable whitespace
    #[serde(default = "default_true")]
    pub whitespace: bool,
    /// Enable fuzzy matching
    #[serde(default = "default_true")]
    pub fuzzy: bool,
    /// Enable near matching
    #[serde(default = "default_true")]
    pub near: bool,
    /// Enable slop
    #[serde(default = "default_true")]
    pub slop: bool,
}

fn default_simple_query_string_flags() -> SimpleQueryStringFlags {
    SimpleQueryStringFlags::default()
}

fn default_fuzzy_max_expansions() -> u32 {
    50
}

fn default_fuzzy_prefix_length() -> u32 {
    0
}

// Note: default_true() is already defined earlier in the file

impl SimpleQueryStringQuery {
    /// Create new simple query string query
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            fields: Vec::new(),
            default_operator: SimpleQueryStringOperator::Or,
            analyze_wildcard: false,
            auto_generate_synonyms_phrase_query: true,
            flags: SimpleQueryStringFlags::default(),
            fuzzy_max_expansions: 50,
            fuzzy_prefix_length: 0,
            fuzzy_transpositions: true,
            lenient: false,
            minimum_should_match: None,
            quote_field_suffix: None,
            boost: 1.0,
        }
    }

    /// Set fields to search
    pub fn fields(mut self, fields: Vec<String>) -> Self {
        self.fields = fields;
        self
    }

    /// Set default operator
    pub fn default_operator(mut self, operator: SimpleQueryStringOperator) -> Self {
        self.default_operator = operator;
        self
    }

    /// Set analyze wildcard
    pub fn analyze_wildcard(mut self, analyze: bool) -> Self {
        self.analyze_wildcard = analyze;
        self
    }

    /// Set auto generate synonyms phrase query
    pub fn auto_generate_synonyms_phrase_query(mut self, auto: bool) -> Self {
        self.auto_generate_synonyms_phrase_query = auto;
        self
    }

    /// Set flags
    pub fn flags(mut self, flags: SimpleQueryStringFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set fuzzy max expansions
    pub fn fuzzy_max_expansions(mut self, max: u32) -> Self {
        self.fuzzy_max_expansions = max;
        self
    }

    /// Set fuzzy prefix length
    pub fn fuzzy_prefix_length(mut self, length: u32) -> Self {
        self.fuzzy_prefix_length = length;
        self
    }

    /// Set fuzzy transpositions
    pub fn fuzzy_transpositions(mut self, transpositions: bool) -> Self {
        self.fuzzy_transpositions = transpositions;
        self
    }

    /// Set lenient mode
    pub fn lenient(mut self, lenient: bool) -> Self {
        self.lenient = lenient;
        self
    }

    /// Set minimum should match
    pub fn minimum_should_match(mut self, msm: impl Into<String>) -> Self {
        self.minimum_should_match = Some(msm.into());
        self
    }

    /// Set quote field suffix
    pub fn quote_field_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.quote_field_suffix = Some(suffix.into());
        self
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }
}

/// Query string query for advanced syntax
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QueryStringQuery {
    /// Query string to parse (supports advanced syntax)
    pub query: String,
    /// Default field to search (optional)
    #[serde(default)]
    pub default_field: Option<String>,
    /// Fields to search (optional, defaults to all if not specified)
    #[serde(default)]
    pub fields: Vec<String>,
    /// Default operator (AND/OR)
    #[serde(default)]
    pub default_operator: QueryStringOperator,
    /// Analyze wildcard (whether to analyze wildcard terms)
    #[serde(default)]
    pub analyze_wildcard: bool,
    /// Allow leading wildcard
    #[serde(default)]
    pub allow_leading_wildcard: bool,
    /// Auto generate phrase queries
    #[serde(default = "default_true")]
    pub auto_generate_phrase_queries: bool,
    /// Enable position increments
    #[serde(default = "default_true")]
    pub enable_position_increments: bool,
    /// Escape characters
    #[serde(default)]
    pub escape: bool,
    /// Fuzziness (e.g., "AUTO", "0", "1", "2")
    #[serde(default)]
    pub fuzziness: Option<String>,
    /// Fuzzy max expansions
    #[serde(default = "default_fuzzy_max_expansions")]
    pub fuzzy_max_expansions: u32,
    /// Fuzzy prefix length
    #[serde(default = "default_fuzzy_prefix_length")]
    pub fuzzy_prefix_length: u32,
    /// Fuzzy transpositions
    #[serde(default)]
    pub fuzzy_transpositions: bool,
    /// Lenient (ignore format-based errors)
    #[serde(default)]
    pub lenient: bool,
    /// Maximum determinized states (for regex)
    #[serde(default = "default_max_determinized_states")]
    pub max_determinized_states: u32,
    /// Minimum should match
    #[serde(default)]
    pub minimum_should_match: Option<String>,
    /// Phrase slop
    #[serde(default)]
    pub phrase_slop: Option<u32>,
    /// Quote analyzer
    #[serde(default)]
    pub quote_analyzer: Option<String>,
    /// Quote field suffix
    #[serde(default)]
    pub quote_field_suffix: Option<String>,
    /// Tie breaker for multi-field queries
    #[serde(default)]
    pub tie_breaker: Option<f32>,
    /// Time zone (for date ranges)
    #[serde(default)]
    pub time_zone: Option<String>,
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

/// Default operator for query string
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum QueryStringOperator {
    /// AND operator (all terms must match)
    #[default]
    And,
    /// OR operator (any term can match)
    Or,
}

fn default_max_determinized_states() -> u32 {
    10000
}

impl QueryStringQuery {
    /// Create new query string query
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            default_field: None,
            fields: Vec::new(),
            default_operator: QueryStringOperator::Or,
            analyze_wildcard: false,
            allow_leading_wildcard: true,
            auto_generate_phrase_queries: true,
            enable_position_increments: true,
            escape: false,
            fuzziness: None,
            fuzzy_max_expansions: 50,
            fuzzy_prefix_length: 0,
            fuzzy_transpositions: true,
            lenient: false,
            max_determinized_states: 10000,
            minimum_should_match: None,
            phrase_slop: None,
            quote_analyzer: None,
            quote_field_suffix: None,
            tie_breaker: None,
            time_zone: None,
            boost: 1.0,
        }
    }

    /// Set default field
    pub fn default_field(mut self, field: impl Into<String>) -> Self {
        self.default_field = Some(field.into());
        self
    }

    /// Set fields to search
    pub fn fields(mut self, fields: Vec<String>) -> Self {
        self.fields = fields;
        self
    }

    /// Set default operator
    pub fn default_operator(mut self, operator: QueryStringOperator) -> Self {
        self.default_operator = operator;
        self
    }

    /// Set analyze wildcard
    pub fn analyze_wildcard(mut self, analyze: bool) -> Self {
        self.analyze_wildcard = analyze;
        self
    }

    /// Set allow leading wildcard
    pub fn allow_leading_wildcard(mut self, allow: bool) -> Self {
        self.allow_leading_wildcard = allow;
        self
    }

    /// Set auto generate phrase queries
    pub fn auto_generate_phrase_queries(mut self, auto: bool) -> Self {
        self.auto_generate_phrase_queries = auto;
        self
    }

    /// Set enable position increments
    pub fn enable_position_increments(mut self, enable: bool) -> Self {
        self.enable_position_increments = enable;
        self
    }

    /// Set escape
    pub fn escape(mut self, escape: bool) -> Self {
        self.escape = escape;
        self
    }

    /// Set fuzziness
    pub fn fuzziness(mut self, fuzziness: impl Into<String>) -> Self {
        self.fuzziness = Some(fuzziness.into());
        self
    }

    /// Set fuzzy max expansions
    pub fn fuzzy_max_expansions(mut self, max: u32) -> Self {
        self.fuzzy_max_expansions = max;
        self
    }

    /// Set fuzzy prefix length
    pub fn fuzzy_prefix_length(mut self, length: u32) -> Self {
        self.fuzzy_prefix_length = length;
        self
    }

    /// Set fuzzy transpositions
    pub fn fuzzy_transpositions(mut self, transpositions: bool) -> Self {
        self.fuzzy_transpositions = transpositions;
        self
    }

    /// Set lenient mode
    pub fn lenient(mut self, lenient: bool) -> Self {
        self.lenient = lenient;
        self
    }

    /// Set maximum determinized states
    pub fn max_determinized_states(mut self, max: u32) -> Self {
        self.max_determinized_states = max;
        self
    }

    /// Set minimum should match
    pub fn minimum_should_match(mut self, msm: impl Into<String>) -> Self {
        self.minimum_should_match = Some(msm.into());
        self
    }

    /// Set phrase slop
    pub fn phrase_slop(mut self, slop: u32) -> Self {
        self.phrase_slop = Some(slop);
        self
    }

    /// Set quote analyzer
    pub fn quote_analyzer(mut self, analyzer: impl Into<String>) -> Self {
        self.quote_analyzer = Some(analyzer.into());
        self
    }

    /// Set quote field suffix
    pub fn quote_field_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.quote_field_suffix = Some(suffix.into());
        self
    }

    /// Set tie breaker
    pub fn tie_breaker(mut self, tie_breaker: f32) -> Self {
        self.tie_breaker = Some(tie_breaker);
        self
    }

    /// Set time zone
    pub fn time_zone(mut self, time_zone: impl Into<String>) -> Self {
        self.time_zone = Some(time_zone.into());
        self
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
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
    /// Script language (optional, defaults to system default)
    #[serde(default)]
    pub lang: Option<String>,
    /// Script ID for stored scripts (optional)
    #[serde(default)]
    pub id: Option<String>,
}

impl ScriptQuery {
    /// Create new script query
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            params: std::collections::HashMap::new(),
            lang: None,
            id: None,
        }
    }

    /// Add a parameter to the script
    pub fn param(mut self, name: impl Into<String>, value: serde_json::Value) -> Self {
        self.params.insert(name.into(), value);
        self
    }

    /// Set script language
    pub fn lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = Some(lang.into());
        self
    }

    /// Set stored script ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add multiple parameters at once
    pub fn params(mut self, params: impl IntoIterator<Item = (String, serde_json::Value)>) -> Self {
        for (key, value) in params {
            self.params.insert(key, value);
        }
        self
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

/// Wrapper query for accepting serialized queries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WrapperQuery {
    /// Serialized query string (JSON)
    pub query: String,
}

impl WrapperQuery {
    /// Create new wrapper query from serialized query string
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
        }
    }

    /// Parse the wrapped query into a Query enum
    pub fn parse(&self) -> Result<Query, serde_json::Error> {
        serde_json::from_str(&self.query)
    }
}

/// Pinned query for promoting specific documents
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PinnedQuery {
    /// Document IDs to promote (pinned at the top)
    pub ids: Vec<String>,
    /// Organic query to execute (results appear below pinned documents)
    pub organic: Box<Query>,
}

impl PinnedQuery {
    /// Create new pinned query
    pub fn new(ids: Vec<String>, organic: Query) -> Self {
        Self {
            ids,
            organic: Box::new(organic),
        }
    }

    /// Add a document ID to pin
    pub fn pin(mut self, id: impl Into<String>) -> Self {
        self.ids.push(id.into());
        self
    }
}

/// Has child query for finding parent documents with matching children
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HasChildQuery {
    /// Child document type (relationship type)
    pub r#type: String,
    /// Query to match child documents
    pub query: Box<Query>,
    /// Score mode for parent documents
    #[serde(default)]
    pub score_mode: ParentChildScoreMode,
    /// Minimum number of matching children required
    #[serde(default)]
    pub min_children: Option<u32>,
    /// Maximum number of matching children to consider
    #[serde(default)]
    pub max_children: Option<u32>,
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

impl HasChildQuery {
    /// Create new has child query
    pub fn new(r#type: impl Into<String>, query: Query) -> Self {
        Self {
            r#type: r#type.into(),
            query: Box::new(query),
            score_mode: ParentChildScoreMode::None,
            min_children: None,
            max_children: None,
            boost: 1.0,
        }
    }

    /// Set score mode
    pub fn score_mode(mut self, mode: ParentChildScoreMode) -> Self {
        self.score_mode = mode;
        self
    }

    /// Set minimum number of children
    pub fn min_children(mut self, min: u32) -> Self {
        self.min_children = Some(min);
        self
    }

    /// Set maximum number of children
    pub fn max_children(mut self, max: u32) -> Self {
        self.max_children = Some(max);
        self
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }
}

/// Has parent query for finding child documents with matching parents
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HasParentQuery {
    /// Parent document type (relationship type)
    pub parent_type: String,
    /// Query to match parent documents
    pub query: Box<Query>,
    /// Score mode for child documents
    #[serde(default)]
    pub score_mode: ParentChildScoreMode,
    /// Boost factor for this query (default: 1.0)
    #[serde(default = "default_boost")]
    pub boost: f32,
}

impl HasParentQuery {
    /// Create new has parent query
    pub fn new(parent_type: impl Into<String>, query: Query) -> Self {
        Self {
            parent_type: parent_type.into(),
            query: Box::new(query),
            score_mode: ParentChildScoreMode::None,
            boost: 1.0,
        }
    }

    /// Set score mode
    pub fn score_mode(mut self, mode: ParentChildScoreMode) -> Self {
        self.score_mode = mode;
        self
    }

    /// Set boost factor for this query
    pub fn boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }
}

/// Score mode for parent-child queries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ParentChildScoreMode {
    /// No scoring (filter only)
    #[default]
    None,
    /// Average score of matching children/parents
    Avg,
    /// Sum of scores from matching children/parents
    Sum,
    /// Maximum score among matching children/parents
    Max,
    /// Minimum score among matching children/parents
    Min,
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
        assert_eq!(geo_query.boost, 1.0);
    }

    #[test]
    fn test_script_query() {
        let script_query = ScriptQuery::new("doc['field'].value > 10");

        assert_eq!(script_query.source, "doc['field'].value > 10");
        assert!(script_query.params.is_empty());
        assert!(script_query.lang.is_none());
        assert!(script_query.id.is_none());
    }

    #[test]
    fn test_script_query_with_param() {
        let script_query =
            ScriptQuery::new("doc['field'].value > param").param("param", serde_json::json!(10));

        assert_eq!(script_query.source, "doc['field'].value > param");
        assert_eq!(script_query.params.len(), 1);
        assert_eq!(
            script_query.params.get("param"),
            Some(&serde_json::json!(10))
        );
    }

    #[test]
    fn test_script_query_with_lang() {
        let script_query = ScriptQuery::new("doc['field'].value > 10").lang("javascript");

        assert_eq!(script_query.lang, Some("javascript".to_string()));
    }

    #[test]
    fn test_script_query_with_id() {
        let script_query = ScriptQuery::new("doc['field'].value > 10").id("my_script");

        assert_eq!(script_query.id, Some("my_script".to_string()));
    }

    #[test]
    fn test_script_query_with_multiple_params() {
        let script_query =
            ScriptQuery::new("doc['field'].value > param1 && doc['field2'].value < param2")
                .param("param1", serde_json::json!(10))
                .param("param2", serde_json::json!(100));

        assert_eq!(script_query.params.len(), 2);
        assert_eq!(
            script_query.params.get("param1"),
            Some(&serde_json::json!(10))
        );
        assert_eq!(
            script_query.params.get("param2"),
            Some(&serde_json::json!(100))
        );
    }

    #[test]
    fn test_script_query_params_builder() {
        let params = vec![
            ("param1".to_string(), serde_json::json!(10)),
            ("param2".to_string(), serde_json::json!(20)),
        ];
        let script_query = ScriptQuery::new("doc['field'].value > param1").params(params);

        assert_eq!(script_query.params.len(), 2);
    }

    #[test]
    fn test_script_query_serialization() {
        let script_query = ScriptQuery::new("doc['field'].value > param")
            .param("param", serde_json::json!(10))
            .lang("javascript")
            .id("my_script");

        let json = serde_json::to_string(&script_query).unwrap();
        assert!(json.contains("source"));
        assert!(json.contains("params"));
        assert!(json.contains("lang"));
        assert!(json.contains("id"));

        let deserialized: ScriptQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.source, "doc['field'].value > param");
        assert_eq!(deserialized.params.len(), 1);
        assert_eq!(deserialized.lang, Some("javascript".to_string()));
        assert_eq!(deserialized.id, Some("my_script".to_string()));
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
        // Updated to use builder pattern
        let script_query =
            ScriptQuery::new("doc['field'].value > param").param("param", serde_json::json!(10));

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

    #[test]
    fn test_wrapper_query() {
        let match_query = MatchQuery::new("title", "test");
        let query_json = serde_json::to_string(&Query::Match(match_query)).unwrap();
        let wrapper = WrapperQuery::new(query_json.clone());

        assert_eq!(wrapper.query, query_json);
    }

    #[test]
    fn test_wrapper_query_parse() {
        let match_query = MatchQuery::new("title", "test");
        let query_json = serde_json::to_string(&Query::Match(match_query)).unwrap();
        let wrapper = WrapperQuery::new(query_json);

        let parsed = wrapper.parse().unwrap();
        assert!(matches!(parsed, Query::Match(_)));
    }

    #[test]
    fn test_wrapper_query_serialization() {
        let query_json = r#"{"match":{"field":"title","query":"test"}}"#;
        let wrapper = WrapperQuery::new(query_json);

        let json = serde_json::to_string(&wrapper).unwrap();
        assert!(json.contains("query"));

        let deserialized: WrapperQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.query, query_json);
    }

    #[test]
    fn test_pinned_query() {
        let organic = Query::Match(MatchQuery::new("title", "test"));
        let pinned = PinnedQuery::new(vec!["doc1".to_string(), "doc2".to_string()], organic);

        assert_eq!(pinned.ids.len(), 2);
        assert_eq!(pinned.ids[0], "doc1");
        assert_eq!(pinned.ids[1], "doc2");
        assert!(matches!(pinned.organic.as_ref(), Query::Match(_)));
    }

    #[test]
    fn test_pinned_query_pin() {
        let organic = Query::Match(MatchQuery::new("title", "test"));
        let pinned = PinnedQuery::new(vec!["doc1".to_string()], organic).pin("doc2");

        assert_eq!(pinned.ids.len(), 2);
        assert_eq!(pinned.ids[1], "doc2");
    }

    #[test]
    fn test_pinned_query_serialization() {
        let organic = Query::Match(MatchQuery::new("title", "test"));
        let pinned = PinnedQuery::new(vec!["doc1".to_string(), "doc2".to_string()], organic);

        let json = serde_json::to_string(&pinned).unwrap();
        assert!(json.contains("ids"));
        assert!(json.contains("organic"));

        let deserialized: PinnedQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.ids.len(), 2);
        assert_eq!(deserialized.ids[0], "doc1");
    }

    #[test]
    fn test_more_like_this_query_builder() {
        let query = MoreLikeThisQuery::new(vec!["title".to_string()], "sample text")
            .min_term_freq(2)
            .max_query_terms(10)
            .min_doc_freq(3)
            .min_word_length(3);

        assert_eq!(query.min_term_freq, 2);
        assert_eq!(query.max_query_terms, 10);
        assert_eq!(query.min_doc_freq, 3);
        assert_eq!(query.min_word_length, 3);
    }

    #[test]
    fn test_nested_query_builder() {
        let inner_query = Query::Term(TermQuery::new("nested.field", "value"));
        let nested = NestedQuery::new("nested", inner_query).score_mode(NestedScoreMode::Max);

        assert_eq!(nested.path, "nested");
        assert!(matches!(nested.score_mode, NestedScoreMode::Max));
    }

    #[test]
    fn test_has_child_query() {
        let child_query = Query::Match(MatchQuery::new("status", "active"));
        let has_child = HasChildQuery::new("comment", child_query);

        assert_eq!(has_child.r#type, "comment");
        assert!(matches!(has_child.query.as_ref(), Query::Match(_)));
        assert!(matches!(has_child.score_mode, ParentChildScoreMode::None));
        assert_eq!(has_child.boost, 1.0);
    }

    #[test]
    fn test_has_child_query_with_score_mode() {
        let child_query = Query::Term(TermQuery::new("status", "active"));
        let has_child =
            HasChildQuery::new("comment", child_query).score_mode(ParentChildScoreMode::Avg);

        assert!(matches!(has_child.score_mode, ParentChildScoreMode::Avg));
    }

    #[test]
    fn test_has_child_query_with_min_max_children() {
        let child_query = Query::Match(MatchQuery::new("status", "active"));
        let has_child = HasChildQuery::new("comment", child_query)
            .min_children(2)
            .max_children(10);

        assert_eq!(has_child.min_children, Some(2));
        assert_eq!(has_child.max_children, Some(10));
    }

    #[test]
    fn test_has_child_query_with_boost() {
        let child_query = Query::Match(MatchQuery::new("status", "active"));
        let has_child = HasChildQuery::new("comment", child_query).boost(2.5);

        assert_eq!(has_child.boost, 2.5);
    }

    #[test]
    fn test_has_child_query_serialization() {
        let child_query = Query::Match(MatchQuery::new("status", "active"));
        let has_child = HasChildQuery::new("comment", child_query)
            .score_mode(ParentChildScoreMode::Avg)
            .min_children(2)
            .boost(1.5);

        let json = serde_json::to_string(&has_child).unwrap();
        assert!(json.contains("type"));
        assert!(json.contains("comment"));
        assert!(json.contains("query"));
        assert!(json.contains("score_mode"));
        assert!(json.contains("min_children"));
        assert!(json.contains("boost"));

        let deserialized: HasChildQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.r#type, "comment");
        assert!(matches!(deserialized.score_mode, ParentChildScoreMode::Avg));
        assert_eq!(deserialized.min_children, Some(2));
        assert_eq!(deserialized.boost, 1.5);
    }

    #[test]
    fn test_has_parent_query() {
        let parent_query = Query::Match(MatchQuery::new("status", "published"));
        let has_parent = HasParentQuery::new("blog", parent_query);

        assert_eq!(has_parent.parent_type, "blog");
        assert!(matches!(has_parent.query.as_ref(), Query::Match(_)));
        assert!(matches!(has_parent.score_mode, ParentChildScoreMode::None));
        assert_eq!(has_parent.boost, 1.0);
    }

    #[test]
    fn test_has_parent_query_with_score_mode() {
        let parent_query = Query::Term(TermQuery::new("status", "published"));
        let has_parent =
            HasParentQuery::new("blog", parent_query).score_mode(ParentChildScoreMode::Max);

        assert!(matches!(has_parent.score_mode, ParentChildScoreMode::Max));
    }

    #[test]
    fn test_has_parent_query_with_boost() {
        let parent_query = Query::Match(MatchQuery::new("status", "published"));
        let has_parent = HasParentQuery::new("blog", parent_query).boost(2.0);

        assert_eq!(has_parent.boost, 2.0);
    }

    #[test]
    fn test_has_parent_query_serialization() {
        let parent_query = Query::Match(MatchQuery::new("status", "published"));
        let has_parent = HasParentQuery::new("blog", parent_query)
            .score_mode(ParentChildScoreMode::Sum)
            .boost(1.5);

        let json = serde_json::to_string(&has_parent).unwrap();
        assert!(json.contains("parent_type"));
        assert!(json.contains("blog"));
        assert!(json.contains("query"));
        assert!(json.contains("score_mode"));
        assert!(json.contains("boost"));

        let deserialized: HasParentQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.parent_type, "blog");
        assert!(matches!(deserialized.score_mode, ParentChildScoreMode::Sum));
        assert_eq!(deserialized.boost, 1.5);
    }

    #[test]
    fn test_geo_distance_query_with_boost() {
        let geo_query = GeoDistanceQuery::new("location", "10km", 40.7128, -74.0060).boost(2.0);

        assert_eq!(geo_query.field, "location");
        assert_eq!(geo_query.distance, "10km");
        assert_eq!(geo_query.location.lat, 40.7128);
        assert_eq!(geo_query.location.lon, -74.0060);
        assert_eq!(geo_query.boost, 2.0);
    }

    #[test]
    fn test_geo_bounding_box_query() {
        let top_left = GeoPoint {
            lat: 40.8,
            lon: -74.0,
        };
        let bottom_right = GeoPoint {
            lat: 40.7,
            lon: -73.9,
        };
        let bbox_query =
            GeoBoundingBoxQuery::new("location", top_left.clone(), bottom_right.clone());

        assert_eq!(bbox_query.field, "location");
        assert_eq!(bbox_query.top_left.lat, top_left.lat);
        assert_eq!(bbox_query.bottom_right.lat, bottom_right.lat);
        assert_eq!(bbox_query.boost, 1.0);
    }

    #[test]
    fn test_geo_bounding_box_query_with_boost() {
        let top_left = GeoPoint {
            lat: 40.8,
            lon: -74.0,
        };
        let bottom_right = GeoPoint {
            lat: 40.7,
            lon: -73.9,
        };
        let bbox_query = GeoBoundingBoxQuery::new("location", top_left, bottom_right).boost(1.5);

        assert_eq!(bbox_query.boost, 1.5);
    }

    #[test]
    fn test_geo_bounding_box_query_serialization() {
        let top_left = GeoPoint {
            lat: 40.8,
            lon: -74.0,
        };
        let bottom_right = GeoPoint {
            lat: 40.7,
            lon: -73.9,
        };
        let bbox_query = GeoBoundingBoxQuery::new("location", top_left, bottom_right).boost(1.5);

        let json = serde_json::to_string(&bbox_query).unwrap();
        assert!(json.contains("field"));
        assert!(json.contains("top_left"));
        assert!(json.contains("bottom_right"));
        assert!(json.contains("boost"));

        let deserialized: GeoBoundingBoxQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "location");
        assert_eq!(deserialized.boost, 1.5);
    }

    #[test]
    fn test_geo_polygon_query() {
        let points = vec![
            GeoPoint {
                lat: 40.8,
                lon: -74.0,
            },
            GeoPoint {
                lat: 40.8,
                lon: -73.9,
            },
            GeoPoint {
                lat: 40.7,
                lon: -73.9,
            },
            GeoPoint {
                lat: 40.7,
                lon: -74.0,
            },
        ];
        let polygon_query = GeoPolygonQuery::new("location", points.clone());

        assert_eq!(polygon_query.field, "location");
        assert_eq!(polygon_query.points.len(), 4);
        assert!(polygon_query.holes.is_empty());
        assert_eq!(polygon_query.boost, 1.0);
    }

    #[test]
    fn test_geo_polygon_query_with_holes() {
        let points = vec![
            GeoPoint {
                lat: 40.8,
                lon: -74.0,
            },
            GeoPoint {
                lat: 40.8,
                lon: -73.9,
            },
            GeoPoint {
                lat: 40.7,
                lon: -73.9,
            },
            GeoPoint {
                lat: 40.7,
                lon: -74.0,
            },
        ];
        let hole = vec![
            GeoPoint {
                lat: 40.75,
                lon: -73.95,
            },
            GeoPoint {
                lat: 40.75,
                lon: -73.92,
            },
            GeoPoint {
                lat: 40.72,
                lon: -73.92,
            },
            GeoPoint {
                lat: 40.72,
                lon: -73.95,
            },
        ];
        let polygon_query = GeoPolygonQuery::new("location", points).add_hole(hole);

        assert_eq!(polygon_query.holes.len(), 1);
        assert_eq!(polygon_query.holes[0].len(), 4);
    }

    #[test]
    fn test_geo_polygon_query_with_boost() {
        let points = vec![GeoPoint {
            lat: 40.8,
            lon: -74.0,
        }];
        let polygon_query = GeoPolygonQuery::new("location", points).boost(2.0);

        assert_eq!(polygon_query.boost, 2.0);
    }

    #[test]
    fn test_geo_polygon_query_serialization() {
        let points = vec![
            GeoPoint {
                lat: 40.8,
                lon: -74.0,
            },
            GeoPoint {
                lat: 40.7,
                lon: -73.9,
            },
        ];
        let polygon_query = GeoPolygonQuery::new("location", points).boost(1.5);

        let json = serde_json::to_string(&polygon_query).unwrap();
        assert!(json.contains("field"));
        assert!(json.contains("points"));
        assert!(json.contains("boost"));

        let deserialized: GeoPolygonQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "location");
        assert_eq!(deserialized.points.len(), 2);
        assert_eq!(deserialized.boost, 1.5);
    }

    #[test]
    fn test_geo_shape_query_point() {
        let shape = GeoShape::Point {
            coordinates: GeoPoint {
                lat: 40.7128,
                lon: -74.0060,
            },
        };
        let shape_query = GeoShapeQuery::new("location", shape);

        assert_eq!(shape_query.field, "location");
        assert!(matches!(shape_query.shape, GeoShape::Point { .. }));
        assert!(matches!(shape_query.relation, GeoShapeRelation::Intersects));
    }

    #[test]
    fn test_geo_shape_query_circle() {
        let shape = GeoShape::Circle {
            coordinates: GeoPoint {
                lat: 40.7128,
                lon: -74.0060,
            },
            radius: "10km".to_string(),
        };
        let shape_query = GeoShapeQuery::new("location", shape);

        assert!(matches!(shape_query.shape, GeoShape::Circle { .. }));
    }

    #[test]
    fn test_geo_shape_query_polygon() {
        let shape = GeoShape::Polygon {
            coordinates: vec![vec![
                GeoPoint {
                    lat: 40.8,
                    lon: -74.0,
                },
                GeoPoint {
                    lat: 40.7,
                    lon: -73.9,
                },
            ]],
        };
        let shape_query = GeoShapeQuery::new("location", shape);

        assert!(matches!(shape_query.shape, GeoShape::Polygon { .. }));
    }

    #[test]
    fn test_geo_shape_query_with_relation() {
        let shape = GeoShape::Point {
            coordinates: GeoPoint {
                lat: 40.7128,
                lon: -74.0060,
            },
        };
        let shape_query =
            GeoShapeQuery::new("location", shape).relation(GeoShapeRelation::Contains);

        assert!(matches!(shape_query.relation, GeoShapeRelation::Contains));
    }

    #[test]
    fn test_geo_shape_query_with_boost() {
        let shape = GeoShape::Point {
            coordinates: GeoPoint {
                lat: 40.7128,
                lon: -74.0060,
            },
        };
        let shape_query = GeoShapeQuery::new("location", shape).boost(2.0);

        assert_eq!(shape_query.boost, 2.0);
    }

    #[test]
    fn test_geo_shape_query_serialization() {
        let shape = GeoShape::Circle {
            coordinates: GeoPoint {
                lat: 40.7128,
                lon: -74.0060,
            },
            radius: "10km".to_string(),
        };
        let shape_query = GeoShapeQuery::new("location", shape)
            .relation(GeoShapeRelation::Within)
            .boost(1.5);

        let json = serde_json::to_string(&shape_query).unwrap();
        assert!(json.contains("field"));
        assert!(json.contains("shape"));
        assert!(json.contains("relation"));
        assert!(json.contains("boost"));

        let deserialized: GeoShapeQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "location");
        assert!(matches!(deserialized.relation, GeoShapeRelation::Within));
        assert_eq!(deserialized.boost, 1.5);
    }

    #[test]
    fn test_percolate_query() {
        let document = serde_json::json!({
            "title": "Elasticsearch guide",
            "content": "This is a guide to Elasticsearch"
        });
        let percolate_query = PercolateQuery::new("document", document.clone());

        assert_eq!(percolate_query.field, "document");
        assert_eq!(percolate_query.document, document);
        assert_eq!(percolate_query.boost, 1.0);
        assert!(percolate_query.index.is_none());
    }

    #[test]
    fn test_percolate_query_with_index() {
        let document = serde_json::json!({"title": "test"});
        let percolate_query = PercolateQuery::new("document", document).index("queries");

        assert_eq!(percolate_query.index, Some("queries".to_string()));
    }

    #[test]
    fn test_percolate_query_with_document_type() {
        let document = serde_json::json!({"title": "test"});
        let percolate_query = PercolateQuery::new("document", document).document_type("alert");

        assert_eq!(percolate_query.document_type, Some("alert".to_string()));
    }

    #[test]
    fn test_percolate_query_with_preferred_sources() {
        let document = serde_json::json!({"title": "test"});
        let sources = vec!["source1".to_string(), "source2".to_string()];
        let percolate_query =
            PercolateQuery::new("document", document).preferred_sources(sources.clone());

        assert_eq!(percolate_query.preferred_sources, Some(sources));
    }

    #[test]
    fn test_percolate_query_with_boost() {
        let document = serde_json::json!({"title": "test"});
        let percolate_query = PercolateQuery::new("document", document).boost(2.0);

        assert_eq!(percolate_query.boost, 2.0);
    }

    #[test]
    fn test_percolate_query_serialization() {
        let document = serde_json::json!({
            "title": "Elasticsearch guide",
            "content": "This is a guide"
        });
        let percolate_query = PercolateQuery::new("document", document.clone())
            .index("queries")
            .document_type("alert")
            .boost(1.5);

        let json = serde_json::to_string(&percolate_query).unwrap();
        assert!(json.contains("field"));
        assert!(json.contains("document"));
        assert!(json.contains("index"));
        assert!(json.contains("document_type"));
        assert!(json.contains("boost"));

        let deserialized: PercolateQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "document");
        assert_eq!(deserialized.document, document);
        assert_eq!(deserialized.index, Some("queries".to_string()));
        assert_eq!(deserialized.document_type, Some("alert".to_string()));
        assert_eq!(deserialized.boost, 1.5);
    }

    #[test]
    fn test_simple_query_string_query() {
        let query = SimpleQueryStringQuery::new("quick brown fox");

        assert_eq!(query.query, "quick brown fox");
        assert!(query.fields.is_empty());
        assert!(matches!(
            query.default_operator,
            SimpleQueryStringOperator::Or
        ));
        assert_eq!(query.boost, 1.0);
    }

    #[test]
    fn test_simple_query_string_query_with_fields() {
        let query = SimpleQueryStringQuery::new("test")
            .fields(vec!["title".to_string(), "content".to_string()]);

        assert_eq!(query.fields.len(), 2);
        assert_eq!(query.fields[0], "title");
    }

    #[test]
    fn test_simple_query_string_query_with_default_operator() {
        let query =
            SimpleQueryStringQuery::new("test").default_operator(SimpleQueryStringOperator::And);

        assert!(matches!(
            query.default_operator,
            SimpleQueryStringOperator::And
        ));
    }

    #[test]
    fn test_simple_query_string_query_with_flags() {
        let flags = SimpleQueryStringFlags {
            fuzzy: false,
            ..Default::default()
        };
        let query = SimpleQueryStringQuery::new("test").flags(flags);

        assert!(!query.flags.fuzzy);
    }

    #[test]
    fn test_simple_query_string_query_with_fuzzy_options() {
        let query = SimpleQueryStringQuery::new("test")
            .fuzzy_max_expansions(100)
            .fuzzy_prefix_length(3)
            .fuzzy_transpositions(false);

        assert_eq!(query.fuzzy_max_expansions, 100);
        assert_eq!(query.fuzzy_prefix_length, 3);
        assert!(!query.fuzzy_transpositions);
    }

    #[test]
    fn test_simple_query_string_query_with_minimum_should_match() {
        let query = SimpleQueryStringQuery::new("test").minimum_should_match("75%");

        assert_eq!(query.minimum_should_match, Some("75%".to_string()));
    }

    #[test]
    fn test_simple_query_string_query_with_boost() {
        let query = SimpleQueryStringQuery::new("test").boost(2.0);

        assert_eq!(query.boost, 2.0);
    }

    #[test]
    fn test_simple_query_string_query_serialization() {
        let query = SimpleQueryStringQuery::new("quick brown fox")
            .fields(vec!["title".to_string()])
            .default_operator(SimpleQueryStringOperator::And)
            .fuzzy_max_expansions(100)
            .boost(1.5);

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("query"));
        assert!(json.contains("fields"));
        assert!(json.contains("default_operator"));
        assert!(json.contains("fuzzy_max_expansions"));
        assert!(json.contains("boost"));

        let deserialized: SimpleQueryStringQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.query, "quick brown fox");
        assert_eq!(deserialized.fields.len(), 1);
        assert!(matches!(
            deserialized.default_operator,
            SimpleQueryStringOperator::And
        ));
        assert_eq!(deserialized.boost, 1.5);
    }

    #[test]
    fn test_query_string_query() {
        use super::{QueryStringOperator, QueryStringQuery};
        let query = QueryStringQuery::new("quick brown fox");

        assert_eq!(query.query, "quick brown fox");
        assert!(query.default_field.is_none());
        assert!(query.fields.is_empty());
        assert!(matches!(query.default_operator, QueryStringOperator::Or));
        assert_eq!(query.boost, 1.0);
    }

    #[test]
    fn test_query_string_query_with_default_field() {
        use super::QueryStringQuery;
        let query = QueryStringQuery::new("test").default_field("title");

        assert_eq!(query.default_field, Some("title".to_string()));
    }

    #[test]
    fn test_query_string_query_with_fields() {
        use super::QueryStringQuery;
        let query =
            QueryStringQuery::new("test").fields(vec!["title".to_string(), "content".to_string()]);

        assert_eq!(query.fields.len(), 2);
        assert_eq!(query.fields[0], "title");
    }

    #[test]
    fn test_query_string_query_with_default_operator() {
        use super::{QueryStringOperator, QueryStringQuery};
        let query = QueryStringQuery::new("test").default_operator(QueryStringOperator::And);

        assert!(matches!(query.default_operator, QueryStringOperator::And));
    }

    #[test]
    fn test_query_string_query_with_fuzziness() {
        use super::QueryStringQuery;
        let query = QueryStringQuery::new("test").fuzziness("AUTO");

        assert_eq!(query.fuzziness, Some("AUTO".to_string()));
    }

    #[test]
    fn test_query_string_query_with_phrase_slop() {
        use super::QueryStringQuery;
        let query = QueryStringQuery::new("test").phrase_slop(2);

        assert_eq!(query.phrase_slop, Some(2));
    }

    #[test]
    fn test_query_string_query_with_tie_breaker() {
        use super::QueryStringQuery;
        let query = QueryStringQuery::new("test").tie_breaker(0.3);

        assert_eq!(query.tie_breaker, Some(0.3));
    }

    #[test]
    fn test_query_string_query_with_time_zone() {
        use super::QueryStringQuery;
        let query = QueryStringQuery::new("test").time_zone("UTC");

        assert_eq!(query.time_zone, Some("UTC".to_string()));
    }

    #[test]
    fn test_query_string_query_with_boost() {
        use super::QueryStringQuery;
        let query = QueryStringQuery::new("test").boost(2.0);

        assert_eq!(query.boost, 2.0);
    }

    #[test]
    fn test_query_string_query_serialization() {
        use super::{QueryStringOperator, QueryStringQuery};
        let query = QueryStringQuery::new("title:(quick OR brown) AND \"fox jumps\"~2")
            .default_field("title")
            .default_operator(QueryStringOperator::And)
            .fuzziness("AUTO")
            .phrase_slop(2)
            .boost(1.5);

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("query"));
        assert!(json.contains("default_field"));
        assert!(json.contains("default_operator"));
        assert!(json.contains("fuzziness"));
        assert!(json.contains("phrase_slop"));
        assert!(json.contains("boost"));

        let deserialized: QueryStringQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.query,
            "title:(quick OR brown) AND \"fox jumps\"~2"
        );
        assert_eq!(deserialized.default_field, Some("title".to_string()));
        assert!(matches!(
            deserialized.default_operator,
            QueryStringOperator::And
        ));
        assert_eq!(deserialized.boost, 1.5);
    }
}
