//! Alias functionality tests

use lexum_core::index::AliasManager;
use lexum_core::index::alias::{AliasAction, AliasOperationsRequest, AliasConfig, AliasName};
use lexum_core::types::IndexName;

#[test]
fn test_alias_creation() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1"), IndexName::new("index2")];

    let alias = manager.create_alias("test-alias", indices, None).unwrap();
    assert_eq!(alias.name.as_str(), "test-alias");
    assert_eq!(alias.index_count(), 2);

    assert!(manager.alias_exists("test-alias"));
    assert_eq!(manager.list_aliases().len(), 1);
}

#[test]
fn test_alias_operations() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1")];

    // Create alias
    manager.create_alias("test-alias", indices, None).unwrap();

    // Add more indices
    let new_indices = vec![IndexName::new("index2"), IndexName::new("index3")];
    let updated_alias = manager
        .add_indices_to_alias("test-alias", new_indices)
        .unwrap();
    assert_eq!(updated_alias.index_count(), 3);

    // Remove some indices
    let remove_indices = vec![IndexName::new("index2")];
    let final_alias = manager
        .remove_indices_from_alias("test-alias", remove_indices)
        .unwrap();
    assert_eq!(final_alias.index_count(), 2);
}

#[test]
fn test_alias_resolution() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1"), IndexName::new("index2")];

    manager
        .create_alias("test-alias", indices.clone(), None)
        .unwrap();

    let resolved = manager.resolve_alias("test-alias").unwrap();
    assert_eq!(resolved, indices);
}

#[test]
fn test_alias_deletion() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1")];

    manager.create_alias("test-alias", indices, None).unwrap();
    assert!(manager.alias_exists("test-alias"));

    manager.delete_alias("test-alias").unwrap();
    assert!(!manager.alias_exists("test-alias"));
}

#[test]
fn test_atomic_alias_operations_success() {
    let manager = AliasManager::new();
    let indices1 = vec![IndexName::new("index1"), IndexName::new("index2")];
    let indices2 = vec![IndexName::new("index3")];

    // Create initial aliases
    manager.create_alias("alias1", indices1, None).unwrap();
    manager.create_alias("alias2", indices2, None).unwrap();

    // Prepare atomic operations
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
    ];

    let request = AliasOperationsRequest::new(operations);
    let response = manager.execute_atomic_operations(request).unwrap();

    assert!(response.acknowledged);
    assert!(response.atomic);
    assert_eq!(response.executed_operations, 2);
    assert_eq!(response.aliases.len(), 2); // alias1 (updated) and alias3 (new)

    // Verify state
    assert!(manager.alias_exists("alias1"));
    assert!(manager.alias_exists("alias2"));
    assert!(manager.alias_exists("alias3"));

    // Check alias1 only has index2 now
    let alias1 = manager.get_alias("alias1").unwrap();
    assert_eq!(alias1.index_count(), 1);
    assert!(alias1.contains_index(&IndexName::new("index2")));
}

#[test]
fn test_atomic_alias_operations_rollback() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1")];

    // Create initial alias
    manager.create_alias("alias1", indices, None).unwrap();

    // Prepare operations that will fail (trying to add alias with same name)
    let operations = vec![
        AliasAction::Add {
            alias: "alias2".into(),
            indices: vec![IndexName::new("index2")],
            config: None,
        },
        AliasAction::Add {
            alias: "alias1".into(), // This will fail - alias already exists
            indices: vec![IndexName::new("index3")],
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

    let alias1 = manager.get_alias("alias1").unwrap();
    assert_eq!(alias1.index_count(), 1);
    assert!(alias1.contains_index(&IndexName::new("index1")));
}

#[test]
fn test_atomic_alias_operations_empty_indices() {
    let manager = AliasManager::new();

    // Prepare operations with empty indices (should fail)
    let operations = vec![AliasAction::Add {
        alias: "alias1".into(),
        indices: vec![], // Empty indices should fail
        config: None,
    }];

    let request = AliasOperationsRequest::new(operations);
    let result = manager.execute_atomic_operations(request);

    // Should fail
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must have at least one target index")
    );
}

#[test]
fn test_atomic_alias_operations_remove_nonexistent() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1")];

    // Create initial alias
    manager.create_alias("alias1", indices, None).unwrap();

    // Prepare operations that will fail (trying to remove from non-existent alias)
    let operations = vec![AliasAction::Remove {
        alias: "nonexistent-alias".into(),
        indices: vec![IndexName::new("index1")],
    }];

    let request = AliasOperationsRequest::new(operations);
    let result = manager.execute_atomic_operations(request);

    // Should fail and rollback
    assert!(result.is_err());

    // Verify state is unchanged
    assert!(manager.alias_exists("alias1"));
    let alias1 = manager.get_alias("alias1").unwrap();
    assert_eq!(alias1.index_count(), 1);
}

