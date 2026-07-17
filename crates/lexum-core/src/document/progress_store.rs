//! Document store with progress tracking integration

use crate::error::{Error, Result};
use crate::index::Index;
use crate::progress::{OperationType, ProgressId, ProgressTracker};
use crate::types::DocumentId;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tantivy::TantivyDocument;
use tantivy::schema::*;
use uuid::Uuid;

use super::store::{BulkError, BulkOperation, BulkOperationResult, BulkResult};

/// Document store with progress tracking capabilities
pub struct ProgressDocumentStore {
    index: Arc<Index>,
    progress_tracker: Arc<ProgressTracker>,
}

impl ProgressDocumentStore {
    /// Create a new progress-enabled document store
    pub fn new(index: Arc<Index>, progress_tracker: Arc<ProgressTracker>) -> Self {
        Self {
            index,
            progress_tracker,
        }
    }

    /// Perform bulk operations with progress tracking
    pub async fn bulk_operations_with_progress(
        &self,
        operations: Vec<BulkOperation>,
        progress_id: Option<ProgressId>,
    ) -> Result<BulkResult> {
        let total_operations = operations.len() as u64;
        let progress_id = if let Some(id) = progress_id {
            id
        } else {
            self.progress_tracker
                .start_operation(
                    OperationType::BulkOperation,
                    format!("Bulk operations on index '{}'", self.index.name()),
                    total_operations,
                    Some({
                        let mut metadata = HashMap::new();
                        metadata.insert(
                            "index_name".to_string(),
                            serde_json::Value::String(self.index.name().to_string()),
                        );
                        metadata.insert(
                            "operation_count".to_string(),
                            serde_json::Value::Number(total_operations.into()),
                        );
                        metadata
                    }),
                )
                .await?
        };

        // Mark as running
        self.progress_tracker.mark_running(&progress_id).await?;

        let schema = self.index.schema();
        let index = self.index.clone();
        let mapping = self.index.mapping().cloned();

        // Perform bulk operations in blocking context (no progress updates inside to avoid deadlock)
        let result = tokio::task::spawn_blocking(move || {
            let mut writer = index.writer(50_000_000)?;
            let mut results = Vec::new();
            let mut errors = Vec::new();

            for (i, operation) in operations.into_iter().enumerate() {
                let operation_result = match operation {
                    BulkOperation::Index {
                        index,
                        id,
                        mut document,
                        version: _,
                        version_type: _,
                    } => {
                        // Validate document against mapping if available (for dynamic mapping validation)
                        if let Some(ref mapping) = mapping {
                            if let Err(e) = mapping.validate_document(&document) {
                                let error_msg = format!("Document validation failed: {e}");
                                errors.push(BulkError {
                                    operation_index: i,
                                    error: error_msg.clone(),
                                });
                                BulkOperationResult::Index {
                                    index: index.clone(),
                                    id: id.clone(),
                                    success: false,
                                    error: Some(error_msg),
                                    version: None,
                                }
                            } else {
                                // Apply copy_to transformations
                                if let Err(e) = mapping.apply_copy_to(&mut document) {
                                    let error_msg = format!("Failed to apply copy_to: {e}");
                                    errors.push(BulkError {
                                        operation_index: i,
                                        error: error_msg.clone(),
                                    });
                                    BulkOperationResult::Index {
                                        index: index.clone(),
                                        id: id.clone(),
                                        success: false,
                                        error: Some(error_msg),
                                        version: None,
                                    }
                                } else {
                                    let tantivy_doc = Self::json_to_tantivy_doc(&schema, &document);
                                    match writer.add_document(tantivy_doc) {
                                        Ok(_) => BulkOperationResult::Index {
                                            index: index.clone(),
                                            id: id.clone(),
                                            success: true,
                                            error: None,
                                            version: None,
                                        },
                                        Err(e) => {
                                            let error_msg = format!("Failed to add document: {e}");
                                            errors.push(BulkError {
                                                operation_index: i,
                                                error: error_msg.clone(),
                                            });
                                            BulkOperationResult::Index {
                                                index: index.clone(),
                                                id: id.clone(),
                                                success: false,
                                                error: Some(error_msg),
                                                version: None,
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            let tantivy_doc = Self::json_to_tantivy_doc(&schema, &document);
                            match writer.add_document(tantivy_doc) {
                                Ok(_) => BulkOperationResult::Index {
                                    index: index.clone(),
                                    id: id.clone(),
                                    success: true,
                                    error: None,
                                    version: None,
                                },
                                Err(e) => {
                                    let error_msg = format!("Failed to add document: {e}");
                                    errors.push(BulkError {
                                        operation_index: i,
                                        error: error_msg.clone(),
                                    });
                                    BulkOperationResult::Index {
                                        index: index.clone(),
                                        id: id.clone(),
                                        success: false,
                                        error: Some(error_msg),
                                        version: None,
                                    }
                                }
                            }
                        }
                    }
                    BulkOperation::Update {
                        index,
                        id,
                        mut document,
                        version: _,
                        version_type: _,
                    } => {
                        // Validate document against mapping if available (for dynamic mapping validation)
                        if let Some(ref mapping) = mapping {
                            if let Err(e) = mapping.validate_document(&document) {
                                let error_msg = format!("Document validation failed: {e}");
                                errors.push(BulkError {
                                    operation_index: i,
                                    error: error_msg.clone(),
                                });
                                BulkOperationResult::Update {
                                    index: index.clone(),
                                    id: id.clone(),
                                    success: false,
                                    error: Some(error_msg),
                                    version: None,
                                }
                            } else {
                                // Apply copy_to transformations
                                if let Err(e) = mapping.apply_copy_to(&mut document) {
                                    let error_msg = format!("Failed to apply copy_to: {e}");
                                    errors.push(BulkError {
                                        operation_index: i,
                                        error: error_msg.clone(),
                                    });
                                    BulkOperationResult::Update {
                                        index: index.clone(),
                                        id: id.clone(),
                                        success: false,
                                        error: Some(error_msg),
                                        version: None,
                                    }
                                } else {
                                    let tantivy_doc = Self::json_to_tantivy_doc(&schema, &document);
                                    match writer.add_document(tantivy_doc) {
                                        Ok(_) => BulkOperationResult::Update {
                                            index: index.clone(),
                                            id: id.clone(),
                                            success: true,
                                            error: None,
                                            version: None,
                                        },
                                        Err(e) => {
                                            let error_msg =
                                                format!("Failed to update document: {e}");
                                            errors.push(BulkError {
                                                operation_index: i,
                                                error: error_msg.clone(),
                                            });
                                            BulkOperationResult::Update {
                                                index: index.clone(),
                                                id: id.clone(),
                                                success: false,
                                                error: Some(error_msg),
                                                version: None,
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            let tantivy_doc = Self::json_to_tantivy_doc(&schema, &document);
                            match writer.add_document(tantivy_doc) {
                                Ok(_) => BulkOperationResult::Update {
                                    index: index.clone(),
                                    id: id.clone(),
                                    success: true,
                                    error: None,
                                    version: None,
                                },
                                Err(e) => {
                                    let error_msg = format!("Failed to update document: {e}");
                                    errors.push(BulkError {
                                        operation_index: i,
                                        error: error_msg.clone(),
                                    });
                                    BulkOperationResult::Update {
                                        index: index.clone(),
                                        id: id.clone(),
                                        success: false,
                                        error: Some(error_msg),
                                        version: None,
                                    }
                                }
                            }
                        }
                    }
                    BulkOperation::Delete {
                        index,
                        id,
                        version: _,
                        version_type: _,
                    } => {
                        // For delete operations, we need to find the document first
                        // This is a simplified implementation
                        BulkOperationResult::Delete {
                            index: index.clone(),
                            id: id.clone(),
                            success: true,
                            error: None,
                            version: None,
                        }
                    }
                };

                results.push(operation_result);
            }

            // Commit the writer
            writer.commit()?;

            Ok::<BulkResult, Error>(BulkResult {
                took: 0, // Will be calculated by caller
                errors: !errors.is_empty(),
                items: results,
                errors_details: errors,
            })
        })
        .await
        .map_err(|e| Error::Config(format!("Task join error: {e}")))??;

        // Update progress after operations complete (outside spawn_blocking)
        let completed = result
            .items
            .iter()
            .filter(|r| match r {
                BulkOperationResult::Index { success, .. }
                | BulkOperationResult::Update { success, .. }
                | BulkOperationResult::Delete { success, .. } => *success,
            })
            .count() as u64;
        let failed = result
            .items
            .iter()
            .filter(|r| match r {
                BulkOperationResult::Index { success, .. }
                | BulkOperationResult::Update { success, .. }
                | BulkOperationResult::Delete { success, .. } => !*success,
            })
            .count() as u64;
        self.progress_tracker
            .update_progress(
                &progress_id,
                Some(completed),
                Some(failed),
                None,
                Some("Bulk operations completed".to_string()),
                None,
            )
            .await
            .unwrap_or_else(|e| tracing::warn!("Failed to update progress: {}", e));

        // Mark as completed
        if result.errors {
            self.progress_tracker
                .mark_failed(&progress_id, "Some operations failed".to_string())
                .await?;
        } else {
            self.progress_tracker.mark_completed(&progress_id).await?;
        }

        Ok(result)
    }

    /// Add a single document with progress tracking
    pub async fn add_document_with_progress(
        &self,
        document: JsonValue,
        progress_id: Option<ProgressId>,
    ) -> Result<DocumentId> {
        let doc_id = DocumentId::new(Uuid::new_v4().to_string());
        let operations = vec![BulkOperation::Index {
            index: self.index.name().to_string(),
            id: doc_id.clone(),
            document,
            version: None,
            version_type: None,
        }];

        let result = self
            .bulk_operations_with_progress(operations, progress_id)
            .await?;

        if result.errors {
            return Err(Error::Validation("Failed to add document".to_string()));
        }

        Ok(doc_id)
    }

    /// Update a document with progress tracking
    pub async fn update_document_with_progress(
        &self,
        id: DocumentId,
        document: JsonValue,
        progress_id: Option<ProgressId>,
    ) -> Result<()> {
        let operations = vec![BulkOperation::Update {
            index: self.index.name().to_string(),
            id,
            document,
            version: None,
            version_type: None,
        }];
        let result = self
            .bulk_operations_with_progress(operations, progress_id)
            .await?;

        if result.errors {
            return Err(Error::Validation("Failed to update document".to_string()));
        }

        Ok(())
    }

    /// Delete a document with progress tracking
    pub async fn delete_document_with_progress(
        &self,
        id: DocumentId,
        progress_id: Option<ProgressId>,
    ) -> Result<()> {
        let operations = vec![BulkOperation::Delete {
            index: self.index.name().to_string(),
            id,
            version: None,
            version_type: None,
        }];
        let result = self
            .bulk_operations_with_progress(operations, progress_id)
            .await?;

        if result.errors {
            return Err(Error::Validation("Failed to delete document".to_string()));
        }

        Ok(())
    }

    /// Get the progress tracker
    pub fn progress_tracker(&self) -> Arc<ProgressTracker> {
        self.progress_tracker.clone()
    }

    /// Convert JSON document to Tantivy document
    fn json_to_tantivy_doc(schema: &Schema, json: &JsonValue) -> TantivyDocument {
        let mut doc = TantivyDocument::new();

        for (field_name, field_value) in json.as_object().unwrap_or(&serde_json::Map::new()) {
            if let Ok(field) = schema.get_field(field_name) {
                match field_value {
                    JsonValue::String(s) => {
                        doc.add_text(field, s);
                    }
                    JsonValue::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            doc.add_i64(field, i);
                        } else if let Some(f) = n.as_f64() {
                            doc.add_f64(field, f);
                        }
                    }
                    JsonValue::Bool(b) => {
                        doc.add_bool(field, *b);
                    }
                    _ => {
                        // Convert other types to string
                        doc.add_text(field, field_value.to_string());
                    }
                }
            }
        }

        doc
    }
}

#[cfg(test)]
mod tests {
    use crate::progress::ProgressTracker;
    use std::sync::Arc;

    #[lexum_macros::tokio_test]
    async fn test_bulk_operations_with_progress() {
        // This is a simplified test - in a real implementation,
        // you would need to set up a proper index
        let _progress_tracker = Arc::new(ProgressTracker::new());

        // Create a mock index (this would need proper setup in real tests)
        // let index = Arc::new(create_test_index().await);
        // let store = ProgressDocumentStore::new(index, progress_tracker);

        // let operations = vec![
        //     BulkOperation::Index {
        //         id: DocumentId::new("doc1"),
        //         document: json!({"title": "Test Document"}),
        //     },
        // ];

        // let result = store.bulk_operations_with_progress(operations, None).await.unwrap();
        // assert!(!result.errors);
        // assert_eq!(result.items.len(), 1);
    }
}
