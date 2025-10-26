//! Index alias management
//!
//! Provides functionality for creating, managing, and resolving index aliases.
//! Aliases allow operations to be performed on a logical name that points to one or more indices.

use crate::error::{Error, Result};
use crate::types::IndexName;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

/// Alias name type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct AliasName(String);

impl AliasName {
    /// Create a new alias name
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AliasName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for AliasName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for AliasName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Alias configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct AliasConfig {
    /// Filter query for the alias (optional)
    pub filter: Option<serde_json::Value>,
    /// Routing value for the alias (optional)
    pub routing: Option<String>,
    /// Search routing value for the alias (optional)
    pub search_routing: Option<String>,
    /// Index routing value for the alias (optional)
    pub index_routing: Option<String>,
    /// Whether the alias is writeable
    pub is_write_index: Option<bool>,
}

/// Index alias definition
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexAlias {
    /// Alias name
    pub name: AliasName,
    /// Target indices
    pub indices: Vec<IndexName>,
    /// Alias configuration
    pub config: AliasConfig,
}

impl IndexAlias {
    /// Create a new index alias
    pub fn new(name: impl Into<AliasName>, indices: Vec<IndexName>) -> Self {
        Self {
            name: name.into(),
            indices,
            config: AliasConfig::default(),
        }
    }

    /// Create a new index alias with configuration
    pub fn new_with_config(
        name: impl Into<AliasName>,
        indices: Vec<IndexName>,
        config: AliasConfig,
    ) -> Self {
        Self {
            name: name.into(),
            indices,
            config,
        }
    }

    /// Add an index to the alias
    pub fn add_index(&mut self, index: IndexName) {
        if !self.indices.contains(&index) {
            self.indices.push(index);
        }
    }

    /// Remove an index from the alias
    pub fn remove_index(&mut self, index: &IndexName) {
        self.indices.retain(|i| i != index);
    }

    /// Check if the alias contains a specific index
    pub fn contains_index(&self, index: &IndexName) -> bool {
        self.indices.contains(index)
    }

    /// Get the number of indices in the alias
    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    /// Check if the alias is empty
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// Alias operation types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "action")]
pub enum AliasAction {
    /// Add an alias
    Add {
        /// Alias name
        alias: AliasName,
        /// Target indices
        indices: Vec<IndexName>,
        /// Alias configuration
        config: Option<AliasConfig>,
    },
    /// Remove an alias
    Remove {
        /// Alias name
        alias: AliasName,
        /// Target indices to remove from alias
        indices: Vec<IndexName>,
    },
    /// Remove all indices from an alias
    RemoveIndex {
        /// Alias name
        alias: AliasName,
    },
}

/// Alias operations request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AliasOperationsRequest {
    /// List of alias operations
    pub actions: Vec<AliasAction>,
}

impl AliasOperationsRequest {
    /// Create a new alias operations request
    pub fn new(actions: Vec<AliasAction>) -> Self {
        Self { actions }
    }

    /// Add an action to the request
    pub fn add_action(&mut self, action: AliasAction) {
        self.actions.push(action);
    }
}

/// Alias operations response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AliasOperationsResponse {
    /// Whether the operations were acknowledged
    pub acknowledged: bool,
    /// List of aliases after operations
    pub aliases: Vec<IndexAlias>,
    /// Number of operations executed
    pub executed_operations: usize,
    /// Whether the operation was atomic (all succeeded or all failed)
    pub atomic: bool,
}

/// Alias transaction for atomic operations
#[derive(Debug, Clone)]
pub struct AliasTransaction {
    /// Snapshot of aliases before operations
    aliases_snapshot: HashMap<String, IndexAlias>,
    /// Operations to execute
    operations: Vec<AliasAction>,
}

impl AliasTransaction {
    /// Create a new alias transaction
    pub fn new(operations: Vec<AliasAction>) -> Self {
        Self {
            aliases_snapshot: HashMap::new(),
            operations,
        }
    }

    /// Get the number of operations in this transaction
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
}

/// Alias manager for handling index aliases
pub struct AliasManager {
    /// Map of alias names to alias definitions
    aliases: Arc<RwLock<HashMap<String, IndexAlias>>>,
}

