//! Index manager implementation

use crate::error::{Error, Result};
use crate::types::IndexName;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tantivy::{Index as TantivyIndex, IndexWriter};

use super::settings::IndexSettings;

/// Index wrapper around Tantivy index
#[derive(Clone)]
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
}

impl IndexManager {
    /// Create a new index manager
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            indices: Arc::new(RwLock::new(HashMap::new())),
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
        tokio::fs::create_dir_all(&index_path).await?;

        // Create Tantivy index
        let tantivy_index = TantivyIndex::create_in_dir(&index_path, schema)
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
    pub fn get_index_stats(&self, name: &str) -> Result<IndexStats> {
        let index = self.get_index(name)?;
        let reader = index.reader()?;
        let searcher = reader.searcher();

        let num_docs = searcher.num_docs();
        let num_segments = searcher.segment_readers().len();

        Ok(IndexStats {
            name: name.to_string(),
            num_docs,
            num_segments,
        })
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

    // Note: Full integration tests with disk I/O will be in tests/ directory
    // These unit tests focus on logic without disk dependencies
}
