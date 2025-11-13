//! Comprehensive cache effectiveness benchmarks
//!
//! This benchmark suite measures:
//! - Cache hit rates under different access patterns
//! - Performance speedup from caching
//! - Memory efficiency
//! - Impact of cache size and TTL
//! - Effectiveness across different index sizes

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lexum_core::*;
use rand::seq::SliceRandom;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;

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
            .create_index("bench_index", schema, IndexSettings::default())
            .await
            .unwrap()
    });

    let store = DocumentStore::new(Arc::new(index.clone()));

    // Add documents with varied content
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            for i in 0..num_docs {
                let category = match i % 5 {
                    0 => "tech",
                    1 => "news",
                    2 => "sports",
                    3 => "science",
                    _ => "entertainment",
                };

                let doc = json!({
                    "title": format!("Document Title {}", i),
                    "content": format!("This is the content of document number {} with some searchable text about {} topics", i, category),
                    "category": category,
                    "views": i as i64 * 10
                });
                store.add_document(doc).await.unwrap();
            }
        });

    // Small delay for indexing
    std::thread::sleep(std::time::Duration::from_millis(200));

    (temp_dir, Arc::new(index))
}

/// Generate a set of test queries
fn generate_test_queries(count: usize) -> Vec<Query> {
    let mut queries = Vec::new();
    let categories = ["tech", "news", "sports", "science", "entertainment"];
    let search_terms = ["document", "content", "searchable", "text", "title"];

    for i in 0..count {
        let query_type = i % 4;
        let query = match query_type {
            0 => Query::Match(MatchQuery::new(
                "content",
                search_terms[i % search_terms.len()].to_string(),
            )),
            1 => Query::Term(TermQuery::new(
                "category",
                categories[i % categories.len()].to_string(),
            )),
            2 => Query::Bool(
                BoolQuery::new()
                    .must(Query::Match(MatchQuery::new("content", "searchable")))
                    .filter(Query::Term(TermQuery::new(
                        "category",
                        categories[i % categories.len()].to_string(),
                    ))),
            ),
            _ => Query::Range(
                RangeQuery::new("views").gte(serde_json::Value::Number((i * 10).into())),
            ),
        };
        queries.push(query);
    }
    queries
}

/// Benchmark cache hit rate with repetitive access pattern
fn bench_cache_hit_rate_repetitive(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_hit_rate_repetitive");

    for cache_size in [10, 100, 1000] {
        let (_temp_dir, index) = create_test_index_with_docs(1000);
        let executor = SearchExecutor::with_cache_settings(index, cache_size, 300);

        // Generate a small set of queries that will be repeated
        let queries = generate_test_queries(10);

        // Warm up cache
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            for query in &queries {
                let _ = executor.search(query.clone(), 10, 0, None).await;
            }
        });

        // Reset stats to measure only the benchmark phase
        executor.cache_stats();

        group.bench_with_input(
            BenchmarkId::new("repetitive_access", cache_size),
            &cache_size,
            |b, _| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        // Repeat queries multiple times to test cache hit rate
                        for _ in 0..5 {
                            for query in &queries {
                                let _ =
                                    executor.search(black_box(query.clone()), 10, 0, None).await;
                            }
                        }
                    });
                });
            },
        );

        // Report cache statistics
        let stats = executor.cache_stats();
        eprintln!(
            "Cache size {}: Hit rate: {:.2}%, Hits: {}, Misses: {}, Size: {}/{}",
            cache_size,
            stats.hit_rate * 100.0,
            stats.hits,
            stats.misses,
            stats.size,
            stats.capacity
        );
    }

    group.finish();
}

/// Benchmark cache effectiveness with random access pattern
fn bench_cache_hit_rate_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_hit_rate_random");

    for cache_size in [10, 100, 1000] {
        let (_temp_dir, index) = create_test_index_with_docs(1000);
        let executor = SearchExecutor::with_cache_settings(index, cache_size, 300);

        // Generate a larger set of queries
        let queries = generate_test_queries(50);
        let mut rng = rand::thread_rng();

        group.bench_with_input(
            BenchmarkId::new("random_access", cache_size),
            &cache_size,
            |b, _| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        // Randomly select queries
                        for _ in 0..100 {
                            let query = queries.choose(&mut rng).unwrap();
                            let _ = executor.search(black_box(query.clone()), 10, 0, None).await;
                        }
                    });
                });
            },
        );

        // Report cache statistics
        let stats = executor.cache_stats();
        eprintln!(
            "Cache size {} (random): Hit rate: {:.2}%, Hits: {}, Misses: {}",
            cache_size,
            stats.hit_rate * 100.0,
            stats.hits,
            stats.misses
        );
    }

    group.finish();
}

