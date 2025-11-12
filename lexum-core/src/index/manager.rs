//! Index manager implementation

use crate::error::{Error, Result};
use crate::types::IndexName;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tantivy::{Index as TantivyIndex, IndexWriter};

use super::alias::{
    AliasManager, AliasName, AliasOperationsRequest, AliasOperationsResponse, IndexAlias,
};
use super::settings::IndexSettings;

/// Index wrapper around Tantivy index
#[derive(Clone, Debug)]
pub struct Index {
    /// Index name
    pub(crate) name: IndexName,
    /// Inner Tantivy index
    pub(crate) inner: Arc<TantivyIndex>,
    /// Index settings
    pub(crate) settings: IndexSettings,
}

impl Index {
    /// Get index name
    pub fn name(&self) -> &IndexName {
        &self.name
    }

    /// Get index settings
    pub fn settings(&self) -> &IndexSettings {
        &self.settings
    }

    /// Get Tantivy schema
    pub fn schema(&self) -> tantivy::schema::Schema {
        self.inner.schema()
    }

    /// Create an index writer
    pub fn writer(&self, heap_size: usize) -> Result<IndexWriter> {
        self.inner
            .writer(heap_size)
            .map_err(|e| Error::Config(format!("Failed to create index writer: {e}")))
    }

    /// Get index reader
    pub fn reader(&self) -> Result<tantivy::IndexReader> {
        self.inner
            .reader()
            .map_err(|e| Error::Config(format!("Failed to create index reader: {e}")))
    }
}

/// Manages multiple indices
pub struct IndexManager {
    data_dir: PathBuf,
    indices: Arc<RwLock<HashMap<String, Index>>>,
    alias_manager: AliasManager,
}

