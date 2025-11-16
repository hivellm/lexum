//! Integration tests for aggregations
//!
//! Tests aggregations with real search execution and index operations.

use lexum_core::aggregation::{
    AggregationExecutor, AggregationSpec, BoxplotAggregation, ExtendedStatsAggregation,
    MedianAbsoluteDeviationAggregation, RateAggregation, StringStatsAggregation, TTestAggregation,
    TopHitsAggregation, TopHitsHighlight, WeightedAverageAggregation,
};
use lexum_core::index::{IndexManager, IndexSettings};
use lexum_core::query::{Query, TermQuery};
use lexum_core::schema::SchemaBuilder;
use lexum_core::search::SearchExecutor;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper function to create a test index with sample documents
async fn create_test_index() -> (Arc<lexum_core::index::Index>, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let manager = Arc::new(IndexManager::new(temp_dir.path()));

    // Create schema
    let schema_builder = SchemaBuilder::new()
        .add_text_field("title")
        .add_i64_field("value")
        .add_f64_field("weight")
        .add_keyword_field("category")
        .add_text_field("text");
    let (schema, _) = schema_builder.build().unwrap();
    let settings = IndexSettings::new();

    // Create index
    let index = manager
        .create_index("test_index", schema, settings)
        .await
        .unwrap();

    // Add sample documents
    let docs = vec![
        serde_json::json!({
            "title": "Document 1",
            "value": 10,
            "weight": 2.0,
            "category": "A",
            "text": "This is a test document"
        }),
        serde_json::json!({
            "title": "Document 2",
            "value": 20,
            "weight": 3.0,
            "category": "A",
            "text": "Another test document"
        }),
        serde_json::json!({
            "title": "Document 3",
            "value": 30,
            "weight": 1.0,
            "category": "B",
            "text": "Third test document"
        }),
        serde_json::json!({
            "title": "Document 4",
            "value": 40,
            "weight": 4.0,
            "category": "B",
            "text": "Fourth test document"
        }),
        serde_json::json!({
            "title": "Document 5",
            "value": 50,
            "weight": 2.5,
            "category": "A",
            "text": "Fifth test document"
        }),
    ];

    for doc in docs {
        index.add_document(doc).await.unwrap();
    }

    index.commit().await.unwrap();
    (index, temp_dir)
}

#[tokio::test]
async fn test_extended_stats_aggregation_integration() {
    let (index, _temp_dir) = create_test_index().await;
    let executor = SearchExecutor::new(index);

    let query = Query::MatchAll;
    let mut aggs = HashMap::new();
    aggs.insert(
        "extended_stats".to_string(),
        AggregationSpec::ExtendedStats(ExtendedStatsAggregation::new("value")),
    );

    let aggs_vec: Vec<AggregationSpec> = aggs.values().cloned().collect();
    let aggs_slice: Option<&[AggregationSpec]> = Some(&aggs_vec);

    let result = executor
        .search_with_aggregations(query, 10, 0, None, aggs_slice)
        .await
        .unwrap();

    assert!(result.aggregations.is_some());
    let aggs_result = result.aggregations.unwrap();
    assert!(aggs_result.contains_key("extended_stats"));

    if let Some(lexum_core::aggregation::AggregationResult::Metric(metric_result)) =
        aggs_result.get("extended_stats")
    {
        let stats: serde_json::Value = serde_json::from_value(metric_result.value.clone()).unwrap();
        assert!(stats.get("count").is_some());
        assert!(stats.get("min").is_some());
        assert!(stats.get("max").is_some());
        assert!(stats.get("avg").is_some());
        assert!(stats.get("sum").is_some());
        assert!(stats.get("variance").is_some());
        assert!(stats.get("std_deviation").is_some());
    } else {
        panic!("Expected Metric result");
    }
}

#[tokio::test]
async fn test_median_absolute_deviation_aggregation_integration() {
    let (index, _temp_dir) = create_test_index().await;
    let executor = SearchExecutor::new(index);

    let query = Query::MatchAll;
    let mut aggs = HashMap::new();
    aggs.insert(
        "mad".to_string(),
        AggregationSpec::MedianAbsoluteDeviation(MedianAbsoluteDeviationAggregation::new("value")),
    );

    let aggs_vec: Vec<AggregationSpec> = aggs.values().cloned().collect();
    let aggs_slice: Option<&[AggregationSpec]> = Some(&aggs_vec);

    let result = executor
        .search_with_aggregations(query, 10, 0, None, aggs_slice)
        .await
        .unwrap();

    assert!(result.aggregations.is_some());
    let aggs_result = result.aggregations.unwrap();
    assert!(aggs_result.contains_key("mad"));

    if let Some(lexum_core::aggregation::AggregationResult::Metric(metric_result)) =
        aggs_result.get("mad")
    {
        let mad: serde_json::Value = serde_json::from_value(metric_result.value.clone()).unwrap();
        assert!(mad.get("value").is_some());
    } else {
        panic!("Expected Metric result");
    }
}

