//! Tests for admin handlers

#[cfg(test)]
mod tests {
    use crate::handlers::admin::*;
    use crate::handlers::index::AppState;
    use axum::extract::State;
    use lexum_core::{IndexManager, SnapshotManager, TemplateManager};
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
    async fn test_get_cluster_health_no_indices() {
        let (state, _) = setup_test_state().await;

        let result = get_cluster_health(State(state)).await;
        assert!(result.is_ok());

        let health = result.unwrap().0;
        assert_eq!(health.status, "yellow");
        assert_eq!(health.number_of_nodes, 1);
        assert_eq!(health.number_of_data_nodes, 1);
        assert_eq!(health.active_primary_shards, 0);
        assert_eq!(health.active_shards, 0);
        assert_eq!(health.relocating_shards, 0);
        assert_eq!(health.initializing_shards, 0);
        assert_eq!(health.unassigned_shards, 0);
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_get_cluster_health_with_indices() {
        let (state, _) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("id", lexum_core::FieldType::Keyword)
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

        let result = get_cluster_health(State(state)).await;
        assert!(result.is_ok());

        let health = result.unwrap().0;
        assert_eq!(health.status, "green");
        assert_eq!(health.number_of_nodes, 1);
        assert_eq!(health.active_primary_shards, 1);
        assert_eq!(health.active_shards, 1);
    }

    #[tokio::test]
    async fn test_get_cluster_stats() {
        let (state, _) = setup_test_state().await;

        let result = get_cluster_stats(State(state)).await;
        assert!(result.is_ok());

        let stats = result.unwrap().0;
        assert_eq!(stats.number_of_indices, 0);
        assert_eq!(stats.number_of_shards, 0);
        assert_eq!(stats.total_documents, 0);
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_get_cluster_stats_with_indices() {
        let (state, _temp_dir) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("id", lexum_core::FieldType::Keyword)
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

        let result = get_cluster_stats(State(state)).await;
        assert!(result.is_ok());

        let stats = result.unwrap().0;
        assert_eq!(stats.number_of_indices, 1);
        assert_eq!(stats.number_of_shards, 1);
    }

    #[tokio::test]
    async fn test_get_cluster_state() {
        let (state, _) = setup_test_state().await;

        let result = get_cluster_state(State(state)).await;
        assert!(result.is_ok());

        let cluster_state = result.unwrap().0;
        assert_eq!(cluster_state.cluster_name, "lexum-cluster");
        assert_eq!(cluster_state.nodes.count, 1);
        assert_eq!(cluster_state.routing_nodes.len(), 1);
        assert_eq!(cluster_state.routing_nodes[0], "node-1");
    }

    #[tokio::test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_get_cluster_state_with_indices() {
        let (state, _temp_dir) = setup_test_state().await;

        // Create a test index
        let schema = lexum_core::SchemaBuilder::new()
            .add_field(
                lexum_core::FieldConfig::new("id", lexum_core::FieldType::Keyword)
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

        let result = get_cluster_state(State(state)).await;
        assert!(result.is_ok());

        let cluster_state = result.unwrap().0;
        assert!(cluster_state.metadata.indices.is_object());
        let indices_obj = cluster_state.metadata.indices.as_object().unwrap();
        assert!(indices_obj.contains_key("test-index"));
    }

    #[tokio::test]
    async fn test_get_node_stats() {
        let (state, _) = setup_test_state().await;

        let result = get_node_stats(State(state)).await;
        assert!(result.is_ok());

        let node_stats = result.unwrap().0;
        assert_eq!(node_stats.name, "lexum-node-1");
        assert_eq!(node_stats.role, "master,data");
        assert!(node_stats.jvm_heap_max_bytes > 0);
        assert!(node_stats.jvm_heap_used_bytes <= node_stats.jvm_heap_max_bytes);
        assert!(node_stats.memory_usage_percent >= 0.0);
        assert!(node_stats.memory_usage_percent <= 100.0);
    }

    #[tokio::test]
    async fn test_get_cluster_settings() {
        let (state, _) = setup_test_state().await;

        let result = get_cluster_settings(State(state)).await;
        assert!(result.is_ok());

        let settings = result.unwrap().0;
        assert_eq!(settings.cluster_name, "lexum-cluster");
        assert!(!settings.persistence.storage_path.is_empty());
        assert!(!settings.persistence.snapshot.repository_path.is_empty());
        assert!(settings.persistence.snapshot.max_snapshots > 0);
        assert!(settings.network.port > 0);
    }

    #[tokio::test]
    async fn test_update_cluster_settings_valid() {
        let (state, _) = setup_test_state().await;

        let request = UpdateClusterSettingsRequest {
            settings: ClusterSettings {
                cluster_name: "test-cluster".to_string(),
                persistence: PersistenceSettings {
                    storage_path: "/tmp/test".to_string(),
                    snapshot: SnapshotSettings {
                        repository_path: "/tmp/snapshots".to_string(),
                        max_snapshots: 20,
                    },
                },
                network: NetworkSettings {
                    bind_address: "127.0.0.1".to_string(),
                    port: 18000,
                    enable_cors: true,
                },
            },
        };

        let result = update_cluster_settings(State(state), axum::Json(request)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_cluster_settings_empty_name() {
        let (state, _) = setup_test_state().await;

        let request = UpdateClusterSettingsRequest {
            settings: ClusterSettings {
                cluster_name: "".to_string(),
                persistence: PersistenceSettings {
                    storage_path: "/tmp/test".to_string(),
                    snapshot: SnapshotSettings {
                        repository_path: "/tmp/snapshots".to_string(),
                        max_snapshots: 20,
                    },
                },
                network: NetworkSettings {
                    bind_address: "127.0.0.1".to_string(),
                    port: 18000,
                    enable_cors: true,
                },
            },
        };

        let result = update_cluster_settings(State(state), axum::Json(request)).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::ApiError::InvalidRequest(_)
        ));
    }

    #[tokio::test]
    async fn test_update_cluster_settings_invalid_port() {
        let (state, _) = setup_test_state().await;

        let request = UpdateClusterSettingsRequest {
            settings: ClusterSettings {
                cluster_name: "test-cluster".to_string(),
                persistence: PersistenceSettings {
                    storage_path: "/tmp/test".to_string(),
                    snapshot: SnapshotSettings {
                        repository_path: "/tmp/snapshots".to_string(),
                        max_snapshots: 20,
                    },
                },
                network: NetworkSettings {
                    bind_address: "127.0.0.1".to_string(),
                    port: 0, // Invalid port
                    enable_cors: true,
                },
            },
        };

        let result = update_cluster_settings(State(state), axum::Json(request)).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::ApiError::InvalidRequest(_)
        ));
    }

