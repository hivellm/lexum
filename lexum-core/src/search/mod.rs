//! Search execution module

pub mod executor;
pub mod field_cache;
pub mod filter_cache;
pub mod highlighter;
pub mod multi_executor;
pub mod optimizer;
pub mod query_cache;
pub mod regex_cache;
pub mod result;

pub use field_cache::{FieldAggregationStats, FieldCache, FieldCacheStats, FieldValue};

pub use executor::SearchExecutor;
pub use filter_cache::{FilterCache, FilterCacheStats};
pub use highlighter::{Highlighter, HighlighterConfig};
pub use multi_executor::MultiIndexSearchExecutor;
pub use optimizer::{QueryAnalysis, QueryOptimizer};
pub use query_cache::{QueryCache, QueryCacheStats};
pub use result::{SearchHit, SearchResult, SortOption, SortOrder};
