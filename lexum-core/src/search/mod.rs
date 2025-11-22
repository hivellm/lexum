//! Search execution module

pub mod collapse;
pub mod executor;
pub mod field_cache;
pub mod filter_cache;
pub mod highlighter;
pub mod inner_hits;
pub mod multi_executor;
pub mod multi_search;
pub mod optimizer;
pub mod point_in_time;
pub mod query_cache;
pub mod regex_cache;
pub mod result;
pub mod scroll;
pub mod search_after;
pub mod suggester;

pub use field_cache::{FieldAggregationStats, FieldCache, FieldCacheStats, FieldValue};

pub use collapse::{
    CollapseConfig, CollapseExecutor, CollapseRequest, CollapseResponse, CollapsedHit,
};
pub use executor::SearchExecutor;
pub use filter_cache::{FilterCache, FilterCacheStats};
pub use highlighter::{Highlighter, HighlighterConfig};
pub use inner_hits::{
    InnerHit, InnerHitsConfig, InnerHitsProcessor, InnerHitsResult, NestedMetadata,
};
pub use multi_executor::MultiIndexSearchExecutor;
pub use multi_search::{MultiSearchExecutor, MultiSearchRequest, MultiSearchResponse};
pub use optimizer::{QueryAnalysis, QueryOptimizer};
pub use point_in_time::{
    PitId, PointInTimeExecutor, PointInTimeManager, PointInTimeRequest, PointInTimeResponse,
    parse_duration,
};
pub use query_cache::{QueryCache, QueryCacheStats};
pub use result::{SearchHit, SearchResult, SortOption, SortOrder};
pub use scroll::{ScrollExecutor, ScrollId, ScrollManager, ScrollRequest, ScrollResponse};
pub use search_after::{SearchAfterExecutor, SearchAfterRequest, SearchAfterResponse};
pub use suggester::{Suggester, SuggesterConfig, Suggestion, SuggestionType};
