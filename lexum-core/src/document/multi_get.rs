//! Multi-Get (mget) - Batch document retrieval

use crate::error::Result;
use crate::index::Index;
use crate::types::DocumentId;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use utoipa::ToSchema;

/// Multi-Get request item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MultiGetItem {
    /// Index name
    #[serde(rename = "_index", skip_serializing_if = "Option::is_none")]
    pub index: Option<String>,
    /// Document ID
    #[serde(rename = "_id")]
    pub id: String,
    /// Stored fields to retrieve (if None, retrieves _source)
    #[serde(rename = "_source", skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceFilter>,
    /// Stored fields to retrieve
    #[serde(rename = "stored_fields", skip_serializing_if = "Option::is_none")]
    pub stored_fields: Option<Vec<String>>,
    /// Routing value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing: Option<String>,
}

/// Source filter
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum SourceFilter {
    /// Include all fields
    IncludeAll(bool),
    /// Include specific fields
    Include(Vec<String>),
    /// Exclude specific fields
    Exclude(Vec<String>),
    /// Include/exclude object
    Object {
        /// Fields to include
        #[serde(skip_serializing_if = "Option::is_none")]
        includes: Option<Vec<String>>,
        /// Fields to exclude
        #[serde(skip_serializing_if = "Option::is_none")]
        excludes: Option<Vec<String>>,
    },
}

/// Multi-Get request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MultiGetRequest {
    /// List of documents to retrieve
    pub docs: Vec<MultiGetItem>,
}

/// Multi-Get response item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MultiGetResponseItem {
    /// Index name
    #[serde(rename = "_index")]
    pub index: String,
    /// Document ID
    #[serde(rename = "_id")]
    pub id: String,
    /// Document version
    #[serde(rename = "_version", skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
    /// Whether document was found
    pub found: bool,
    /// Document source (if found)
    #[serde(rename = "_source", skip_serializing_if = "Option::is_none")]
    pub source: Option<JsonValue>,
    /// Stored fields (if requested)
    #[serde(rename = "fields", skip_serializing_if = "Option::is_none")]
    pub fields: Option<JsonValue>,
    /// Error (if not found or error occurred)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MultiGetError>,
}

/// Multi-Get error
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MultiGetError {
    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error reason
    pub reason: String,
}

/// Multi-Get response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MultiGetResponse {
    /// Response items
    pub docs: Vec<MultiGetResponseItem>,
}

/// Multi-Get operations
pub struct MultiGet {
    index: Arc<Index>,
    store: Arc<crate::document::store::DocumentStore>,
}

impl MultiGet {
    /// Create new Multi-Get handler
    pub fn new(index: Arc<Index>) -> Self {
        let store = Arc::new(crate::document::store::DocumentStore::new(index.clone()));
        Self { index, store }
    }

    /// Retrieve multiple documents
    pub async fn get(&self, request: MultiGetRequest) -> Result<MultiGetResponse> {
        let mut response_items = Vec::new();

        for item in request.docs {
            let doc_id = DocumentId::new(item.id.clone());
            let index_name = item.index.as_deref().unwrap_or(self.index.name().as_str());

            // Get document
            match self.store.get_document(&doc_id).await {
                Ok(mut doc) => {
                    // Apply source filtering if specified
                    if let Some(ref source_filter) = item.source {
                        doc = Self::apply_source_filter(doc, source_filter);
                    }

                    // Apply stored fields filtering if specified
                    let fields = item
                        .stored_fields
                        .as_ref()
                        .map(|stored_fields| Self::extract_stored_fields(&doc, stored_fields));

                    response_items.push(MultiGetResponseItem {
                        index: index_name.to_string(),
                        id: item.id,
                        version: Some(1), // Note: Version tracking not yet implemented
                        found: true,
                        source: Some(doc),
                        fields,
                        error: None,
                    });
                }
                Err(e) => {
                    let error_reason = if e.to_string().contains("not found") {
                        "Document not found".to_string()
                    } else {
                        e.to_string()
                    };
                    let error_type = if e.to_string().contains("not found") {
                        "not_found_exception"
                    } else {
                        "internal_exception"
                    };
                    response_items.push(MultiGetResponseItem {
                        index: index_name.to_string(),
                        id: item.id,
                        version: None,
                        found: false,
                        source: None,
                        fields: None,
                        error: Some(MultiGetError {
                            error_type: error_type.to_string(),
                            reason: error_reason,
                        }),
                    });
                }
            }
        }

        Ok(MultiGetResponse {
            docs: response_items,
        })
    }

