//! Document operation handlers

use crate::error::{ApiError, ApiResult, ErrorResponse};
use crate::handlers::index::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use lexum_core::DocumentStore;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Add document request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AddDocumentRequest {
    /// Document data
    pub document: JsonValue,
}

/// Add document response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AddDocumentResponse {
    /// Document ID
    pub id: String,
}

/// Add document handler
#[utoipa::path(
    post,
    path = "/indices/{index_name}/documents",
    params(
        ("index_name" = String, Path, description = "Index name")
    ),
    request_body = AddDocumentRequest,
    responses(
        (status = 201, description = "Document added successfully", body = AddDocumentResponse),
        (status = 404, description = "Index not found", body = ErrorResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse)
    ),
    tag = "Documents"
)]
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
#[utoipa::path(
    get,
    path = "/indices/{index_name}/documents/{doc_id}",
    params(
        ("index_name" = String, Path, description = "Index name"),
        ("doc_id" = String, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document retrieved successfully", body = JsonValue),
        (status = 404, description = "Index or document not found", body = ErrorResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse)
    ),
    tag = "Documents"
)]
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
#[utoipa::path(
    put,
    path = "/indices/{index_name}/documents/{doc_id}",
    params(
        ("index_name" = String, Path, description = "Index name"),
        ("doc_id" = String, Path, description = "Document ID")
    ),
    request_body = AddDocumentRequest,
    responses(
        (status = 200, description = "Document updated successfully", body = AddDocumentResponse),
        (status = 404, description = "Index or document not found", body = ErrorResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse)
    ),
    tag = "Documents"
)]
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
#[utoipa::path(
    delete,
    path = "/indices/{index_name}/documents/{doc_id}",
    params(
        ("index_name" = String, Path, description = "Index name"),
        ("doc_id" = String, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document deleted successfully"),
        (status = 404, description = "Index or document not found", body = ErrorResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse)
    ),
    tag = "Documents"
)]
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
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BulkRequest {
    /// List of operations to perform
    pub operations: Vec<BulkOperation>,
}

/// Bulk operations response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BulkResponse {
    /// Whether all operations succeeded
    pub errors: bool,
    /// Number of operations
    pub took_ms: u64,
    /// Results for each operation
    pub items: Vec<BulkOperationResult>,
}

