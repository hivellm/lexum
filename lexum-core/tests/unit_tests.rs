//! Unit tests for lexum-core

use lexum_core::types::{DocumentId, Score};
use lexum_core::*;
use serde_json::json;

#[test]
fn test_query_builder_all_types() {
    // Match query
    let query = QueryBuilder::match_query("title", "test");
    assert!(matches!(query, Query::Match(_)));

    // Term query
    let query = QueryBuilder::term_query("status", "active");
    assert!(matches!(query, Query::Term(_)));

    // Fuzzy query
    let query = QueryBuilder::fuzzy_query("name", "jhon");
    assert!(matches!(query, Query::Fuzzy(_)));

    // Phrase query
    let query = QueryBuilder::phrase_query("content", "quick brown fox");
    assert!(matches!(query, Query::Phrase(_)));

    // Match all
    let query = QueryBuilder::match_all();
    assert!(matches!(query, Query::MatchAll));

    // Boolean query
    let bool_query = QueryBuilder::bool_query()
        .must(Query::Match(MatchQuery::new("field", "value")))
        .should(Query::Term(TermQuery::new("status", "active")))
        .must_not(Query::Term(TermQuery::new("deleted", "true")));

    assert_eq!(bool_query.must.len(), 1);
    assert_eq!(bool_query.should.len(), 1);
    assert_eq!(bool_query.must_not.len(), 1);
}

#[test]
fn test_fuzzy_query_builder() {
    let query = FuzzyQuery::new("name", "jhon")
        .fuzziness(1)
        .prefix_length(2)
        .transpositions(false);

    assert_eq!(query.field, "name");
    assert_eq!(query.value, "jhon");
    assert_eq!(query.fuzziness, 1);
    assert_eq!(query.prefix_length, 2);
    assert!(!query.transpositions);

    // Test fuzziness capping
    let query = FuzzyQuery::new("name", "test").fuzziness(10);
    assert_eq!(query.fuzziness, 2); // Should cap at 2
}

#[test]
fn test_phrase_query_builder() {
    let query = PhraseQuery::new("content", "quick brown fox");
    assert_eq!(query.field, "content");
    assert_eq!(query.phrase, "quick brown fox");
    assert_eq!(query.slop, 0);

    let query = PhraseQuery::new("content", "fox dog").slop(3);
    assert_eq!(query.slop, 3);
}

#[test]
fn test_bool_query_combinations() {
    let query = BoolQuery::new()
        .must(Query::Match(MatchQuery::new("title", "rust")))
        .must(Query::Match(MatchQuery::new("content", "programming")))
        .should(Query::Term(TermQuery::new("category", "tutorial")))
        .should(Query::Term(TermQuery::new("category", "guide")))
        .must_not(Query::Term(TermQuery::new("status", "draft")))
        .filter(Query::Term(TermQuery::new("published", "true")));

    assert_eq!(query.must.len(), 2);
    assert_eq!(query.should.len(), 2);
    assert_eq!(query.must_not.len(), 1);
    assert_eq!(query.filter.len(), 1);
}

#[test]
fn test_range_query_builder() {
    let query = RangeQuery::new("age").gte(json!(18)).lte(json!(65));

    assert_eq!(query.field, "age");
    assert!(query.gte.is_some());
    assert!(query.lte.is_some());
    assert!(query.gt.is_none());
    assert!(query.lt.is_none());

    let query = RangeQuery::new("score").gt(json!(0.5)).lt(json!(1.0));

    assert!(query.gt.is_some());
    assert!(query.lt.is_some());
}

#[test]
fn test_sort_options() {
    let asc = SortOption::asc("field");
    assert_eq!(asc.field, "field");
    assert_eq!(asc.order, SortOrder::Asc);

    let desc = SortOption::desc("field");
    assert_eq!(desc.field, "field");
    assert_eq!(desc.order, SortOrder::Desc);

    let custom = SortOption::new("custom_field", SortOrder::Asc);
    assert_eq!(custom.field, "custom_field");
    assert_eq!(custom.order, SortOrder::Asc);
}

#[test]
fn test_search_result_creation() {
    let hits = vec![
        SearchHit::new(
            DocumentId::new("doc1"),
            Score::new(0.95),
            json!({"title": "Test"}),
        ),
        SearchHit::new(
            DocumentId::new("doc2"),
            Score::new(0.85),
            json!({"title": "Another Test"}),
        ),
    ];

    let result = SearchResult::new(hits.clone(), 2, 10);
    assert_eq!(result.total, 2);
    assert_eq!(result.took_ms, 10);
    assert_eq!(result.hits.len(), 2);

    let empty = SearchResult::empty();
    assert_eq!(empty.total, 0);
    assert_eq!(empty.hits.len(), 0);
}

