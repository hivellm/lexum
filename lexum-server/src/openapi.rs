//! OpenAPI specification for Lexum REST API

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// OpenAPI specification
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::health::health_check,
        // Temporarily disable other paths to focus on snapshot functionality
        // crate::handlers::index::create_index,
        // crate::handlers::index::list_indices,
        // crate::handlers::index::get_index,
        // crate::handlers::index::delete_index,
        // crate::handlers::document::add_document,
        // crate::handlers::document::get_document,
        // crate::handlers::document::update_document,
        // crate::handlers::document::delete_document,
        // crate::handlers::document::bulk_operations,
        // crate::handlers::search::search,
        crate::handlers::snapshot::create_or_update_repository,
        crate::handlers::snapshot::get_repository,
        crate::handlers::snapshot::list_repositories,
        crate::handlers::snapshot::create_snapshot,
        crate::handlers::snapshot::get_snapshot,
        crate::handlers::snapshot::delete_snapshot,
        crate::handlers::snapshot::list_snapshots,
        crate::handlers::snapshot::restore_snapshot,
        crate::handlers::snapshot::get_snapshot_stats,
        crate::handlers::snapshot::get_global_snapshot_stats,
    ),
    components(
        schemas(
            // Temporarily disable other schemas to focus on snapshot functionality
            // crate::handlers::index::FieldDefinition,
            // crate::handlers::index::CreateIndexRequest,
            // crate::handlers::index::IndexInfo,
            // crate::handlers::index::ListIndicesResponse,
            // crate::handlers::document::AddDocumentRequest,
            // crate::handlers::document::AddDocumentResponse,
            // crate::handlers::document::BulkOperation,
            // crate::handlers::document::BulkRequest,
            // crate::handlers::document::BulkResponse,
            // crate::handlers::search::SearchRequest,
            // lexum_core::SearchResult,
            // lexum_core::SearchHit,
            crate::handlers::snapshot::CreateRepositoryRequest,
            crate::handlers::snapshot::RepositoryResponse,
            lexum_core::CreateSnapshotRequest,
            lexum_core::SnapshotInfo,
            crate::handlers::snapshot::SnapshotListResponse,
            lexum_core::RestoreSnapshotRequest,
            lexum_core::SnapshotStats,
            // crate::error::ApiError,
        )
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Indices", description = "Index management endpoints"),
        (name = "Documents", description = "Document CRUD operations"),
        (name = "Search", description = "Search operations"),
        (name = "Snapshots", description = "Snapshot and repository management"),
    ),
    info(
        title = "Lexum Search Engine API",
        version = "0.1.0",
        description = "REST API for Lexum distributed search engine",
        contact(
            name = "Lexum Team",
            email = "team@lexum.dev"
        ),
        license(
            name = "Apache-2.0",
            url = "https://www.apache.org/licenses/LICENSE-2.0"
        )
    ),
    servers(
        (url = "http://localhost:9200", description = "Development server"),
        (url = "https://api.lexum.dev", description = "Production server")
    )
)]
pub struct ApiDoc;

/// Create Swagger UI for the API
pub fn create_swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi())
}
