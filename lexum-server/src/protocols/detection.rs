//! Protocol detection middleware
//! Detects protocol from request headers and routes to appropriate handler

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

/// Protocol type detected from headers
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub enum Protocol {
    Rest,
    StreamableHttp,
    Mcp,
    Umicp,
}

/// Detect protocol from request headers
pub fn detect_protocol(request: &Request) -> Protocol {
    let headers = request.headers();

    // Check for UMICP protocol header
    if headers.contains_key("x-umicp-protocol") {
        return Protocol::Umicp;
    }

    // Check for MCP protocol header
    if headers.contains_key("x-mcp-protocol") || request.uri().path().starts_with("/mcp") {
        return Protocol::Mcp;
    }

    // Check for StreamableHTTP header or stream endpoint
    if headers.contains_key("x-streamable-http")
        || request.uri().path().ends_with("/_search/stream")
    {
        return Protocol::StreamableHttp;
    }

    // Default to REST
    Protocol::Rest
}

/// Protocol detection middleware
pub async fn protocol_detection_middleware(request: Request, next: Next) -> Response {
    let _protocol = detect_protocol(&request);

    // Store protocol in request extensions for downstream handlers
    // For now, just pass through - routing is handled by router
    next.run(request).await
}
