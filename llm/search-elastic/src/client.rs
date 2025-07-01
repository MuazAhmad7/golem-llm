//! ElasticSearch client implementation with authentication and connection management

use std::collections::HashMap;
use std::time::Duration;
use anyhow::{anyhow, Result};
use reqwest::{Client, Method, Response, header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;
use base64::Engine as _;

/// Configuration for the ElasticSearch client
#[derive(Debug, Clone)]
pub struct ElasticConfig {
    pub endpoint: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub api_key: Option<String>,
    pub cloud_id: Option<String>,
    pub timeout: Duration,
    pub max_retries: u32,
}

impl ElasticConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let endpoint = std::env::var("SEARCH_PROVIDER_ENDPOINT")
            .or_else(|_| std::env::var("ELASTICSEARCH_ENDPOINT"))
            .unwrap_or_else(|_| "http://localhost:9200".to_string());

        let username = std::env::var("ELASTICSEARCH_USERNAME")
            .or_else(|_| std::env::var("ELASTIC_USERNAME"))
            .ok();

        let password = std::env::var("ELASTICSEARCH_PASSWORD")
            .or_else(|_| std::env::var("ELASTIC_PASSWORD"))
            .ok();

        let api_key = std::env::var("ELASTICSEARCH_API_KEY")
            .or_else(|_| std::env::var("ELASTIC_API_KEY"))
            .ok();

        let cloud_id = std::env::var("ELASTIC_CLOUD_ID").ok();

        let timeout = std::env::var("SEARCH_PROVIDER_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .map_err(|_| anyhow!("Invalid timeout value"))?;

        let max_retries = std::env::var("SEARCH_PROVIDER_MAX_RETRIES")
            .unwrap_or_else(|_| "3".to_string())
            .parse::<u32>()
            .map_err(|_| anyhow!("Invalid max_retries value"))?;

        // If cloud_id is provided, parse it to get the endpoint
        let final_endpoint = if let Some(ref cloud_id) = cloud_id {
            parse_cloud_id(cloud_id)?
        } else {
            endpoint
        };

        Ok(Self {
            endpoint: final_endpoint,
            username,
            password,
            api_key,
            cloud_id,
            timeout: Duration::from_secs(timeout),
            max_retries,
        })
    }
}

/// Parse Elastic Cloud ID to get the endpoint
fn parse_cloud_id(cloud_id: &str) -> Result<String> {
    let parts: Vec<&str> = cloud_id.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid cloud_id format"));
    }

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(parts[1])
        .map_err(|_| anyhow!("Failed to decode cloud_id"))?;
    
    let decoded_str = String::from_utf8(decoded)
        .map_err(|_| anyhow!("Invalid UTF-8 in cloud_id"))?;

    let endpoint_parts: Vec<&str> = decoded_str.split('$').collect();
    if endpoint_parts.is_empty() {
        return Err(anyhow!("Invalid cloud_id content"));
    }

    Ok(format!("https://{}", endpoint_parts[0]))
}

/// ElasticSearch API client
pub struct ElasticClient {
    config: ElasticConfig,
    http_client: Client,
    base_url: Url,
}

impl ElasticClient {
    /// Create a new ElasticSearch client
    pub fn new(config: ElasticConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let http_client = Client::builder()
            .timeout(config.timeout)
            .default_headers(headers)
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        let base_url = Url::parse(&config.endpoint)
            .map_err(|e| anyhow!("Invalid endpoint URL: {}", e))?;

        Ok(Self {
            config,
            http_client,
            base_url,
        })
    }

