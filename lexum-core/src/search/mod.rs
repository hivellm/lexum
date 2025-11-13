//! Search execution module

pub mod executor;
pub mod field_cache;
pub mod filter_cache;
pub mod multi_executor;
pub mod optimizer;
pub mod result;

pub use field_cache::{FieldCache, FieldCacheStats, FieldValue};

pub use executor::SearchExecutor;
pub use filter_cache::{FilterCache, FilterCacheStats};
pub use multi_executor::MultiIndexSearchExecutor;
pub use optimizer::{QueryAnalysis, QueryOptimizer};
pub use result::{SearchHit, SearchResult, SortOption, SortOrder};
