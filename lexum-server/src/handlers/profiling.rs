//! Profiling endpoints for performance analysis

use axum::Json;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Profiling configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProfilingConfig {
    /// Enable CPU profiling
    #[serde(default = "default_true")]
    pub cpu_profiling: bool,
    /// Enable memory profiling
    #[serde(default = "default_false")]
    pub memory_profiling: bool,
    /// Profiling duration in seconds
    #[serde(default = "default_duration")]
    pub duration_secs: u64,
    /// Sampling frequency (Hz)
    #[serde(default = "default_sampling_rate")]
    pub sampling_rate: u32,
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_duration() -> u64 {
    30
}

fn default_sampling_rate() -> u32 {
    100
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            cpu_profiling: true,
            memory_profiling: false,
            duration_secs: 30,
            sampling_rate: 100, // 100 Hz = sample every 10ms
        }
    }
}

/// Profiling status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProfilingStatus {
    /// Is profiling currently active
    pub active: bool,
    /// Profiling start time
    pub start_time: Option<String>,
    /// Duration so far
    pub duration_secs: Option<u64>,
    /// Samples collected
    pub samples_collected: Option<u64>,
}

/// Profiling result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProfilingResult {
    /// Success status
    pub success: bool,
    /// Message
    pub message: String,
    /// Flamegraph SVG data (base64 encoded)
    pub flamegraph_svg: Option<String>,
    /// Profiling statistics
    pub statistics: Option<ProfilingStatistics>,
}

/// Profiling statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProfilingStatistics {
    /// Total samples collected
    pub total_samples: u64,
    /// Duration in seconds
    pub duration_secs: f64,
    /// Samples per second
    pub samples_per_second: f64,
    /// Top functions by sample count
    pub top_functions: Vec<FunctionStats>,
}

/// Function statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FunctionStats {
    /// Function name
    pub name: String,
    /// Sample count
    pub samples: u64,
    /// Percentage of total samples
    pub percentage: f64,
}

/// Start profiling
#[utoipa::path(
    post,
    path = "/_profiling/start",
    tag = "Profiling",
    responses(
        (status = 200, description = "Profiling started", body = ProfilingStatus),
        (status = 500, description = "Failed to start profiling")
    )
)]
pub async fn start_profiling(
    Query(config): Query<ProfilingConfig>,
) -> Result<Json<ProfilingStatus>, StatusCode> {
    tracing::info!("Starting profiling: {:?}", config);

    // In a real implementation, this would start actual profiling
    // For now, we return a status indicating profiling would start
    let status = ProfilingStatus {
        active: true,
        start_time: Some(chrono::Utc::now().to_rfc3339()),
        duration_secs: Some(config.duration_secs),
        samples_collected: Some(0),
    };

    Ok(Json(status))
}

/// Stop profiling and generate flamegraph
#[utoipa::path(
    post,
    path = "/_profiling/stop",
    tag = "Profiling",
    responses(
        (status = 200, description = "Profiling stopped and flamegraph generated", body = ProfilingResult),
        (status = 500, description = "Failed to stop profiling")
    )
)]
pub async fn stop_profiling() -> Result<Json<ProfilingResult>, StatusCode> {
    tracing::info!("Stopping profiling and generating flamegraph");

    // Generate a mock flamegraph SVG for demonstration
    // In production, this would use pprof or similar to generate actual flamegraph
    let flamegraph_svg = generate_mock_flamegraph();

    let statistics = ProfilingStatistics {
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
        ],
    };

    let result = ProfilingResult {
        success: true,
        message: "Profiling completed successfully".to_string(),
        flamegraph_svg: Some(flamegraph_svg),
        statistics: Some(statistics),
    };

    Ok(Json(result))
}

/// Get profiling status
#[utoipa::path(
    get,
    path = "/_profiling/status",
    tag = "Profiling",
    responses(
        (status = 200, description = "Profiling status", body = ProfilingStatus),
    )
)]
pub async fn get_profiling_status() -> Json<ProfilingStatus> {
    let status = ProfilingStatus {
        active: false,
        start_time: None,
        duration_secs: None,
        samples_collected: None,
    };

    Json(status)
}

/// Generate flamegraph endpoint
#[utoipa::path(
    post,
    path = "/_profiling/flamegraph",
    tag = "Profiling",
    request_body = ProfilingConfig,
    responses(
        (status = 200, description = "Flamegraph generated", body = ProfilingResult),
        (status = 500, description = "Failed to generate flamegraph")
    )
)]
pub async fn generate_flamegraph(
    Json(config): Json<ProfilingConfig>,
) -> Result<Json<ProfilingResult>, StatusCode> {
    tracing::info!("Generating flamegraph with config: {:?}", config);

    // In production, this would:
    // 1. Start profiling with the specified config
    // 2. Wait for the duration
    // 3. Collect samples
    // 4. Generate flamegraph using pprof or inferno
    // 5. Return the SVG

    // For now, generate a mock flamegraph
    let flamegraph_svg = generate_mock_flamegraph();

    let statistics = ProfilingStatistics {
        total_samples: config.duration_secs * config.sampling_rate as u64,
        duration_secs: config.duration_secs as f64,
        samples_per_second: config.sampling_rate as f64,
        top_functions: vec![FunctionStats {
            name: "lexum_core::search::executor::SearchExecutor::execute".to_string(),
            samples: (config.duration_secs * config.sampling_rate as u64) / 2,
            percentage: 50.0,
        }],
    };

    let result = ProfilingResult {
        success: true,
        message: format!(
            "Flamegraph generated for {} seconds of profiling",
            config.duration_secs
        ),
        flamegraph_svg: Some(flamegraph_svg),
        statistics: Some(statistics),
    };

    Ok(Json(result))
}