#[test]
fn test_config_defaults() {
    let config = Config::default();

    assert_eq!(config.cluster.name, "lexum-cluster");
    assert!(
        config
            .cluster
            .initial_master_nodes
            .contains(&"node-1".to_string())
    );

    assert_eq!(config.network.http_port, 9200);
    assert_eq!(config.network.transport_port, 9300);
    assert_eq!(config.network.host, "0.0.0.0");

    assert_eq!(config.logging.level, "info");
    assert_eq!(config.logging.format, "json");

    assert_eq!(config.path.data, "./data");
}

#[test]
fn test_field_types() {
    use lexum_core::schema::FieldType;

    let text = FieldType::Text;
    assert!(matches!(text, FieldType::Text));

    let keyword = FieldType::Keyword;
    assert!(matches!(keyword, FieldType::Keyword));

    let i64_type = FieldType::I64;
    assert!(matches!(i64_type, FieldType::I64));

    let f64_type = FieldType::F64;
    assert!(matches!(f64_type, FieldType::F64));

    let date = FieldType::Date;
    assert!(matches!(date, FieldType::Date));
}

#[test]
fn test_field_config() {
    use lexum_core::schema::FieldConfig;
    use lexum_core::schema::FieldType;

    let config = FieldConfig {
        name: "test_field".to_string(),
        field_type: FieldType::Text,
        stored: true,
        indexed: true,
        fast: false,
    };

    assert_eq!(config.name, "test_field");
    assert!(matches!(config.field_type, FieldType::Text));
    assert!(config.stored);
    assert!(config.indexed);
    assert!(!config.fast);
}

#[test]
fn test_types() {
    let doc_id = DocumentId::new("test_id");
    assert_eq!(doc_id.as_str(), "test_id");
    assert_eq!(doc_id.to_string(), "test_id");

    let index_name = lexum_core::types::IndexName::new("test_index");
    assert_eq!(index_name.as_str(), "test_index");

    let score = Score::new(0.95_f32);
    assert_eq!(score.value(), 0.95_f32);
}

#[test]
fn test_error_types() {
    let validation_error = Error::Validation("Invalid input".to_string());
    assert!(matches!(validation_error, Error::Validation(_)));

    let config_error = Error::Config("Bad config".to_string());
    assert!(matches!(config_error, Error::Config(_)));

    let io_error = Error::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "File not found",
    ));
    assert!(matches!(io_error, Error::Io(_)));
}

#[test]
fn test_query_serialization() {
    let query = QueryBuilder::match_query("title", "test");
    let json = serde_json::to_string(&query).unwrap();
    assert!(json.contains("match"));

    let deserialized: Query = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Query::Match(_)));
}

#[test]
fn test_bool_query_serialization() {
    let query = BoolQuery::new()
        .must(Query::Match(MatchQuery::new("title", "rust")))
        .should(Query::Term(TermQuery::new("category", "programming")));

    let json = serde_json::to_string(&Query::Bool(query)).unwrap();
    assert!(json.contains("must"));
    assert!(json.contains("should"));

    let deserialized: Query = serde_json::from_str(&json).unwrap();
    match deserialized {
        Query::Bool(bq) => {
            assert_eq!(bq.must.len(), 1);
            assert_eq!(bq.should.len(), 1);
        }
        _ => panic!("Expected Bool query"),
    }
}

#[test]
fn test_fuzzy_query_serialization() {
    let query = FuzzyQuery::new("name", "jhon").fuzziness(1);
    let json = serde_json::to_string(&Query::Fuzzy(query)).unwrap();
    assert!(json.contains("fuzzy"));
    assert!(json.contains("fuzziness"));

    let deserialized: Query = serde_json::from_str(&json).unwrap();
    match deserialized {
        Query::Fuzzy(fq) => {
            assert_eq!(fq.field, "name");
            assert_eq!(fq.value, "jhon");
            assert_eq!(fq.fuzziness, 1);
        }
        _ => panic!("Expected Fuzzy query"),
    }
}

#[test]
fn test_phrase_query_serialization() {
    let query = PhraseQuery::new("content", "quick fox").slop(2);
    let json = serde_json::to_string(&Query::Phrase(query)).unwrap();
    assert!(json.contains("phrase"));
    assert!(json.contains("slop"));

    let deserialized: Query = serde_json::from_str(&json).unwrap();
    match deserialized {
        Query::Phrase(pq) => {
            assert_eq!(pq.field, "content");
            assert_eq!(pq.phrase, "quick fox");
            assert_eq!(pq.slop, 2);
        }
        _ => panic!("Expected Phrase query"),
    }
}

#[test]
fn test_sort_option_serialization() {
    let sort = SortOption::desc("timestamp");
    let json = serde_json::to_string(&sort).unwrap();
    assert!(json.contains("timestamp"));
    assert!(json.contains("desc"));

    let deserialized: SortOption = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.field, "timestamp");
    assert_eq!(deserialized.order, SortOrder::Desc);
}

#[test]
fn test_search_result_serialization() {
    let result = SearchResult::new(vec![], 0, 5);
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("hits"));
    assert!(json.contains("total"));
    assert!(json.contains("took_ms"));

    let deserialized: SearchResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.total, 0);
    assert_eq!(deserialized.took_ms, 5);
}
