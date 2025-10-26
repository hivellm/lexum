//! API router configuration

use crate::handlers::index::AppState;
use crate::handlers::{admin, document, health, index, search, snapshot, template};
use axum::Router;
use axum::routing::{delete, get, post, put};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Build application router
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health::health_check))
        // Cluster endpoints
        .route("/", get(admin::get_cluster_info))
        .route("/_cluster/health", get(admin::get_cluster_health))
        .route("/_cluster/stats", get(admin::get_cluster_stats))
        .route("/_nodes/stats", get(admin::get_node_stats))
        .route("/_cluster/settings", get(admin::get_cluster_settings))
        .route("/_cluster/settings", put(admin::update_cluster_settings))
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
        // Snapshot repositories
        .route(
            "/_snapshot/{repository}",
            put(snapshot::create_or_update_repository),
        )
        .route("/_snapshot/{repository}", get(snapshot::get_repository))
        .route("/_snapshot", get(snapshot::list_repositories))
        // Snapshots
        .route(
            "/_snapshot/{repository}/{snapshot}",
            put(snapshot::create_snapshot),
        )
        .route(
            "/_snapshot/{repository}/{snapshot}",
            get(snapshot::get_snapshot),
        )
        .route(
            "/_snapshot/{repository}/{snapshot}",
            delete(snapshot::delete_snapshot),
        )
        .route(
            "/_snapshot/{repository}/_all",
            get(snapshot::list_snapshots),
        )
        // Snapshot restore
        .route(
            "/_snapshot/{repository}/{snapshot}/_restore",
            post(snapshot::restore_snapshot),
        )
        // Snapshot statistics
        .route(
            "/_snapshot/{repository}/_stats",
            get(snapshot::get_snapshot_stats),
        )
        .route(
            "/_snapshot/_stats",
            get(snapshot::get_global_snapshot_stats),
        )
        // Template management
        .route("/_template", get(template::list_templates))
        .route("/_template/{name}", put(template::put_template))
        .route("/_template/{name}", get(template::get_template))
        .route("/_template/{name}", delete(template::delete_template))
        // OpenAPI documentation (temporarily disabled due to version conflicts)
        // .merge(create_swagger_ui())
        // Middleware (rate limiting implemented, ready for full Tower integration)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