#[tokio::test]
async fn test_weighted_average_aggregation_integration() {
    let (index, _temp_dir) = create_test_index().await;
    let executor = SearchExecutor::new(index);

    let query = Query::MatchAll;
    let mut aggs = HashMap::new();
    aggs.insert(
        "weighted_avg".to_string(),
        AggregationSpec::WeightedAverage(WeightedAverageAggregation::new("value", "weight")),
    );

    let aggs_vec: Vec<AggregationSpec> = aggs.values().cloned().collect();
    let aggs_slice: Option<&[AggregationSpec]> = Some(&aggs_vec);

    let result = executor
        .search_with_aggregations(query, 10, 0, None, aggs_slice)
        .await
        .unwrap();

    assert!(result.aggregations.is_some());
    let aggs_result = result.aggregations.unwrap();
    assert!(aggs_result.contains_key("weighted_avg"));

    if let Some(lexum_core::aggregation::AggregationResult::Metric(metric_result)) =
        aggs_result.get("weighted_avg")
    {
        let weighted_avg: serde_json::Value =
            serde_json::from_value(metric_result.value.clone()).unwrap();
        assert!(weighted_avg.get("value").is_some());
    } else {
        panic!("Expected Metric result");
    }
}

#[tokio::test]
async fn test_string_stats_aggregation_integration() {
    let (index, _temp_dir) = create_test_index().await;
    let executor = SearchExecutor::new(index);

    let query = Query::MatchAll;
    let mut aggs = HashMap::new();
    aggs.insert(
        "string_stats".to_string(),
        AggregationSpec::StringStats(StringStatsAggregation::new("text")),
    );

    let aggs_vec: Vec<AggregationSpec> = aggs.values().cloned().collect();
    let aggs_slice: Option<&[AggregationSpec]> = Some(&aggs_vec);

    let result = executor
        .search_with_aggregations(query, 10, 0, None, aggs_slice)
        .await
        .unwrap();

    assert!(result.aggregations.is_some());
    let aggs_result = result.aggregations.unwrap();
    assert!(aggs_result.contains_key("string_stats"));

    if let Some(lexum_core::aggregation::AggregationResult::Metric(metric_result)) =
        aggs_result.get("string_stats")
    {
        let stats: serde_json::Value = serde_json::from_value(metric_result.value.clone()).unwrap();
        assert!(stats.get("count").is_some());
        assert!(stats.get("min_length").is_some());
        assert!(stats.get("max_length").is_some());
        assert!(stats.get("avg_length").is_some());
    } else {
        panic!("Expected Metric result");
    }
}

#[tokio::test]
async fn test_boxplot_aggregation_integration() {
    let (index, _temp_dir) = create_test_index().await;
    let executor = SearchExecutor::new(index);

    let query = Query::MatchAll;
    let mut aggs = HashMap::new();
    aggs.insert(
        "boxplot".to_string(),
        AggregationSpec::Boxplot(BoxplotAggregation::new("value")),
    );

    let aggs_vec: Vec<AggregationSpec> = aggs.values().cloned().collect();
    let aggs_slice: Option<&[AggregationSpec]> = Some(&aggs_vec);

    let result = executor
        .search_with_aggregations(query, 10, 0, None, aggs_slice)
        .await
        .unwrap();

    assert!(result.aggregations.is_some());
    let aggs_result = result.aggregations.unwrap();
    assert!(aggs_result.contains_key("boxplot"));

    if let Some(lexum_core::aggregation::AggregationResult::Metric(metric_result)) =
        aggs_result.get("boxplot")
    {
        let boxplot: serde_json::Value =
            serde_json::from_value(metric_result.value.clone()).unwrap();
        assert!(boxplot.get("min").is_some());
        assert!(boxplot.get("max").is_some());
        assert!(boxplot.get("q1").is_some());
        assert!(boxplot.get("median").is_some());
        assert!(boxplot.get("q3").is_some());
    } else {
        panic!("Expected Metric result");
    }
}