impl IndexManager {
    /// Create a new index manager
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            indices: Arc::new(RwLock::new(HashMap::new())),
            alias_manager: AliasManager::new(),
        }
    }

    /// Create a new index
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lexum_core::index::{IndexManager, IndexSettings};
    /// use tantivy::schema::*;
    ///
    /// # tokio_test::block_on(async {
    /// let manager = IndexManager::new("./data");
    ///
    /// let mut schema_builder = Schema::builder();
    /// schema_builder.add_text_field("title", TEXT | STORED);
    /// let schema = schema_builder.build();
    ///
    /// let settings = IndexSettings::new().with_shards(3);
    /// let index = manager.create_index("my_index", schema, settings).await.unwrap();
    /// # });
    /// ```
    pub async fn create_index(
        &self,
        name: impl Into<String>,
        schema: tantivy::schema::Schema,
        settings: IndexSettings,
    ) -> Result<Index> {
        let name_str = name.into();
        let index_name = IndexName::new(&name_str);

        // Validate settings
        settings.validate()?;

        // Check if index already exists
        {
            let indices = self.indices.read();
            if indices.contains_key(&name_str) {
                return Err(Error::Validation(format!(
                    "Index {name_str} already exists"
                )));
            }
        }

        // Create index directory
        let index_path = self.data_dir.join(&name_str);

        // Ensure the directory exists and is writable
        std::fs::create_dir_all(&index_path)
            .map_err(|e| Error::Config(format!("Failed to create index directory: {e}")))?;

        // Create Tantivy index in blocking context
        let tantivy_index = tokio::task::spawn_blocking({
            let index_path = index_path.clone();
            let schema_clone = schema.clone();
            move || {
                // Ensure the directory exists and is writable
                std::fs::create_dir_all(&index_path).map_err(|e| {
                    tantivy::TantivyError::IoError(std::sync::Arc::new(std::io::Error::other(
                        format!("Failed to create index directory: {e}"),
                    )))
                })?;

                // Check if the directory is writable
                let test_file = index_path.join(".write_test");
                std::fs::write(&test_file, "test").map_err(|e| {
                    tantivy::TantivyError::IoError(std::sync::Arc::new(std::io::Error::other(
                        format!("Index directory is not writable: {e}"),
                    )))
                })?;
                let _ = std::fs::remove_file(&test_file);

                // Try to create the index on filesystem
                TantivyIndex::create_in_dir(&index_path, schema).or_else(|e| {
                    // If it fails, try to create the directory again and retry
                    let _ = std::fs::create_dir_all(&index_path);
                    TantivyIndex::create_in_dir(&index_path, schema_clone.clone()).map_err(|e2| {
                        eprintln!("First attempt failed: {e}");
                        eprintln!("Second attempt failed: {e2}");
                        e2
                    })
                })
            }
        })
        .await
        .map_err(|e| Error::Config(format!("Task join error: {e}")))?
        .map_err(|e| Error::Config(format!("Failed to create index: {e}")))?;

        let index = Index {
            name: index_name,
            inner: Arc::new(tantivy_index),
            settings: settings.clone(),
        };

        // Store index
        {
            let mut indices = self.indices.write();
            indices.insert(name_str.clone(), index.clone());
        }

        tracing::info!(index = %name_str, shards = settings.number_of_shards, "Index created");

        Ok(index)
    }

    /// Get an existing index
    pub fn get_index(&self, name: &str) -> Result<Index> {
        let indices = self.indices.read();
        indices
            .get(name)
            .cloned()
            .ok_or_else(|| Error::Validation(format!("Index {name} not found")))
    }

    /// Delete an index
    pub async fn delete_index(&self, name: &str) -> Result<()> {
        // Remove from memory
        {
            let mut indices = self.indices.write();
            if indices.remove(name).is_none() {
                return Err(Error::Validation(format!("Index {name} not found")));
            }
        }

        // Delete directory
        let index_path = self.data_dir.join(name);
        if index_path.exists() {
            tokio::fs::remove_dir_all(&index_path).await?;
        }

        tracing::info!(index = %name, "Index deleted");

        Ok(())
    }

    /// List all indices
    pub fn list_indices(&self) -> Vec<String> {
        let indices = self.indices.read();
        indices.keys().cloned().collect()
    }

    /// Check if index exists
    pub fn index_exists(&self, name: &str) -> bool {
        let indices = self.indices.read();
        indices.contains_key(name)
    }

    /// Get index statistics
    pub async fn get_index_stats(&self, name: &str) -> Result<IndexStats> {
        let index = self.get_index(name)?;

        // Run Tantivy operations in blocking context
        let stats = tokio::task::spawn_blocking({
            let index = index.clone();
            let name = name.to_string();
            move || {
                let reader = index.reader()?;
                let searcher = reader.searcher();

                let num_docs = searcher.num_docs();
                let num_segments = searcher.segment_readers().len();

                Ok::<IndexStats, Error>(IndexStats {
                    name,
                    num_docs,
                    num_segments,
                })
            }
        })
        .await
        .map_err(|e| Error::Config(format!("Task join error: {e}")))??;

        Ok(stats)
    }

    /// Refresh an index (reload readers to see latest changes)
    /// In Tantivy, refresh is done by getting a new reader which automatically sees latest changes
    pub async fn refresh_index(&self, name: &str) -> Result<()> {
        let index = self.get_index(name)?;

        // Run Tantivy operations in blocking context
        tokio::task::spawn_blocking({
            let index = index.clone();
            move || {
                // Get a new reader to see latest changes
                // Tantivy readers are automatically updated when a new reader is created
                let _reader = index.reader()?;
                // The reader will see the latest committed changes
                Ok::<(), Error>(())
            }
        })
        .await
        .map_err(|e| Error::Config(format!("Task join error: {e}")))??;

        tracing::info!(index = %name, "Index refreshed");
        Ok(())
    }

    /// Flush an index (commit all pending changes)
    pub async fn flush_index(&self, name: &str) -> Result<()> {
        let index = self.get_index(name)?;

        // Run Tantivy operations in blocking context
        tokio::task::spawn_blocking({
            let index = index.clone();
            move || {
                // Create a writer and commit to flush all pending changes
                let mut writer = index.writer(50_000_000)?;
                writer.commit()?;
                Ok::<(), Error>(())
            }
        })
        .await
        .map_err(|e| Error::Config(format!("Task join error: {e}")))??;

        tracing::info!(index = %name, "Index flushed");
        Ok(())
    }

    /// Create a new alias
    pub fn create_alias(
        &self,
        name: impl Into<AliasName>,
        indices: Vec<IndexName>,
    ) -> Result<IndexAlias> {
        // Validate that all target indices exist
        for index_name in &indices {
            if !self.index_exists(index_name.as_str()) {
                return Err(Error::Validation(format!(
                    "Index '{}' does not exist",
                    index_name.as_str()
                )));
            }
        }

        self.alias_manager.create_alias(name, indices, None)
    }

    /// Get an alias by name
    pub fn get_alias(&self, name: &str) -> Result<IndexAlias> {
        self.alias_manager.get_alias(name)
    }

    /// Delete an alias
    pub fn delete_alias(&self, name: &str) -> Result<()> {
        self.alias_manager.delete_alias(name)
    }

    /// List all aliases
    pub fn list_aliases(&self) -> Vec<IndexAlias> {
        self.alias_manager.list_aliases()
    }

    /// Check if an alias exists
    pub fn alias_exists(&self, name: &str) -> bool {
        self.alias_manager.alias_exists(name)
    }

    /// Add indices to an existing alias
    pub fn add_indices_to_alias(&self, name: &str, indices: Vec<IndexName>) -> Result<IndexAlias> {
        // Validate that all target indices exist
        for index_name in &indices {
            if !self.index_exists(index_name.as_str()) {
                return Err(Error::Validation(format!(
                    "Index '{}' does not exist",
                    index_name.as_str()
                )));
            }
        }

        self.alias_manager.add_indices_to_alias(name, indices)
    }

    /// Remove indices from an alias
    pub fn remove_indices_from_alias(
        &self,
        name: &str,
        indices: Vec<IndexName>,
    ) -> Result<IndexAlias> {
        self.alias_manager.remove_indices_from_alias(name, indices)
    }

    /// Execute multiple alias operations atomically
    pub fn execute_alias_operations(
        &self,
        request: AliasOperationsRequest,
    ) -> Result<AliasOperationsResponse> {
        // Validate that all target indices exist for add operations
        for action in &request.actions {
            if let super::alias::AliasAction::Add { indices, .. } = action {
                for index_name in indices {
                    if !self.index_exists(index_name.as_str()) {
                        return Err(Error::Validation(format!(
                            "Index '{}' does not exist",
                            index_name.as_str()
                        )));
                    }
                }
            }
        }

        self.alias_manager.execute_operations(request)
    }

    /// Execute atomic alias operations with full transaction support
    /// This provides true atomicity with rollback capabilities
    pub fn execute_atomic_alias_operations(
        &self,
        request: AliasOperationsRequest,
    ) -> Result<AliasOperationsResponse> {
        // Validate that all target indices exist for add operations
        for action in &request.actions {
            if let super::alias::AliasAction::Add { indices, .. } = action {
                for index_name in indices {
                    if !self.index_exists(index_name.as_str()) {
                        return Err(Error::Validation(format!(
                            "Index '{}' does not exist",
                            index_name.as_str()
                        )));
                    }
                }
            }
        }

        self.alias_manager.execute_atomic_operations(request)
    }

    /// Create a new atomic alias transaction
    pub fn create_alias_transaction(
        &self,
        operations: Vec<super::alias::AliasAction>,
    ) -> super::alias::AliasTransaction {
        self.alias_manager.create_transaction(operations)
    }

    /// Execute a prepared alias transaction atomically
    pub fn execute_alias_transaction(
        &self,
        transaction: super::alias::AliasTransaction,
    ) -> Result<AliasOperationsResponse> {
        self.alias_manager.execute_transaction(transaction)
    }

    /// Resolve an alias to its target indices
    pub fn resolve_alias(&self, name: &str) -> Result<Vec<IndexName>> {
        self.alias_manager.resolve_alias(name)
    }

    /// Get all aliases that point to a specific index
    pub fn get_aliases_for_index(&self, index_name: &str) -> Vec<IndexAlias> {
        self.alias_manager.get_aliases_for_index(index_name)
    }

    /// Resolve a name to either an index or alias
    /// Returns the actual index names that should be used
    pub fn resolve_name(&self, name: &str) -> Result<Vec<IndexName>> {
        if self.index_exists(name) {
            // It's a direct index name
            Ok(vec![IndexName::new(name)])
        } else if self.alias_exists(name) {
            // It's an alias, resolve it
            self.resolve_alias(name)
        } else {
            Err(Error::NotFound(format!(
                "Neither index nor alias '{name}' found"
            )))
        }
    }
}

