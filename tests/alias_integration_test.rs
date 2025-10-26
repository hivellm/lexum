//! Integration tests for alias functionality across all components

use anyhow::Result;
use lexum_core::index::{IndexManager, IndexSettings};
use lexum_core::schema::SchemaBuilder;
use lexum_core::types::IndexName;
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
        .add_text_field("content");
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

#[tokio::test]
async fn test_alias_creation_and_resolution() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    create_test_index(&manager, "index1").await?;
    create_test_index(&manager, "index2").await?;
    create_test_index(&manager, "index3").await?;

    // Create aliases
    let indices1 = vec![IndexName::new("index1"), IndexName::new("index2")];
    manager.create_alias("multi_alias", indices1)?;

    let indices2 = vec![IndexName::new("index3")];
    manager.create_alias("single_alias", indices2)?;

    // Test alias resolution
    let resolved_multi = manager.resolve_name("multi_alias")?;
    assert_eq!(resolved_multi.len(), 2);
    assert!(resolved_multi.iter().any(|i| i.as_str() == "index1"));
    assert!(resolved_multi.iter().any(|i| i.as_str() == "index2"));

    let resolved_single = manager.resolve_name("single_alias")?;
    assert_eq!(resolved_single.len(), 1);
    assert_eq!(resolved_single[0].as_str(), "index3");

    // Test direct index resolution
    let resolved_direct = manager.resolve_name("index1")?;
    assert_eq!(resolved_direct.len(), 1);
    assert_eq!(resolved_direct[0].as_str(), "index1");

    Ok(())
}

#[tokio::test]
async fn test_alias_operations_integration() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    create_test_index(&manager, "index1").await?;
    create_test_index(&manager, "index2").await?;
    create_test_index(&manager, "index3").await?;

    // Create initial alias
    let indices = vec![IndexName::new("index1")];
    manager.create_alias("test_alias", indices)?;

    // Add more indices to alias
    let new_indices = vec![IndexName::new("index2"), IndexName::new("index3")];
    let updated_alias = manager.add_indices_to_alias("test_alias", new_indices)?;
    assert_eq!(updated_alias.index_count(), 3);

    // Remove some indices from alias
    let remove_indices = vec![IndexName::new("index2")];
    let final_alias = manager.remove_indices_from_alias("test_alias", remove_indices)?;
    assert_eq!(final_alias.index_count(), 2);

    // Verify final state
    let resolved = manager.resolve_name("test_alias")?;
    assert_eq!(resolved.len(), 2);
    assert!(resolved.iter().any(|i| i.as_str() == "index1"));
    assert!(resolved.iter().any(|i| i.as_str() == "index3"));

    Ok(())
}

#[tokio::test]
async fn test_atomic_alias_operations_integration() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    create_test_index(&manager, "index1").await?;
    create_test_index(&manager, "index2").await?;
    create_test_index(&manager, "index3").await?;
    create_test_index(&manager, "index4").await?;

    // Create initial aliases
    let indices1 = vec![IndexName::new("index1"), IndexName::new("index2")];
    manager.create_alias("alias1", indices1)?;

    let indices2 = vec![IndexName::new("index3")];
    manager.create_alias("alias2", indices2)?;

    // Perform atomic operations
    use lexum_core::index::alias::{AliasAction, AliasOperationsRequest};
    
    let operations = vec![
        AliasAction::Add {
            alias: "alias3".into(),
            indices: vec![IndexName::new("index4")],
            config: None,
        },
        AliasAction::Remove {
            alias: "alias1".into(),
            indices: vec![IndexName::new("index1")],
        },
        AliasAction::RemoveIndex {
            alias: "alias2".into(),
        },
    ];

    let request = AliasOperationsRequest::new(operations);
    let response = manager.execute_atomic_operations(request)?;

    assert!(response.acknowledged);
    assert!(response.atomic);
    assert_eq!(response.executed_operations, 3);

    // Verify final state
    assert!(manager.alias_exists("alias1")); // Still exists but with only index2
    assert!(!manager.alias_exists("alias2")); // Removed completely
    assert!(manager.alias_exists("alias3")); // New alias with index4

    let alias1 = manager.get_alias("alias1")?;
    assert_eq!(alias1.index_count(), 1);
    assert!(alias1.contains_index(&IndexName::new("index2")));

    let alias3 = manager.get_alias("alias3")?;
    assert_eq!(alias3.index_count(), 1);
    assert!(alias3.contains_index(&IndexName::new("index4")));

    Ok(())
}

#[tokio::test]
async fn test_alias_with_configuration() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with configuration
    use lexum_core::index::alias::AliasConfig;
    
    let config = AliasConfig {
        filter: Some(serde_json::json!({
            "term": {
                "status": "active"
            }
        })),
        routing: Some("user1".to_string()),
        search_routing: Some("user1".to_string()),
        index_routing: Some("user1".to_string()),
        is_write_index: Some(true),
    };

    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("configured_alias", indices, Some(config))?;

    assert_eq!(alias.name.as_str(), "configured_alias");
    assert_eq!(alias.index_count(), 1);
    assert!(alias.config.filter.is_some());
    assert_eq!(alias.config.routing, Some("user1".to_string()));
    assert_eq!(alias.config.is_write_index, Some(true));

    Ok(())
}

