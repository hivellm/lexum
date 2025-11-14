//! Performance and stress tests for alias operations

use anyhow::Result;
use lexum_core::index::{IndexManager, IndexSettings};
use lexum_core::schema::SchemaBuilder;
use lexum_core::types::IndexName;
use std::sync::Arc;
use std::time::Instant;
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

#[lexum_macros::tokio_test]
async fn test_alias_creation_performance() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    for i in 0..1000 {
        create_test_index(&manager, &format!("index{}", i)).await?;
    }

    // Test alias creation performance
    let start = Instant::now();
    for i in 0..1000 {
        let indices = vec![IndexName::new(&format!("index{}", i))];
        manager.create_alias(&format!("alias{}", i), indices, None)?;
    }
    let duration = start.elapsed();

    println!("Created 1000 aliases in {:?}", duration);
    assert!(duration.as_millis() < 5000); // Should complete within 5 seconds

    // Verify all aliases were created
    let aliases = manager.list_aliases();
    assert_eq!(aliases.len(), 1000);

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_resolution_performance() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    for i in 0..1000 {
        create_test_index(&manager, &format!("index{}", i)).await?;
    }

    // Create aliases
    for i in 0..1000 {
        let indices = vec![IndexName::new(&format!("index{}", i))];
        manager.create_alias(&format!("alias{}", i), indices, None)?;
    }

    // Test alias resolution performance
    let start = Instant::now();
    for i in 0..1000 {
        let resolved = manager.resolve_name(&format!("alias{}", i))?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].as_str(), &format!("index{}", i));
    }
    let duration = start.elapsed();

    println!("Resolved 1000 aliases in {:?}", duration);
    assert!(duration.as_millis() < 1000); // Should complete within 1 second

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_large_alias_performance() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    for i in 0..10000 {
        create_test_index(&manager, &format!("index{}", i)).await?;
    }

    // Create alias pointing to all indices
    let indices: Vec<IndexName> = (0..10000)
        .map(|i| IndexName::new(&format!("index{}", i)))
        .collect();

    let start = Instant::now();
    manager.create_alias("massive_alias", indices)?;
    let creation_time = start.elapsed();

    println!("Created alias with 10000 indices in {:?}", creation_time);
    assert!(creation_time.as_millis() < 10000); // Should complete within 10 seconds

    // Test resolution performance
    let start = Instant::now();
    let resolved = manager.resolve_name("massive_alias")?;
    let resolution_time = start.elapsed();

    println!("Resolved alias with 10000 indices in {:?}", resolution_time);
    assert_eq!(resolved.len(), 10000);
    assert!(resolution_time.as_millis() < 1000); // Should resolve quickly

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_concurrent_alias_operations() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    for i in 0..100 {
        create_test_index(&manager, &format!("index{}", i)).await?;
    }

    // Spawn multiple concurrent tasks
    let mut handles = vec![];
    let start = Instant::now();

    for i in 0..100 {
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

    let duration = start.elapsed();
    println!("Created 100 aliases concurrently in {:?}", duration);
    assert!(duration.as_millis() < 2000); // Should complete within 2 seconds

    // Verify all aliases were created
    let aliases = manager.list_aliases();
    assert_eq!(aliases.len(), 100);

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_atomic_operations_performance() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    for i in 0..1000 {
        create_test_index(&manager, &format!("index{}", i)).await?;
    }

    // Test atomic operations performance
    use lexum_core::index::alias::{AliasAction, AliasOperationsRequest};
    
    let start = Instant::now();
    
    // Create 100 atomic operations
    let mut operations = vec![];
    for i in 0..100 {
        operations.push(AliasAction::Add {
            alias: format!("alias{}", i).into(),
            indices: vec![IndexName::new(&format!("index{}", i))],
            config: None,
        });
    }

    let request = AliasOperationsRequest::new(operations);
    let response = manager.execute_atomic_operations(request)?;
    
    let duration = start.elapsed();

    println!("Executed 100 atomic operations in {:?}", duration);
    assert!(response.acknowledged);
    assert!(response.atomic);
    assert_eq!(response.executed_operations, 100);
    assert!(duration.as_millis() < 2000); // Should complete within 2 seconds

    // Verify all aliases were created
    let aliases = manager.list_aliases();
    assert_eq!(aliases.len(), 100);

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_memory_usage() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    for i in 0..10000 {
        create_test_index(&manager, &format!("index{}", i)).await?;
    }

    // Create many aliases to test memory usage
    let start = Instant::now();
    for i in 0..10000 {
        let indices = vec![IndexName::new(&format!("index{}", i))];
        manager.create_alias(&format!("alias{}", i), indices, None)?;
    }
    let duration = start.elapsed();

    println!("Created 10000 aliases in {:?}", duration);
    assert!(duration.as_millis() < 10000); // Should complete within 10 seconds

    // Test memory usage by listing all aliases
    let start = Instant::now();
    let aliases = manager.list_aliases();
    let list_duration = start.elapsed();

    println!("Listed 10000 aliases in {:?}", list_duration);
    assert_eq!(aliases.len(), 10000);
    assert!(list_duration.as_millis() < 1000); // Should list quickly

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_stress_test() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    for i in 0..1000 {
        create_test_index(&manager, &format!("index{}", i)).await?;
    }

    // Stress test: rapid create/delete operations
    let start = Instant::now();
    
    for round in 0..10 {
        // Create aliases
        for i in 0..100 {
            let indices = vec![IndexName::new(&format!("index{}", i))];
            manager.create_alias(&format!("stress_alias_{}_{}", round, i), indices, None)?;
        }
        
        // Delete aliases
        for i in 0..100 {
            manager.delete_alias(&format!("stress_alias_{}_{}", round, i))?;
        }
    }
    
    let duration = start.elapsed();

    println!("Stress test completed in {:?}", duration);
    assert!(duration.as_millis() < 30000); // Should complete within 30 seconds

    // Verify no aliases remain
    let aliases = manager.list_aliases();
    assert_eq!(aliases.len(), 0);

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_with_complex_config_performance() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    for i in 0..1000 {
        create_test_index(&manager, &format!("index{}", i)).await?;
    }

    // Test alias creation with complex configuration
    use lexum_core::index::alias::AliasConfig;
    
    let start = Instant::now();
    
    for i in 0..1000 {
        let config = AliasConfig {
            filter: Some(serde_json::json!({
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
            routing: Some(format!("user{}", i)),
            search_routing: Some(format!("user{}", i)),
            index_routing: Some(format!("user{}", i)),
            is_write_index: Some(i % 2 == 0),
        };

        let indices = vec![IndexName::new(&format!("index{}", i))];
        manager.create_alias(&format!("complex_alias{}", i), indices, Some(config))?;
    }
    
    let duration = start.elapsed();

    println!("Created 1000 aliases with complex config in {:?}", duration);
    assert!(duration.as_millis() < 10000); // Should complete within 10 seconds

    // Verify all aliases were created
    let aliases = manager.list_aliases();
    assert_eq!(aliases.len(), 1000);

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_resolution_under_load() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    for i in 0..1000 {
        create_test_index(&manager, &format!("index{}", i)).await?;
    }

    // Create aliases
    for i in 0..1000 {
        let indices = vec![IndexName::new(&format!("index{}", i))];
        manager.create_alias(&format!("alias{}", i), indices, None)?;
    }

    // Test resolution under load
    let start = Instant::now();
    
    let mut handles = vec![];
    for _ in 0..10 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            for i in 0..1000 {
                let resolved = manager_clone.resolve_name(&format!("alias{}", i))?;
                assert_eq!(resolved.len(), 1);
                assert_eq!(resolved[0].as_str(), &format!("index{}", i));
            }
            Ok::<(), anyhow::Error>(())
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await??;
    }
    
    let duration = start.elapsed();

    println!("Resolved 10000 aliases under load in {:?}", duration);
    assert!(duration.as_millis() < 5000); // Should complete within 5 seconds

    Ok(())
}

#[lexum_macros::tokio_test]
async fn test_alias_operations_mixed_workload() -> Result<()> {
    let (_temp_dir, manager) = create_test_manager().await?;
    
    // Create test indices
    for i in 0..500 {
        create_test_index(&manager, &format!("index{}", i)).await?;
    }

    // Mixed workload: create, update, delete operations
    let start = Instant::now();
    
    // Create initial aliases
    for i in 0..100 {
        let indices = vec![IndexName::new(&format!("index{}", i))];
        manager.create_alias(&format!("alias{}", i), indices, None)?;
    }

    // Update aliases (add more indices)
    for i in 0..100 {
        let new_indices = vec![IndexName::new(&format!("index{}", i + 100))];
        manager.add_indices_to_alias(&format!("alias{}", i), new_indices)?;
    }

    // Delete some aliases
    for i in 0..50 {
        manager.delete_alias(&format!("alias{}", i))?;
    }

    // Create new aliases
    for i in 0..50 {
        let indices = vec![IndexName::new(&format!("index{}", i + 200))];
        manager.create_alias(&format!("new_alias{}", i), indices, None)?;
    }
    
    let duration = start.elapsed();

    println!("Mixed workload completed in {:?}", duration);
    assert!(duration.as_millis() < 5000); // Should complete within 5 seconds

    // Verify final state
    let aliases = manager.list_aliases();
    assert_eq!(aliases.len(), 100); // 50 remaining + 50 new

    Ok(())
}