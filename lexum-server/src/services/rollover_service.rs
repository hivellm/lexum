//! Automatic rollover service

use crate::handlers::index::AppState;
use crate::handlers::rollover::{
    IndexStats, RolloverConditions, check_rollover_conditions, generate_rollover_index_name,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use tracing::{error, info};

/// Rollover configuration for an index
#[derive(Debug, Clone)]
pub struct IndexRolloverConfig {
    /// The conditions that trigger a rollover
    pub conditions: RolloverConditions,
    /// Whether automatic rollover is enabled for this index
    pub enabled: bool,
    /// The last time rollover conditions were checked
    pub last_checked: Instant,
}

/// Automatic rollover service
pub struct RolloverService {
    state: Arc<AppState>,
    configs: Arc<RwLock<HashMap<String, IndexRolloverConfig>>>,
    check_interval: Duration,
}

impl RolloverService {
    /// Create a new rollover service
    pub fn new(state: Arc<AppState>, check_interval: Duration) -> Self {
        Self {
            state,
            configs: Arc::new(RwLock::new(HashMap::new())),
            check_interval,
        }
    }

    /// Start the rollover service
    pub async fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(self.check_interval);

            loop {
                interval.tick().await;

                if let Err(e) = self.check_and_rollover().await {
                    error!("Error in rollover service: {}", e);
                }
            }
        })
    }

    /// Check all indices for rollover conditions and perform rollover if needed
    async fn check_and_rollover(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let configs = self.configs.read().await;
        let indices = self.state.index_manager.list_indices();

        for (index_name, config) in configs.iter() {
            if !config.enabled {
                continue;
            }

            if !indices.contains(index_name) {
                continue;
            }

            // Get current index statistics
            let index_stats = self.state.index_manager.get_index_stats(index_name).await?;
            let stats = IndexStats {
                num_docs: index_stats.num_docs,
                size_in_bytes: index_stats.num_docs * 1024, // Mock: assume 1KB per document
                age_in_millis: config.last_checked.elapsed().as_millis() as u64,
                num_primary_shards: 1, // Mock: assume single shard
            };

            // Check if rollover conditions are met
            let (conditions_met, rolled_over_due_to) =
                check_rollover_conditions(&config.conditions, &stats);

            if conditions_met {
                info!(
                    "Rollover conditions met for index '{}': {}",
                    index_name,
                    rolled_over_due_to.as_deref().unwrap_or("unknown")
                );

                // Perform automatic rollover
                if let Err(e) = self.perform_automatic_rollover(index_name).await {
                    error!(
                        "Failed to perform automatic rollover for index '{}': {}",
                        index_name, e
                    );
                }
            }
        }

        Ok(())
    }

    /// Perform automatic rollover for an index
    async fn perform_automatic_rollover(
        &self,
        index_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let new_index_name = generate_rollover_index_name(index_name);

        info!(
            "Performing automatic rollover: '{}' -> '{}'",
            index_name, new_index_name
        );

        // Get the schema from the old index
        let old_index_info = self.state.index_manager.get_index(index_name)?;
        let schema = old_index_info.schema();
        let settings = old_index_info.settings().clone();

        // Create new index with the same schema
        self.state
            .index_manager
            .create_index(&new_index_name, schema, settings)
            .await?;

        // Update any aliases pointing to the old index
        // In a real implementation, we would update aliases here

        info!(
            "Automatic rollover completed: '{}' -> '{}'",
            index_name, new_index_name
        );
        Ok(())
    }

    /// Configure rollover for an index
    pub async fn configure_rollover(
        &self,
        index_name: String,
        conditions: RolloverConditions,
        enabled: bool,
    ) {
        let mut configs = self.configs.write().await;
        configs.insert(
            index_name,
            IndexRolloverConfig {
                conditions,
                enabled,
                last_checked: Instant::now(),
            },
        );
    }

    /// Get rollover configuration for an index
    pub async fn get_rollover_config(&self, index_name: &str) -> Option<IndexRolloverConfig> {
        let configs = self.configs.read().await;
        configs.get(index_name).cloned()
    }

    /// Remove rollover configuration for an index
    pub async fn remove_rollover_config(&self, index_name: &str) {
        let mut configs = self.configs.write().await;
        configs.remove(index_name);
    }

    /// List all configured rollover indices
    pub async fn list_rollover_configs(&self) -> HashMap<String, IndexRolloverConfig> {
        let configs = self.configs.read().await;
        configs.clone()
    }
}

/// Start the rollover service
pub async fn start_rollover_service(state: Arc<AppState>) -> tokio::task::JoinHandle<()> {
    let service = RolloverService::new(state, Duration::from_secs(60)); // Check every minute
    service.start().await
}
