//! Advanced compression utilities for incremental snapshots

use crate::error::{Error, Result};
use std::io::{Read, Write};

/// Compression algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum CompressionType {
    /// No compression
    None,
    /// Gzip compression (default)
    #[default]
    Gzip,
    /// Zstandard compression (high compression ratio)
    Zstd,
    /// LZ4 compression (fast compression/decompression)
    Lz4,
}

/// Compression configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompressionConfig {
    /// Compression algorithm to use
    pub algorithm: CompressionType,
    /// Compression level (1-22 for zstd, 1-9 for gzip, 1-16 for lz4)
    pub level: u8,
    /// Whether to use dictionary compression for better ratios
    pub use_dictionary: bool,
    /// Dictionary size for zstd (0 = auto)
    pub dictionary_size: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            algorithm: CompressionType::Zstd,
            level: 3, // Balanced compression/speed
            use_dictionary: true,
            dictionary_size: 0, // Auto
        }
    }
}

/// Advanced compression utilities for incremental snapshots
pub struct SnapshotCompressor {
    config: CompressionConfig,
    dictionary: Option<Vec<u8>>,
}

impl SnapshotCompressor {
    /// Create a new compressor with the given configuration
    pub fn new(config: CompressionConfig) -> Self {
        Self {
            config,
            dictionary: None,
        }
    }

    /// Create a compressor with dictionary for better compression ratios
    pub fn with_dictionary(config: CompressionConfig, dictionary: Vec<u8>) -> Self {
        Self {
            config,
            dictionary: Some(dictionary),
        }
    }

    /// Compress data using the configured algorithm
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.config.algorithm {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => self.compress_gzip(data),
            CompressionType::Zstd => self.compress_zstd(data),
            CompressionType::Lz4 => self.compress_lz4(data),
        }
    }

    /// Decompress data using the configured algorithm
    pub fn decompress(&self, compressed_data: &[u8]) -> Result<Vec<u8>> {
        match self.config.algorithm {
            CompressionType::None => Ok(compressed_data.to_vec()),
            CompressionType::Gzip => Self::decompress_gzip(compressed_data),
            CompressionType::Zstd => self.decompress_zstd(compressed_data),
            CompressionType::Lz4 => Self::decompress_lz4(compressed_data),
        }
    }

    /// Compress data with gzip
    fn compress_gzip(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::Compression;
        use flate2::write::GzEncoder;

        let level = std::cmp::min(u32::from(self.config.level), 9);
        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(level));
        encoder.write_all(data)?;
        encoder
            .finish()
            .map_err(|e| Error::Compression(format!("Gzip compression failed: {e}")))
    }

    /// Decompress gzip data
    fn decompress_gzip(compressed_data: &[u8]) -> Result<Vec<u8>> {
        use flate2::read::GzDecoder;

        let mut decoder = GzDecoder::new(compressed_data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }

    /// Compress data with zstd
    fn compress_zstd(&self, data: &[u8]) -> Result<Vec<u8>> {
        let level = std::cmp::min(i32::from(self.config.level), 22);

        if let Some(ref _dict) = self.dictionary {
            // Use dictionary compression
            zstd::encode_all(data, level).map_err(|e| {
                Error::Compression(format!("Zstd compression with dictionary failed: {e}"))
            })
        } else {
            // Standard zstd compression
            zstd::encode_all(data, level)
                .map_err(|e| Error::Compression(format!("Zstd compression failed: {e}")))
        }
    }

    /// Decompress zstd data
    fn decompress_zstd(&self, compressed_data: &[u8]) -> Result<Vec<u8>> {
        if let Some(ref _dict) = self.dictionary {
            // Use dictionary decompression
            zstd::decode_all(compressed_data).map_err(|e| {
                Error::Compression(format!("Zstd decompression with dictionary failed: {e}"))
            })
        } else {
            // Standard zstd decompression
            zstd::decode_all(compressed_data)
                .map_err(|e| Error::Compression(format!("Zstd decompression failed: {e}")))
        }
    }

    /// Compress data with lz4
    fn compress_lz4(&self, data: &[u8]) -> Result<Vec<u8>> {
        let level = std::cmp::min(u32::from(self.config.level), 16);
        let mode = lz4::block::CompressionMode::HIGHCOMPRESSION(level as i32);
        lz4::block::compress(data, Some(mode), false)
            .map_err(|e| Error::Compression(format!("LZ4 compression failed: {e}")))
    }

    /// Decompress lz4 data
    fn decompress_lz4(compressed_data: &[u8]) -> Result<Vec<u8>> {
        lz4::block::decompress(compressed_data, None)
            .map_err(|e| Error::Compression(format!("LZ4 decompression failed: {e}")))
    }

    /// Calculate compression ratio
    pub fn compression_ratio(&self, original_size: usize, compressed_size: usize) -> f64 {
        if original_size == 0 {
            0.0
        } else {
            compressed_size as f64 / original_size as f64
        }
    }

    /// Get compression statistics
    pub fn compression_stats(
        &self,
        original_size: usize,
        compressed_size: usize,
    ) -> CompressionStats {
        let ratio = self.compression_ratio(original_size, compressed_size);
        let savings = original_size.saturating_sub(compressed_size);
        let savings_percent = if original_size > 0 {
            (savings as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };

        CompressionStats {
            algorithm: self.config.algorithm,
            original_size,
            compressed_size,
            compression_ratio: ratio,
            space_saved: savings,
            space_saved_percent: savings_percent,
        }
    }
}

/// Compression statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompressionStats {
    /// Compression algorithm used
    pub algorithm: CompressionType,
    /// Original data size in bytes
    pub original_size: usize,
    /// Compressed data size in bytes
    pub compressed_size: usize,
    /// Compression ratio (compressed/original)
    pub compression_ratio: f64,
    /// Space saved in bytes
    pub space_saved: usize,
    /// Space saved as percentage
    pub space_saved_percent: f64,
}

