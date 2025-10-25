use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lexum_core::config::ServerConfig;
use lexum_core::index::manager::IndexManager;
use lexum_core::schema::{FieldDef, FieldType};
use lexum_core::search::executor::SearchExecutor;
use lexum_core::search::query::{BoolQuery, MatchQuery, Query, TermQuery};
use serde_json::json;
use std::sync::Arc;

fn setup_test_index() -> (IndexManager, String) {
    let config = ServerConfig::default();
    let manager = IndexManager::new(&config.data_dir).unwrap();
    
    let index_name = format!("bench_index_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis());
    
    let schema = vec![
        FieldDef {
            name: "title".to_string(),
            field_type: FieldType::Text,
            stored: true,
        },
        FieldDef {
            name: "content".to_string(),
            field_type: FieldType::Text,
            stored: true,
        },
        FieldDef {
            name: "category".to_string(),
            field_type: FieldType::String,
            stored: true,
        },
    ];
    
    manager.create_index(&index_name, schema).unwrap();
    
    // Index sample documents
    for i in 0..1000 {
        let doc = json!({
            "title": format!("Document {}", i),
            "content": format!("This is the content of document {}. It contains various words for searching.", i),
            "category": format!("category_{}", i % 10),
        });
        manager.index_document(&index_name, &doc).unwrap();
    }
    
    manager.commit(&index_name).unwrap();
    
    (manager, index_name)
}

fn bench_match_query(c: &mut Criterion) {
    let (manager, index_name) = setup_test_index();
    let index = manager.get_index(&index_name).unwrap();
    let executor = SearchExecutor::new(Arc::new(index));
    
    c.bench_function("match_query_simple", |b| {
        b.iter(|| {
            let query = Query::Match(MatchQuery {
                field: "content".to_string(),
                query: "document".to_string(),
            });
            executor.search(black_box(&query), 10, 0, None).unwrap()
        })
    });
    
    let _ = manager.delete_index(&index_name);
}

fn bench_term_query(c: &mut Criterion) {
    let (manager, index_name) = setup_test_index();
    let index = manager.get_index(&index_name).unwrap();
    let executor = SearchExecutor::new(Arc::new(index));
    
    c.bench_function("term_query_exact", |b| {
        b.iter(|| {
            let query = Query::Term(TermQuery {
                field: "category".to_string(),
                value: "category_5".to_string(),
            });
            executor.search(black_box(&query), 10, 0, None).unwrap()
        })
    });
    
    let _ = manager.delete_index(&index_name);
}

fn bench_bool_query(c: &mut Criterion) {
    let (manager, index_name) = setup_test_index();
    let index = manager.get_index(&index_name).unwrap();
    let executor = SearchExecutor::new(Arc::new(index));
    
    c.bench_function("bool_query_complex", |b| {
        b.iter(|| {
            let query = Query::Bool(BoolQuery {
                must: Some(vec![
                    Query::Match(MatchQuery {
                        field: "content".to_string(),
                        query: "document".to_string(),
                    }),
                ]),
                should: Some(vec![
                    Query::Term(TermQuery {
                        field: "category".to_string(),
                        value: "category_5".to_string(),
                    }),
                ]),
                must_not: None,
            });
            executor.search(black_box(&query), 10, 0, None).unwrap()
        })
    });
    
    let _ = manager.delete_index(&index_name);
}

fn bench_pagination(c: &mut Criterion) {
    let (manager, index_name) = setup_test_index();
    let index = manager.get_index(&index_name).unwrap();
    let executor = SearchExecutor::new(Arc::new(index));
    
    let query = Query::Match(MatchQuery {
        field: "content".to_string(),
        query: "document".to_string(),
    });
    
    let mut group = c.benchmark_group("pagination");
    
    for offset in [0, 50, 100, 200].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(offset), offset, |b, &offset| {
            b.iter(|| {
                executor.search(black_box(&query), 20, offset, None).unwrap()
            })
        });
    }
    
    group.finish();
    let _ = manager.delete_index(&index_name);
}

fn bench_cache(c: &mut Criterion) {
    let (manager, index_name) = setup_test_index();
    let index = manager.get_index(&index_name).unwrap();
    let executor = SearchExecutor::new(Arc::new(index));
    
    let query = Query::Match(MatchQuery {
        field: "content".to_string(),
        query: "document".to_string(),
    });
    
    let mut group = c.benchmark_group("cache");
    
    group.bench_function("cache_miss", |b| {
        b.iter(|| {
            executor.clear_cache();
            executor.search(black_box(&query), 10, 0, None).unwrap()
        })
    });
    
    // Warm up cache
    executor.search(&query, 10, 0, None).unwrap();
    
    group.bench_function("cache_hit", |b| {
        b.iter(|| {
            executor.search(black_box(&query), 10, 0, None).unwrap()
        })
    });
    
    group.finish();
    let _ = manager.delete_index(&index_name);
}

criterion_group!(
    benches,
    bench_match_query,
    bench_term_query,
    bench_bool_query,
    bench_pagination,
    bench_cache
);

criterion_main!(benches);

