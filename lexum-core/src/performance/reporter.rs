//! Performance reporting and visualization

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::performance::{Metrics, Profile, AnalysisReport};

/// Performance reporter for generating reports
#[derive(Debug, Clone)]
pub struct PerformanceReporter {
    /// Report format
    format: ReportFormat,
    /// Include detailed measurements
    include_details: bool,
    /// Include system metrics
    include_system: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Json,
    Yaml,
    Text,
    Html,
    Csv,
}

impl PerformanceReporter {
    /// Create new reporter
    pub fn new() -> Self {
        Self {
            format: ReportFormat::Text,
            include_details: true,
            include_system: true,
        }
    }

    /// Set report format
    pub fn with_format(mut self, format: ReportFormat) -> Self {
        self.format = format;
        self
    }

    /// Set whether to include detailed measurements
    pub fn with_details(mut self, include: bool) -> Self {
        self.include_details = include;
        self
    }

    /// Set whether to include system metrics
    pub fn with_system(mut self, include: bool) -> Self {
        self.include_system = include;
        self
    }

    /// Generate performance report
    pub fn generate_report(
        &self,
        metrics: &Metrics,
        profiles: &HashMap<String, Profile>,
        analysis: &AnalysisReport,
    ) -> String {
        match self.format {
            ReportFormat::Json => self.generate_json_report(metrics, profiles, analysis),
            ReportFormat::Yaml => self.generate_yaml_report(metrics, profiles, analysis),
            ReportFormat::Text => self.generate_text_report(metrics, profiles, analysis),
            ReportFormat::Html => self.generate_html_report(metrics, profiles, analysis),
            ReportFormat::Csv => self.generate_csv_report(metrics, profiles, analysis),
        }
    }

