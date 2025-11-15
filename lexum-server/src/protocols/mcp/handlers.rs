//! MCP Tool handlers for Lexum operations

use crate::handlers::index::AppState;
use lexum_core::{Query, SearchExecutor};
use rmcp::model::{CallToolRequestParam, CallToolResult, Content, ErrorData};
use serde_json::json;
use std::sync::Arc;

/// Handle MCP tool calls
pub async fn handle_mcp_tool(
    request: CallToolRequestParam,
    state: Arc<AppState>,
) -> Result<CallToolResult, ErrorData> {
    match request.name.as_ref() {
        "search" => handle_search(request, state).await,
        "retrieve" => handle_retrieve(request, state).await,
        "aggregate" => handle_aggregate(request, state).await,
        "list_indices" => handle_list_indices(request, state).await,
        "create_index" => handle_create_index(request, state).await,
        "get_mapping" => handle_get_mapping(request, state).await,
        "update_mapping" => handle_update_mapping(request, state).await,
        _ => Err(ErrorData::invalid_params("Unknown tool", None)),
    }
}

/// Handle search operation
async fn handle_search(
    request: CallToolRequestParam,
    state: Arc<AppState>,
) -> Result<CallToolResult, ErrorData> {
    let args = request
        .arguments
        .as_ref()
        .ok_or_else(|| ErrorData::invalid_params("Missing arguments", None))?;

    let index_name = args
        .get("index")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorData::invalid_params("Missing index", None))?;

    // Resolve alias
    let target_indices = state
        .index_manager
        .resolve_name(index_name)
        .map_err(|e| ErrorData::internal_error(format!("Index not found: {e}"), None))?;

    if target_indices.is_empty() {
        return Err(ErrorData::internal_error(
            format!("Index not found: {index_name}"),
            None,
        ));
    }

    let index = state
        .index_manager
        .get_index(target_indices[0].as_str())
        .map_err(|e| ErrorData::internal_error(format!("Failed to get index: {e}"), None))?;

    // Build query
    let query = if let Some(q) = args.get("q").and_then(|v| v.as_str()) {
        let text_fields = index.get_text_field_names();
        if text_fields.is_empty() {
            Query::MatchAll
        } else if text_fields.len() == 1 {
            Query::Match(lexum_core::MatchQuery::new(&text_fields[0], q.to_string()))
        } else {
            let mut bool_query = lexum_core::BoolQuery::new();
            for field in text_fields {
                bool_query = bool_query.should(Query::Match(lexum_core::MatchQuery::new(
                    &field,
                    q.to_string(),
                )));
            }
            Query::Bool(bool_query)
        }
    } else if let Some(query_obj) = args.get("query") {
        serde_json::from_value(query_obj.clone())
            .map_err(|e| ErrorData::invalid_params(format!("Invalid query: {e}"), None))?
    } else {
        Query::MatchAll
    };

    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    // Execute search
    let executor = SearchExecutor::new(Arc::new(index));
    let result = executor
        .search(query, limit, offset, None)
        .await
        .map_err(|e| ErrorData::internal_error(format!("Search failed: {e}"), None))?;

    let response = json!({
        "hits": result.hits,
        "total": result.total,
        "took": result.took_ms
    });

    Ok(CallToolResult::success(vec![Content::text(
        response.to_string(),
    )]))
}

