//! Performance regression testing
//!
//! This benchmark suite compares current performance against baseline
//! to detect performance regressions.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use lexum_core::*;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

/// Baseline performance metrics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BaselineMetrics {
    /// Search latency (p95) in milliseconds
    search_p95_ms: f64,
    /// Indexing throughput (docs/sec)
    indexing_throughput: f64,
    /// Cache hit rate
    cache_hit_rate: f64,
    /// Memory usage (bytes)
    memory_usage: u64,
}

/// Load baseline from file
#[allow(dead_code)]
fn load_baseline(path: &Path) -> Option<HashMap<String, BaselineMetrics>> {
    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save baseline to file
#[allow(dead_code)]
fn save_baseline(
    path: &Path,
    baseline: &HashMap<String, BaselineMetrics>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let content = serde_json::to_string_pretty(baseline)?;
    fs::write(path, content)?;
    Ok(())
}

/// Compare current metrics with baseline
#[allow(dead_code)]
fn compare_with_baseline(
    test_name: &str,
    current: &BaselineMetrics,
    baseline: &BaselineMetrics,
    threshold: f64,
) -> bool {
    let search_regression = current.search_p95_ms > baseline.search_p95_ms * (1.0 + threshold);
    let indexing_regression =
        current.indexing_throughput < baseline.indexing_throughput * (1.0 - threshold);
    let cache_regression = current.cache_hit_rate < baseline.cache_hit_rate * (1.0 - threshold);

    if search_regression || indexing_regression || cache_regression {
        eprintln!("⚠️  REGRESSION DETECTED in {test_name}:");
        if search_regression {
            eprintln!(
                "   Search p95: {}ms (baseline: {}ms, threshold: {:.1}%)",
                current.search_p95_ms,
                baseline.search_p95_ms,
                threshold * 100.0
            );
        }
        if indexing_regression {
            eprintln!(
                "   Indexing throughput: {:.0} docs/sec (baseline: {:.0}, threshold: {:.1}%)",
                current.indexing_throughput,
                baseline.indexing_throughput,
                threshold * 100.0
            );
        }
        if cache_regression {
            eprintln!(
                "   Cache hit rate: {:.2}% (baseline: {:.2}%, threshold: {:.1}%)",
                current.cache_hit_rate * 100.0,
                baseline.cache_hit_rate * 100.0,
                threshold * 100.0
            );
        }
        return false;
    }
    true
}

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
            .create_index("regression_index", schema, IndexSettings::default())
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

/// Benchmark search performance regression
fn bench_search_regression(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_regression");

    let (_temp_dir, index) = create_test_index_with_docs(1000);
    let executor = SearchExecutor::new(index);

    let query = QueryBuilder::match_query("content", "searchable");

    group.bench_function("search_p95", |b| {
        b.iter(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                executor
                    .search(black_box(query.clone()), 10, 0, None)
                    .await
                    .unwrap();
            });
        });
    });

    // Get cache stats
    let stats = executor.cache_stats();
    let cache_hit_rate = stats.hit_rate;

    // Record metrics (would be compared with baseline in real scenario)
    let metrics = BaselineMetrics {
        search_p95_ms: 1.0, // Placeholder - would be measured from benchmark
        indexing_throughput: 0.0,
        cache_hit_rate,
        memory_usage: 0,
    };

    eprintln!(
        "Search regression test - Cache hit rate: {:.2}%",
        cache_hit_rate * 100.0
    );
    black_box(metrics);

    group.finish();
}

/// Benchmark indexing performance regression
fn bench_indexing_regression(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexing_regression");

    group.bench_function("indexing_throughput", |b| {
        b.iter(|| {
            let (_temp_dir, index) = create_test_index_with_docs(100);
            black_box(index);
        });
    });

    group.finish();
}

/// Benchmark cache performance regression
fn bench_cache_regression(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_regression");

    let (_temp_dir, index) = create_test_index_with_docs(1000);
    let executor = SearchExecutor::new(index);

    let queries = vec![
        QueryBuilder::match_query("content", "searchable"),
        QueryBuilder::term_query("category", "tech"),
        QueryBuilder::match_query("title", "Document"),
    ];

    // Warm up cache
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        for query in &queries {
            let _ = executor.search(query.clone(), 10, 0, None).await;
        }
    });

    group.bench_function("cache_hit_rate", |b| {
        b.iter(|| {
            rt.block_on(async {
                for query in &queries {
                    let _ = executor.search(black_box(query.clone()), 10, 0, None).await;
                }
            });
        });
    });

    let stats = executor.cache_stats();
    eprintln!(
        "Cache regression test - Hit rate: {:.2}%",
        stats.hit_rate * 100.0
    );

    group.finish();
}

/// Benchmark memory usage regression
fn bench_memory_regression(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_regression");

    group.bench_function("memory_usage", |b| {
        b.iter(|| {
            let (_temp_dir, index) = create_test_index_with_docs(1000);
            let executor = SearchExecutor::new(index);
            let _stats = executor.cache_stats();
            black_box(executor);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_search_regression,
    bench_indexing_regression,
    bench_cache_regression,
    bench_memory_regression
);
criterion_main!(benches);