#[tokio::test]
async fn test_alias_error_handling() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Test creating alias with empty indices
    let result = manager.create_alias("empty_alias", vec![], None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must have at least one target index"));

    // Test creating duplicate alias
    let indices = vec![IndexName::new("index1")];
    manager.create_alias("duplicate_alias", indices.clone())?;
    
    let result = manager.create_alias("duplicate_alias", indices, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));

    // Test getting non-existent alias
    let result = manager.get_alias("nonexistent_alias");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));

    // Test deleting non-existent alias
    let result = manager.delete_alias("nonexistent_alias");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));

    Ok(())
}

#[tokio::test]
async fn test_alias_concurrent_operations() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    for i in 0..10 {
        create_test_index(&manager, &format!("index{}", i)).await?;
    }

    // Spawn multiple tasks to create aliases concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let indices = vec![IndexName::new(&format!("index{}", i))];
            manager_clone.create_alias(&format!("alias{}", i), indices, None)
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok());
    }

    // Verify all aliases were created
    let aliases = manager.list_aliases();
    assert_eq!(aliases.len(), 10);

    // Verify each alias resolves correctly
    for i in 0..10 {
        let resolved = manager.resolve_name(&format!("alias{}", i))?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].as_str(), &format!("index{}", i));
    }

    Ok(())
}

#[tokio::test]
async fn test_alias_listing_and_management() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    create_test_index(&manager, "index1").await?;
    create_test_index(&manager, "index2").await?;
    create_test_index(&manager, "index3").await?;

    // Initially no aliases
    let aliases = manager.list_aliases();
    assert!(aliases.is_empty());

    // Create multiple aliases
    manager.create_alias("alias1", vec![IndexName::new("index1")], None)?;
    manager.create_alias("alias2", vec![IndexName::new("index2")], None)?;
    manager.create_alias("alias3", vec![IndexName::new("index3")], None)?;

    // List aliases
    let aliases = manager.list_aliases();
    assert_eq!(aliases.len(), 3);

    let alias_names: Vec<&str> = aliases.iter().map(|a| a.name.as_str()).collect();
    assert!(alias_names.contains(&"alias1"));
    assert!(alias_names.contains(&"alias2"));
    assert!(alias_names.contains(&"alias3"));

    // Delete one alias
    manager.delete_alias("alias2")?;

    // Verify deletion
    let aliases = manager.list_aliases();
    assert_eq!(aliases.len(), 2);

    let alias_names: Vec<&str> = aliases.iter().map(|a| a.name.as_str()).collect();
    assert!(alias_names.contains(&"alias1"));
    assert!(!alias_names.contains(&"alias2"));
    assert!(alias_names.contains(&"alias3"));

    Ok(())
}

#[tokio::test]
async fn test_alias_transaction_rollback() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    create_test_index(&manager, "index1").await?;
    create_test_index(&manager, "index2").await?;

    // Create initial alias
    let indices = vec![IndexName::new("index1")];
    manager.create_alias("alias1", indices, None)?;

    // Create transaction that will fail (duplicate alias)
    use lexum_core::index::alias::{AliasAction, AliasOperationsRequest};
    
    let operations = vec![
        AliasAction::Add {
            alias: "alias2".into(),
            indices: vec![IndexName::new("index2")],
            config: None,
        },
        AliasAction::Add {
            alias: "alias1".into(), // This will fail - alias already exists
            indices: vec![IndexName::new("index2")],
            config: None,
        },
    ];

    let request = AliasOperationsRequest::new(operations);
    let result = manager.execute_atomic_operations(request);

    // Should fail and rollback
    assert!(result.is_err());

    // Verify state is unchanged (rollback worked)
    assert!(manager.alias_exists("alias1"));
    assert!(!manager.alias_exists("alias2"));

    let alias1 = manager.get_alias("alias1")?;
    assert_eq!(alias1.index_count(), 1);
    assert!(alias1.contains_index(&IndexName::new("index1")));

    Ok(())
}

#[tokio::test]
async fn test_alias_performance_large_scale() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create many indices
    for i in 0..100 {
        create_test_index(&manager, &format!("index{}", i)).await?;
    }

    // Create alias pointing to all indices
    let indices: Vec<IndexName> = (0..100)
        .map(|i| IndexName::new(&format!("index{}", i)))
        .collect();
    
    let start = std::time::Instant::now();
    manager.create_alias("massive_alias", indices)?;
    let creation_time = start.elapsed();

    // Test resolution performance
    let start = std::time::Instant::now();
    let resolved = manager.resolve_name("massive_alias")?;
    let resolution_time = start.elapsed();

    assert_eq!(resolved.len(), 100);
    assert!(creation_time.as_millis() < 1000); // Should create quickly
    assert!(resolution_time.as_millis() < 100); // Should resolve quickly

    Ok(())
}

#[tokio::test]
async fn test_alias_with_complex_filtering() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test index
    create_test_index(&manager, "index1").await?;

    // Create alias with complex filter
    use lexum_core::index::alias::AliasConfig;
    
    let config = AliasConfig {
        filter: Some(serde_json::json!({
            "bool": {
                "must": [
                    {"term": {"status": "active"}},
                    {"range": {"created_at": {"gte": "2023-01-01"}}},
                    {"term": {"category": "technology"}}
                ],
                "should": [
                    {"term": {"priority": "high"}},
                    {"term": {"featured": true}}
                ],
                "minimum_should_match": 1
            }
        })),
        routing: Some("tech_users".to_string()),
        search_routing: Some("tech_users".to_string()),
        index_routing: Some("tech_users".to_string()),
        is_write_index: Some(true),
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

    Ok(())
}