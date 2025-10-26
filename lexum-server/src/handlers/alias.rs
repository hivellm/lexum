//! Alias management handlers

use crate::error::{ApiError, ApiResult};
use axum::Json;
use axum::extract::{Path, State};
use lexum_core::{
    AliasConfig, AliasOperationsRequest, AliasOperationsResponse,
    IndexAlias, types::IndexName,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Create alias request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAliasRequest {
    /// Target indices
    pub indices: Vec<String>,
    /// Alias configuration
    pub config: Option<AliasConfig>,
}

/// Create alias response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAliasResponse {
    /// Whether the operation was acknowledged
    pub acknowledged: bool,
    /// Created alias
    pub alias: IndexAlias,
}

/// Get alias response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetAliasResponse {
    /// Alias information
    pub alias: IndexAlias,
}

/// List aliases response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListAliasesResponse {
    /// List of aliases
    pub aliases: Vec<IndexAlias>,
}

/// Add indices to alias request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AddIndicesToAliasRequest {
    /// Indices to add
    pub indices: Vec<String>,
}

/// Remove indices from alias request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RemoveIndicesFromAliasRequest {
    /// Indices to remove
    pub indices: Vec<String>,
}

/// Create a new alias
///
/// # Arguments
///
/// * `State(state)` - Application state
/// * `Path(alias_name)` - Alias name
/// * `Json(request)` - Create alias request
///
/// # Returns
///
/// * `ApiResult<Json<CreateAliasResponse>>` - Created alias information
#[utoipa::path(
    post,
    path = "/_aliases/{alias_name}",
    params(
        ("alias_name" = String, Path, description = "Alias name")
    ),
    request_body = CreateAliasRequest,
    responses(
        (status = 200, description = "Alias created successfully", body = CreateAliasResponse),
        (status = 400, description = "Bad request", body = ApiError),
        (status = 409, description = "Alias already exists", body = ApiError),
        (status = 404, description = "Target index not found", body = ApiError),
    ),
    tag = "Aliases"
)]
pub async fn create_alias(
    State(state): State<crate::handlers::index::AppState>,
    Path(alias_name): Path<String>,
    Json(request): Json<CreateAliasRequest>,
) -> ApiResult<Json<CreateAliasResponse>> {
    // Convert string indices to IndexName
    let indices: Vec<IndexName> = request
        .indices
        .into_iter()
        .map(IndexName::new)
        .collect();

    // Create the alias
    let alias = state
        .index_manager
        .create_alias(alias_name.as_str(), indices)
        .map_err(|e| ApiError::from(e))?;

    Ok(Json(CreateAliasResponse {
        acknowledged: true,
        alias,
    }))
}

/// Get an alias by name
///
/// # Arguments
///
/// * `State(state)` - Application state
/// * `Path(alias_name)` - Alias name
///
/// # Returns
///
/// * `ApiResult<Json<GetAliasResponse>>` - Alias information
#[utoipa::path(
    get,
    path = "/_aliases/{alias_name}",
    params(
        ("alias_name" = String, Path, description = "Alias name")
    ),
    responses(
        (status = 200, description = "Alias found", body = GetAliasResponse),
        (status = 404, description = "Alias not found", body = ApiError),
    ),
    tag = "Aliases"
)]
pub async fn get_alias(
    State(state): State<crate::handlers::index::AppState>,
    Path(alias_name): Path<String>,
) -> ApiResult<Json<GetAliasResponse>> {
    let alias = state
        .index_manager
        .get_alias(&alias_name)
        .map_err(|e| ApiError::from(e))?;

    Ok(Json(GetAliasResponse { alias }))
}

/// Delete an alias
///
/// # Arguments
///
/// * `State(state)` - Application state
/// * `Path(alias_name)` - Alias name
///
/// # Returns
///
/// * `ApiResult<()>` - Success response
#[utoipa::path(
    delete,
    path = "/_aliases/{alias_name}",
    params(
        ("alias_name" = String, Path, description = "Alias name")
    ),
    responses(
        (status = 200, description = "Alias deleted successfully"),
        (status = 404, description = "Alias not found", body = ApiError),
    ),
    tag = "Aliases"
)]
pub async fn delete_alias(
    State(state): State<crate::handlers::index::AppState>,
    Path(alias_name): Path<String>,
) -> ApiResult<()> {
    state
        .index_manager
        .delete_alias(&alias_name)
        .map_err(|e| ApiError::from(e))?;

    Ok(())
}

/// List all aliases
///
/// # Arguments
///
/// * `State(state)` - Application state
///
/// # Returns
///
/// * `ApiResult<Json<ListAliasesResponse>>` - List of aliases
#[utoipa::path(
    get,
    path = "/_aliases",
    responses(
        (status = 200, description = "List of aliases", body = ListAliasesResponse),
    ),
    tag = "Aliases"
)]
pub async fn list_aliases(
    State(state): State<crate::handlers::index::AppState>,
) -> ApiResult<Json<ListAliasesResponse>> {
    let aliases = state.index_manager.list_aliases();

    Ok(Json(ListAliasesResponse { aliases }))
}