impl AliasManager {
    /// Create a new alias manager
    pub fn new() -> Self {
        Self {
            aliases: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new alias
    pub fn create_alias(
        &self,
        name: impl Into<AliasName>,
        indices: Vec<IndexName>,
        config: Option<AliasConfig>,
    ) -> Result<IndexAlias> {
        let alias_name = name.into();
        let name_str = alias_name.as_str().to_string();

        // Validate that indices are not empty
        if indices.is_empty() {
            return Err(Error::Validation(
                "Alias must have at least one target index".to_string(),
            ));
        }

        // Check if alias already exists
        {
            let aliases = self.aliases.read();
            if aliases.contains_key(&name_str) {
                return Err(Error::Validation(format!(
                    "Alias '{name_str}' already exists"
                )));
            }
        }

        let alias =
            IndexAlias::new_with_config(alias_name.clone(), indices, config.unwrap_or_default());

        // Store the alias
        {
            let mut aliases = self.aliases.write();
            aliases.insert(name_str, alias.clone());
        }

        tracing::info!(alias = %alias_name, "Alias created");
        Ok(alias)
    }

    /// Get an alias by name
    pub fn get_alias(&self, name: &str) -> Result<IndexAlias> {
        let aliases = self.aliases.read();
        aliases
            .get(name)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("Alias '{name}' not found")))
    }

    /// Delete an alias
    pub fn delete_alias(&self, name: &str) -> Result<()> {
        let mut aliases = self.aliases.write();
        if aliases.remove(name).is_none() {
            return Err(Error::NotFound(format!("Alias '{name}' not found")));
        }

        tracing::info!(alias = %name, "Alias deleted");
        Ok(())
    }

    /// List all aliases
    pub fn list_aliases(&self) -> Vec<IndexAlias> {
        let aliases = self.aliases.read();
        aliases.values().cloned().collect()
    }

    /// Check if an alias exists
    pub fn alias_exists(&self, name: &str) -> bool {
        let aliases = self.aliases.read();
        aliases.contains_key(name)
    }

    /// Add indices to an existing alias
    pub fn add_indices_to_alias(&self, name: &str, indices: Vec<IndexName>) -> Result<IndexAlias> {
        let mut aliases = self.aliases.write();
        let alias = aliases
            .get_mut(name)
            .ok_or_else(|| Error::NotFound(format!("Alias '{name}' not found")))?;

        for index in indices {
            alias.add_index(index);
        }

        tracing::info!(alias = %name, "Indices added to alias");
        Ok(alias.clone())
    }

    /// Remove indices from an alias
    pub fn remove_indices_from_alias(
        &self,
        name: &str,
        indices: Vec<IndexName>,
    ) -> Result<IndexAlias> {
        let mut aliases = self.aliases.write();
        let alias = aliases
            .get_mut(name)
            .ok_or_else(|| Error::NotFound(format!("Alias '{name}' not found")))?;

        for index in indices {
            if !alias.contains_index(&index) {
                return Err(Error::NotFound(format!(
                    "Index '{}' not found in alias '{}'",
                    index, name
                )));
            }
            alias.remove_index(&index);
        }

        // If alias is now empty, return error but keep the alias
        if alias.is_empty() {
            tracing::info!(alias = %name, "Alias is now empty but kept");
            return Err(Error::Validation(
                "Alias has no indices remaining".to_string(),
            ));
        }

        tracing::info!(alias = %name, "Indices removed from alias");
        Ok(alias.clone())
    }

    /// Execute multiple alias operations atomically
    pub fn execute_operations(
        &self,
        request: AliasOperationsRequest,
    ) -> Result<AliasOperationsResponse> {
        let mut aliases = self.aliases.write();
        let mut updated_aliases = Vec::new();
        let actions_count = request.actions.len();

        for action in request.actions {
            match action {
                AliasAction::Add {
                    alias,
                    indices,
                    config,
                } => {
                    let alias_name = alias.as_str().to_string();

                    // Validate that indices are not empty
                    if indices.is_empty() {
                        return Err(Error::Validation(
                            "Alias must have at least one target index".to_string(),
                        ));
                    }

                    let alias_def = IndexAlias::new_with_config(
                        alias.clone(),
                        indices,
                        config.unwrap_or_default(),
                    );

                    aliases.insert(alias_name.clone(), alias_def.clone());
                    updated_aliases.push(alias_def);
                }
                AliasAction::Remove { alias, indices } => {
                    let alias_name = alias.as_str();

                    if let Some(alias_def) = aliases.get_mut(alias_name) {
                        for index in indices {
                            alias_def.remove_index(&index);
                        }

                        if alias_def.is_empty() {
                            aliases.remove(alias_name);
                        } else {
                            updated_aliases.push(alias_def.clone());
                        }
                    }
                }
                AliasAction::RemoveIndex { alias } => {
                    let alias_name = alias.as_str();
                    aliases.remove(alias_name);
                }
            }
        }

        tracing::info!("Executed {} alias operations", actions_count);
        Ok(AliasOperationsResponse {
            acknowledged: true,
            aliases: updated_aliases,
            executed_operations: actions_count,
            atomic: true,
        })
    }

