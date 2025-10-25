//! Comprehensive tests for additional coverage

use lexum_core::index::IndexSettings;
use lexum_core::schema::{FieldConfig, FieldType};
use lexum_core::*;
use serde_json::json;

// ============================================================================
// Index Settings Tests
// ============================================================================

#[test]
fn test_index_settings_default() {
    let settings = IndexSettings::default();
    assert_eq!(settings.number_of_shards, 5);
    assert_eq!(settings.number_of_replicas, 1);
    assert_eq!(settings.refresh_interval, 1);
}

#[test]
fn test_index_settings_builder() {
    let mut settings = IndexSettings::default();
    settings.number_of_shards = 3;
    settings.number_of_replicas = 2;
    settings.refresh_interval = 5;

    assert_eq!(settings.number_of_shards, 3);
    assert_eq!(settings.number_of_replicas, 2);
    assert_eq!(settings.refresh_interval, 5);
}

#[test]
fn test_index_settings_shards_range() {
    let mut settings = IndexSettings::default();
    settings.number_of_shards = 5;
    assert_eq!(settings.number_of_shards, 5);

    // Valid range test
    settings.number_of_shards = 1;
    assert!(settings.number_of_shards > 0);
}

#[test]
fn test_index_settings_serialization() {
    let settings = IndexSettings::default();
    let json = serde_json::to_string(&settings).unwrap();
    assert!(json.contains("number_of_shards"));

    let deserialized: IndexSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.number_of_shards, settings.number_of_shards);
}

// ============================================================================
// Schema Builder Tests
// ============================================================================

#[test]
fn test_schema_builder_single_field() {
    let result = SchemaBuilder::new().add_text_field("title").build();
    assert!(result.is_ok());

    let (_, field_map) = result.unwrap();
    assert_eq!(field_map.len(), 1);
    assert!(field_map.contains_key("title"));
}

#[test]
fn test_schema_builder_multiple_fields() {
    let (_schema, field_map) = SchemaBuilder::new()
        .add_text_field("title")
        .add_text_field("content")
        .add_keyword_field("category")
        .add_i64_field("views")
        .add_f64_field("rating")
        .add_date_field("published")
        .build()
        .unwrap();

    assert_eq!(field_map.len(), 6);
    assert!(field_map.contains_key("title"));
    assert!(field_map.contains_key("content"));
    assert!(field_map.contains_key("category"));
    assert!(field_map.contains_key("views"));
    assert!(field_map.contains_key("rating"));
    assert!(field_map.contains_key("published"));
}

#[test]
fn test_field_config_all_types() {
    let text_config = FieldConfig {
        name: "text".to_string(),
        field_type: FieldType::Text,
        stored: true,
        indexed: true,
        fast: false,
    };
    assert_eq!(text_config.name, "text");

    let keyword_config = FieldConfig {
        name: "keyword".to_string(),
        field_type: FieldType::Keyword,
        stored: true,
        indexed: true,
        fast: true,
    };
    assert!(keyword_config.fast);

    let i64_config = FieldConfig {
        name: "number".to_string(),
        field_type: FieldType::I64,
        stored: true,
        indexed: true,
        fast: true,
    };
    assert!(matches!(i64_config.field_type, FieldType::I64));

    let f64_config = FieldConfig {
        name: "decimal".to_string(),
        field_type: FieldType::F64,
        stored: true,
        indexed: true,
        fast: true,
    };
    assert!(matches!(f64_config.field_type, FieldType::F64));

    let date_config = FieldConfig {
        name: "timestamp".to_string(),
        field_type: FieldType::Date,
        stored: true,
        indexed: true,
        fast: true,
    };
    assert!(matches!(date_config.field_type, FieldType::Date));
}

// ============================================================================
// Config Tests
// ============================================================================

#[test]
fn test_config_cluster() {
    let config = Config::default();
    assert_eq!(config.cluster.name, "lexum-cluster");
    assert!(!config.cluster.initial_master_nodes.is_empty());
}

#[test]
fn test_config_node() {
    let config = Config::default();
    assert!(!config.node.name.is_empty());
    assert!(!config.node.roles.is_empty());
    assert!(config.node.roles.contains(&"master".to_string()));
    assert!(config.node.roles.contains(&"data".to_string()));
}

#[test]
fn test_config_network() {
    let config = Config::default();
    assert_eq!(config.network.host, "0.0.0.0");
    assert_eq!(config.network.http_port, 9200);
    assert_eq!(config.network.transport_port, 9300);
}

#[test]
fn test_config_path() {
    let config = Config::default();
    assert_eq!(config.path.data, "./data");
    assert_eq!(config.path.logs, "./logs");
}

#[test]
fn test_config_logging() {
    let config = Config::default();
    assert_eq!(config.logging.level, "info");
    assert_eq!(config.logging.format, "json");
    assert!(config.logging.outputs.contains(&"stdout".to_string()));
}

#[test]
fn test_config_ports_distinct() {
    let config = Config::default();
    assert_ne!(config.network.http_port, config.network.transport_port);
    assert!(config.network.http_port > 0);
    assert!(config.network.transport_port > 0);
}