    /// Apply source filter to document
    fn apply_source_filter(doc: JsonValue, filter: &SourceFilter) -> JsonValue {
        match filter {
            SourceFilter::IncludeAll(true) => doc,
            SourceFilter::IncludeAll(false) => JsonValue::Object(serde_json::Map::new()),
            SourceFilter::Include(fields) => Self::include_fields(&doc, fields),
            SourceFilter::Exclude(fields) => Self::exclude_fields(&doc, fields),
            SourceFilter::Object { includes, excludes } => {
                let mut result = doc;
                if let Some(includes) = includes {
                    result = Self::include_fields(&result, includes);
                }
                if let Some(excludes) = excludes {
                    result = Self::exclude_fields(&result, excludes);
                }
                result
            }
        }
    }

    /// Include only specified fields
    fn include_fields(doc: &JsonValue, fields: &[String]) -> JsonValue {
        if let JsonValue::Object(map) = doc {
            let mut result = serde_json::Map::new();
            for field in fields {
                if let Some(value) = map.get(field) {
                    result.insert(field.clone(), value.clone());
                }
            }
            JsonValue::Object(result)
        } else {
            doc.clone()
        }
    }

    /// Exclude specified fields
    fn exclude_fields(doc: &JsonValue, fields: &[String]) -> JsonValue {
        if let JsonValue::Object(map) = doc {
            let mut result = map.clone();
            for field in fields {
                result.remove(field);
            }
            JsonValue::Object(result)
        } else {
            doc.clone()
        }
    }

    /// Extract stored fields from document
    fn extract_stored_fields(doc: &JsonValue, stored_fields: &[String]) -> JsonValue {
        if let JsonValue::Object(map) = doc {
            let mut result = serde_json::Map::new();
            for field in stored_fields {
                if let Some(value) = map.get(field) {
                    result.insert(field.clone(), value.clone());
                }
            }
            JsonValue::Object(result)
        } else {
            JsonValue::Object(serde_json::Map::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_include_fields() {
        let doc = serde_json::json!({
            "field1": "value1",
            "field2": "value2",
            "field3": "value3"
        });

        let result = MultiGet::include_fields(&doc, &["field1".to_string(), "field3".to_string()]);
        assert_eq!(result["field1"], "value1");
        assert_eq!(result["field3"], "value3");
        assert!(!result.as_object().unwrap().contains_key("field2"));
    }

    #[test]
    fn test_exclude_fields() {
        let doc = serde_json::json!({
            "field1": "value1",
            "field2": "value2",
            "field3": "value3"
        });

        let result = MultiGet::exclude_fields(&doc, &["field2".to_string()]);
        assert_eq!(result["field1"], "value1");
        assert_eq!(result["field3"], "value3");
        assert!(!result.as_object().unwrap().contains_key("field2"));
    }

    #[test]
    fn test_apply_source_filter_include() {
        let doc = serde_json::json!({
            "field1": "value1",
            "field2": "value2"
        });

        let filter = SourceFilter::Include(vec!["field1".to_string()]);
        let result = MultiGet::apply_source_filter(doc, &filter);
        assert_eq!(result["field1"], "value1");
        assert!(!result.as_object().unwrap().contains_key("field2"));
    }

    #[test]
    fn test_apply_source_filter_exclude() {
        let doc = serde_json::json!({
            "field1": "value1",
            "field2": "value2"
        });

        let filter = SourceFilter::Exclude(vec!["field2".to_string()]);
        let result = MultiGet::apply_source_filter(doc, &filter);
        assert_eq!(result["field1"], "value1");
        assert!(!result.as_object().unwrap().contains_key("field2"));
    }
}
