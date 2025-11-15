//! Stored field compression optimization
//!
//! This module provides compression optimization for stored fields
//! to reduce storage space and improve I/O performance.

use crate::error::{Error, Result};
use crate::snapshot::compression::{CompressionConfig, CompressionType, SnapshotCompressor};
use serde_json::{Value as JsonValue, json};

/// Configuration for stored field compression
#[derive(Debug, Clone)]
pub struct StoredFieldCompressionConfig {
    /// Minimum field size (bytes) to compress
    pub min_size_to_compress: usize,
    /// Compression algorithm to use
    pub compression_type: CompressionType,
    /// Compression level
    pub compression_level: u8,
    /// Fields to always compress (by name)
    pub always_compress: Vec<String>,
    /// Fields to never compress (by name)
    pub never_compress: Vec<String>,
}

impl Default for StoredFieldCompressionConfig {
    fn default() -> Self {
        Self {
            min_size_to_compress: 100, // Compress fields > 100 bytes
            compression_type: CompressionType::Zstd,
            compression_level: 3, // Balanced compression/speed
            always_compress: vec![
                "content".to_string(),
                "body".to_string(),
                "description".to_string(),
            ],
            never_compress: vec!["_id".to_string(), "id".to_string()],
        }
    }
}

/// Compressor for stored fields
pub struct StoredFieldCompressor {
    config: StoredFieldCompressionConfig,
    compressor: SnapshotCompressor,
}

impl StoredFieldCompressor {
    /// Create a new stored field compressor with default config
    pub fn new() -> Self {
        Self::with_config(StoredFieldCompressionConfig::default())
    }

    /// Create a new stored field compressor with custom config
    pub fn with_config(config: StoredFieldCompressionConfig) -> Self {
        let compression_config = CompressionConfig {
            algorithm: config.compression_type,
            level: config.compression_level,
            use_dictionary: false,
            dictionary_size: 0,
        };

        Self {
            config,
            compressor: SnapshotCompressor::new(compression_config),
        }
    }

    /// Compress stored fields in a document
    ///
    /// This optimizes large stored fields by compressing them,
    /// while leaving small fields and metadata uncompressed.
    pub fn compress_document(&self, document: &JsonValue) -> Result<JsonValue> {
        match document {
            JsonValue::Object(obj) => {
                let mut compressed = serde_json::Map::new();

                for (key, value) in obj {
                    if self.should_compress_field(key, value) {
                        match self.compress_field_value(value) {
                            Ok(compressed_bytes) => {
                                // Store compressed value with metadata
                                // Convert bytes to JSON array for storage
                                let data_array: Vec<u64> =
                                    compressed_bytes.iter().map(|&b| u64::from(b)).collect();
                                compressed.insert(
                                    key.clone(),
                                    json!({
                                        "_compressed": true,
                                        "_algorithm": format!("{:?}", self.config.compression_type),
                                        "_data": data_array
                                    }),
                                );
                            }
                            Err(e) => {
                                // If compression fails, store original
                                tracing::warn!(field = %key, error = %e, "Failed to compress field, storing original");
                                compressed.insert(key.clone(), value.clone());
                            }
                        }
                    } else {
                        // Don't compress, store as-is
                        compressed.insert(key.clone(), value.clone());
                    }
                }

                Ok(JsonValue::Object(compressed))
            }
            _ => Ok(document.clone()),
        }
    }

    /// Decompress stored fields in a document
    pub fn decompress_document(&self, document: &JsonValue) -> Result<JsonValue> {
        match document {
            JsonValue::Object(obj) => {
                let mut decompressed = serde_json::Map::new();

                for (key, value) in obj {
                    if Self::is_compressed_field(value) {
                        match self.decompress_field_value(value) {
                            Ok(decompressed_value) => {
                                decompressed.insert(key.clone(), decompressed_value);
                            }
                            Err(e) => {
                                return Err(Error::Compression(format!(
                                    "Failed to decompress field {key}: {e}"
                                )));
                            }
                        }
                    } else {
                        decompressed.insert(key.clone(), value.clone());
                    }
                }

                Ok(JsonValue::Object(decompressed))
            }
            _ => Ok(document.clone()),
        }
    }

    /// Check if a field should be compressed
    fn should_compress_field(&self, field_name: &str, value: &JsonValue) -> bool {
        // Never compress fields in never_compress list
        if self.config.never_compress.iter().any(|f| f == field_name) {
            return false;
        }

        // Always compress fields in always_compress list
        if self.config.always_compress.iter().any(|f| f == field_name) {
            return true;
        }

        // Compress if field size exceeds threshold
        let field_size = self.estimate_field_size(value);
        field_size >= self.config.min_size_to_compress
    }

    /// Check if a field value is compressed
    fn is_compressed_field(value: &JsonValue) -> bool {
        if let Some(obj) = value.as_object() {
            obj.contains_key("_compressed")
                && obj.get("_compressed").and_then(|v| v.as_bool()) == Some(true)
        } else {
            false
        }
    }

