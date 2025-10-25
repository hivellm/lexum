//! API router configuration

use crate::handlers::index::AppState;
use crate::handlers::{document, health, index, search};
use axum::Router;
use axum::routing::{delete, get, post, put};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Build application router
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health::health_check))
        // Index management
        .route("/api/v1/indices", post(index::create_index))
        .route("/api/v1/indices", get(index::list_indices))
        .route("/api/v1/indices/{name}", get(index::get_index))
        .route("/api/v1/indices/{name}", delete(index::delete_index))
        // Document operations
        .route(
            "/api/v1/indices/{index}/documents",
            post(document::add_document),
        )
        .route(
            "/api/v1/indices/{index}/documents/{id}",
            get(document::get_document),
        )
        .route(
            "/api/v1/indices/{index}/documents/{id}",
            put(document::update_document),
        )
        .route(
            "/api/v1/indices/{index}/documents/{id}",
            delete(document::delete_document),
        )
        // Bulk operations
        .route("/api/v1/bulk", post(document::bulk_operations))
        // Search
        .route("/api/v1/indices/{index}/search", post(search::search))
        // Middleware (rate limiting implemented, ready for full Tower integration)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
