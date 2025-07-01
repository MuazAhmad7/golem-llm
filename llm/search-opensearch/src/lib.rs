//! OpenSearch provider implementation for the golem:search interface
//! 
//! OpenSearch is a fork of ElasticSearch and maintains API compatibility,
//! so this implementation largely reuses ElasticSearch patterns.

use anyhow::Result;
use log::{debug, error, info};
use std::collections::HashMap;
use std::time::Duration;
use reqwest::{Client, Method, header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}};
use serde_json::{Value, json};
use url::Url;
use base64::Engine as _;

use golem_search::{
    SearchError, SearchResult, Doc, SearchQuery, SearchResults, Schema,
    SearchCapabilities, FieldType, SchemaField,
};

/// Configuration for the OpenSearch client
#[derive(Debug, Clone)]
pub struct OpenSearchConfig {
    pub endpoint: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub api_key: Option<String>,
    pub timeout: Duration,
    pub max_retries: u32,
}

impl OpenSearchConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let endpoint = std::env::var("SEARCH_PROVIDER_ENDPOINT")
            .or_else(|_| std::env::var("OPENSEARCH_ENDPOINT"))
            .unwrap_or_else(|_| "http://localhost:9200".to_string());

        let username = std::env::var("OPENSEARCH_USERNAME")
            .or_else(|_| std::env::var("OPENSEARCH_USER"))
            .ok();

        let password = std::env::var("OPENSEARCH_PASSWORD")
            .or_else(|_| std::env::var("OPENSEARCH_PASS"))
            .ok();

        let api_key = std::env::var("OPENSEARCH_API_KEY").ok();

        let timeout = std::env::var("SEARCH_PROVIDER_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("Invalid timeout value"))?;

        let max_retries = std::env::var("SEARCH_PROVIDER_MAX_RETRIES")
            .unwrap_or_else(|_| "3".to_string())
            .parse::<u32>()
            .map_err(|_| anyhow::anyhow!("Invalid max_retries value"))?;

        Ok(Self {
            endpoint,
            username,
            password,
            api_key,
            timeout: Duration::from_secs(timeout),
            max_retries,
        })
    }
}

/// OpenSearch API client - similar to ElasticSearch client
pub struct OpenSearchClient {
    config: OpenSearchConfig,
    http_client: Client,
    base_url: Url,
}

impl OpenSearchClient {
    /// Create a new OpenSearch client
    pub fn new(config: OpenSearchConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let http_client = Client::builder()
            .timeout(config.timeout)
            .default_headers(headers)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

        let base_url = Url::parse(&config.endpoint)
            .map_err(|e| anyhow::anyhow!("Invalid endpoint URL: {}", e))?;

        Ok(Self {
            config,
            http_client,
            base_url,
        })
    }

    /// Execute an HTTP request with authentication
    fn request_sync(&self, method: Method, path: &str, body: Option<Value>) -> Result<reqwest::Response> {
        let url = self.base_url.join(path)
            .map_err(|e| anyhow::anyhow!("Failed to build URL: {}", e))?;

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
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?;

        Ok(response)
    }

