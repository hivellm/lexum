//! IP range aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::{Error, Result};
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use utoipa::ToSchema;

/// IP range definition
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum IpRange {
    /// Range with CIDR notation
    Cidr {
        /// CIDR notation (e.g., "192.168.1.0/24")
        mask: String,
        /// Custom key for this range
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
    },
    /// Range with from and to IPs
    FromTo {
        /// Lower bound IP (inclusive)
        #[serde(skip_serializing_if = "Option::is_none")]
        from: Option<String>,
        /// Upper bound IP (exclusive)
        #[serde(skip_serializing_if = "Option::is_none")]
        to: Option<String>,
        /// Custom key for this range
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
    },
    /// Simple range with just a key
    KeyOnly {
        /// Custom key
        key: String,
        /// Lower bound IP (inclusive)
        #[serde(skip_serializing_if = "Option::is_none")]
        from: Option<String>,
        /// Upper bound IP (exclusive)
        #[serde(skip_serializing_if = "Option::is_none")]
        to: Option<String>,
    },
}

impl IpRange {
    /// Get the key, or generate one
    fn key(&self) -> String {
        match self {
            IpRange::Cidr { key, mask } => key.clone().unwrap_or_else(|| mask.clone()),
            IpRange::FromTo { key, from, to } => {
                if let Some(k) = key {
                    k.clone()
                } else {
                    format!(
                        "{from_str}-{to_str}",
                        from_str = from.as_deref().unwrap_or("*"),
                        to_str = to.as_deref().unwrap_or("*")
                    )
                }
            }
            IpRange::KeyOnly { key, .. } => key.clone(),
        }
    }

    /// Check if an IP address matches this range
    fn matches(&self, ip: IpAddr) -> bool {
        match self {
            IpRange::Cidr { mask, .. } => {
                // Parse CIDR notation
                if let Ok((network_ip, prefix_len)) = parse_cidr(mask) {
                    match (ip, network_ip) {
                        (IpAddr::V4(ipv4), IpAddr::V4(net_ipv4)) => {
                            matches_ipv4_cidr(ipv4, net_ipv4, prefix_len)
                        }
                        (IpAddr::V6(ipv6), IpAddr::V6(net_ipv6)) => {
                            matches_ipv6_cidr(ipv6, net_ipv6, prefix_len)
                        }
                        _ => false, // IPv4/IPv6 mismatch
                    }
                } else {
                    false
                }
            }
            IpRange::FromTo { from, to, .. } | IpRange::KeyOnly { from, to, .. } => {
                let from_ip = from.as_ref().and_then(|f| f.parse::<IpAddr>().ok());
                let to_ip = to.as_ref().and_then(|t| t.parse::<IpAddr>().ok());

                match (from_ip, to_ip) {
                    (Some(from_addr), Some(to_addr)) => ip >= from_addr && ip < to_addr,
                    (Some(from_addr), None) => ip >= from_addr,
                    (None, Some(to_addr)) => ip < to_addr,
                    (None, None) => true, // Open range matches all
                }
            }
        }
    }
}

/// IP range aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IpRangeAggregation {
    /// Field to aggregate on
    pub field: String,
    /// IP ranges to create buckets for
    pub ranges: Vec<IpRange>,
    /// Return keyed response (key: bucket) instead of array
    #[serde(default)]
    pub keyed: bool,
}