/// Handle retrieve operation
async fn handle_retrieve(
    request: CallToolRequestParam,
    state: Arc<AppState>,
) -> Result<CallToolResult, ErrorData> {
    let args = request
        .arguments
        .as_ref()
        .ok_or_else(|| ErrorData::invalid_params("Missing arguments", None))?;

    let index_name = args
        .get("index")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorData::invalid_params("Missing index", None))?;

    let doc_id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorData::invalid_params("Missing id", None))?;

    // Resolve alias
    let target_indices = state
        .index_manager
        .resolve_name(index_name)
        .map_err(|e| ErrorData::internal_error(format!("Index not found: {e}"), None))?;

    if target_indices.is_empty() {
        return Err(ErrorData::internal_error(
            format!("Index not found: {index_name}"),
            None,
        ));
    }

    let index = state
        .index_manager
        .get_index(target_indices[0].as_str())
        .map_err(|e| ErrorData::internal_error(format!("Failed to get index: {e}"), None))?;

    // Retrieve document
    let store = lexum_core::DocumentStore::new(Arc::new(index));
    let doc = store
        .get_document(&lexum_core::types::DocumentId::new(doc_id.to_string()))
        .await
        .map_err(|e| {
            ErrorData::internal_error(format!("Failed to retrieve document: {e}"), None)
        })?;

    let response = json!({
        "id": doc_id,
        "source": doc
    });

    Ok(CallToolResult::success(vec![Content::text(
        response.to_string(),
    )]))
}

/// Handle aggregate operation
async fn handle_aggregate(
    request: CallToolRequestParam,
    state: Arc<AppState>,
) -> Result<CallToolResult, ErrorData> {
    let args = request
        .arguments
        .as_ref()
        .ok_or_else(|| ErrorData::invalid_params("Missing arguments", None))?;

    let index_name = args
        .get("index")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorData::invalid_params("Missing index", None))?;

    // Resolve alias
    let target_indices = state
        .index_manager
        .resolve_name(index_name)
        .map_err(|e| ErrorData::internal_error(format!("Index not found: {e}"), None))?;

    if target_indices.is_empty() {
        return Err(ErrorData::internal_error(
            format!("Index not found: {index_name}"),
            None,
        ));
    }

    let index = state
        .index_manager
        .get_index(target_indices[0].as_str())
        .map_err(|e| ErrorData::internal_error(format!("Failed to get index: {e}"), None))?;

    // Build query
    let query = if let Some(query_obj) = args.get("query") {
        serde_json::from_value(query_obj.clone())
            .map_err(|e| ErrorData::invalid_params(format!("Invalid query: {e}"), None))?
    } else {
        Query::MatchAll
    };

    // Parse aggregations
    let aggregations = if let Some(aggs_obj) = args.get("aggregations").and_then(|v| v.as_object())
    {
        let mut agg_specs = Vec::new();
        for (name, spec) in aggs_obj {
            // Try to deserialize as AggregationSpec
            match serde_json::from_value::<lexum_core::aggregation::AggregationSpec>(spec.clone()) {
                Ok(agg_spec) => agg_specs.push(agg_spec),
                Err(e) => {
                    return Err(ErrorData::invalid_params(
                        format!("Invalid aggregation '{name}': {e}"),
                        None,
                    ));
                }
            }
        }
        Some(agg_specs)
    } else {
        None
    };

    // Execute search with aggregations
    let executor = SearchExecutor::new(Arc::new(index));
    let result = executor
        .search_with_aggregations(query, 0, 0, None, aggregations.as_deref())
        .await
        .map_err(|e| ErrorData::internal_error(format!("Aggregation failed: {e}"), None))?;

    let response = json!({
        "aggregations": result.aggregations,
        "total": result.total
    });

    Ok(CallToolResult::success(vec![Content::text(
        response.to_string(),
    )]))
}

/// Handle list indices operation
async fn handle_list_indices(
    _request: CallToolRequestParam,
    state: Arc<AppState>,
) -> Result<CallToolResult, ErrorData> {
    let indices = state.index_manager.list_indices();

    let response = json!({
        "indices": indices.iter().map(|name| {
            json!({
                "name": name
            })
        }).collect::<Vec<_>>(),
        "total": indices.len()
    });

    Ok(CallToolResult::success(vec![Content::text(
        response.to_string(),
    )]))
}

