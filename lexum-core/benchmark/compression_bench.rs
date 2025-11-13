//! Compression ratio benchmarks
//!
//! This benchmark suite measures compression ratios and performance
//! for different compression algorithms (gzip, zstd, lz4) at various levels.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lexum_core::snapshot::compression::{CompressionConfig, CompressionType, SnapshotCompressor};
use serde_json::json;

/// Generate test data with different characteristics
fn generate_test_data(data_type: &str, size_kb: usize) -> Vec<u8> {
    match data_type {
        "text" => {
            // Repetitive text data (highly compressible)
            let text =
                "This is a sample text that will be repeated many times. ".repeat(size_kb * 10);
            text.into_bytes()
        }
        "json" => {
            // JSON data (moderately compressible)
            let mut data = Vec::new();
            for i in 0..size_kb * 10 {
                let doc = json!({
                    "id": i,
                    "title": format!("Document {}", i),
                    "content": format!("This is the content of document number {}", i),
                    "category": if i % 2 == 0 { "tech" } else { "news" },
                    "views": i * 10,
                    "tags": vec!["tag1", "tag2", "tag3"]
                });
                data.extend_from_slice(serde_json::to_string(&doc).unwrap().as_bytes());
                data.push(b'\n');
            }
            data
        }
        "random" => {
            // Random data (low compressibility)
            use rand::RngCore;
            let mut rng = rand::thread_rng();
            let mut data = vec![0u8; size_kb * 1024];
            rng.fill_bytes(&mut data);
            data
        }
        "mixed" => {
            // Mixed data (realistic scenario)
            let mut data = Vec::new();
            for i in 0..size_kb {
                let doc = json!({
                    "id": i,
                    "title": format!("Document Title {}", i),
                    "content": format!("This is the content of document number {} with some searchable text", i),
                    "metadata": {
                        "created": "2024-01-01T00:00:00Z",
                        "updated": "2024-01-02T00:00:00Z",
                        "tags": vec!["tag1", "tag2", "tag3", "tag4", "tag5"]
                    }
                });
                data.extend_from_slice(serde_json::to_string(&doc).unwrap().as_bytes());
                data.push(b'\n');
            }
            data
        }
        _ => vec![0; size_kb * 1024],
    }
}

/// Benchmark compression ratios for different algorithms
fn bench_compression_ratios(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratios");

    let data_types = vec!["text", "json", "mixed"];
    let sizes_kb = vec![10, 100, 1000];

    for data_type in &data_types {
        for size_kb in &sizes_kb {
            let data = generate_test_data(data_type, *size_kb);
            let original_size = data.len();

            // Test different compression algorithms
            let algorithms = vec![
                (CompressionType::Gzip, "gzip"),
                (CompressionType::Zstd, "zstd"),
                (CompressionType::Lz4, "lz4"),
            ];

            for (algorithm, name) in &algorithms {
                let config = CompressionConfig {
                    algorithm: *algorithm,
                    level: 3, // Default level
                    use_dictionary: false,
                    dictionary_size: 0,
                };

                let compressor = SnapshotCompressor::new(config);

                group.bench_with_input(
                    BenchmarkId::new(format!("{name}_{size_kb}kb"), data_type),
                    &data,
                    |b, data| {
                        b.iter(|| {
                            let compressed = compressor.compress(black_box(data)).unwrap();
                            let ratio =
                                compressor.compression_ratio(original_size, compressed.len());
                            black_box(ratio);
                        });
                    },
                );

                // Report compression ratio
                let compressed = compressor.compress(&data).unwrap();
                let ratio = compressor.compression_ratio(original_size, compressed.len());
                eprintln!(
                    "{} {} ({}KB): Original: {} bytes, Compressed: {} bytes, Ratio: {:.2}%",
                    name,
                    data_type,
                    size_kb,
                    original_size,
                    compressed.len(),
                    ratio * 100.0
                );
            }
        }
    }

    group.finish();
}

/// Benchmark compression at different levels
fn bench_compression_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_levels");

    let data = generate_test_data("mixed", 100);
    let original_size = data.len();

    // Test zstd at different levels
    for level in [1, 3, 6, 9, 12, 15, 18, 22] {
        let config = CompressionConfig {
            algorithm: CompressionType::Zstd,
            level,
            use_dictionary: false,
            dictionary_size: 0,
        };

        let compressor = SnapshotCompressor::new(config);

        group.bench_with_input(BenchmarkId::new("zstd_level", level), &level, |b, _| {
            b.iter(|| {
                let compressed = compressor.compress(black_box(&data)).unwrap();
                black_box(compressed);
            });
        });

        // Report compression ratio
        let compressed = compressor.compress(&data).unwrap();
        let ratio = compressor.compression_ratio(original_size, compressed.len());
        eprintln!(
            "Zstd level {}: Original: {} bytes, Compressed: {} bytes, Ratio: {:.2}%",
            level,
            original_size,
            compressed.len(),
            ratio * 100.0
        );
    }

    // Test gzip at different levels
    for level in [1, 3, 6, 9] {
        let config = CompressionConfig {
            algorithm: CompressionType::Gzip,
            level,
            use_dictionary: false,
            dictionary_size: 0,
        };

        let compressor = SnapshotCompressor::new(config);

        group.bench_with_input(BenchmarkId::new("gzip_level", level), &level, |b, _| {
            b.iter(|| {
                let compressed = compressor.compress(black_box(&data)).unwrap();
                black_box(compressed);
            });
        });

        // Report compression ratio
        let compressed = compressor.compress(&data).unwrap();
        let ratio = compressor.compression_ratio(original_size, compressed.len());
        eprintln!(
            "Gzip level {}: Original: {} bytes, Compressed: {} bytes, Ratio: {:.2}%",
            level,
            original_size,
            compressed.len(),
            ratio * 100.0
        );
    }

    group.finish();
}