    /// Generate JSON report
    fn generate_json_report(
        &self,
        metrics: &Metrics,
        profiles: &HashMap<String, Profile>,
        analysis: &AnalysisReport,
    ) -> String {
        let report = PerformanceReport {
            metrics: if self.include_details { Some(metrics.clone()) } else { None },
            profiles: if self.include_details { Some(profiles.clone()) } else { None },
            analysis: analysis.clone(),
            system_metrics: if self.include_system { Some(metrics.system.clone()) } else { None },
        };
        
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "Error generating JSON report".to_string())
    }

    /// Generate YAML report
    fn generate_yaml_report(
        &self,
        metrics: &Metrics,
        profiles: &HashMap<String, Profile>,
        analysis: &AnalysisReport,
    ) -> String {
        let report = PerformanceReport {
            metrics: if self.include_details { Some(metrics.clone()) } else { None },
            profiles: if self.include_details { Some(profiles.clone()) } else { None },
            analysis: analysis.clone(),
            system_metrics: if self.include_system { Some(metrics.system.clone()) } else { None },
        };
        
        serde_yaml::to_string(&report).unwrap_or_else(|_| "Error generating YAML report".to_string())
    }

    /// Generate text report
    fn generate_text_report(
        &self,
        metrics: &Metrics,
        profiles: &HashMap<String, Profile>,
        analysis: &AnalysisReport,
    ) -> String {
        let mut report = String::new();
        
        report.push_str("=== LEXUM PERFORMANCE REPORT ===\n\n");
        
        // Summary
        report.push_str("SUMMARY:\n");
        report.push_str(&format!("Total Operations: {}\n", profiles.len()));
        report.push_str(&format!("Total Measurements: {}\n", 
            profiles.values().map(|p| p.count).sum::<u64>()));
        
        if self.include_system {
            report.push_str(&format!("Memory Usage: {} MB\n", metrics.system.memory_usage / 1024 / 1024));
            report.push_str(&format!("CPU Usage: {:.1}%\n", metrics.system.cpu_usage));
        }
        
        report.push_str("\n");
        
        // Top slow operations
        if !analysis.slow_operations.is_empty() {
            report.push_str("SLOW OPERATIONS:\n");
            for op in &analysis.slow_operations {
                report.push_str(&format!("  {}: {:.2}ms avg ({} calls)\n", 
                    op.name, 
                    op.avg_time.as_secs_f64() * 1000.0,
                    op.count));
                report.push_str(&format!("    Recommendation: {}\n", op.recommendation));
            }
            report.push_str("\n");
        }
        
        // Inconsistent operations
        if !analysis.inconsistent_operations.is_empty() {
            report.push_str("INCONSISTENT OPERATIONS:\n");
            for op in &analysis.inconsistent_operations {
                report.push_str(&format!("  {}: {:.2}ms std dev (CV: {:.2})\n", 
                    op.name,
                    op.std_dev.as_secs_f64() * 1000.0,
                    op.coefficient_of_variation));
                report.push_str(&format!("    Recommendation: {}\n", op.recommendation));
            }
            report.push_str("\n");
        }
        
        // High frequency operations
        if !analysis.high_frequency_operations.is_empty() {
            report.push_str("HIGH FREQUENCY OPERATIONS:\n");
            for op in &analysis.high_frequency_operations {
                report.push_str(&format!("  {}: {} calls, {:.2}ms avg\n", 
                    op.name,
                    op.count,
                    op.avg_time.as_secs_f64() * 1000.0));
                report.push_str(&format!("    Recommendation: {}\n", op.recommendation));
            }
            report.push_str("\n");
        }
        
        // Detailed profiles
        if self.include_details && !profiles.is_empty() {
            report.push_str("DETAILED PROFILES:\n");
            for (name, profile) in profiles {
                report.push_str(&format!("\n{}:\n", name));
                report.push_str(&format!("  Count: {}\n", profile.count));
                report.push_str(&format!("  Total Time: {:.2}ms\n", profile.total_time.as_secs_f64() * 1000.0));
                report.push_str(&format!("  Average Time: {:.2}ms\n", profile.avg_time.as_secs_f64() * 1000.0));
                report.push_str(&format!("  Min Time: {:.2}ms\n", profile.min_time.as_secs_f64() * 1000.0));
                report.push_str(&format!("  Max Time: {:.2}ms\n", profile.max_time.as_secs_f64() * 1000.0));
                report.push_str(&format!("  Std Dev: {:.2}ms\n", profile.std_dev.as_secs_f64() * 1000.0));
                report.push_str(&format!("  P50: {:.2}ms\n", profile.get_p50().as_secs_f64() * 1000.0));
                report.push_str(&format!("  P95: {:.2}ms\n", profile.get_p95().as_secs_f64() * 1000.0));
                report.push_str(&format!("  P99: {:.2}ms\n", profile.get_p99().as_secs_f64() * 1000.0));
                report.push_str(&format!("  Throughput: {:.2} ops/sec\n", profile.get_throughput()));
            }
        }
        
        report
    }

    /// Generate HTML report
    fn generate_html_report(
        &self,
        metrics: &Metrics,
        profiles: &HashMap<String, Profile>,
        analysis: &AnalysisReport,
    ) -> String {
        let mut html = String::new();
        
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<title>Lexum Performance Report</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
        html.push_str("h1, h2, h3 { color: #333; }\n");
        html.push_str("table { border-collapse: collapse; width: 100%; margin: 10px 0; }\n");
        html.push_str("th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
        html.push_str("th { background-color: #f2f2f2; }\n");
        html.push_str(".warning { color: #ff6600; }\n");
        html.push_str(".error { color: #ff0000; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");
        
        html.push_str("<h1>Lexum Performance Report</h1>\n");
        
        // Summary
        html.push_str("<h2>Summary</h2>\n");
        html.push_str("<table>\n");
        html.push_str("<tr><th>Metric</th><th>Value</th></tr>\n");
        html.push_str(&format!("<tr><td>Total Operations</td><td>{}</td></tr>\n", profiles.len()));
        html.push_str(&format!("<tr><td>Total Measurements</td><td>{}</td></tr>\n", 
            profiles.values().map(|p| p.count).sum::<u64>()));
        
        if self.include_system {
            html.push_str(&format!("<tr><td>Memory Usage</td><td>{} MB</td></tr>\n", 
                metrics.system.memory_usage / 1024 / 1024));
            html.push_str(&format!("<tr><td>CPU Usage</td><td>{:.1}%</td></tr>\n", 
                metrics.system.cpu_usage));
        }
        
        html.push_str("</table>\n");
        
        // Slow operations
        if !analysis.slow_operations.is_empty() {
            html.push_str("<h2 class=\"warning\">Slow Operations</h2>\n");
            html.push_str("<table>\n");
            html.push_str("<tr><th>Operation</th><th>Avg Time (ms)</th><th>Calls</th><th>Recommendation</th></tr>\n");
            
            for op in &analysis.slow_operations {
                html.push_str(&format!(
                    "<tr><td>{}</td><td>{:.2}</td><td>{}</td><td>{}</td></tr>\n",
                    op.name,
                    op.avg_time.as_secs_f64() * 1000.0,
                    op.count,
                    op.recommendation
                ));
            }
            
            html.push_str("</table>\n");
        }
        
        // Detailed profiles
        if self.include_details && !profiles.is_empty() {
            html.push_str("<h2>Detailed Profiles</h2>\n");
            html.push_str("<table>\n");
            html.push_str("<tr><th>Operation</th><th>Count</th><th>Avg (ms)</th><th>Min (ms)</th><th>Max (ms)</th><th>P95 (ms)</th><th>Throughput (ops/sec)</th></tr>\n");
            
            for (name, profile) in profiles {
                html.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{:.2}</td><td>{:.2}</td><td>{:.2}</td><td>{:.2}</td><td>{:.2}</td></tr>\n",
                    name,
                    profile.count,
                    profile.avg_time.as_secs_f64() * 1000.0,
                    profile.min_time.as_secs_f64() * 1000.0,
                    profile.max_time.as_secs_f64() * 1000.0,
                    profile.get_p95().as_secs_f64() * 1000.0,
                    profile.get_throughput()
                ));
            }
            
            html.push_str("</table>\n");
        }
        
        html.push_str("</body>\n</html>\n");
        html
    }

    /// Generate CSV report
    fn generate_csv_report(
        &self,
        _metrics: &Metrics,
        profiles: &HashMap<String, Profile>,
        _analysis: &AnalysisReport,
    ) -> String {
        let mut csv = String::new();
        
        // Header
        csv.push_str("Operation,Count,Total Time (ms),Avg Time (ms),Min Time (ms),Max Time (ms),Std Dev (ms),P50 (ms),P95 (ms),P99 (ms),Throughput (ops/sec)\n");
        
        // Data rows
        for (name, profile) in profiles {
            csv.push_str(&format!(
                "{},{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}\n",
                name,
                profile.count,
                profile.total_time.as_secs_f64() * 1000.0,
                profile.avg_time.as_secs_f64() * 1000.0,
                profile.min_time.as_secs_f64() * 1000.0,
                profile.max_time.as_secs_f64() * 1000.0,
                profile.std_dev.as_secs_f64() * 1000.0,
                profile.get_p50().as_secs_f64() * 1000.0,
                profile.get_p95().as_secs_f64() * 1000.0,
                profile.get_p99().as_secs_f64() * 1000.0,
                profile.get_throughput()
            ));
        }
        
        csv
    }
}

