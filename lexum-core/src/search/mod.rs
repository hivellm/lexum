//! Search execution module

pub mod executor;
pub mod multi_executor;
pub mod result;

pub use executor::SearchExecutor;
pub use multi_executor::MultiIndexSearchExecutor;
pub use result::{SearchHit, SearchResult, SortOption, SortOrder};
