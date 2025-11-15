//! Bottleneck analysis endpoints for performance optimization

use axum::Json;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::handlers::profiling::{FunctionStats, ProfilingStatistics};

/// Bottleneck analysis request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BottleneckAnalysisRequest {
    /// Minimum percentage threshold for bottleneck detection (default: 5.0)
    pub min_percentage: Option<f64>,
    /// Minimum sample count threshold (default: 100)
    pub min_samples: Option<u64>,
    /// Include recommendations
    pub include_recommendations: Option<bool>,
}

impl Default for BottleneckAnalysisRequest {
    fn default() -> Self {
        Self {
            min_percentage: Some(5.0),
            min_samples: Some(100),
            include_recommendations: Some(true),
        }
    }
}

/// Bottleneck analysis result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BottleneckAnalysisResult {
    /// Success status
    pub success: bool,
    /// Message
    pub message: String,
    /// Detected bottlenecks
    pub bottlenecks: Vec<Bottleneck>,
    /// Summary statistics
    pub summary: BottleneckSummary,
    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Detected bottleneck
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Bottleneck {
    /// Function/operation name
    pub name: String,
    /// Percentage of total time spent
    pub percentage: f64,
    /// Sample count
    pub samples: u64,
    /// Estimated time spent (seconds)
    pub estimated_time_secs: f64,
    /// Severity level
    pub severity: BottleneckSeverity,
    /// Category of bottleneck
    pub category: BottleneckCategory,
    /// Specific recommendations for this bottleneck
    pub recommendations: Vec<String>,
}

/// Bottleneck severity level
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BottleneckSeverity {
    /// Low severity (< 10% of time)
    Low,
    /// Medium severity (10-25% of time)
    Medium,
    /// High severity (25-50% of time)
    High,
    /// Critical severity (> 50% of time)
    Critical,
}

/// Bottleneck category
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BottleneckCategory {
    /// Search/query execution
    Search,
    /// Index operations
    Indexing,
    /// I/O operations
    Io,
    /// Memory operations
    Memory,
    /// Network operations
    Network,
    /// Serialization/deserialization
    Serialization,
    /// Cache operations
    Cache,
    /// Unknown/other
    Other,
}

/// Bottleneck summary statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BottleneckSummary {
    /// Total bottlenecks detected
    pub total_bottlenecks: usize,
    /// Critical bottlenecks count
    pub critical_count: usize,
    /// High severity bottlenecks count
    pub high_count: usize,
    /// Medium severity bottlenecks count
    pub medium_count: usize,
    /// Low severity bottlenecks count
    pub low_count: usize,
    /// Total percentage covered by bottlenecks
    pub total_percentage: f64,
    /// Estimated performance improvement if bottlenecks are fixed
    pub estimated_improvement_percent: f64,
}

/// Analyze profiling statistics to identify bottlenecks
#[utoipa::path(
    post,
    path = "/_profiling/bottlenecks",
    tag = "Profiling",
    request_body = BottleneckAnalysisRequest,
    responses(
        (status = 200, description = "Bottleneck analysis completed", body = BottleneckAnalysisResult),
        (status = 500, description = "Failed to analyze bottlenecks")
    )
)]
pub async fn analyze_bottlenecks(
    Json(request): Json<BottleneckAnalysisRequest>,
) -> Result<Json<BottleneckAnalysisResult>, StatusCode> {
    tracing::info!("Analyzing bottlenecks with request: {:?}", request);

    // In production, this would use actual profiling data
    // For now, we'll use mock data based on common bottlenecks
    let mock_stats = create_mock_profiling_stats();

    let min_percentage = request.min_percentage.unwrap_or(5.0);
    let min_samples = request.min_samples.unwrap_or(100);
    let include_recommendations = request.include_recommendations.unwrap_or(true);

    let bottlenecks = detect_bottlenecks(&mock_stats, min_percentage, min_samples);
    let summary = calculate_summary(&bottlenecks);
    let recommendations = if include_recommendations {
        generate_recommendations(&bottlenecks, &summary)
    } else {
        Vec::new()
    };

    let result = BottleneckAnalysisResult {
        success: true,
        message: format!(
            "Identified {} bottlenecks covering {:.1}% of execution time",
            bottlenecks.len(),
            summary.total_percentage
        ),
        bottlenecks,
        summary,
        recommendations,
    };

    Ok(Json(result))
}

