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
            ApiError::DocumentNotFound("test".to_string()).status_code(),
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
        assert_eq!(
            ApiError::Core(lexum_core::Error::Config("test".to_string())).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_error_response_creation() {
        let error = ApiError::IndexNotFound("test_index".to_string());
        let response = error.to_response();

        assert_eq!(response.error, "Index not found: test_index");
        assert_eq!(response.details, None);
    }

    #[test]
    fn test_error_response_serialization() {
        let response = ErrorResponse {
            error: "Test error".to_string(),
            details: Some("Test details".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Test error"));
        assert!(json.contains("Test details"));
    }

    #[test]
    fn test_validation_error() {
        let validation_error = ValidationError {
            field: "name".to_string(),
            message: "Name is required".to_string(),
        };

        assert_eq!(validation_error.field, "name");
        assert_eq!(validation_error.message, "Name is required");
    }

    #[test]
    fn test_api_error_display() {
        let index_error = ApiError::IndexNotFound("test".to_string());
        assert_eq!(index_error.to_string(), "Index not found: test");

        let doc_error = ApiError::DocumentNotFound("doc123".to_string());
        assert_eq!(doc_error.to_string(), "Document not found: doc123");

        let invalid_error = ApiError::InvalidRequest("Invalid data".to_string());
        assert_eq!(invalid_error.to_string(), "Invalid request: Invalid data");

        let internal_error = ApiError::Internal("Database error".to_string());
        assert_eq!(
            internal_error.to_string(),
            "Internal server error: Database error"
        );
    }

    #[test]
    fn test_core_error_conversion() {
        let core_error = lexum_core::Error::Config("Configuration error".to_string());
        let api_error: ApiError = core_error.into();

        match api_error {
            ApiError::Core(_) => (),
            _ => panic!("Expected Core error variant"),
        }
    }

    #[test]
    fn test_into_response() {
        let error = ApiError::IndexNotFound("test".to_string());
        let response = error.into_response();

        // The response should be a valid HTTP response
        assert!(response.status().is_client_error());
    }

    #[test]
    fn test_api_result_type() {
        fn success_function() -> String {
            "success".to_string()
        }

        fn error_function() -> ApiResult<String> {
            Err(ApiError::InvalidRequest("test".to_string()))
        }

        assert_eq!(success_function(), "success");
        assert!(error_function().is_err());
    }
}