    /// Create an index
    pub async fn create_index(&self, name: &str, settings: Option<Value>) -> Result<Value> {
        let body = settings.unwrap_or_else(|| json!({}));
        let response = self.request_sync(Method::PUT, name, Some(body))?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to create index: {}", error_text))
        }
    }

    /// Delete an index
    pub async fn delete_index(&self, name: &str) -> Result<Value> {
        let response = self.request_sync(Method::DELETE, name, None)?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to delete index: {}", error_text))
        }
    }

    /// List all indexes
    pub async fn list_indexes(&self) -> Result<Vec<String>> {
        let response = self.request_sync(Method::GET, "_cat/indices?format=json", None)?;
        
        if response.status().is_success() {
            let indices: Vec<Value> = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            
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
            Err(anyhow::anyhow!("Failed to list indexes: {}", error_text))
        }
    }

    /// Index a document
    pub async fn index_document(&self, index: &str, id: &str, document: Value) -> Result<Value> {
        let path = format!("{}/_doc/{}", index, id);
        let response = self.request_sync(Method::PUT, &path, Some(document))?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to index document: {}", error_text))
        }
    }

    /// Get a document by ID
    pub async fn get_document(&self, index: &str, id: &str) -> Result<Option<Value>> {
        let path = format!("{}/_doc/{}", index, id);
        let response = self.request_sync(Method::GET, &path, None)?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(Some(result))
        } else if response.status().as_u16() == 404 {
            Ok(None)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to get document: {}", error_text))
        }
    }

    /// Delete a document by ID
    pub async fn delete_document(&self, index: &str, id: &str) -> Result<Value> {
        let path = format!("{}/_doc/{}", index, id);
        let response = self.request_sync(Method::DELETE, &path, None)?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to delete document: {}", error_text))
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
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Bulk operation failed: {}", error_text))
        }
    }

    /// Search documents
    pub async fn search(&self, index: &str, query: Value) -> Result<Value> {
        let path = format!("{}/_search", index);
        let response = self.request_sync(Method::POST, &path, Some(query))?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Search failed: {}", error_text))
        }
    }

    /// Get index mapping
    pub async fn get_mapping(&self, index: &str) -> Result<Value> {
        let path = format!("{}/_mapping", index);
        let response = self.request_sync(Method::GET, &path, None)?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to get mapping: {}", error_text))
        }
    }

    /// Put index mapping
    pub async fn put_mapping(&self, index: &str, mapping: Value) -> Result<Value> {
        let path = format!("{}/_mapping", index);
        let response = self.request_sync(Method::PUT, &path, Some(mapping))?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to put mapping: {}", error_text))
        }
    }
}

/// Map OpenSearch errors to SearchError
pub fn map_opensearch_error(error: anyhow::Error) -> SearchError {
    let error_string = error.to_string();
    
    if error_string.contains("index_not_found") || error_string.contains("404") {
        SearchError::IndexNotFound(error_string)
    } else if error_string.contains("parsing_exception") || error_string.contains("400") {
        SearchError::InvalidQuery(error_string)
    } else if error_string.contains("timeout") {
        SearchError::Timeout
    } else if error_string.contains("rate") || error_string.contains("429") {
        SearchError::RateLimited
    } else {
        SearchError::Internal(error_string)
    }
}

/// The OpenSearch search provider implementation
pub struct OpenSearchProvider {
    client: OpenSearchClient,
}

impl OpenSearchProvider {
    /// Create a new OpenSearch provider
    pub async fn new() -> SearchResult<Self> {
        let config = OpenSearchConfig::from_env()
            .map_err(|e| {
                error!("Failed to load OpenSearch configuration: {}", e);
                SearchError::Internal(format!("Configuration error: {}", e))
            })?;

        let client = OpenSearchClient::new(config)
            .map_err(|e| {
                error!("Failed to create OpenSearch client: {}", e);
                SearchError::Internal(format!("Client initialization error: {}", e))
            })?;

        info!("OpenSearch search provider initialized successfully");
        Ok(Self { client })
    }

    /// Get OpenSearch-specific capabilities
    pub fn get_capabilities(&self) -> SearchCapabilities {
        SearchCapabilities {
            supports_index_creation: true,
            supports_schema_definition: true,
            supports_facets: true,
            supports_highlighting: true,
            supports_full_text_search: true,
            supports_vector_search: true, // OpenSearch has native vector search support
            supports_streaming: true, // Via scroll API
            supports_geo_search: true,
            supports_aggregations: true,
            max_batch_size: Some(1000),
            max_query_size: Some(32768),
            supported_field_types: vec![
                FieldType::Text,
                FieldType::Keyword,
                FieldType::Integer,
                FieldType::Float,
                FieldType::Boolean,
                FieldType::Date,
                FieldType::GeoPoint,
            ],
            provider_features: {
                let mut features = HashMap::new();
                features.insert("vector_search".to_string(), serde_json::Value::String("native".to_string()));
                features.insert("neural_search".to_string(), serde_json::Value::String("supported".to_string()));
                features.insert("anomaly_detection".to_string(), serde_json::Value::String("supported".to_string()));
                features
            },
        }
    }

