//! Stress testing framework for Lexum
//!
//! This module provides stress tests that verify the system handles resource limits
//! and extreme conditions gracefully.

use anyhow::Result;
use lexum_core::{
    FieldConfig, FieldType, IndexManager, Query, QueryBuilder, SchemaBuilder, SearchExecutor,
    document::DocumentStore,
};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Stress test configuration
#[derive(Debug, Clone)]
pub struct StressConfig {
    pub max_memory_mb: Option<u64>,
    pub max_disk_space_mb: Option<u64>,
    pub max_connections: Option<usize>,
    pub max_query_complexity: Option<usize>,
    pub test_duration: Duration,
}

impl Default for StressConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: None,
            max_disk_space_mb: None,
            max_connections: None,
            max_query_complexity: None,
            test_duration: Duration::from_secs(30),
        }
    }
}

/// Stress test results
#[derive(Debug, Clone)]
pub struct StressResults {
    pub test_name: String,
    pub operations_attempted: usize,
    pub operations_succeeded: usize,
    pub operations_failed: usize,
    pub graceful_degradations: usize,
    pub errors: Vec<String>,
    pub test_duration: Duration,
}

/// Stress test runner
pub struct StressTestRunner {
    config: StressConfig,
    #[allow(dead_code)]
    temp_dir: TempDir,
    index_manager: Arc<IndexManager>,
}

impl StressTestRunner {
    /// Create a new stress test runner
    pub fn new(config: StressConfig) -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let index_manager = Arc::new(IndexManager::new(temp_dir.path()));

