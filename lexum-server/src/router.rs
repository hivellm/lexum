//! API router configuration

use crate::handlers::index::AppState;
use crate::handlers::{
    admin, alias, auth, batch, bottleneck, document, health, index, mapping, metrics, profiling,
    progress, progress_bulk, query_ops, reindex, rollover, scroll, search, snapshot, suggest,
    template,
};
use crate::middleware::http2_push::{Http2PushConfig, Http2PushLayer};
use crate::middleware::ip_filter::{IpFilterConfig, IpFilterLayer};
use crate::middleware::query_complexity::{QueryComplexityLimitConfig, QueryComplexityLimitLayer};
use crate::middleware::rate_limit::{RateLimitConfig, RateLimitLayer};
use crate::middleware::request_size::{RequestSizeLimitConfig, RequestSizeLimitLayer};
use crate::protocols::{mcp, streamable_http, umicp};
use axum::Router;
use axum::routing::{delete, get, post, put};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Build application router with MCP as main server
pub fn build_router(state: AppState, http2_push_config: &Http2PushConfig) -> Router {
    let metrics_state = state.metrics.clone();
    let state_arc = Arc::new(state);

    // Create MCP router (main server) using StreamableHTTP transport
    let mcp_router = create_mcp_router(state_arc.clone());

    // Create REST API router
    let rest_router = Router::new()
        // Health check
        .route("/health", get(health::health_check))
        .route("/_ready", get(health::readiness_check))
        // Metrics endpoint
        .route(
            "/_metrics",
            get(metrics::metrics_handler).with_state(metrics_state),
        )
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
        .route("/api/v1/indices/{name}/stats", get(index::get_index_stats))
        .route("/api/v1/indices/{name}/refresh", post(index::refresh_index))
        .route("/api/v1/indices/{name}/flush", post(index::flush_index))
        // Mapping endpoints
        .route(
            "/api/v1/indices/{index}/_mapping",
            get(mapping::get_mapping),
        )
        .route(
            "/api/v1/indices/{index}/_mapping",
            put(mapping::update_mapping),
        )
        .route(
            "/api/v1/indices/{index}/_mapping/{field}",
            get(mapping::get_field_mapping),
        )
        .route("/api/v1/_mapping", get(mapping::get_all_mappings))
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
        // Batch requests
        .route("/api/v1/_batch", post(batch::batch_requests))
        .route(
            "/api/v1/bulk/progress/{progress_id}",
            get(progress_bulk::get_bulk_progress),
        )
        // Search
        .route("/api/v1/indices/{index}/search", post(search::search))
        .route("/api/v1/indices/{index}/search", get(search::search_get))
        // StreamableHTTP streaming search
        .route(
            "/api/v1/indices/{index}/_search/stream",
            post(streamable_http::stream_search),
        )
        // Scroll API
        .route(
            "/api/v1/indices/{index}/_search/scroll",
            post(scroll::create_scroll),
        )
        .route("/api/v1/_search/scroll", post(scroll::scroll))
        .route(
            "/api/v1/_search/scroll/{scroll_id}",
            delete(scroll::clear_scroll),
        )
        .route(
            "/api/v1/_search/scroll/_all",
            delete(scroll::clear_all_scrolls),
        )
        // Query-based operations
        .route(
            "/api/v1/indices/{index}/_update_by_query",
            post(query_ops::update_by_query),
        )
        .route(
            "/api/v1/indices/{index}/_delete_by_query",
            post(query_ops::delete_by_query),
        )
        // Multi-Get and Multi-Search
        .route("/api/v1/_mget", post(query_ops::multi_get))
        .route("/api/v1/_msearch", post(query_ops::multi_search))
        .route(
            "/api/v1/indices/{index}/_explain/{id}",
            get(search::explain),
        )
        // Search suggestions
        .route("/api/v1/indices/{index}/_suggest", get(suggest::suggest))
        .route(
            "/api/v1/indices/{index}/_suggest",
            post(suggest::suggest_post),
        )
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
        // Authentication endpoints
        .route("/api/v1/auth/keys", post(auth::generate_api_key))
        .route("/api/v1/auth/keys", get(auth::list_api_keys))
        .route("/api/v1/auth/keys", delete(auth::revoke_api_key))
        // Profiling endpoints
        .route(
            "/_profiling/start",
            axum::routing::post(profiling::start_profiling),
        )
        .route(
            "/_profiling/stop",
            axum::routing::post(profiling::stop_profiling),
        )
        .route(
            "/_profiling/status",
            axum::routing::get(profiling::get_profiling_status),
        )
        .route(
            "/_profiling/flamegraph",
            axum::routing::post(profiling::generate_flamegraph),
        )
        .route(
            "/_profiling/instructions",
            axum::routing::get(profiling::get_profiling_instructions),
        )
        .route(
            "/_profiling/bottlenecks",
            axum::routing::post(bottleneck::analyze_bottlenecks),
        )
        // OpenAPI documentation (temporarily disabled due to version conflicts)
        // .merge(create_swagger_ui())
        // Middleware (security layers)
        .layer(
            ServiceBuilder::new()
                .layer(IpFilterLayer::new(IpFilterConfig::default()))
                .layer(RateLimitLayer::new(RateLimitConfig::default()))
                .layer(RequestSizeLimitLayer::new(RequestSizeLimitConfig::default()))
                .layer(QueryComplexityLimitLayer::new(
                    QueryComplexityLimitConfig::default(),
                )),
        )
        .layer(CorsLayer::permissive())
        // HTTP/2 push hints layer - adds Link headers for resource preloading
        .layer(Http2PushLayer::new(http2_push_config.clone()))
        // Compression layer - compresses response bodies (gzip, deflate, br)
        .layer(CompressionLayer::new())
        .layer(
            TraceLayer::new_for_http()
                .on_request(|request: &axum::http::Request<_>, span: &tracing::Span| {
                    let method = request.method();
                    let uri = request.uri();
                    tracing::info!(method = %method, uri = %uri, "Incoming request");
                    span.record("http.method", method.as_str());
                    span.record("http.uri", uri.to_string());
                })
                .on_failure(
                    |error: ServerErrorsFailureClass,
                     latency: std::time::Duration,
                     _span: &tracing::Span| {
                        tracing::error!(
                            latency_ms = latency.as_millis(),
                            error = ?error,
                            "Request failed with 500 error - check logs above for request details"
                        );
                    },
                ),
        )
        .with_state(state_arc.as_ref().clone());

    // Create UMICP router
    let umicp_router = Router::new()
        .route("/umicp", post(umicp::umicp_handler))
        .with_state(state_arc.as_ref().clone());

    // Merge all routers - MCP as main server, then REST, then UMICP
    Router::new()
        .merge(mcp_router)
        .merge(rest_router)
        .merge(umicp_router)
}

/// Create MCP router with StreamableHTTP transport
fn create_mcp_router(state: Arc<AppState>) -> Router {
    use hyper::service::Service;
    use hyper_util::service::TowerToHyperService;
    use rmcp::transport::streamable_http_server::StreamableHttpService;
    use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;

    let state_clone = state.clone();

    // Create StreamableHTTP service
    let streamable_service = StreamableHttpService::new(
        move || {
            Ok(mcp::LexumMcpService {
                state: state_clone.clone(),
            })
        },
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // Convert to axum-compatible service
    let hyper_service = TowerToHyperService::new(streamable_service);

    // Create router with the MCP endpoint
    Router::new().route(
        "/mcp",
        axum::routing::any(move |req: axum::extract::Request| {
            let service = hyper_service.clone();
            async move {
                // Forward request to hyper service
                match service.call(req).await {
                    Ok(response) => Ok(response),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            }
        }),
    )
}
