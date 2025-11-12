//! Index rollover handler

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Rollover conditions
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct RolloverConditions {
    /// Maximum age (e.g., "7d", "30d", "1h")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<String>,
    /// Maximum size (e.g., "5gb", "1tb")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<String>,
    /// Maximum number of documents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_docs: Option<u64>,
    /// Maximum number of primary shards
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_primary_shard_size: Option<String>,
}

/// Rollover request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RolloverRequest {
    /// Rollover conditions
    #[serde(default)]
    pub conditions: RolloverConditions,
    /// New index name (optional, will be auto-generated if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_index: Option<String>,
    /// Dry run - check conditions without actually rolling over
    #[serde(default)]
    pub dry_run: bool,
}

/// Rollover response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RolloverResponse {
    /// Whether rollover was acknowledged
    pub acknowledged: bool,
    /// Whether conditions were met
    pub conditions_met: bool,
    /// Old index name
    pub old_index: String,
    /// New index name
    pub new_index: String,
    /// Whether this was a dry run
    pub dry_run: bool,
    /// Rolled over due to which condition
    pub rolled_over_due_to: Option<String>,
    /// Index statistics
    pub index_stats: IndexStats,
}

/// Index statistics for rollover
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexStats {
    /// Number of documents
    pub num_docs: u64,
    /// Index size in bytes
    pub size_in_bytes: u64,
    /// Index age in milliseconds
    pub age_in_millis: u64,
    /// Number of primary shards
    pub num_primary_shards: u32,
}

/// Rollover index handler
#[utoipa::path(
    post,
    path = "/api/v1/indices/{index_name}/_rollover",
    params(
        ("index_name" = String, Path, description = "Index name to rollover")
    ),
    request_body = RolloverRequest,
    responses(
        (status = 200, description = "Rollover completed successfully", body = RolloverResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Index not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Indices"
)]
pub async fn rollover_index(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
    Json(request): Json<RolloverRequest>,
) -> ApiResult<Json<RolloverResponse>> {
    tracing::info!("Rollover request for index '{}'", index_name);

    // Check if index exists
    if !state.index_manager.index_exists(&index_name) {
        return Err(ApiError::IndexNotFound(index_name));
    }

    // Get index statistics
    let index_stats = state
        .index_manager
        .get_index_stats(&index_name)
        .await
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    let stats = IndexStats {
        num_docs: index_stats.num_docs,
        size_in_bytes: index_stats.num_docs * 1024, // Mock: assume 1KB per document
        age_in_millis: 0,                           // Mock: assume new index
        num_primary_shards: 1,                      // Mock: assume single shard
    };

    // Check rollover conditions
    let (conditions_met, rolled_over_due_to) =
        check_rollover_conditions(&request.conditions, &stats);

    if !conditions_met && !request.dry_run {
        return Ok(Json(RolloverResponse {
            acknowledged: true,
            conditions_met: false,
            old_index: index_name.clone(),
            new_index: String::new(),
            dry_run: false,
            rolled_over_due_to: None,
            index_stats: stats,
        }));
    }

    // Generate new index name if not provided
    let new_index_name = if let Some(name) = request.new_index {
        name
    } else {
        generate_rollover_index_name(&index_name)
    };

    if request.dry_run {
        return Ok(Json(RolloverResponse {
            acknowledged: true,
            conditions_met,
            old_index: index_name,
            new_index: new_index_name,
            dry_run: true,
            rolled_over_due_to,
            index_stats: stats,
        }));
    }

    // Perform actual rollover
    perform_rollover(&state, &index_name, &new_index_name).await?;

    Ok(Json(RolloverResponse {
        acknowledged: true,
        conditions_met,
        old_index: index_name,
        new_index: new_index_name,
        dry_run: false,
        rolled_over_due_to,
        index_stats: stats,
    }))
}

/// Check if rollover conditions are met
pub fn check_rollover_conditions(
    conditions: &RolloverConditions,
    stats: &IndexStats,
) -> (bool, Option<String>) {
    // Check max age
    if let Some(ref max_age) = conditions.max_age {
        if let Some(age_limit) = parse_duration(max_age) {
            if stats.age_in_millis >= age_limit {
                return (true, Some(format!("max_age:{max_age}")));
            }
        }
    }

    // Check max size
    if let Some(ref max_size) = conditions.max_size {
        if let Some(size_limit) = parse_size(max_size) {
            if stats.size_in_bytes >= size_limit {
                return (true, Some(format!("max_size:{max_size}")));
            }
        }
    }

    // Check max docs
    if let Some(max_docs) = conditions.max_docs {
        if stats.num_docs >= max_docs {
            return (true, Some(format!("max_docs:{max_docs}")));
        }
    }

    // Check max primary shard size
    if let Some(ref max_primary_shard_size) = conditions.max_primary_shard_size {
        if let Some(shard_size_limit) = parse_size(max_primary_shard_size) {
            let avg_shard_size = stats.size_in_bytes / u64::from(stats.num_primary_shards);
            if avg_shard_size >= shard_size_limit {
                return (
                    true,
                    Some(format!("max_primary_shard_size:{max_primary_shard_size}")),
                );
            }
        }
    }

    (false, None)
}