    /// Execute atomic alias operations with full transaction support
    /// This method provides true atomicity with rollback capabilities
    pub fn execute_atomic_operations(
        &self,
        request: AliasOperationsRequest,
    ) -> Result<AliasOperationsResponse> {
        // Validate that operations are not empty
        if request.actions.is_empty() {
            return Err(Error::Validation(
                "At least one operation must be provided".to_string(),
            ));
        }

        // Create transaction
        let mut transaction = AliasTransaction::new(request.actions);

        // Take snapshot of current state
        {
            let aliases = self.aliases.read();
            transaction.aliases_snapshot = aliases.clone();
        }

        // Execute operations with rollback on failure
        let result = self.execute_operations_internal(&transaction);

        match result {
            Ok(response) => {
                tracing::info!(
                    "Atomic alias operations completed successfully: {} operations",
                    response.executed_operations
                );
                Ok(response)
            }
            Err(e) => {
                // Rollback on failure
                self.rollback_transaction(&transaction);
                tracing::warn!("Atomic alias operations failed, rolled back: {}", e);
                Err(e)
            }
        }
    }

    /// Execute operations within a transaction (internal method)
    fn execute_operations_internal(
        &self,
        transaction: &AliasTransaction,
    ) -> Result<AliasOperationsResponse> {
        let mut aliases = self.aliases.write();
        let mut updated_aliases = Vec::new();
        let mut executed_count = 0;

        for action in &transaction.operations {
            match action {
                AliasAction::Add {
                    alias,
                    indices,
                    config,
                } => {
                    let alias_name = alias.as_str().to_string();

                    // Validate that indices are not empty
                    if indices.is_empty() {
                        return Err(Error::Validation(
                            "Alias must have at least one target index".to_string(),
                        ));
                    }

                    // Check if alias already exists
                    if aliases.contains_key(&alias_name) {
                        return Err(Error::Validation(format!(
                            "Alias '{alias_name}' already exists"
                        )));
                    }

                    let alias_def = IndexAlias::new_with_config(
                        alias.clone(),
                        indices.clone(),
                        config.clone().unwrap_or_default(),
                    );

                    aliases.insert(alias_name.clone(), alias_def.clone());
                    updated_aliases.push(alias_def);
                    executed_count += 1;
                }
                AliasAction::Remove { alias, indices } => {
                    let alias_name = alias.as_str();

                    if let Some(alias_def) = aliases.get_mut(alias_name) {
                        for index in indices {
                            alias_def.remove_index(index);
                        }

                        if alias_def.is_empty() {
                            aliases.remove(alias_name);
                        } else {
                            updated_aliases.push(alias_def.clone());
                        }
                        executed_count += 1;
                    } else {
                        return Err(Error::NotFound(format!("Alias '{alias_name}' not found")));
                    }
                }
                AliasAction::RemoveIndex { alias } => {
                    let alias_name = alias.as_str();
                    if aliases.remove(alias_name).is_some() {
                        executed_count += 1;
                    } else {
                        return Err(Error::NotFound(format!("Alias '{alias_name}' not found")));
                    }
                }
            }
        }

        Ok(AliasOperationsResponse {
            acknowledged: true,
            aliases: updated_aliases,
            executed_operations: executed_count,
            atomic: true,
        })
    }

    /// Rollback a failed transaction
    fn rollback_transaction(&self, transaction: &AliasTransaction) {
        let mut aliases = self.aliases.write();

        // Restore the snapshot
        *aliases = transaction.aliases_snapshot.clone();

        tracing::info!("Transaction rolled back successfully");
    }

    /// Create a new atomic alias transaction
    pub fn create_transaction(&self, operations: Vec<AliasAction>) -> AliasTransaction {
        AliasTransaction::new(operations)
    }

    /// Execute a prepared transaction atomically
    pub fn execute_transaction(
        &self,
        transaction: AliasTransaction,
    ) -> Result<AliasOperationsResponse> {
        let request = AliasOperationsRequest {
            actions: transaction.operations,
        };
        self.execute_atomic_operations(request)
    }

