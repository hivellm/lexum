//! Alias functionality tests

use lexum_core::index::AliasManager;
use lexum_core::index::alias::{AliasAction, AliasOperationsRequest};
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