/// Handle create index operation
async fn handle_create_index(
    request: CallToolRequestParam,
    state: Arc<AppState>,
) -> Result<CallToolResult, ErrorData> {
    use crate::handlers::index::FieldDefinition;
    use lexum_core::schema::{ElasticsearchMapping, mapping_to_schema};
    use lexum_core::{FieldConfig, FieldType, SchemaBuilder};

    let args = request
        .arguments
        .as_ref()
        .ok_or_else(|| ErrorData::invalid_params("Missing arguments", None))?;

    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorData::invalid_params("Missing name", None))?;

    // Validate name
    if name.is_empty() {
        return Err(ErrorData::invalid_params(
            "Index name cannot be empty",
            None,
        ));
    }

    // Check if index already exists
    if state.index_manager.get_index(name).is_ok() {
        return Err(ErrorData::invalid_params(
            format!("Index '{name}' already exists"),
            None,
        ));
    }

    // Parse mappings if provided
    let mappings = args.get("mappings").cloned();

    // Parse settings if provided
    let settings = if let Some(settings_obj) = args.get("settings") {
        serde_json::from_value(settings_obj.clone())
            .map_err(|e| ErrorData::invalid_params(format!("Invalid settings: {e}"), None))?
    } else {
        lexum_core::index::settings::IndexSettings::default()
    };

    // Parse fields if provided (alternative to mappings)
    let fields = if let Some(fields_arr) = args.get("fields").and_then(|v| v.as_array()) {
        fields_arr
            .iter()
            .filter_map(|f| serde_json::from_value::<FieldDefinition>(f.clone()).ok())
            .collect()
    } else {
        Vec::new()
    };

    // Find matching templates
    let matching_templates = state.template_manager.find_matching_templates(name);

    // Merge settings with templates
    let mut final_settings = settings;
    for template in matching_templates.iter().rev() {
        let template_settings = template.settings.to_index_settings();
        final_settings.merge(template_settings);
    }

    // Build schema from mappings or fields
    let (schema, final_mapping) = if let Some(ref mappings_json) = mappings {
        // Use Elasticsearch mappings if provided
        let request_mapping: ElasticsearchMapping = serde_json::from_value(mappings_json.clone())
            .map_err(|e| {
            ErrorData::invalid_params(format!("Invalid mapping format: {e}"), None)
        })?;

        // Validate request mapping
        request_mapping
            .validate()
            .map_err(|e| ErrorData::invalid_params(format!("Invalid mapping: {e}"), None))?;

        // Merge with template mappings
        let mut template_mappings = Vec::new();
        for template in matching_templates.iter().rev() {
            if let Ok(template_mapping) = template.mappings.to_elasticsearch_mapping() {
                template_mappings.push(template_mapping);
            }
        }

        let mut final_mapping = if !template_mappings.is_empty() {
            ElasticsearchMapping::merge_all(template_mappings)
        } else {
            ElasticsearchMapping::new()
        };

        final_mapping.merge(request_mapping);
        final_mapping
            .validate()
            .map_err(|e| ErrorData::invalid_params(format!("Invalid merged mapping: {e}"), None))?;

        let schema = mapping_to_schema(&final_mapping).map_err(|e| {
            ErrorData::internal_error(format!("Failed to convert mapping to schema: {e}"), None)
        })?;

        (schema, Some(final_mapping))
    } else if !fields.is_empty() {
        // Use fields if provided
        let mut builder = SchemaBuilder::new();
        for field in &fields {
            let field_type = match field.field_type.as_str() {
                "text" => FieldType::Text,
                "keyword" => FieldType::Keyword,
                "i64" => FieldType::I64,
                "f64" => FieldType::F64,
                "date" => FieldType::Date,
                "boolean" => FieldType::Boolean,
                _ => {
                    return Err(ErrorData::invalid_params(
                        format!("Unknown field type: {}", field.field_type),
                        None,
                    ));
                }
            };

            let mut field_config = FieldConfig::new(&field.name, field_type);
            if field.stored {
                field_config = field_config.stored(true);
            }
            if field.indexed {
                field_config = field_config.indexed(true);
            }
            if field.fast {
                field_config = field_config.fast(true);
            }

            builder = builder.add_field(field_config);
        }

        let (schema, _) = builder
            .build()
            .map_err(|e| ErrorData::internal_error(format!("Failed to build schema: {e}"), None))?;
        (schema, None)
    } else {
        return Err(ErrorData::invalid_params(
            "Either 'mappings' or 'fields' must be provided".to_string(),
            None,
        ));
    };

    // Create the index
    let result = if let Some(mapping) = final_mapping {
        state
            .index_manager
            .create_index_with_mapping(name, schema, final_settings, Some(mapping))
            .await
    } else {
        state
            .index_manager
            .create_index(name, schema, final_settings)
            .await
    };

    match result {
        Ok(_index) => {
            // Get stats to get num_docs
            let stats = state
                .index_manager
                .get_index_stats(name)
                .await
                .map_err(|e| {
                    ErrorData::internal_error(format!("Failed to get index stats: {e}"), None)
                })?;

            let response = json!({
                "status": "created",
                "index": name,
                "num_docs": stats.num_docs
            });

            Ok(CallToolResult::success(vec![Content::text(
                response.to_string(),
            )]))
        }
        Err(e) => {
            let error_msg = format!("Failed to create index: {e}");
            Err(ErrorData::internal_error(error_msg, None))
        }
    }
}

