//! HTTP client for Lexum API

use anyhow::Result;
use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Lexum HTTP client
pub struct LexumClient {
    base_url: String,
    client: Client,
}

impl LexumClient {
    /// Create new client
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
        }
    }

    /// GET request
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            anyhow::bail!("HTTP {status} - {text}");
        }

        Ok(response.json().await?)
    }

    /// POST request
    pub async fn post<T: Serialize, R: DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.post(&url).json(body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            anyhow::bail!("HTTP {status} - {text}");
        }

        Ok(response.json().await?)
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