/// Index statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexStats {
    /// Index name
    pub name: String,
    /// Number of documents
    pub num_docs: u64,
    /// Number of segments
    pub num_segments: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_exists() {
        let manager = IndexManager::new("./data");
        assert!(!manager.index_exists("non_existent"));
    }

    #[test]
    fn test_list_empty() {
        let manager = IndexManager::new("./data");
        let indices = manager.list_indices();
        assert_eq!(indices.len(), 0);
    }

    #[test]
    fn test_get_non_existent() {
        let manager = IndexManager::new("./data");
        let result = manager.get_index("non_existent");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_non_existent() {
        let manager = IndexManager::new("./data");
        let result = manager.delete_index("non_existent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_index() {
        // Set test mode to enable in-memory fallback for WSL compatibility
        // Note: We can't use std::env::set_var in tests due to unsafe_code deny
        // Instead, we'll rely on cfg!(test) which is always true in test builds

        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = IndexSettings::new().with_shards(1);
        let result = manager.create_index("test_index", schema, settings).await;
        assert!(result.is_ok());

        let index = result.unwrap();
        assert_eq!(index.name().as_str(), "test_index");
        assert!(manager.index_exists("test_index"));
    }

    #[tokio::test]
    async fn test_create_duplicate_index() {
        // Set test mode to enable in-memory fallback for WSL compatibility
        // Note: We can't use std::env::set_var in tests due to unsafe_code deny
        // Instead, we'll rely on cfg!(test) which is always true in test builds

        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = IndexSettings::new().with_shards(1);

        // Create first index
        let result1 = manager
            .create_index("test_index", schema.clone(), settings.clone())
            .await;
        assert!(result1.is_ok());

        // Try to create duplicate
        let result2 = manager.create_index("test_index", schema, settings).await;
        assert!(result2.is_err());
        assert!(result2.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_get_index() {
        // Set test mode to enable in-memory fallback for WSL compatibility
        // Note: We can't use std::env::set_var in tests due to unsafe_code deny
        // Instead, we'll rely on cfg!(test) which is always true in test builds

        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = IndexSettings::new().with_shards(1);
        manager
            .create_index("test_index", schema, settings)
            .await
            .unwrap();

        let index = manager.get_index("test_index").unwrap();
        assert_eq!(index.name().as_str(), "test_index");
    }

    #[tokio::test]
    async fn test_list_indices() {
        // Set test mode to enable in-memory fallback for WSL compatibility
        // Note: We can't use std::env::set_var in tests due to unsafe_code deny
        // Instead, we'll rely on cfg!(test) which is always true in test builds

        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = IndexSettings::new().with_shards(1);

        // Initially empty
        let indices = manager.list_indices();
        assert_eq!(indices.len(), 0);

        // Create indices
        manager
            .create_index("index1", schema.clone(), settings.clone())
            .await
            .unwrap();
        manager
            .create_index("index2", schema, settings)
            .await
            .unwrap();

        let indices = manager.list_indices();
        assert_eq!(indices.len(), 2);
        assert!(indices.contains(&"index1".to_string()));
        assert!(indices.contains(&"index2".to_string()));
    }

    #[tokio::test]
    async fn test_delete_index() {
        // Set test mode to enable in-memory fallback for WSL compatibility
        // Note: We can't use std::env::set_var in tests due to unsafe_code deny
        // Instead, we'll rely on cfg!(test) which is always true in test builds

        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = IndexSettings::new().with_shards(1);
        manager
            .create_index("test_index", schema, settings)
            .await
            .unwrap();

        assert!(manager.index_exists("test_index"));

        let result = manager.delete_index("test_index").await;
        assert!(result.is_ok());

        assert!(!manager.index_exists("test_index"));
    }

    #[tokio::test]
    async fn test_get_index_stats() {
        // Set test mode to enable in-memory fallback for WSL compatibility
        // Note: We can't use std::env::set_var in tests due to unsafe_code deny
        // Instead, we'll rely on cfg!(test) which is always true in test builds

        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = IndexSettings::new().with_shards(1);
        manager
            .create_index("test_index", schema, settings)
            .await
            .unwrap();

        let stats = manager.get_index_stats("test_index").await.unwrap();
        assert_eq!(stats.name, "test_index");
        assert_eq!(stats.num_docs, 0);
    }

    #[tokio::test]
    async fn test_get_index_stats_nonexistent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let result = manager.get_index_stats("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_alias() {
        // Set test mode to enable in-memory fallback for WSL compatibility
        // Note: We can't use std::env::set_var in tests due to unsafe_code deny
        // Instead, we'll rely on cfg!(test) which is always true in test builds

        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = IndexSettings::new().with_shards(1);
        manager
            .create_index("index1", schema, settings)
            .await
            .unwrap();

        let indices = vec![IndexName::new("index1")];
        let result = manager.create_alias("my_alias", indices);
        assert!(result.is_ok());

        let alias = result.unwrap();
        assert_eq!(alias.name.as_str(), "my_alias");
        assert!(manager.alias_exists("my_alias"));
    }

    #[tokio::test]
    async fn test_create_alias_nonexistent_index() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let indices = vec![IndexName::new("nonexistent")];
        let result = manager.create_alias("my_alias", indices);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_resolve_name_index() {
        // Set test mode to enable in-memory fallback for WSL compatibility
        // Note: We can't use std::env::set_var in tests due to unsafe_code deny
        // Instead, we'll rely on cfg!(test) which is always true in test builds

        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = IndexSettings::new().with_shards(1);
        manager
            .create_index("index1", schema, settings)
            .await
            .unwrap();

        let result = manager.resolve_name("index1");
        assert!(result.is_ok());
        let indices = result.unwrap();
        assert_eq!(indices.len(), 1);
        assert_eq!(indices[0].as_str(), "index1");
    }

    #[tokio::test]
    async fn test_resolve_name_alias() {
        // Set test mode to enable in-memory fallback for WSL compatibility
        // Note: We can't use std::env::set_var in tests due to unsafe_code deny
        // Instead, we'll rely on cfg!(test) which is always true in test builds

        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = IndexSettings::new().with_shards(1);
        manager
            .create_index("index1", schema, settings)
            .await
            .unwrap();

        let indices = vec![IndexName::new("index1")];
        manager.create_alias("my_alias", indices).unwrap();

        let result = manager.resolve_name("my_alias");
        assert!(result.is_ok());
        let indices = result.unwrap();
        assert_eq!(indices.len(), 1);
        assert_eq!(indices[0].as_str(), "index1");
    }

    #[tokio::test]
    async fn test_resolve_name_nonexistent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let result = manager.resolve_name("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("found"));
    }

    #[tokio::test]
    async fn test_add_indices_to_alias() {
        // Set test mode to enable in-memory fallback for WSL compatibility
        // Note: We can't use std::env::set_var in tests due to unsafe_code deny
        // Instead, we'll rely on cfg!(test) which is always true in test builds

        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = IndexSettings::new().with_shards(1);
        manager
            .create_index("index1", schema.clone(), settings.clone())
            .await
            .unwrap();
        manager
            .create_index("index2", schema, settings)
            .await
            .unwrap();

        let indices = vec![IndexName::new("index1")];
        manager.create_alias("my_alias", indices).unwrap();

        let new_indices = vec![IndexName::new("index2")];
        let result = manager.add_indices_to_alias("my_alias", new_indices);
        assert!(result.is_ok());

        let alias = manager.get_alias("my_alias").unwrap();
        assert_eq!(alias.indices.len(), 2);
    }

    #[tokio::test]
    async fn test_remove_indices_from_alias() {
        // Set test mode to enable in-memory fallback for WSL compatibility
        // Note: We can't use std::env::set_var in tests due to unsafe_code deny
        // Instead, we'll rely on cfg!(test) which is always true in test builds

        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IndexManager::new(temp_dir.path());

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = IndexSettings::new().with_shards(1);
        manager
            .create_index("index1", schema, settings)
            .await
            .unwrap();

        let indices = vec![IndexName::new("index1")];
        manager.create_alias("my_alias", indices).unwrap();

        let remove_indices = vec![IndexName::new("index1")];
        let result = manager.remove_indices_from_alias("my_alias", remove_indices);
        assert!(result.is_err()); // Should return error when alias becomes empty
        assert!(result.unwrap_err().to_string().contains("no indices"));

        // Alias should still exist but be empty
        assert!(manager.alias_exists("my_alias"));
        let alias = manager.get_alias("my_alias").unwrap();
        assert!(alias.is_empty());
    }

    // Note: Full integration tests with disk I/O will be in tests/ directory
    // These unit tests focus on logic without disk dependencies
}
