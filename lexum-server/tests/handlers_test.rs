//! Handler tests for lexum-server

use lexum_core::*;
use lexum_server::handlers::document::*;
use lexum_server::handlers::search::*;
use serde_json::json;

// ============================================================================
// Document Handler Tests
// ============================================================================

#[test]
fn test_add_document_request() {
    let request = AddDocumentRequest {
        document: json!({
            "title": "Test Document",
            "content": "This is a test"
        }),
    };

    assert!(request.document.is_object());
    assert_eq!(request.document["title"], "Test Document");
}

#[test]
fn test_add_document_response() {
    let response = AddDocumentResponse {
        id: "doc123".to_string(),
    };

    assert_eq!(response.id, "doc123");
}

#[test]
fn test_bulk_operation_index() {
    let op = BulkOperation::Index {
        index: "test".to_string(),
        id: Some("1".to_string()),
        document: json!({"title": "Test"}),
    };

    match op {
        BulkOperation::Index {
            index,
            id,
            document,
        } => {
            assert_eq!(index, "test");
            assert_eq!(id, Some("1".to_string()));
            assert!(document.is_object());
        }
        _ => panic!("Expected Index operation"),
    }
}

#[test]
fn test_bulk_operation_create() {
    let op = BulkOperation::Create {
        index: "test".to_string(),
        id: "2".to_string(),
        document: json!({"title": "New Doc"}),
    };

    match op {
        BulkOperation::Create {
            index,
            id,
            document,
        } => {
            assert_eq!(index, "test");
            assert_eq!(id, "2");
            assert!(document.is_object());
        }
        _ => panic!("Expected Create operation"),
    }
}

#[test]
fn test_bulk_operation_update() {
    let op = BulkOperation::Update {
        index: "test".to_string(),
        id: "3".to_string(),
        document: json!({"title": "Updated"}),
    };

    match op {
        BulkOperation::Update {
            index,
            id,
            document,
        } => {
            assert_eq!(index, "test");
            assert_eq!(id, "3");
            assert_eq!(document["title"], "Updated");
        }
        _ => panic!("Expected Update operation"),
    }
}

#[test]
fn test_bulk_operation_delete() {
    let op = BulkOperation::Delete {
        index: "test".to_string(),
        id: "4".to_string(),
    };

    match op {
        BulkOperation::Delete { index, id } => {
            assert_eq!(index, "test");
            assert_eq!(id, "4");
        }
        _ => panic!("Expected Delete operation"),
    }
}

#[test]
fn test_bulk_operation_result_success() {
    let result = BulkOperationResult {
        success: true,
        action: "index".to_string(),
        index: "test".to_string(),
        id: Some("1".to_string()),
        error: None,
    };

    assert!(result.success);
    assert_eq!(result.action, "index");
    assert!(result.error.is_none());
}

#[test]
fn test_bulk_operation_result_failure() {
    let result = BulkOperationResult {
        success: false,
        action: "index".to_string(),
        index: "test".to_string(),
        id: Some("1".to_string()),
        error: Some("Index not found".to_string()),
    };

    assert!(!result.success);
    assert!(result.error.is_some());
    assert_eq!(result.error.unwrap(), "Index not found");
}

#[test]
fn test_bulk_request_multiple_operations() {
    let request = BulkRequest {
        operations: vec![
            BulkOperation::Index {
                index: "test".to_string(),
                id: Some("1".to_string()),
                document: json!({"title": "Doc 1"}),
            },
            BulkOperation::Create {
                index: "test".to_string(),
                id: "2".to_string(),
                document: json!({"title": "Doc 2"}),
            },
            BulkOperation::Delete {
                index: "test".to_string(),
                id: "3".to_string(),
            },
        ],
    };

    assert_eq!(request.operations.len(), 3);
}

#[test]
fn test_bulk_response_structure() {
    let response = BulkResponse {
        errors: false,
        took_ms: 100,
        items: vec![BulkOperationResult {
            success: true,
            action: "index".to_string(),
            index: "test".to_string(),
            id: Some("1".to_string()),
            error: None,
        }],
    };

    assert!(!response.errors);
    assert_eq!(response.took_ms, 100);
    assert_eq!(response.items.len(), 1);
}

// ============================================================================
// Search Handler Tests
// ============================================================================

#[test]
fn test_search_request_defaults() {
    let request = SearchRequest {
        query: QueryBuilder::match_all(),
        limit: 10,
        offset: 0,
        sort: None,
    };

    assert_eq!(request.limit, 10);
    assert_eq!(request.offset, 0);
    assert!(request.sort.is_none());
}

#[test]
fn test_search_request_with_sort() {
    let request = SearchRequest {
        query: QueryBuilder::match_query("title", "test"),
        limit: 20,
        offset: 5,
        sort: Some(SortOption::asc("date")),
    };

    assert_eq!(request.limit, 20);
    assert_eq!(request.offset, 5);
    assert!(request.sort.is_some());

    let sort = request.sort.unwrap();
    assert_eq!(sort.field, "date");
    assert_eq!(sort.order, SortOrder::Asc);
}

#[test]
fn test_search_request_with_fuzzy_query() {
    let request = SearchRequest {
        query: QueryBuilder::fuzzy_query("name", "jhon"),
        limit: 10,
        offset: 0,
        sort: None,
    };

    assert!(matches!(request.query, Query::Fuzzy(_)));
}

#[test]
fn test_search_request_with_phrase_query() {
    let request = SearchRequest {
        query: QueryBuilder::phrase_query("content", "quick brown fox"),
        limit: 10,
        offset: 0,
        sort: None,
    };

    assert!(matches!(request.query, Query::Phrase(_)));
}

#[test]
fn test_search_request_with_bool_query() {
    let bool_query = BoolQuery::new()
        .must(Query::Match(MatchQuery::new("title", "rust")))
        .should(Query::Term(TermQuery::new("category", "tutorial")));

    let request = SearchRequest {
        query: Query::Bool(bool_query),
        limit: 50,
        offset: 0,
        sort: Some(SortOption::desc("_score")),
    };

    assert_eq!(request.limit, 50);
    assert!(matches!(request.query, Query::Bool(_)));
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_bulk_operation_serialization() {
    let op = BulkOperation::Index {
        index: "test".to_string(),
        id: Some("1".to_string()),
        document: json!({"title": "Test"}),
    };

    let json = serde_json::to_string(&op).unwrap();
    assert!(json.contains("index"));
    assert!(json.contains("_index"));

    let deserialized: BulkOperation = serde_json::from_str(&json).unwrap();
    match deserialized {
        BulkOperation::Index { index, id, .. } => {
            assert_eq!(index, "test");
            assert_eq!(id, Some("1".to_string()));
        }
        _ => panic!("Expected Index operation"),
    }
}

#[test]
fn test_search_request_serialization() {
    let request = SearchRequest {
        query: QueryBuilder::match_query("title", "test"),
        limit: 15,
        offset: 5,
        sort: Some(SortOption::desc("timestamp")),
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("query"));
    assert!(json.contains("limit"));
    assert!(json.contains("offset"));

    let deserialized: SearchRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.limit, 15);
    assert_eq!(deserialized.offset, 5);
}