impl Default for PerformanceReporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance report data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub metrics: Option<Metrics>,
    pub profiles: Option<HashMap<String, Profile>>,
    pub analysis: AnalysisReport,
    pub system_metrics: Option<crate::performance::SystemMetrics>,
}

/// Benchmark configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Number of iterations
    pub iterations: usize,
    /// Warmup iterations
    pub warmup: usize,
    /// Measurement duration
    pub duration: Duration,
    /// Enable profiling
    pub enable_profiling: bool,
    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 1000,
            warmup: 100,
            duration: Duration::from_secs(10),
            enable_profiling: true,
            enable_metrics: true,
        }
    }
}

/// Benchmark runner
pub struct BenchmarkRunner {
    config: BenchmarkConfig,
    profiler: Option<crate::performance::Profiler>,
    metrics_collector: Option<crate::performance::MetricsCollector>,
}

impl BenchmarkRunner {
    /// Create new benchmark runner
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            profiler: if config.enable_profiling {
                Some(crate::performance::Profiler::new())
            } else {
                None
            },
            metrics_collector: if config.enable_metrics {
                Some(crate::performance::MetricsCollector::new())
            } else {
                None
            },
            config,
        }
    }

    /// Run a benchmark
    pub async fn run_benchmark<F, R>(&self, name: &str, operation: F) -> BenchmarkResult
    where
        F: Fn() -> R,
    {
        let mut measurements = Vec::new();
        
        // Warmup
        for _ in 0..self.config.warmup {
            let _ = operation();
        }
        
        // Actual measurements
        for _ in 0..self.config.iterations {
            let start = std::time::Instant::now();
            let _ = operation();
            let duration = start.elapsed();
            measurements.push(duration);
        }
        
        // Calculate statistics
        let count = measurements.len() as u64;
        let total: Duration = measurements.iter().sum();
        let avg = Duration::from_nanos((total.as_nanos() / count as u128) as u64);
        let min = measurements.iter().min().copied().unwrap_or(Duration::ZERO);
        let max = measurements.iter().max().copied().unwrap_or(Duration::ZERO);
        
        // Calculate standard deviation
        let variance = measurements
            .iter()
            .map(|&d| {
                let diff = d.as_nanos() as f64 - avg.as_nanos() as f64;
                diff * diff
            })
            .sum::<f64>() / (count - 1) as f64;
        let std_dev = Duration::from_nanos(variance.sqrt() as u64);
        
        BenchmarkResult {
            name: name.to_string(),
            count,
            total_time: total,
            avg_time: avg,
            min_time: min,
            max_time: max,
            std_dev,
            measurements,
        }
    }

    /// Get profiler
    pub fn profiler(&self) -> Option<&crate::performance::Profiler> {
        self.profiler.as_ref()
    }

    /// Get metrics collector
    pub fn metrics_collector(&self) -> Option<&crate::performance::MetricsCollector> {
        self.metrics_collector.as_ref()
    }
}

/// Benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub count: u64,
    pub total_time: Duration,
    pub avg_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
    pub std_dev: Duration,
    pub measurements: Vec<Duration>,
}

impl BenchmarkResult {
    /// Get throughput (operations per second)
    pub fn get_throughput(&self) -> f64 {
        if self.avg_time.is_zero() {
            0.0
        } else {
            1.0 / self.avg_time.as_secs_f64()
        }
    }

    /// Get percentile
    pub fn get_percentile(&self, percentile: f64) -> Duration {
        if self.measurements.is_empty() {
            return Duration::ZERO;
        }
        
        let mut sorted = self.measurements.clone();
        sorted.sort();
        
        let index = ((percentile / 100.0) * (sorted.len() - 1) as f64) as usize;
        sorted[index.min(sorted.len() - 1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_reporter() {
        let reporter = PerformanceReporter::new()
            .with_format(ReportFormat::Text)
            .with_details(true)
            .with_system(true);
        
        let metrics = Metrics::new();
        let profiles = HashMap::new();
        let analysis = AnalysisReport::new();
        
        let report = reporter.generate_report(&metrics, &profiles, &analysis);
        assert!(report.contains("LEXUM PERFORMANCE REPORT"));
    }

    #[test]
    fn test_benchmark_runner() {
        let config = BenchmarkConfig::default();
        let runner = BenchmarkRunner::new(config);
        
        // This would be an async test in a real implementation
        // For now, just test that the runner can be created
        assert!(runner.profiler().is_some());
        assert!(runner.metrics_collector().is_some());
    }
}