    /// Create an index
    pub async fn create_index(&self, name: &str, schema: Option<&Schema>) -> SearchResult<()> {
        info!("Creating OpenSearch index: {}", name);

        let settings = if let Some(schema) = schema {
            // Convert schema to OpenSearch mapping (same as ElasticSearch)
            let mapping = self.schema_to_mapping(schema)?;
            Some(mapping)
        } else {
            None
        };

        self.client
            .create_index(name, settings)
            .await
            .map_err(|e| {
                error!("Failed to create index {}: {}", name, e);
                map_opensearch_error(e)
            })?;

        info!("Successfully created OpenSearch index: {}", name);
        Ok(())
    }

    /// Convert schema to OpenSearch mapping (reuse ElasticSearch logic)
    fn schema_to_mapping(&self, schema: &Schema) -> SearchResult<Value> {
        let mut properties = serde_json::Map::new();
        
        for field in &schema.fields {
            let field_mapping = match field.field_type {
                FieldType::Text => {
                    json!({
                        "type": "text",
                        "index": field.index,
                        "analyzer": "standard"
                    })
                }
                FieldType::Keyword => {
                    json!({
                        "type": "keyword",
                        "index": field.index
                    })
                }
                FieldType::Integer => {
                    json!({
                        "type": "integer",
                        "index": field.index
                    })
                }
                FieldType::Float => {
                    json!({
                        "type": "float",
                        "index": field.index
                    })
                }
                FieldType::Boolean => {
                    json!({
                        "type": "boolean",
                        "index": field.index
                    })
                }
                FieldType::Date => {
                    json!({
                        "type": "date",
                        "index": field.index,
                        "format": "strict_date_optional_time||epoch_millis"
                    })
                }
                FieldType::GeoPoint => {
                    json!({
                        "type": "geo_point",
                        "index": field.index
                    })
                }
            };
            
            properties.insert(field.name.clone(), field_mapping);
        }
        
        Ok(json!({
            "mappings": {
                "properties": properties
            }
        }))
    }

    /// Convert query to OpenSearch format (reuse ElasticSearch logic)
    fn query_to_opensearch(&self, query: &SearchQuery) -> SearchResult<Value> {
        let mut opensearch_query = json!({
            "query": {
                "bool": {
                    "must": [],
                    "filter": []
                }
            }
        });
        
        // Add main query
        if let Some(ref q) = query.q {
            if !q.trim().is_empty() {
                let query_part = json!({
                    "multi_match": {
                        "query": q,
                        "type": "best_fields",
                        "operator": "or"
                    }
                });
                opensearch_query["query"]["bool"]["must"]
                    .as_array_mut()
                    .unwrap()
                    .push(query_part);
            }
        }
        
        // Add filters
        for filter in &query.filters {
            if let Some((field, value)) = filter.split_once(':') {
                let filter_part = json!({
                    "term": {
                        field: value
                    }
                });
                opensearch_query["query"]["bool"]["filter"]
                    .as_array_mut()
                    .unwrap()
                    .push(filter_part);
            }
        }
        
        // Add pagination
        if let Some(page) = query.page {
            let per_page = query.per_page.unwrap_or(10);
            opensearch_query["from"] = json!(page * per_page);
            opensearch_query["size"] = json!(per_page);
        } else if let Some(offset) = query.offset {
            let size = query.per_page.unwrap_or(10);
            opensearch_query["from"] = json!(offset);
            opensearch_query["size"] = json!(size);
        } else {
            opensearch_query["size"] = json!(query.per_page.unwrap_or(10));
        }
        
        Ok(opensearch_query)
    }