/// Bulk operations handler
#[utoipa::path(
    post,
    path = "/api/v1/bulk",
    request_body = BulkRequest,
    responses(
        (status = 200, description = "Bulk operations completed", body = BulkResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Documents"
)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_document_request_serialization() {
        let request = AddDocumentRequest {
            document: serde_json::json!({
                "title": "Test Document",
                "content": "This is test content"
            }),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: AddDocumentRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.document, deserialized.document);
    }

    #[test]
    fn test_add_document_response_serialization() {
        let response = AddDocumentResponse {
            id: "doc123".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: AddDocumentResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.id, deserialized.id);
    }

    #[test]
    fn test_bulk_response_serialization() {
        let response = BulkResponse {
            errors: false,
            took_ms: 100,
            items: vec![
                BulkOperationResult {
                    success: true,
                    action: "index".to_string(),
                    index: "test_index".to_string(),
                    id: Some("doc1".to_string()),
                    error: None,
                },
                BulkOperationResult {
                    success: false,
                    action: "index".to_string(),
                    index: "test_index".to_string(),
                    id: Some("doc2".to_string()),
                    error: Some("Invalid document".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: BulkResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.errors, deserialized.errors);
        assert_eq!(response.took_ms, deserialized.took_ms);
        assert_eq!(response.items.len(), deserialized.items.len());
    }

    #[test]
    fn test_bulk_operation_serialization() {
        let operations = vec![
            BulkOperation::Index {
                index: "test_index".to_string(),
                id: Some("doc1".to_string()),
                document: serde_json::json!({"title": "Doc 1"}),
            },
            BulkOperation::Create {
                index: "test_index".to_string(),
                id: "doc2".to_string(),
                document: serde_json::json!({"title": "Doc 2"}),
            },
            BulkOperation::Update {
                index: "test_index".to_string(),
                id: "doc3".to_string(),
                document: serde_json::json!({"title": "Doc 3"}),
            },
            BulkOperation::Delete {
                index: "test_index".to_string(),
                id: "doc4".to_string(),
            },
        ];

        for operation in operations {
            let json = serde_json::to_string(&operation).unwrap();
            let _deserialized: BulkOperation = serde_json::from_str(&json).unwrap();

            // Verify the operation can be serialized and deserialized
            assert!(
                matches!(operation, BulkOperation::Index { .. })
                    || matches!(operation, BulkOperation::Create { .. })
                    || matches!(operation, BulkOperation::Update { .. })
                    || matches!(operation, BulkOperation::Delete { .. })
            );
        }
    }

    #[test]
    fn test_bulk_operation_result_serialization() {
        let result = BulkOperationResult {
            success: true,
            action: "index".to_string(),
            index: "test_index".to_string(),
            id: Some("doc123".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: BulkOperationResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.success, deserialized.success);
        assert_eq!(result.action, deserialized.action);
        assert_eq!(result.index, deserialized.index);
        assert_eq!(result.id, deserialized.id);
        assert_eq!(result.error, deserialized.error);
    }

    #[test]
    fn test_bulk_operation_result_with_error() {
        let result = BulkOperationResult {
            success: false,
            action: "create".to_string(),
            index: "test_index".to_string(),
            id: Some("doc456".to_string()),
            error: Some("Document already exists".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: BulkOperationResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.success, deserialized.success);
        assert_eq!(result.error, deserialized.error);
    }

    #[test]
    fn test_bulk_request_serialization() {
        let request = BulkRequest {
            operations: vec![
                BulkOperation::Index {
                    index: "test_index".to_string(),
                    id: None,
                    document: serde_json::json!({"title": "Test"}),
                },
                BulkOperation::Delete {
                    index: "test_index".to_string(),
                    id: "doc1".to_string(),
                },
            ],
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: BulkRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.operations.len(), deserialized.operations.len());
    }

    #[test]
    fn test_bulk_operation_variants() {
        // Test Index operation with ID
        let index_with_id = BulkOperation::Index {
            index: "test".to_string(),
            id: Some("doc1".to_string()),
            document: serde_json::json!({"title": "Test"}),
        };
        let json = serde_json::to_string(&index_with_id).unwrap();
        let deserialized: BulkOperation = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, BulkOperation::Index { .. }));

        // Test Index operation without ID
        let index_without_id = BulkOperation::Index {
            index: "test".to_string(),
            id: None,
            document: serde_json::json!({"title": "Test"}),
        };
        let json = serde_json::to_string(&index_without_id).unwrap();
        let deserialized: BulkOperation = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, BulkOperation::Index { .. }));

        // Test Create operation
        let create_op = BulkOperation::Create {
            index: "test".to_string(),
            id: "doc2".to_string(),
            document: serde_json::json!({"title": "Test"}),
        };
        let json = serde_json::to_string(&create_op).unwrap();
        let deserialized: BulkOperation = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, BulkOperation::Create { .. }));

        // Test Update operation
        let update_op = BulkOperation::Update {
            index: "test".to_string(),
            id: "doc3".to_string(),
            document: serde_json::json!({"title": "Updated"}),
        };
        let json = serde_json::to_string(&update_op).unwrap();
        let deserialized: BulkOperation = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, BulkOperation::Update { .. }));

        // Test Delete operation
        let delete_op = BulkOperation::Delete {
            index: "test".to_string(),
            id: "doc4".to_string(),
        };
        let json = serde_json::to_string(&delete_op).unwrap();
        let deserialized: BulkOperation = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, BulkOperation::Delete { .. }));
    }

    #[test]
    fn test_add_document_request_with_complex_json() {
        let complex_doc = serde_json::json!({
            "title": "Complex Document",
            "content": "This is a complex document with nested structures",
            "metadata": {
                "author": "Test Author",
                "tags": ["test", "document", "complex"],
                "nested": {
                    "level1": {
                        "level2": "deep value"
                    }
                }
            },
            "numbers": [1, 2, 3, 4, 5],
            "boolean": true,
            "null_value": null
        });

        let request = AddDocumentRequest {
            document: complex_doc,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: AddDocumentRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.document, deserialized.document);
    }

    #[test]
    fn test_bulk_response_with_mixed_results() {
        let response = BulkResponse {
            errors: true, // Some operations failed
            took_ms: 250,
            items: vec![
                BulkOperationResult {
                    success: true,
                    action: "index".to_string(),
                    index: "test_index".to_string(),
                    id: Some("doc1".to_string()),
                    error: None,
                },
                BulkOperationResult {
                    success: false,
                    action: "create".to_string(),
                    index: "test_index".to_string(),
                    id: Some("doc2".to_string()),
                    error: Some("Document already exists".to_string()),
                },
                BulkOperationResult {
                    success: true,
                    action: "update".to_string(),
                    index: "test_index".to_string(),
                    id: Some("doc3".to_string()),
                    error: None,
                },
                BulkOperationResult {
                    success: false,
                    action: "delete".to_string(),
                    index: "nonexistent_index".to_string(),
                    id: Some("doc4".to_string()),
                    error: Some("Index not found".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: BulkResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.errors, deserialized.errors);
        assert_eq!(response.took_ms, deserialized.took_ms);
        assert_eq!(response.items.len(), deserialized.items.len());

        // Check that we have both successful and failed operations
        let success_count = response.items.iter().filter(|item| item.success).count();
        let failure_count = response.items.iter().filter(|item| !item.success).count();
        assert_eq!(success_count, 2);
        assert_eq!(failure_count, 2);
    }

    #[test]
    fn test_bulk_operation_with_empty_document() {
        let operation = BulkOperation::Index {
            index: "test_index".to_string(),
            id: None,
            document: serde_json::json!({}),
        };

        let json = serde_json::to_string(&operation).unwrap();
        let deserialized: BulkOperation = serde_json::from_str(&json).unwrap();

        assert!(matches!(deserialized, BulkOperation::Index { .. }));
    }

    #[test]
    fn test_bulk_operation_with_null_id() {
        let operation = BulkOperation::Index {
            index: "test_index".to_string(),
            id: None,
            document: serde_json::json!({"title": "Test"}),
        };

        let json = serde_json::to_string(&operation).unwrap();
        let deserialized: BulkOperation = serde_json::from_str(&json).unwrap();

        assert!(matches!(
            deserialized,
            BulkOperation::Index { id: None, .. }
        ));
    }

    #[test]
    fn test_bulk_operation_result_without_id() {
        let result = BulkOperationResult {
            success: true,
            action: "index".to_string(),
            index: "test_index".to_string(),
            id: None,
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: BulkOperationResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.id, deserialized.id);
        assert!(deserialized.id.is_none());
    }

    #[test]
    fn test_bulk_operation_result_without_error() {
        let result = BulkOperationResult {
            success: true,
            action: "index".to_string(),
            index: "test_index".to_string(),
            id: Some("doc123".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: BulkOperationResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.error, deserialized.error);
        assert!(deserialized.error.is_none());
    }

    #[test]
    fn test_add_document_response_with_uuid() {
        let response = AddDocumentResponse {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: AddDocumentResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.id, deserialized.id);
    }

    #[test]
    fn test_bulk_request_empty_operations() {
        let request = BulkRequest { operations: vec![] };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: BulkRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.operations.len(), 0);
        assert_eq!(deserialized.operations.len(), 0);
    }

    #[test]
    fn test_bulk_response_no_errors() {
        let response = BulkResponse {
            errors: false,
            took_ms: 50,
            items: vec![
                BulkOperationResult {
                    success: true,
                    action: "index".to_string(),
                    index: "test_index".to_string(),
                    id: Some("doc1".to_string()),
                    error: None,
                },
                BulkOperationResult {
                    success: true,
                    action: "index".to_string(),
                    index: "test_index".to_string(),
                    id: Some("doc2".to_string()),
                    error: None,
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: BulkResponse = serde_json::from_str(&json).unwrap();

        assert!(!response.errors);
        assert!(!deserialized.errors);
        assert_eq!(response.items.len(), 2);
    }
}
