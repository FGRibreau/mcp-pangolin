//! HTTP client for making Pangolin API calls

use anyhow::{Context, Result};
use reqwest::Client;
use std::collections::HashMap;
use tracing::debug;
use url::Url;

use crate::swagger::build_url;
use crate::types::HttpMethod;

/// HTTP client for making Pangolin API calls
#[derive(Debug, Clone)]
pub struct PangolinClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl PangolinClient {
    /// Create a new Pangolin client
    pub fn new(base_url: &str, api_key: String) -> Result<Self> {
        // Validate the URL
        Url::parse(base_url).context("Invalid base URL")?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            base_url: base_url.to_string(),
            api_key,
        })
    }

    /// Call a Pangolin API endpoint
    pub async fn call(
        &self,
        method: HttpMethod,
        path: &str,
        path_params: HashMap<String, String>,
        query_params: HashMap<String, String>,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        // Build the URL with path parameters
        let url = build_url(&self.base_url, path, &path_params);

        debug!("Calling Pangolin API: {} {}", method.as_str(), url);

        // Build the request
        let mut request = match method {
            HttpMethod::Get => self.client.get(&url),
            HttpMethod::Post => self.client.post(&url),
            HttpMethod::Put => self.client.put(&url),
            HttpMethod::Delete => self.client.delete(&url),
            HttpMethod::Patch => self.client.patch(&url),
        };

        // Add Bearer token authentication
        request = request.header("Authorization", format!("Bearer {}", self.api_key));

        // Add query parameters
        if !query_params.is_empty() {
            request = request.query(&query_params);
        }

        // Add JSON body if present
        if let Some(body) = body {
            request = request
                .header("Content-Type", "application/json")
                .json(&body);
        }

        // Send the request
        let response = request
            .send()
            .await
            .context("Failed to send request to Pangolin API")?;

        let status = response.status();
        let text = response.text().await.context("Failed to read response")?;

        debug!("Response status: {}, body length: {}", status, text.len());

        if !status.is_success() {
            // Try to parse error response as JSON for better error messages
            let error_msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| {
                    v.get("message")
                        .or_else(|| v.get("error"))
                        .and_then(|m| m.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or(text.clone());

            anyhow::bail!("Pangolin API error ({}): {}", status, error_msg);
        }

        // Try to parse as JSON, fallback to string value
        let json: serde_json::Value = if text.is_empty() {
            serde_json::json!({"status": "success"})
        } else {
            serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text))
        };

        Ok(json)
    }
}