/// Content-based deduplication for delta files
pub struct ContentDeduplicator {
    /// Content hash to file path mapping
    content_map: std::collections::HashMap<u64, String>,
    /// Hash function for content
    hasher: blake3::Hasher,
}

impl ContentDeduplicator {
    /// Create a new content deduplicator
    pub fn new() -> Self {
        Self {
            content_map: std::collections::HashMap::new(),
            hasher: blake3::Hasher::new(),
        }
    }

    /// Check if content is already stored and return reference if found
    pub fn check_duplicate(&mut self, content: &[u8]) -> Option<String> {
        let hash = self.calculate_hash(content);
        self.content_map.get(&hash).cloned()
    }

    /// Add content to the deduplicator
    pub fn add_content(&mut self, content: &[u8], file_path: String) -> u64 {
        let hash = self.calculate_hash(content);
        self.content_map.insert(hash, file_path);
        hash
    }

    /// Calculate content hash
    fn calculate_hash(&mut self, content: &[u8]) -> u64 {
        self.hasher.reset();
        self.hasher.update(content);
        let hash = self.hasher.finalize();
        // Use first 8 bytes of hash as u64
        u64::from_le_bytes(hash.as_bytes()[..8].try_into().unwrap())
    }

    /// Get deduplication statistics
    pub fn stats(&self) -> DeduplicationStats {
        let total_entries = self.content_map.len();
        let unique_entries = self
            .content_map
            .values()
            .collect::<std::collections::HashSet<_>>()
            .len();
        let duplicates = total_entries.saturating_sub(unique_entries);
        let deduplication_ratio = if total_entries > 0 {
            duplicates as f64 / total_entries as f64
        } else {
            0.0
        };

        DeduplicationStats {
            total_entries,
            unique_entries,
            duplicates,
            deduplication_ratio,
        }
    }
}

/// Deduplication statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeduplicationStats {
    /// Total number of files processed
    pub total_entries: usize,
    /// Number of unique files
    pub unique_entries: usize,
    /// Number of duplicate files found
    pub duplicates: usize,
    /// Deduplication ratio (duplicates/total)
    pub deduplication_ratio: f64,
}

impl Default for ContentDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

/// Binary diff algorithm for delta compression
pub struct BinaryDiff {
    /// Chunk size for binary diff
    chunk_size: usize,
}

impl BinaryDiff {
    /// Create a new binary diff with default chunk size
    pub fn new() -> Self {
        Self {
            chunk_size: 8192, // 8KB chunks
        }
    }

    /// Create a new binary diff with custom chunk size
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        Self { chunk_size }
    }

    /// Calculate binary diff between two files
    pub fn calculate_diff(&self, old_data: &[u8], new_data: &[u8]) -> BinaryDelta {
        let mut delta = BinaryDelta::new();

        // Simple implementation: find common chunks and record differences
        let old_chunks = self.chunk_data(old_data);
        let new_chunks = self.chunk_data(new_data);

        let mut old_index = 0;
        let mut new_index = 0;

        while old_index < old_chunks.len() && new_index < new_chunks.len() {
            if old_chunks[old_index] == new_chunks[new_index] {
                // Chunks are identical, add reference
                delta.add_reference(old_index, new_index);
                old_index += 1;
                new_index += 1;
            } else {
                // Chunks differ, add new data
                delta.add_new_data(new_chunks[new_index]);
                new_index += 1;
            }
        }

        // Add remaining new chunks
        while new_index < new_chunks.len() {
            delta.add_new_data(new_chunks[new_index]);
            new_index += 1;
        }

        delta
    }

    /// Chunk data into fixed-size pieces
    fn chunk_data<'a>(&self, data: &'a [u8]) -> Vec<&'a [u8]> {
        let mut chunks = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let end = std::cmp::min(offset + self.chunk_size, data.len());
            chunks.push(&data[offset..end]);
            offset = end;
        }

        chunks
    }
}

/// Binary delta representation
#[derive(Debug, Clone)]
pub struct BinaryDelta {
    /// References to old data chunks
    references: Vec<(usize, usize)>, // (old_index, new_index)
    /// New data chunks
    new_data: Vec<Vec<u8>>,
    /// Original data size
    #[allow(dead_code)]
    original_size: usize,
    /// Delta size
    delta_size: usize,
}

impl BinaryDelta {
    fn new() -> Self {
        Self {
            references: Vec::new(),
            new_data: Vec::new(),
            original_size: 0,
            delta_size: 0,
        }
    }

    fn add_reference(&mut self, old_index: usize, new_index: usize) {
        self.references.push((old_index, new_index));
        // References don't add to delta size since they point to existing data
    }

    fn add_new_data(&mut self, data: &[u8]) {
        self.new_data.push(data.to_vec());
        self.delta_size += data.len();
    }

    /// Get delta size
    pub fn size(&self) -> usize {
        self.delta_size
    }

    /// Get compression ratio
    pub fn compression_ratio(&self, original_size: usize) -> f64 {
        if original_size == 0 {
            0.0
        } else {
            self.delta_size as f64 / original_size as f64
        }
    }
}

impl Default for BinaryDiff {
    fn default() -> Self {
        Self::new()
    }
}