impl AggregationTrait for IpRangeAggregation {
    fn name(&self) -> &str {
        "ip_range"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut bucket_counts: HashMap<String, usize> = HashMap::new();

        // Initialize all ranges with 0 count
        for range in &self.ranges {
            let key = range.key();
            bucket_counts.insert(key, 0);
        }

        // Process each hit
        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(ip) = parse_ip_from_value(field_value) {
                    // Check which range this IP belongs to
                    for range in &self.ranges {
                        if range.matches(ip) {
                            let key = range.key();
                            *bucket_counts.entry(key).or_insert(0) += 1;
                            break; // Only count in first matching range
                        }
                    }
                }
            }
        }

        // Convert to buckets, preserving range order
        let mut bucket_vec: Vec<Bucket> = Vec::new();
        for range in &self.ranges {
            let key = range.key();
            let count = bucket_counts.get(&key).copied().unwrap_or(0);
            bucket_vec.push(Bucket::new(JsonValue::String(key), count));
        }

        if self.keyed {
            // Return keyed format
            let mut keyed_map = HashMap::new();
            for bucket in bucket_vec {
                if let JsonValue::String(key) = &bucket.key {
                    keyed_map.insert(key.clone(), bucket);
                }
            }
            Ok(AggregationResult::Buckets(
                BucketAggregationResult::new_keyed(keyed_map),
            ))
        } else {
            Ok(AggregationResult::Buckets(BucketAggregationResult::new(
                bucket_vec,
            )))
        }
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut merged_counts: HashMap<String, usize> = HashMap::new();

        // Initialize all ranges
        for range in &self.ranges {
            let key = range.key();
            merged_counts.insert(key, 0);
        }

        // Merge results from all shards
        for result in results {
            if let AggregationResult::Buckets(bucket_result) = result {
                for bucket in bucket_result.buckets() {
                    if let JsonValue::String(key) = &bucket.key {
                        *merged_counts.entry(key.clone()).or_insert(0) += bucket.doc_count;
                    }
                }
            }
        }

        // Convert to buckets, preserving range order
        let mut bucket_vec: Vec<Bucket> = Vec::new();
        for range in &self.ranges {
            let key = range.key();
            let count = merged_counts.get(&key).copied().unwrap_or(0);
            bucket_vec.push(Bucket::new(JsonValue::String(key), count));
        }

        if self.keyed {
            // Return keyed format
            let mut keyed_map = HashMap::new();
            for bucket in bucket_vec {
                if let JsonValue::String(key) = &bucket.key {
                    keyed_map.insert(key.clone(), bucket);
                }
            }
            Ok(AggregationResult::Buckets(
                BucketAggregationResult::new_keyed(keyed_map),
            ))
        } else {
            Ok(AggregationResult::Buckets(BucketAggregationResult::new(
                bucket_vec,
            )))
        }
    }
}

impl IpRangeAggregation {
    /// Create new IP range aggregation
    pub fn new(field: impl Into<String>, ranges: Vec<IpRange>) -> Self {
        Self {
            field: field.into(),
            ranges,
            keyed: false,
        }
    }

    /// Set keyed response
    pub fn with_keyed(mut self, keyed: bool) -> Self {
        self.keyed = keyed;
        self
    }
}

/// Parse CIDR notation (e.g., "192.168.1.0/24")
fn parse_cidr(cidr: &str) -> Result<(IpAddr, u8)> {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return Err(Error::Config(format!("Invalid CIDR notation: {cidr}")));
    }

    let ip_str = parts[0];
    let prefix_str = parts[1];

    let ip: IpAddr = ip_str
        .parse()
        .map_err(|_| Error::Config(format!("Invalid IP address: {ip_str}")))?;

    let prefix: u8 = prefix_str
        .parse()
        .map_err(|_| Error::Config(format!("Invalid prefix length: {prefix_str}")))?;

    // Validate prefix length
    let max_prefix = match ip {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    };

    if prefix > max_prefix {
        return Err(Error::Config(format!(
            "Prefix length {prefix} exceeds maximum {max_prefix}"
        )));
    }

    Ok((ip, prefix))
}

/// Check if IPv4 address matches CIDR
fn matches_ipv4_cidr(ip: Ipv4Addr, network: Ipv4Addr, prefix_len: u8) -> bool {
    if prefix_len == 0 {
        return true; // /0 matches all
    }

    let ip_u32 = u32::from(ip);
    let network_u32 = u32::from(network);
    let mask = if prefix_len == 32 {
        0xFFFFFFFFu32
    } else {
        !((1u32 << (32 - prefix_len)) - 1)
    };

    (ip_u32 & mask) == (network_u32 & mask)
}

