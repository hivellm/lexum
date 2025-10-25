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
