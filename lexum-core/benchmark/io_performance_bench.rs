//! I/O performance benchmarks
//!
//! This benchmark suite measures the performance of various I/O operations,
//! including read-ahead optimization, buffered writes, and memory-mapped files.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lexum_core::IndexManager;
use lexum_core::io::{
    BufferedFileWriter, DEFAULT_READ_AHEAD_SIZE, DEFAULT_WRITE_BUFFER_SIZE, ReadAheadReader,
};
use lexum_core::{IndexSettings, SchemaBuilder};
use tempfile::TempDir;
use tokio::fs;

fn create_test_file(size_mb: usize) -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_data.bin");

    // Create test data
    let data = vec![0u8; size_mb * 1024 * 1024];
    std::fs::write(&file_path, &data).unwrap();

    (temp_dir, file_path)
}

/// Benchmark: Buffered vs unbuffered writes
fn bench_buffered_writes(c: &mut Criterion) {
    let mut group = c.benchmark_group("io_buffered_writes");

    for size_mb in [1, 10, 100] {
        let data = vec![0u8; size_mb * 1024 * 1024];

        // Unbuffered write
        group.bench_with_input(BenchmarkId::new("unbuffered", size_mb), &size_mb, |b, _| {
            b.iter(|| {
                let temp_dir = TempDir::new().unwrap();
                let file_path = temp_dir.path().join("test.bin");
                std::fs::write(&file_path, black_box(&data)).unwrap();
            });
        });

        // Buffered write
        group.bench_with_input(BenchmarkId::new("buffered", size_mb), &size_mb, |b, _| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            b.iter(|| {
                let temp_dir = TempDir::new().unwrap();
                let file_path = temp_dir.path().join("test.bin");
                let writer = BufferedFileWriter::with_capacity(DEFAULT_WRITE_BUFFER_SIZE);
                rt.block_on(async {
                    writer
                        .write_all(&file_path, black_box(&data))
                        .await
                        .unwrap();
                });
            });
        });
    }

    group.finish();
}

/// Benchmark: Read-ahead vs standard reads
fn bench_read_ahead(c: &mut Criterion) {
    let mut group = c.benchmark_group("io_read_ahead");

    for size_mb in [1, 10, 50] {
        let (_temp_dir, file_path) = create_test_file(size_mb);

        // Standard read
        group.bench_with_input(BenchmarkId::new("standard", size_mb), &size_mb, |b, _| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            b.iter(|| {
                rt.block_on(async {
                    let data = fs::read(black_box(&file_path)).await.unwrap();
                    black_box(data);
                });
            });
        });

        // Read-ahead
        group.bench_with_input(BenchmarkId::new("read_ahead", size_mb), &size_mb, |b, _| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            b.iter(|| {
                rt.block_on(async {
                    let mut reader =
                        ReadAheadReader::new(black_box(&file_path), DEFAULT_READ_AHEAD_SIZE)
                            .await
                            .unwrap();
                    let data = reader.read_to_end().await.unwrap();
                    black_box(data);
                });
            });
        });
    }

    group.finish();
}

/// Benchmark: Sequential vs random reads
fn bench_read_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("io_read_patterns");

    let (_temp_dir, file_path) = create_test_file(10);
    let file_size = std::fs::metadata(&file_path).unwrap().len() as usize;

    // Sequential reads
    group.bench_function("sequential", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let mut file = fs::File::open(&file_path).await.unwrap();
                let mut buffer = vec![0u8; 4096];
                let mut total_read = 0;
                while total_read < file_size {
                    let bytes_read = tokio::io::AsyncReadExt::read(&mut file, &mut buffer)
                        .await
                        .unwrap();
                    if bytes_read == 0 {
                        break;
                    }
                    total_read += bytes_read;
                    black_box(&buffer[..bytes_read]);
                }
            });
        });
    });

    // Random reads
    group.bench_function("random", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        use rand::Rng;
        let mut rng = rand::thread_rng();
        b.iter(|| {
            rt.block_on(async {
                let mut file = fs::File::open(&file_path).await.unwrap();
                for _ in 0..100 {
                    let offset = rng.gen_range(0..file_size.saturating_sub(4096));
                    let _ = tokio::io::AsyncSeekExt::seek(
                        &mut file,
                        std::io::SeekFrom::Start(offset as u64),
                    )
                    .await;
                    let mut buffer = vec![0u8; 4096];
                    let _ = tokio::io::AsyncReadExt::read(&mut file, &mut buffer).await;
                    black_box(&buffer);
                }
            });
        });
    });

    group.finish();
}

/// Benchmark: Memory-mapped vs standard index access
fn bench_memory_mapped_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("io_memory_mapped_index");

    let temp_dir = TempDir::new().unwrap();
    let manager = IndexManager::new(temp_dir.path());

    let (schema, _) = SchemaBuilder::new()
        .add_text_field("title")
        .add_text_field("content")
        .build()
        .unwrap();

    // Create index with memory-mapped storage
    let rt = tokio::runtime::Runtime::new().unwrap();
    let index_mmap = rt.block_on(async {
        let settings = IndexSettings::default().with_memory_mapped_storage(true);
        manager
            .create_index("mmap_index", schema.clone(), settings)
            .await
            .unwrap()
    });

    // Create index without memory-mapped storage
    let index_standard = rt.block_on(async {
        let settings = IndexSettings::default().with_memory_mapped_storage(false);
        manager
            .create_index("standard_index", schema, settings)
            .await
            .unwrap()
    });

    // Benchmark search operations
    group.bench_function("memory_mapped_search", |b| {
        b.iter(|| {
            rt.block_on(async {
                let reader = index_mmap.reader().unwrap();
                let searcher = reader.searcher();
                black_box(searcher.num_docs());
            });
        });
    });

    group.bench_function("standard_search", |b| {
        b.iter(|| {
            rt.block_on(async {
                let reader = index_standard.reader().unwrap();
                let searcher = reader.searcher();
                black_box(searcher.num_docs());
            });
        });
    });

    group.finish();
}

/// Benchmark: Write buffer size impact
fn bench_write_buffer_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("io_write_buffer_sizes");

    let data = vec![0u8; 10 * 1024 * 1024]; // 10MB

    for buffer_size_kb in [4, 16, 64, 256, 1024] {
        group.bench_with_input(
            BenchmarkId::new("buffer_size", buffer_size_kb),
            &buffer_size_kb,
            |b, &buffer_size_kb| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let file_path = temp_dir.path().join("test.bin");
                    let writer = BufferedFileWriter::with_capacity(buffer_size_kb * 1024);
                    rt.block_on(async {
                        writer
                            .write_all(&file_path, black_box(&data))
                            .await
                            .unwrap();
                    });
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_buffered_writes,
    bench_read_ahead,
    bench_read_patterns,
    bench_memory_mapped_index,
    bench_write_buffer_sizes
);
criterion_main!(benches);
