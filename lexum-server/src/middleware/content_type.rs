//! Content-Type validation middleware
//!
//! This middleware validates that requests with JSON bodies have the correct
//! Content-Type header set to "application/json" or "application/json; charset=utf-8".

use axum::Json;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::task::{Context, Poll};
use tower::Service;

/// Configuration for Content-Type validation
#[derive(Debug, Clone)]
pub struct ContentTypeValidationConfig {
    /// Whether to enforce Content-Type validation
    pub enabled: bool,
    /// Allowed Content-Type values (default: ["application/json", "application/json; charset=utf-8"])
    pub allowed_types: Vec<String>,
}

impl Default for ContentTypeValidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_types: vec![
                "application/json".to_string(),
                "application/json; charset=utf-8".to_string(),
            ],
        }
    }
}

/// Error response for Content-Type validation failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTypeErrorResponse {
    /// Error message
    pub error: String,
    /// Details about the error
    pub details: Option<String>,
}

impl IntoResponse for ContentTypeErrorResponse {
    fn into_response(self) -> Response {
        let body = Json(self);
        (StatusCode::BAD_REQUEST, body).into_response()
    }
}

/// Content-Type validation layer
#[derive(Debug, Clone)]
pub struct ContentTypeValidationLayer {
    config: ContentTypeValidationConfig,
}

impl ContentTypeValidationLayer {
    /// Create a new Content-Type validation layer
    pub fn new(config: ContentTypeValidationConfig) -> Self {
        Self { config }
    }

    /// Create a new Content-Type validation layer with default config
    pub fn default() -> Self {
        Self::new(ContentTypeValidationConfig::default())
    }
}

impl<S> tower::Layer<S> for ContentTypeValidationLayer {
    type Service = ContentTypeValidationService<S>;

    fn layer(&self, service: S) -> Self::Service {
        ContentTypeValidationService {
            inner: service,
            config: self.config.clone(),
        }
    }
}

/// Content-Type validation service
#[derive(Debug, Clone)]
pub struct ContentTypeValidationService<S> {
    inner: S,
    config: ContentTypeValidationConfig,
}