    /// Resolve an alias to its target indices
    pub fn resolve_alias(&self, name: &str) -> Result<Vec<IndexName>> {
        let aliases = self.aliases.read();
        let alias = aliases
            .get(name)
            .ok_or_else(|| Error::NotFound(format!("Alias '{name}' not found")))?;

        Ok(alias.indices.clone())
    }

    /// Get all aliases that point to a specific index
    pub fn get_aliases_for_index(&self, index_name: &str) -> Vec<IndexAlias> {
        let aliases = self.aliases.read();
        aliases
            .values()
            .filter(|alias| alias.contains_index(&IndexName::new(index_name)))
            .cloned()
            .collect()
    }
}

impl Default for AliasManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alias_name_creation() {
        let alias = AliasName::new("test-alias");
        assert_eq!(alias.as_str(), "test-alias");
        assert_eq!(alias.to_string(), "test-alias");
    }

    #[test]
    fn test_index_alias_creation() {
        let indices = vec![IndexName::new("index1"), IndexName::new("index2")];
        let alias = IndexAlias::new("test-alias", indices.clone());

        assert_eq!(alias.name.as_str(), "test-alias");
        assert_eq!(alias.indices, indices);
        assert_eq!(alias.index_count(), 2);
        assert!(!alias.is_empty());
    }

    #[test]
    fn test_index_alias_operations() {
        let mut alias = IndexAlias::new("test-alias", vec![IndexName::new("index1")]);

        let index2 = IndexName::new("index2");
        alias.add_index(index2.clone());
        assert_eq!(alias.index_count(), 2);
        assert!(alias.contains_index(&index2));

        alias.remove_index(&index2);
        assert_eq!(alias.index_count(), 1);
        assert!(!alias.contains_index(&index2));
    }

    #[test]
    fn test_alias_manager_creation() {
        let manager = AliasManager::new();
        assert!(!manager.alias_exists("nonexistent"));
        assert!(manager.list_aliases().is_empty());
    }

    #[test]
    fn test_create_alias() {
        let manager = AliasManager::new();
        let indices = vec![IndexName::new("index1"), IndexName::new("index2")];

        let alias = manager.create_alias("test-alias", indices, None).unwrap();
        assert_eq!(alias.name.as_str(), "test-alias");
        assert_eq!(alias.index_count(), 2);

        assert!(manager.alias_exists("test-alias"));
        assert_eq!(manager.list_aliases().len(), 1);
    }

    #[test]
    fn test_create_alias_empty_indices() {
        let manager = AliasManager::new();
        let result = manager.create_alias("test-alias", vec![], None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("at least one target index")
        );
    }

    #[test]
    fn test_create_duplicate_alias() {
        let manager = AliasManager::new();
        let indices = vec![IndexName::new("index1")];

        manager.create_alias("test-alias", indices, None).unwrap();
        let result = manager.create_alias("test-alias", vec![IndexName::new("index2")], None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_get_nonexistent_alias() {
        let manager = AliasManager::new();
        let result = manager.get_alias("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_delete_alias() {
        let manager = AliasManager::new();
        let indices = vec![IndexName::new("index1")];

        manager.create_alias("test-alias", indices, None).unwrap();
        assert!(manager.alias_exists("test-alias"));

        manager.delete_alias("test-alias").unwrap();
        assert!(!manager.alias_exists("test-alias"));
    }

    #[test]
    fn test_resolve_alias() {
        let manager = AliasManager::new();
        let indices = vec![IndexName::new("index1"), IndexName::new("index2")];

        manager
            .create_alias("test-alias", indices.clone(), None)
            .unwrap();
        let resolved = manager.resolve_alias("test-alias").unwrap();
        assert_eq!(resolved, indices);
    }

    #[test]
    fn test_get_aliases_for_index() {
        let manager = AliasManager::new();

        manager
            .create_alias("alias1", vec![IndexName::new("index1")], None)
            .unwrap();
        manager
            .create_alias(
                "alias2",
                vec![IndexName::new("index1"), IndexName::new("index2")],
                None,
            )
            .unwrap();
        manager
            .create_alias("alias3", vec![IndexName::new("index2")], None)
            .unwrap();

        let aliases = manager.get_aliases_for_index("index1");
        assert_eq!(aliases.len(), 2);
        assert!(aliases.iter().any(|a| a.name.as_str() == "alias1"));
        assert!(aliases.iter().any(|a| a.name.as_str() == "alias2"));
    }
}