#[tokio::test]
async fn test_rate_aggregation_integration() {
    let (index, _temp_dir) = create_test_index().await;
    let executor = SearchExecutor::new(index);

    let query = Query::MatchAll;
    let mut aggs = HashMap::new();
    aggs.insert(
        "rate".to_string(),
        AggregationSpec::Rate(RateAggregation::new("value").mode("sum")),
    );

    let aggs_vec: Vec<AggregationSpec> = aggs.values().cloned().collect();
    let aggs_slice: Option<&[AggregationSpec]> = Some(&aggs_vec);

    let result = executor
        .search_with_aggregations(query, 10, 0, None, aggs_slice)
        .await
        .unwrap();

    assert!(result.aggregations.is_some());
    let aggs_result = result.aggregations.unwrap();
    assert!(aggs_result.contains_key("rate"));

    if let Some(lexum_core::aggregation::AggregationResult::Metric(metric_result)) =
        aggs_result.get("rate")
    {
        let rate: serde_json::Value = serde_json::from_value(metric_result.value.clone()).unwrap();
        assert!(rate.get("unit").is_some());
        // value may be None if no documents match
    } else {
        panic!("Expected Metric result");
    }
}

#[tokio::test]
async fn test_t_test_aggregation_integration() {
    let (index, _temp_dir) = create_test_index().await;
    let executor = SearchExecutor::new(index);

    let query = Query::MatchAll;
    let a_filter = Query::Term(TermQuery::new("category", "A"));
    let b_filter = Query::Term(TermQuery::new("category", "B"));
    let mut aggs = HashMap::new();
    aggs.insert(
        "t_test".to_string(),
        AggregationSpec::TTest(TTestAggregation::new("value", a_filter, b_filter)),
    );

    let aggs_vec: Vec<AggregationSpec> = aggs.values().cloned().collect();
    let aggs_slice: Option<&[AggregationSpec]> = Some(&aggs_vec);

    let result = executor
        .search_with_aggregations(query, 10, 0, None, aggs_slice)
        .await
        .unwrap();

    assert!(result.aggregations.is_some());
    let aggs_result = result.aggregations.unwrap();
    assert!(aggs_result.contains_key("t_test"));

    if let Some(lexum_core::aggregation::AggregationResult::Metric(metric_result)) =
        aggs_result.get("t_test")
    {
        let t_test: serde_json::Value =
            serde_json::from_value(metric_result.value.clone()).unwrap();
        assert!(t_test.get("a").is_some());
        assert!(t_test.get("b").is_some());
    } else {
        panic!("Expected Metric result");
    }
}

#[tokio::test]
async fn test_top_hits_aggregation_integration() {
    let (index, _temp_dir) = create_test_index().await;
    let executor = SearchExecutor::new(index);

    let query = Query::MatchAll;
    let mut aggs = HashMap::new();
    aggs.insert(
        "top_hits".to_string(),
        AggregationSpec::TopHits(TopHitsAggregation::new().size(3)),
    );

    let aggs_vec: Vec<AggregationSpec> = aggs.values().cloned().collect();
    let aggs_slice: Option<&[AggregationSpec]> = Some(&aggs_vec);

    let result = executor
        .search_with_aggregations(query, 10, 0, None, aggs_slice)
        .await
        .unwrap();

    assert!(result.aggregations.is_some());
    let aggs_result = result.aggregations.unwrap();
    assert!(aggs_result.contains_key("top_hits"));

    if let Some(lexum_core::aggregation::AggregationResult::Metric(metric_result)) =
        aggs_result.get("top_hits")
    {
        let top_hits: serde_json::Value =
            serde_json::from_value(metric_result.value.clone()).unwrap();
        assert!(top_hits.get("total").is_some());
        assert!(top_hits.get("hits").is_some());
        let hits = top_hits.get("hits").unwrap().as_array().unwrap();
        assert!(hits.len() <= 3); // Should respect size limit
    } else {
        panic!("Expected Metric result");
    }
}

#[tokio::test]
async fn test_top_hits_aggregation_with_sort_integration() {
    let (index, _temp_dir) = create_test_index().await;
    let executor = SearchExecutor::new(index);

    let query = Query::MatchAll;
    let mut aggs = HashMap::new();
    aggs.insert(
        "top_hits".to_string(),
        AggregationSpec::TopHits(
            TopHitsAggregation::new()
                .size(3)
                .sort(lexum_core::search::result::SortOption::asc("title")),
        ),
    );

    let aggs_vec: Vec<AggregationSpec> = aggs.values().cloned().collect();
    let aggs_slice: Option<&[AggregationSpec]> = Some(&aggs_vec);

    let result = executor
        .search_with_aggregations(query, 10, 0, None, aggs_slice)
        .await
        .unwrap();

    assert!(result.aggregations.is_some());
    let aggs_result = result.aggregations.unwrap();
    assert!(aggs_result.contains_key("top_hits"));

    if let Some(lexum_core::aggregation::AggregationResult::Metric(metric_result)) =
        aggs_result.get("top_hits")
    {
        let top_hits: serde_json::Value =
            serde_json::from_value(metric_result.value.clone()).unwrap();
        let hits = top_hits.get("hits").unwrap().as_array().unwrap();
        assert!(hits.len() <= 3);
    } else {
        panic!("Expected Metric result");
    }
}

