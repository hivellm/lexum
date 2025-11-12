//! Rollover handler tests

use lexum_server::handlers::rollover::{
    IndexStats, RolloverConditions, RolloverRequest, RolloverResponse, check_rollover_conditions,
    generate_rollover_index_name,
};

#[tokio::test]
async fn test_rollover_conditions_max_docs() {
    let conditions = RolloverConditions {
        max_docs: Some(100),
        ..Default::default()
    };

    let stats = IndexStats {
        num_docs: 150,
        size_in_bytes: 0,
        age_in_millis: 0,
        num_primary_shards: 1,
    };

    let (met, reason) = check_rollover_conditions(&conditions, &stats);
    assert!(met);
    assert_eq!(reason, Some("max_docs:100".to_string()));
}

#[tokio::test]
async fn test_rollover_conditions_max_size() {
    let conditions = RolloverConditions {
        max_size: Some("1mb".to_string()),
        ..Default::default()
    };

    let stats = IndexStats {
        num_docs: 0,
        size_in_bytes: 2 * 1024 * 1024, // 2MB
        age_in_millis: 0,
        num_primary_shards: 1,
    };

    let (met, reason) = check_rollover_conditions(&conditions, &stats);
    assert!(met);
    assert_eq!(reason, Some("max_size:1mb".to_string()));
}

#[tokio::test]
async fn test_rollover_conditions_max_age() {
    let conditions = RolloverConditions {
        max_age: Some("7d".to_string()),
        ..Default::default()
    };

    // 7 days = 7 * 24 * 60 * 60 * 1000 = 604800000 milliseconds
    let stats = IndexStats {
        num_docs: 0,
        size_in_bytes: 0,
        age_in_millis: 604800000 + 1000, // 7 days + 1 second
        num_primary_shards: 1,
    };

    let (met, reason) = check_rollover_conditions(&conditions, &stats);
    assert!(met);
    assert_eq!(reason, Some("max_age:7d".to_string()));
}

#[tokio::test]
async fn test_rollover_conditions_not_met() {
    let conditions = RolloverConditions {
        max_docs: Some(100),
        max_size: Some("1mb".to_string()),
        ..Default::default()
    };

    let stats = IndexStats {
        num_docs: 50,
        size_in_bytes: 500 * 1024, // 500KB
        age_in_millis: 0,
        num_primary_shards: 1,
    };

    let (met, reason) = check_rollover_conditions(&conditions, &stats);
    assert!(!met);
    assert_eq!(reason, None);
}

#[test]
fn test_generate_rollover_index_name() {
    // Test with existing number suffix
    let result = generate_rollover_index_name("logs-000001");
    assert_eq!(result, "logs-000002");

    // Test with different number
    let result = generate_rollover_index_name("logs-000123");
    assert_eq!(result, "logs-000124");

    // Test without number suffix
    let result = generate_rollover_index_name("logs");
    assert_eq!(result, "logs-000001");

    // Test with dash but no number
    let result = generate_rollover_index_name("logs-daily");
    assert_eq!(result, "logs-daily-000001");
}

#[test]
fn test_rollover_request_serialization() {
    let request = RolloverRequest {
        conditions: RolloverConditions {
            max_docs: Some(1000),
            max_size: Some("5gb".to_string()),
            ..Default::default()
        },
        new_index: Some("logs-new".to_string()),
        dry_run: true,
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("max_docs"));
    assert!(json.contains("1000"));
    assert!(json.contains("dry_run"));
}

#[test]
fn test_rollover_response_serialization() {
    let response = RolloverResponse {
        acknowledged: true,
        conditions_met: true,
        old_index: "logs-old".to_string(),
        new_index: "logs-new".to_string(),
        dry_run: false,
        rolled_over_due_to: Some("max_docs:1000".to_string()),
        index_stats: IndexStats {
            num_docs: 1000,
            size_in_bytes: 1024 * 1024,
            age_in_millis: 0,
            num_primary_shards: 1,
        },
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("acknowledged"));
    assert!(json.contains("conditions_met"));
    assert!(json.contains("logs-old"));
    assert!(json.contains("logs-new"));
}

#[test]
fn test_parse_duration_indirect() {
    // Test duration parsing indirectly through check_rollover_conditions
    // 7 days = 7 * 24 * 60 * 60 * 1000 = 604800000 milliseconds

    let conditions = RolloverConditions {
        max_age: Some("7d".to_string()),
        ..Default::default()
    };

    let stats = IndexStats {
        num_docs: 0,
        size_in_bytes: 0,
        age_in_millis: 604800000 + 1000, // 7 days + 1 second
        num_primary_shards: 1,
    };

    let (met, reason) = check_rollover_conditions(&conditions, &stats);
    assert!(met);
    assert_eq!(reason, Some("max_age:7d".to_string()));
}

#[test]
fn test_parse_size() {
    // Note: parse_size is private, testing through check_rollover_conditions
    // This test verifies size parsing indirectly through rollover conditions

    let conditions = RolloverConditions {
        max_size: Some("1mb".to_string()),
        ..Default::default()
    };

    let stats = IndexStats {
        num_docs: 0,
        size_in_bytes: 2 * 1024 * 1024, // 2MB - exceeds limit
        age_in_millis: 0,
        num_primary_shards: 1,
    };

    let (met, reason) = check_rollover_conditions(&conditions, &stats);
    assert!(met);
    assert_eq!(reason, Some("max_size:1mb".to_string()));
}