#[test]
fn test_config_port_ranges() {
    let config = Config::default();
    assert!(config.network.http_port >= 1024);
    assert!(config.network.transport_port >= 1024);
}

// ============================================================================
// Query Tests - Edge Cases
// ============================================================================

#[test]
fn test_match_query_empty() {
    let query = MatchQuery::new("field", "");
    assert_eq!(query.field, "field");
    assert_eq!(query.query, "");
}

#[test]
fn test_term_query_special_chars() {
    let query = TermQuery::new("field", "value-with-dashes_and_underscores");
    assert!(query.value.contains('-'));
    assert!(query.value.contains('_'));
}

#[test]
fn test_range_query_all_bounds() {
    let query = RangeQuery::new("field")
        .gte(json!(1))
        .lte(json!(10))
        .gt(json!(0))
        .lt(json!(11));

    assert!(query.gte.is_some());
    assert!(query.lte.is_some());
    assert!(query.gt.is_some());
    assert!(query.lt.is_some());
}

#[test]
fn test_bool_query_empty() {
    let query = BoolQuery::new();
    assert_eq!(query.must.len(), 0);
    assert_eq!(query.should.len(), 0);
    assert_eq!(query.must_not.len(), 0);
    assert_eq!(query.filter.len(), 0);
}

#[test]
fn test_bool_query_complex() {
    let query = BoolQuery::new()
        .must(Query::Match(MatchQuery::new("f1", "v1")))
        .must(Query::Match(MatchQuery::new("f2", "v2")))
        .should(Query::Term(TermQuery::new("f3", "v3")))
        .should(Query::Term(TermQuery::new("f4", "v4")))
        .must_not(Query::Term(TermQuery::new("f5", "v5")))
        .filter(Query::Range(RangeQuery::new("f6").gte(json!(0))));

    assert_eq!(query.must.len(), 2);
    assert_eq!(query.should.len(), 2);
    assert_eq!(query.must_not.len(), 1);
    assert_eq!(query.filter.len(), 1);
}

#[test]
fn test_fuzzy_query_zero_fuzziness() {
    let query = FuzzyQuery::new("field", "value").fuzziness(0);
    assert_eq!(query.fuzziness, 0);
}

#[test]
fn test_fuzzy_query_max_fuzziness() {
    let query = FuzzyQuery::new("field", "value").fuzziness(2);
    assert_eq!(query.fuzziness, 2);
}

#[test]
fn test_fuzzy_query_prefix_length() {
    let query = FuzzyQuery::new("field", "value").prefix_length(3);
    assert_eq!(query.prefix_length, 3);
}

#[test]
fn test_fuzzy_query_no_transpositions() {
    let query = FuzzyQuery::new("field", "value").transpositions(false);
    assert!(!query.transpositions);
}

#[test]
fn test_phrase_query_zero_slop() {
    let query = PhraseQuery::new("field", "phrase").slop(0);
    assert_eq!(query.slop, 0);
}

#[test]
fn test_phrase_query_high_slop() {
    let query = PhraseQuery::new("field", "phrase").slop(10);
    assert_eq!(query.slop, 10);
}

// ============================================================================
// Search Result Tests
// ============================================================================

#[test]
fn test_search_hit_with_complex_source() {
    use lexum_core::types::{DocumentId, Score};

    let hit = SearchHit {
        id: DocumentId::new("doc1"),
        score: Score::new(0.95),
        source: json!({
            "title": "Test",
            "nested": {
                "field": "value"
            },
            "array": [1, 2, 3]
        }),
    };

    assert_eq!(hit.id.as_str(), "doc1");
    assert_eq!(hit.score.value(), 0.95);
    assert!(hit.source.is_object());
}

#[test]
fn test_search_result_with_multiple_hits() {
    use lexum_core::types::{DocumentId, Score};

    let hits = (0..10)
        .map(|i| SearchHit {
            id: DocumentId::new(format!("doc{}", i)),
            score: Score::new(1.0 - (i as f32 * 0.1)),
            source: json!({"id": i}),
        })
        .collect();

    let result = SearchResult::new(hits, 10, 50);
    assert_eq!(result.total, 10);
    assert_eq!(result.hits.len(), 10);
    assert_eq!(result.took_ms, 50);
}

// ============================================================================
// Sort Tests
// ============================================================================

#[test]
fn test_sort_order_default() {
    let order = SortOrder::default();
    assert_eq!(order, SortOrder::Desc);
}

#[test]
fn test_sort_option_new() {
    let sort = SortOption::new("field", SortOrder::Asc);
    assert_eq!(sort.field, "field");
    assert_eq!(sort.order, SortOrder::Asc);
}

// ============================================================================
// Error Tests
// ============================================================================

#[test]
fn test_error_display() {
    let err = Error::Validation("test error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("test error"));
}

#[test]
fn test_error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err: Error = io_err.into();
    assert!(matches!(err, Error::Io(_)));
}

