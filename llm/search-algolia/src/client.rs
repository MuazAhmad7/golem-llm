use std::collections::HashMap;
use std::time::Duration;
use anyhow::{anyhow, Result};
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

/// Configuration for the Algolia client
#[derive(Debug, Clone)]
pub struct AlgoliaConfig {
    pub app_id: String,
    pub api_key: String,
    pub timeout: Duration,
}

impl AlgoliaConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let app_id = std::env::var("ALGOLIA_APP_ID")
            .map_err(|_| anyhow!("ALGOLIA_APP_ID environment variable is required"))?;
        let api_key = std::env::var("ALGOLIA_API_KEY")
            .map_err(|_| anyhow!("ALGOLIA_API_KEY environment variable is required"))?;
        
        let timeout = std::env::var("SEARCH_PROVIDER_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .map_err(|_| anyhow!("Invalid timeout value"))?;

        Ok(Self {
            app_id,
            api_key,
            timeout: Duration::from_secs(timeout),
        })
    }
}

/// Algolia API client
pub struct AlgoliaClient {
    config: AlgoliaConfig,
    http_client: Client,
}

impl AlgoliaClient {
    /// Create a new Algolia client
    pub fn new(config: AlgoliaConfig) -> Result<Self> {
        let http_client = Client::builder()
            .pool_idle_timeout(Some(Duration::from_secs(30)))
            .pool_max_idle_per_host(10)
            .timeout(config.timeout)
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Get the base URL for API requests
    fn base_url(&self) -> String {
        format!("https://{}-dsn.algolia.net/1", self.config.app_id)
    }

    /// Make an authenticated request to the Algolia API
    async fn request<T: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&T>,
    ) -> Result<Response> {
        let url = format!("{}/{}", self.base_url(), path.trim_start_matches('/'));
        
        let mut request = self.http_client
            .request(method, &url)
            .header("X-Algolia-Application-Id", &self.config.app_id)
            .header("X-Algolia-API-Key", &self.config.api_key)
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Algolia API error {}: {}", status, error_text));
        }