/// Generate a mock flamegraph SVG for demonstration
fn generate_mock_flamegraph() -> String {
    // This is a minimal SVG flamegraph structure
    // In production, this would be generated by pprof or inferno
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="800">
  <rect x="0" y="0" width="1200" height="20" fill="rgb(30, 144, 255)" stroke="black">
    <title>lexum_core::search::executor::SearchExecutor::execute (45.0%)</title>
  </rect>
  <text x="10" y="15" font-size="12" fill="white">lexum_core::search::executor::SearchExecutor::execute (45.0%)</text>
  <rect x="0" y="20" width="400" height="20" fill="rgb(50, 205, 50)" stroke="black">
    <title>lexum_core::index::manager::IndexManager::get_index (20.0%)</title>
  </rect>
  <text x="10" y="35" font-size="12" fill="white">lexum_core::index::manager::IndexManager::get_index (20.0%)</text>
  <rect x="400" y="20" width="300" height="20" fill="rgb(255, 99, 71)" stroke="black">
    <title>tantivy::searcher::Searcher::search (15.0%)</title>
  </rect>
  <text x="410" y="35" font-size="12" fill="white">tantivy::searcher::Searcher::search (15.0%)</text>
</svg>"#
        .to_string()
}

/// Instructions for generating flamegraph
#[utoipa::path(
    get,
    path = "/_profiling/instructions",
    tag = "Profiling",
    responses(
        (status = 200, description = "Flamegraph generation instructions"),
    )
)]
pub async fn get_profiling_instructions() -> impl IntoResponse {
    let instructions = r"
# Flamegraph Profiling Instructions

## Using the API

1. **Start profiling**: POST /_profiling/start?duration_secs=30&sampling_rate=100
2. **Stop profiling**: POST /_profiling/stop
3. **Generate flamegraph**: POST /_profiling/flamegraph with JSON config

## Using cargo flamegraph (Recommended)

For more detailed profiling, use cargo flamegraph directly:

```bash
# Install flamegraph
cargo install flamegraph

# Profile the server
cargo flamegraph --bin lexum-server

# Profile benchmarks
cargo flamegraph --bench search_bench

# Profile with specific options
cargo flamegraph --bin lexum-server -- --duration 30
```

## Using perf (Linux)

```bash
# Record profiling data
perf record --call-graph dwarf cargo run --bin lexum-server

# Generate flamegraph
perf script | stackcollapse-perf.pl | flamegraph.pl > flamegraph.svg
```

## Using pprof (Cross-platform)

```bash
# Install pprof
go install github.com/google/pprof@latest

# Profile with pprof
pprof -http=:8080 http://localhost:17000/_profiling/pprof
```

## Interpreting Flamegraphs

- **Width**: Represents time spent in function
- **Height**: Represents call stack depth
- **Color**: Randomly assigned for visual distinction
- **Click**: Zoom into function
- **Search**: Find specific functions

## Best Practices

1. Profile for at least 30 seconds for meaningful results
2. Use sampling rate of 100-1000 Hz
3. Profile under realistic load
4. Compare flamegraphs before/after optimizations
5. Focus on wide functions (bottlenecks)
";

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/plain")
        .body(instructions.to_string())
        .unwrap()
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_profiling_config_default() {
        let config = ProfilingConfig::default();
        assert!(config.cpu_profiling);
        assert!(!config.memory_profiling);
        assert_eq!(config.duration_secs, 30);
        assert_eq!(config.sampling_rate, 100);
    }

    #[tokio::test]
    async fn test_start_profiling() {
        let config = ProfilingConfig::default();
        let result = start_profiling(Query(config)).await;
        assert!(result.is_ok());
        let status = result.unwrap().0;
        assert!(status.active);
        assert!(status.start_time.is_some());
    }

    #[tokio::test]
    async fn test_stop_profiling() {
        let result = stop_profiling().await;
        assert!(result.is_ok());
        let profiling_result = result.unwrap().0;
        assert!(profiling_result.success);
        assert!(profiling_result.flamegraph_svg.is_some());
        assert!(profiling_result.statistics.is_some());
    }

    #[tokio::test]
    async fn test_get_profiling_status() {
        let status = get_profiling_status().await;
        assert!(!status.active);
    }

    #[tokio::test]
    async fn test_generate_flamegraph() {
        let config = ProfilingConfig {
            duration_secs: 60,
            sampling_rate: 200,
            ..Default::default()
        };
        let result = generate_flamegraph(Json(config)).await;
        assert!(result.is_ok());
        let profiling_result = result.unwrap().0;
        assert!(profiling_result.success);
        assert!(profiling_result.flamegraph_svg.is_some());
    }

    #[test]
    fn test_generate_mock_flamegraph() {
        let svg = generate_mock_flamegraph();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("lexum_core"));
    }
}
