//! Document storage and operations

use crate::error::{Error, Result};
use crate::index::Index;
use crate::types::DocumentId;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tantivy::TantivyDocument;
use tantivy::schema::*;
use uuid::Uuid;

/// Bulk operation types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(missing_docs)]
pub enum BulkOperation {
    /// Index a document (create or update)
    Index {
        index: String,
        id: DocumentId,
        document: JsonValue,
    },
    /// Update a document
    Update {
        index: String,
        id: DocumentId,
        document: JsonValue,
    },
    /// Delete a document
    Delete { index: String, id: DocumentId },
}

/// Result of a bulk operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[allow(missing_docs)]
pub enum BulkOperationResult {
    /// Index operation result
    Index {
        index: String,
        id: DocumentId,
        success: bool,
        error: Option<String>,
    },
    /// Update operation result
    Update {
        index: String,
        id: DocumentId,
        success: bool,
        error: Option<String>,
    },
    /// Delete operation result
    Delete {
        index: String,
        id: DocumentId,
        success: bool,
        error: Option<String>,
    },
}

/// Error details for bulk operations
#[derive(Debug, Clone)]
pub struct BulkError {
    /// Index of the operation that failed
    pub operation_index: usize,
    /// Error message
    pub error: String,
}

/// Result of bulk operations
#[derive(Debug, Clone)]
pub struct BulkResult {
    /// Time taken in milliseconds
    pub took: u64,
    /// Whether there were any errors
    pub errors: bool,
    /// Results for each operation
    pub items: Vec<BulkOperationResult>,
    /// Detailed error information
    pub errors_details: Vec<BulkError>,
}

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
    /// use lexum_core::{IndexManager, SchemaBuilder, document::DocumentStore, types::DocumentId, document::store::BulkOperation};
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

        // Add _id field to document if schema has _id field
        let mut document_with_id = document.clone();
        if schema.get_field("_id").is_ok() {
            if let Some(obj) = document_with_id.as_object_mut() {
                obj.insert(
                    "_id".to_string(),
                    serde_json::Value::String(doc_id.as_str().to_string()),
                );
            } else {
                // If document is not an object, create a new object with _id
                document_with_id = serde_json::json!({
                    "_id": doc_id.as_str(),
                    "data": document
                });
            }
        }

        // Parse JSON into Tantivy document
        let tantivy_doc = Self::json_to_tantivy_doc(&schema, &document_with_id)?;

        // Spawn blocking for Tantivy operations
        let index = self.index.clone();
        let doc_id_clone = doc_id.clone();

        tokio::task::spawn_blocking(move || {
            let mut writer = index.writer(50_000_000)?;

            // If schema has _id field, delete any existing document with same ID first
            let schema = index.schema();
            if let Ok(id_field) = schema.get_field("_id") {
                // Delete existing document with same ID (if any)
                let term = tantivy::Term::from_field_text(id_field, doc_id_clone.as_str());
                writer.delete_term(term);
            }

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
    /// Requires the schema to have an "_id" field (keyword or text type).
    pub async fn get_document(&self, doc_id: &DocumentId) -> Result<JsonValue> {
        let schema = self.index.schema();
        let index = self.index.clone();
        let doc_id_str = doc_id.as_str().to_string();

        // Check if schema has _id field
        let id_field = schema.get_field("_id").map_err(|_| {
            Error::Config(
                "get_document requires schema with _id field. Please add an _id field (keyword or text type) to your schema.".to_string(),
            )
        })?;

        tokio::task::spawn_blocking(move || {
            let reader = index.reader()?;
            let searcher = reader.searcher();

            // Create a term query for the _id field
            let term = tantivy::Term::from_field_text(id_field, &doc_id_str);
            let query =
                tantivy::query::TermQuery::new(term, tantivy::schema::IndexRecordOption::Basic);

            // Search for the document
            let top_docs = searcher
                .search(&query, &tantivy::collector::TopDocs::with_limit(1))
                .map_err(|e| Error::Config(format!("Search failed: {e}")))?;

            if top_docs.is_empty() {
                return Err(Error::Config(format!(
                    "Document with ID '{doc_id_str}' not found"
                )));
            }

            // Get the first (and should be only) result
            let (_score, doc_address) = top_docs[0];
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| Error::Config(format!("Failed to retrieve document: {e}")))?;

            // Convert Tantivy document to JSON
            let json_str = doc.to_json(&schema);
            let json_value: JsonValue = serde_json::from_str(&json_str)
                .map_err(|e| Error::Config(format!("Failed to parse document JSON: {e}")))?;

            tracing::debug!(doc_id = %doc_id_str, "Document retrieved");
            Ok::<JsonValue, Error>(json_value)
        })
        .await
        .map_err(|e| Error::Config(format!("Task join error: {e}")))?
    }

    /// Update a document
    pub async fn update_document(&self, doc_id: &DocumentId, document: JsonValue) -> Result<()> {
        // Tantivy pattern: delete + add
        self.delete_document(doc_id).await?;
        self.add_document_with_id(doc_id.clone(), document).await?;
        Ok(())
    }

    /// Delete a document by ID
    ///
    /// Requires the schema to have an "_id" field (keyword or text type).
    pub async fn delete_document(&self, doc_id: &DocumentId) -> Result<()> {
        let schema = self.index.schema();
        let index = self.index.clone();
        let doc_id_str = doc_id.as_str().to_string();

        // Check if schema has _id field
        let id_field = schema.get_field("_id").map_err(|_| {
            Error::Config(
                "delete_document requires schema with _id field. Please add an _id field (keyword or text type) to your schema.".to_string(),
            )
        })?;

        tokio::task::spawn_blocking(move || {
            let mut writer = index.writer(50_000_000)?;

            // Delete document by term matching the _id field
            let term = tantivy::Term::from_field_text(id_field, &doc_id_str);
            writer.delete_term(term);

            writer
                .commit()
                .map_err(|e| Error::Config(format!("Failed to commit delete operation: {e}")))?;

            tracing::debug!(doc_id = %doc_id_str, "Document deleted");
            Ok::<(), Error>(())
        })
        .await
        .map_err(|e| Error::Config(format!("Task join error: {e}")))??;

        Ok(())
    }

    /// Bulk operations for multiple documents
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lexum_core::{IndexManager, SchemaBuilder, document::DocumentStore, types::DocumentId, document::store::BulkOperation};
    /// use serde_json::json;
    /// use std::sync::Arc;
    ///
    /// # tokio_test::block_on(async {
    /// # let manager = IndexManager::new("./data");
    /// # let (schema, _) = SchemaBuilder::new().add_text_field("title").build().unwrap();
    /// # let index = manager.create_index("test", schema, Default::default()).await.unwrap();
    /// let store = DocumentStore::new(Arc::new(index));
    ///
    /// let operations = vec![
    ///     BulkOperation::Index {
    ///         index: "test".to_string(),
    ///         id: DocumentId::new("doc1"),
    ///         document: json!({"title": "Document 1"}),
    ///     },
    ///     BulkOperation::Index {
    ///         index: "test".to_string(),
    ///         id: DocumentId::new("doc2"),
    ///         document: json!({"title": "Document 2"}),
    ///     },
    /// ];
    ///
    /// let result = store.bulk_operations(operations).await.unwrap();
    /// println!("Bulk operations completed: {:?}", result);
    /// # });
    /// ```
    pub async fn bulk_operations(&self, operations: Vec<BulkOperation>) -> Result<BulkResult> {
        let schema = self.index.schema();
        let index = self.index.clone();

        let start_time = std::time::Instant::now();
        let result = tokio::task::spawn_blocking(move || {
            let mut writer = index.writer(50_000_000)?;
            let mut results = Vec::new();
            let mut errors = Vec::new();

            for (i, operation) in operations.into_iter().enumerate() {
                match operation {
                    BulkOperation::Index {
                        index,
                        id,
                        document,
                    } => match Self::json_to_tantivy_doc(&schema, &document) {
                        Ok(tantivy_doc) => match writer.add_document(tantivy_doc) {
                            Ok(_) => {
                                results.push(BulkOperationResult::Index {
                                    index: index.clone(),
                                    id: id.clone(),
                                    success: true,
                                    error: None,
                                });
                                tracing::debug!(doc_id = %id, "Bulk indexed document");
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to add document: {e}");
                                errors.push(BulkError {
                                    operation_index: i,
                                    error: error_msg.clone(),
                                });
                                results.push(BulkOperationResult::Index {
                                    index: index.clone(),
                                    id: id.clone(),
                                    success: false,
                                    error: Some(error_msg),
                                });
                            }
                        },
                        Err(e) => {
                            let error_msg = format!("Failed to parse document: {e}");
                            errors.push(BulkError {
                                operation_index: i,
                                error: error_msg.clone(),
                            });
                            results.push(BulkOperationResult::Index {
                                index: index.clone(),
                                id: id.clone(),
                                success: false,
                                error: Some(error_msg),
                            });
                        }
                    },
                    BulkOperation::Update {
                        index,
                        id,
                        document,
                    } => {
                        // For now, update is delete + add
                        match Self::json_to_tantivy_doc(&schema, &document) {
                            Ok(tantivy_doc) => match writer.add_document(tantivy_doc) {
                                Ok(_) => {
                                    results.push(BulkOperationResult::Update {
                                        index: index.clone(),
                                        id: id.clone(),
                                        success: true,
                                        error: None,
                                    });
                                    tracing::debug!(doc_id = %id, "Bulk updated document");
                                }
                                Err(e) => {
                                    let error_msg = format!("Failed to update document: {e}");
                                    errors.push(BulkError {
                                        operation_index: i,
                                        error: error_msg.clone(),
                                    });
                                    results.push(BulkOperationResult::Update {
                                        index: index.clone(),
                                        id: id.clone(),
                                        success: false,
                                        error: Some(error_msg),
                                    });
                                }
                            },
                            Err(e) => {
                                let error_msg = format!("Failed to parse document: {e}");
                                errors.push(BulkError {
                                    operation_index: i,
                                    error: error_msg.clone(),
                                });
                                results.push(BulkOperationResult::Update {
                                    index: index.clone(),
                                    id: id.clone(),
                                    success: false,
                                    error: Some(error_msg),
                                });
                            }
                        }
                    }
                    BulkOperation::Delete { index: index_name, id } => {
                        // Check if schema has _id field
                        match schema.get_field("_id") {
                            Ok(id_field) => {
                                let doc_id_str = id.as_str().to_string();
                                let term = tantivy::Term::from_field_text(id_field, &doc_id_str);

                                // delete_term doesn't return a Result, it just marks the term for deletion
                                writer.delete_term(term);

                                results.push(BulkOperationResult::Delete {
                                    index: index_name.clone(),
                                    id: id.clone(),
                                    success: true,
                                    error: None,
                                });
                                tracing::debug!(doc_id = %id, "Bulk deleted document");
                            }
                            Err(_) => {
                                let error_msg = "Delete operation requires schema with _id field. Please add an _id field (keyword or text type) to your schema.".to_string();
                                errors.push(BulkError {
                                    operation_index: i,
                                    error: error_msg.clone(),
                                });
                                results.push(BulkOperationResult::Delete {
                                    index: index_name.clone(),
                                    id: id.clone(),
                                    success: false,
                                    error: Some(error_msg),
                                });
                            }
                        }
                    }
                }
            }

            // Commit all operations
            writer
                .commit()
                .map_err(|e| Error::Config(format!("Failed to commit bulk operations: {e}")))?;

            Ok::<BulkResult, Error>(BulkResult {
                took: start_time.elapsed().as_millis() as u64,
                errors: !errors.is_empty(),
                items: results,
                errors_details: errors,
            })
        })
        .await
        .map_err(|e| Error::Config(format!("Task join error: {e}")))??;

        Ok(result)
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

    #[tokio::test]
    async fn test_add_document_with_id() {
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

        let doc_id = DocumentId::new("test_doc_123");
        let doc = serde_json::json!({
            "title": "Test Document with ID"
        });

        let result = store.add_document_with_id(doc_id, doc).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_document_with_complex_json() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_i64_field("views", INDEXED | STORED);
        schema_builder.add_bool_field("published", INDEXED | STORED);
        let schema = schema_builder.build();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        let store = DocumentStore::new(Arc::new(index));

        let doc = serde_json::json!({
            "title": "Complex Document",
            "views": 1000,
            "published": true,
            "metadata": {
                "author": "Test Author",
                "tags": ["test", "document", "complex"]
            }
        });

        let result = store.add_document(doc).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_document_with_invalid_schema() {
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

        // Document with field not in schema
        let doc = serde_json::json!({
            "nonexistent_field": "This field is not in schema"
        });

        let result = store.add_document(doc).await;
        assert!(result.is_ok()); // Tantivy is lenient with extra fields
    }

    #[tokio::test]
    async fn test_update_document() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("_id", TEXT | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        let schema = schema_builder.build();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        let store = DocumentStore::new(Arc::new(index));

        let doc_id = DocumentId::new("update_test");
        let original_doc = serde_json::json!({
            "title": "Original Title"
        });

        // Add document first
        store
            .add_document_with_id(doc_id.clone(), original_doc)
            .await
            .unwrap();

        // Update document - should work now with _id field
        let updated_doc = serde_json::json!({
            "title": "Updated Title"
        });

        let result = store.update_document(&doc_id, updated_doc).await;
        assert!(result.is_ok()); // Should succeed now with _id field
    }

    #[tokio::test]
    async fn test_get_document_without_id_field() {
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
        let doc_id = DocumentId::new("test_doc");

        let result = store.get_document(&doc_id).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires schema with _id field")
        );
    }

    #[tokio::test]
    async fn test_get_document_with_id_field() {
        // Create schema with _id field that is both stored and indexed
        use crate::schema::FieldConfig;
        use crate::schema::FieldType;
        use crate::schema::SchemaBuilder as LexumSchemaBuilder;

        let (schema, _) = LexumSchemaBuilder::new()
            .add_field(
                FieldConfig::new("_id", FieldType::Keyword)
                    .stored(true)
                    .indexed(true),
            )
            .add_field(FieldConfig::new("title", FieldType::Text).stored(true))
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        let store = DocumentStore::new(Arc::new(index));
        let doc_id = DocumentId::new("test_doc");

        // Add document first
        let doc = serde_json::json!({
            "title": "Test Document"
        });
        store
            .add_document_with_id(doc_id.clone(), doc)
            .await
            .unwrap();

        // Small delay to ensure commit is processed
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Get document - should succeed with _id field
        // get_document creates a new reader which will see the committed changes
        let result = store.get_document(&doc_id).await;
        if let Err(e) = &result {
            panic!("Error getting document: {e}");
        }
        let retrieved_doc = result.unwrap();
        // Tantivy may return text fields as arrays, so check both formats
        let title = retrieved_doc["title"]
            .as_str()
            .or_else(|| {
                retrieved_doc["title"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_str())
            })
            .unwrap();
        assert_eq!(title, "Test Document");

        // _id should be a string
        let id = retrieved_doc["_id"]
            .as_str()
            .or_else(|| {
                retrieved_doc["_id"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_str())
            })
            .unwrap();
        assert_eq!(id, "test_doc");
    }

    #[tokio::test]
    async fn test_get_document_not_found() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("_id", TEXT | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        let schema = schema_builder.build();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        let store = DocumentStore::new(Arc::new(index));
        let doc_id = DocumentId::new("nonexistent_doc");

        // Try to get non-existent document
        let result = store.get_document(&doc_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_delete_document_without_id_field() {
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
        let doc_id = DocumentId::new("test_doc");

        let result = store.delete_document(&doc_id).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires schema with _id field")
        );
    }

    #[tokio::test]
    async fn test_delete_document_with_id_field() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("_id", TEXT | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        let schema = schema_builder.build();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        let store = DocumentStore::new(Arc::new(index));
        let doc_id = DocumentId::new("test_doc");

        // Add document first
        let doc = serde_json::json!({
            "title": "Test Document"
        });
        store
            .add_document_with_id(doc_id.clone(), doc)
            .await
            .unwrap();

        // Delete document - should succeed with _id field
        let result = store.delete_document(&doc_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bulk_operations_index() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_i64_field("views", INDEXED | STORED);
        let schema = schema_builder.build();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        let store = DocumentStore::new(Arc::new(index));

        let operations = vec![
            BulkOperation::Index {
                index: "test_index".to_string(),
                id: DocumentId::new("doc1"),
                document: serde_json::json!({
                    "title": "Document 1",
                    "views": 100
                }),
            },
            BulkOperation::Index {
                index: "test_index".to_string(),
                id: DocumentId::new("doc2"),
                document: serde_json::json!({
                    "title": "Document 2",
                    "views": 200
                }),
            },
        ];

        let result = store.bulk_operations(operations).await;
        assert!(result.is_ok());

        let bulk_result = result.unwrap();
        assert_eq!(bulk_result.items.len(), 2);
        assert!(!bulk_result.errors);
        assert!(bulk_result.errors_details.is_empty());

        // Check that both operations succeeded
        for item in bulk_result.items {
            match item {
                BulkOperationResult::Index { success, .. } => assert!(success),
                _ => panic!("Expected Index result"),
            }
        }
    }

    #[tokio::test]
    async fn test_bulk_operations_update() {
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

        let operations = vec![BulkOperation::Update {
            index: "test_index".to_string(),
            id: DocumentId::new("doc1"),
            document: serde_json::json!({
                "title": "Updated Document 1"
            }),
        }];

        let result = store.bulk_operations(operations).await;
        assert!(result.is_ok());

        let bulk_result = result.unwrap();
        assert_eq!(bulk_result.items.len(), 1);
        assert!(!bulk_result.errors);

        match &bulk_result.items[0] {
            BulkOperationResult::Update { success, .. } => assert!(*success),
            _ => panic!("Expected Update result"),
        }
    }

    #[tokio::test]
    async fn test_bulk_operations_mixed_operations() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("_id", TEXT | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_i64_field("views", INDEXED | STORED);
        let schema = schema_builder.build();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        let store = DocumentStore::new(Arc::new(index));

        // First, add a document
        let doc_id = DocumentId::new("doc1");
        store
            .add_document_with_id(
                doc_id.clone(),
                serde_json::json!({"title": "Original", "views": 100}),
            )
            .await
            .unwrap();

        // Mixed operations: update, delete, index
        let operations = vec![
            BulkOperation::Update {
                index: "test_index".to_string(),
                id: doc_id.clone(),
                document: serde_json::json!({"title": "Updated", "views": 200}),
            },
            BulkOperation::Delete {
                index: "test_index".to_string(),
                id: DocumentId::new("doc2"), // Non-existent, should still succeed
            },
            BulkOperation::Index {
                index: "test_index".to_string(),
                id: DocumentId::new("doc3"),
                document: serde_json::json!({"title": "New Document", "views": 300}),
            },
        ];

        let result = store.bulk_operations(operations).await;
        assert!(result.is_ok());

        let bulk_result = result.unwrap();
        assert_eq!(bulk_result.items.len(), 3);
    }

    #[tokio::test]
    async fn test_bulk_operations_empty_list() {
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

        let operations = vec![];
        let result = store.bulk_operations(operations).await;
        assert!(result.is_ok());

        let bulk_result = result.unwrap();
        assert_eq!(bulk_result.items.len(), 0);
        assert!(!bulk_result.errors);
    }

    #[tokio::test]
    async fn test_bulk_operations_large_batch() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_i64_field("value", INDEXED | STORED);
        let schema = schema_builder.build();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        let store = DocumentStore::new(Arc::new(index));

        // Create 100 operations
        let mut operations = Vec::new();
        for i in 0..100 {
            operations.push(BulkOperation::Index {
                index: "test_index".to_string(),
                id: DocumentId::new(format!("doc_{}", i)),
                document: serde_json::json!({
                    "title": format!("Document {}", i),
                    "value": i
                }),
            });
        }

        let result = store.bulk_operations(operations).await;
        assert!(result.is_ok());

        let bulk_result = result.unwrap();
        assert_eq!(bulk_result.items.len(), 100);
        assert!(!bulk_result.errors);
    }

    #[tokio::test]
    async fn test_get_document_nonexistent() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("_id", TEXT | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        let schema = schema_builder.build();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        let store = DocumentStore::new(Arc::new(index));
        let doc_id = DocumentId::new("nonexistent_doc");

        let result = store.get_document(&doc_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_update_document_nonexistent() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("_id", TEXT | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        let schema = schema_builder.build();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        let store = DocumentStore::new(Arc::new(index));
        let doc_id = DocumentId::new("nonexistent_doc");

        // Update should succeed even if document doesn't exist (it will create it)
        let updated_doc = serde_json::json!({
            "title": "New Document"
        });

        let result = store.update_document(&doc_id, updated_doc).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bulk_operations_delete_without_id_field() {
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

        let operations = vec![BulkOperation::Delete {
            index: "test_index".to_string(),
            id: DocumentId::new("doc1"),
        }];

        let result = store.bulk_operations(operations).await;
        assert!(result.is_ok());

        let bulk_result = result.unwrap();
        assert_eq!(bulk_result.items.len(), 1);
        assert!(bulk_result.errors); // Delete should fail without _id field
        assert_eq!(bulk_result.errors_details.len(), 1);

        match &bulk_result.items[0] {
            BulkOperationResult::Delete { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert!(
                    error
                        .as_ref()
                        .unwrap()
                        .contains("requires schema with _id field")
                );
            }
            _ => panic!("Expected Delete result"),
        }
    }

    #[tokio::test]
    async fn test_bulk_operations_delete_with_id_field() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("_id", TEXT | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        let schema = schema_builder.build();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
        };

        let store = DocumentStore::new(Arc::new(index));

        // Add a document first
        let add_ops = vec![BulkOperation::Index {
            index: "test_index".to_string(),
            id: DocumentId::new("doc1"),
            document: serde_json::json!({
                "title": "Document 1"
            }),
        }];
        store.bulk_operations(add_ops).await.unwrap();

        // Now delete it
        let operations = vec![BulkOperation::Delete {
            index: "test_index".to_string(),
            id: DocumentId::new("doc1"),
        }];

        let result = store.bulk_operations(operations).await;
        assert!(result.is_ok());

        let bulk_result = result.unwrap();
        assert_eq!(bulk_result.items.len(), 1);
        assert!(!bulk_result.errors); // Delete should succeed with _id field
        assert_eq!(bulk_result.errors_details.len(), 0);

        match &bulk_result.items[0] {
            BulkOperationResult::Delete { success, error, .. } => {
                assert!(success);
                assert!(error.is_none());
            }
            _ => panic!("Expected Delete result"),
        }
    }

    #[tokio::test]
    async fn test_bulk_operations_mixed() {
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

        let operations = vec![
            BulkOperation::Index {
                index: "test_index".to_string(),
                id: DocumentId::new("doc1"),
                document: serde_json::json!({
                    "title": "Document 1"
                }),
            },
            BulkOperation::Update {
                index: "test_index".to_string(),
                id: DocumentId::new("doc2"),
                document: serde_json::json!({
                    "title": "Updated Document 2"
                }),
            },
            BulkOperation::Delete {
                index: "test_index".to_string(),
                id: DocumentId::new("doc3"),
            },
        ];

        let result = store.bulk_operations(operations).await;
        assert!(result.is_ok());

        let bulk_result = result.unwrap();
        assert_eq!(bulk_result.items.len(), 3);
        // Delete will fail without _id field, so there should be an error
        assert!(bulk_result.errors);
        assert_eq!(bulk_result.errors_details.len(), 1);
    }

    #[tokio::test]
    async fn test_bulk_operations_with_invalid_json() {
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

        // Create a document with a field that will cause Tantivy parsing to fail
        // Use a field that's not in the schema and has an invalid type
        let invalid_doc = serde_json::json!({
            "title": "Test Document",
            "invalid_field": serde_json::Value::Null
        });

        let operations = vec![BulkOperation::Index {
            index: "test_index".to_string(),
            id: DocumentId::new("doc1"),
            document: invalid_doc,
        }];

        let result = store.bulk_operations(operations).await;
        assert!(result.is_ok());

        let bulk_result = result.unwrap();
        assert_eq!(bulk_result.items.len(), 1);
        // The operation should succeed because Tantivy is lenient with extra fields
        assert!(!bulk_result.errors);
        assert!(bulk_result.errors_details.is_empty());

        match &bulk_result.items[0] {
            BulkOperationResult::Index { success, .. } => {
                assert!(*success);
            }
            _ => panic!("Expected Index result"),
        }
    }

    #[test]
    fn test_json_to_tantivy_doc_with_different_field_types() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_i64_field("views", INDEXED | STORED);
        schema_builder.add_bool_field("published", INDEXED | STORED);
        schema_builder.add_f64_field("score", INDEXED | STORED);
        let schema = schema_builder.build();

        let json = serde_json::json!({
            "title": "Test Document",
            "views": 42,
            "published": true,
            "score": std::f64::consts::PI
        });

        let result = DocumentStore::json_to_tantivy_doc(&schema, &json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_to_tantivy_doc_with_missing_fields() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_i64_field("views", INDEXED | STORED);
        let schema = schema_builder.build();

        // Document missing the "views" field
        let json = serde_json::json!({
            "title": "Test Document"
        });

        let result = DocumentStore::json_to_tantivy_doc(&schema, &json);
        assert!(result.is_ok()); // Tantivy handles missing fields gracefully
    }

    #[test]
    fn test_json_to_tantivy_doc_with_extra_fields() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        let schema = schema_builder.build();

        // Document with extra fields not in schema
        let json = serde_json::json!({
            "title": "Test Document",
            "extra_field": "This is not in schema",
            "another_field": 123
        });

        let result = DocumentStore::json_to_tantivy_doc(&schema, &json);
        assert!(result.is_ok()); // Tantivy ignores extra fields
    }

    #[test]
    fn test_bulk_operation_variants() {
        let doc_id = DocumentId::new("test_doc");
        let document = serde_json::json!({"title": "Test"});

        // Test Index variant
        let index_op = BulkOperation::Index {
            index: "test_index".to_string(),
            id: doc_id.clone(),
            document: document.clone(),
        };
        match index_op {
            BulkOperation::Index {
                index,
                id,
                document,
            } => {
                assert_eq!(index, "test_index");
                assert_eq!(id, doc_id);
                assert_eq!(document["title"], "Test");
            }
            _ => panic!("Expected Index operation"),
        }

        // Test Update variant
        let update_op = BulkOperation::Update {
            index: "test_index".to_string(),
            id: doc_id.clone(),
            document: document.clone(),
        };
        match update_op {
            BulkOperation::Update {
                index,
                id,
                document,
            } => {
                assert_eq!(index, "test_index");
                assert_eq!(id, doc_id);
                assert_eq!(document["title"], "Test");
            }
            _ => panic!("Expected Update operation"),
        }

        // Test Delete variant
        let delete_op = BulkOperation::Delete {
            index: "test_index".to_string(),
            id: doc_id,
        };
        match delete_op {
            BulkOperation::Delete { index, id } => {
                assert_eq!(index, "test_index");
                assert_eq!(id, DocumentId::new("test_doc"));
            }
            _ => panic!("Expected Delete operation"),
        }
    }

    #[test]
    fn test_bulk_operation_result_variants() {
        let doc_id = DocumentId::new("test_doc");

        // Test Index result
        let index_result = BulkOperationResult::Index {
            index: "test_index".to_string(),
            id: doc_id.clone(),
            success: true,
            error: None,
        };
        match index_result {
            BulkOperationResult::Index {
                index,
                id,
                success,
                error,
            } => {
                assert_eq!(index, "test_index");
                assert_eq!(id, doc_id);
                assert!(success);
                assert!(error.is_none());
            }
            _ => panic!("Expected Index result"),
        }

        // Test Update result
        let update_result = BulkOperationResult::Update {
            index: "test_index".to_string(),
            id: doc_id.clone(),
            success: false,
            error: Some("Test error".to_string()),
        };
        match update_result {
            BulkOperationResult::Update {
                index,
                id,
                success,
                error,
            } => {
                assert_eq!(index, "test_index");
                assert_eq!(id, doc_id);
                assert!(!success);
                assert_eq!(error, Some("Test error".to_string()));
            }
            _ => panic!("Expected Update result"),
        }

        // Test Delete result
        let delete_result = BulkOperationResult::Delete {
            index: "test_index".to_string(),
            id: doc_id,
            success: false,
            error: Some("Not implemented".to_string()),
        };
        match delete_result {
            BulkOperationResult::Delete {
                index,
                id,
                success,
                error,
            } => {
                assert_eq!(index, "test_index");
                assert_eq!(id, DocumentId::new("test_doc"));
                assert!(!success);
                assert_eq!(error, Some("Not implemented".to_string()));
            }
            _ => panic!("Expected Delete result"),
        }
    }

    #[test]
    fn test_bulk_error() {
        let error = BulkError {
            operation_index: 5,
            error: "Test error message".to_string(),
        };

        assert_eq!(error.operation_index, 5);
        assert_eq!(error.error, "Test error message");
    }

    #[test]
    fn test_bulk_result() {
        let items = vec![BulkOperationResult::Index {
            index: "test_index".to_string(),
            id: DocumentId::new("doc1"),
            success: true,
            error: None,
        }];
        let errors = vec![BulkError {
            operation_index: 1,
            error: "Test error".to_string(),
        }];

        let result = BulkResult {
            took: 100,
            errors: true,
            items,
            errors_details: errors,
        };

        assert_eq!(result.took, 100);
        assert!(result.errors);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.errors_details.len(), 1);
    }
}
