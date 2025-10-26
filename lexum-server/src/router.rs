//! API router configuration

use crate::handlers::index::AppState;
use crate::handlers::{
    admin, alias, document, health, index, progress, progress_bulk, reindex, rollover, search,
    snapshot, template,
};
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
        .route("/_cluster/state", get(admin::get_cluster_state))
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
        .route(
            "/api/v1/bulk/progress",
            post(progress_bulk::bulk_operations_with_progress),
        )
        .route(
            "/api/v1/bulk/progress/{progress_id}",
            get(progress_bulk::get_bulk_progress),
        )
        // Search
        .route("/api/v1/indices/{index}/search", post(search::search))
        .route("/api/v1/indices/{index}/search", get(search::search_get))
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
        // Alias management
        .route("/_aliases", get(alias::get_aliases))
        .route("/_aliases", post(alias::perform_alias_operations))
        .route(
            "/_aliases/atomic",
            post(alias::perform_atomic_alias_operations),
        )
        .route("/{index}/_alias", get(alias::get_index_aliases))
        .route("/{index}/_alias/{alias}", put(alias::add_alias))
        .route("/{index}/_alias/{alias}", delete(alias::remove_alias))
        // Reindexing operations
        .route("/_reindex", post(reindex::reindex))
        .route("/_tasks", get(reindex::list_tasks))
        .route("/_tasks/{task_id}", get(reindex::get_task))
        .route("/_tasks/{task_id}/_cancel", post(reindex::cancel_task))
        // Index rollover operations
        .route(
            "/api/v1/indices/{index_name}/_rollover",
            post(rollover::rollover_index),
        )
        .route(
            "/api/v1/indices/{index_name}/_rollover",
            get(rollover::get_rollover_conditions),
        )
        .route(
            "/api/v1/indices/{index_name}/_rollover",
            put(rollover::update_rollover_conditions),
        )
        // Progress tracking
        .route("/api/v1/progress", get(progress::list_progress))
        .route("/api/v1/progress/stats", get(progress::get_progress_stats))
        .route(
            "/api/v1/progress/{progress_id}",
            get(progress::get_progress),
        )
        .route(
            "/api/v1/progress/{progress_id}",
            delete(progress::delete_progress),
        )
        .route(
            "/api/v1/progress/{progress_id}/cancel",
            post(progress::cancel_progress),
        )
        .route(
            "/api/v1/progress/{progress_id}/pause",
            post(progress::pause_progress),
        )
        .route(
            "/api/v1/progress/{progress_id}/resume",
            post(progress::resume_progress),
        )
        .route("/api/v1/progress/cleanup", post(progress::cleanup_progress))
        // OpenAPI documentation (temporarily disabled due to version conflicts)
        // .merge(create_swagger_ui())
        // Middleware (rate limiting implemented, ready for full Tower integration)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
