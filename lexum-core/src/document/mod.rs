//! Document operations module

pub mod multi_get;
pub mod progress_store;
pub mod query_operations;
pub mod store;
pub mod stored_field_compression;

pub use multi_get::{MultiGet, MultiGetRequest, MultiGetResponse};
pub use progress_store::ProgressDocumentStore;
pub use query_operations::{
    DeleteByQueryRequest, DeleteByQueryResponse, QueryOperations, UpdateByQueryRequest,
    UpdateByQueryResponse,
};
pub use store::DocumentStore;
