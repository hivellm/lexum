//! Request size limiting middleware

use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tower::Layer;
use tower::Service;
use tracing::{debug, warn};

/// Request size limit configuration
#[derive(Clone, Debug)]
pub struct RequestSizeLimitConfig {
    /// Maximum request body size in bytes
    pub max_body_size: usize,
    /// Maximum header size in bytes
    pub max_header_size: usize,
    /// Maximum URL length
    pub max_url_length: usize,
}

impl Default for RequestSizeLimitConfig {
    fn default() -> Self {
        Self {
            max_body_size: 10 * 1024 * 1024, // 10MB default
            max_header_size: 8 * 1024,       // 8KB default
            max_url_length: 2048,            // 2KB default
        }
    }
}

/// Request size limiting layer
#[derive(Clone)]
pub struct RequestSizeLimitLayer {
    config: RequestSizeLimitConfig,
}

impl RequestSizeLimitLayer {
    /// Create new request size limiting layer
    pub fn new(config: RequestSizeLimitConfig) -> Self {
        Self { config }
    }

    /// Get configuration
    pub fn config(&self) -> &RequestSizeLimitConfig {
        &self.config
    }

    /// Check if request size is within limits
    fn check_request_size(&self, request: &Request) -> Result<(), SizeLimitError> {
        // Check URL length
        if request.uri().to_string().len() > self.config.max_url_length {
            warn!(
                url_length = request.uri().to_string().len(),
                max_url_length = self.config.max_url_length,
                "Request URL too long"
            );
            return Err(SizeLimitError::UrlTooLong);
        }

        // Check header size
        let total_header_size: usize = request
            .headers()
            .iter()
            .map(|(name, value)| name.as_str().len() + value.len())
            .sum();

        if total_header_size > self.config.max_header_size {
            warn!(
                header_size = total_header_size,
                max_header_size = self.config.max_header_size,
                "Request headers too large"
            );
            return Err(SizeLimitError::HeadersTooLarge);
        }

        // Body size is checked by reading Content-Length header
        if let Some(content_length) = request.headers().get("content-length") {
            if let Ok(length_str) = content_length.to_str() {
                if let Ok(length) = length_str.parse::<usize>() {
                    if length > self.config.max_body_size {
                        warn!(
                            body_size = length,
                            max_body_size = self.config.max_body_size,
                            "Request body too large"
                        );
                        return Err(SizeLimitError::BodyTooLarge);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Request size limit errors
#[derive(Debug, Clone, Copy)]
pub enum SizeLimitError {
    /// Request URL exceeds maximum length
    UrlTooLong,
    /// Request headers exceed maximum size
    HeadersTooLarge,
    /// Request body exceeds maximum size
    BodyTooLarge,
}

impl SizeLimitError {
    fn status_code(&self) -> StatusCode {
        match self {
            SizeLimitError::UrlTooLong => StatusCode::URI_TOO_LONG,
            SizeLimitError::HeadersTooLarge => StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
            SizeLimitError::BodyTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        }
    }

    fn message(&self) -> &'static str {
        match self {
            SizeLimitError::UrlTooLong => "Request URL exceeds maximum length",
            SizeLimitError::HeadersTooLarge => "Request headers exceed maximum size",
            SizeLimitError::BodyTooLarge => "Request body exceeds maximum size",
        }
    }
}

impl<S> Layer<S> for RequestSizeLimitLayer {
    type Service = RequestSizeLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestSizeLimitService {
            inner,
            layer: self.clone(),
        }
    }
}

/// Request size limiting service
#[derive(Clone)]
pub struct RequestSizeLimitService<S> {
    inner: S,
    layer: RequestSizeLimitLayer,
}

impl<S> Service<Request> for RequestSizeLimitService<S>
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

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        match self.layer.check_request_size(&request) {
            Ok(_) => {
                debug!("Request size within limits");
                let mut inner = self.inner.clone();
                Box::pin(async move {
                    let response = inner.call(request).await?;
                    let response: Response<axum::body::Body> = response.into_response();
                    Ok(response)
                })
            }
            Err(error) => {
                warn!(
                    error = ?error,
                    "Request size limit exceeded"
                );
                let status = error.status_code();
                let message = error.message();
                Box::pin(async move {
                    let response = (
                        status,
                        [("Content-Type", "application/json")],
                        format!(
                            r#"{{"error":{{"type":"size_limit_error","message":"{}","status":{}}}"#,
                            message,
                            status.as_u16()
                        ),
                    )
                        .into_response();
                    Ok(response)
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_request_size_limit_config_default() {
        let config = RequestSizeLimitConfig::default();
        assert_eq!(config.max_body_size, 10 * 1024 * 1024);
        assert_eq!(config.max_header_size, 8 * 1024);
        assert_eq!(config.max_url_length, 2048);
    }

    #[test]
    fn test_request_size_limit_config_custom() {
        let config = RequestSizeLimitConfig {
            max_body_size: 5 * 1024 * 1024,
            max_header_size: 4 * 1024,
            max_url_length: 1024,
        };
        assert_eq!(config.max_body_size, 5 * 1024 * 1024);
        assert_eq!(config.max_header_size, 4 * 1024);
        assert_eq!(config.max_url_length, 1024);
    }

    #[test]
    fn test_size_limit_error_status_codes() {
        assert_eq!(
            SizeLimitError::UrlTooLong.status_code(),
            StatusCode::URI_TOO_LONG
        );
        assert_eq!(
            SizeLimitError::HeadersTooLarge.status_code(),
            StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE
        );
        assert_eq!(
            SizeLimitError::BodyTooLarge.status_code(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[test]
    fn test_check_request_size_url_too_long() {
        let config = RequestSizeLimitConfig {
            max_url_length: 100,
            ..Default::default()
        };
        let layer = RequestSizeLimitLayer::new(config);
        let long_url = "http://localhost/".to_string() + &"a".repeat(200);
        let request = Request::builder()
            .uri(long_url)
            .body(axum::body::Body::empty())
            .unwrap();

        let result = layer.check_request_size(&request);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SizeLimitError::UrlTooLong));
    }

    #[test]
    fn test_check_request_size_within_limits() {
        let config = RequestSizeLimitConfig::default();
        let layer = RequestSizeLimitLayer::new(config);
        let request = Request::builder()
            .uri("http://localhost/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = layer.check_request_size(&request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_request_size_body_too_large() {
        let config = RequestSizeLimitConfig {
            max_body_size: 1000,
            ..Default::default()
        };
        let layer = RequestSizeLimitLayer::new(config);
        let mut request = Request::builder()
            .uri("http://localhost/test")
            .body(axum::body::Body::empty())
            .unwrap();
        request
            .headers_mut()
            .insert("content-length", HeaderValue::from_str("2000").unwrap());

        let result = layer.check_request_size(&request);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SizeLimitError::BodyTooLarge));
    }
}
