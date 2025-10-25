//! Document operation handlers

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use lexum_core::DocumentStore;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Add document request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddDocumentRequest {
    /// Document data
    pub document: JsonValue,
}

/// Add document response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddDocumentResponse {
    /// Document ID
    pub id: String,
}

/// Add document handler
pub async fn add_document(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
    Json(request): Json<AddDocumentRequest>,
) -> ApiResult<(StatusCode, Json<AddDocumentResponse>)> {
    let index = state
        .index_manager
        .get_index(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    let store = DocumentStore::new(Arc::new(index));
    let doc_id = store.add_document(request.document).await?;

    Ok((
        StatusCode::CREATED,
        Json(AddDocumentResponse {
            id: doc_id.to_string(),
        }),
    ))
}

/// Get document handler
pub async fn get_document(
    State(state): State<AppState>,
    Path((index_name, doc_id)): Path<(String, String)>,
) -> ApiResult<Json<JsonValue>> {
    let index = state
        .index_manager
        .get_index(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    let store = DocumentStore::new(Arc::new(index));
    let doc = store
        .get_document(&lexum_core::types::DocumentId::new(doc_id.clone()))
        .await
        .map_err(|_| ApiError::DocumentNotFound(doc_id))?;

    Ok(Json(doc))
}

/// Update document handler
pub async fn update_document(
    State(state): State<AppState>,
    Path((index_name, doc_id)): Path<(String, String)>,
    Json(request): Json<AddDocumentRequest>,
) -> ApiResult<StatusCode> {
    let index = state
        .index_manager
        .get_index(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    let store = DocumentStore::new(Arc::new(index));
    store
        .update_document(
            &lexum_core::types::DocumentId::new(doc_id),
            request.document,
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Delete document handler
pub async fn delete_document(
    State(state): State<AppState>,
    Path((index_name, doc_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let index = state
        .index_manager
        .get_index(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    let store = DocumentStore::new(Arc::new(index));
    store
        .delete_document(&lexum_core::types::DocumentId::new(doc_id))
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Bulk operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum BulkOperation {
    /// Index a document (create or update)
    Index {
        /// Index name
        #[serde(rename = "_index")]
        index: String,
        /// Optional document ID
        #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Document data
        document: JsonValue,
    },
    /// Create a document (fails if exists)
    Create {
        /// Index name
        #[serde(rename = "_index")]
        index: String,
        /// Document ID
        #[serde(rename = "_id")]
        id: String,
        /// Document data
        document: JsonValue,
    },
    /// Update a document
    Update {
        /// Index name
        #[serde(rename = "_index")]
        index: String,
        /// Document ID
        #[serde(rename = "_id")]
        id: String,
        /// Document data
        document: JsonValue,
    },
    /// Delete a document
    Delete {
        /// Index name
        #[serde(rename = "_index")]
        index: String,
        /// Document ID
        #[serde(rename = "_id")]
        id: String,
    },
}

/// Bulk operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Operation action
    pub action: String,
    /// Index name
    #[serde(rename = "_index")]
    pub index: String,
    /// Document ID (if applicable)
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Bulk operations request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkRequest {
    /// List of operations to perform
    pub operations: Vec<BulkOperation>,
}

/// Bulk operations response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkResponse {
    /// Whether all operations succeeded
    pub errors: bool,
    /// Number of operations
    pub took_ms: u64,
    /// Results for each operation
    pub items: Vec<BulkOperationResult>,
}

/// Bulk operations handler
pub async fn bulk_operations(
    State(state): State<AppState>,
    Json(request): Json<BulkRequest>,
) -> ApiResult<Json<BulkResponse>> {
    use std::time::Instant;
    let start = Instant::now();

    let mut results = Vec::new();
    let mut has_errors = false;

    for operation in request.operations {
        let result = match operation {
            BulkOperation::Index {
                index: index_name,
                id,
                document,
            } => match state.index_manager.get_index(&index_name) {
                Ok(index) => {
                    let store = DocumentStore::new(Arc::new(index));
                    match store.add_document(document).await {
                        Ok(doc_id) => BulkOperationResult {
                            success: true,
                            action: "index".to_string(),
                            index: index_name,
                            id: Some(id.unwrap_or_else(|| doc_id.to_string())),
                            error: None,
                        },
                        Err(e) => {
                            has_errors = true;
                            BulkOperationResult {
                                success: false,
                                action: "index".to_string(),
                                index: index_name,
                                id,
                                error: Some(e.to_string()),
                            }
                        }
                    }
                }
                Err(e) => {
                    has_errors = true;
                    BulkOperationResult {
                        success: false,
                        action: "index".to_string(),
                        index: index_name,
                        id,
                        error: Some(e.to_string()),
                    }
                }
            },
            BulkOperation::Create {
                index: index_name,
                id,
                document,
            } => {
                // Same as index for now (we don't check if exists)
                match state.index_manager.get_index(&index_name) {
                    Ok(index) => {
                        let store = DocumentStore::new(Arc::new(index));
                        match store.add_document(document).await {
                            Ok(_) => BulkOperationResult {
                                success: true,
                                action: "create".to_string(),
                                index: index_name,
                                id: Some(id),
                                error: None,
                            },
                            Err(e) => {
                                has_errors = true;
                                BulkOperationResult {
                                    success: false,
                                    action: "create".to_string(),
                                    index: index_name,
                                    id: Some(id),
                                    error: Some(e.to_string()),
                                }
                            }
                        }
                    }
                    Err(e) => {
                        has_errors = true;
                        BulkOperationResult {
                            success: false,
                            action: "create".to_string(),
                            index: index_name,
                            id: Some(id),
                            error: Some(e.to_string()),
                        }
                    }
                }
            }
            BulkOperation::Update {
                index: index_name,
                id,
                document,
            } => match state.index_manager.get_index(&index_name) {
                Ok(index) => {
                    let store = DocumentStore::new(Arc::new(index));
                    match store
                        .update_document(&lexum_core::types::DocumentId::new(id.clone()), document)
                        .await
                    {
                        Ok(_) => BulkOperationResult {
                            success: true,
                            action: "update".to_string(),
                            index: index_name,
                            id: Some(id),
                            error: None,
                        },
                        Err(e) => {
                            has_errors = true;
                            BulkOperationResult {
                                success: false,
                                action: "update".to_string(),
                                index: index_name,
                                id: Some(id),
                                error: Some(e.to_string()),
                            }
                        }
                    }
                }
                Err(e) => {
                    has_errors = true;
                    BulkOperationResult {
                        success: false,
                        action: "update".to_string(),
                        index: index_name,
                        id: Some(id),
                        error: Some(e.to_string()),
                    }
                }
            },
            BulkOperation::Delete {
                index: index_name,
                id,
            } => match state.index_manager.get_index(&index_name) {
                Ok(index) => {
                    let store = DocumentStore::new(Arc::new(index));
                    match store
                        .delete_document(&lexum_core::types::DocumentId::new(id.clone()))
                        .await
                    {
                        Ok(_) => BulkOperationResult {
                            success: true,
                            action: "delete".to_string(),
                            index: index_name,
                            id: Some(id),
                            error: None,
                        },
                        Err(e) => {
                            has_errors = true;
                            BulkOperationResult {
                                success: false,
                                action: "delete".to_string(),
                                index: index_name,
                                id: Some(id),
                                error: Some(e.to_string()),
                            }
                        }
                    }
                }
                Err(e) => {
                    has_errors = true;
                    BulkOperationResult {
                        success: false,
                        action: "delete".to_string(),
                        index: index_name,
                        id: Some(id),
                        error: Some(e.to_string()),
                    }
                }
            },
        };

        results.push(result);
    }

    Ok(Json(BulkResponse {
        errors: has_errors,
        took_ms: start.elapsed().as_millis() as u64,
        items: results,
    }))
}