    /// Estimate field size in bytes
    fn estimate_field_size(&self, value: &JsonValue) -> usize {
        match value {
            JsonValue::String(s) => s.len(),
            JsonValue::Array(arr) => arr.iter().map(|v| self.estimate_field_size(v)).sum(),
            JsonValue::Object(obj) => {
                obj.values()
                    .map(|v| self.estimate_field_size(v))
                    .sum::<usize>()
                    + obj.len() * 10 // Rough estimate for keys
            }
            _ => {
                // For numbers, booleans, null - estimate as small
                8
            }
        }
    }

    /// Compress a field value
    fn compress_field_value(&self, value: &JsonValue) -> Result<Vec<u8>> {
        let json_bytes = serde_json::to_vec(value)?;
        self.compressor.compress(&json_bytes)
    }

    /// Decompress a field value
    fn decompress_field_value(&self, value: &JsonValue) -> Result<JsonValue> {
        if let Some(obj) = value.as_object() {
            if let Some(data) = obj.get("_data") {
                // Data is stored as array of numbers (bytes)
                if let Some(arr) = data.as_array() {
                    let compressed: std::result::Result<Vec<u8>, Error> = arr
                        .iter()
                        .map(|v| {
                            v.as_u64().map(|n| n as u8).ok_or_else(|| {
                                Error::Compression("Invalid byte in compressed data".to_string())
                            })
                        })
                        .collect();
                    let compressed = compressed?;
                    let decompressed = self.compressor.decompress(&compressed)?;
                    let json_value: JsonValue = serde_json::from_slice(&decompressed)?;
                    Ok(json_value)
                } else {
                    Err(Error::Compression(
                        "Compressed data is not an array".to_string(),
                    ))
                }
            } else {
                Err(Error::Compression(
                    "Missing _data field in compressed value".to_string(),
                ))
            }
        } else {
            Err(Error::Compression(
                "Compressed value is not an object".to_string(),
            ))
        }
    }

    /// Get compression statistics for a document
    pub fn compression_stats(
        &self,
        original: &JsonValue,
        compressed: &JsonValue,
    ) -> CompressionStats {
        let original_size = self.estimate_document_size(original);
        let compressed_size = self.estimate_document_size(compressed);
        let savings = original_size.saturating_sub(compressed_size);
        let savings_percent = if original_size > 0 {
            (savings as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };

        CompressionStats {
            original_size,
            compressed_size,
            savings,
            savings_percent,
            compression_ratio: if original_size > 0 {
                compressed_size as f64 / original_size as f64
            } else {
                0.0
            },
        }
    }

    /// Estimate total document size
    fn estimate_document_size(&self, document: &JsonValue) -> usize {
        match document {
            JsonValue::Object(obj) => {
                obj.values()
                    .map(|v| self.estimate_field_size(v))
                    .sum::<usize>()
                    + obj.keys().map(|k| k.len()).sum::<usize>()
            }
            _ => self.estimate_field_size(document),
        }
    }
}

impl Default for StoredFieldCompressor {
    fn default() -> Self {
        Self::new()
    }
}

/// Compression statistics
#[derive(Debug, Clone)]
pub struct CompressionStats {
    /// Original size in bytes
    pub original_size: usize,
    /// Compressed size in bytes
    pub compressed_size: usize,
    /// Space saved in bytes
    pub savings: usize,
    /// Space saved as percentage
    pub savings_percent: f64,
    /// Compression ratio (compressed/original)
    pub compression_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_should_compress_large_field() {
        let compressor = StoredFieldCompressor::new();
        let large_value = json!("x".repeat(200)); // 200 bytes

        assert!(compressor.should_compress_field("content", &large_value));
    }

    #[test]
    fn test_should_not_compress_small_field() {
        let compressor = StoredFieldCompressor::new();
        let small_value = json!("small");

        assert!(!compressor.should_compress_field("title", &small_value));
    }

    #[test]
    fn test_should_not_compress_id_field() {
        let compressor = StoredFieldCompressor::new();
        let large_value = json!("x".repeat(200));

        assert!(!compressor.should_compress_field("_id", &large_value));
    }

    #[test]
    fn test_compress_decompress_document() {
        let compressor = StoredFieldCompressor::new();
        let document = json!({
            "title": "Small field",
            "content": "x".repeat(500),
            "_id": "doc123"
        });

        let compressed = compressor.compress_document(&document).unwrap();
        let decompressed = compressor.decompress_document(&compressed).unwrap();

        assert_eq!(document, decompressed);
    }

    #[test]
    fn test_compression_stats() {
        let compressor = StoredFieldCompressor::new();
        let document = json!({
            "content": "x".repeat(1000)
        });

        let compressed = compressor.compress_document(&document).unwrap();
        let stats = compressor.compression_stats(&document, &compressed);

        assert!(stats.compressed_size < stats.original_size);
        assert!(stats.savings_percent > 0.0);
    }
}
