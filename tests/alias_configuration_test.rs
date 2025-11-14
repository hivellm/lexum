//! Tests for alias configuration and routing features

use anyhow::Result;
use lexum_core::index::{IndexManager, IndexSettings};
use lexum_core::schema::SchemaBuilder;
use lexum_core::types::IndexName;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

/// Test helper to create a test index manager
async fn create_test_manager() -> Result<(TempDir, Arc<IndexManager>)> {
    let temp_dir = TempDir::new()?;
    let manager = Arc::new(IndexManager::new(temp_dir.path()));
    Ok((temp_dir, manager))
}

/// Test helper to create a test index
async fn create_test_index(manager: &Arc<IndexManager>, name: &str) -> Result<()> {
    let schema_builder = SchemaBuilder::new()
        .add_text_field("title")
        .add_text_field("content")
        .add_text_field("status")
        .add_text_field("category")
        .add_text_field("priority")
        .add_text_field("featured")
        .add_text_field("created_at");
    let (schema, _) = schema_builder.build()?;
    let settings = IndexSettings::new();

    // Retry index creation with exponential backoff
    let mut attempts = 0;
    let max_attempts = 3;
    loop {
        attempts += 1;
        match manager.create_index(name, schema.clone(), settings.clone()).await {
            Ok(_) => return Ok(()),
            Err(e) if attempts < max_attempts => {
                eprintln!("Attempt {attempts} failed: {e}");
                sleep(Duration::from_millis(100 * attempts)).await;
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    }
}

#[lexum_macros::tokio_test]
async fn test_alias_with_simple_filter() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with simple filter
    use lexum_core::index::alias::AliasConfig;
    
    let config = AliasConfig {
        filter: Some(json!({
            "term": {
                "status": "active"
            }
        })),
        ..Default::default()
    };

    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("active_alias", indices, Some(config))?;

    assert_eq!(alias.name.as_str(), "active_alias");
    assert!(alias.config.filter.is_some());
    
    let filter = alias.config.filter.unwrap();
    assert!(filter.get("term").is_some());
    assert_eq!(filter["term"]["status"], "active");

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_with_complex_filter() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with complex boolean filter
    use lexum_core::index::alias::AliasConfig;
    
    let config = AliasConfig {
        filter: Some(json!({
            "bool": {
                "must": [
                    {"term": {"status": "active"}},
                    {"range": {"created_at": {"gte": "2023-01-01"}}}
                ],
                "should": [
                    {"term": {"priority": "high"}},
                    {"term": {"featured": true}}
                ],
                "minimum_should_match": 1
            }
        })),
        ..Default::default()
    };

    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("complex_filter_alias", indices, Some(config))?;

    assert_eq!(alias.name.as_str(), "complex_filter_alias");
    assert!(alias.config.filter.is_some());
    
    let filter = alias.config.filter.unwrap();
    assert!(filter.get("bool").is_some());
    
    let bool_query = filter.get("bool").unwrap();
    assert!(bool_query.get("must").is_some());
    assert!(bool_query.get("should").is_some());
    assert_eq!(bool_query["minimum_should_match"], 1);

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_with_routing() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with routing configuration
    use lexum_core::index::alias::AliasConfig;
    
    let config = AliasConfig {
        routing: Some("user123".to_string()),
        search_routing: Some("user123".to_string()),
        index_routing: Some("user123".to_string()),
        ..Default::default()
    };

    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("routed_alias", indices, Some(config))?;

    assert_eq!(alias.name.as_str(), "routed_alias");
    assert_eq!(alias.config.routing, Some("user123".to_string()));
    assert_eq!(alias.config.search_routing, Some("user123".to_string()));
    assert_eq!(alias.config.index_routing, Some("user123".to_string()));

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_with_write_index_flag() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with write index flag
    use lexum_core::index::alias::AliasConfig;
    
    let config = AliasConfig {
        is_write_index: Some(true),
        ..Default::default()
    };

    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("write_alias", indices, Some(config))?;

    assert_eq!(alias.name.as_str(), "write_alias");
    assert_eq!(alias.config.is_write_index, Some(true));

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_with_combined_configuration() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with all configuration options
    use lexum_core::index::alias::AliasConfig;
    
    let config = AliasConfig {
        filter: Some(json!({
            "bool": {
                "must": [
                    {"term": {"status": "active"}},
                    {"range": {"created_at": {"gte": "2023-01-01"}}}
                ]
            }
        })),
        routing: Some("user456".to_string()),
        search_routing: Some("user456".to_string()),
        index_routing: Some("user456".to_string()),
        is_write_index: Some(true),
    };

    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("full_config_alias", indices, Some(config))?;

    assert_eq!(alias.name.as_str(), "full_config_alias");
    assert!(alias.config.filter.is_some());
    assert_eq!(alias.config.routing, Some("user456".to_string()));
    assert_eq!(alias.config.search_routing, Some("user456".to_string()));
    assert_eq!(alias.config.index_routing, Some("user456".to_string()));
    assert_eq!(alias.config.is_write_index, Some(true));

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_config_serialization() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with configuration
    use lexum_core::index::alias::AliasConfig;
    
    let config = AliasConfig {
        filter: Some(json!({
            "term": {"status": "active"}
        })),
        routing: Some("user789".to_string()),
        search_routing: Some("user789".to_string()),
        index_routing: Some("user789".to_string()),
        is_write_index: Some(false),
    };

    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("serialization_test_alias", indices, Some(config))?;

    // Test serialization
    let json = serde_json::to_string(&alias.config)?;
    assert!(json.contains("filter"));
    assert!(json.contains("routing"));
    assert!(json.contains("search_routing"));
    assert!(json.contains("index_routing"));
    assert!(json.contains("is_write_index"));

    // Test deserialization
    let deserialized_config: AliasConfig = serde_json::from_str(&json)?;
    assert_eq!(deserialized_config.filter, alias.config.filter);
    assert_eq!(deserialized_config.routing, alias.config.routing);
    assert_eq!(deserialized_config.search_routing, alias.config.search_routing);
    assert_eq!(deserialized_config.index_routing, alias.config.index_routing);
    assert_eq!(deserialized_config.is_write_index, alias.config.is_write_index);

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_config_defaults() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with default configuration
    use lexum_core::index::alias::AliasConfig;
    
    let config = AliasConfig::default();
    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("default_config_alias", indices, Some(config))?;

    assert_eq!(alias.name.as_str(), "default_config_alias");
    assert!(alias.config.filter.is_none());
    assert!(alias.config.routing.is_none());
    assert!(alias.config.search_routing.is_none());
    assert!(alias.config.index_routing.is_none());
    assert!(alias.config.is_write_index.is_none());

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_with_nested_filters() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with nested filter structure
    use lexum_core::index::alias::AliasConfig;
    
    let config = AliasConfig {
        filter: Some(json!({
            "bool": {
                "must": [
                    {
                        "bool": {
                            "should": [
                                {"term": {"status": "active"}},
                                {"term": {"status": "pending"}}
                            ]
                        }
                    },
                    {
                        "range": {
                            "created_at": {
                                "gte": "2023-01-01",
                                "lte": "2023-12-31"
                            }
                        }
                    }
                ],
                "should": [
                    {"term": {"priority": "high"}},
                    {"term": {"featured": true}}
                ],
                "must_not": [
                    {"term": {"deleted": true}}
                ]
            }
        })),
        ..Default::default()
    };

    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("nested_filter_alias", indices, Some(config))?;

    assert_eq!(alias.name.as_str(), "nested_filter_alias");
    assert!(alias.config.filter.is_some());
    
    let filter = alias.config.filter.unwrap();
    assert!(filter.get("bool").is_some());
    
    let bool_query = filter.get("bool").unwrap();
    assert!(bool_query.get("must").is_some());
    assert!(bool_query.get("should").is_some());
    assert!(bool_query.get("must_not").is_some());

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_with_multiple_routing_values() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with different routing values
    use lexum_core::index::alias::AliasConfig;
    
    let config = AliasConfig {
        routing: Some("general_routing".to_string()),
        search_routing: Some("search_routing".to_string()),
        index_routing: Some("index_routing".to_string()),
        ..Default::default()
    };

    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("multi_routing_alias", indices, Some(config))?;

    assert_eq!(alias.name.as_str(), "multi_routing_alias");
    assert_eq!(alias.config.routing, Some("general_routing".to_string()));
    assert_eq!(alias.config.search_routing, Some("search_routing".to_string()));
    assert_eq!(alias.config.index_routing, Some("index_routing".to_string()));

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_config_validation() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Test alias creation with invalid JSON filter
    use lexum_core::index::alias::AliasConfig;
    
    let config = AliasConfig {
        filter: Some(json!({
            "invalid_query": {
                "field": "value"
            }
        })),
        ..Default::default()
    };

    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("validation_test_alias", indices, Some(config))?;

    // The alias should still be created even with invalid filter
    // (validation happens at query time, not at alias creation time)
    assert_eq!(alias.name.as_str(), "validation_test_alias");
    assert!(alias.config.filter.is_some());

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_with_empty_configuration() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with empty configuration
    use lexum_core::index::alias::AliasConfig;
    
    let config = AliasConfig {
        filter: Some(json!({})),
        routing: Some("".to_string()),
        search_routing: Some("".to_string()),
        index_routing: Some("".to_string()),
        is_write_index: Some(false),
    };

    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("empty_config_alias", indices, Some(config))?;

    assert_eq!(alias.name.as_str(), "empty_config_alias");
    assert!(alias.config.filter.is_some());
    assert_eq!(alias.config.routing, Some("".to_string()));
    assert_eq!(alias.config.search_routing, Some("".to_string()));
    assert_eq!(alias.config.index_routing, Some("".to_string()));
    assert_eq!(alias.config.is_write_index, Some(false));

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_config_clone_and_equality() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with configuration
    use lexum_core::index::alias::AliasConfig;
    
    let config1 = AliasConfig {
        filter: Some(json!({"term": {"status": "active"}})),
        routing: Some("user123".to_string()),
        search_routing: Some("user123".to_string()),
        index_routing: Some("user123".to_string()),
        is_write_index: Some(true),
    };

    let indices = vec![IndexName::new("index1")];
    let alias1 = manager.create_alias("clone_test_alias1", indices.clone(), Some(config1.clone()))?;

    // Test clone
    let config2 = config1.clone();
    let alias2 = manager.create_alias("clone_test_alias2", indices, Some(config2))?;

    // Test equality
    assert_eq!(alias1.config, alias2.config);
    assert_eq!(alias1.config.filter, alias2.config.filter);
    assert_eq!(alias1.config.routing, alias2.config.routing);
    assert_eq!(alias1.config.search_routing, alias2.config.search_routing);
    assert_eq!(alias1.config.index_routing, alias2.config.index_routing);
    assert_eq!(alias1.config.is_write_index, alias2.config.is_write_index);

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_with_large_filter() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with large filter
    use lexum_core::index::alias::AliasConfig;
    
    let mut must_conditions = Vec::new();
    for i in 0..100 {
        must_conditions.push(json!({
            "term": {
                "field": format!("value{}", i)
            }
        }));
    }

    let config = AliasConfig {
        filter: Some(json!({
            "bool": {
                "must": must_conditions
            }
        })),
        ..Default::default()
    };

    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("large_filter_alias", indices, Some(config))?;

    assert_eq!(alias.name.as_str(), "large_filter_alias");
    assert!(alias.config.filter.is_some());
    
    let filter = alias.config.filter.unwrap();
    assert!(filter.get("bool").is_some());
    
    let bool_query = filter.get("bool").unwrap();
    let must_conditions = bool_query.get("must").unwrap().as_array().unwrap();
    assert_eq!(must_conditions.len(), 100);

    Ok(())
}