impl<S> Service<Request> for ContentTypeValidationService<S>
where
    S: Service<Request> + Clone + Send + 'static,
    S::Response: IntoResponse,
    S::Future: Send + 'static,
{
    type Response = Response<axum::body::Body>;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        // Skip validation if disabled
        if !self.config.enabled {
            let mut inner = self.inner.clone();
            return Box::pin(async move {
                let response = inner.call(req).await?;
                let response: Response<axum::body::Body> = response.into_response();
                Ok(response)
            });
        }

        // Only validate POST, PUT, PATCH requests (methods that typically have bodies)
        let method = req.method();
        let should_validate = matches!(
            method.as_str(),
            "POST" | "PUT" | "PATCH" | "DELETE" // DELETE can have body in some APIs
        );

        if !should_validate {
            let mut inner = self.inner.clone();
            return Box::pin(async move {
                let response = inner.call(req).await?;
                let response: Response<axum::body::Body> = response.into_response();
                Ok(response)
            });
        }

        // Check if request has a body (Content-Length > 0 or Transfer-Encoding: chunked)
        let has_body = req
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok())
            .map(|len| len > 0)
            .unwrap_or_else(|| {
                // Check for chunked encoding
                req.headers()
                    .get("transfer-encoding")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.contains("chunked"))
                    .unwrap_or(false)
            });

        // If no body, skip validation
        if !has_body {
            let mut inner = self.inner.clone();
            return Box::pin(async move {
                let response = inner.call(req).await?;
                let response: Response<axum::body::Body> = response.into_response();
                Ok(response)
            });
        }

        // Validate Content-Type header
        let content_type = req.headers().get("content-type");

        if content_type.is_none() {
            let error_response = ContentTypeErrorResponse {
                error: "Missing Content-Type header".to_string(),
                details: Some(
                    "Please include the header: Content-Type: application/json".to_string(),
                ),
            };
            return Box::pin(async move {
                let response: Response<axum::body::Body> = error_response.into_response();
                Ok(response)
            });
        }

        let content_type_str = content_type
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        // Check if Content-Type is allowed
        let is_valid = self
            .config
            .allowed_types
            .iter()
            .any(|allowed| content_type_str.starts_with(allowed));

        if !is_valid {
            let error_response = ContentTypeErrorResponse {
                error: format!(
                    "Invalid Content-Type: '{content_type_str}'. Expected: application/json"
                ),
                details: Some(format!(
                    "Please set Content-Type to one of: {}",
                    self.config
                        .allowed_types
                        .iter()
                        .map(|s| format!("'{s}'"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            };
            return Box::pin(async move {
                let response: Response<axum::body::Body> = error_response.into_response();
                Ok(response)
            });
        }

        // Content-Type is valid, proceed with request
        let mut inner = self.inner.clone();
        Box::pin(async move {
            let response = inner.call(req).await?;
            let response: Response<axum::body::Body> = response.into_response();
            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::Method;

    #[tokio::test]
    async fn test_content_type_validation_valid() {
        let config = ContentTypeValidationConfig::default();
        let layer = ContentTypeValidationLayer::new(config);

        // Create a mock service that just returns OK
        let service = tower::service_fn(|_req: Request| async {
            Ok::<_, axum::Error>(
                axum::response::Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap()
                    .into_response(),
            )
        });

        let mut service = layer.layer(service);

        // Create request with valid Content-Type
        let req = Request::builder()
            .method(Method::POST)
            .header("content-type", "application/json")
            .header("content-length", "10")
            .body(Body::from("{\"test\":1}"))
            .unwrap();

        // Should pass validation
        let result = service.ready().await.unwrap().call(req).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_content_type_validation_missing() {
        let config = ContentTypeValidationConfig::default();
        let layer = ContentTypeValidationLayer::new(config);

        let service = tower::service_fn(|_req: Request| async {
            Ok::<_, axum::Error>(
                axum::response::Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap()
                    .into_response(),
            )
        });

        let mut service = layer.layer(service);

        // Create request without Content-Type
        let req = Request::builder()
            .method(Method::POST)
            .header("content-length", "10")
            .body(Body::from("{\"test\":1}"))
            .unwrap();

        // Should fail validation with 400 Bad Request
        let result = service.ready().await.unwrap().call(req).await;
        assert!(result.is_ok()); // Returns Ok with error response
        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_content_type_validation_invalid() {
        let config = ContentTypeValidationConfig::default();
        let layer = ContentTypeValidationLayer::new(config);

        let service = tower::service_fn(|_req: Request| async {
            Ok::<_, axum::Error>(
                axum::response::Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap()
                    .into_response(),
            )
        });

        let mut service = layer.layer(service);

        // Create request with invalid Content-Type
        let req = Request::builder()
            .method(Method::POST)
            .header("content-type", "text/plain")
            .header("content-length", "10")
            .body(Body::from("test data"))
            .unwrap();

        // Should fail validation with 400 Bad Request
        let result = service.ready().await.unwrap().call(req).await;
        assert!(result.is_ok()); // Returns Ok with error response
        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_content_type_validation_get_skipped() {
        let config = ContentTypeValidationConfig::default();
        let layer = ContentTypeValidationLayer::new(config);

        let service = tower::service_fn(|_req: Request| async {
            Ok::<_, axum::Error>(
                axum::response::Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap()
                    .into_response(),
            )
        });

        let mut service = layer.layer(service);

        // GET requests should skip validation
        let req = Request::builder()
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();

        // Should pass (validation skipped for GET)
        let result = service.ready().await.unwrap().call(req).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_content_type_validation_no_body_skipped() {
        let config = ContentTypeValidationConfig::default();
        let layer = ContentTypeValidationLayer::new(config);

        let service = tower::service_fn(|_req: Request| async {
            Ok::<_, axum::Error>(
                axum::response::Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap()
                    .into_response(),
            )
        });

        let mut service = layer.layer(service);

        // POST request without body should skip validation
        let req = Request::builder()
            .method(Method::POST)
            .header("content-length", "0")
            .body(Body::empty())
            .unwrap();

        // Should pass (validation skipped for empty body)
        let result = service.ready().await.unwrap().call(req).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