#[test]
fn test_alias_transaction_creation() {
    let manager = AliasManager::new();
    let operations = vec![AliasAction::Add {
        alias: "alias1".into(),
        indices: vec![IndexName::new("index1")],
        config: None,
    }];

    let transaction = manager.create_transaction(operations);
    assert_eq!(transaction.operation_count(), 1);
}

#[test]
fn test_alias_transaction_execution() {
    let manager = AliasManager::new();
    let operations = vec![
        AliasAction::Add {
            alias: "alias1".into(),
            indices: vec![IndexName::new("index1")],
            config: None,
        },
        AliasAction::Add {
            alias: "alias2".into(),
            indices: vec![IndexName::new("index2")],
            config: None,
        },
    ];

    let transaction = manager.create_transaction(operations);
    let response = manager.execute_transaction(transaction).unwrap();

    assert!(response.acknowledged);
    assert!(response.atomic);
    assert_eq!(response.executed_operations, 2);
    assert!(manager.alias_exists("alias1"));
    assert!(manager.alias_exists("alias2"));
}

#[test]
fn test_atomic_alias_operations_complex_scenario() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1"), IndexName::new("index2")];

    // Create initial alias
    manager.create_alias("alias1", indices, None).unwrap();

    // Complex atomic operations: add, remove, and remove_index
    let operations = vec![
        AliasAction::Add {
            alias: "alias2".into(),
            indices: vec![IndexName::new("index3")],
            config: None,
        },
        AliasAction::Add {
            alias: "alias3".into(),
            indices: vec![IndexName::new("index4"), IndexName::new("index5")],
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
    let response = manager.execute_atomic_operations(request).unwrap();

    assert!(response.acknowledged);
    assert!(response.atomic);
    assert_eq!(response.executed_operations, 4);

    // Verify final state
    assert!(manager.alias_exists("alias1")); // Still exists but with only index2
    assert!(!manager.alias_exists("alias2")); // Removed completely
    assert!(manager.alias_exists("alias3")); // New alias with index4 and index5

    let alias1 = manager.get_alias("alias1").unwrap();
    assert_eq!(alias1.index_count(), 1);
    assert!(alias1.contains_index(&IndexName::new("index2")));

    let alias3 = manager.get_alias("alias3").unwrap();
    assert_eq!(alias3.index_count(), 2);
    assert!(alias3.contains_index(&IndexName::new("index4")));
    assert!(alias3.contains_index(&IndexName::new("index5")));
}

// ============================================================================
// Enhanced Alias Manager Tests
// ============================================================================

#[test]
fn test_alias_creation_with_config() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1"), IndexName::new("index2")];
    let config = AliasConfig {
        filter: Some(serde_json::json!({"term": {"status": "active"}})),
        routing: Some("user1".to_string()),
        search_routing: Some("user1".to_string()),
        index_routing: Some("user1".to_string()),
        is_write_index: Some(true),
    };

    let alias = manager.create_alias("test-alias", indices, Some(config)).unwrap();
    
    assert_eq!(alias.name.as_str(), "test-alias");
    assert_eq!(alias.index_count(), 2);
    assert!(alias.config.filter.is_some());
    assert_eq!(alias.config.routing, Some("user1".to_string()));
    assert_eq!(alias.config.is_write_index, Some(true));
}

