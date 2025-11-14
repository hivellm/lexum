//! Large-scale load testing for Lexum (1M+ documents)

use anyhow::Result;
use clap::{Arg, Command};
use lexum_core::{
    FieldConfig, FieldType, IndexManager, IndexSettings, Query, QueryBuilder, SchemaBuilder,
    SearchExecutor, document::DocumentStore,
};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sys_info;
use tokio::time::sleep;

/// Large-scale load test configuration
#[derive(Debug, Clone)]
struct ScaleLoadTestConfig {
    /// Number of documents to index
    pub document_count: usize,
    /// Number of concurrent indexing workers
    pub indexing_workers: usize,
    /// Batch size for bulk indexing
    pub batch_size: usize,
    /// Number of search queries to run
    pub search_queries: usize,
    /// Number of concurrent search workers
    pub search_workers: usize,
    /// Index name
    pub index_name: String,
    /// Data directory
    pub data_dir: String,
    /// Enable progress reporting
    pub progress_reporting: bool,
}

impl Default for ScaleLoadTestConfig {
    fn default() -> Self {
        Self {
            document_count: 1_000_000, // 1M documents
            indexing_workers: 10,
            batch_size: 1000,
            search_queries: 10_000,
            search_workers: 50,
            index_name: "scale_test_index".to_string(),
            data_dir: "./data/scale_test".to_string(),
            progress_reporting: true,
        }
    }
}

/// Scale load test results
#[derive(Debug, Clone)]
struct ScaleLoadTestResults {
    /// Indexing results
    pub indexing: IndexingResults,
    /// Search results
    pub search: SearchResults,
    /// Overall test duration
    pub total_duration_secs: f64,
}

/// Indexing results
#[derive(Debug, Clone)]
struct IndexingResults {
    /// Total documents indexed
    pub documents_indexed: usize,
    /// Total time taken
    pub duration_secs: f64,
    /// Documents per second
    pub docs_per_second: f64,
    /// Average time per document (ms)
    pub avg_time_per_doc_ms: f64,
    /// P95 time per document (ms)
    pub p95_time_per_doc_ms: f64,
    /// P99 time per document (ms)
    pub p99_time_per_doc_ms: f64,
    /// Memory usage peak (MB)
    pub peak_memory_mb: f64,
}

/// Search results
#[derive(Debug, Clone)]
struct SearchResults {
    /// Total queries executed
    pub queries_executed: usize,
    /// Successful queries
    pub successful_queries: usize,
    /// Failed queries
    pub failed_queries: usize,
    /// Total time taken
    pub duration_secs: f64,
    /// Queries per second
    pub queries_per_second: f64,
    /// Average response time (ms)
    pub avg_response_time_ms: f64,
    /// P95 response time (ms)
    pub p95_response_time_ms: f64,
    /// P99 response time (ms)
    pub p99_response_time_ms: f64,
}