    #[tokio::test]
    async fn test_update_cluster_settings_empty_storage_path() {
        let (state, _) = setup_test_state().await;

        let request = UpdateClusterSettingsRequest {
            settings: ClusterSettings {
                cluster_name: "test-cluster".to_string(),
                persistence: PersistenceSettings {
                    storage_path: "".to_string(), // Empty path
                    snapshot: SnapshotSettings {
                        repository_path: "/tmp/snapshots".to_string(),
                        max_snapshots: 20,
                    },
                },
                network: NetworkSettings {
                    bind_address: "127.0.0.1".to_string(),
                    port: 18000,
                    enable_cors: true,
                },
            },
        };

        let result = update_cluster_settings(State(state), axum::Json(request)).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::ApiError::InvalidRequest(_)
        ));
    }

    #[tokio::test]
    async fn test_get_cluster_info() {
        let result = get_cluster_info().await;
        assert!(result.is_ok());

        let info = result.unwrap().0;
        assert_eq!(info.name, "lexum-cluster");
        assert_eq!(info.cluster_uuid, "12345678-1234-1234-1234-123456789abc");
        assert!(!info.version.number.is_empty());
        assert!(!info.version.build_hash.is_empty());
        assert!(!info.version.build_date.is_empty());
        assert!(!info.version.lucene_version.is_empty());
    }
}
