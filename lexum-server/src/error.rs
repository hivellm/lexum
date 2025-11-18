//! API error types

use axum::Json;
use axum::extract::rejection::JsonRejection;
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

    /// Template not found
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    /// Alias not found
    #[error("Alias not found: {0}")]
    AliasNotFound(String),

    /// Task not found
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Internal server error
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Core error
    #[error("Core error: {0}")]
    Core(#[from] lexum_core::Error),

    /// Validation error with field details
    #[error("Validation error: {0}")]
    Validation(String),

    /// Authentication error
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Authorization error
    #[error("Authorization failed: {0}")]
    Authorization(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Service unavailable
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Timeout error
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
}

impl ApiError {
    /// Get HTTP status code for error
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::IndexNotFound(_)
            | Self::DocumentNotFound(_)
            | Self::TemplateNotFound(_)
            | Self::AliasNotFound(_)
            | Self::TaskNotFound(_) => StatusCode::NOT_FOUND,
            Self::InvalidRequest(_) | Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Authentication(_) => StatusCode::UNAUTHORIZED,
            Self::Authorization(_) => StatusCode::FORBIDDEN,
            Self::RateLimitExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Timeout(_) => StatusCode::REQUEST_TIMEOUT,
            Self::Network(_) => StatusCode::BAD_GATEWAY,
            Self::Serialization(_) => StatusCode::BAD_REQUEST,
            Self::Configuration(_) => StatusCode::INTERNAL_SERVER_ERROR,
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

/// Convert JsonRejection to ApiError for better error messages
impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        let (error_message, details) = match &rejection {
            JsonRejection::JsonDataError(err) => {
                let err_str = err.to_string();
                // Try to extract line/column information from error message
                let details = extract_json_error_details(&err_str);
                (
                    format!("Invalid JSON format: {err}"),
                    details,
                )
            }
            JsonRejection::JsonSyntaxError(err) => {
                let err_str = err.to_string();
                // Try to extract line/column information from error message
                let details = extract_json_error_details(&err_str);
                (
                    format!("JSON syntax error: {err}"),
                    details,
                )
            }
            JsonRejection::MissingJsonContentType(_) => {
                (
                    "Missing Content-Type: application/json header".to_string(),
                    Some("Please ensure your request includes the header: Content-Type: application/json".to_string()),
                )
            }
            JsonRejection::BytesRejection(_) => {
                (
                    "Failed to read request body".to_string(),
                    Some("The request body could not be read. Check that the request is properly formatted.".to_string()),
                )
            }
            _ => {
                (
                    format!("JSON parsing error: {rejection}"),
                    Some("Check that your JSON payload is valid and properly formatted.".to_string()),
                )
            }
        };

        // Create error with details
        let mut api_error = ApiError::Serialization(error_message);
        // Store details in the error message for now (we'll enhance ErrorResponse later)
        if let Some(details) = details {
            if let ApiError::Serialization(ref mut msg) = api_error {
                *msg = format!("{msg}\nDetails: {details}");
            }
        }
        api_error
    }
}

/// Extract line/column information from JSON error messages
pub(crate) fn extract_json_error_details(error_msg: &str) -> Option<String> {
    // Look for common patterns in serde_json error messages
    // Example: "key must be a string at line 1 column 2"
    // Example: "invalid type: integer `123`, expected a string at line 2 column 5"

    let mut details = Vec::new();

    // Extract line number
    if let Some(line_pos) = error_msg.find("line ") {
        let line_part = &error_msg[line_pos..];
        if let Some(space_pos) = line_part.find(' ') {
            let after_line = &line_part[space_pos + 1..];
            if let Some(end_pos) = after_line.find(|c: char| !c.is_ascii_digit()) {
                if let Ok(line_num) = after_line[..end_pos].parse::<usize>() {
                    details.push(format!("Line: {line_num}"));
                }
            }
        }
    }

    // Extract column number
    if let Some(col_pos) = error_msg.find("column ") {
        let col_part = &error_msg[col_pos..];
        if let Some(space_pos) = col_part.find(' ') {
            let after_col = &col_part[space_pos + 1..];
            if let Some(end_pos) = after_col.find(|c: char| !c.is_ascii_digit() && c != ' ') {
                if let Ok(col_num) = after_col[..end_pos].trim().parse::<usize>() {
                    details.push(format!("Column: {col_num}"));
                }
            }
        }
    }

    // Extract field name if present
    if let Some(field_pos) = error_msg.find("field `") {
        let field_part = &error_msg[field_pos + 7..];
        if let Some(end_pos) = field_part.find('`') {
            let field_name = &field_part[..end_pos];
            details.push(format!("Field: {field_name}"));
        }
    }

    // Extract expected type if present
    if let Some(expected_pos) = error_msg.find("expected ") {
        let expected_part = &error_msg[expected_pos + 9..];
        if let Some(end_pos) = expected_part.find([' ', '`', '\n']) {
            let expected_type = expected_part[..end_pos].trim();
            if !expected_type.is_empty() {
                details.push(format!("Expected type: {expected_type}"));
            }
        }
    }

    if details.is_empty() {
        None
    } else {
        Some(details.join(", "))
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

    #[test]
    fn test_extract_json_error_details_with_line_column() {
        let error_msg = "key must be a string at line 1 column 2";
        let details = extract_json_error_details(error_msg);
        assert!(details.is_some());
        let details_str = details.unwrap();
        assert!(details_str.contains("Line: 1"));
        assert!(details_str.contains("Column: 2"));
    }

    #[test]
    fn test_extract_json_error_details_with_field() {
        let error_msg =
            "invalid type: integer `123`, expected a string at field `age` at line 2 column 5";
        let details = extract_json_error_details(error_msg);
        assert!(details.is_some());
        let details_str = details.unwrap();
        assert!(details_str.contains("Line: 2"));
        assert!(details_str.contains("Column: 5"));
        assert!(details_str.contains("Field: age"));
        assert!(details_str.contains("Expected type: string"));
    }

    #[test]
    fn test_extract_json_error_details_no_info() {
        let error_msg = "Some generic error";
        let details = extract_json_error_details(error_msg);
        assert!(details.is_none());
    }

    // Note: MissingJsonContentType is private, so we can't test it directly
    // The error handling is tested through integration tests
}
