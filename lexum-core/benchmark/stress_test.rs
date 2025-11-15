//! Stress testing benchmarks for performance optimization
//!
//! This benchmark suite tests system behavior under extreme conditions
//! to identify breaking points and verify graceful degradation.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lexum_core::*;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::task;

fn create_test_index_with_docs(num_docs: usize) -> (TempDir, Arc<Index>) {
    let temp_dir = TempDir::new().unwrap();
    let manager = IndexManager::new(temp_dir.path());

    let (schema, _) = SchemaBuilder::new()
        .add_text_field("title")
        .add_text_field("content")
        .add_keyword_field("category")
        .add_i64_field("views")
        .build()
        .unwrap();

    let index = tokio::runtime::Runtime::new().unwrap().block_on(async {
        manager
            .create_index("stress_index", schema, IndexSettings::default())
            .await
            .unwrap()
    });

    let store = DocumentStore::new(Arc::new(index.clone()));

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            for i in 0..num_docs {
                let doc = json!({
                    "title": format!("Document Title {}", i),
                    "content": format!("This is the content of document number {} with some searchable text", i),
                    "category": if i % 2 == 0 { "tech" } else { "news" },
                    "views": i as i64 * 10
                });
                store.add_document(doc).await.unwrap();
            }
        });

    std::thread::sleep(std::time::Duration::from_millis(200));

    (temp_dir, Arc::new(index))
}

/// Stress test: High concurrent search operations
fn bench_stress_concurrent_searches(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_concurrent_searches");

    let (_temp_dir, index) = create_test_index_with_docs(10000);
    let executor = Arc::new(SearchExecutor::new(index));

    let query = QueryBuilder::match_query("content", "searchable");

    for concurrency in [1, 10, 50, 100, 200] {
        group.bench_with_input(
            BenchmarkId::new("concurrent", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let mut handles = Vec::new();
                        for _ in 0..concurrency {
                            let executor = executor.clone();
                            let query = query.clone();
                            handles.push(task::spawn(async move {
                                executor.search(black_box(query), 10, 0, None).await
                            }));
                        }
                        for handle in handles {
                            let _ = handle.await;
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

/// Stress test: Large result sets
fn bench_stress_large_results(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_large_results");

    let (_temp_dir, index) = create_test_index_with_docs(100000);
    let executor = SearchExecutor::new(index);

    let query = QueryBuilder::match_query("content", "searchable");

    for limit in [10, 100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::new("limit", limit), &limit, |b, &limit| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let _ = executor
                        .search(black_box(query.clone()), limit, 0, None)
                        .await;
                });
            });
        });
    }

    group.finish();
}

/// Stress test: Complex queries
fn bench_stress_complex_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_complex_queries");

    let (_temp_dir, index) = create_test_index_with_docs(10000);
    let executor = SearchExecutor::new(index);

    // Create increasingly complex queries
    let queries = [
        QueryBuilder::match_query("content", "searchable"),
        Query::Bool(
            QueryBuilder::bool_query()
                .must(QueryBuilder::match_query("content", "searchable"))
                .filter(QueryBuilder::term_query("category", "tech")),
        ),
        Query::Bool(
            QueryBuilder::bool_query()
                .must(QueryBuilder::match_query("content", "searchable"))
                .must(QueryBuilder::match_query("title", "Document"))
                .filter(QueryBuilder::term_query("category", "tech"))
                .filter(Query::Range(
                    QueryBuilder::range_query("views").gte(json!(100)),
                )),
        ),
    ];

    for (i, query) in queries.iter().enumerate() {
        group.bench_function(format!("complexity_{}", i + 1), |b| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let _ = executor.search(black_box(query.clone()), 10, 0, None).await;
                });
            });
        });
    }

    group.finish();
}

/// Stress test: Sustained load
fn bench_stress_sustained_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_sustained_load");

    let (_temp_dir, index) = create_test_index_with_docs(10000);
    let executor = SearchExecutor::new(index);

    let query = QueryBuilder::match_query("content", "searchable");

    group.bench_function("sustained_1000_ops", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                for _ in 0..1000 {
                    let _ = executor.search(black_box(query.clone()), 10, 0, None).await;
                }
            });
        });
    });

    group.finish();
}

/// Stress test: Memory pressure
fn bench_stress_memory_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_memory_pressure");

    // Create multiple indices to test memory pressure
    for num_indices in [1, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("indices", num_indices),
            &num_indices,
            |b, &num_indices| {
                b.iter(|| {
                    let mut indices = Vec::new();
                    for _ in 0..num_indices {
                        let (_temp_dir, index) = create_test_index_with_docs(1000);
                        let executor = SearchExecutor::new(index);
                        indices.push(executor);
                    }
                    black_box(indices);
                });
            },
        );
    }

    group.finish();
}

/// Stress test: Cache eviction under pressure
fn bench_stress_cache_eviction(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_cache_eviction");

    let (_temp_dir, index) = create_test_index_with_docs(10000);
    let executor = SearchExecutor::with_cache_settings(index, 100, 300); // Small cache

    // Generate many unique queries to force evictions
    let queries: Vec<_> = (0..500)
        .map(|i| QueryBuilder::match_query("content", format!("term{i}")))
        .collect();

    group.bench_function("many_unique_queries", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                for query in &queries {
                    let _ = executor.search(black_box(query.clone()), 10, 0, None).await;
                }
            });
        });
    });

    let stats = executor.cache_stats();
    eprintln!(
        "Cache evictions: LRU={}, Expired={}, Total inserts={}",
        stats.lru_evictions, stats.expired_evictions, stats.total_inserts
    );

    group.finish();
}

criterion_group!(
    benches,
    bench_stress_concurrent_searches,
    bench_stress_large_results,
    bench_stress_complex_queries,
    bench_stress_sustained_load,
    bench_stress_memory_pressure,
    bench_stress_cache_eviction
);
criterion_main!(benches);
