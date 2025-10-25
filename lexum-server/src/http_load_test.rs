//! HTTP load testing framework for Lexum REST API

use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// HTTP load test configuration
#[derive(Debug, Clone)]
pub struct HttpLoadTestConfig {
    /// Base URL of the server
    pub base_url: String,
    /// Number of concurrent clients
    pub concurrent_clients: usize,
    /// Number of requests per client
    pub requests_per_client: usize,
    /// Delay between requests (milliseconds)
    pub request_delay_ms: u64,
    /// Test duration (seconds)
    pub test_duration_secs: u64,
    /// Index name for testing
    pub index_name: String,
    /// API key for authentication (optional)
    pub api_key: Option<String>,
}

impl Default for HttpLoadTestConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:9200".to_string(),
            concurrent_clients: 10,
            requests_per_client: 100,
            request_delay_ms: 100,
            test_duration_secs: 60,
            index_name: "http_load_test_index".to_string(),
            api_key: None,
        }
    }
}

/// HTTP load test results
#[derive(Debug, Clone)]
pub struct HttpLoadTestResults {
    /// Total requests made
    pub total_requests: usize,
    /// Successful requests
    pub successful_requests: usize,
    /// Failed requests
    pub failed_requests: usize,
    /// Average response time (milliseconds)
    pub avg_response_time_ms: f64,
    /// Minimum response time (milliseconds)
    pub min_response_time_ms: f64,
    /// Maximum response time (milliseconds)
    pub max_response_time_ms: f64,
    /// 95th percentile response time (milliseconds)
    pub p95_response_time_ms: f64,
    /// 99th percentile response time (milliseconds)
    pub p99_response_time_ms: f64,
    /// Requests per second
    pub requests_per_second: f64,
    /// Test duration (seconds)
    pub test_duration_secs: f64,
    /// Error breakdown
    pub error_breakdown: std::collections::HashMap<String, usize>,
}

/// HTTP load test client
pub struct HttpLoadTestClient {
    client: Client,
    config: HttpLoadTestConfig,
    response_times: Vec<f64>,
    errors: std::collections::HashMap<String, usize>,
}

