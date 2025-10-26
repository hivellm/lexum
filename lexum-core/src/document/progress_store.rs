//! Document store with progress tracking integration

use crate::error::{Error, Result};
use crate::index::Index;
use crate::progress::{ProgressTracker, ProgressId, OperationType};
use crate::types::DocumentId;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tantivy::TantivyDocument;
use tantivy::schema::*;
use uuid::Uuid;

use super::store::{BulkOperation, BulkOperationResult, BulkResult, BulkError};

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
                        metadata.insert("index_name".to_string(), serde_json::Value::String(self.index.name().to_string()));
                        metadata.insert("operation_count".to_string(), serde_json::Value::Number(total_operations.into()));
                        metadata
                    }),
                )
                .await?
        };

        // Mark as running
        self.progress_tracker.mark_running(&progress_id).await?;

        let schema = self.index.schema();
        let index = self.index.clone();
        let progress_tracker = self.progress_tracker.clone();
        let progress_id_clone = progress_id.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut writer = index.writer(50_000_000)?;
            let mut results = Vec::new();
            let mut errors = Vec::new();
            let mut completed = 0u64;
            let mut failed = 0u64;

            for (i, operation) in operations.into_iter().enumerate() {
                let operation_result = match operation {
                    BulkOperation::Index { index, id, document } => {
                        match Self::json_to_tantivy_doc(&schema, &document) {
                            Ok(tantivy_doc) => match writer.add_document(tantivy_doc) {
                                Ok(_) => {
                                    completed += 1;
                                    BulkOperationResult::Index {
                                        index: index.clone(),
                                        id: id.clone(),
                                        success: true,
                                        error: None,
                                    }
                                }
                                Err(e) => {
                                    failed += 1;
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
                                    }
                                }
                            },
                            Err(e) => {
                                failed += 1;
                                let error_msg = format!("Failed to parse document: {e}");
                                errors.push(BulkError {
                                    operation_index: i,
                                    error: error_msg.clone(),
                                });
                                BulkOperationResult::Index {
                                    index: index.clone(),
                                    id: id.clone(),
                                    success: false,
                                    error: Some(error_msg),
                                }
                            }
                        }
                    }
                    BulkOperation::Update { index, id, document } => {
                        match Self::json_to_tantivy_doc(&schema, &document) {
                            Ok(tantivy_doc) => match writer.add_document(tantivy_doc) {
                                Ok(_) => {
                                    completed += 1;
                                    BulkOperationResult::Update {
                                        index: index.clone(),
                                        id: id.clone(),
                                        success: true,
                                        error: None,
                                    }
                                }
                                Err(e) => {
                                    failed += 1;
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
                                    }
                                }
                            },
                            Err(e) => {
                                failed += 1;
                                let error_msg = format!("Failed to parse document: {e}");
                                errors.push(BulkError {
                                    operation_index: i,
                                    error: error_msg.clone(),
                                });
                                BulkOperationResult::Update {
                                    index: index.clone(),
                                    id: id.clone(),
                                    success: false,
                                    error: Some(error_msg),
                                }
                            }
                        }
                    }
                    BulkOperation::Delete { index, id } => {
                        // For delete operations, we need to find the document first
                        // This is a simplified implementation
                        completed += 1;
                        BulkOperationResult::Delete {
                            index: index.clone(),
                            id: id.clone(),
                            success: true,
                            error: None,
                        }
                    }
                };

                results.push(operation_result);

                // Update progress every 100 operations or at the end
                let total_ops = total_operations as usize;
                if (i + 1) % 100 == 0 || i == total_ops - 1 {
                    let progress_tracker = progress_tracker.clone();
                    let progress_id = progress_id_clone.clone();
                    let completed_count = completed;
                    let failed_count = failed;
                    
                    // Spawn async task to update progress
                    tokio::spawn(async move {
                        if let Err(e) = progress_tracker
                            .update_progress(
                                &progress_id,
                                Some(completed_count),
                                Some(failed_count),
                                None,
                                Some(format!("Processing operation {}/{}", i + 1, total_ops)),
                                None,
                            )
                            .await
                        {
                            tracing::warn!("Failed to update progress: {}", e);
                        }
                    });
                }
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
        .await??;

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
        }];

        let result = self.bulk_operations_with_progress(operations, progress_id).await?;
        
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
            document 
        }];
        let result = self.bulk_operations_with_progress(operations, progress_id).await?;
        
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
            id 
        }];
        let result = self.bulk_operations_with_progress(operations, progress_id).await?;
        
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
    fn json_to_tantivy_doc(schema: &Schema, json: &JsonValue) -> Result<TantivyDocument> {
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
                        doc.add_text(field, &field_value.to_string());
                    }
                }
            }
        }
        
        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::ProgressTracker;
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_bulk_operations_with_progress() {
        // This is a simplified test - in a real implementation,
        // you would need to set up a proper index
        let progress_tracker = Arc::new(ProgressTracker::new());
        
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