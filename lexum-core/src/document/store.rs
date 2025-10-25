//! Document storage and operations

use crate::error::{Error, Result};
use crate::index::Index;
use crate::types::DocumentId;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tantivy::TantivyDocument;
use tantivy::schema::*;
use uuid::Uuid;

/// Document store for managing documents in an index
pub struct DocumentStore {
    index: Arc<Index>,
}

impl DocumentStore {
    /// Create a new document store for an index
    pub fn new(index: Arc<Index>) -> Self {
        Self { index }
    }

    /// Add a document with auto-generated ID
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lexum_core::{IndexManager, SchemaBuilder, document::DocumentStore};
    /// use serde_json::json;
    /// use std::sync::Arc;
    ///
    /// # tokio_test::block_on(async {
    /// # let manager = IndexManager::new("./data");
    /// # let (schema, _) = SchemaBuilder::new().add_text_field("title").build().unwrap();
    /// # let index = manager.create_index("test", schema, Default::default()).await.unwrap();
    /// let store = DocumentStore::new(Arc::new(index));
    ///
    /// let doc = json!({
    ///     "title": "Test Document"
    /// });
    ///
    /// let doc_id = store.add_document(doc).await.unwrap();
    /// println!("Document ID: {}", doc_id);
    /// # });
    /// ```
    pub async fn add_document(&self, document: JsonValue) -> Result<DocumentId> {
        let doc_id = DocumentId::new(Uuid::new_v4().to_string());
        self.add_document_with_id(doc_id.clone(), document).await?;
        Ok(doc_id)
    }

    /// Add a document with specific ID
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lexum_core::{IndexManager, SchemaBuilder, document::DocumentStore, types::DocumentId};
    /// use serde_json::json;
    /// use std::sync::Arc;
    ///
    /// # tokio_test::block_on(async {
    /// # let manager = IndexManager::new("./data");
    /// # let (schema, _) = SchemaBuilder::new().add_text_field("title").build().unwrap();
    /// # let index = manager.create_index("test", schema, Default::default()).await.unwrap();
    /// let store = DocumentStore::new(Arc::new(index));
    ///
    /// let doc_id = DocumentId::new("doc_123");
    /// let doc = json!({"title": "Custom ID Document"});
    ///
    /// store.add_document_with_id(doc_id, doc).await.unwrap();
    /// # });
    /// ```
    pub async fn add_document_with_id(
        &self,
        doc_id: DocumentId,
        document: JsonValue,
    ) -> Result<()> {
        let schema = self.index.schema();

        // Parse JSON into Tantivy document
        let tantivy_doc = Self::json_to_tantivy_doc(&schema, &document)?;

        // Spawn blocking for Tantivy operations
        let index = self.index.clone();
        let doc_id_clone = doc_id.clone();

        tokio::task::spawn_blocking(move || {
            let mut writer = index.writer(50_000_000)?;

            // Tantivy doesn't have native document ID, we'll add it as a field if schema has "_id"
            // For now, just add the document
            writer
                .add_document(tantivy_doc)
                .map_err(|e| Error::Config(format!("Failed to add document: {e}")))?;
            writer
                .commit()
                .map_err(|e| Error::Config(format!("Failed to commit: {e}")))?;

            tracing::debug!(doc_id = %doc_id_clone, "Document added");
            Ok::<(), Error>(())
        })
        .await
        .map_err(|e| Error::Config(format!("Task join error: {e}")))??;

        Ok(())
    }

    /// Get a document by ID
    ///
    /// Note: Current implementation requires "_id" field in schema
    pub async fn get_document(&self, _doc_id: &DocumentId) -> Result<JsonValue> {
        // Will implement with proper ID field support
        Err(Error::Config(
            "get_document requires schema with _id field - not yet implemented".to_string(),
        ))
    }

    /// Update a document
    pub async fn update_document(&self, doc_id: &DocumentId, document: JsonValue) -> Result<()> {
        // Tantivy pattern: delete + add
        self.delete_document(doc_id).await?;
        self.add_document_with_id(doc_id.clone(), document).await?;
        Ok(())
    }

    /// Delete a document by ID
    pub async fn delete_document(&self, _doc_id: &DocumentId) -> Result<()> {
        // Will implement with proper ID field support
        Err(Error::Config(
            "delete_document requires schema with _id field - not yet implemented".to_string(),
        ))
    }

    /// Convert JSON to Tantivy document
    fn json_to_tantivy_doc(schema: &Schema, json: &JsonValue) -> Result<TantivyDocument> {
        let json_str = serde_json::to_string(json)
            .map_err(|e| Error::Config(format!("Failed to serialize JSON: {e}")))?;

        TantivyDocument::parse_json(schema, &json_str)
            .map_err(|e| Error::Config(format!("Failed to parse document: {e}")))
    }

    // Note: tantivy_doc_to_json will be implemented when get_document is fully implemented
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SchemaBuilder;
    use tantivy::schema::*;

    #[test]
    fn test_json_to_tantivy_doc() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_i64_field("views", INDEXED | STORED);
        let schema = schema_builder.build();

        let json = serde_json::json!({
            "title": "Test Document",
            "views": 42
        });

        let result = DocumentStore::json_to_tantivy_doc(&schema, &json);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_document() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        let schema = schema_builder.build();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        let store = DocumentStore::new(Arc::new(index));

        let doc = serde_json::json!({
            "title": "Test Document"
        });

        let result = store.add_document(doc).await;
        assert!(result.is_ok());
    }
}