/// Detect bottlenecks from profiling statistics
fn detect_bottlenecks(
    stats: &ProfilingStatistics,
    min_percentage: f64,
    min_samples: u64,
) -> Vec<Bottleneck> {
    let mut bottlenecks = Vec::new();

    for func in &stats.top_functions {
        if func.percentage < min_percentage || func.samples < min_samples {
            continue;
        }

        let severity = determine_severity(func.percentage);
        let category = categorize_bottleneck(&func.name);
        let estimated_time =
            (func.samples as f64 / stats.samples_per_second) * func.percentage / 100.0;
        let recommendations =
            generate_bottleneck_recommendations(&func.name, func.percentage, &category);

        bottlenecks.push(Bottleneck {
            name: func.name.clone(),
            percentage: func.percentage,
            samples: func.samples,
            estimated_time_secs: estimated_time,
            severity,
            category,
            recommendations,
        });
    }

    // Sort by percentage (highest first)
    bottlenecks.sort_by(|a, b| b.percentage.partial_cmp(&a.percentage).unwrap());

    bottlenecks
}

/// Determine severity level based on percentage
fn determine_severity(percentage: f64) -> BottleneckSeverity {
    if percentage >= 50.0 {
        BottleneckSeverity::Critical
    } else if percentage >= 25.0 {
        BottleneckSeverity::High
    } else if percentage >= 10.0 {
        BottleneckSeverity::Medium
    } else {
        BottleneckSeverity::Low
    }
}

/// Categorize bottleneck based on function name
fn categorize_bottleneck(name: &str) -> BottleneckCategory {
    let name_lower = name.to_lowercase();

    if name_lower.contains("search")
        || name_lower.contains("query")
        || name_lower.contains("execute")
    {
        BottleneckCategory::Search
    } else if name_lower.contains("index")
        || name_lower.contains("add_document")
        || name_lower.contains("bulk")
    {
        BottleneckCategory::Indexing
    } else if name_lower.contains("read")
        || name_lower.contains("write")
        || name_lower.contains("disk")
        || name_lower.contains("file")
    {
        BottleneckCategory::Io
    } else if name_lower.contains("memory")
        || name_lower.contains("alloc")
        || name_lower.contains("heap")
    {
        BottleneckCategory::Memory
    } else if name_lower.contains("network")
        || name_lower.contains("http")
        || name_lower.contains("socket")
    {
        BottleneckCategory::Network
    } else if name_lower.contains("serialize")
        || name_lower.contains("deserialize")
        || name_lower.contains("json")
    {
        BottleneckCategory::Serialization
    } else if name_lower.contains("cache") {
        BottleneckCategory::Cache
    } else {
        BottleneckCategory::Other
    }
}