/// Parse duration string (e.g., "7d", "30d", "1h")
pub(crate) fn parse_duration(duration: &str) -> Option<u64> {
    if duration.is_empty() {
        return None;
    }

    let duration = duration.to_lowercase();
    let (number, unit) = if duration.ends_with('d') {
        (duration.trim_end_matches('d'), "days")
    } else if duration.ends_with('h') {
        (duration.trim_end_matches('h'), "hours")
    } else if duration.ends_with('m') {
        (duration.trim_end_matches('m'), "minutes")
    } else if duration.ends_with('s') {
        (duration.trim_end_matches('s'), "seconds")
    } else {
        return None;
    };

    let number: u64 = number.parse().ok()?;

    let millis = match unit {
        "days" => number * 24 * 60 * 60 * 1000,
        "hours" => number * 60 * 60 * 1000,
        "minutes" => number * 60 * 1000,
        "seconds" => number * 1000,
        _ => return None,
    };

    Some(millis)
}

/// Parse size string (e.g., "5gb", "1tb", "500mb")
pub(crate) fn parse_size(size: &str) -> Option<u64> {
    if size.is_empty() {
        return None;
    }

    let size = size.to_lowercase();
    let (number, unit) = if size.ends_with("tb") {
        (size.trim_end_matches("tb"), "terabytes")
    } else if size.ends_with("gb") {
        (size.trim_end_matches("gb"), "gigabytes")
    } else if size.ends_with("mb") {
        (size.trim_end_matches("mb"), "megabytes")
    } else if size.ends_with("kb") {
        (size.trim_end_matches("kb"), "kilobytes")
    } else if size.ends_with('b') {
        (size.trim_end_matches('b'), "bytes")
    } else {
        return None;
    };

    let number: f64 = number.parse().ok()?;

    let bytes = match unit {
        "terabytes" => (number * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64,
        "gigabytes" => (number * 1024.0 * 1024.0 * 1024.0) as u64,
        "megabytes" => (number * 1024.0 * 1024.0) as u64,
        "kilobytes" => (number * 1024.0) as u64,
        "bytes" => number as u64,
        _ => return None,
    };

    Some(bytes)
}

/// Generate rollover index name
pub fn generate_rollover_index_name(original_name: &str) -> String {
    // Find the last dash followed by a number
    if let Some(last_dash) = original_name.rfind('-') {
        let suffix = &original_name[last_dash + 1..];
        if let Ok(num) = suffix.parse::<u64>() {
            let base_name = &original_name[..last_dash];
            return format!("{}-{:06}", base_name, num + 1);
        }
    }

    // If no number suffix, add -000001
    format!("{original_name}-000001")
}

/// Perform the actual rollover operation
async fn perform_rollover(
    state: &AppState,
    old_index: &str,
    new_index: &str,
) -> Result<(), ApiError> {
    tracing::info!(
        "Performing rollover from '{}' to '{}'",
        old_index,
        new_index
    );

    // Get the schema from the old index
    let old_index_info = state
        .index_manager
        .get_index(old_index)
        .map_err(|_| ApiError::IndexNotFound(old_index.to_string()))?;

    // Create new index with the same schema
    let schema = old_index_info.schema();
    let settings = old_index_info.settings().clone();

    state
        .index_manager
        .create_index(new_index, schema, settings)
        .await
        .map_err(|e| ApiError::InvalidRequest(e.to_string()))?;

    // Copy all documents from old index to new index
    copy_documents(state, old_index, new_index).await?;

    // Update any aliases pointing to the old index to point to the new index
    update_aliases_for_rollover(state, old_index, new_index).await?;

    tracing::info!("Rollover completed: '{}' -> '{}'", old_index, new_index);
    Ok(())
}

/// Copy all documents from source index to destination index
async fn copy_documents(
    state: &AppState,
    source_index: &str,
    dest_index: &str,
) -> Result<(), ApiError> {
    use lexum_core::document::store::DocumentStore;
    use lexum_core::query::Query;
    use lexum_core::search::executor::SearchExecutor;
    use std::sync::Arc;

    tracing::info!(
        "Copying documents from '{}' to '{}'",
        source_index,
        dest_index
    );

    let source_index_arc = Arc::new(
        state
            .index_manager
            .get_index(source_index)
            .map_err(|_| ApiError::IndexNotFound(source_index.to_string()))?,
    );
    let dest_index_arc = Arc::new(
        state
            .index_manager
            .get_index(dest_index)
            .map_err(|_| ApiError::IndexNotFound(dest_index.to_string()))?,
    );

    let search_executor = SearchExecutor::new(source_index_arc.clone());
    let dest_store = DocumentStore::new(dest_index_arc);

    // Copy documents in batches
    let batch_size = 100;
    let mut offset = 0;
    let mut total_copied = 0;

    loop {
        // Search for documents in current batch
        let search_result = search_executor
            .search(Query::MatchAll, batch_size, offset, None)
            .await
            .map_err(|e| ApiError::InvalidRequest(format!("Search failed: {e}")))?;

        if search_result.hits.is_empty() {
            break;
        }

        // Copy each document
        for hit in &search_result.hits {
            let doc_id = hit.id.clone();
            let document = hit.source.clone();

            dest_store
                .add_document_with_id(doc_id, document)
                .await
                .map_err(|e| ApiError::InvalidRequest(format!("Failed to copy document: {e}")))?;

            total_copied += 1;
        }

        offset += batch_size;

        // Log progress every 1000 documents
        if total_copied % 1000 == 0 {
            tracing::info!("Copied {} documents so far...", total_copied);
        }
    }

    tracing::info!(
        "Copied {} documents from '{}' to '{}'",
        total_copied,
        source_index,
        dest_index
    );
    Ok(())
}

/// Update aliases pointing to old index to point to new index
async fn update_aliases_for_rollover(
    state: &AppState,
    old_index: &str,
    new_index: &str,
) -> Result<(), ApiError> {
    use lexum_core::index::alias::AliasOperationsRequest;

    tracing::info!("Updating aliases from '{}' to '{}'", old_index, new_index);

    // Get all aliases for the old index
    let aliases = state.index_manager.get_aliases_for_index(old_index);

    if aliases.is_empty() {
        tracing::info!("No aliases found for index '{}'", old_index);
        return Ok(());
    }

    // Update each alias to point to the new index
    for alias in aliases {
        let alias_name = alias.name.as_str().to_string();
        let old_index_name = lexum_core::types::IndexName::new(old_index);
        let new_index_name = lexum_core::types::IndexName::new(new_index);

        // Remove old index from alias and add new index
        let operations = AliasOperationsRequest {
            actions: vec![
                lexum_core::index::alias::AliasAction::Remove {
                    alias: lexum_core::index::alias::AliasName::new(&alias_name),
                    indices: vec![old_index_name.clone()],
                },
                lexum_core::index::alias::AliasAction::Add {
                    alias: lexum_core::index::alias::AliasName::new(&alias_name),
                    indices: vec![new_index_name],
                    config: Some(alias.config),
                },
            ],
        };

        state
            .index_manager
            .execute_alias_operations(operations)
            .map_err(|e| {
                ApiError::InvalidRequest(format!("Failed to update alias '{}': {e}", alias_name))
            })?;

        tracing::info!("Updated alias '{}' to point to '{}'", alias_name, new_index);
    }

    Ok(())
}

/// Get rollover conditions for an index
#[utoipa::path(
    get,
    path = "/api/v1/indices/{index_name}/_rollover",
    params(
        ("index_name" = String, Path, description = "Index name to get rollover conditions for")
    ),
    responses(
        (status = 200, description = "Rollover conditions retrieved successfully", body = RolloverConditions),
        (status = 404, description = "Index not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Indices"
)]
pub async fn get_rollover_conditions(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
) -> ApiResult<Json<RolloverConditions>> {
    tracing::info!("Getting rollover conditions for index '{}'", index_name);

    // Check if index exists
    if !state.index_manager.index_exists(&index_name) {
        return Err(ApiError::IndexNotFound(index_name));
    }

    // In a real implementation, we would store and retrieve rollover conditions
    // For now, return default conditions
    let conditions = RolloverConditions::default();

    Ok(Json(conditions))
}

/// Update rollover conditions for an index
#[utoipa::path(
    put,
    path = "/api/v1/indices/{index_name}/_rollover",
    params(
        ("index_name" = String, Path, description = "Index name to update rollover conditions for")
    ),
    request_body = RolloverConditions,
    responses(
        (status = 200, description = "Rollover conditions updated successfully", body = serde_json::Value),
        (status = 404, description = "Index not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Indices"
)]
pub async fn update_rollover_conditions(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
    Json(conditions): Json<RolloverConditions>,
) -> ApiResult<Json<serde_json::Value>> {
    tracing::info!("Updating rollover conditions for index '{}'", index_name);

    // Check if index exists
    if !state.index_manager.index_exists(&index_name) {
        return Err(ApiError::IndexNotFound(index_name));
    }

    // In a real implementation, we would store the rollover conditions
    // For now, just acknowledge the update
    let response = serde_json::json!({
        "acknowledged": true,
        "index": index_name,
        "conditions": conditions
    });

    Ok(Json(response))
}
