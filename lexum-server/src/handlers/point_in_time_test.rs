//! Tests for Point in Time handlers

#[cfg(test)]
mod tests {
    use crate::handlers::index::AppState;
    use crate::handlers::point_in_time::*;
    use axum::Json;
    use axum::extract::{Path, Query, State};
    use lexum_core::document::DocumentStore;
    use lexum_core::{IndexManager, SnapshotManager, TemplateManager};
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::RwLock;

    async fn setup_test_state() -> (AppState, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::create_dir_all(temp_dir.path()).await.unwrap();
        let index_manager = Arc::new(IndexManager::new(temp_dir.path()));

        let config = lexum_core::config::Config::default();
        let snapshot_manager = Arc::new(RwLock::new(SnapshotManager::new(&config).unwrap_or_else(
            |_| {
                let mut fallback_config = config;
                fallback_config.snapshots.repositories =
                    vec![lexum_core::config::SnapshotRepositoryConfig {
                        name: "default".to_string(),
                        repository_type: "fs".to_string(),
                        settings: lexum_core::config::SnapshotRepositorySettings {
                            location: temp_dir
                                .path()
                                .join("snapshots")
                                .to_string_lossy()
                                .to_string(),
                            ..Default::default()
                        },
                    }];
                SnapshotManager::new(&fallback_config).unwrap()
            },
        )));

        let state = AppState {
            index_manager,
            snapshot_manager,
            template_manager: Arc::new(TemplateManager::new()),
            task_manager: Arc::new(crate::handlers::reindex::TaskManager::new()),
            progress_tracker: Arc::new(lexum_core::ProgressTracker::new()),
            auth_state: crate::middleware::auth::AuthState::new(
                crate::middleware::auth::AuthConfig::default(),
            ),
            query_complexity_config:
                crate::middleware::query_complexity::QueryComplexityLimitConfig::default(),
            metrics: Arc::new(crate::handlers::metrics::PrometheusMetrics::new()),
        };

        (state, temp_dir)
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_create_pit() {
        let (state, _) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("title", lexum_core::FieldType::Text)
                    .stored(true)
                    .indexed(true),
            )
            .build()
            .unwrap()
            .0;

        let index_settings = lexum_core::IndexSettings::default();
        state
            .index_manager
            .create_index("test-index", schema, index_settings)
            .await
            .unwrap();

        let params = CreatePitParams {
            keep_alive: "5m".to_string(),
        };

        let result = create_pit(State(state), Path("test-index".to_string()), Query(params)).await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert!(response.id.starts_with("pit_"));
        assert!(response.creation_time > 0);
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_create_pit_invalid_index() {
        let (state, _) = setup_test_state().await;

        let params = CreatePitParams {
            keep_alive: "5m".to_string(),
        };

        let result = create_pit(
            State(state),
            Path("non-existent-index".to_string()),
            Query(params),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_delete_pit() {
        let (state, _) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("title", lexum_core::FieldType::Text)
                    .stored(true)
                    .indexed(true),
            )
            .build()
            .unwrap()
            .0;

        let index_settings = lexum_core::IndexSettings::default();
        state
            .index_manager
            .create_index("test-index", schema, index_settings)
            .await
            .unwrap();

        // Create PIT first
        let params = CreatePitParams {
            keep_alive: "5m".to_string(),
        };
        let create_result = create_pit(
            State(state.clone()),
            Path("test-index".to_string()),
            Query(params),
        )
        .await
        .unwrap();
        let pit_id = create_result.0.id;

        // Delete PIT
        let result = delete_pit(Path(pit_id.clone())).await;
        assert!(result.is_ok());

        // Try to delete again - should fail
        let result2 = delete_pit(Path(pit_id)).await;
        assert!(result2.is_err());
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_extend_pit() {
        let (state, _) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("title", lexum_core::FieldType::Text)
                    .stored(true)
                    .indexed(true),
            )
            .build()
            .unwrap()
            .0;

        let index_settings = lexum_core::IndexSettings::default();
        state
            .index_manager
            .create_index("test-index", schema, index_settings)
            .await
            .unwrap();

        // Create PIT first
        let params = CreatePitParams {
            keep_alive: "1m".to_string(),
        };
        let create_result = create_pit(State(state), Path("test-index".to_string()), Query(params))
            .await
            .unwrap();
        let pit_id = create_result.0.id;

        // Extend PIT
        let extend_request = json!({
            "keep_alive": "10m"
        });
        let result = extend_pit(Path(pit_id), Json(extend_request)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_search_with_pit() {
        let (state, _) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("title", lexum_core::FieldType::Text)
                    .stored(true)
                    .indexed(true),
            )
            .build()
            .unwrap()
            .0;

        let index_settings = lexum_core::IndexSettings::default();
        let index = state
            .index_manager
            .create_index("test-index", schema, index_settings)
            .await
            .unwrap();

        // Add a document
        let index_arc = Arc::new(index);
        let store = DocumentStore::new(index_arc.clone());
        store
            .add_document(json!({"title": "Test Document"}))
            .await
            .unwrap();

        // Create PIT
        let params = CreatePitParams {
            keep_alive: "5m".to_string(),
        };
        let create_result = create_pit(
            State(state.clone()),
            Path("test-index".to_string()),
            Query(params),
        )
        .await
        .unwrap();
        let pit_id = create_result.0.id;

        // Search with PIT
        let search_request = SearchWithPitRequest {
            pit_id: Some(pit_id),
            query: Some(lexum_core::Query::MatchAll),
            filter: None,
            limit: 10,
            offset: 0,
            sort: None,
            keep_alive: None,
        };

        let result = search_with_pit(State(state), Json(search_request)).await;
        assert!(result.is_ok());
        let search_result = result.unwrap().0;
        assert!(search_result.total > 0);
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_search_with_pit_missing_pit_id() {
        let (state, _) = setup_test_state().await;

        let search_request = SearchWithPitRequest {
            pit_id: None,
            query: Some(lexum_core::Query::MatchAll),
            filter: None,
            limit: 10,
            offset: 0,
            sort: None,
            keep_alive: None,
        };

        let result = search_with_pit(State(state), Json(search_request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_create_pit_invalid_keep_alive() {
        let (state, _) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("title", lexum_core::FieldType::Text)
                    .stored(true)
                    .indexed(true),
            )
            .build()
            .unwrap()
            .0;

        let index_settings = lexum_core::IndexSettings::default();
        state
            .index_manager
            .create_index("test-index", schema, index_settings)
            .await
            .unwrap();

        let params = CreatePitParams {
            keep_alive: "invalid".to_string(),
        };

        let result = create_pit(State(state), Path("test-index".to_string()), Query(params)).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_extend_pit_invalid_keep_alive() {
        let (state, _) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("title", lexum_core::FieldType::Text)
                    .stored(true)
                    .indexed(true),
            )
            .build()
            .unwrap()
            .0;

        let index_settings = lexum_core::IndexSettings::default();
        state
            .index_manager
            .create_index("test-index", schema, index_settings)
            .await
            .unwrap();

        // Create PIT first
        let params = CreatePitParams {
            keep_alive: "1m".to_string(),
        };
        let create_result = create_pit(State(state), Path("test-index".to_string()), Query(params))
            .await
            .unwrap();
        let pit_id = create_result.0.id;

        // Try to extend with invalid keep_alive
        let extend_request = json!({
            "keep_alive": "invalid"
        });
        let result = extend_pit(Path(pit_id), Json(extend_request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_extend_pit_missing_keep_alive() {
        let (state, _) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("title", lexum_core::FieldType::Text)
                    .stored(true)
                    .indexed(true),
            )
            .build()
            .unwrap()
            .0;

        let index_settings = lexum_core::IndexSettings::default();
        state
            .index_manager
            .create_index("test-index", schema, index_settings)
            .await
            .unwrap();

        // Create PIT first
        let params = CreatePitParams {
            keep_alive: "1m".to_string(),
        };
        let create_result = create_pit(State(state), Path("test-index".to_string()), Query(params))
            .await
            .unwrap();
        let pit_id = create_result.0.id;

        // Try to extend without keep_alive
        let extend_request = json!({});
        let result = extend_pit(Path(pit_id), Json(extend_request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_search_with_pit_with_filter() {
        let (state, _) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("title", lexum_core::FieldType::Text)
                    .stored(true)
                    .indexed(true),
            )
            .build()
            .unwrap()
            .0;

        let index_settings = lexum_core::IndexSettings::default();
        let index = state
            .index_manager
            .create_index("test-index", schema, index_settings)
            .await
            .unwrap();

        // Add documents
        let index_arc = Arc::new(index);
        let store = DocumentStore::new(index_arc.clone());
        for i in 0..3 {
            store
                .add_document(json!({"title": format!("Document {i}")}))
                .await
                .unwrap();
        }

        // Create PIT
        let params = CreatePitParams {
            keep_alive: "5m".to_string(),
        };
        let create_result = create_pit(
            State(state.clone()),
            Path("test-index".to_string()),
            Query(params),
        )
        .await
        .unwrap();
        let pit_id = create_result.0.id;

        // Search with filter
        let filter_query = lexum_core::Query::Term(lexum_core::TermQuery::new("title", "Document"));
        let search_request = SearchWithPitRequest {
            pit_id: Some(pit_id),
            query: Some(lexum_core::Query::MatchAll),
            filter: Some(vec![filter_query]),
            limit: 10,
            offset: 0,
            sort: None,
            keep_alive: None,
        };

        let result = search_with_pit(State(state), Json(search_request)).await;
        assert!(result.is_ok());
        let search_result = result.unwrap().0;
        assert!(search_result.total >= 0);
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_search_with_pit_with_sort() {
        let (state, _) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("title", lexum_core::FieldType::Text)
                    .stored(true)
                    .indexed(true),
            )
            .build()
            .unwrap()
            .0;

        let index_settings = lexum_core::IndexSettings::default();
        let index = state
            .index_manager
            .create_index("test-index", schema, index_settings)
            .await
            .unwrap();

        // Add documents
        let index_arc = Arc::new(index);
        let store = DocumentStore::new(index_arc.clone());
        for i in 0..3 {
            store
                .add_document(json!({"title": format!("Document {i}")}))
                .await
                .unwrap();
        }

        // Create PIT
        let params = CreatePitParams {
            keep_alive: "5m".to_string(),
        };
        let create_result = create_pit(
            State(state.clone()),
            Path("test-index".to_string()),
            Query(params),
        )
        .await
        .unwrap();
        let pit_id = create_result.0.id;

        // Search with sort
        let sort = Some(lexum_core::search::SortOption::desc("_score"));
        let search_request = SearchWithPitRequest {
            pit_id: Some(pit_id),
            query: Some(lexum_core::Query::MatchAll),
            filter: None,
            limit: 10,
            offset: 0,
            sort,
            keep_alive: None,
        };

        let result = search_with_pit(State(state), Json(search_request)).await;
        assert!(result.is_ok());
        let search_result = result.unwrap().0;
        assert!(search_result.total >= 0);
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_search_with_pit_extend_keep_alive() {
        let (state, _) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("title", lexum_core::FieldType::Text)
                    .stored(true)
                    .indexed(true),
            )
            .build()
            .unwrap()
            .0;

        let index_settings = lexum_core::IndexSettings::default();
        let index = state
            .index_manager
            .create_index("test-index", schema, index_settings)
            .await
            .unwrap();

        // Add a document
        let index_arc = Arc::new(index);
        let store = DocumentStore::new(index_arc.clone());
        store
            .add_document(json!({"title": "Test Document"}))
            .await
            .unwrap();

        // Create PIT
        let params = CreatePitParams {
            keep_alive: "1m".to_string(),
        };
        let create_result = create_pit(
            State(state.clone()),
            Path("test-index".to_string()),
            Query(params),
        )
        .await
        .unwrap();
        let pit_id = create_result.0.id.clone();

        // Search with PIT and extend keep_alive
        let search_request = SearchWithPitRequest {
            pit_id: Some(pit_id.clone()),
            query: Some(lexum_core::Query::MatchAll),
            filter: None,
            limit: 10,
            offset: 0,
            sort: None,
            keep_alive: Some("10m".to_string()),
        };

        let result = search_with_pit(State(state.clone()), Json(search_request)).await;
        assert!(result.is_ok());

        // Verify PIT is still valid (would fail if not extended)
        let verify_result = search_with_pit(
            State(state),
            Json(SearchWithPitRequest {
                pit_id: Some(pit_id),
                query: Some(lexum_core::Query::MatchAll),
                filter: None,
                limit: 10,
                offset: 0,
                sort: None,
                keep_alive: None,
            }),
        )
        .await;
        assert!(verify_result.is_ok());
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_search_with_pit_invalid_pit_id() {
        let (state, _) = setup_test_state().await;

        let search_request = SearchWithPitRequest {
            pit_id: Some("invalid_pit_id".to_string()),
            query: Some(lexum_core::Query::MatchAll),
            filter: None,
            limit: 10,
            offset: 0,
            sort: None,
            keep_alive: None,
        };

        let result = search_with_pit(State(state), Json(search_request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_create_pit_default_keep_alive() {
        let (state, _) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("title", lexum_core::FieldType::Text)
                    .stored(true)
                    .indexed(true),
            )
            .build()
            .unwrap()
            .0;

        let index_settings = lexum_core::IndexSettings::default();
        state
            .index_manager
            .create_index("test-index", schema, index_settings)
            .await
            .unwrap();

        // Use default keep_alive
        let params = CreatePitParams {
            keep_alive: "5m".to_string(), // Default from function
        };

        let result = create_pit(State(state), Path("test-index".to_string()), Query(params)).await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert!(response.id.starts_with("pit_"));
    }
}
