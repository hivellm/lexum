//! Query complexity limiting middleware

use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::Value;
use tower::Layer;
use tower::Service;
use tracing::{debug, warn};

/// Query complexity limit configuration
#[derive(Clone, Debug)]
pub struct QueryComplexityLimitConfig {
    /// Maximum query depth (nested queries)
    pub max_depth: usize,
    /// Maximum number of clauses in bool query
    pub max_clauses: usize,
    /// Maximum number of terms in a query
    pub max_terms: usize,
    /// Maximum query string length
    pub max_query_length: usize,
    /// Whether to enable complexity checking
    pub enabled: bool,
}

impl Default for QueryComplexityLimitConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            max_clauses: 1024,
            max_terms: 10000,
            max_query_length: 10000,
            enabled: true,
        }
    }
}

/// Query complexity limiting layer
#[derive(Clone)]
pub struct QueryComplexityLimitLayer {
    config: QueryComplexityLimitConfig,
}

impl QueryComplexityLimitLayer {
    /// Create new query complexity limiting layer
    pub fn new(config: QueryComplexityLimitConfig) -> Self {
        Self { config }
    }

    /// Get configuration
    pub fn config(&self) -> &QueryComplexityLimitConfig {
        &self.config
    }

    /// Check query complexity from request body
    fn check_query_complexity(&self, request: &Request) -> Result<(), ComplexityError> {
        if !self.config.enabled {
            return Ok(());
        }

        // Try to extract query from request body
        // Note: This is a simplified check - in production you'd need to
        // actually parse the body, but for middleware we check headers/URI params
        if let Some(query) = request.uri().query() {
            if query.len() > self.config.max_query_length {
                warn!(
                    query_length = query.len(),
                    max_query_length = self.config.max_query_length,
                    "Query string too long"
                );
                return Err(ComplexityError::QueryTooLong);
            }
        }

        // Check for query parameters that might indicate complex queries
        if let Some(q) = request.uri().query() {
            // Count number of parameters as a proxy for complexity
            let param_count = q.split('&').count();
            if param_count > 100 {
                warn!(param_count, "Query has too many parameters");
                return Err(ComplexityError::TooManyParameters);
            }
        }

        Ok(())
    }

    /// Analyze query JSON for complexity
    pub fn analyze_query_json(&self, query_json: &Value) -> Result<(), ComplexityError> {
        if !self.config.enabled {
            return Ok(());
        }

        let depth = self.calculate_depth(query_json, 0);
        if depth > self.config.max_depth {
            warn!(
                depth,
                max_depth = self.config.max_depth,
                "Query depth exceeds maximum"
            );
            return Err(ComplexityError::QueryTooDeep);
        }

        let clauses = self.count_clauses(query_json);
        if clauses > self.config.max_clauses {
            warn!(
                clauses,
                max_clauses = self.config.max_clauses,
                "Query has too many clauses"
            );
            return Err(ComplexityError::TooManyClauses);
        }

        let terms = self.count_terms(query_json);
        if terms > self.config.max_terms {
            warn!(
                terms,
                max_terms = self.config.max_terms,
                "Query has too many terms"
            );
            return Err(ComplexityError::TooManyTerms);
        }

        Ok(())
    }

    fn calculate_depth(&self, value: &Value, current_depth: usize) -> usize {
        match value {
            Value::Object(map) => {
                let mut max_depth = current_depth;
                for v in map.values() {
                    let depth = self.calculate_depth(v, current_depth + 1);
                    max_depth = max_depth.max(depth);
                }
                max_depth
            }
            Value::Array(arr) => {
                let mut max_depth = current_depth;
                for v in arr {
                    let depth = self.calculate_depth(v, current_depth + 1);
                    max_depth = max_depth.max(depth);
                }
                max_depth
            }
            _ => current_depth,
        }
    }

    fn count_clauses(&self, value: &Value) -> usize {
        match value {
            Value::Object(map) => {
                let mut count = 0;
                // Check for bool query structure
                if map.contains_key("bool") {
                    if let Some(bool_obj) = map.get("bool").and_then(|v| v.as_object()) {
                        for key in ["must", "should", "must_not", "filter"] {
                            if let Some(arr) = bool_obj.get(key).and_then(|v| v.as_array()) {
                                count += arr.len();
                                for clause in arr {
                                    count += self.count_clauses(clause);
                                }
                            }
                        }
                    }
                } else {
                    // Count other query types
                    count += 1;
                    for v in map.values() {
                        count += self.count_clauses(v);
                    }
                }
                count
            }
            Value::Array(arr) => {
                let mut count = arr.len();
                for v in arr {
                    count += self.count_clauses(v);
                }
                count
            }
            _ => 0,
        }
    }

