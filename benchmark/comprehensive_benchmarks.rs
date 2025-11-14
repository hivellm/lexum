//! Comprehensive performance benchmarking suite for Lexum
//! 
//! This benchmark suite tests various aspects of Lexum performance:
//! - Query performance across different query types
//! - Indexing performance with different document sizes
//! - Memory usage and resource consumption
//! - Concurrent operations
//! - Cache performance
//! - Search optimization effectiveness

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput, PlotConfiguration, AxisScale};
use lexum_core::config::ServerConfig;
use lexum_core::index::manager::IndexManager;
use lexum_core::schema::{FieldDef, FieldType, SchemaBuilder};
use lexum_core::search::executor::SearchExecutor;
use lexum_core::search::optimizer::{QueryOptimizer, QueryAnalysis};
use lexum_core::query::types::*;
use lexum_core::query::QueryBuilder;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark configuration
struct BenchmarkConfig {
    small_docs: usize,
    medium_docs: usize,
    large_docs: usize,
    huge_docs: usize,
    concurrent_queries: usize,
    cache_size_limit: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            small_docs: 1000,
            medium_docs: 5000,
            large_docs: 10000,
            huge_docs: 50000,
            concurrent_queries: 10,
            cache_size_limit: 1000,
        }
    }
}

/// Setup test index with specified document count and size
async fn setup_test_index(
    manager: &IndexManager,
    doc_count: usize,
    doc_size: DocSize,
) -> (Arc<lexum_core::index::Index>, String) {
    let index_name = format!(
        "bench_{}_{}_{}",
        doc_size.as_str(),
        doc_count,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    let (schema, _) = SchemaBuilder::new()
        .add_text_field("title")
        .add_text_field("content")
        .add_text_field("description")
        .add_string_field("category")
        .add_string_field("status")
        .add_i64_field("score")
        .add_f64_field("rating")
        .add_date_field("created_at")
        .build()
        .unwrap();

    let index = manager.create_index(&index_name, schema, Default::default()).await.unwrap();

    // Generate documents based on size
    let documents = generate_documents(doc_count, doc_size);
    
    // Index documents in batches for better performance
    const BATCH_SIZE: usize = 100;
    for chunk in documents.chunks(BATCH_SIZE) {
        for doc in chunk {
            manager.add_document(&index_name, doc.clone()).await.unwrap();
        }
    }

    manager.commit(&index_name).await.unwrap();
    (index, index_name)
}

#[derive(Clone, Copy)]
enum DocSize {
    Small,
    Medium,
    Large,
    Huge,
}

impl DocSize {
    fn as_str(&self) -> &'static str {
        match self {
            DocSize::Small => "small",
            DocSize::Medium => "medium",
            DocSize::Large => "large",
            DocSize::Huge => "huge",
        }
    }

    fn content_length(&self) -> usize {
        match self {
            DocSize::Small => 100,
            DocSize::Medium => 500,
            DocSize::Large => 2000,
            DocSize::Huge => 10000,
        }
    }
}

fn generate_documents(count: usize, size: DocSize) -> Vec<serde_json::Value> {
    let content_length = size.content_length();
    let mut documents = Vec::with_capacity(count);
    
    for i in 0..count {
        let content = generate_text_content(content_length, i);
        let doc = json!({
            "title": format!("Document {} - {}", i, generate_title(i)),
            "content": content,
            "description": format!("Description for document {} with some additional details", i),
            "category": format!("category_{}", i % 20),
            "status": if i % 3 == 0 { "active" } else { "inactive" },
            "score": i as i64,
            "rating": (i as f64) / 100.0,
            "created_at": chrono::Utc::now().to_rfc3339(),
        });
        documents.push(doc);
    }
    
    documents
}

