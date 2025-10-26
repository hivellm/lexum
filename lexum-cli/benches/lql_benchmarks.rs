//! LQL (Lexum Query Language) benchmark suite
//!
//! This module provides comprehensive benchmarks for LQL query parsing,
//! optimization, and execution performance.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lexum_cli::lql::{LqlParser, QueryOptimizer};
use lexum_core::Query;
use std::time::Duration;

/// Benchmark LQL parsing performance
fn bench_lql_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("lql_parsing");
    group.measurement_time(Duration::from_secs(10));

    let test_queries = vec![
        ("simple_term", "title:rust"),
        ("simple_match", "MATCH content:programming"),
        ("from_query", "FROM documents WHERE status:active"),
        (
            "select_query",
            "SELECT * FROM documents WHERE category:tech",
        ),
        ("boolean_query", "title:rust AND category:programming"),
        ("range_query", "score:[8.0,10.0]"),
        ("fuzzy_query", "content:~programming"),
        ("phrase_query", "content:\"machine learning\""),
        (
            "complex_boolean",
            "title:rust AND (category:programming OR category:systems) AND score:[7.0,10.0]",
        ),
        ("wildcard_query", "content:test*"),
    ];

    for (name, query) in test_queries {
        group.bench_with_input(BenchmarkId::new("parse", name), &query, |b, query| {
            b.iter(|| LqlParser::parse(black_box(query)).unwrap())
        });
    }

    group.finish();
}

/// Benchmark LQL query optimization performance
fn bench_lql_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("lql_optimization");
    group.measurement_time(Duration::from_secs(10));

    let test_queries = vec![
        (
            "term_query",
            Query::Term(lexum_core::query::types::TermQuery {
                field: "status".to_string(),
                value: "active".to_string(),
            }),
        ),
        (
            "match_query",
            Query::Match(lexum_core::query::types::MatchQuery {
                field: "content".to_string(),
                query: "programming".to_string(),
            }),
        ),
        (
            "fuzzy_query",
            Query::Fuzzy(lexum_core::query::types::FuzzyQuery {
                field: "content".to_string(),
                value: "programming".to_string(),
                fuzziness: 2,
                prefix_length: 0,
                transpositions: true,
            }),
        ),
        (
            "range_query",
            Query::Range(lexum_core::query::types::RangeQuery {
                field: "score".to_string(),
                min: Some("8.0".to_string()),
                max: Some("10.0".to_string()),
                min_inclusive: true,
                max_inclusive: true,
            }),
        ),
        (
            "boolean_query",
            Query::Boolean(lexum_core::query::types::BooleanQuery {
                must: vec![
                    Query::Term(lexum_core::query::types::TermQuery {
                        field: "status".to_string(),
                        value: "active".to_string(),
                    }),
                    Query::Match(lexum_core::query::types::MatchQuery {
                        field: "content".to_string(),
                        query: "programming".to_string(),
                    }),
                ],
                should: vec![],
                must_not: vec![],
                filter: vec![],
            }),
        ),
    ];

    for (name, query) in test_queries {
        group.bench_with_input(BenchmarkId::new("optimize", name), &query, |b, query| {
            b.iter(|| QueryOptimizer::optimize(black_box(query.clone())))
        });
    }

    group.finish();
}

/// Benchmark LQL parsing with optimization
fn bench_lql_parse_with_plan(c: &mut Criterion) {
    let mut group = c.benchmark_group("lql_parse_with_plan");
    group.measurement_time(Duration::from_secs(10));

    let test_queries = vec![
        ("simple_term", "title:rust"),
        ("simple_match", "MATCH content:programming"),
        ("from_query", "FROM documents WHERE status:active"),
        ("boolean_query", "title:rust AND category:programming"),
        (
            "complex_query",
            "title:rust AND (category:programming OR category:systems) AND score:[7.0,10.0]",
        ),
    ];

    for (name, query) in test_queries {
        group.bench_with_input(
            BenchmarkId::new("parse_with_plan", name),
            &query,
            |b, query| b.iter(|| LqlParser::parse_with_plan(black_box(query)).unwrap()),
        );
    }

    group.finish();
}

/// Benchmark query cache performance
fn bench_query_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_cache");
    group.measurement_time(Duration::from_secs(10));

    let queries = vec![
        "title:rust",
        "MATCH content:programming",
        "FROM documents WHERE status:active",
        "title:rust AND category:programming",
    ];

    // Warm up the cache
    for query in &queries {
        let _ = LqlParser::parse(query).unwrap();
    }

    for query in queries {
        group.bench_with_input(
            BenchmarkId::new("cached_parse", query),
            &query,
            |b, query| b.iter(|| LqlParser::parse(black_box(query)).unwrap()),
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_lql_parsing,
    bench_lql_optimization,
    bench_lql_parse_with_plan,
    bench_query_cache
);

criterion_main!(benches);