    fn count_terms(&self, value: &Value) -> usize {
        match value {
            Value::Object(map) => {
                let mut count = 0;
                // Check for term queries
                if map.contains_key("term") || map.contains_key("terms") {
                    if let Some(terms_obj) = map.get("terms").and_then(|v| v.as_object()) {
                        for v in terms_obj.values() {
                            if let Some(arr) = v.as_array() {
                                count += arr.len();
                            } else {
                                count += 1;
                            }
                        }
                    } else {
                        count += 1;
                    }
                }
                for v in map.values() {
                    count += self.count_terms(v);
                }
                count
            }
            Value::Array(arr) => {
                let mut count = 0;
                for v in arr {
                    count += self.count_terms(v);
                }
                count
            }
            _ => 0,
        }
    }
}

/// Query complexity errors
#[derive(Debug, Clone, Copy)]
pub enum ComplexityError {
    /// Query string exceeds maximum length
    QueryTooLong,
    /// Query depth exceeds maximum allowed
    QueryTooDeep,
    /// Query has too many clauses
    TooManyClauses,
    /// Query has too many terms
    TooManyTerms,
    /// Query has too many parameters
    TooManyParameters,
}

impl ComplexityError {
    /// Get HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    /// Get human-readable error message
    pub fn message(&self) -> &'static str {
        match self {
            ComplexityError::QueryTooLong => "Query string exceeds maximum length",
            ComplexityError::QueryTooDeep => "Query depth exceeds maximum allowed",
            ComplexityError::TooManyClauses => "Query has too many clauses",
            ComplexityError::TooManyTerms => "Query has too many terms",
            ComplexityError::TooManyParameters => "Query has too many parameters",
        }
    }
}

impl<S> Layer<S> for QueryComplexityLimitLayer {
    type Service = QueryComplexityLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        QueryComplexityLimitService {
            inner,
            layer: self.clone(),
        }
    }
}

/// Query complexity limiting service
#[derive(Clone)]
pub struct QueryComplexityLimitService<S> {
    inner: S,
    layer: QueryComplexityLimitLayer,
}

impl<S> Service<Request> for QueryComplexityLimitService<S>
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
        match self.layer.check_query_complexity(&request) {
            Ok(_) => {
                debug!("Query complexity within limits");
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
                    "Query complexity limit exceeded"
                );
                let status = error.status_code();
                let message = error.message();
                Box::pin(async move {
                    let response = (
                        status,
                        [("Content-Type", "application/json")],
                        format!(
                            r#"{{"error":{{"type":"complexity_error","message":"{}","status":{}}}"#,
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
    use serde_json::json;

    #[test]
    fn test_query_complexity_limit_config_default() {
        let config = QueryComplexityLimitConfig::default();
        assert_eq!(config.max_depth, 10);
        assert_eq!(config.max_clauses, 1024);
        assert_eq!(config.max_terms, 10000);
        assert_eq!(config.max_query_length, 10000);
        assert!(config.enabled);
    }

    #[test]
    fn test_query_complexity_limit_config_custom() {
        let config = QueryComplexityLimitConfig {
            max_depth: 5,
            max_clauses: 512,
            max_terms: 5000,
            max_query_length: 5000,
            enabled: false,
        };
        assert_eq!(config.max_depth, 5);
        assert!(!config.enabled);
    }

    #[test]
    fn test_analyze_query_json_simple() {
        let config = QueryComplexityLimitConfig::default();
        let layer = QueryComplexityLimitLayer::new(config);
        let query = json!({
            "match": {
                "title": "test"
            }
        });

        let result = layer.analyze_query_json(&query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_query_json_too_deep() {
        let config = QueryComplexityLimitConfig {
            max_depth: 3,
            ..Default::default()
        };
        let layer = QueryComplexityLimitLayer::new(config);
        // Create a deeply nested query
        let mut query = json!({"nested": {}});
        for _ in 0..10 {
            query = json!({"nested": query});
        }

        let result = layer.analyze_query_json(&query);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ComplexityError::QueryTooDeep));
    }

    #[test]
    fn test_count_clauses_bool_query() {
        let config = QueryComplexityLimitConfig::default();
        let layer = QueryComplexityLimitLayer::new(config);
        let query = json!({
            "bool": {
                "must": [
                    {"match": {"title": "test"}},
                    {"match": {"content": "test"}}
                ],
                "should": [
                    {"term": {"status": "active"}}
                ]
            }
        });

        let clauses = layer.count_clauses(&query);
        assert!(clauses >= 3); // At least 3 clauses
    }

    #[test]
    fn test_check_query_complexity_disabled() {
        let config = QueryComplexityLimitConfig {
            enabled: false,
            ..Default::default()
        };
        let layer = QueryComplexityLimitLayer::new(config);
        let request = Request::builder()
            .uri("http://localhost/test?q=".to_string() + &"a".repeat(20000))
            .body(axum::body::Body::empty())
            .unwrap();

        let result = layer.check_query_complexity(&request);
        assert!(result.is_ok()); // Should pass when disabled
    }
}
