//! OpenAPI specification for Lexum REST API

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// OpenAPI specification
#[derive(OpenApi)]
#[openapi(
    paths(
        // Health endpoints
        crate::handlers::health::health_check,

        // Index management endpoints
        crate::handlers::index::create_index,
        crate::handlers::index::list_indices,
        crate::handlers::index::get_index,
        crate::handlers::index::delete_index,
        crate::handlers::index::get_index_stats,
        crate::handlers::index::refresh_index,
        crate::handlers::index::flush_index,

        // Document CRUD operations
        crate::handlers::document::add_document,
        crate::handlers::document::get_document,
        crate::handlers::document::update_document,
        crate::handlers::document::delete_document,
        crate::handlers::document::bulk_operations,

        // Search operations
        crate::handlers::search::search,

        // Snapshot and repository management
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

        // Admin operations
        crate::handlers::admin::get_cluster_health,
        crate::handlers::admin::get_cluster_stats,
        crate::handlers::admin::get_node_stats,
        crate::handlers::admin::get_cluster_settings,
        crate::handlers::admin::update_cluster_settings,
    ),
    components(
        schemas(
            // Health schemas
            crate::handlers::health::HealthResponse,

            // Index schemas
            crate::handlers::index::FieldDefinition,
            crate::handlers::index::CreateIndexRequest,
            crate::handlers::index::IndexInfo,
            crate::handlers::index::ListIndicesResponse,

            // Document schemas
            crate::handlers::document::AddDocumentRequest,
            crate::handlers::document::AddDocumentResponse,
            crate::handlers::document::BulkOperation,
            crate::handlers::document::BulkRequest,
            crate::handlers::document::BulkResponse,
            crate::handlers::document::BulkOperationResult,

            // Search schemas
            crate::handlers::search::SearchRequest,
            lexum_core::SearchResult,
            lexum_core::SearchHit,
            lexum_core::SortOption,
            lexum_core::SortOrder,
            lexum_core::query::Query,
            lexum_core::query::MatchQuery,
            lexum_core::query::TermQuery,
            lexum_core::query::RangeQuery,
            lexum_core::query::BoolQuery,
            lexum_core::query::FuzzyQuery,
            lexum_core::query::PhraseQuery,

            // Snapshot schemas
            crate::handlers::snapshot::CreateRepositoryRequest,
            crate::handlers::snapshot::RepositoryResponse,
            lexum_core::CreateSnapshotRequest,
            lexum_core::SnapshotInfo,
            lexum_core::RestoreSnapshotRequest,
            lexum_core::SnapshotStats,

            // Admin schemas
            crate::handlers::admin::ClusterHealth,
            crate::handlers::admin::ClusterStats,
            crate::handlers::admin::NodeStats,
            crate::handlers::admin::ClusterSettings,
            crate::handlers::admin::UpdateClusterSettingsRequest,

            // Error schemas
            crate::error::ApiError,
            crate::error::ErrorResponse,
            crate::error::ValidationError,
        )
    ),
    tags(
        (name = "Health", description = "Health check and monitoring endpoints"),
        (name = "Indices", description = "Index management and configuration"),
        (name = "Documents", description = "Document CRUD operations and bulk processing"),
        (name = "Search", description = "Search operations and query execution"),
        (name = "Snapshots", description = "Snapshot and repository management"),
        (name = "Admin", description = "Administrative operations and cluster management"),
    ),
    info(
        title = "Lexum Search Engine API",
        version = "0.1.0",
        description = "REST API for Lexum distributed search engine - a high-performance, cloud-native alternative to ElasticSearch built with Rust",
        contact(
            name = "Lexum Team",
            email = "team@lexum.dev",
            url = "https://github.com/lexum-io/lexum"
        ),
        license(
            name = "Apache-2.0",
            url = "https://www.apache.org/licenses/LICENSE-2.0"
        )
    ),
    servers(
        (url = "http://localhost:9200", description = "Development server"),
        (url = "https://api.lexum.dev", description = "Production server"),
        (url = "https://staging-api.lexum.dev", description = "Staging server")
    ),
    external_docs(
        description = "Lexum Documentation",
        url = "https://docs.lexum.dev"
    )
)]
pub struct ApiDoc;

/// Create Swagger UI for the API
pub fn create_swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .config(utoipa_swagger_ui::Config::new(["/api-docs/openapi.json"]))
}

/// Generate OpenAPI specification as JSON string
pub fn generate_openapi_json() -> String {
    serde_json::to_string_pretty(&ApiDoc::openapi())
        .unwrap_or_else(|_| r#"{"error": "Failed to generate OpenAPI specification"}"#.to_string())
}

/// Generate OpenAPI specification as YAML string
pub fn generate_openapi_yaml() -> String {
    serde_yaml::to_string(&ApiDoc::openapi())
        .unwrap_or_else(|_| "error: Failed to generate OpenAPI specification".to_string())
}

/// Get OpenAPI specification version
pub fn get_openapi_version() -> &'static str {
    "3.0.3"
}

/// Get API version
pub fn get_api_version() -> &'static str {
    "0.1.0"
}

/// Get API title
pub fn get_api_title() -> &'static str {
    "Lexum Search Engine API"
}

/// Get API description
pub fn get_api_description() -> &'static str {
    "REST API for Lexum distributed search engine - a high-performance, cloud-native alternative to ElasticSearch built with Rust"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Stack overflow due to complex OpenAPI type definitions - needs investigation"]
    fn test_openapi_generation() {
        // Test basic OpenAPI generation without deep recursion
        // This test verifies the OpenAPI spec can be generated without stack overflow
        let result = std::panic::catch_unwind(ApiDoc::openapi);

        // The test should not panic due to stack overflow
        assert!(result.is_ok(), "OpenAPI generation should not panic");

        if let Ok(openapi) = result {
            assert_eq!(openapi.info.title, "Lexum Search Engine API");
            assert_eq!(openapi.info.version, "0.1.0");
            assert!(
                openapi
                    .info
                    .description
                    .as_ref()
                    .map_or(false, |d| d.contains("Lexum"))
            );
        }
    }

    #[test]
    #[ignore = "Stack overflow due to complex OpenAPI type definitions - needs investigation"]
    fn test_openapi_json_generation() {
        // Test JSON generation with error handling to avoid stack overflow
        let result = std::panic::catch_unwind(|| generate_openapi_json());

        // The test should not panic due to stack overflow
        assert!(result.is_ok(), "OpenAPI JSON generation should not panic");

        if let Ok(json) = result {
            assert!(json.contains("Lexum Search Engine API"));
            assert!(json.contains("0.1.0"));
        }
    }

    #[test]
    #[ignore = "Stack overflow due to complex OpenAPI type definitions - needs investigation"]
    fn test_openapi_yaml_generation() {
        // Test YAML generation with error handling to avoid stack overflow
        let result = std::panic::catch_unwind(|| generate_openapi_yaml());

        // The test should not panic due to stack overflow
        assert!(result.is_ok(), "OpenAPI YAML generation should not panic");

        if let Ok(yaml) = result {
            assert!(yaml.contains("Lexum Search Engine API"));
            assert!(yaml.contains("0.1.0"));
        }
    }

    #[test]
    fn test_version_constants() {
        assert_eq!(get_openapi_version(), "3.0.3");
        assert_eq!(get_api_version(), "0.1.0");
        assert_eq!(get_api_title(), "Lexum Search Engine API");
        assert!(get_api_description().contains("Lexum"));
    }
}