    /// Convert OpenSearch response to search results (reuse ElasticSearch logic)
    fn response_to_results(&self, response: &Value) -> SearchResult<SearchResults> {
        let hits_obj = response
            .get("hits")
            .ok_or_else(|| SearchError::Internal("Missing hits in response".to_string()))?;
        
        let total = hits_obj
            .get("total")
            .and_then(|t| {
                if t.is_number() {
                    t.as_u64()
                } else {
                    t.get("value").and_then(|v| v.as_u64())
                }
            })
            .map(|t| t as u32);
        
        let hits_array = hits_obj
            .get("hits")
            .and_then(|h| h.as_array())
            .ok_or_else(|| SearchError::Internal("Missing hits array in response".to_string()))?;
        
        let mut hits = Vec::new();
        for hit in hits_array {
            let id = hit
                .get("_id")
                .and_then(|id| id.as_str())
                .ok_or_else(|| SearchError::Internal("Missing document ID".to_string()))?
                .to_string();
            
            let source = hit.get("_source");
            let content = if let Some(source) = source {
                Some(serde_json::to_string(source)
                    .map_err(|e| SearchError::Internal(e.to_string()))?)
            } else {
                None
            };
            
            let score = hit.get("_score").and_then(|s| s.as_f64());
            let highlights = hit.get("highlight")
                .map(|h| serde_json::to_string(h))
                .transpose()
                .map_err(|e| SearchError::Internal(e.to_string()))?;
            
            hits.push(golem_search::SearchHit {
                id,
                score,
                content,
                highlights,
            });
        }
        
        let facets = response.get("aggregations")
            .map(|aggs| serde_json::to_string(aggs).unwrap_or_default());
        
        let took_ms = response
            .get("took")
            .and_then(|t| t.as_u64())
            .map(|t| t as u32);
        
        Ok(SearchResults {
            total,
            page: None,
            per_page: None,
            hits,
            facets,
            took_ms,
        })
    }

    /// Basic CRUD and search operations
    pub async fn delete_index(&self, name: &str) -> SearchResult<()> {
        self.client.delete_index(name).await.map_err(map_opensearch_error)?;
        Ok(())
    }

    pub async fn list_indexes(&self) -> SearchResult<Vec<String>> {
        self.client.list_indexes().await.map_err(map_opensearch_error)
    }

    pub async fn upsert(&self, index: &str, doc: &Doc) -> SearchResult<()> {
        let content: Value = serde_json::from_str(&doc.content)
            .map_err(|e| SearchError::InvalidQuery(e.to_string()))?;
        
        self.client.index_document(index, &doc.id, content).await
            .map_err(map_opensearch_error)?;
        Ok(())
    }

    pub async fn get(&self, index: &str, id: &str) -> SearchResult<Option<Doc>> {
        let result = self.client.get_document(index, id).await
            .map_err(map_opensearch_error)?;
        
        if let Some(response) = result {
            let id = response.get("_id")
                .and_then(|id| id.as_str())
                .ok_or_else(|| SearchError::Internal("Missing document ID".to_string()))?
                .to_string();
            
            let source = response.get("_source")
                .ok_or_else(|| SearchError::Internal("Missing document source".to_string()))?;
            
            let content = serde_json::to_string(source)
                .map_err(|e| SearchError::Internal(e.to_string()))?;
            
            Ok(Some(Doc { id, content }))
        } else {
            Ok(None)
        }
    }

    pub async fn search(&self, index: &str, query: &SearchQuery) -> SearchResult<SearchResults> {
        let opensearch_query = self.query_to_opensearch(query)?;
        let response = self.client.search(index, opensearch_query).await
            .map_err(map_opensearch_error)?;
        self.response_to_results(&response)
    }
}