//! Network performance benchmarks
//!
//! This benchmark suite measures network performance including:
//! - Connection pooling effectiveness
//! - Request batching performance
//! - Network throughput
//! - Request latency
//! - Serialization performance

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use reqwest::Client;
use serde_json::json;
use std::time::Instant;
use tokio::runtime::Runtime;

/// Setup test HTTP client with connection pooling
fn create_client_with_pooling() -> Client {
    Client::builder()
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap()
}

/// Setup test HTTP client without connection pooling
fn create_client_without_pooling() -> Client {
    Client::builder()
        .pool_max_idle_per_host(0)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap()
}

/// Generate test JSON payload
fn generate_test_payload(size: usize) -> serde_json::Value {
    let mut data = Vec::new();
    for i in 0..size {
        data.push(json!({
            "id": i,
            "title": format!("Document Title {}", i),
            "content": format!("This is the content of document number {} with some searchable text", i),
            "category": if i % 2 == 0 { "tech" } else { "news" },
            "views": i * 10,
            "tags": vec!["tag1", "tag2", "tag3", "tag4", "tag5"]
        }));
    }
    json!(data)
}

/// Benchmark connection pooling effectiveness
fn bench_connection_pooling(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_pooling");
    let rt = Runtime::new().unwrap();

    // Note: This benchmark requires a running server
    // For actual testing, you would need to start a test server
    let base_url = "http://localhost:8080";

    group.bench_function("with_pooling", |b| {
        let client = create_client_with_pooling();
        b.iter(|| {
            rt.block_on(async {
                // Simulate multiple requests reusing connections
                for _ in 0..10 {
                    let _ = client.get(format!("{base_url}/api/v1/health")).send().await;
                }
            });
        });
    });

    group.bench_function("without_pooling", |b| {
        let client = create_client_without_pooling();
        b.iter(|| {
            rt.block_on(async {
                // Simulate multiple requests without connection reuse
                for _ in 0..10 {
                    let _ = client.get(format!("{base_url}/api/v1/health")).send().await;
                }
            });
        });
    });

    group.finish();
}

/// Benchmark request batching performance
fn bench_request_batching(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_batching");
    let rt = Runtime::new().unwrap();

    let base_url = "http://localhost:8080";
    let client = create_client_with_pooling();

    for batch_size in [1, 5, 10, 20, 50] {
        // Generate batch request
        let batch_requests: Vec<_> = (0..batch_size)
            .map(|_| {
                json!({
                    "method": "GET",
                    "path": format!("/api/v1/indices/test_index/stats"),
                    "headers": {},
                    "body": null
                })
            })
            .collect();

        let batch_payload = json!({
            "requests": batch_requests
        });

        group.bench_with_input(
            BenchmarkId::new("batch_size", batch_size),
            &batch_size,
            |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        let _ = client
                            .post(format!("{base_url}/api/v1/_batch"))
                            .json(&batch_payload)
                            .send()
                            .await;
                    });
                });
            },
        );

        // Compare with individual requests
        group.bench_with_input(
            BenchmarkId::new("individual_requests", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    rt.block_on(async {
                        for _ in 0..batch_size {
                            let _ = client
                                .get(format!("{base_url}/api/v1/indices/test_index/stats"))
                                .send()
                                .await;
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark serialization performance
fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    for payload_size in [10, 100, 1000] {
        let payload = generate_test_payload(payload_size);

        // Benchmark JSON serialization
        group.bench_with_input(
            BenchmarkId::new("serialize", payload_size),
            &payload,
            |b, payload| {
                b.iter(|| {
                    let _ = black_box(serde_json::to_string(payload).unwrap());
                });
            },
        );

        // Benchmark JSON deserialization
        let serialized = serde_json::to_string(&payload).unwrap();
        group.bench_with_input(
            BenchmarkId::new("deserialize", payload_size),
            &serialized,
            |b, serialized| {
                b.iter(|| {
                    let _: serde_json::Value = black_box(serde_json::from_str(serialized).unwrap());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark network throughput
fn bench_network_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("network_throughput");
    let rt = Runtime::new().unwrap();

    let base_url = "http://localhost:8080";
    let client = create_client_with_pooling();

    for concurrent_requests in [1, 5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::new("concurrent", concurrent_requests),
            &concurrent_requests,
            |b, &concurrent| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut handles = Vec::new();
                        for _ in 0..concurrent {
                            let client = client.clone();
                            let url = format!("{base_url}/api/v1/health");
                            handles.push(tokio::spawn(async move {
                                let _ = client.get(&url).send().await;
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

/// Benchmark request latency
fn bench_request_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_latency");
    let rt = Runtime::new().unwrap();

    let base_url = "http://localhost:8080";
    let client = create_client_with_pooling();

    let endpoints = vec![
        "/api/v1/health",
        "/api/v1/indices",
        "/api/v1/indices/test_index/stats",
    ];

    for endpoint in &endpoints {
        group.bench_function(endpoint.replace("/", "_"), |b| {
            b.iter(|| {
                rt.block_on(async {
                    let start = Instant::now();
                    let _ = client.get(format!("{base_url}{endpoint}")).send().await;
                    black_box(start.elapsed());
                });
            });
        });
    }

    group.finish();
}

/// Benchmark payload size impact
fn bench_payload_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("payload_size");
    let rt = Runtime::new().unwrap();

    let base_url = "http://localhost:8080";
    let client = create_client_with_pooling();

    for payload_size in [10, 100, 500, 1000] {
        let payload = generate_test_payload(payload_size);

        group.bench_with_input(
            BenchmarkId::new("post_payload", payload_size),
            &payload,
            |b, payload| {
                b.iter(|| {
                    rt.block_on(async {
                        let _ = client
                            .post(format!("{base_url}/api/v1/indices/test_index/documents"))
                            .json(payload)
                            .send()
                            .await;
                    });
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_connection_pooling,
    bench_request_batching,
    bench_serialization,
    bench_network_throughput,
    bench_request_latency,
    bench_payload_size
);
criterion_main!(benches);