/// Benchmark compression vs decompression speed
fn bench_compression_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_speed");

    let data = generate_test_data("mixed", 100);

    let algorithms = vec![
        (CompressionType::Gzip, "gzip"),
        (CompressionType::Zstd, "zstd"),
        (CompressionType::Lz4, "lz4"),
    ];

    for (algorithm, name) in &algorithms {
        let config = CompressionConfig {
            algorithm: *algorithm,
            level: 3,
            use_dictionary: false,
            dictionary_size: 0,
        };

        let compressor = SnapshotCompressor::new(config);

        // Benchmark compression
        group.bench_function(format!("{name}_compress"), |b| {
            b.iter(|| {
                let compressed = compressor.compress(black_box(&data)).unwrap();
                black_box(compressed);
            });
        });

        // Benchmark decompression
        let compressed = compressor.compress(&data).unwrap();
        group.bench_function(format!("{name}_decompress"), |b| {
            b.iter(|| {
                let decompressed = compressor.decompress(black_box(&compressed)).unwrap();
                black_box(decompressed);
            });
        });
    }

    group.finish();
}

/// Benchmark compression with dictionary
fn bench_compression_with_dictionary(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_dictionary");

    let data = generate_test_data("mixed", 100);
    let original_size = data.len();

    // Create dictionary from sample data
    let dictionary = data[..data.len() / 10].to_vec();

    // Test with and without dictionary
    let config_without = CompressionConfig {
        algorithm: CompressionType::Zstd,
        level: 3,
        use_dictionary: false,
        dictionary_size: 0,
    };

    let config_with = CompressionConfig {
        algorithm: CompressionType::Zstd,
        level: 3,
        use_dictionary: true,
        dictionary_size: dictionary.len(),
    };

    let compressor_without = SnapshotCompressor::new(config_without);
    let compressor_with = SnapshotCompressor::with_dictionary(config_with, dictionary.clone());

    // Benchmark without dictionary
    group.bench_function("zstd_without_dict", |b| {
        b.iter(|| {
            let compressed = compressor_without.compress(black_box(&data)).unwrap();
            black_box(compressed);
        });
    });

    let compressed_without = compressor_without.compress(&data).unwrap();
    let ratio_without =
        compressor_without.compression_ratio(original_size, compressed_without.len());
    eprintln!(
        "Zstd without dictionary: Ratio: {:.2}%",
        ratio_without * 100.0
    );

    // Benchmark with dictionary
    group.bench_function("zstd_with_dict", |b| {
        b.iter(|| {
            let compressed = compressor_with.compress(black_box(&data)).unwrap();
            black_box(compressed);
        });
    });

    let compressed_with = compressor_with.compress(&data).unwrap();
    let ratio_with = compressor_with.compression_ratio(original_size, compressed_with.len());
    eprintln!("Zstd with dictionary: Ratio: {:.2}%", ratio_with * 100.0);

    group.finish();
}

/// Benchmark compression for different data types
fn bench_compression_data_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_data_types");

    let data_types = vec![
        ("text", "highly_compressible"),
        ("json", "moderately_compressible"),
        ("mixed", "realistic"),
        ("random", "low_compressibility"),
    ];

    for (data_type, label) in &data_types {
        let data = generate_test_data(data_type, 100);
        let original_size = data.len();

        let config = CompressionConfig {
            algorithm: CompressionType::Zstd,
            level: 3,
            use_dictionary: false,
            dictionary_size: 0,
        };

        let compressor = SnapshotCompressor::new(config);

        group.bench_with_input(BenchmarkId::from_parameter(label), &data, |b, data| {
            b.iter(|| {
                let compressed = compressor.compress(black_box(data)).unwrap();
                black_box(compressed);
            });
        });

        // Report compression ratio
        let compressed = compressor.compress(&data).unwrap();
        let ratio = compressor.compression_ratio(original_size, compressed.len());
        eprintln!(
            "{} data: Original: {} bytes, Compressed: {} bytes, Ratio: {:.2}%",
            label,
            original_size,
            compressed.len(),
            ratio * 100.0
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_compression_ratios,
    bench_compression_levels,
    bench_compression_speed,
    bench_compression_with_dictionary,
    bench_compression_data_types
);
criterion_main!(benches);