    /// Execute an HTTP request with authentication - synchronous version for now
    fn request_sync(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Response> {
        let url = self.base_url.join(path)
            .map_err(|e| anyhow!("Failed to build URL: {}", e))?;

        let mut request = self.http_client.request(method, url);

        // Add authentication
        if let Some(ref api_key) = self.config.api_key {
            request = request.header(AUTHORIZATION, format!("ApiKey {}", api_key));
        } else if let (Some(ref username), Some(ref password)) = 
            (&self.config.username, &self.config.password) {
            let auth = base64::engine::general_purpose::STANDARD
                .encode(format!("{}:{}", username, password));
            request = request.header(AUTHORIZATION, format!("Basic {}", auth));
        }

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send()
            .map_err(|e| anyhow!("Request failed: {}", e))?;

        Ok(response)
    }

    /// Check cluster health
    pub async fn health_check(&self) -> Result<bool> {
        let response = self.request_sync(Method::GET, "_cluster/health", None)?;
        Ok(response.status().is_success())
    }

    /// Create an index
    pub async fn create_index(&self, name: &str, settings: Option<Value>) -> Result<Value> {
        let body = settings.unwrap_or_else(|| json!({}));
        let response = self.request_sync(Method::PUT, name, Some(body))?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Failed to create index: {}", error_text))
        }
    }

    /// Delete an index
    pub async fn delete_index(&self, name: &str) -> Result<Value> {
        let response = self.request_sync(Method::DELETE, name, None)?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Failed to delete index: {}", error_text))
        }
    }

    /// List all indexes
    pub async fn list_indexes(&self) -> Result<Vec<String>> {
        let response = self.request_sync(Method::GET, "_cat/indices?format=json", None)?;
        
        if response.status().is_success() {
            let indices: Vec<Value> = response.json()
                .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
            
            let names = indices.into_iter()
                .filter_map(|index| {
                    index.get("index")
                        .and_then(|name| name.as_str())
                        .map(|s| s.to_string())
                })
                .collect();
            
            Ok(names)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Failed to list indexes: {}", error_text))
        }
    }

    /// Index a document
    pub async fn index_document(
        &self,
        index: &str,
        id: &str,
        document: Value,
    ) -> Result<Value> {
        let path = format!("{}/_doc/{}", index, id);
        let response = self.request_sync(Method::PUT, &path, Some(document))?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Failed to index document: {}", error_text))
        }
    }

    /// Get a document by ID
    pub async fn get_document(&self, index: &str, id: &str) -> Result<Option<Value>> {
        let path = format!("{}/_doc/{}", index, id);
        let response = self.request_sync(Method::GET, &path, None)?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
            Ok(Some(result))
        } else if response.status().as_u16() == 404 {
            Ok(None)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Failed to get document: {}", error_text))
        }
    }

    /// Delete a document by ID
    pub async fn delete_document(&self, index: &str, id: &str) -> Result<Value> {
        let path = format!("{}/_doc/{}", index, id);
        let response = self.request_sync(Method::DELETE, &path, None)?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Failed to delete document: {}", error_text))
        }
    }

    /// Bulk operation
    pub async fn bulk(&self, operations: Vec<Value>) -> Result<Value> {
        let mut body = String::new();
        for op in operations {
            body.push_str(&serde_json::to_string(&op)?);
            body.push('\n');
        }

        let url = self.base_url.join("_bulk")?;
        let response = self.http_client
            .post(url)
            .header(CONTENT_TYPE, "application/x-ndjson")
            .body(body)
            .send()?;

        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Bulk operation failed: {}", error_text))
        }
    }

    /// Search documents
    pub async fn search(&self, index: &str, query: Value) -> Result<Value> {
        let path = format!("{}/_search", index);
        let response = self.request_sync(Method::POST, &path, Some(query))?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Search failed: {}", error_text))
        }
    }

    /// Get index mapping
    pub async fn get_mapping(&self, index: &str) -> Result<Value> {
        let path = format!("{}/_mapping", index);
        let response = self.request_sync(Method::GET, &path, None)?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Failed to get mapping: {}", error_text))
        }
    }

    /// Put index mapping
    pub async fn put_mapping(&self, index: &str, mapping: Value) -> Result<Value> {
        let path = format!("{}/_mapping", index);
        let response = self.request_sync(Method::PUT, &path, Some(mapping))?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!("Failed to put mapping: {}", error_text))
        }
    }
}