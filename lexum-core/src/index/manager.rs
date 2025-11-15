//! Index manager implementation

use crate::error::{Error, Result};
use crate::schema::mapping::ElasticsearchMapping;
use crate::types::IndexName;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tantivy::directory::MmapDirectory;
use tantivy::{Index as TantivyIndex, IndexWriter};

use super::alias::{
    AliasManager, AliasName, AliasOperationsRequest, AliasOperationsResponse, IndexAlias,
};
use super::settings::{IndexSettings, StorageSettings};

/// Index wrapper around Tantivy index
#[derive(Clone, Debug)]
pub struct Index {
    /// Index name
    pub(crate) name: IndexName,
    /// Inner Tantivy index
    pub(crate) inner: Arc<TantivyIndex>,
    /// Index settings
    pub(crate) settings: IndexSettings,
    /// Original Elasticsearch mapping (if index was created with mappings)
    /// This is used for dynamic mapping validation and copy_to support
    pub(crate) mapping: Option<ElasticsearchMapping>,
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

    /// Get all text field names from the schema (excluding keyword fields)
    /// Returns field names that are indexed and searchable
    /// Note: This includes all indexed string fields except _id (which is typically a keyword)
    pub fn get_text_field_names(&self) -> Vec<String> {
        let schema = self.schema();
        schema
            .fields()
            .filter_map(|(_, field_entry)| {
                // Check if field is a string type and indexed
                if matches!(field_entry.field_type(), tantivy::schema::FieldType::Str(_))
                    && field_entry.is_indexed()
                {
                    let field_name = field_entry.name();
                    // Skip _id fields as they're typically keywords
                    if field_name == "_id" {
                        None
                    } else {
                        Some(field_name.to_string())
                    }
                } else {
                    None
                }
            })
            .collect()
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

    /// Get original Elasticsearch mapping (if available)
    pub fn mapping(&self) -> Option<&ElasticsearchMapping> {
        self.mapping.as_ref()
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
        let storage_settings = settings.storage.clone();
        let tantivy_index = tokio::task::spawn_blocking({
            let index_path = index_path.clone();
            let schema_clone = schema.clone();
            move || {
                // Ensure the directory exists and is writable
                // If directory exists and is not empty, remove it first (for tests)
                if index_path.exists() {
                    // Check if directory is empty
                    let is_empty = std::fs::read_dir(&index_path)
                        .map(|mut entries| entries.next().is_none())
                        .unwrap_or(false);

                    if !is_empty {
                        // Remove existing directory for clean test state
                        let _ = std::fs::remove_dir_all(&index_path);
                    }
                }

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

                // Create the index using the configured storage backend
                create_tantivy_index(&index_path, schema, &storage_settings).or_else(|e| {
                    // If memory-mapped storage fails, fallback to regular storage
                    if storage_settings.enable_memory_mapped_storage {
                        eprintln!("First attempt failed: {e}");
                        eprintln!("Falling back to regular storage (memory-mapped not supported)");
                        let mut fallback_settings = storage_settings.clone();
                        fallback_settings.enable_memory_mapped_storage = false;
                        create_tantivy_index(&index_path, schema_clone.clone(), &fallback_settings)
                            .map_err(|e2| {
                                eprintln!("Second attempt failed: {e2}");
                                e2
                            })
                    } else {
                        // If it fails, try to create the directory again and retry
                        let _ = std::fs::create_dir_all(&index_path);
                        create_tantivy_index(&index_path, schema_clone.clone(), &storage_settings)
                            .map_err(|e2| {
                                eprintln!("First attempt failed: {e}");
                                eprintln!("Second attempt failed: {e2}");
                                e2
                            })
                    }
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
            mapping: None, // Will be set by create_index_with_mapping if needed
        };

        // Store index
        {
            let mut indices = self.indices.write();
            indices.insert(name_str.clone(), index.clone());
        }

        tracing::info!(index = %name_str, shards = settings.number_of_shards, "Index created");

        Ok(index)
    }

    /// Create a new index with Elasticsearch mapping
    /// This is a convenience method that stores the mapping for dynamic validation and copy_to support
    pub async fn create_index_with_mapping(
        &self,
        name: impl Into<String>,
        schema: tantivy::schema::Schema,
        settings: IndexSettings,
        mapping: Option<ElasticsearchMapping>,
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
        let storage_settings = settings.storage.clone();
        let tantivy_index = tokio::task::spawn_blocking({
            let index_path = index_path.clone();
            let schema_clone = schema.clone();
            move || {
                // Ensure the directory exists and is writable
                // If directory exists and is not empty, remove it first (for tests)
                if index_path.exists() {
                    // Check if directory is empty
                    let is_empty = std::fs::read_dir(&index_path)
                        .map(|mut entries| entries.next().is_none())
                        .unwrap_or(false);

                    if !is_empty {
                        // Remove existing directory for clean test state
                        let _ = std::fs::remove_dir_all(&index_path);
                    }
                }

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

                // Create the index using the configured storage backend
                create_tantivy_index(&index_path, schema, &storage_settings).or_else(|e| {
                    // If memory-mapped storage fails, fallback to regular storage
                    if storage_settings.enable_memory_mapped_storage {
                        eprintln!("First attempt failed: {e}");
                        eprintln!("Falling back to regular storage (memory-mapped not supported)");
                        let mut fallback_settings = storage_settings.clone();
                        fallback_settings.enable_memory_mapped_storage = false;
                        create_tantivy_index(&index_path, schema_clone.clone(), &fallback_settings)
                            .map_err(|e2| {
                                eprintln!("Second attempt failed: {e2}");
                                e2
                            })
                    } else {
                        // If it fails, try to create the directory again and retry
                        let _ = std::fs::create_dir_all(&index_path);
                        create_tantivy_index(&index_path, schema_clone.clone(), &storage_settings)
                            .map_err(|e2| {
                                eprintln!("First attempt failed: {e}");
                                eprintln!("Second attempt failed: {e2}");
                                e2
                            })
                    }
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
            mapping,
        };

        // Store index
        {
            let mut indices = self.indices.write();
            indices.insert(name_str.clone(), index.clone());
        }

        tracing::info!(index = %name_str, shards = settings.number_of_shards, "Index created with mapping");

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

fn create_tantivy_index(
    index_path: &Path,
    schema: tantivy::schema::Schema,
    storage: &StorageSettings,
) -> tantivy::Result<TantivyIndex> {
    // Ensure directory exists and is empty for new index creation
    if index_path.exists() {
        // Remove existing directory to ensure clean state
        let _ = std::fs::remove_dir_all(index_path);
    }
    std::fs::create_dir_all(index_path).map_err(|e| {
        tantivy::TantivyError::IoError(std::sync::Arc::new(std::io::Error::other(format!(
            "Failed to create index directory: {e}"
        ))))
    })?;

    // Always try regular storage first in tests/WSL environments
    // Memory-mapped can be problematic in WSL
    let use_mmap = storage.enable_memory_mapped_storage;

    if use_mmap {
        // Try memory-mapped first, but always fallback to regular storage on error
        match MmapDirectory::open(index_path) {
            Ok(directory) => {
                TantivyIndex::open_or_create(directory, schema.clone()).or_else(|err| {
                    tracing::warn!(
                        path = %index_path.display(),
                        error = %err,
                        "Falling back to filesystem directory after mmap failure"
                    );
                    TantivyIndex::create_in_dir(index_path, schema)
                })
            }
            Err(err) => {
                // MmapDirectory::open failed (e.g., WSL doesn't support it)
                // Fallback to regular filesystem directory
                tracing::warn!(
                    path = %index_path.display(),
                    error = %err,
                    "Unable to open mmap directory; using filesystem directory instead"
                );
                TantivyIndex::create_in_dir(index_path, schema)
            }
        }
    } else {
        // Use regular filesystem directory
        TantivyIndex::create_in_dir(index_path, schema)
    }
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

    #[lexum_macros::tokio_test]
    async fn test_delete_non_existent() {
        let manager = IndexManager::new("./data");
        let result = manager.delete_index("non_existent").await;
        assert!(result.is_err());
    }

    // Helper function to create test settings with memory-mapped disabled (for WSL compatibility)
    fn test_settings() -> IndexSettings {
        IndexSettings::new()
            .with_shards(1)
            .with_memory_mapped_storage(false)
    }

    // Helper function to create a test directory that works in both WSL and Windows
    // Uses Linux native paths in WSL to avoid filesystem issues with Windows-mounted drives
    fn create_test_dir() -> PathBuf {
        use std::env;
        use std::time::{SystemTime, UNIX_EPOCH};

        // Detect WSL by checking multiple indicators
        let cargo_manifest = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let is_wsl_mounted = cargo_manifest.contains("/mnt/")
            || current_dir.to_string_lossy().contains("/mnt/")
            || env::var("WSL_DISTRO_NAME").is_ok();

        let workspace_test_dir = if is_wsl_mounted {
            // In WSL: use HOME directory which is always native Linux filesystem
            // This completely avoids 9p filesystem protocol issues
            let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let mut linux_path = PathBuf::from(home);
            linux_path.push(".lexum-test-data");
            linux_path
        } else if let Ok(workspace_root) = env::var("CARGO_MANIFEST_DIR") {
            // Native Windows or Linux: use workspace-relative path
            PathBuf::from(workspace_root)
                .join("target")
                .join("test-data")
        } else {
            // Fallback: use current directory + target/test-data
            current_dir.join("target").join("test-data")
        };

        // Create base test-data directory
        std::fs::create_dir_all(&workspace_test_dir).ok();

        // Create unique subdirectory with timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut test_path = workspace_test_dir;
        test_path.push(format!("lexum_test_{timestamp}"));
        std::fs::create_dir_all(&test_path).unwrap();
        test_path
    }

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_create_index() {
        // create_test_dir() automatically detects WSL and uses Linux native paths
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
        let result = manager.create_index("test_index", schema, settings).await;
        assert!(result.is_ok());

        let index = result.unwrap();
        assert_eq!(index.name().as_str(), "test_index");
        assert!(manager.index_exists("test_index"));
    }

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_create_duplicate_index() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();

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

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_create_index_without_memory_mapped_storage() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();

        let result = manager.create_index("fs_index", schema, settings).await;
        assert!(result.is_ok());
    }

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_get_index() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
        manager
            .create_index("test_index", schema, settings)
            .await
            .unwrap();

        let index = manager.get_index("test_index").unwrap();
        assert_eq!(index.name().as_str(), "test_index");
    }

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_list_indices() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

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

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_delete_index() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
        manager
            .create_index("test_index", schema, settings)
            .await
            .unwrap();

        assert!(manager.index_exists("test_index"));

        let result = manager.delete_index("test_index").await;
        assert!(result.is_ok());

        assert!(!manager.index_exists("test_index"));
    }

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_get_index_stats() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
        manager
            .create_index("test_index", schema, settings)
            .await
            .unwrap();

        let stats = manager.get_index_stats("test_index").await.unwrap();
        assert_eq!(stats.name, "test_index");
        assert_eq!(stats.num_docs, 0);
    }

    #[lexum_macros::tokio_test]
    async fn test_get_index_stats_nonexistent() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let result = manager.get_index_stats("nonexistent").await;
        assert!(result.is_err());
    }

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_create_alias() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
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

    #[lexum_macros::tokio_test]
    async fn test_create_alias_nonexistent_index() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let indices = vec![IndexName::new("nonexistent")];
        let result = manager.create_alias("my_alias", indices);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_resolve_name_index() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
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

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_resolve_name_alias() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
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

    #[lexum_macros::tokio_test]
    async fn test_resolve_name_nonexistent() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let result = manager.resolve_name("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("found"));
    }

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_add_indices_to_alias() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

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

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_remove_indices_from_alias() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
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

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_refresh_index() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
        manager
            .create_index("test_index", schema, settings)
            .await
            .unwrap();

        let result = manager.refresh_index("test_index").await;
        assert!(result.is_ok());
    }

    #[lexum_macros::tokio_test]
    async fn test_refresh_index_not_found() {
        let manager = IndexManager::new("./data");
        let result = manager.refresh_index("non_existent").await;
        assert!(result.is_err());
    }

    #[lexum_macros::tokio_test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    async fn test_flush_index() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
        manager
            .create_index("test_index", schema, settings)
            .await
            .unwrap();

        let result = manager.flush_index("test_index").await;
        assert!(result.is_ok());
    }

    #[lexum_macros::tokio_test]
    async fn test_flush_index_not_found() {
        let manager = IndexManager::new("./data");
        let result = manager.flush_index("non_existent").await;
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    fn test_get_aliases_for_index() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        // Create an index
        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
        tokio_test::block_on(async {
            manager
                .create_index("test_index", schema, settings)
                .await
                .unwrap();
        });

        // Create aliases pointing to the index
        let indices = vec![IndexName::new("test_index")];
        manager.create_alias("alias1", indices.clone()).unwrap();
        manager.create_alias("alias2", indices).unwrap();

        let aliases = manager.get_aliases_for_index("test_index");
        assert_eq!(aliases.len(), 2);
        assert!(aliases.iter().any(|a| a.name.as_str() == "alias1"));
        assert!(aliases.iter().any(|a| a.name.as_str() == "alias2"));
    }

    #[test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    fn test_get_aliases_for_index_none() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        // Create an index without aliases
        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
        tokio_test::block_on(async {
            manager
                .create_index("test_index", schema, settings)
                .await
                .unwrap();
        });

        let aliases = manager.get_aliases_for_index("test_index");
        assert_eq!(aliases.len(), 0);
    }

    #[test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    fn test_create_index_duplicate() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
        tokio_test::block_on(async {
            manager
                .create_index("test_index", schema.clone(), settings.clone())
                .await
                .unwrap();

            // Try to create the same index again
            let result = manager.create_index("test_index", schema, settings).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("already exists"));
        });
    }

    #[test]
    fn test_create_alias_with_nonexistent_index() {
        let manager = IndexManager::new("./data");
        let indices = vec![IndexName::new("non_existent")];
        let result = manager.create_alias("my_alias", indices);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    #[ignore = "WSL/Tantivy compatibility issue - use Windows native or Linux native paths"]
    fn test_add_indices_to_alias_nonexistent_index() {
        let temp_dir = create_test_dir();
        let manager = IndexManager::new(&temp_dir);

        // Create an index and alias
        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();

        let settings = test_settings();
        tokio_test::block_on(async {
            manager
                .create_index("index1", schema, settings)
                .await
                .unwrap();
        });

        let indices = vec![IndexName::new("index1")];
        manager.create_alias("my_alias", indices).unwrap();

        // Try to add a non-existent index
        let new_indices = vec![IndexName::new("non_existent")];
        let result = manager.add_indices_to_alias("my_alias", new_indices);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    // Note: Full integration tests with disk I/O will be in tests/ directory
    // These unit tests focus on logic without disk dependencies
}