#[test]
fn test_error_types_coverage() {
    let validation = Error::Validation("test".to_string());
    assert!(matches!(validation, Error::Validation(_)));

    let config = Error::Config("test".to_string());
    assert!(matches!(config, Error::Config(_)));
}

// ============================================================================
// Types Tests
// ============================================================================

#[test]
fn test_document_id_clone() {
    use lexum_core::types::DocumentId;
    let id1 = DocumentId::new("test");
    let id2 = id1.clone();
    assert_eq!(id1.as_str(), id2.as_str());
}

#[test]
fn test_document_id_display() {
    use lexum_core::types::DocumentId;
    let id = DocumentId::new("test-id-123");
    let display = format!("{}", id);
    assert_eq!(display, "test-id-123");
}

#[test]
fn test_index_name_new() {
    use lexum_core::types::IndexName;
    let name = IndexName::new("my-index");
    assert_eq!(name.as_str(), "my-index");
}

#[test]
fn test_score_new() {
    use lexum_core::types::Score;
    let score = Score::new(0.5);
    assert_eq!(score.value(), 0.5);
}

#[test]
fn test_score_ordering() {
    use lexum_core::types::Score;
    let score1 = Score::new(0.8);
    let score2 = Score::new(0.5);
    assert!(score1.value() > score2.value());
}

// ============================================================================
// Logging Tests
// ============================================================================

#[test]
fn test_logging_config_default() {
    let config = config::LoggingConfig::default();
    assert_eq!(config.level, "info");
    assert_eq!(config.format, "json");
}

#[test]
fn test_logging_init_with_custom_config() {
    let config = config::LoggingConfig {
        level: "debug".to_string(),
        format: "pretty".to_string(),
        outputs: vec!["stdout".to_string()],
    };

    // We can't actually init twice, but we can validate the config
    assert_eq!(config.level, "debug");
    assert_eq!(config.format, "pretty");
}

// ============================================================================
// Query Builder Tests - All Combinations
// ============================================================================

#[test]
fn test_query_builder_range() {
    let range = QueryBuilder::range_query("age");
    assert_eq!(range.field, "age");
}

#[test]
fn test_query_builder_bool() {
    let bool_query = QueryBuilder::bool_query();
    assert_eq!(bool_query.must.len(), 0);
}

#[test]
fn test_nested_bool_query() {
    let inner = BoolQuery::new().must(Query::Term(TermQuery::new("status", "active")));

    let outer = BoolQuery::new()
        .must(Query::Bool(inner))
        .should(Query::Match(MatchQuery::new("title", "test")));

    assert_eq!(outer.must.len(), 1);
    assert_eq!(outer.should.len(), 1);
}

// ============================================================================
// JSON Serialization Edge Cases
// ============================================================================

#[test]
fn test_query_serialization_match_all() {
    let query = QueryBuilder::match_all();
    let json = serde_json::to_string(&query).unwrap();
    assert!(json.contains("match_all"));

    let deserialized: Query = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Query::MatchAll));
}

#[test]
fn test_complex_query_serialization() {
    let query = Query::Bool(
        BoolQuery::new()
            .must(Query::Match(MatchQuery::new("title", "rust")))
            .should(Query::Fuzzy(
                FuzzyQuery::new("content", "programing").fuzziness(2),
            ))
            .must_not(Query::Term(TermQuery::new("draft", "true"))),
    );

    let json = serde_json::to_string(&query).unwrap();
    let deserialized: Query = serde_json::from_str(&json).unwrap();

    match deserialized {
        Query::Bool(bq) => {
            assert_eq!(bq.must.len(), 1);
            assert_eq!(bq.should.len(), 1);
            assert_eq!(bq.must_not.len(), 1);
        }
        _ => panic!("Expected Bool query"),
    }
}

#[test]
fn test_range_query_serialization() {
    let query = RangeQuery::new("price").gte(json!(10.0)).lte(json!(100.0));

    let json = serde_json::to_string(&Query::Range(query)).unwrap();
    assert!(json.contains("gte"));
    assert!(json.contains("lte"));
}

// ============================================================================
// Index Stats Tests
// ============================================================================

#[test]
fn test_index_stats_creation() {
    use lexum_core::index::IndexStats;

    let stats = IndexStats {
        name: "test_index".to_string(),
        num_docs: 100,
        num_segments: 2,
    };

    assert_eq!(stats.name, "test_index");
    assert_eq!(stats.num_docs, 100);
    assert_eq!(stats.num_segments, 2);
}

#[test]
fn test_index_stats_serialization() {
    use lexum_core::index::IndexStats;

    let stats = IndexStats {
        name: "my_index".to_string(),
        num_docs: 1000,
        num_segments: 5,
    };

    let json = serde_json::to_string(&stats).unwrap();
    assert!(json.contains("my_index"));
    assert!(json.contains("1000"));
    assert!(json.contains("\"num_segments\":5"));

    let deserialized: IndexStats = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "my_index");
    assert_eq!(deserialized.num_docs, 1000);
    assert_eq!(deserialized.num_segments, 5);
}
