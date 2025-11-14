//! Concurrency performance benchmarks
//!
//! This benchmark suite measures the performance of concurrent operations,
//! including thread pool efficiency, work stealing, and lock-free data structures.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lexum_core::concurrency::{LockFreeCache, ThreadPoolConfig, WorkStealingQueue};
use lexum_core::{IndexManager, SearchExecutor};
use lexum_core::{IndexSettings, QueryBuilder, SchemaBuilder};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::task;

fn create_test_index_with_docs(num_docs: usize) -> (TempDir, Arc<lexum_core::Index>) {
    let temp_dir = TempDir::new().unwrap();
    let manager = IndexManager::new(temp_dir.path());

    let (schema, _) = SchemaBuilder::new()
        .add_text_field("title")
        .add_text_field("content")
        .add_keyword_field("category")
        .build()
        .unwrap();

    let index = tokio::runtime::Runtime::new().unwrap().block_on(async {
        manager
            .create_index("bench_index", schema, IndexSettings::default())
            .await
            .unwrap()
    });

    let store = lexum_core::DocumentStore::new(Arc::new(index.clone()));

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            for i in 0..num_docs {
                let doc = serde_json::json!({
                    "title": format!("Document Title {}", i),
                    "content": format!("This is the content of document number {} with some searchable text", i),
                    "category": if i % 2 == 0 { "tech" } else { "news" }
                });
                store.add_document(doc).await.unwrap();
            }
        });

    std::thread::sleep(std::time::Duration::from_millis(200));

    (temp_dir, Arc::new(index))
}

/// Benchmark: Concurrent search operations
fn bench_concurrent_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_search");

    let (_temp_dir, index) = create_test_index_with_docs(5000);
    let executor = Arc::new(SearchExecutor::new(index));

    let queries = [
        QueryBuilder::match_query("content", "searchable"),
        QueryBuilder::match_query("title", "Document"),
        QueryBuilder::term_query("category", "tech"),
    ];

    for concurrency in [1, 4, 8, 16, 32] {
        group.bench_with_input(
            BenchmarkId::new("concurrent", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let mut handles = Vec::new();
                        for i in 0..concurrency {
                            let executor = executor.clone();
                            let query = queries[i % queries.len()].clone();
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

/// Benchmark: Work stealing queue performance
fn bench_work_stealing(c: &mut Criterion) {
    let mut group = c.benchmark_group("work_stealing");

    for num_workers in [2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("push_pop", num_workers),
            &num_workers,
            |b, &num_workers| {
                let queue = WorkStealingQueue::new(num_workers);
                b.iter(|| {
                    // Push tasks
                    for i in 0..100 {
                        queue.push(black_box(i));
                    }
                    // Pop tasks from worker 0
                    for _ in 0..100 {
                        let _ = queue.pop(0);
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Lock-free cache performance
fn bench_lock_free_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("lock_free_cache");

    let cache = Arc::new(LockFreeCache::new(Duration::from_secs(60)));

    // Warm up
    for i in 0..1000 {
        cache.insert(i, format!("value_{i}"));
    }

    group.bench_function("get", |b| {
        let cache = Arc::clone(&cache);
        b.iter(|| {
            for i in 0..100 {
                let _ = cache.get(&black_box(i % 1000));
            }
        });
    });

    group.bench_function("insert", |b| {
        let cache = Arc::clone(&cache);
        b.iter(|| {
            for i in 0..100 {
                cache.insert(black_box(i + 10000), format!("value_{i}"));
            }
        });
    });

    group.bench_function("concurrent_get", |b| {
        let cache = Arc::clone(&cache);
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut handles = Vec::new();
                for _ in 0..10 {
                    let cache = Arc::clone(&cache);
                    handles.push(task::spawn(async move {
                        for i in 0..10 {
                            let _ = cache.get(&black_box(i % 1000));
                        }
                    }));
                }
                for handle in handles {
                    let _ = handle.await;
                }
            });
        });
    });

    group.finish();
}

/// Benchmark: Thread pool configuration impact
fn bench_thread_pool_config(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_pool_config");

    let configs = vec![
        ("default", ThreadPoolConfig::default()),
        ("cpu_bound", ThreadPoolConfig::for_cpu_bound()),
        ("io_bound", ThreadPoolConfig::for_io_bound()),
        ("mixed", ThreadPoolConfig::for_mixed()),
    ];

    for (name, config) in configs {
        group.bench_function(name, |b| {
            b.iter(|| {
                let _cpu = black_box(config.cpu_threads);
                let _io = black_box(config.io_threads);
                let optimal = ThreadPoolConfig::calculate_optimal_threads(0.5, 1, 100);
                black_box(optimal);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_concurrent_search,
    bench_work_stealing,
    bench_lock_free_cache,
    bench_thread_pool_config
);
criterion_main!(benches);
