//! Tests for rollover functionality

use super::rollover::*;
use crate::handlers::index::AppState;
use lexum_core::index::{IndexManager, IndexSettings};
use lexum_core::schema::SchemaBuilder;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Create a test AppState
async fn create_test_state() -> AppState {
    let temp_dir = std::env::temp_dir().join("lexum_rollover_test");
    std::fs::create_dir_all(&temp_dir).ok();

    let index_manager = Arc::new(IndexManager::new(&temp_dir));

    // Create default managers for AppState
    let config = lexum_core::config::Config::default();
    let snapshot_manager = Arc::new(RwLock::new(
        lexum_core::SnapshotManager::new(&config).unwrap_or_else(|_| {
            let mut fallback_config = config;
            fallback_config.snapshots.repositories =
                vec![lexum_core::config::SnapshotRepositoryConfig {
                    name: "default".to_string(),
                    repository_type: "fs".to_string(),
                    settings: lexum_core::config::SnapshotRepositorySettings {
                        location: temp_dir.join("snapshots").to_string_lossy().to_string(),
                        ..Default::default()
                    },
                }];
            lexum_core::SnapshotManager::new(&fallback_config).unwrap()
        }),
    ));

    let template_manager = Arc::new(lexum_core::TemplateManager::new());
    let task_manager = Arc::new(crate::handlers::reindex::TaskManager::new());

    AppState {
        index_manager,
        snapshot_manager,
        template_manager,
        task_manager,
        progress_tracker: Arc::new(lexum_core::ProgressTracker::new()),
    }
}

#[tokio::test]
async fn test_rollover_conditions_max_docs() {
    let conditions = RolloverConditions {
        max_docs: Some(1000),
        ..Default::default()
    };

    let stats = IndexStats {
        num_docs: 1000,
        size_in_bytes: 1024000,
        age_in_millis: 0,
        num_primary_shards: 1,
    };

    let (met, reason) = check_rollover_conditions(&conditions, &stats);
    assert!(met);
    assert_eq!(reason, Some("max_docs:1000".to_string()));
}

#[tokio::test]
async fn test_rollover_conditions_max_size() {
    let conditions = RolloverConditions {
        max_size: Some("1mb".to_string()),
        ..Default::default()
    };

    let stats = IndexStats {
        num_docs: 100,
        size_in_bytes: 1024 * 1024, // 1MB
        age_in_millis: 0,
        num_primary_shards: 1,
    };

    let (met, reason) = check_rollover_conditions(&conditions, &stats);
    assert!(met);
    assert_eq!(reason, Some("max_size:1mb".to_string()));
}

#[tokio::test]
async fn test_rollover_conditions_max_age() {
    let conditions = RolloverConditions {
        max_age: Some("1h".to_string()),
        ..Default::default()
    };

    let stats = IndexStats {
        num_docs: 100,
        size_in_bytes: 102400,
        age_in_millis: 60 * 60 * 1000, // 1 hour in milliseconds
        num_primary_shards: 1,
    };

    let (met, reason) = check_rollover_conditions(&conditions, &stats);
    assert!(met);
    assert_eq!(reason, Some("max_age:1h".to_string()));
}

#[tokio::test]
async fn test_rollover_conditions_not_met() {
    let conditions = RolloverConditions {
        max_docs: Some(1000),
        max_size: Some("1gb".to_string()),
        max_age: Some("1d".to_string()),
        ..Default::default()
    };

    let stats = IndexStats {
        num_docs: 100,
        size_in_bytes: 102400,
        age_in_millis: 1000, // 1 second
        num_primary_shards: 1,
    };

    let (met, reason) = check_rollover_conditions(&conditions, &stats);
    assert!(!met);
    assert_eq!(reason, None);
}

#[tokio::test]
async fn test_generate_rollover_index_name_with_number() {
    let result = generate_rollover_index_name("logs-2023-01-01");
    assert_eq!(result, "logs-2023-01-000002");
}

#[tokio::test]
async fn test_generate_rollover_index_name_without_number() {
    let result = generate_rollover_index_name("logs");
    assert_eq!(result, "logs-000001");
}

#[tokio::test]
async fn test_generate_rollover_index_name_increment() {
    let result = generate_rollover_index_name("logs-000001");
    assert_eq!(result, "logs-000002");
}

#[tokio::test]
#[ignore] // Temporarily disabled due to Tantivy filesystem compatibility issues in WSL
async fn test_rollover_index_dry_run() {
    use tokio::time::{Duration, timeout};

    let test_future = async {
        let state = create_test_state().await;

        // Create a test index
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("content")
            .build()
            .unwrap();
        let settings = IndexSettings::default();

        state
            .index_manager
            .create_index("test-index", schema, settings)
            .await
            .unwrap();

        let request = RolloverRequest {
            conditions: RolloverConditions {
                max_docs: Some(1000),
                ..Default::default()
            },
            new_index: None,
            dry_run: true,
        };

        let result = rollover_index(
            axum::extract::State(state),
            axum::extract::Path("test-index".to_string()),
            axum::Json(request),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert!(response.dry_run);
        assert_eq!(response.old_index, "test-index");
        assert_eq!(response.new_index, "test-index-000001");
    };

    // Run with timeout
    timeout(Duration::from_secs(10), test_future).await.unwrap();
}

#[tokio::test]
async fn test_rollover_index_not_found() {
    let state = create_test_state().await;

    let request = RolloverRequest {
        conditions: RolloverConditions::default(),
        new_index: None,
        dry_run: true,
    };

    let result = rollover_index(
        axum::extract::State(state),
        axum::extract::Path("nonexistent-index".to_string()),
        axum::Json(request),
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
#[ignore] // Temporarily disabled due to Tantivy filesystem compatibility issues in WSL
async fn test_get_rollover_conditions() {
    use tokio::time::{Duration, timeout};

    let test_future = async {
        let state = create_test_state().await;

        // Create a test index
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("content")
            .build()
            .unwrap();
        let settings = IndexSettings::default();

        state
            .index_manager
            .create_index("test-index", schema, settings)
            .await
            .unwrap();

        let result = get_rollover_conditions(
            axum::extract::State(state),
            axum::extract::Path("test-index".to_string()),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response.max_docs, None);
        assert_eq!(response.max_size, None);
        assert_eq!(response.max_age, None);
    };

    // Run with timeout
    timeout(Duration::from_secs(10), test_future).await.unwrap();
}

#[tokio::test]
#[ignore] // Temporarily disabled due to Tantivy filesystem compatibility issues in WSL
async fn test_update_rollover_conditions() {
    use tokio::time::{Duration, timeout};

    let test_future = async {
        let state = create_test_state().await;

        // Create a test index
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("content")
            .build()
            .unwrap();
        let settings = IndexSettings::default();

        state
            .index_manager
            .create_index("test-index", schema, settings)
            .await
            .unwrap();

        let conditions = RolloverConditions {
            max_docs: Some(1000),
            max_size: Some("1gb".to_string()),
            max_age: Some("7d".to_string()),
            ..Default::default()
        };

        let result = update_rollover_conditions(
            axum::extract::State(state),
            axum::extract::Path("test-index".to_string()),
            axum::Json(conditions),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response["acknowledged"], true);
        assert_eq!(response["index"], "test-index");
    };

    // Run with timeout
    timeout(Duration::from_secs(10), test_future).await.unwrap();
}