/// Run large-scale load test
async fn run_scale_load_test(config: ScaleLoadTestConfig) -> Result<ScaleLoadTestResults> {
    println!("Starting large-scale load test:");
    println!("  Documents: {}", config.document_count);
    println!("  Indexing workers: {}", config.indexing_workers);
    println!("  Batch size: {}", config.batch_size);
    println!("  Search queries: {}", config.search_queries);
    println!("  Search workers: {}", config.search_workers);
    println!();

    let start_time = Instant::now();

    // Setup index
    println!("Setting up index...");
    let index_manager = Arc::new(IndexManager::new(&config.data_dir));
    let index_settings = IndexSettings::default();
    let (schema, _) = SchemaBuilder::new()
        .add_field(
            FieldConfig::new("id", FieldType::Keyword)
                .stored(true)
                .indexed(true),
        )
        .add_field(
            FieldConfig::new("title", FieldType::Text)
                .stored(true)
                .indexed(true),
        )
        .add_field(
            FieldConfig::new("content", FieldType::Text)
                .stored(true)
                .indexed(true),
        )
        .add_field(
            FieldConfig::new("category", FieldType::Keyword)
                .stored(true)
                .indexed(true),
        )
        .add_field(
            FieldConfig::new("score", FieldType::F64)
                .stored(true)
                .indexed(true),
        )
        .build()?;

    let index = index_manager
        .create_index(&config.index_name, schema, index_settings)
        .await?;

    println!("Index created successfully.");
    println!();

    // Run indexing test
    println!("Starting indexing test...");
    let indexing_results = run_indexing_test(
        Arc::new(index.clone()),
        config.document_count,
        config.indexing_workers,
        config.batch_size,
        config.progress_reporting,
    )
    .await?;

    println!("Indexing completed:");
    println!(
        "  Documents indexed: {}",
        indexing_results.documents_indexed
    );
    println!("  Duration: {:.2}s", indexing_results.duration_secs);
    println!(
        "  Throughput: {:.2} docs/sec",
        indexing_results.docs_per_second
    );
    println!(
        "  Avg time per doc: {:.2}ms",
        indexing_results.avg_time_per_doc_ms
    );
    println!(
        "  Peak memory increase: {:.2} MB",
        indexing_results.peak_memory_mb
    );
    println!();

    // Wait a bit for index to stabilize
    println!("Waiting for index to stabilize...");
    sleep(Duration::from_secs(2)).await;

    // Run search test
    println!("Starting search test...");
    let search_results = run_search_test(
        Arc::new(index.clone()),
        config.search_queries,
        config.search_workers,
        config.progress_reporting,
    )
    .await?;

    println!("Search completed:");
    println!("  Queries executed: {}", search_results.queries_executed);
    println!("  Successful: {}", search_results.successful_queries);
    println!("  Failed: {}", search_results.failed_queries);
    println!("  Duration: {:.2}s", search_results.duration_secs);
    println!("  Throughput: {:.2} QPS", search_results.queries_per_second);
    println!(
        "  Avg response time: {:.2}ms",
        search_results.avg_response_time_ms
    );
    println!(
        "  P95 response time: {:.2}ms",
        search_results.p95_response_time_ms
    );
    println!(
        "  P99 response time: {:.2}ms",
        search_results.p99_response_time_ms
    );
    println!();

    // Cleanup
    println!("Cleaning up...");
    index_manager.delete_index(&config.index_name).await?;

    let total_duration = start_time.elapsed();

    Ok(ScaleLoadTestResults {
        indexing: indexing_results,
        search: search_results,
        total_duration_secs: total_duration.as_secs_f64(),
    })
}

/// Run indexing test
async fn run_indexing_test(
    index: Arc<lexum_core::Index>,
    document_count: usize,
    workers: usize,
    batch_size: usize,
    progress_reporting: bool,
) -> Result<IndexingResults> {
    // Measure initial memory usage
    let initial_memory_mb = sys_info::mem_info()
        .map(|info| info.total as f64 / 1024.0 - info.free as f64 / 1024.0)
        .unwrap_or(0.0);

    let start_time = Instant::now();
    let mut response_times = Vec::new();
    let mut documents_indexed = 0;
    let mut peak_memory_mb = initial_memory_mb;

    // Calculate documents per worker
    let docs_per_worker = document_count / workers;
    let remainder = document_count % workers;

    let mut handles = Vec::new();

    for worker_id in 0..workers {
        let index = index.clone();
        let worker_docs = docs_per_worker + if worker_id < remainder { 1 } else { 0 };
        let batch_size = batch_size;

        let handle = tokio::spawn(async move {
            let mut local_indexed = 0;
            let mut local_times = Vec::new();

            for batch_start in (0..worker_docs).step_by(batch_size) {
                let batch_end = (batch_start + batch_size).min(worker_docs);
                let batch = (batch_start..batch_end).map(|i| {
                    let doc_id = format!("doc_{}_{}", worker_id, i);
                    json!({
                        "id": doc_id,
                        "title": format!("Document {} from worker {}", i, worker_id),
                        "content": format!("This is the content of document {} from worker {}. It contains some text for testing purposes.", i, worker_id),
                        "category": format!("category_{}", i % 10),
                        "score": (i as f64) * 0.1
                    })
                }).collect::<Vec<_>>();

                let batch_start_time = Instant::now();

                // Index documents in batch
                let document_store = DocumentStore::new(index.clone());
                for doc in batch {
                    if let Err(e) = document_store.add_document(doc).await {
                        eprintln!("Error indexing document: {}", e);
                    } else {
                        local_indexed += 1;
                    }
                }

                let batch_time = batch_start_time.elapsed();
                local_times.push(batch_time.as_millis() as f64 / (batch_end - batch_start) as f64);

                if progress_reporting && local_indexed % 10000 == 0 {
                    println!(
                        "  Worker {}: {} documents indexed",
                        worker_id, local_indexed
                    );
                    // Update peak memory during progress reporting
                    if let Ok(mem_info) = sys_info::mem_info() {
                        let current_memory_mb =
                            mem_info.total as f64 / 1024.0 - mem_info.free as f64 / 1024.0;
                        if current_memory_mb > peak_memory_mb {
                            peak_memory_mb = current_memory_mb;
                        }
                    }
                }
            }

            (local_indexed, local_times)
        });

        handles.push(handle);
    }

    // Collect results
    for handle in handles {
        let (local_indexed, local_times) = handle.await?;
        documents_indexed += local_indexed;
        response_times.extend(local_times);
    }

    let duration = start_time.elapsed();
    let duration_secs = duration.as_secs_f64();
    let docs_per_second = documents_indexed as f64 / duration_secs;

    // Calculate percentiles
    response_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95_idx = (response_times.len() as f64 * 0.95) as usize;
    let p99_idx = (response_times.len() as f64 * 0.99) as usize;

    // Measure final memory usage
    if let Ok(mem_info) = sys_info::mem_info() {
        let final_memory_mb = mem_info.total as f64 / 1024.0 - mem_info.free as f64 / 1024.0;
        if final_memory_mb > peak_memory_mb {
            peak_memory_mb = final_memory_mb;
        }
    }

    // Calculate memory increase (peak - initial)
    let memory_increase_mb = (peak_memory_mb - initial_memory_mb).max(0.0);

    Ok(IndexingResults {
        documents_indexed,
        duration_secs,
        docs_per_second,
        avg_time_per_doc_ms: response_times.iter().sum::<f64>() / response_times.len() as f64,
        p95_time_per_doc_ms: response_times.get(p95_idx).copied().unwrap_or(0.0),
        p99_time_per_doc_ms: response_times.get(p99_idx).copied().unwrap_or(0.0),
        peak_memory_mb: memory_increase_mb,
    })
}