/// Check if IPv6 address matches CIDR
fn matches_ipv6_cidr(ip: Ipv6Addr, network: Ipv6Addr, prefix_len: u8) -> bool {
    if prefix_len == 0 {
        return true; // /0 matches all
    }

    let ip_u128 = u128::from(ip);
    let network_u128 = u128::from(network);
    let mask = if prefix_len == 128 {
        0xFFFFFFFF_FFFFFFFF_FFFFFFFF_FFFFFFFFu128
    } else {
        !((1u128 << (128 - prefix_len)) - 1)
    };

    (ip_u128 & mask) == (network_u128 & mask)
}

/// Parse IP address from JSON value
fn parse_ip_from_value(value: &JsonValue) -> Option<IpAddr> {
    if let Some(s) = value.as_str() {
        s.parse::<IpAddr>().ok()
    } else if let Some(num) = value.as_u64() {
        // Try to parse as IPv4 (32-bit number)
        if num <= u32::MAX as u64 {
            Some(IpAddr::V4(Ipv4Addr::from(num as u32)))
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::field_cache::FieldCache;
    use crate::search::result::SearchHit;
    use crate::types::{DocumentId, Score};

    fn create_test_hit_ip(id: &str, field: &str, ip_str: &str) -> SearchHit {
        SearchHit {
            id: DocumentId::new(id),
            score: Score::new(1.0),
            source: serde_json::json!({ field: ip_str }),
        }
    }

    #[test]
    fn test_ip_range_aggregation_cidr() {
        let ranges = vec![
            IpRange::Cidr {
                mask: "192.168.1.0/24".to_string(),
                key: None,
            },
            IpRange::Cidr {
                mask: "10.0.0.0/8".to_string(),
                key: None,
            },
        ];

        let agg = IpRangeAggregation::new("ip", ranges);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_ip("1", "ip", "192.168.1.10"),
            create_test_hit_ip("2", "ip", "192.168.1.20"),
            create_test_hit_ip("3", "ip", "10.0.0.5"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 2);
            let buckets = bucket_result.buckets();
            assert_eq!(buckets[0].doc_count, 2); // 192.168.1.0/24
            assert_eq!(buckets[1].doc_count, 1); // 10.0.0.0/8
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_ip_range_aggregation_from_to() {
        let ranges = vec![
            IpRange::FromTo {
                from: Some("192.168.1.0".to_string()),
                to: Some("192.168.1.100".to_string()),
                key: None,
            },
            IpRange::FromTo {
                from: Some("192.168.1.100".to_string()),
                to: Some("192.168.1.200".to_string()),
                key: None,
            },
        ];

        let agg = IpRangeAggregation::new("ip", ranges);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_ip("1", "ip", "192.168.1.50"),
            create_test_hit_ip("2", "ip", "192.168.1.150"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 2);
            let buckets = bucket_result.buckets();
            assert_eq!(buckets[0].doc_count, 1);
            assert_eq!(buckets[1].doc_count, 1);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_ip_range_aggregation_ipv6() {
        let ranges = vec![IpRange::Cidr {
            mask: "2001:db8::/32".to_string(),
            key: None,
        }];

        let agg = IpRangeAggregation::new("ip", ranges);
        let field_cache = FieldCache::new();

        let hits = vec![create_test_hit_ip("1", "ip", "2001:db8::1")];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 1);
            assert_eq!(bucket_result.buckets()[0].doc_count, 1);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_ip_range_aggregation_keyed() {
        let ranges = vec![IpRange::Cidr {
            mask: "192.168.1.0/24".to_string(),
            key: Some("private_network".to_string()),
        }];

        let agg = IpRangeAggregation::new("ip", ranges).with_keyed(true);
        let field_cache = FieldCache::new();

        let hits = vec![create_test_hit_ip("1", "ip", "192.168.1.10")];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert!(bucket_result.is_keyed());
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_ip_range_aggregation_merge() {
        let ranges = vec![IpRange::Cidr {
            mask: "192.168.1.0/24".to_string(),
            key: None,
        }];

        let agg = IpRangeAggregation::new("ip", ranges);
        let field_cache = FieldCache::new();

        let hits1 = vec![create_test_hit_ip("1", "ip", "192.168.1.10")];
        let hits2 = vec![create_test_hit_ip("2", "ip", "192.168.1.20")];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            assert_eq!(bucket_result.buckets().len(), 1);
            assert_eq!(bucket_result.buckets()[0].doc_count, 2);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_ip_range_aggregation_empty_hits() {
        let ranges = vec![IpRange::Cidr {
            mask: "192.168.1.0/24".to_string(),
            key: None,
        }];

        let agg = IpRangeAggregation::new("ip", ranges);
        let field_cache = FieldCache::new();
        let hits = vec![];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 1);
            assert_eq!(bucket_result.buckets()[0].doc_count, 0);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_ip_range_aggregation_missing_field() {
        let ranges = vec![IpRange::Cidr {
            mask: "192.168.1.0/24".to_string(),
            key: None,
        }];

        let agg = IpRangeAggregation::new("ip", ranges);
        let field_cache = FieldCache::new();

        let hits = vec![SearchHit {
            id: DocumentId::new("1"),
            score: Score::new(1.0),
            source: serde_json::json!({ "other_field": "value" }),
        }];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 1);
            assert_eq!(bucket_result.buckets()[0].doc_count, 0);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_parse_cidr() {
        // Valid IPv4 CIDR
        assert!(parse_cidr("192.168.1.0/24").is_ok());
        assert!(parse_cidr("10.0.0.0/8").is_ok());
        assert!(parse_cidr("0.0.0.0/0").is_ok()); // /0 matches all

        // Valid IPv6 CIDR
        assert!(parse_cidr("2001:db8::/32").is_ok());
        assert!(parse_cidr("::/0").is_ok()); // /0 matches all

        // Invalid CIDR
        assert!(parse_cidr("192.168.1.0").is_err()); // Missing prefix
        assert!(parse_cidr("192.168.1.0/33").is_err()); // Prefix too large for IPv4
        assert!(parse_cidr("2001:db8::/129").is_err()); // Prefix too large for IPv6
        assert!(parse_cidr("invalid/24").is_err()); // Invalid IP
    }

    #[test]
    fn test_matches_ipv4_cidr() {
        let network: Ipv4Addr = "192.168.1.0".parse().unwrap();
        let ip1: Ipv4Addr = "192.168.1.10".parse().unwrap();
        let ip2: Ipv4Addr = "192.168.2.10".parse().unwrap();

        assert!(matches_ipv4_cidr(ip1, network, 24));
        assert!(!matches_ipv4_cidr(ip2, network, 24));
    }

    #[test]
    fn test_matches_ipv6_cidr() {
        let network: Ipv6Addr = "2001:db8::".parse().unwrap();
        let ip1: Ipv6Addr = "2001:db8::1".parse().unwrap();
        let ip2: Ipv6Addr = "2001:db9::1".parse().unwrap();

        assert!(matches_ipv6_cidr(ip1, network, 32));
        assert!(!matches_ipv6_cidr(ip2, network, 32));
    }

    #[test]
    fn test_open_ended_ranges() {
        let ranges = vec![
            IpRange::FromTo {
                from: None,
                to: Some("192.168.1.100".to_string()),
                key: Some("before".to_string()),
            },
            IpRange::FromTo {
                from: Some("192.168.1.100".to_string()),
                to: None,
                key: Some("after".to_string()),
            },
        ];

        let agg = IpRangeAggregation::new("ip", ranges);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_ip("1", "ip", "192.168.1.50"), // Before
            create_test_hit_ip("2", "ip", "192.168.1.150"), // After
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 2);
        } else {
            panic!("Expected Buckets result");
        }
    }
}