#[test]
fn test_alias_creation_duplicate_name() {
    let manager = AliasManager::new();
    let indices1 = vec![IndexName::new("index1")];
    let indices2 = vec![IndexName::new("index2")];

    // Create first alias
    manager.create_alias("test-alias", indices1, None).unwrap();

    // Try to create alias with same name
    let result = manager.create_alias("test-alias", indices2, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_alias_creation_empty_indices() {
    let manager = AliasManager::new();
    let result = manager.create_alias("test-alias", vec![], None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must have at least one target index"));
}

#[test]
fn test_alias_get_nonexistent() {
    let manager = AliasManager::new();
    let result = manager.get_alias("nonexistent-alias");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_alias_delete_nonexistent() {
    let manager = AliasManager::new();
    let result = manager.delete_alias("nonexistent-alias");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_alias_list_empty() {
    let manager = AliasManager::new();
    let aliases = manager.list_aliases();
    assert!(aliases.is_empty());
}

#[test]
fn test_alias_list_multiple() {
    let manager = AliasManager::new();
    
    // Create multiple aliases
    manager.create_alias("alias1", vec![IndexName::new("index1")], None).unwrap();
    manager.create_alias("alias2", vec![IndexName::new("index2")], None).unwrap();
    manager.create_alias("alias3", vec![IndexName::new("index3")], None).unwrap();

    let aliases = manager.list_aliases();
    assert_eq!(aliases.len(), 3);
    
    let alias_names: Vec<&str> = aliases.iter().map(|a| a.name.as_str()).collect();
    assert!(alias_names.contains(&"alias1"));
    assert!(alias_names.contains(&"alias2"));
    assert!(alias_names.contains(&"alias3"));
}

#[test]
fn test_alias_add_indices_to_nonexistent() {
    let manager = AliasManager::new();
    let result = manager.add_indices_to_alias("nonexistent-alias", vec![IndexName::new("index1")]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_alias_remove_indices_from_nonexistent() {
    let manager = AliasManager::new();
    let result = manager.remove_indices_from_alias("nonexistent-alias", vec![IndexName::new("index1")]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_alias_remove_nonexistent_indices() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1")];
    manager.create_alias("test-alias", indices, None).unwrap();

    // Try to remove non-existent indices
    let result = manager.remove_indices_from_alias("test-alias", vec![IndexName::new("nonexistent-index")]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_alias_remove_all_indices() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1")];
    manager.create_alias("test-alias", indices, None).unwrap();

    // Remove all indices
    let result = manager.remove_indices_from_alias("test-alias", vec![IndexName::new("index1")]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no indices"));

    // Alias should still exist but be empty
    assert!(manager.alias_exists("test-alias"));
    let alias = manager.get_alias("test-alias").unwrap();
    assert!(alias.is_empty());
}

#[test]
fn test_alias_contains_index() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1"), IndexName::new("index2")];
    let alias = manager.create_alias("test-alias", indices, None).unwrap();

    assert!(alias.contains_index(&IndexName::new("index1")));
    assert!(alias.contains_index(&IndexName::new("index2")));
    assert!(!alias.contains_index(&IndexName::new("index3")));
}

#[test]
fn test_alias_index_count() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1"), IndexName::new("index2"), IndexName::new("index3")];
    let alias = manager.create_alias("test-alias", indices, None).unwrap();

    assert_eq!(alias.index_count(), 3);
}

#[test]
fn test_alias_is_empty() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1")];
    let alias = manager.create_alias("test-alias", indices, None).unwrap();

    assert!(!alias.is_empty());
}

#[test]
fn test_alias_remove_index() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1"), IndexName::new("index2")];
    let mut alias = manager.create_alias("test-alias", indices, None).unwrap();

    alias.remove_index(&IndexName::new("index1"));
    assert_eq!(alias.index_count(), 1);
    assert!(!alias.contains_index(&IndexName::new("index1")));
    assert!(alias.contains_index(&IndexName::new("index2")));
}

#[test]
fn test_alias_remove_nonexistent_index() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1")];
    let mut alias = manager.create_alias("test-alias", indices, None).unwrap();

    // Try to remove non-existent index
    alias.remove_index(&IndexName::new("nonexistent-index"));
    assert_eq!(alias.index_count(), 1); // Should remain unchanged
}

#[test]
fn test_alias_operations_request_validation() {
    let manager = AliasManager::new();

    // Test empty operations
    let operations = vec![];
    let request = AliasOperationsRequest::new(operations);
    let result = manager.execute_atomic_operations(request);
    assert!(result.is_err());

    // Test operations with empty indices
    let operations = vec![AliasAction::Add {
        alias: "test-alias".into(),
        indices: vec![],
        config: None,
    }];
    let request = AliasOperationsRequest::new(operations);
    let result = manager.execute_atomic_operations(request);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must have at least one target index"));
}

#[test]
fn test_alias_transaction_validation() {
    let manager = AliasManager::new();

    // Test empty transaction
    let operations = vec![];
    let transaction = manager.create_transaction(operations);
    assert_eq!(transaction.operation_count(), 0);

    // Test transaction with operations
    let operations = vec![
        AliasAction::Add {
            alias: "alias1".into(),
            indices: vec![IndexName::new("index1")],
            config: None,
        },
        AliasAction::Add {
            alias: "alias2".into(),
            indices: vec![IndexName::new("index2")],
            config: None,
        },
    ];
    let transaction = manager.create_transaction(operations);
    assert_eq!(transaction.operation_count(), 2);
}

#[test]
fn test_alias_transaction_execution_failure() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1")];

    // Create initial alias
    manager.create_alias("alias1", indices, None).unwrap();

    // Create transaction that will fail (duplicate alias)
    let operations = vec![
        AliasAction::Add {
            alias: "alias2".into(),
            indices: vec![IndexName::new("index2")],
            config: None,
        },
        AliasAction::Add {
            alias: "alias1".into(), // This will fail - alias already exists
            indices: vec![IndexName::new("index3")],
            config: None,
        },
    ];

    let transaction = manager.create_transaction(operations);
    let result = manager.execute_transaction(transaction);

    // Should fail and rollback
    assert!(result.is_err());

    // Verify state is unchanged
    assert!(manager.alias_exists("alias1"));
    assert!(!manager.alias_exists("alias2"));
}

#[test]
fn test_alias_concurrent_operations() {
    use std::sync::Arc;
    use std::thread;

    let manager = Arc::new(AliasManager::new());
    let mut handles = vec![];

    // Spawn multiple threads to create aliases concurrently
    for i in 0..10 {
        let manager_clone = manager.clone();
        let handle = thread::spawn(move || {
            let indices = vec![IndexName::new(&format!("index{}", i))];
            manager_clone.create_alias(format!("alias{}", i), indices, None)
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_ok());
    }

    // Verify all aliases were created
    let aliases = manager.list_aliases();
    assert_eq!(aliases.len(), 10);
}

#[test]
fn test_alias_name_validation() {
    let manager = AliasManager::new();
    let indices = vec![IndexName::new("index1")];

    // Test valid alias names
    let valid_names = vec![
        "valid-alias",
        "alias123",
        "alias_with_underscore",
        "alias-with-dash",
        "ALIAS",
        "alias123",
    ];

    for name in valid_names {
        let result = manager.create_alias(name, indices.clone(), None);
        assert!(result.is_ok(), "Failed to create alias with name: {}", name);
        manager.delete_alias(name).unwrap();
    }
}

#[test]
fn test_alias_config_defaults() {
    let config = AliasConfig::default();
    assert!(config.filter.is_none());
    assert!(config.routing.is_none());
    assert!(config.search_routing.is_none());
    assert!(config.index_routing.is_none());
    assert!(config.is_write_index.is_none());
}

#[test]
fn test_alias_config_serialization() {
    let config = AliasConfig {
        filter: Some(serde_json::json!({"term": {"status": "active"}})),
        routing: Some("user1".to_string()),
        search_routing: Some("user1".to_string()),
        index_routing: Some("user1".to_string()),
        is_write_index: Some(true),
    };

    // Test serialization
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("filter"));
    assert!(json.contains("routing"));
    assert!(json.contains("is_write_index"));

    // Test deserialization
    let deserialized: AliasConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.filter, config.filter);
    assert_eq!(deserialized.routing, config.routing);
    assert_eq!(deserialized.is_write_index, config.is_write_index);
}

#[test]
fn test_alias_name_serialization() {
    let alias_name = AliasName::new("test-alias");
    
    // Test serialization
    let json = serde_json::to_string(&alias_name).unwrap();
    assert_eq!(json, "\"test-alias\"");

    // Test deserialization
    let deserialized: AliasName = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.as_str(), "test-alias");
}

#[test]
fn test_alias_name_display() {
    let alias_name = AliasName::new("test-alias");
    assert_eq!(format!("{}", alias_name), "test-alias");
}

#[test]
fn test_alias_name_from_string() {
    let alias_name: AliasName = "test-alias".into();
    assert_eq!(alias_name.as_str(), "test-alias");

    let alias_name: AliasName = "test-alias".to_string().into();
    assert_eq!(alias_name.as_str(), "test-alias");
}

#[test]
fn test_alias_name_clone() {
    let alias_name1 = AliasName::new("test-alias");
    let alias_name2 = alias_name1.clone();
    assert_eq!(alias_name1.as_str(), alias_name2.as_str());
}

#[test]
fn test_alias_name_hash() {
    use std::collections::HashSet;
    
    let mut set = HashSet::new();
    let alias_name1 = AliasName::new("test-alias");
    let alias_name2 = AliasName::new("test-alias");
    
    set.insert(alias_name1);
    set.insert(alias_name2);
    
    // Should only have one entry since they're equal
    assert_eq!(set.len(), 1);
}

#[test]
fn test_alias_name_equality() {
    let alias_name1 = AliasName::new("test-alias");
    let alias_name2 = AliasName::new("test-alias");
    let alias_name3 = AliasName::new("different-alias");
    
    assert_eq!(alias_name1, alias_name2);
    assert_ne!(alias_name1, alias_name3);
}