fn generate_text_content(length: usize, seed: usize) -> String {
    let words = [
        "search", "engine", "performance", "benchmark", "testing", "optimization",
        "query", "index", "document", "content", "analysis", "results",
        "fast", "efficient", "scalable", "distributed", "concurrent", "parallel",
        "memory", "cpu", "disk", "network", "latency", "throughput", "bandwidth",
        "algorithm", "data", "structure", "database", "storage", "retrieval",
        "ranking", "scoring", "relevance", "precision", "recall", "accuracy",
        "machine", "learning", "artificial", "intelligence", "natural", "language",
        "processing", "text", "analysis", "tokenization", "stemming", "lemmatization",
        "clustering", "classification", "categorization", "filtering", "faceting",
        "aggregation", "statistics", "analytics", "visualization", "dashboard",
        "monitoring", "logging", "debugging", "profiling", "tracing", "metrics",
        "observability", "telemetry", "alerting", "notification", "reporting",
        "api", "rest", "graphql", "websocket", "http", "https", "tcp", "udp",
        "json", "xml", "yaml", "csv", "binary", "serialization", "deserialization",
        "compression", "encryption", "security", "authentication", "authorization",
        "permission", "access", "control", "privacy", "compliance", "audit",
        "backup", "recovery", "replication", "consistency", "availability",
        "partitioning", "sharding", "load", "balancing", "failover", "redundancy",
    ];
    
    let mut content = String::with_capacity(length);
    let mut current_length = 0;
    let mut word_index = seed;
    
    while current_length < length {
        let word = words[word_index % words.len()];
        if current_length > 0 {
            content.push(' ');
            current_length += 1;
        }
        content.push_str(word);
        current_length += word.len();
        word_index = word_index.wrapping_mul(1103515245).wrapping_add(12345);
    }
    
    content.truncate(length);
    content
}

fn generate_title(seed: usize) -> String {
    let adjectives = ["Amazing", "Incredible", "Fantastic", "Outstanding", "Remarkable"];
    let nouns = ["Search", "Engine", "System", "Platform", "Solution"];
    
    let adj = adjectives[seed % adjectives.len()];
    let noun = nouns[(seed / adjectives.len()) % nouns.len()];
    
    format!("{} {}", adj, noun)
}

/// Benchmark different query types
fn bench_query_types(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = ServerConfig::default();
    let manager = IndexManager::new(&config.data_dir).unwrap();
    
    rt.block_on(async {
        let (index, index_name) = setup_test_index(&manager, 10000, DocSize::Medium).await;
        let executor = SearchExecutor::new(index);
        
        let mut group = c.benchmark_group("query_types");
        group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
        
        // Match query
        group.bench_function("match_query", |b| {
            let query = Query::Match(MatchQuery::new("content", "search engine performance"));
            b.iter(|| {
                rt.block_on(executor.search(black_box(query.clone()), 10, 0, None)).unwrap()
            })
        });
        
        // Term query
        group.bench_function("term_query", |b| {
            let query = Query::Term(TermQuery::new("category", "category_5"));
            b.iter(|| {
                rt.block_on(executor.search(black_box(query.clone()), 10, 0, None)).unwrap()
            })
        });
        
        // Range query
        group.bench_function("range_query", |b| {
            let query = Query::Range(RangeQuery::new("score")
                .gte(json!(100))
                .lte(json!(500)));
            b.iter(|| {
                rt.block_on(executor.search(black_box(query.clone()), 10, 0, None)).unwrap()
            })
        });
        
        // Boolean query
        group.bench_function("bool_query", |b| {
            let query = Query::Bool(BoolQuery::new()
                .must(Query::Match(MatchQuery::new("content", "search")))
                .should(Query::Term(TermQuery::new("status", "active")))
                .must_not(Query::Term(TermQuery::new("category", "category_0"))));
            b.iter(|| {
                rt.block_on(executor.search(black_box(query.clone()), 10, 0, None)).unwrap()
            })
        });
        
        // Fuzzy query
        group.bench_function("fuzzy_query", |b| {
            let query = Query::Fuzzy(FuzzyQuery::new("content", "search engin")
                .fuzziness(2));
            b.iter(|| {
                rt.block_on(executor.search(black_box(query.clone()), 10, 0, None)).unwrap()
            })
        });
        
        // Phrase query
        group.bench_function("phrase_query", |b| {
            let query = Query::Phrase(PhraseQuery::new("content", "search engine performance"));
            b.iter(|| {
                rt.block_on(executor.search(black_box(query.clone()), 10, 0, None)).unwrap()
            })
        });
        
        // Regex query
        group.bench_function("regex_query", |b| {
            let query = Query::Regex(RegexQuery::new("content", "search.*engine")
                .case_sensitive(false));
            b.iter(|| {
                rt.block_on(executor.search(black_box(query.clone()), 10, 0, None)).unwrap()
            })
        });
        
        // Wildcard query
        group.bench_function("wildcard_query", |b| {
            let query = Query::Wildcard(WildcardQuery::new("title", "Document*"));
            b.iter(|| {
                rt.block_on(executor.search(black_box(query.clone()), 10, 0, None)).unwrap()
            })
        });
        
        group.finish();
        
        let _ = manager.delete_index(&index_name).await;
    });
}

