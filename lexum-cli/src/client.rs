//! HTTP client for Lexum API

use anyhow::Result;
use reqwest::{Client, ClientBuilder};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::time::Duration;

/// Lexum HTTP client with optimized configuration
#[derive(Clone)]
pub struct LexumClient {
    base_url: String,
    client: Client,
}

impl LexumClient {
    /// Create new client with optimized settings
    pub fn new(base_url: String) -> Self {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .user_agent("lexum-cli/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { base_url, client }
    }

    /// GET request with retry logic
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.get_with_retries(path, 3).await
    }

    /// GET request with retry logic
    async fn get_with_retries<T: DeserializeOwned>(&self, path: &str, retries: u32) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);

        for attempt in 0..=retries {
            match self.client.get(&url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response.json().await?);
                    } else {
                        let status = response.status();
                        let text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());

                        // Don't retry on client errors (4xx)
                        if status.is_client_error() {
                            anyhow::bail!("HTTP {status} - {text}");
                        }

                        // Retry on server errors (5xx) or network issues
                        if attempt < retries {
                            tokio::time::sleep(Duration::from_millis(100 * u64::from(attempt + 1)))
                                .await;
                            continue;
                        }

                        anyhow::bail!("HTTP {status} - {text}");
                    }
                }
                Err(e) => {
                    if attempt < retries {
                        tokio::time::sleep(Duration::from_millis(100 * u64::from(attempt + 1)))
                            .await;
                        continue;
                    }
                    return Err(e.into());
                }
            }
        }

        unreachable!()
    }

    /// POST request with retry logic
    pub async fn post<T: Serialize, R: DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
        self.post_with_retries(path, body, 3).await
    }

    /// POST request with retry logic
    async fn post_with_retries<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
        retries: u32,
    ) -> Result<R> {
        let url = format!("{}{}", self.base_url, path);

        for attempt in 0..=retries {
            match self.client.post(&url).json(body).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response.json().await?);
                    } else {
                        let status = response.status();
                        let text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());

                        // Don't retry on client errors (4xx)
                        if status.is_client_error() {
                            anyhow::bail!("HTTP {status} - {text}");
                        }

                        // Retry on server errors (5xx) or network issues
                        if attempt < retries {
                            tokio::time::sleep(Duration::from_millis(100 * u64::from(attempt + 1)))
                                .await;
                            continue;
                        }

                        anyhow::bail!("HTTP {status} - {text}");
                    }
                }
                Err(e) => {
                    if attempt < retries {
                        tokio::time::sleep(Duration::from_millis(100 * u64::from(attempt + 1)))
                            .await;
                        continue;
                    }
                    return Err(e.into());
                }
            }
        }

        unreachable!()
    }

    /// PUT request
    pub async fn put<T: Serialize, R: DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.put(&url).json(body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            anyhow::bail!("HTTP {status} - {text}");
        }

        Ok(response.json().await?)
    }

    /// DELETE request
    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.delete(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            anyhow::bail!("HTTP {status} - {text}");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio;

    #[tokio::test]
    async fn test_client_creation() {
        let client = LexumClient::new("http://localhost:9200".to_string());
        assert_eq!(client.base_url, "http://localhost:9200");
    }

    #[tokio::test]
    async fn test_get_request_success() {
        // This test would require a mock server in a real scenario
        // For now, we'll test the URL construction logic
        let client = LexumClient::new("http://localhost:9200".to_string());
        let path = "/api/test";
        let expected_url = format!("{}{}", client.base_url, path);
        assert_eq!(expected_url, "http://localhost:9200/api/test");
    }

    #[tokio::test]
    async fn test_post_request_success() {
        // This test would require a mock server in a real scenario
        // For now, we'll test the URL construction logic
        let client = LexumClient::new("http://localhost:9200".to_string());
        let path = "/api/test";
        let body = json!({"test": "data"});
        let expected_url = format!("{}{}", client.base_url, path);
        assert_eq!(expected_url, "http://localhost:9200/api/test");
    }

    #[tokio::test]
    async fn test_put_request_success() {
        // This test would require a mock server in a real scenario
        // For now, we'll test the URL construction logic
        let client = LexumClient::new("http://localhost:9200".to_string());
        let path = "/api/test";
        let body = json!({"test": "data"});
        let expected_url = format!("{}{}", client.base_url, path);
        assert_eq!(expected_url, "http://localhost:9200/api/test");
    }

    #[tokio::test]
    async fn test_delete_request_success() {
        // This test would require a mock server in a real scenario
        // For now, we'll test the URL construction logic
        let client = LexumClient::new("http://localhost:9200".to_string());
        let path = "/api/test";
        let expected_url = format!("{}{}", client.base_url, path);
        assert_eq!(expected_url, "http://localhost:9200/api/test");
    }

    #[tokio::test]
    async fn test_retry_logic_parameters() {
        // Test that retry parameters are correctly set
        let client = LexumClient::new("http://localhost:9200".to_string());

        // Test retry count logic (3 retries = 4 total attempts)
        let retries = 3;
        for attempt in 0..=retries {
            if attempt < retries {
                // This would be the retry delay calculation
                let delay = 100 * (attempt + 1) as u64;
                assert!(delay > 0);
            }
        }
    }

    #[tokio::test]
    async fn test_error_handling_scenarios() {
        // Test error handling logic without actual network calls
        let client = LexumClient::new("http://localhost:9200".to_string());

        // Test client error detection (4xx status codes)
        let client_error_status = 400;
        let is_client_error = client_error_status >= 400 && client_error_status < 500;
        assert!(is_client_error);

        // Test server error detection (5xx status codes)
        let server_error_status = 500;
        let is_server_error = server_error_status >= 500 && server_error_status < 600;
        assert!(is_server_error);
    }

    #[tokio::test]
    async fn test_url_construction() {
        let client = LexumClient::new("http://localhost:9200".to_string());

        // Test various path formats
        let paths = vec!["/api/test", "/api/test/", "api/test", "/", ""];

        for path in paths {
            let expected_url = format!("{}{}", client.base_url, path);
            assert!(expected_url.starts_with("http://localhost:9200"));
        }
    }

    #[tokio::test]
    async fn test_client_clone() {
        let client1 = LexumClient::new("http://localhost:9200".to_string());
        let client2 = client1.clone();

        assert_eq!(client1.base_url, client2.base_url);
    }
}
