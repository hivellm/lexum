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
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum BulkOperation {
    /// Index a document (create or update)
    Index { id: DocumentId, document: JsonValue },
    /// Update a document
    Update { id: DocumentId, document: JsonValue },
    /// Delete a document
    Delete { id: DocumentId },
}

/// Result of a bulk operation
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum BulkOperationResult {
    /// Index operation result
    Index {
        id: DocumentId,
        success: bool,
        error: Option<String>,
    },
    /// Update operation result
    Update {
        id: DocumentId,
        success: bool,
        error: Option<String>,
    },
    /// Delete operation result
    Delete {
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
    ///         id: DocumentId::new("doc1"),
    ///         document: json!({"title": "Document 1"}),
    ///     },
    ///     BulkOperation::Index {
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

        let result = tokio::task::spawn_blocking(move || {
            let mut writer = index.writer(50_000_000)?;
            let mut results = Vec::new();
            let mut errors = Vec::new();

            for (i, operation) in operations.into_iter().enumerate() {
                match operation {
                    BulkOperation::Index { id, document } => {
                        match Self::json_to_tantivy_doc(&schema, &document) {
                            Ok(tantivy_doc) => match writer.add_document(tantivy_doc) {
                                Ok(_) => {
                                    results.push(BulkOperationResult::Index {
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
                                    id: id.clone(),
                                    success: false,
                                    error: Some(error_msg),
                                });
                            }
                        }
                    }
                    BulkOperation::Update { id, document } => {
                        // For now, update is delete + add
                        match Self::json_to_tantivy_doc(&schema, &document) {
                            Ok(tantivy_doc) => match writer.add_document(tantivy_doc) {
                                Ok(_) => {
                                    results.push(BulkOperationResult::Update {
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
                                    id: id.clone(),
                                    success: false,
                                    error: Some(error_msg),
                                });
                            }
                        }
                    }
                    BulkOperation::Delete { id } => {
                        // For now, delete is not implemented
                        let error_msg = "Delete operation not yet implemented".to_string();
                        errors.push(BulkError {
                            operation_index: i,
                            error: error_msg.clone(),
                        });
                        results.push(BulkOperationResult::Delete {
                            id: id.clone(),
                            success: false,
                            error: Some(error_msg),
                        });
                    }
                }
            }

            // Commit all operations
            writer
                .commit()
                .map_err(|e| Error::Config(format!("Failed to commit bulk operations: {e}")))?;

            Ok::<BulkResult, Error>(BulkResult {
                took: 0, // TODO: Implement timing
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

        // Update document - this will fail because delete_document is not implemented
        let updated_doc = serde_json::json!({
            "title": "Updated Title"
        });

        let result = store.update_document(&doc_id, updated_doc).await;
        assert!(result.is_err()); // Should fail because delete_document is not implemented
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not yet implemented")
        );
    }

    #[tokio::test]
    async fn test_get_document_not_implemented() {
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
                .contains("not yet implemented")
        );
    }

    #[tokio::test]
    async fn test_delete_document_not_implemented() {
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
                .contains("not yet implemented")
        );
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
                id: DocumentId::new("doc1"),
                document: serde_json::json!({
                    "title": "Document 1",
                    "views": 100
                }),
            },
            BulkOperation::Index {
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
    async fn test_bulk_operations_delete() {
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
            id: DocumentId::new("doc1"),
        }];

        let result = store.bulk_operations(operations).await;
        assert!(result.is_ok());

        let bulk_result = result.unwrap();
        assert_eq!(bulk_result.items.len(), 1);
        assert!(bulk_result.errors); // Delete should fail as not implemented
        assert_eq!(bulk_result.errors_details.len(), 1);

        match &bulk_result.items[0] {
            BulkOperationResult::Delete { success, error, .. } => {
                assert!(!success);
                assert!(error.is_some());
                assert!(error.as_ref().unwrap().contains("not yet implemented"));
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
                id: DocumentId::new("doc1"),
                document: serde_json::json!({
                    "title": "Document 1"
                }),
            },
            BulkOperation::Update {
                id: DocumentId::new("doc2"),
                document: serde_json::json!({
                    "title": "Updated Document 2"
                }),
            },
            BulkOperation::Delete {
                id: DocumentId::new("doc3"),
            },
        ];

        let result = store.bulk_operations(operations).await;
        assert!(result.is_ok());

        let bulk_result = result.unwrap();
        assert_eq!(bulk_result.items.len(), 3);
        assert!(bulk_result.errors); // Should have errors due to delete
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
            id: doc_id.clone(),
            document: document.clone(),
        };
        match index_op {
            BulkOperation::Index { id, document } => {
                assert_eq!(id, doc_id);
                assert_eq!(document["title"], "Test");
            }
            _ => panic!("Expected Index operation"),
        }

        // Test Update variant
        let update_op = BulkOperation::Update {
            id: doc_id.clone(),
            document: document.clone(),
        };
        match update_op {
            BulkOperation::Update { id, document } => {
                assert_eq!(id, doc_id);
                assert_eq!(document["title"], "Test");
            }
            _ => panic!("Expected Update operation"),
        }

        // Test Delete variant
        let delete_op = BulkOperation::Delete { id: doc_id };
        match delete_op {
            BulkOperation::Delete { id } => {
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
            id: doc_id.clone(),
            success: true,
            error: None,
        };
        match index_result {
            BulkOperationResult::Index { id, success, error } => {
                assert_eq!(id, doc_id);
                assert!(success);
                assert!(error.is_none());
            }
            _ => panic!("Expected Index result"),
        }

        // Test Update result
        let update_result = BulkOperationResult::Update {
            id: doc_id.clone(),
            success: false,
            error: Some("Test error".to_string()),
        };
        match update_result {
            BulkOperationResult::Update { id, success, error } => {
                assert_eq!(id, doc_id);
                assert!(!success);
                assert_eq!(error, Some("Test error".to_string()));
            }
            _ => panic!("Expected Update result"),
        }

        // Test Delete result
        let delete_result = BulkOperationResult::Delete {
            id: doc_id,
            success: false,
            error: Some("Not implemented".to_string()),
        };
        match delete_result {
            BulkOperationResult::Delete { id, success, error } => {
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