/// Generate recommendations for a specific bottleneck
fn generate_bottleneck_recommendations(
    name: &str,
    percentage: f64,
    category: &BottleneckCategory,
) -> Vec<String> {
    let mut recommendations = Vec::new();

    match category {
        BottleneckCategory::Search => {
            recommendations.push("Consider optimizing query structure".to_string());
            recommendations.push("Enable query caching for repeated queries".to_string());
            if percentage > 25.0 {
                recommendations.push("Review index schema and field types".to_string());
                recommendations.push("Consider adding more specific filters".to_string());
            }
        }
        BottleneckCategory::Indexing => {
            recommendations
                .push("Use bulk operations instead of individual document adds".to_string());
            recommendations.push("Consider increasing batch size".to_string());
            if percentage > 25.0 {
                recommendations.push("Review document structure and field mappings".to_string());
                recommendations
                    .push("Consider async indexing for non-critical documents".to_string());
            }
        }
        BottleneckCategory::Io => {
            recommendations.push("Consider using memory-mapped files".to_string());
            recommendations.push("Enable read-ahead optimization".to_string());
            if percentage > 25.0 {
                recommendations.push("Review disk I/O patterns".to_string());
                recommendations.push("Consider using faster storage (SSD)".to_string());
            }
        }
        BottleneckCategory::Memory => {
            recommendations.push("Review memory allocation patterns".to_string());
            recommendations.push("Consider using object pooling".to_string());
            if percentage > 25.0 {
                recommendations.push("Profile memory usage with detailed tools".to_string());
                recommendations
                    .push("Consider reducing memory allocations in hot paths".to_string());
            }
        }
        BottleneckCategory::Network => {
            recommendations.push("Enable connection pooling".to_string());
            recommendations.push("Consider request batching".to_string());
            if percentage > 25.0 {
                recommendations.push("Review network serialization overhead".to_string());
                recommendations.push("Consider compression for large payloads".to_string());
            }
        }
        BottleneckCategory::Serialization => {
            recommendations.push("Consider using binary serialization (bincode)".to_string());
            recommendations.push("Cache serialized results when possible".to_string());
            if percentage > 25.0 {
                recommendations
                    .push("Review data structures for serialization efficiency".to_string());
                recommendations
                    .push("Consider streaming serialization for large objects".to_string());
            }
        }
        BottleneckCategory::Cache => {
            recommendations.push("Review cache hit rates".to_string());
            recommendations.push("Consider increasing cache size".to_string());
            if percentage > 25.0 {
                recommendations.push("Review cache eviction policies".to_string());
                recommendations.push("Consider cache warming strategies".to_string());
            }
        }
        BottleneckCategory::Other => {
            recommendations.push(format!("Investigate function: {name}"));
            if percentage > 25.0 {
                recommendations.push("Consider profiling this function in detail".to_string());
            }
        }
    }

    recommendations
}

/// Calculate summary statistics
fn calculate_summary(bottlenecks: &[Bottleneck]) -> BottleneckSummary {
    let total_bottlenecks = bottlenecks.len();
    let critical_count = bottlenecks
        .iter()
        .filter(|b| matches!(b.severity, BottleneckSeverity::Critical))
        .count();
    let high_count = bottlenecks
        .iter()
        .filter(|b| matches!(b.severity, BottleneckSeverity::High))
        .count();
    let medium_count = bottlenecks
        .iter()
        .filter(|b| matches!(b.severity, BottleneckSeverity::Medium))
        .count();
    let low_count = bottlenecks
        .iter()
        .filter(|b| matches!(b.severity, BottleneckSeverity::Low))
        .count();

    let total_percentage: f64 = bottlenecks.iter().map(|b| b.percentage).sum();

    // Estimate improvement: if we fix critical bottlenecks, we can improve by their percentage
    // High severity bottlenecks contribute less improvement potential
    let estimated_improvement = bottlenecks
        .iter()
        .map(|b| {
            match b.severity {
                BottleneckSeverity::Critical => b.percentage * 0.8, // 80% improvement potential
                BottleneckSeverity::High => b.percentage * 0.6,
                BottleneckSeverity::Medium => b.percentage * 0.4,
                BottleneckSeverity::Low => b.percentage * 0.2,
            }
        })
        .sum();

    BottleneckSummary {
        total_bottlenecks,
        critical_count,
        high_count,
        medium_count,
        low_count,
        total_percentage,
        estimated_improvement_percent: estimated_improvement,
    }
}

