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
fn check_rollover_conditions(
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
fn parse_duration(duration: &str) -> Option<u64> {
    if duration.is_empty() {
        return None;
    }

    let (num_str, unit) = if let Some(stripped) = duration.strip_suffix('d') {
        (stripped, "d")
    } else if let Some(stripped) = duration.strip_suffix('h') {
        (stripped, "h")
    } else if let Some(stripped) = duration.strip_suffix('m') {
        (stripped, "m")
    } else if let Some(stripped) = duration.strip_suffix('s') {
        (stripped, "s")
    } else {
        return None;
    };

    let num: u64 = num_str.parse().ok()?;

    let millis = match unit {
        "s" => num * 1000,
        "m" => num * 60 * 1000,
        "h" => num * 60 * 60 * 1000,
        "d" => num * 24 * 60 * 60 * 1000,
        _ => return None,
    };

    Some(millis)
}

/// Parse size string (e.g., "5gb", "1tb", "100mb")
fn parse_size(size: &str) -> Option<u64> {
    if size.is_empty() {
        return None;
    }

    let (num_str, unit) = if let Some(stripped) = size.strip_suffix("gb") {
        (stripped, "gb")
    } else if let Some(stripped) = size.strip_suffix("tb") {
        (stripped, "tb")
    } else if let Some(stripped) = size.strip_suffix("mb") {
        (stripped, "mb")
    } else if let Some(stripped) = size.strip_suffix("kb") {
        (stripped, "kb")
    } else if let Some(stripped) = size.strip_suffix("b") {
        (stripped, "b")
    } else {
        return None;
    };

    let num: f64 = num_str.parse().ok()?;

    let bytes = match unit {
        "b" => num as u64,
        "kb" => (num * 1024.0) as u64,
        "mb" => (num * 1024.0 * 1024.0) as u64,
        "gb" => (num * 1024.0 * 1024.0 * 1024.0) as u64,
        "tb" => (num * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64,
        _ => return None,
    };

    Some(bytes)
}

/// Generate rollover index name
fn generate_rollover_index_name(original_name: &str) -> String {
    // Find the last dash followed by a number
    if let Some(last_dash) = original_name.rfind('-') {
        let suffix = &original_name[last_dash + 1..];
        if let Ok(num) = suffix.parse::<u64>() {
            let base_name = &original_name[..last_dash];
            return format!("{}-{}", base_name, num + 1);
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

    // In a real implementation, we would:
    // 1. Copy all documents from old index to new index
    // 2. Update any aliases pointing to the old index
    // 3. Optionally close or delete the old index
    // 4. Update any templates or configurations

    tracing::info!("Rollover completed: '{}' -> '{}'", old_index, new_index);
    Ok(())
}

/// Get rollover conditions for an index
#[utoipa::path(
    get,
    path = "/api/v1/indices/{index_name}/_rollover",
    params(
        ("index_name" = String, Path, description = "Index name")
    ),
    responses(
        (status = 200, description = "Rollover conditions retrieved successfully", body = RolloverConditions),
        (status = 404, description = "Index not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Indices"
)]
pub async fn get_rollover_conditions(
    State(_state): State<AppState>,
    Path(index_name): Path<String>,
) -> ApiResult<Json<RolloverConditions>> {
    tracing::info!("Getting rollover conditions for index '{}'", index_name);

    // In a real implementation, this would retrieve the configured
    // rollover conditions for the index from storage
    let conditions = RolloverConditions {
        max_age: Some("30d".to_string()),
        max_size: Some("5gb".to_string()),
        max_docs: Some(1000000),
        max_primary_shard_size: Some("1gb".to_string()),
    };

    Ok(Json(conditions))
}

/// Update rollover conditions for an index
#[utoipa::path(
    put,
    path = "/api/v1/indices/{index_name}/_rollover",
    params(
        ("index_name" = String, Path, description = "Index name")
    ),
    request_body = RolloverConditions,
    responses(
        (status = 200, description = "Rollover conditions updated successfully"),
        (status = 404, description = "Index not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Indices"
)]
pub async fn update_rollover_conditions(
    State(_state): State<AppState>,
    Path(index_name): Path<String>,
    Json(conditions): Json<RolloverConditions>,
) -> ApiResult<()> {
    tracing::info!("Updating rollover conditions for index '{}'", index_name);

    // In a real implementation, this would store the rollover
    // conditions for the index in persistent storage
    tracing::debug!("Rollover conditions: {:?}", conditions);

    Ok(())
}