        Ok(response)
    }

    /// Create an index
    pub async fn create_index(&self, name: &str) -> Result<()> {
        // Algolia creates indices automatically when you add data
        // We'll just validate the name here
        if name.is_empty() {
            return Err(anyhow!("Index name cannot be empty"));
        }
        
        // Create empty index by adding a temporary object and then deleting it
        let temp_doc = serde_json::json!({
            "objectID": "__temp_init_object__",
            "temp": true
        });
        
        self.request(Method::POST, &format!("indexes/{}/", name), Some(&temp_doc)).await?;
        self.request(Method::DELETE, &format!("indexes/{}/objects/__temp_init_object__", name), None::<&()>).await?;
        
        Ok(())
    }

    /// Delete an index
    pub async fn delete_index(&self, name: &str) -> Result<()> {
        self.request(Method::DELETE, &format!("indexes/{}", name), None::<&()>).await?;
        Ok(())
    }

    /// List all indices
    pub async fn list_indices(&self) -> Result<Vec<String>> {
        let response = self.request(Method::GET, "indexes", None::<&()>).await?;
        let data: ListIndicesResponse = response.json().await
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        
        Ok(data.items.into_iter().map(|item| item.name).collect())
    }

    /// Update index settings
    pub async fn update_index_settings(&self, name: &str, settings: &AlgoliaIndexSettings) -> Result<()> {
        self.request(Method::PUT, &format!("indexes/{}/settings", name), Some(settings)).await?;
        Ok(())
    }

    /// Add or update a single object
    pub async fn upsert_object(&self, index: &str, object_id: &str, object: &Value) -> Result<()> {
        self.request(Method::PUT, &format!("indexes/{}/objects/{}", index, object_id), Some(object)).await?;
        Ok(())
    }

    /// Batch add or update objects
    pub async fn batch_objects(&self, index: &str, objects: &[Value]) -> Result<Vec<String>> {
        let requests: Vec<BatchRequest> = objects.iter().map(|obj| {
            BatchRequest {
                action: "addObject".to_string(),
                body: obj.clone(),
            }
        }).collect();

        let batch_request = BatchRequestWrapper { requests };
        let response = self.request(Method::POST, &format!("indexes/{}/batch", index), Some(&batch_request)).await?;
        let batch_response: BatchResponse = response.json().await
            .map_err(|e| anyhow!("Failed to parse batch response: {}", e))?;
        
        Ok(batch_response.object_ids)
    }

    /// Get an object by ID
    pub async fn get_object(&self, index: &str, object_id: &str) -> Result<Value> {
        let response = self.request(Method::GET, &format!("indexes/{}/objects/{}", index, object_id), None::<&()>).await?;
        let object: Value = response.json().await
            .map_err(|e| anyhow!("Failed to parse object: {}", e))?;
        Ok(object)
    }

    /// Delete an object by ID
    pub async fn delete_object(&self, index: &str, object_id: &str) -> Result<()> {
        self.request(Method::DELETE, &format!("indexes/{}/objects/{}", index, object_id), None::<&()>).await?;
        Ok(())
    }

    /// Delete multiple objects by IDs
    pub async fn delete_objects(&self, index: &str, object_ids: &[String]) -> Result<()> {
        let requests: Vec<BatchRequest> = object_ids.iter().map(|id| {
            BatchRequest {
                action: "deleteObject".to_string(),
                body: serde_json::json!({ "objectID": id }),
            }
        }).collect();

        let batch_request = BatchRequestWrapper { requests };
        self.request(Method::POST, &format!("indexes/{}/batch", index), Some(&batch_request)).await?;
        Ok(())
    }

    /// Search an index
    pub async fn search(&self, index: &str, query: &AlgoliaSearchQuery) -> Result<AlgoliaSearchResults> {
        let response = self.request(Method::POST, &format!("indexes/{}/query", index), Some(query)).await?;
        let results: AlgoliaSearchResults = response.json().await
            .map_err(|e| anyhow!("Failed to parse search results: {}", e))?;
        Ok(results)
    }
}

// Algolia API types
#[derive(Debug, Serialize, Deserialize)]
pub struct AlgoliaIndexSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub searchable_attributes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes_for_faceting: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unretrievable_attributes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_ranking: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typo_tolerance: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_pre_tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_post_tag: Option<String>,
}

impl Default for AlgoliaIndexSettings {
    fn default() -> Self {
        Self {
            searchable_attributes: None,
            attributes_for_faceting: None,
            unretrievable_attributes: None,
            ranking: None,
            custom_ranking: None,
            typo_tolerance: None,
            highlight_pre_tag: Some("<em>".to_string()),
            highlight_post_tag: Some("</em>".to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlgoliaSearchQuery {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(rename = "hitsPerPage")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hits_per_page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_pre_tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_post_tag: Option<String>,
    #[serde(rename = "attributesToRetrieve")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes_to_retrieve: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlgoliaSearchResults {
    pub hits: Vec<AlgoliaSearchHit>,
    #[serde(rename = "nbHits")]
    pub nb_hits: u32,
    pub page: u32,
    #[serde(rename = "hitsPerPage")]
    pub hits_per_page: u32,
    #[serde(rename = "processingTimeMS")]
    pub processing_time_ms: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facets: Option<HashMap<String, HashMap<String, u32>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlgoliaSearchHit {
    #[serde(rename = "objectID")]
    pub object_id: String,
    #[serde(flatten)]
    pub data: Value,
    #[serde(rename = "_highlightResult")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_result: Option<Value>,
    #[serde(rename = "_rankingInfo")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking_info: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListIndicesResponse {
    items: Vec<IndexInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexInfo {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BatchRequest {
    action: String,
    body: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct BatchRequestWrapper {
    requests: Vec<BatchRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BatchResponse {
    #[serde(rename = "objectIDs")]
    object_ids: Vec<String>,
}