/// Benchmark indexing performance with different document sizes
fn bench_indexing_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = ServerConfig::default();
    let manager = IndexManager::new(&config.data_dir).unwrap();
    
    let mut group = c.benchmark_group("indexing_performance");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
    
    for doc_size in [DocSize::Small, DocSize::Medium, DocSize::Large, DocSize::Huge] {
        for doc_count in [100, 1000, 5000, 10000] {
            group.throughput(Throughput::Elements(doc_count as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("indexing_{}", doc_size.as_str()), doc_count),
                &doc_count,
                |b, &doc_count| {
                    b.iter(|| {
                        rt.block_on(async {
                            let (_, index_name) = setup_test_index(&manager, doc_count, doc_size).await;
                            let _ = manager.delete_index(&index_name).await;
                        })
                    })
                },
            );
        }
    }
    
    group.finish();
}

/// Benchmark search performance with different result limits
fn bench_search_limits(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = ServerConfig::default();
    let manager = IndexManager::new(&config.data_dir).unwrap();
    
    rt.block_on(async {
        let (index, index_name) = setup_test_index(&manager, 20000, DocSize::Medium).await;
        let executor = SearchExecutor::new(index);
        
        let query = Query::Match(MatchQuery::new("content", "search engine"));
        
        let mut group = c.benchmark_group("search_limits");
        group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
        
        for limit in [1, 10, 50, 100, 500, 1000] {
            group.throughput(Throughput::Elements(limit as u64));
            group.bench_with_input(
                BenchmarkId::from_parameter(limit),
                &limit,
                |b, &limit| {
                    b.iter(|| {
                        rt.block_on(executor.search(black_box(query.clone()), limit, 0, None)).unwrap()
                    })
                },
            );
        }
        
        group.finish();
        let _ = manager.delete_index(&index_name).await;
    });
}

/// Benchmark query optimization effectiveness
fn bench_query_optimization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = ServerConfig::default();
    let manager = IndexManager::new(&config.data_dir).unwrap();
    
    rt.block_on(async {
        let (index, index_name) = setup_test_index(&manager, 15000, DocSize::Medium).await;
        let executor = SearchExecutor::new(index);
        let optimizer = QueryOptimizer::new();
        
        // Complex query that should benefit from optimization
        let complex_query = Query::Bool(BoolQuery::new()
            .must(Query::Match(MatchQuery::new("content", "search")))
            .must(Query::Bool(BoolQuery::new()
                .should(Query::Term(TermQuery::new("status", "active")))
                .should(Query::Term(TermQuery::new("status", "pending")))))
            .must_not(Query::Term(TermQuery::new("category", "category_0")))
            .filter(Query::Range(RangeQuery::new("score").gte(json!(0)))));

        let mut group = c.benchmark_group("query_optimization");
        
        // Unoptimized query
        group.bench_function("unoptimized", |b| {
            b.iter(|| {
                rt.block_on(executor.search(black_box(complex_query.clone()), 10, 0, None)).unwrap()
            })
        });
        
        // Optimized query
        group.bench_function("optimized", |b| {
            let optimized_query = optimizer.optimize(complex_query.clone()).unwrap();
            b.iter(|| {
                rt.block_on(executor.search(black_box(optimized_query.clone()), 10, 0, None)).unwrap()
            })
        });
        
        group.finish();
        let _ = manager.delete_index(&index_name).await;
    });
}