impl HttpLoadTestClient {
    /// Create a new HTTP load test client
    pub fn new(config: HttpLoadTestConfig) -> Self {
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10));

        // Add API key header if provided
        if let Some(api_key) = &config.api_key {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::HeaderName::from_static("x-api-key"),
                reqwest::header::HeaderValue::from_str(api_key).unwrap(),
            );
            client_builder = client_builder.default_headers(headers);
        }

        Self {
            client: client_builder.build().unwrap(),
            config,
            response_times: Vec::new(),
            errors: std::collections::HashMap::new(),
        }
    }

    /// Setup test data
    pub async fn setup(&mut self) -> Result<()> {
        println!("Setting up HTTP load test...");

        // Create index
        let index_url = format!("{}/api/v1/indices", self.config.base_url);
        let index_payload = json!({
            "settings": {
                "number_of_shards": 1,
                "number_of_replicas": 0
            },
            "mappings": {
                "properties": {
                    "title": {
                        "type": "text",
                        "analyzer": "standard"
                    },
                    "content": {
                        "type": "text",
                        "analyzer": "standard"
                    },
                    "category": {
                        "type": "keyword"
                    },
                    "score": {
                        "type": "float"
                    },
                    "created_at": {
                        "type": "date"
                    }
                }
            }
        });

        let response = self
            .client
            .post(&index_url)
            .json(&index_payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Failed to create index: {error_text}"));
        }

        // Add some test documents
        self.add_test_documents().await?;

        println!("HTTP load test setup complete");
        Ok(())
    }

    /// Add test documents to the index
    async fn add_test_documents(&self) -> Result<()> {
        let documents = [
            json!({
                "title": "Introduction to Rust Programming",
                "content": "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety.",
                "category": "programming",
                "score": 9.5,
                "created_at": "2024-01-01T00:00:00Z"
            }),
            json!({
                "title": "Advanced Search Algorithms",
                "content": "Search algorithms are fundamental to computer science and are used in many applications.",
                "category": "algorithms",
                "score": 8.7,
                "created_at": "2024-01-02T00:00:00Z"
            }),
            json!({
                "title": "Database Design Principles",
                "content": "Good database design is crucial for application performance and maintainability.",
                "category": "database",
                "score": 8.2,
                "created_at": "2024-01-03T00:00:00Z"
            }),
            json!({
                "title": "Machine Learning Fundamentals",
                "content": "Machine learning is a subset of artificial intelligence that focuses on algorithms that can learn from data.",
                "category": "ai",
                "score": 9.1,
                "created_at": "2024-01-04T00:00:00Z"
            }),
            json!({
                "title": "Web Development Best Practices",
                "content": "Modern web development requires understanding of multiple technologies and frameworks.",
                "category": "web",
                "score": 7.8,
                "created_at": "2024-01-05T00:00:00Z"
            }),
        ];

        for (i, doc) in documents.iter().enumerate() {
            let doc_url = format!(
                "{}/api/v1/indices/{}/documents",
                self.config.base_url, self.config.index_name
            );
            let payload = json!({ "document": doc });

            let response = self.client.post(&doc_url).json(&payload).send().await?;

            if !response.status().is_success() {
                let error_text = response.text().await?;
                return Err(anyhow::anyhow!("Failed to add document {i}: {error_text}"));
            }
        }

        Ok(())
    }

    /// Run a single request
    async fn run_single_request(&mut self) -> Result<()> {
        let start = Instant::now();

        // Randomly choose between different types of requests
        let request_type = rand::random::<u8>() % 4;

        let result = match request_type {
            0 => self.run_health_check().await,
            1 => self.run_cluster_health().await,
            2 => self.run_search_request().await,
            3 => self.run_index_stats().await,
            _ => unreachable!(),
        };

        let duration = start.elapsed();
        let duration_ms = duration.as_secs_f64() * 1000.0;

        self.response_times.push(duration_ms);

        if let Err(e) = result {
            let error_key = format!("{e}");
            *self.errors.entry(error_key).or_insert(0) += 1;
        }

        Ok(())
    }

    /// Run health check request
    async fn run_health_check(&self) -> Result<()> {
        let url = format!("{}/health", self.config.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Health check failed: {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Run cluster health request
    async fn run_cluster_health(&self) -> Result<()> {
        let url = format!("{}/_cluster/health", self.config.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Cluster health failed: {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Run search request
    async fn run_search_request(&self) -> Result<()> {
        let url = format!(
            "{}/api/v1/indices/{}/search",
            self.config.base_url, self.config.index_name
        );

        let search_queries = [
            json!({
                "query": {
                    "match": {
                        "content": "programming"
                    }
                },
                "limit": 10
            }),
            json!({
                "query": {
                    "term": {
                        "category": "programming"
                    }
                },
                "limit": 5
            }),
            json!({
                "query": {
                    "range": {
                        "score": {
                            "gte": 8.0
                        }
                    }
                },
                "limit": 10
            }),
            json!({
                "query": {
                    "bool": {
                        "must": [
                            {
                                "match": {
                                    "content": "machine learning"
                                }
                            }
                        ],
                        "filter": [
                            {
                                "range": {
                                    "score": {
                                        "gte": 8.0
                                    }
                                }
                            }
                        ]
                    }
                },
                "limit": 10
            }),
        ];

        let query = &search_queries[rand::random::<usize>() % search_queries.len()];
        let response = self.client.post(&url).json(query).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Search request failed: {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Run index stats request
    async fn run_index_stats(&self) -> Result<()> {
        let url = format!(
            "{}/api/v1/indices/{}/stats",
            self.config.base_url, self.config.index_name
        );
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Index stats failed: {}", response.status()));
        }

        Ok(())
    }

    /// Run the load test
    pub async fn run_load_test(&mut self) -> Result<HttpLoadTestResults> {
        println!("Starting HTTP load test...");
        println!("Concurrent clients: {}", self.config.concurrent_clients);
        println!("Requests per client: {}", self.config.requests_per_client);
        println!("Test duration: {} seconds", self.config.test_duration_secs);

        let start_time = Instant::now();
        let mut handles = Vec::new();

        // Spawn concurrent clients
        for client_id in 0..self.config.concurrent_clients {
            let mut client = self.clone();
            let requests_per_client = self.config.requests_per_client;
            let request_delay = Duration::from_millis(self.config.request_delay_ms);
            let test_duration = Duration::from_secs(self.config.test_duration_secs);

            let handle = tokio::spawn(async move {
                let mut request_count = 0;
                let client_start = Instant::now();

                while client_start.elapsed() < test_duration && request_count < requests_per_client
                {
                    if let Err(e) = client.run_single_request().await {
                        eprintln!("Client {client_id} request failed: {e}");
                    }

                    request_count += 1;

                    if request_delay > Duration::ZERO {
                        sleep(request_delay).await;
                    }
                }

                client
            });

            handles.push(handle);
        }

        // Wait for all clients to complete
        let mut all_clients = Vec::new();
        for handle in handles {
            if let Ok(client) = handle.await {
                all_clients.push(client);
            }
        }

        let test_duration = start_time.elapsed();

        // Aggregate results from all clients
        let mut total_requests = 0;
        let mut successful_requests = 0;
        let mut failed_requests = 0;
        let mut all_response_times = Vec::new();
        let mut all_errors = std::collections::HashMap::new();

        for client in all_clients {
            total_requests += client.response_times.len();
            successful_requests +=
                client.response_times.len() - client.errors.values().sum::<usize>();
            failed_requests += client.errors.values().sum::<usize>();
            all_response_times.extend(client.response_times);

            for (error, count) in client.errors {
                *all_errors.entry(error).or_insert(0) += count;
            }
        }

        // Calculate statistics
        all_response_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg_response_time = if !all_response_times.is_empty() {
            all_response_times.iter().sum::<f64>() / all_response_times.len() as f64
        } else {
            0.0
        };

        let min_response_time = all_response_times.first().copied().unwrap_or(0.0);
        let max_response_time = all_response_times.last().copied().unwrap_or(0.0);

        let p95_index = (all_response_times.len() as f64 * 0.95) as usize;
        let p95_response_time = all_response_times
            .get(p95_index.min(all_response_times.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0.0);

        let p99_index = (all_response_times.len() as f64 * 0.99) as usize;
        let p99_response_time = all_response_times
            .get(p99_index.min(all_response_times.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0.0);

        let requests_per_second = if test_duration.as_secs_f64() > 0.0 {
            total_requests as f64 / test_duration.as_secs_f64()
        } else {
            0.0
        };

        Ok(HttpLoadTestResults {
            total_requests,
            successful_requests,
            failed_requests,
            avg_response_time_ms: avg_response_time,
            min_response_time_ms: min_response_time,
            max_response_time_ms: max_response_time,
            p95_response_time_ms: p95_response_time,
            p99_response_time_ms: p99_response_time,
            requests_per_second,
            test_duration_secs: test_duration.as_secs_f64(),
            error_breakdown: all_errors,
        })
    }

    /// Cleanup test data
    pub async fn cleanup(&self) -> Result<()> {
        println!("Cleaning up HTTP load test...");

        // Delete the test index
        let index_url = format!(
            "{}/api/v1/indices/{}",
            self.config.base_url, self.config.index_name
        );
        let response = self.client.delete(&index_url).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            eprintln!("Failed to delete index: {error_text}");
        }

        println!("HTTP load test cleanup complete");
        Ok(())
    }
}

impl Clone for HttpLoadTestClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
            response_times: Vec::new(),
            errors: std::collections::HashMap::new(),
        }
    }
}

/// HTTP load test runner
pub struct HttpLoadTestRunner;

impl HttpLoadTestRunner {
    /// Run a load test with given configuration
    pub async fn run_test(config: HttpLoadTestConfig) -> Result<HttpLoadTestResults> {
        let mut client = HttpLoadTestClient::new(config.clone());

        // Setup
        client.setup().await?;

        // Run test
        let results = client.run_load_test().await?;

        // Cleanup
        client.cleanup().await?;

        Ok(results)
    }

    /// Run multiple load tests with different configurations
    pub async fn run_test_suite() -> Result<Vec<(String, HttpLoadTestResults)>> {
        let mut results = Vec::new();

        // Light load test
        let light_config = HttpLoadTestConfig {
            concurrent_clients: 5,
            requests_per_client: 50,
            request_delay_ms: 200,
            test_duration_secs: 30,
            index_name: "light_http_load_test".to_string(),
            ..Default::default()
        };

        println!("Running light HTTP load test...");
        let light_results = Self::run_test(light_config).await?;
        results.push(("Light Load".to_string(), light_results));

        // Medium load test
        let medium_config = HttpLoadTestConfig {
            concurrent_clients: 20,
            requests_per_client: 100,
            request_delay_ms: 100,
            test_duration_secs: 60,
            index_name: "medium_http_load_test".to_string(),
            ..Default::default()
        };

        println!("Running medium HTTP load test...");
        let medium_results = Self::run_test(medium_config).await?;
        results.push(("Medium Load".to_string(), medium_results));

        // Heavy load test
        let heavy_config = HttpLoadTestConfig {
            concurrent_clients: 50,
            requests_per_client: 200,
            request_delay_ms: 50,
            test_duration_secs: 120,
            index_name: "heavy_http_load_test".to_string(),
            ..Default::default()
        };

        println!("Running heavy HTTP load test...");
        let heavy_results = Self::run_test(heavy_config).await?;
        results.push(("Heavy Load".to_string(), heavy_results));

        Ok(results)
    }
}

/// Print detailed results
pub fn print_detailed_results(name: &str, result: &HttpLoadTestResults) {
    println!("\n=== {name} Detailed Results ===");
    println!("Total Requests: {}", result.total_requests);
    println!(
        "Successful: {} ({:.2}%)",
        result.successful_requests,
        if result.total_requests > 0 {
            (result.successful_requests as f64 / result.total_requests as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "Failed: {} ({:.2}%)",
        result.failed_requests,
        if result.total_requests > 0 {
            (result.failed_requests as f64 / result.total_requests as f64) * 100.0
        } else {
            0.0
        }
    );
    println!("Test Duration: {:.2} seconds", result.test_duration_secs);
    println!("Requests per Second: {:.2}", result.requests_per_second);
    println!("\nResponse Times:");
    println!("  Average: {:.2} ms", result.avg_response_time_ms);
    println!("  Minimum: {:.2} ms", result.min_response_time_ms);
    println!("  Maximum: {:.2} ms", result.max_response_time_ms);
    println!("  95th Percentile: {:.2} ms", result.p95_response_time_ms);
    println!("  99th Percentile: {:.2} ms", result.p99_response_time_ms);

    if !result.error_breakdown.is_empty() {
        println!("\nError Breakdown:");
        for (error, count) in &result.error_breakdown {
            println!("  {error}: {count}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_load_test_config_default() {
        let config = HttpLoadTestConfig::default();
        assert_eq!(config.base_url, "http://127.0.0.1:9200");
        assert_eq!(config.concurrent_clients, 10);
        assert_eq!(config.requests_per_client, 100);
    }

    #[tokio::test]
    async fn test_http_load_test_client_creation() {
        let config = HttpLoadTestConfig::default();
        let client = HttpLoadTestClient::new(config);
        assert_eq!(client.config.base_url, "http://127.0.0.1:9200");
    }
}
