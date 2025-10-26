//! Alias functionality tests

use lexum_core::index::AliasManager;
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
