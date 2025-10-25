//! API error types

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

/// API result type
pub type ApiResult<T> = std::result::Result<T, ApiError>;

/// API error response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
    /// Error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Validation error details
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ValidationError {
    /// Field that failed validation
    pub field: String,
    /// Validation error message
    pub message: String,
}

/// API error types
#[derive(Debug, thiserror::Error, utoipa::ToSchema)]
pub enum ApiError {
    /// Index not found
    #[error("Index not found: {0}")]
    IndexNotFound(String),

    /// Document not found
    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Internal server error
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Core error
    #[error("Core error: {0}")]
    Core(#[from] lexum_core::Error),
}

impl ApiError {
    /// Get HTTP status code for error
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::IndexNotFound(_) | Self::DocumentNotFound(_) => StatusCode::NOT_FOUND,
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) | Self::Core(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Convert to error response
    pub fn to_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: self.to_string(),
            details: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(self.to_response());

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            ApiError::IndexNotFound("test".to_string()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ApiError::InvalidRequest("test".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::Internal("test".to_string()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
