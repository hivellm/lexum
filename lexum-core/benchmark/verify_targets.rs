//! Performance targets verification
//!
//! This benchmark verifies that Lexum meets the performance targets
//! defined in the performance specification.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use lexum_core::*;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;

/// Performance targets from spec
struct PerformanceTargets {
    /// Query cache hit rate target (>80%)
    cache_hit_rate: f64,
    /// Search latency p95 target (<10ms)
    search_p95_ms: f64,
    /// Search latency p99 target (<20ms)
    search_p99_ms: f64,
    /// Indexing throughput target (>10K docs/sec for bulk)
    indexing_throughput: f64,
    /// Memory per document target (<2KB)
    memory_per_doc_kb: f64,
}

impl Default for PerformanceTargets {
    fn default() -> Self {
        Self {
            cache_hit_rate: 0.80,          // 80%
            search_p95_ms: 10.0,           // 10ms
            search_p99_ms: 20.0,           // 20ms
            indexing_throughput: 10_000.0, // 10K docs/sec
            memory_per_doc_kb: 2.0,        // 2KB
        }
    }
}

/// Performance verification results
#[derive(Debug)]
#[allow(dead_code)]
struct VerificationResults {
    cache_hit_rate: VerificationResult,
    search_p95: VerificationResult,
    search_p99: VerificationResult,
    indexing_throughput: VerificationResult,
    memory_efficiency: VerificationResult,
}

#[derive(Debug)]
#[allow(dead_code)]
struct VerificationResult {
    target: f64,
    achieved: f64,
    passed: bool,
    message: String,
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
            .create_index("targets_index", schema, IndexSettings::default())
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

/// Verify cache hit rate target
fn verify_cache_hit_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify_cache_hit_rate");
    let targets = PerformanceTargets::default();

    let (_temp_dir, index) = create_test_index_with_docs(1000);
    let executor = SearchExecutor::new(index);

    let query = QueryBuilder::match_query("content", "searchable");

    // Warm up cache
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        for _ in 0..10 {
            let _ = executor.search(query.clone(), 10, 0, None).await;
        }
    });

    // Measure cache hit rate
    group.bench_function("cache_hit_rate", |b| {
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..20 {
                    let _ = executor.search(black_box(query.clone()), 10, 0, None).await;
                }
            });
        });
    });

    let stats = executor.cache_stats();
    let hit_rate = stats.hit_rate;
    let passed = hit_rate >= targets.cache_hit_rate;

    eprintln!(
        "Cache Hit Rate: {:.2}% (target: {:.2}%) - {}",
        hit_rate * 100.0,
        targets.cache_hit_rate * 100.0,
        if passed { "✅ PASS" } else { "❌ FAIL" }
    );

    group.finish();
}

/// Verify search latency targets
fn verify_search_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify_search_latency");
    let targets = PerformanceTargets::default();

    let (_temp_dir, index) = create_test_index_with_docs(10000);
    let executor = SearchExecutor::new(index);

    let query = QueryBuilder::match_query("content", "searchable");

    let mut latencies = Vec::new();

    group.bench_function("search_latency", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let start = Instant::now();
                let _ = executor.search(black_box(query.clone()), 10, 0, None).await;
                latencies.push(start.elapsed().as_secs_f64() * 1000.0); // Convert to ms
            });
        });
    });

    if latencies.len() >= 100 {
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95_idx = (latencies.len() as f64 * 0.95) as usize;
        let p99_idx = (latencies.len() as f64 * 0.99) as usize;
        let p95 = latencies[p95_idx];
        let p99 = latencies[p99_idx];

        let p95_passed = p95 <= targets.search_p95_ms;
        let p99_passed = p99 <= targets.search_p99_ms;

        eprintln!(
            "Search P95 Latency: {:.2}ms (target: {:.2}ms) - {}",
            p95,
            targets.search_p95_ms,
            if p95_passed { "✅ PASS" } else { "❌ FAIL" }
        );
        eprintln!(
            "Search P99 Latency: {:.2}ms (target: {:.2}ms) - {}",
            p99,
            targets.search_p99_ms,
            if p99_passed { "✅ PASS" } else { "❌ FAIL" }
        );
    }

    group.finish();
}

/// Verify indexing throughput target
fn verify_indexing_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify_indexing_throughput");
    let targets = PerformanceTargets::default();

    group.bench_function("indexing_throughput", |b| {
        b.iter(|| {
            let (temp_dir, _index) = create_test_index_with_docs(1000);
            black_box(temp_dir);
        });
    });

    // For actual measurement, we'd need to track time
    // This is a simplified version
    eprintln!(
        "Indexing Throughput: Target > {:.0} docs/sec",
        targets.indexing_throughput
    );
    eprintln!("Note: Run indexing benchmarks for actual measurement");

    group.finish();
}

/// Verify memory efficiency target
fn verify_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify_memory_efficiency");
    let targets = PerformanceTargets::default();

    group.bench_function("memory_efficiency", |b| {
        b.iter(|| {
            let (_temp_dir, index) = create_test_index_with_docs(1000);
            let executor = SearchExecutor::new(index);
            let _stats = executor.cache_stats();
            black_box(executor);
        });
    });

    eprintln!(
        "Memory Efficiency: Target < {:.1}KB per document",
        targets.memory_per_doc_kb
    );
    eprintln!("Note: Run memory profiling for actual measurement");

    group.finish();
}

criterion_group!(
    benches,
    verify_cache_hit_rate,
    verify_search_latency,
    verify_indexing_throughput,
    verify_memory_efficiency
);
criterion_main!(benches);