/// Benchmark cache performance
fn bench_cache_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = ServerConfig::default();
    let manager = IndexManager::new(&config.data_dir).unwrap();
    
    rt.block_on(async {
        let (index, index_name) = setup_test_index(&manager, 10000, DocSize::Medium).await;
        let executor = SearchExecutor::new(index);
        
        let queries = vec![
            Query::Match(MatchQuery::new("content", "search engine")),
            Query::Match(MatchQuery::new("content", "performance testing")),
            Query::Term(TermQuery::new("status", "active")),
            Query::Bool(BoolQuery::new()
                .must(Query::Match(MatchQuery::new("content", "benchmark")))
                .should(Query::Term(TermQuery::new("category", "category_1")))),
        ];
        
        let mut group = c.benchmark_group("cache_performance");
        
        // Cache miss (cold cache)
        group.bench_function("cache_miss", |b| {
            b.iter(|| {
                executor.clear_cache();
                for query in &queries {
                    rt.block_on(executor.search(black_box(query.clone()), 10, 0, None)).unwrap();
                }
            })
        });
        
        // Cache hit (warm cache)
        group.bench_function("cache_hit", |b| {
            // Warm up cache
            for query in &queries {
                rt.block_on(executor.search(query.clone(), 10, 0, None)).await.unwrap();
            }
            
            b.iter(|| {
                for query in &queries {
                    rt.block_on(executor.search(black_box(query.clone()), 10, 0, None)).unwrap();
                }
            })
        });
        
        group.finish();
        let _ = manager.delete_index(&index_name).await;
    });
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = ServerConfig::default();
    let manager = IndexManager::new(&config.data_dir).unwrap();
    
    rt.block_on(async {
        let (index, index_name) = setup_test_index(&manager, 20000, DocSize::Medium).await;
        let executor = SearchExecutor::new(index);
        
        let queries = vec![
            Query::Match(MatchQuery::new("content", "search")),
            Query::Match(MatchQuery::new("content", "engine")),
            Query::Match(MatchQuery::new("content", "performance")),
            Query::Term(TermQuery::new("status", "active")),
            Query::Term(TermQuery::new("category", "category_1")),
        ];
        
        let mut group = c.benchmark_group("concurrent_operations");
        
        for concurrency in [1, 2, 4, 8, 16] {
            group.bench_with_input(
                BenchmarkId::from_parameter(concurrency),
                &concurrency,
                |b, &concurrency| {
                    b.iter(|| {
                        let handles: Vec<_> = (0..concurrency)
                            .map(|i| {
                                let executor = executor.clone();
                                let query = queries[i % queries.len()].clone();
                                tokio::spawn(async move {
                                    executor.search(query, 10, 0, None).await
                                })
                            })
                            .collect();
                        
                        for handle in handles {
                            rt.block_on(handle).unwrap().unwrap();
                        }
                    })
                },
            );
        }
        
        group.finish();
        let _ = manager.delete_index(&index_name).await;
    });
}

/// Benchmark memory usage
fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = ServerConfig::default();
    let manager = IndexManager::new(&config.data_dir).unwrap();
    
    let mut group = c.benchmark_group("memory_usage");
    
    for doc_count in [1000, 5000, 10000, 20000, 50000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(doc_count),
            &doc_count,
            |b, &doc_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let (_, index_name) = setup_test_index(&manager, doc_count, DocSize::Medium).await;
                        let _ = manager.delete_index(&index_name).await;
                    })
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark query analysis performance
fn bench_query_analysis(c: &mut Criterion) {
    let optimizer = QueryOptimizer::new();
    
    let complex_queries = vec![
        Query::Bool(BoolQuery::new()
            .must(Query::Match(MatchQuery::new("content", "search")))
            .should(Query::Term(TermQuery::new("status", "active")))
            .must_not(Query::Term(TermQuery::new("category", "category_0")))),
        Query::Bool(BoolQuery::new()
            .must(Query::Bool(BoolQuery::new()
                .must(Query::Match(MatchQuery::new("title", "test")))
                .should(Query::Fuzzy(FuzzyQuery::new("content", "search")))
                .must_not(Query::Regex(RegexQuery::new("description", ".*spam.*")))))),
        Query::FunctionScore(FunctionScoreQuery::new(
            Query::Match(MatchQuery::new("content", "performance"))
        )),
    ];
    
    let mut group = c.benchmark_group("query_analysis");
    
    group.bench_function("analyze_complex_queries", |b| {
        b.iter(|| {
            for query in &complex_queries {
                let _analysis = optimizer.analyze(black_box(query));
            }
        })
    });
    
    group.bench_function("optimize_complex_queries", |b| {
        b.iter(|| {
            for query in &complex_queries {
                let _optimized = optimizer.optimize(black_box(query.clone())).unwrap();
            }
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_query_types,
    bench_indexing_performance,
    bench_search_limits,
    bench_query_optimization,
    bench_cache_performance,
    bench_concurrent_operations,
    bench_memory_usage,
    bench_query_analysis
);

criterion_main!(benches);