#[tokio::test]
async fn test_top_hits_aggregation_with_highlight_integration() {
    let (index, _temp_dir) = create_test_index().await;
    let executor = SearchExecutor::new(index);

    let query = Query::MatchAll;
    let highlight = TopHitsHighlight {
        fields: vec!["text".to_string()],
        pre_tag: "<mark>".to_string(),
        post_tag: "</mark>".to_string(),
        fragment_size: 100,
        max_fragments: 3,
    };
    let mut aggs = HashMap::new();
    aggs.insert(
        "top_hits".to_string(),
        AggregationSpec::TopHits(TopHitsAggregation::new().size(2).highlight(highlight)),
    );

    let aggs_vec: Vec<AggregationSpec> = aggs.values().cloned().collect();
    let aggs_slice: Option<&[AggregationSpec]> = Some(&aggs_vec);

    let result = executor
        .search_with_aggregations(query, 10, 0, None, aggs_slice)
        .await
        .unwrap();

    assert!(result.aggregations.is_some());
    let aggs_result = result.aggregations.unwrap();
    assert!(aggs_result.contains_key("top_hits"));

    if let Some(lexum_core::aggregation::AggregationResult::Metric(metric_result)) =
        aggs_result.get("top_hits")
    {
        let top_hits: serde_json::Value =
            serde_json::from_value(metric_result.value.clone()).unwrap();
        let hits = top_hits.get("hits").unwrap().as_array().unwrap();
        assert!(hits.len() <= 2);
    } else {
        panic!("Expected Metric result");
    }
}

#[tokio::test]
async fn test_multiple_aggregations_integration() {
    let (index, _temp_dir) = create_test_index().await;
    let executor = SearchExecutor::new(index);

    let query = Query::MatchAll;
    let mut aggs = HashMap::new();
    aggs.insert(
        "extended_stats".to_string(),
        AggregationSpec::ExtendedStats(ExtendedStatsAggregation::new("value")),
    );
    aggs.insert(
        "boxplot".to_string(),
        AggregationSpec::Boxplot(BoxplotAggregation::new("value")),
    );
    aggs.insert(
        "top_hits".to_string(),
        AggregationSpec::TopHits(TopHitsAggregation::new().size(2)),
    );

    let aggs_vec: Vec<AggregationSpec> = aggs.values().cloned().collect();
    let aggs_slice: Option<&[AggregationSpec]> = Some(&aggs_vec);

    let result = executor
        .search_with_aggregations(query, 10, 0, None, aggs_slice)
        .await
        .unwrap();

    assert!(result.aggregations.is_some());
    let aggs_result = result.aggregations.unwrap();
    assert!(aggs_result.contains_key("extended_stats"));
    assert!(aggs_result.contains_key("boxplot"));
    assert!(aggs_result.contains_key("top_hits"));
}

#[tokio::test]
async fn test_aggregation_executor_merge_integration() {
    let (index, _temp_dir) = create_test_index().await;
    let executor = AggregationExecutor::new(index.clone(), Default::default());

    use lexum_core::search::result::SearchHit;
    use lexum_core::types::{DocumentId, Score};

    // Create sample hits
    let mut hits1 = vec![];
    hits1.push(SearchHit {
        id: DocumentId::new("1"),
        score: Score::new(0.9),
        source: serde_json::json!({ "value": 10 }),
    });

    let mut hits2 = vec![];
    hits2.push(SearchHit {
        id: DocumentId::new("2"),
        score: Score::new(0.8),
        source: serde_json::json!({ "value": 20 }),
    });

    // Execute aggregations on both sets
    let mut aggs = vec![AggregationSpec::ExtendedStats(
        ExtendedStatsAggregation::new("value"),
    )];

    let result1 = executor.execute(&aggs, &hits1).unwrap();
    let result2 = executor.execute(&aggs, &hits2).unwrap();

    // Merge results
    let merged_result = executor
        .merge(
            &aggs[0],
            &[
                result1.get("extended_stats").unwrap().clone(),
                result2.get("extended_stats").unwrap().clone(),
            ],
        )
        .unwrap();

    // Verify merged result
    if let lexum_core::aggregation::AggregationResult::Metric(metric_result) = merged_result {
        let stats: serde_json::Value = serde_json::from_value(metric_result.value).unwrap();
        let count = stats.get("count").unwrap().as_u64().unwrap();
        assert_eq!(count, 2); // Should have merged both results
    } else {
        panic!("Expected Metric result");
    }
}