/// Handle get mapping operation
async fn handle_get_mapping(
    request: CallToolRequestParam,
    state: Arc<AppState>,
) -> Result<CallToolResult, ErrorData> {
    use lexum_core::schema::schema_to_mapping;

    let args = request
        .arguments
        .as_ref()
        .ok_or_else(|| ErrorData::invalid_params("Missing arguments", None))?;

    let index_name = args
        .get("index")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorData::invalid_params("Missing index", None))?;

    // Resolve index alias if needed
    let resolved_index = state
        .index_manager
        .resolve_alias(index_name)
        .ok()
        .and_then(|indices| indices.first().map(|idx| idx.to_string()))
        .unwrap_or_else(|| index_name.to_string());

    // Get the index
    let index = state
        .index_manager
        .get_index(&resolved_index)
        .map_err(|_| ErrorData::invalid_params(format!("Index '{index_name}' not found"), None))?;

    // Convert schema to Elasticsearch mapping
    let schema = index.schema();
    let mapping = schema_to_mapping(&schema).map_err(|e| {
        ErrorData::internal_error(format!("Failed to convert schema to mapping: {e}"), None)
    })?;

    // Serialize to JSON
    let mappings_json = serde_json::to_value(&mapping).map_err(|e| {
        ErrorData::internal_error(format!("Failed to serialize mapping: {e}"), None)
    })?;

    let response = json!({
        "index": index_name,
        "mappings": mappings_json
    });

    Ok(CallToolResult::success(vec![Content::text(
        response.to_string(),
    )]))
}

/// Handle update mapping operation
async fn handle_update_mapping(
    request: CallToolRequestParam,
    _state: Arc<AppState>,
) -> Result<CallToolResult, ErrorData> {
    use lexum_core::schema::ElasticsearchMapping;

    let args = request
        .arguments
        .as_ref()
        .ok_or_else(|| ErrorData::invalid_params("Missing arguments", None))?;

    let _index_name = args
        .get("index")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorData::invalid_params("Missing index", None))?;

    let mappings = args
        .get("mappings")
        .ok_or_else(|| ErrorData::invalid_params("Missing mappings", None))?;

    // Parse and validate mapping
    let _mapping: ElasticsearchMapping = serde_json::from_value(mappings.clone())
        .map_err(|e| ErrorData::invalid_params(format!("Invalid mapping format: {e}"), None))?;

    // Mapping updates are not yet supported
    Err(ErrorData::internal_error(
        "Mapping updates are not yet supported. Index schemas cannot be modified after creation."
            .to_string(),
        None,
    ))
}