/// Run search test
async fn run_search_test(
    index: Arc<lexum_core::Index>,
    query_count: usize,
    workers: usize,
    progress_reporting: bool,
) -> Result<SearchResults> {
    let start_time = Instant::now();
    let mut response_times = Vec::new();
    let mut successful_queries = 0;
    let mut failed_queries = 0;

    let queries_per_worker = query_count / workers;
    let remainder = query_count % workers;

    let mut handles = Vec::new();

    for worker_id in 0..workers {
        let index = index.clone();
        let worker_queries = queries_per_worker + if worker_id < remainder { 1 } else { 0 };

        let handle = tokio::spawn(async move {
            let mut local_successful = 0;
            let mut local_failed = 0;
            let mut local_times = Vec::new();

            let search_executor = SearchExecutor::new(index.clone());

            for i in 0..worker_queries {
                // Create different query types
                let query = if i % 3 == 0 {
                    QueryBuilder::match_query("title", &format!("Document {}", i % 1000))
                } else if i % 3 == 1 {
                    QueryBuilder::term_query("category", &format!("category_{}", i % 10))
                } else {
                    let mut range_query = QueryBuilder::range_query("score");
                    range_query = range_query.gte(serde_json::json!((i % 100) as f64 * 0.1));
                    range_query = range_query.lte(serde_json::json!((i % 100 + 10) as f64 * 0.1));
                    Query::Range(range_query)
                };

                let query_start = Instant::now();
                match search_executor.search(query, 10, 0, None).await {
                    Ok(_) => {
                        local_successful += 1;
                        let query_time = query_start.elapsed();
                        local_times.push(query_time.as_millis() as f64);
                    }
                    Err(_) => {
                        local_failed += 1;
                    }
                }

                if progress_reporting && (i + 1) % 1000 == 0 {
                    println!("  Worker {}: {} queries executed", worker_id, i + 1);
                }
            }

            (local_successful, local_failed, local_times)
        });

        handles.push(handle);
    }

    // Collect results
    for handle in handles {
        let (local_successful, local_failed, local_times) = handle.await?;
        successful_queries += local_successful;
        failed_queries += local_failed;
        response_times.extend(local_times);
    }

    let duration = start_time.elapsed();
    let duration_secs = duration.as_secs_f64();
    let queries_per_second = (successful_queries + failed_queries) as f64 / duration_secs;

    // Calculate percentiles
    response_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95_idx = (response_times.len() as f64 * 0.95) as usize;
    let p99_idx = (response_times.len() as f64 * 0.99) as usize;

    Ok(SearchResults {
        queries_executed: successful_queries + failed_queries,
        successful_queries,
        failed_queries,
        duration_secs,
        queries_per_second,
        avg_response_time_ms: response_times.iter().sum::<f64>() / response_times.len() as f64,
        p95_response_time_ms: response_times.get(p95_idx).copied().unwrap_or(0.0),
        p99_response_time_ms: response_times.get(p99_idx).copied().unwrap_or(0.0),
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let matches = Command::new("lexum-load-test-scale")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Large-scale load testing for Lexum (1M+ documents)")
        .arg(
            Arg::new("documents")
                .short('d')
                .long("documents")
                .value_name("NUMBER")
                .help("Number of documents to index")
                .default_value("1000000"),
        )
        .arg(
            Arg::new("indexing-workers")
                .long("indexing-workers")
                .value_name("NUMBER")
                .help("Number of concurrent indexing workers")
                .default_value("10"),
        )
        .arg(
            Arg::new("batch-size")
                .long("batch-size")
                .value_name("NUMBER")
                .help("Batch size for bulk indexing")
                .default_value("1000"),
        )
        .arg(
            Arg::new("search-queries")
                .long("search-queries")
                .value_name("NUMBER")
                .help("Number of search queries to execute")
                .default_value("10000"),
        )
        .arg(
            Arg::new("search-workers")
                .long("search-workers")
                .value_name("NUMBER")
                .help("Number of concurrent search workers")
                .default_value("50"),
        )
        .arg(
            Arg::new("index-name")
                .long("index-name")
                .value_name("NAME")
                .help("Index name for testing")
                .default_value("scale_test_index"),
        )
        .arg(
            Arg::new("data-dir")
                .long("data-dir")
                .value_name("PATH")
                .help("Data directory for index storage")
                .default_value("./data/scale_test"),
        )
        .arg(
            Arg::new("no-progress")
                .long("no-progress")
                .help("Disable progress reporting")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let config = ScaleLoadTestConfig {
        document_count: matches
            .get_one::<String>("documents")
            .unwrap()
            .parse::<usize>()?,
        indexing_workers: matches
            .get_one::<String>("indexing-workers")
            .unwrap()
            .parse::<usize>()?,
        batch_size: matches
            .get_one::<String>("batch-size")
            .unwrap()
            .parse::<usize>()?,
        search_queries: matches
            .get_one::<String>("search-queries")
            .unwrap()
            .parse::<usize>()?,
        search_workers: matches
            .get_one::<String>("search-workers")
            .unwrap()
            .parse::<usize>()?,
        index_name: matches.get_one::<String>("index-name").unwrap().clone(),
        data_dir: matches.get_one::<String>("data-dir").unwrap().clone(),
        progress_reporting: !matches.get_flag("no-progress"),
    };

    let results = run_scale_load_test(config).await?;

    // Print final summary
    println!("=== Large-Scale Load Test Summary ===");
    println!();
    println!("Indexing Results:");
    println!(
        "  Documents indexed: {}",
        results.indexing.documents_indexed
    );
    println!("  Duration: {:.2}s", results.indexing.duration_secs);
    println!(
        "  Throughput: {:.2} docs/sec",
        results.indexing.docs_per_second
    );
    println!(
        "  Avg time per doc: {:.2}ms",
        results.indexing.avg_time_per_doc_ms
    );
    println!(
        "  P95 time per doc: {:.2}ms",
        results.indexing.p95_time_per_doc_ms
    );
    println!(
        "  P99 time per doc: {:.2}ms",
        results.indexing.p99_time_per_doc_ms
    );
    println!(
        "  Peak memory increase: {:.2} MB",
        results.indexing.peak_memory_mb
    );
    println!();
    println!("Search Results:");
    println!("  Queries executed: {}", results.search.queries_executed);
    println!("  Successful: {}", results.search.successful_queries);
    println!("  Failed: {}", results.search.failed_queries);
    println!(
        "  Success rate: {:.2}%",
        (results.search.successful_queries as f64 / results.search.queries_executed as f64) * 100.0
    );
    println!("  Duration: {:.2}s", results.search.duration_secs);
    println!("  Throughput: {:.2} QPS", results.search.queries_per_second);
    println!(
        "  Avg response time: {:.2}ms",
        results.search.avg_response_time_ms
    );
    println!(
        "  P95 response time: {:.2}ms",
        results.search.p95_response_time_ms
    );
    println!(
        "  P99 response time: {:.2}ms",
        results.search.p99_response_time_ms
    );
    println!();
    println!("Total test duration: {:.2}s", results.total_duration_secs);

    Ok(())
}
