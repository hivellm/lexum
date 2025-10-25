//! Search performance benchmarks

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lexum_core::*;
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

    // Add documents
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

    // Small delay for indexing
    std::thread::sleep(std::time::Duration::from_millis(200));

    (temp_dir, Arc::new(index))
}

fn bench_match_query(c: &mut Criterion) {
    let (_temp_dir, index) = create_test_index_with_docs(1000);
    let executor = SearchExecutor::new(index);

    c.bench_function("match_query_1k_docs", |b| {
        b.iter(|| {
            let query = QueryBuilder::match_query("content", "searchable text");
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                executor
                    .search(black_box(query), 10, 0, None)
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_term_query(c: &mut Criterion) {
    let (_temp_dir, index) = create_test_index_with_docs(1000);
    let executor = SearchExecutor::new(index);

    c.bench_function("term_query_1k_docs", |b| {
        b.iter(|| {
            let query = QueryBuilder::term_query("category", "tech");
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                executor
                    .search(black_box(query), 10, 0, None)
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_fuzzy_query(c: &mut Criterion) {
    let (_temp_dir, index) = create_test_index_with_docs(1000);
    let executor = SearchExecutor::new(index);

    c.bench_function("fuzzy_query_1k_docs", |b| {
        b.iter(|| {
            let query = QueryBuilder::fuzzy_query("title", "Documnt"); // typo
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                executor
                    .search(black_box(query), 10, 0, None)
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_phrase_query(c: &mut Criterion) {
    let (_temp_dir, index) = create_test_index_with_docs(1000);
    let executor = SearchExecutor::new(index);

    c.bench_function("phrase_query_1k_docs", |b| {
        b.iter(|| {
            let query = QueryBuilder::phrase_query("content", "searchable text");
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                executor
                    .search(black_box(query), 10, 0, None)
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_bool_query(c: &mut Criterion) {
    let (_temp_dir, index) = create_test_index_with_docs(1000);
    let executor = SearchExecutor::new(index);

    c.bench_function("bool_query_1k_docs", |b| {
        b.iter(|| {
            let query = Query::Bool(
                BoolQuery::new()
                    .must(Query::Match(MatchQuery::new("content", "searchable")))
                    .filter(Query::Term(TermQuery::new("category", "tech"))),
            );
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                executor
                    .search(black_box(query), 10, 0, None)
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_query_cache(c: &mut Criterion) {
    let (_temp_dir, index) = create_test_index_with_docs(1000);
    let executor = SearchExecutor::new(index);

    let query = QueryBuilder::match_query("content", "searchable");

    c.bench_function("query_cache_cold", |b| {
        b.iter(|| {
            executor.clear_cache();
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                executor
                    .search(black_box(query.clone()), 10, 0, None)
                    .await
                    .unwrap()
            })
        })
    });

    c.bench_function("query_cache_hot", |b| {
        // Warm up cache
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { executor.search(query.clone(), 10, 0, None).await.unwrap() });

        b.iter(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                executor
                    .search(black_box(query.clone()), 10, 0, None)
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_sorting(c: &mut Criterion) {
    let (_temp_dir, index) = create_test_index_with_docs(1000);
    let executor = SearchExecutor::new(index);

    c.bench_function("sort_by_score", |b| {
        b.iter(|| {
            let query = QueryBuilder::match_all();
            let sort = Some(SortOption::desc("_score"));
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                executor
                    .search(black_box(query), 10, 0, black_box(sort))
                    .await
                    .unwrap()
            })
        })
    });

    c.bench_function("sort_by_field", |b| {
        b.iter(|| {
            let query = QueryBuilder::match_all();
            let sort = Some(SortOption::asc("views"));
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                executor
                    .search(black_box(query), 10, 0, black_box(sort))
                    .await
                    .unwrap()
            })
        })
    });
}

criterion_group!(
    benches,
    bench_match_query,
    bench_term_query,
    bench_fuzzy_query,
    bench_phrase_query,
    bench_bool_query,
    bench_query_cache,
    bench_sorting
);
criterion_main!(benches);
