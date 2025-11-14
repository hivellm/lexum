//! Handler tests for lexum-server

use lexum_core::*;
use lexum_server::handlers::document::BulkOperation as ServerBulkOperation;
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
    let op = ServerBulkOperation::Index {
        index: "test".to_string(),
        id: Some("1".to_string()),
        document: json!({"title": "Test"}),
    };

    match op {
        ServerBulkOperation::Index {
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
    let op = ServerBulkOperation::Create {
        index: "test".to_string(),
        id: "2".to_string(),
        document: json!({"title": "New Doc"}),
    };

    match op {
        ServerBulkOperation::Create {
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
    let op = ServerBulkOperation::Update {
        index: "test".to_string(),
        id: "3".to_string(),
        document: json!({"title": "Updated"}),
    };

    match op {
        ServerBulkOperation::Update {
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
    let op = ServerBulkOperation::Delete {
        index: "test".to_string(),
        id: "4".to_string(),
    };

    match op {
        ServerBulkOperation::Delete { index, id } => {
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
            ServerBulkOperation::Index {
                index: "test".to_string(),
                id: Some("1".to_string()),
                document: json!({"title": "Doc 1"}),
            },
            ServerBulkOperation::Create {
                index: "test".to_string(),
                id: "2".to_string(),
                document: json!({"title": "Doc 2"}),
            },
            ServerBulkOperation::Delete {
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
        filter: None,
        query: QueryBuilder::match_all(),
        limit: 10,
        offset: 0,
        sort: None,
        fields: None,
        highlight: None,
        explain: false,
        min_score: None,
        q: None,
    };

    assert_eq!(request.limit, 10);
    assert_eq!(request.offset, 0);
    assert!(request.sort.is_none());
}

#[test]
fn test_search_request_with_sort() {
    let request = SearchRequest {
        filter: None,
        query: QueryBuilder::match_query("title", "test"),
        limit: 20,
        offset: 5,
        sort: Some(SortOption::asc("date")),
        fields: None,
        highlight: None,
        explain: false,
        min_score: None,
        q: None,
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
        filter: None,
        query: QueryBuilder::fuzzy_query("name", "jhon"),
        limit: 10,
        offset: 0,
        sort: None,
        fields: None,
        highlight: None,
        explain: false,
        min_score: None,
        q: None,
    };

    assert!(matches!(request.query, Query::Fuzzy(_)));
}

#[test]
fn test_search_request_with_phrase_query() {
    let request = SearchRequest {
        filter: None,
        query: QueryBuilder::phrase_query("content", "quick brown fox"),
        limit: 10,
        offset: 0,
        sort: None,
        fields: None,
        highlight: None,
        explain: false,
        min_score: None,
        q: None,
    };

    assert!(matches!(request.query, Query::Phrase(_)));
}

#[test]
fn test_search_request_with_bool_query() {
    let bool_query = BoolQuery::new()
        .must(Query::Match(MatchQuery::new("title", "rust")))
        .should(Query::Term(TermQuery::new("category", "tutorial")));

    let request = SearchRequest {
        filter: None,
        query: Query::Bool(bool_query),
        limit: 50,
        offset: 0,
        sort: Some(SortOption::desc("_score")),
        fields: None,
        highlight: None,
        explain: false,
        min_score: None,
        q: None,
    };

    assert_eq!(request.limit, 50);
    assert!(matches!(request.query, Query::Bool(_)));
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_bulk_operation_serialization() {
    let op = ServerBulkOperation::Index {
        index: "test".to_string(),
        id: Some("1".to_string()),
        document: json!({"title": "Test"}),
    };

    let json = serde_json::to_string(&op).unwrap();
    assert!(json.contains("index"));
    assert!(json.contains("_index"));

    let deserialized: BulkOperation = serde_json::from_str(&json).unwrap();
    match deserialized {
        ServerBulkOperation::Index { index, id, .. } => {
            assert_eq!(index, "test");
            assert_eq!(id, Some("1".to_string()));
        }
        _ => panic!("Expected Index operation"),
    }
}

#[test]
fn test_search_request_serialization() {
    let request = SearchRequest {
        filter: None,
        query: QueryBuilder::match_query("title", "test"),
        limit: 15,
        offset: 5,
        sort: Some(SortOption::desc("timestamp")),
        fields: None,
        highlight: None,
        explain: false,
        min_score: None,
        q: None,
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("query"));
    assert!(json.contains("limit"));
    assert!(json.contains("offset"));

    let deserialized: SearchRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.limit, 15);
    assert_eq!(deserialized.offset, 5);
}

// ============================================================================
// Rollover Handler Tests
// ============================================================================

#[test]
fn test_rollover_conditions() {
    use lexum_server::handlers::rollover::RolloverConditions;

    let conditions = RolloverConditions {
        max_age: Some("7d".to_string()),
        max_size: Some("5gb".to_string()),
        max_docs: Some(1000000),
        max_primary_shard_size: Some("1gb".to_string()),
    };

    assert_eq!(conditions.max_age, Some("7d".to_string()));
    assert_eq!(conditions.max_size, Some("5gb".to_string()));
    assert_eq!(conditions.max_docs, Some(1000000));
    assert_eq!(conditions.max_primary_shard_size, Some("1gb".to_string()));
}

#[test]
fn test_rollover_request() {
    use lexum_server::handlers::rollover::{RolloverConditions, RolloverRequest};

    let conditions = RolloverConditions {
        max_docs: Some(1000),
        ..Default::default()
    };

    let request = RolloverRequest {
        conditions,
        new_index: Some("new-index".to_string()),
        dry_run: true,
    };

    assert_eq!(request.conditions.max_docs, Some(1000));
    assert_eq!(request.new_index, Some("new-index".to_string()));
    assert!(request.dry_run);
}

#[test]
fn test_rollover_response() {
    use lexum_server::handlers::rollover::{IndexStats, RolloverResponse};

    let stats = IndexStats {
        num_docs: 1000,
        size_in_bytes: 1024000,
        age_in_millis: 86400000, // 1 day
        num_primary_shards: 1,
    };

    let response = RolloverResponse {
        acknowledged: true,
        conditions_met: true,
        old_index: "old-index".to_string(),
        new_index: "new-index".to_string(),
        dry_run: false,
        rolled_over_due_to: Some("max_docs:1000".to_string()),
        index_stats: stats,
    };

    assert!(response.acknowledged);
    assert!(response.conditions_met);
    assert_eq!(response.old_index, "old-index");
    assert_eq!(response.new_index, "new-index");
    assert!(!response.dry_run);
    assert_eq!(
        response.rolled_over_due_to,
        Some("max_docs:1000".to_string())
    );
    assert_eq!(response.index_stats.num_docs, 1000);
}

#[test]
fn test_rollover_conditions_parsing() {
    use lexum_server::handlers::rollover::{
        IndexStats, RolloverConditions, check_rollover_conditions,
    };

    let conditions = RolloverConditions {
        max_docs: Some(1000),
        ..Default::default()
    };

    let stats = IndexStats {
        num_docs: 1500, // Exceeds max_docs
        size_in_bytes: 1024000,
        age_in_millis: 86400000,
        num_primary_shards: 1,
    };

    let (conditions_met, reason) = check_rollover_conditions(&conditions, &stats);
    assert!(conditions_met);
    assert_eq!(reason, Some("max_docs:1000".to_string()));
}

#[test]
fn test_rollover_conditions_not_met() {
    use lexum_server::handlers::rollover::{
        IndexStats, RolloverConditions, check_rollover_conditions,
    };

    let conditions = RolloverConditions {
        max_docs: Some(10000),
        ..Default::default()
    };

    let stats = IndexStats {
        num_docs: 1000, // Below max_docs
        size_in_bytes: 1024000,
        age_in_millis: 86400000,
        num_primary_shards: 1,
    };

    let (conditions_met, reason) = check_rollover_conditions(&conditions, &stats);
    assert!(!conditions_met);
    assert_eq!(reason, None);
}

#[test]
fn test_rollover_index_name_generation() {
    use lexum_server::handlers::rollover::generate_rollover_index_name;

    // Test with existing number suffix
    let result = generate_rollover_index_name("logs-2023-01-01");
    assert_eq!(result, "logs-2023-01-000002");

    // Test with different number
    let result = generate_rollover_index_name("logs-000001");
    assert_eq!(result, "logs-000002");

    // Test without number suffix
    let result = generate_rollover_index_name("logs");
    assert_eq!(result, "logs-000001");
}