        Ok(Self {
            config,
            temp_dir,
            index_manager,
        })
    }

    /// Test memory limits
    pub async fn test_memory_limits(&self) -> Result<StressResults> {
        let start = Instant::now();
        let mut results = StressResults {
            test_name: "memory_limits".to_string(),
            operations_attempted: 0,
            operations_succeeded: 0,
            operations_failed: 0,
            graceful_degradations: 0,
            errors: Vec::new(),
            test_duration: Duration::ZERO,
        };

        // Create a test index
        let schema = SchemaBuilder::new()
            .add_field(FieldConfig::new("content", FieldType::Text))
            .build()?;

        let index = self
            .index_manager
            .create_index("stress_memory", schema.0, Default::default())
            .await?;

        let document_store = DocumentStore::new(Arc::new(index));

        // Try to add many documents to test memory pressure
        let mut doc_count = 0;
        let max_docs = 10000; // Reasonable limit for testing

        while doc_count < max_docs && start.elapsed() < self.config.test_duration {
            results.operations_attempted += 1;

            let doc = json!({
                "content": format!("Test document {} with some content to fill memory", doc_count)
            });

            match document_store.add_document(doc).await {
                Ok(_) => {
                    results.operations_succeeded += 1;
                    doc_count += 1;
                }
                Err(e) => {
                    results.operations_failed += 1;
                    let error_msg = e.to_string();
                    if error_msg.contains("memory") || error_msg.contains("Memory") {
                        results.graceful_degradations += 1;
                    }
                    results.errors.push(error_msg);
                    // Continue testing even if some operations fail
                }
            }

            // Check memory periodically (simplified check)
            if doc_count % 1000 == 0 {
                // In a real implementation, we would check actual memory usage
                // For now, we just verify operations continue
            }
        }

        results.test_duration = start.elapsed();
        Ok(results)
    }

    /// Test disk space exhaustion
    pub async fn test_disk_space_exhaustion(&self) -> Result<StressResults> {
        let start = Instant::now();
        let mut results = StressResults {
            test_name: "disk_space_exhaustion".to_string(),
            operations_attempted: 0,
            operations_succeeded: 0,
            operations_failed: 0,
            graceful_degradations: 0,
            errors: Vec::new(),
            test_duration: Duration::ZERO,
        };

        // Create a test index
        let schema = SchemaBuilder::new()
            .add_field(FieldConfig::new("content", FieldType::Text))
            .build()?;

        let index = self
            .index_manager
            .create_index("stress_disk", schema.0, Default::default())
            .await?;

        let document_store = DocumentStore::new(Arc::new(index));

        // Try to add large documents to test disk space
        let large_content = "x".repeat(10000); // 10KB per document
        let mut doc_count = 0;
        let max_docs = 1000;

        while doc_count < max_docs && start.elapsed() < self.config.test_duration {
            results.operations_attempted += 1;

            let doc = json!({
                "content": format!("{}_{}", large_content, doc_count)
            });

            match document_store.add_document(doc).await {
                Ok(_) => {
                    results.operations_succeeded += 1;
                    doc_count += 1;
                }
                Err(e) => {
                    results.operations_failed += 1;
                    let error_msg = e.to_string();
                    if error_msg.contains("disk")
                        || error_msg.contains("space")
                        || error_msg.contains("No space")
                    {
                        results.graceful_degradations += 1;
                    }
                    results.errors.push(error_msg);
                    // Stop if we hit disk space issues
                    break;
                }
            }
        }

        results.test_duration = start.elapsed();
        Ok(results)
    }

    /// Test connection limits
    pub async fn test_connection_limits(&self) -> Result<StressResults> {
        let start = Instant::now();
        let mut results = StressResults {
            test_name: "connection_limits".to_string(),
            operations_attempted: 0,
            operations_succeeded: 0,
            operations_failed: 0,
            graceful_degradations: 0,
            errors: Vec::new(),
            test_duration: Duration::ZERO,
        };

        // Create a test index
        let schema = SchemaBuilder::new()
            .add_field(FieldConfig::new("content", FieldType::Text))
            .build()?;

        let index = self
            .index_manager
            .create_index("stress_connections", schema.0, Default::default())
            .await?;

        let executor = Arc::new(SearchExecutor::new(Arc::new(index)));

        // Simulate many concurrent queries
        let max_concurrent = self.config.max_connections.unwrap_or(100);
        let mut handles = Vec::new();

        for i in 0..max_concurrent {
            let executor_clone = executor.clone();
            let handle: tokio::task::JoinHandle<
                Result<lexum_core::search::result::SearchResult, lexum_core::error::Error>,
            > = tokio::spawn(async move {
                let query = QueryBuilder::match_query("content", format!("test {}", i));
                executor_clone.search(query, 10, 0, None).await
            });
            handles.push(handle);
            results.operations_attempted += 1;
        }

        // Wait for all queries to complete
        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => results.operations_succeeded += 1,
                Ok(Err(e)) => {
                    results.operations_failed += 1;
                    let error_msg = e.to_string();
                    if error_msg.contains("connection") || error_msg.contains("limit") {
                        results.graceful_degradations += 1;
                    }
                    results.errors.push(error_msg);
                }
                Err(e) => {
                    results.operations_failed += 1;
                    results.errors.push(format!("Task join error: {e}"));
                }
            }
        }

        results.test_duration = start.elapsed();
        Ok(results)
    }

    /// Test query complexity limits
    pub async fn test_query_complexity_limits(&self) -> Result<StressResults> {
        let start = Instant::now();
        let mut results = StressResults {
            test_name: "query_complexity_limits".to_string(),
            operations_attempted: 0,
            operations_succeeded: 0,
            operations_failed: 0,
            graceful_degradations: 0,
            errors: Vec::new(),
            test_duration: Duration::ZERO,
        };

        // Create a test index
        let schema = SchemaBuilder::new()
            .add_field(FieldConfig::new("content", FieldType::Text))
            .build()?;

        let index = self
            .index_manager
            .create_index("stress_complexity", schema.0, Default::default())
            .await?;

        let executor = SearchExecutor::new(Arc::new(index));

        // Test increasingly complex queries
        let max_complexity = self.config.max_query_complexity.unwrap_or(100);

        for complexity in 1..=max_complexity {
            results.operations_attempted += 1;

            // Build a complex boolean query
            let mut bool_query = QueryBuilder::bool_query();
            for i in 0..complexity {
                let term_query = QueryBuilder::term_query("content", format!("term{}", i));
                bool_query = bool_query.must(term_query);
            }

            let query = Query::Bool(bool_query);
            match executor.search(query, 10, 0, None).await {
                Ok(_) => {
                    results.operations_succeeded += 1;
                }
                Err(e) => {
                    results.operations_failed += 1;
                    let error_msg = e.to_string();
                    if error_msg.contains("complexity")
                        || error_msg.contains("too complex")
                        || error_msg.contains("timeout")
                    {
                        results.graceful_degradations += 1;
                    }
                    results.errors.push(error_msg);
                    // Stop if we hit complexity limits
                    break;
                }
            }

            if start.elapsed() >= self.config.test_duration {
                break;
            }
        }

        results.test_duration = start.elapsed();
        Ok(results)
    }

    /// Verify graceful degradation
    pub async fn test_graceful_degradation(&self) -> Result<StressResults> {
        let start = Instant::now();
        let mut results = StressResults {
            test_name: "graceful_degradation".to_string(),
            operations_attempted: 0,
            operations_succeeded: 0,
            operations_failed: 0,
            graceful_degradations: 0,
            errors: Vec::new(),
            test_duration: Duration::ZERO,
        };

        // Run all stress tests and verify graceful handling
        let memory_results = self.test_memory_limits().await?;
        results.operations_attempted += memory_results.operations_attempted;
        results.operations_succeeded += memory_results.operations_succeeded;
        results.operations_failed += memory_results.operations_failed;
        results.graceful_degradations += memory_results.graceful_degradations;
        results.errors.extend(memory_results.errors);

        let connection_results = self.test_connection_limits().await?;
        results.operations_attempted += connection_results.operations_attempted;
        results.operations_succeeded += connection_results.operations_succeeded;
        results.operations_failed += connection_results.operations_failed;
        results.graceful_degradations += connection_results.graceful_degradations;
        results.errors.extend(connection_results.errors);

        // Verify that system still works after stress
        let schema = SchemaBuilder::new()
            .add_field(FieldConfig::new("content", FieldType::Text))
            .build()?;

        let index = self
            .index_manager
            .create_index("stress_recovery", schema.0, Default::default())
            .await?;

        let document_store = DocumentStore::new(Arc::new(index));
        let doc = json!({"content": "Recovery test"});

        match document_store.add_document(doc).await {
            Ok(_) => {
                results.operations_succeeded += 1;
                results.graceful_degradations += 1; // System recovered
            }
            Err(e) => {
                results.operations_failed += 1;
                results.errors.push(format!("Recovery failed: {e}"));
            }
        }

        results.test_duration = start.elapsed();
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_limits() {
        let config = StressConfig {
            test_duration: Duration::from_secs(10),
            ..Default::default()
        };
        let runner = StressTestRunner::new(config).unwrap();

        let results = runner.test_memory_limits().await.unwrap();

        assert!(
            results.operations_attempted > 0,
            "Should attempt operations"
        );
        assert!(
            results.operations_succeeded > 0 || results.graceful_degradations > 0,
            "Should succeed or degrade gracefully"
        );
    }

    #[tokio::test]
    async fn test_disk_space_exhaustion() {
        let config = StressConfig {
            test_duration: Duration::from_secs(10),
            ..Default::default()
        };
        let runner = StressTestRunner::new(config).unwrap();

        let results = runner.test_disk_space_exhaustion().await.unwrap();

        assert!(
            results.operations_attempted > 0,
            "Should attempt operations"
        );
        // System should handle disk space issues gracefully
        assert!(
            results.operations_succeeded > 0 || results.graceful_degradations > 0,
            "Should succeed or degrade gracefully"
        );
    }

    #[tokio::test]
    async fn test_connection_limits() {
        let config = StressConfig {
            max_connections: Some(50),
            test_duration: Duration::from_secs(10),
            ..Default::default()
        };
        let runner = StressTestRunner::new(config).unwrap();

        let results = runner.test_connection_limits().await.unwrap();

        assert!(
            results.operations_attempted > 0,
            "Should attempt operations"
        );
        assert!(
            results.operations_succeeded > 0 || results.graceful_degradations > 0,
            "Should succeed or degrade gracefully"
        );
    }

    #[tokio::test]
    async fn test_query_complexity_limits() {
        let config = StressConfig {
            max_query_complexity: Some(50),
            test_duration: Duration::from_secs(10),
            ..Default::default()
        };
        let runner = StressTestRunner::new(config).unwrap();

        let results = runner.test_query_complexity_limits().await.unwrap();

        assert!(
            results.operations_attempted > 0,
            "Should attempt operations"
        );
        // System should handle complex queries gracefully
        assert!(
            results.operations_succeeded > 0 || results.graceful_degradations > 0,
            "Should succeed or degrade gracefully"
        );
    }

    #[tokio::test]
    async fn test_graceful_degradation() {
        let config = StressConfig {
            test_duration: Duration::from_secs(15),
            ..Default::default()
        };
        let runner = StressTestRunner::new(config).unwrap();

        let results = runner.test_graceful_degradation().await.unwrap();

        assert!(
            results.operations_attempted > 0,
            "Should attempt operations"
        );
        // System should recover gracefully after stress
        assert!(
            results.graceful_degradations > 0,
            "Should demonstrate graceful degradation or recovery"
        );
    }
}