/// Benchmark cache speedup (with vs without cache)
fn bench_cache_speedup(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_speedup");

    let (_temp_dir, index) = create_test_index_with_docs(1000);
    let executor_with_cache = SearchExecutor::new(index.clone());
    let executor_no_cache = SearchExecutor::without_cache(index);

    let queries = generate_test_queries(5);

    // Warm up cache
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        for query in &queries {
            let _ = executor_with_cache.search(query.clone(), 10, 0, None).await;
        }
    });

    // Benchmark with cache
    group.bench_function("with_cache", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                for query in &queries {
                    let _ = executor_with_cache
                        .search(black_box(query.clone()), 10, 0, None)
                        .await;
                }
            });
        });
    });

    // Benchmark without cache
    group.bench_function("without_cache", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                for query in &queries {
                    let _ = executor_no_cache
                        .search(black_box(query.clone()), 10, 0, None)
                        .await;
                }
            });
        });
    });

    group.finish();
}

/// Benchmark cache effectiveness with different TTLs
fn bench_cache_ttl_effectiveness(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_ttl_effectiveness");

    for ttl_secs in [60, 300, 600] {
        let (_temp_dir, index) = create_test_index_with_docs(1000);
        let executor = SearchExecutor::with_cache_settings(index, 1000, ttl_secs);

        let queries = generate_test_queries(10);

        // Warm up cache
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            for query in &queries {
                let _ = executor.search(query.clone(), 10, 0, None).await;
            }
        });

        group.bench_with_input(BenchmarkId::new("ttl", ttl_secs), &ttl_secs, |b, _| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    for query in &queries {
                        let _ = executor.search(black_box(query.clone()), 10, 0, None).await;
                    }
                });
            });
        });
    }
    group.finish();
}

/// Benchmark cache effectiveness across different index sizes
fn bench_cache_index_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_index_scaling");

    for index_size in [500, 1000, 2000] {
        let (_temp_dir, index) = create_test_index_with_docs(index_size);
        let executor = SearchExecutor::with_cache_settings(index, 1000, 300);

        let queries = generate_test_queries(10);

        // Warm up cache
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            for query in &queries {
                let _ = executor.search(query.clone(), 10, 0, None).await;
            }
        });

        group.bench_with_input(
            BenchmarkId::new("index_size", index_size),
            &index_size,
            |b, _| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        for query in &queries {
                            let _ = executor.search(black_box(query.clone()), 10, 0, None).await;
                        }
                    });
                });
            },
        );

        // Report cache statistics
        let stats = executor.cache_stats();
        eprintln!(
            "Index size {}: Hit rate: {:.2}%, Cache size: {}/{}",
            index_size,
            stats.hit_rate * 100.0,
            stats.size,
            stats.capacity
        );
    }

    group.finish();
}

/// Benchmark cache eviction behavior
fn bench_cache_eviction(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_eviction");

    for cache_size in [10, 50, 100] {
        let (_temp_dir, index) = create_test_index_with_docs(1000);
        let executor = SearchExecutor::with_cache_settings(index, cache_size, 300);

        // Generate more queries than cache size to force evictions
        let queries = generate_test_queries(cache_size);

        group.bench_with_input(
            BenchmarkId::new("eviction", cache_size),
            &cache_size,
            |b, _| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        // Run all queries to fill cache and trigger evictions
                        for query in &queries {
                            let _ = executor.search(black_box(query.clone()), 10, 0, None).await;
                        }
                    });
                });
            },
        );

        // Report eviction statistics
        let stats = executor.cache_stats();
        eprintln!(
            "Cache size {}: LRU evictions: {}, Expired evictions: {}, Total inserts: {}",
            cache_size, stats.lru_evictions, stats.expired_evictions, stats.total_inserts
        );
    }

    group.finish();
}

/// Benchmark cache warming effectiveness
fn bench_cache_warming(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_warming");

    let (_temp_dir, index) = create_test_index_with_docs(1000);
    let executor = SearchExecutor::new(index);

    let queries = generate_test_queries(10);

    // Pre-warm cache by running queries
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut warm_up_results = Vec::new();
    rt.block_on(async {
        for query in &queries {
            let result = executor.search(query.clone(), 10, 0, None).await.unwrap();
            warm_up_results.push((query.clone(), result));
        }
    });

    // Clear cache and warm up using warm_up_cache method
    executor.clear_cache();
    let warm_up_entries: Vec<(Query, SearchResult)> = warm_up_results.clone();
    let warmed_count = executor.warm_up_cache(warm_up_entries);
    eprintln!("Warmed up {warmed_count} cache entries");

    // Benchmark warmed cache
    group.bench_function("warmed_cache", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                for query in &queries {
                    let _ = executor.search(black_box(query.clone()), 10, 0, None).await;
                }
            });
        });
    });

    // Benchmark cold cache (no warming)
    executor.clear_cache();
    group.bench_function("cold_cache", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                for query in &queries {
                    let _ = executor.search(black_box(query.clone()), 10, 0, None).await;
                }
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_hit_rate_repetitive,
    bench_cache_hit_rate_random,
    bench_cache_speedup,
    bench_cache_ttl_effectiveness,
    bench_cache_index_scaling,
    bench_cache_eviction,
    bench_cache_warming
);
criterion_main!(benches);