/// Add indices to an existing alias
///
/// # Arguments
///
/// * `State(state)` - Application state
/// * `Path(alias_name)` - Alias name
/// * `Json(request)` - Add indices request
///
/// # Returns
///
/// * `ApiResult<Json<GetAliasResponse>>` - Updated alias information
#[utoipa::path(
    post,
    path = "/_aliases/{alias_name}/indices",
    params(
        ("alias_name" = String, Path, description = "Alias name")
    ),
    request_body = AddIndicesToAliasRequest,
    responses(
        (status = 200, description = "Indices added successfully", body = GetAliasResponse),
        (status = 400, description = "Bad request", body = ApiError),
        (status = 404, description = "Alias or index not found", body = ApiError),
    ),
    tag = "Aliases"
)]
pub async fn add_indices_to_alias(
    State(state): State<crate::handlers::index::AppState>,
    Path(alias_name): Path<String>,
    Json(request): Json<AddIndicesToAliasRequest>,
) -> ApiResult<Json<GetAliasResponse>> {
    // Convert string indices to IndexName
    let indices: Vec<IndexName> = request
        .indices
        .into_iter()
        .map(IndexName::new)
        .collect();

    // Add indices to alias
    let alias = state
        .index_manager
        .add_indices_to_alias(&alias_name, indices)
        .map_err(|e| ApiError::from(e))?;

    Ok(Json(GetAliasResponse { alias }))
}

/// Remove indices from an alias
///
/// # Arguments
///
/// * `State(state)` - Application state
/// * `Path(alias_name)` - Alias name
/// * `Json(request)` - Remove indices request
///
/// # Returns
///
/// * `ApiResult<Json<GetAliasResponse>>` - Updated alias information
#[utoipa::path(
    delete,
    path = "/_aliases/{alias_name}/indices",
    params(
        ("alias_name" = String, Path, description = "Alias name")
    ),
    request_body = RemoveIndicesFromAliasRequest,
    responses(
        (status = 200, description = "Indices removed successfully", body = GetAliasResponse),
        (status = 400, description = "Bad request", body = ApiError),
        (status = 404, description = "Alias not found", body = ApiError),
    ),
    tag = "Aliases"
)]
pub async fn remove_indices_from_alias(
    State(state): State<crate::handlers::index::AppState>,
    Path(alias_name): Path<String>,
    Json(request): Json<RemoveIndicesFromAliasRequest>,
) -> ApiResult<Json<GetAliasResponse>> {
    // Convert string indices to IndexName
    let indices: Vec<IndexName> = request
        .indices
        .into_iter()
        .map(IndexName::new)
        .collect();

    // Remove indices from alias
    let alias = state
        .index_manager
        .remove_indices_from_alias(&alias_name, indices)
        .map_err(|e| ApiError::from(e))?;

    Ok(Json(GetAliasResponse { alias }))
}

/// Execute multiple alias operations atomically
///
/// # Arguments
///
/// * `State(state)` - Application state
/// * `Json(request)` - Alias operations request
///
/// # Returns
///
/// * `ApiResult<Json<AliasOperationsResponse>>` - Operations result
#[utoipa::path(
    post,
    path = "/_aliases",
    request_body = AliasOperationsRequest,
    responses(
        (status = 200, description = "Operations executed successfully", body = AliasOperationsResponse),
        (status = 400, description = "Bad request", body = ApiError),
        (status = 404, description = "Target index not found", body = ApiError),
    ),
    tag = "Aliases"
)]
pub async fn execute_alias_operations(
    State(state): State<crate::handlers::index::AppState>,
    Json(request): Json<AliasOperationsRequest>,
) -> ApiResult<Json<AliasOperationsResponse>> {
    let response = state
        .index_manager
        .execute_alias_operations(request)
        .map_err(|e| ApiError::from(e))?;

    Ok(Json(response))
}

/// Get all aliases that point to a specific index
///
/// # Arguments
///
/// * `State(state)` - Application state
/// * `Path(index_name)` - Index name
///
/// # Returns
///
/// * `ApiResult<Json<ListAliasesResponse>>` - List of aliases for the index
#[utoipa::path(
    get,
    path = "/_aliases/index/{index_name}",
    params(
        ("index_name" = String, Path, description = "Index name")
    ),
    responses(
        (status = 200, description = "List of aliases for the index", body = ListAliasesResponse),
        (status = 404, description = "Index not found", body = ApiError),
    ),
    tag = "Aliases"
)]
pub async fn get_aliases_for_index(
    State(state): State<crate::handlers::index::AppState>,
    Path(index_name): Path<String>,
) -> ApiResult<Json<ListAliasesResponse>> {
    // Check if index exists
    if !state.index_manager.index_exists(&index_name) {
        return Err(ApiError::IndexNotFound(format!("Index '{}' not found", index_name)));
    }

    let aliases = state.index_manager.get_aliases_for_index(&index_name);

    Ok(Json(ListAliasesResponse { aliases }))
}