/// Generate overall recommendations
fn generate_recommendations(
    bottlenecks: &[Bottleneck],
    summary: &BottleneckSummary,
) -> Vec<String> {
    let mut recommendations = Vec::new();

    if summary.critical_count > 0 {
        recommendations.push(format!(
            "CRITICAL: {} critical bottlenecks detected. Address these first for maximum impact.",
            summary.critical_count
        ));
    }

    if summary.total_percentage > 80.0 {
        recommendations.push(
            "High percentage of time spent in bottlenecks. Significant optimization potential."
                .to_string(),
        );
    }

    // Category-specific recommendations
    let search_bottlenecks: Vec<_> = bottlenecks
        .iter()
        .filter(|b| matches!(b.category, BottleneckCategory::Search))
        .collect();
    if !search_bottlenecks.is_empty() {
        recommendations.push(
            "Search bottlenecks detected. Consider query optimization and caching.".to_string(),
        );
    }

    let io_bottlenecks: Vec<_> = bottlenecks
        .iter()
        .filter(|b| matches!(b.category, BottleneckCategory::Io))
        .collect();
    if !io_bottlenecks.is_empty() {
        recommendations.push(
            "I/O bottlenecks detected. Consider memory-mapped files and read-ahead optimization."
                .to_string(),
        );
    }

    if summary.estimated_improvement_percent > 20.0 {
        recommendations.push(format!(
            "Estimated performance improvement: {:.1}% if bottlenecks are addressed",
            summary.estimated_improvement_percent
        ));
    }

    recommendations
}

/// Create mock profiling statistics for demonstration
fn create_mock_profiling_stats() -> ProfilingStatistics {
    ProfilingStatistics {
        total_samples: 1000,
        duration_secs: 30.0,
        samples_per_second: 33.33,
        top_functions: vec![
            FunctionStats {
                name: "lexum_core::search::executor::SearchExecutor::execute".to_string(),
                samples: 450,
                percentage: 45.0,
            },
            FunctionStats {
                name: "lexum_core::index::manager::IndexManager::get_index".to_string(),
                samples: 200,
                percentage: 20.0,
            },
            FunctionStats {
                name: "tantivy::searcher::Searcher::search".to_string(),
                samples: 150,
                percentage: 15.0,
            },
            FunctionStats {
                name: "serde_json::from_str".to_string(),
                samples: 100,
                percentage: 10.0,
            },
            FunctionStats {
                name: "lexum_core::index::manager::IndexManager::add_document".to_string(),
                samples: 50,
                percentage: 5.0,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_severity() {
        assert!(matches!(
            determine_severity(60.0),
            BottleneckSeverity::Critical
        ));
        assert!(matches!(determine_severity(30.0), BottleneckSeverity::High));
        assert!(matches!(
            determine_severity(15.0),
            BottleneckSeverity::Medium
        ));
        assert!(matches!(determine_severity(5.0), BottleneckSeverity::Low));
    }

    #[test]
    fn test_categorize_bottleneck() {
        assert!(matches!(
            categorize_bottleneck("SearchExecutor::execute"),
            BottleneckCategory::Search
        ));
        assert!(matches!(
            categorize_bottleneck("IndexManager::add_document"),
            BottleneckCategory::Indexing
        ));
        assert!(matches!(
            categorize_bottleneck("read_file"),
            BottleneckCategory::Io
        ));
        assert!(matches!(
            categorize_bottleneck("serialize_json"),
            BottleneckCategory::Serialization
        ));
    }

    #[test]
    fn test_detect_bottlenecks() {
        let stats = create_mock_profiling_stats();
        let bottlenecks = detect_bottlenecks(&stats, 5.0, 50);

        assert!(!bottlenecks.is_empty());
        assert!(bottlenecks[0].percentage >= bottlenecks[1].percentage); // Sorted
    }

    #[test]
    fn test_calculate_summary() {
        let stats = create_mock_profiling_stats();
        let bottlenecks = detect_bottlenecks(&stats, 5.0, 50);
        let summary = calculate_summary(&bottlenecks);

        assert!(summary.total_bottlenecks > 0);
        assert!(summary.total_percentage > 0.0);
    }

    #[test]
    fn test_generate_recommendations() {
        let stats = create_mock_profiling_stats();
        let bottlenecks = detect_bottlenecks(&stats, 5.0, 50);
        let summary = calculate_summary(&bottlenecks);
        let recommendations = generate_recommendations(&bottlenecks, &summary);

        assert!(!recommendations.is_empty());
    }
}
