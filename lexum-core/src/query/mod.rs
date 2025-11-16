//! Query types and builders

pub mod builder;
pub mod types;

pub use builder::QueryBuilder;
pub use types::{
    BoolQuery, FuzzyQuery, MatchQuery, MultiMatchOperator, MultiMatchQuery, MultiMatchType,
    PhraseQuery, Query, RangeQuery, RegexQuery, TermQuery, WildcardQuery,
};
