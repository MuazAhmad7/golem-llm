//! Typesense provider implementation for the golem:search interface
//! 
//! Typesense is an open-source search engine optimized for instant search experiences.
//! It features built-in typo tolerance, faceted search, and geo-search capabilities.

use anyhow::Result;
use log::{debug, error, info};
use std::collections::HashMap;
use std::time::Duration;
use reqwest::{Client, Method, header::{HeaderMap, HeaderValue, CONTENT_TYPE}};
use serde_json::{Value, json};
use url::Url;

// Use the generated WIT types
use golem::search::types::{
    SearchError, Doc, SearchQuery, SearchResults, Schema,
    SearchCapabilities, FieldType, SchemaField,
};

// Helper type alias
type SearchResult<T> = Result<T, SearchError>;

/// Configuration for the Typesense client
#[derive(Debug, Clone)]
pub struct TypesenseConfig {
    pub endpoint: String,
    pub api_key: String,
    pub timeout: Duration,
    pub max_retries: u32,
}

impl TypesenseConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let endpoint = std::env::var("SEARCH_PROVIDER_ENDPOINT")
            .or_else(|_| std::env::var("TYPESENSE_ENDPOINT"))
            .unwrap_or_else(|_| "http://localhost:8108".to_string());

        let api_key = std::env::var("TYPESENSE_API_KEY")
            .or_else(|_| std::env::var("SEARCH_PROVIDER_API_KEY"))
            .map_err(|_| anyhow::anyhow!("TYPESENSE_API_KEY is required"))?;

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
            api_key,
            timeout: Duration::from_secs(timeout),
            max_retries,
        })
    }
}

/// Typesense API client
pub struct TypesenseClient {
    config: TypesenseConfig,
    http_client: Client,
    base_url: Url,
}

impl TypesenseClient {
    /// Create a new Typesense client
    pub fn new(config: TypesenseConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("X-TYPESENSE-API-KEY", HeaderValue::from_str(&config.api_key)?);

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

    /// Execute an HTTP request
    fn request_sync(&self, method: Method, path: &str, body: Option<Value>) -> Result<reqwest::Response> {
        let url = self.base_url.join(path)
            .map_err(|e| anyhow::anyhow!("Failed to build URL: {}", e))?;

        let mut request = self.http_client.request(method, url);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send()
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?;

        Ok(response)
    }

    /// Create a collection (Typesense equivalent of index)
    pub async fn create_collection(&self, schema: Value) -> Result<Value> {
        let response = self.request_sync(Method::POST, "collections", Some(schema))?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to create collection: {}", error_text))
        }
    }

    /// Delete a collection
    pub async fn delete_collection(&self, name: &str) -> Result<Value> {
        let path = format!("collections/{}", name);
        let response = self.request_sync(Method::DELETE, &path, None)?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to delete collection: {}", error_text))
        }
    }

    /// List all collections
    pub async fn list_collections(&self) -> Result<Vec<String>> {
        let response = self.request_sync(Method::GET, "collections", None)?;
        
        if response.status().is_success() {
            let collections: Vec<Value> = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            
            let names = collections.into_iter()
                .filter_map(|collection| {
                    collection.get("name")
                        .and_then(|name| name.as_str())
                        .map(|s| s.to_string())
                })
                .collect();
            
            Ok(names)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to list collections: {}", error_text))
        }
    }

    /// Index a document
    pub async fn index_document(&self, collection: &str, document: Value) -> Result<Value> {
        let path = format!("collections/{}/documents", collection);
        let response = self.request_sync(Method::POST, &path, Some(document))?;
        
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

    /// Upsert a document
    pub async fn upsert_document(&self, collection: &str, document: Value) -> Result<Value> {
        let path = format!("collections/{}/documents?action=upsert", collection);
        let response = self.request_sync(Method::POST, &path, Some(document))?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to upsert document: {}", error_text))
        }
    }

    /// Get a document by ID
    pub async fn get_document(&self, collection: &str, id: &str) -> Result<Option<Value>> {
        let path = format!("collections/{}/documents/{}", collection, id);
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
    pub async fn delete_document(&self, collection: &str, id: &str) -> Result<Value> {
        let path = format!("collections/{}/documents/{}", collection, id);
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

    /// Search documents
    pub async fn search(&self, collection: &str, params: &[(&str, &str)]) -> Result<Value> {
        let path = format!("collections/{}/documents/search", collection);
        let mut url = self.base_url.join(&path)?;
        
        // Add query parameters
        for (key, value) in params {
            url.query_pairs_mut().append_pair(key, value);
        }

        let response = self.http_client.get(url).send()?;
        
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

    /// Get collection schema
    pub async fn get_collection(&self, name: &str) -> Result<Value> {
        let path = format!("collections/{}", name);
        let response = self.request_sync(Method::GET, &path, None)?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to get collection: {}", error_text))
        }
    }
}

/// Map Typesense errors to SearchError
pub fn map_typesense_error(error: anyhow::Error) -> SearchError {
    let error_string = error.to_string();
    
    if error_string.contains("collection not found") || error_string.contains("404") {
        SearchError::IndexNotFound(error_string)
    } else if error_string.contains("bad request") || error_string.contains("400") {
        SearchError::InvalidQuery(error_string)
    } else if error_string.contains("timeout") {
        SearchError::Timeout
    } else if error_string.contains("rate") || error_string.contains("429") {
        SearchError::RateLimited
    } else {
        SearchError::Internal(error_string)
    }
}

/// The Typesense search provider implementation
pub struct TypesenseProvider {
    client: TypesenseClient,
}

impl TypesenseProvider {
    /// Create a new Typesense provider
    pub async fn new() -> SearchResult<Self> {
        let config = TypesenseConfig::from_env()
            .map_err(|e| {
                error!("Failed to load Typesense configuration: {}", e);
                SearchError::Internal(format!("Configuration error: {}", e))
            })?;

        let client = TypesenseClient::new(config)
            .map_err(|e| {
                error!("Failed to create Typesense client: {}", e);
                SearchError::Internal(format!("Client initialization error: {}", e))
            })?;

        info!("Typesense search provider initialized successfully");
        Ok(Self { client })
    }

    /// Get Typesense-specific capabilities
    pub fn get_capabilities(&self) -> SearchCapabilities {
        SearchCapabilities {
            supports_index_creation: true,
            supports_schema_definition: true,
            supports_facets: true,
            supports_highlighting: true,
            supports_full_text_search: true,
            supports_vector_search: true, // Typesense supports vector search
            supports_streaming: false, // Typesense doesn't have scroll API
            supports_geo_search: true,
            supports_aggregations: true,
            max_batch_size: Some(100), // Typesense prefers smaller batches
            max_query_size: Some(2048),
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
                features.insert("typo_tolerance".to_string(), serde_json::Value::String("automatic".to_string()));
                features.insert("instant_search".to_string(), serde_json::Value::String("optimized".to_string()));
                features.insert("geo_search".to_string(), serde_json::Value::String("native".to_string()));
                features.insert("vector_search".to_string(), serde_json::Value::String("supported".to_string()));
                serde_json::to_string(&features).unwrap_or_default()
            },
        }
    }

    /// Convert WIT Schema to Typesense collection schema
    fn schema_to_typesense(&self, schema: &Schema, collection_name: &str) -> SearchResult<Value> {
        let mut fields = Vec::new();
        
        for field in &schema.fields {
            let field_type = match field.field_type {
                FieldType::Text => "string",
                FieldType::Keyword => "string",
                FieldType::Integer => "int32",
                FieldType::Float => "float",
                FieldType::Boolean => "bool",
                FieldType::Date => "int64", // Typesense uses timestamps
                FieldType::GeoPoint => "geopoint",
            };
            
            let mut typesense_field = json!({
                "name": field.name,
                "type": field_type,
                "index": field.index
            });

            // Add faceting support
            if field.facet {
                typesense_field["facet"] = json!(true);
            }

            // Add sorting support
            if field.sort {
                typesense_field["sort"] = json!(true);
            }

            fields.push(typesense_field);
        }
        
        Ok(json!({
            "name": collection_name,
            "fields": fields,
            "default_sorting_field": schema.primary_key.as_ref().unwrap_or(&"id".to_string())
        }))
    }

    /// Convert Typesense collection to WIT Schema
    fn typesense_to_schema(&self, collection: &Value) -> SearchResult<Schema> {
        let fields_array = collection
            .get("fields")
            .and_then(|f| f.as_array())
            .ok_or_else(|| SearchError::Internal("Missing fields in collection".to_string()))?;
        
        let mut fields = Vec::new();
        
        for field in fields_array {
            let name = field
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| SearchError::Internal("Missing field name".to_string()))?
                .to_string();
            
            let field_type_str = field
                .get("type")
                .and_then(|t| t.as_str())
                .ok_or_else(|| SearchError::Internal("Missing field type".to_string()))?;
            
            let field_type = match field_type_str {
                "string" => {
                    // Distinguish between text and keyword based on faceting
                    if field.get("facet").and_then(|f| f.as_bool()).unwrap_or(false) {
                        FieldType::Keyword
                    } else {
                        FieldType::Text
                    }
                }
                "int32" | "int64" => FieldType::Integer,
                "float" => FieldType::Float,
                "bool" => FieldType::Boolean,
                "geopoint" => FieldType::GeoPoint,
                _ => FieldType::Text, // Default fallback
            };
            
            let index = field
                .get("index")
                .and_then(|i| i.as_bool())
                .unwrap_or(true);
            
            let facet = field
                .get("facet")
                .and_then(|f| f.as_bool())
                .unwrap_or(false);
            
            let sort = field
                .get("sort")
                .and_then(|s| s.as_bool())
                .unwrap_or(false);
            
            fields.push(SchemaField {
                name,
                field_type,
                required: false, // Typesense doesn't have required fields
                facet,
                sort,
                index,
            });
        }
        
        let primary_key = collection
            .get("default_sorting_field")
            .and_then(|p| p.as_str())
            .map(|s| s.to_string());
        
        Ok(Schema {
            fields,
            primary_key,
        })
    }

    /// Convert WIT SearchQuery to Typesense search parameters
    fn query_to_typesense_params(&self, query: &SearchQuery) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();
        
        // Main query
        if let Some(ref q) = query.q {
            if !q.trim().is_empty() {
                params.push(("q", q.clone()));
                params.push(("query_by", "*".to_string())); // Search all fields
            }
        } else {
            params.push(("q", "*".to_string()));
            params.push(("query_by", "*".to_string()));
        }
        
        // Filters
        if !query.filters.is_empty() {
            let filter_str = query.filters.join(" && ");
            params.push(("filter_by", filter_str));
        }
        
        // Sorting
        if !query.sort.is_empty() {
            let sort_str = query.sort.join(",");
            params.push(("sort_by", sort_str));
        }
        
        // Pagination
        let per_page = query.per_page.unwrap_or(10);
        params.push(("per_page", per_page.to_string()));
        
        if let Some(page) = query.page {
            params.push(("page", (page + 1).to_string())); // Typesense is 1-indexed
        } else if let Some(offset) = query.offset {
            let page = (offset / per_page) + 1;
            params.push(("page", page.to_string()));
        }
        
        // Facets
        if !query.facets.is_empty() {
            let facet_str = query.facets.join(",");
            params.push(("facet_by", facet_str));
        }
        
        // Highlighting
        if let Some(ref highlight_config) = query.highlight {
            if !highlight_config.fields.is_empty() {
                let highlight_fields = highlight_config.fields.join(",");
                params.push(("highlight_fields", highlight_fields));
                
                if let Some(ref pre_tag) = highlight_config.pre_tag {
                    params.push(("highlight_start_tag", pre_tag.clone()));
                }
                
                if let Some(ref post_tag) = highlight_config.post_tag {
                    params.push(("highlight_end_tag", post_tag.clone()));
                }
            }
        }
        
        params
    }

    /// Convert Typesense search response to WIT SearchResults
    fn response_to_results(&self, response: &Value) -> SearchResult<SearchResults> {
        let found = response
            .get("found")
            .and_then(|f| f.as_u64())
            .map(|f| f as u32);
        
        let hits_array = response
            .get("hits")
            .and_then(|h| h.as_array())
            .ok_or_else(|| SearchError::Internal("Missing hits array in response".to_string()))?;
        
        let mut hits = Vec::new();
        for hit in hits_array {
            let document = hit
                .get("document")
                .ok_or_else(|| SearchError::Internal("Missing document in hit".to_string()))?;
            
            let id = document
                .get("id")
                .and_then(|id| id.as_str())
                .ok_or_else(|| SearchError::Internal("Missing document ID".to_string()))?
                .to_string();
            
            let content = serde_json::to_string(document)
                .map_err(|e| SearchError::Internal(e.to_string()))?;
            
            let score = hit.get("text_match").and_then(|s| s.as_f64());
            
            let highlights = hit.get("highlights")
                .map(|h| serde_json::to_string(h))
                .transpose()
                .map_err(|e| SearchError::Internal(e.to_string()))?;
            
            hits.push(golem::search::types::SearchHit {
                id,
                score,
                content: Some(content),
                highlights,
            });
        }
        
        let facets = response.get("facet_counts")
            .map(|f| serde_json::to_string(f).unwrap_or_default());
        
        let took_ms = response
            .get("search_time_ms")
            .and_then(|t| t.as_u64())
            .map(|t| t as u32);
        
        Ok(SearchResults {
            total: found,
            page: None,
            per_page: None,
            hits,
            facets,
            took_ms,
        })
    }

    /// Basic CRUD and search operations
    pub async fn create_index(&self, name: &str, schema: Option<&Schema>) -> SearchResult<()> {
        info!("Creating Typesense collection: {}", name);

        let collection_schema = if let Some(schema) = schema {
            self.schema_to_typesense(schema, name)?
        } else {
            // Default schema with just an id field
            json!({
                "name": name,
                "fields": [
                    {
                        "name": "id",
                        "type": "string",
                        "facet": false
                    }
                ],
                "default_sorting_field": "id"
            })
        };

        self.client
            .create_collection(collection_schema)
            .await
            .map_err(map_typesense_error)?;

        info!("Successfully created Typesense collection: {}", name);
        Ok(())
    }

    pub async fn delete_index(&self, name: &str) -> SearchResult<()> {
        self.client.delete_collection(name).await.map_err(map_typesense_error)?;
        Ok(())
    }

    pub async fn list_indexes(&self) -> SearchResult<Vec<String>> {
        self.client.list_collections().await.map_err(map_typesense_error)
    }

    pub async fn upsert(&self, index: &str, doc: &Doc) -> SearchResult<()> {
        let mut content: Value = serde_json::from_str(&doc.content)
            .map_err(|e| SearchError::InvalidQuery(e.to_string()))?;
        
        // Ensure the document has an id field
        content["id"] = json!(doc.id);
        
        self.client.upsert_document(index, content).await
            .map_err(map_typesense_error)?;
        Ok(())
    }

    pub async fn get(&self, index: &str, id: &str) -> SearchResult<Option<Doc>> {
        let result = self.client.get_document(index, id).await
            .map_err(map_typesense_error)?;
        
        if let Some(response) = result {
            let content = serde_json::to_string(&response)
                .map_err(|e| SearchError::Internal(e.to_string()))?;
            
            Ok(Some(Doc { 
                id: id.to_string(), 
                content 
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn delete(&self, index: &str, id: &str) -> SearchResult<()> {
        self.client.delete_document(index, id).await
            .map_err(map_typesense_error)?;
        Ok(())
    }

    pub async fn search(&self, index: &str, query: &SearchQuery) -> SearchResult<SearchResults> {
        let params = self.query_to_typesense_params(query);
        let param_refs: Vec<(&str, &str)> = params.iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();
        
        let response = self.client.search(index, &param_refs).await
            .map_err(map_typesense_error)?;
        
        self.response_to_results(&response)
    }

    pub async fn get_schema(&self, index: &str) -> SearchResult<Schema> {
        let collection = self.client.get_collection(index).await
            .map_err(map_typesense_error)?;
        
        self.typesense_to_schema(&collection)
    }
}

// WIT bindings
wit_bindgen::generate!({
    world: "typesense-provider",
    path: "wit",
    generate_unused_types: true,
    with: {
        "golem:search/types@1.0.0": generate,
        "golem:search/core@1.0.0": generate,
    },
});

use exports::golem::search::core::Guest;

// Export the implementation
struct Component;

impl Guest for Component {
    fn search(index: String, query: SearchQuery) -> SearchResult<SearchResults> {
        // Synchronous wrapper for the async implementation
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = TypesenseProvider::new().await?;
            provider.search(&index, &query).await
        })
    }

    fn upsert(index: String, doc: Doc) -> SearchResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = TypesenseProvider::new().await?;
            provider.upsert(&index, &doc).await
        })
    }

    fn get(index: String, id: String) -> SearchResult<Option<Doc>> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = TypesenseProvider::new().await?;
            provider.get(&index, &id).await
        })
    }

    fn delete(index: String, id: String) -> SearchResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = TypesenseProvider::new().await?;
            provider.delete(&index, &id).await
        })
    }

    fn create_index(name: String, schema: Option<Schema>) -> SearchResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = TypesenseProvider::new().await?;
            provider.create_index(&name, schema.as_ref()).await
        })
    }

    fn delete_index(name: String) -> SearchResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = TypesenseProvider::new().await?;
            provider.delete_index(&name).await
        })
    }

    fn list_indexes() -> SearchResult<Vec<String>> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = TypesenseProvider::new().await?;
            provider.list_indexes().await
        })
    }

    fn get_schema(index: String) -> SearchResult<Schema> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = TypesenseProvider::new().await?;
            provider.get_schema(&index).await
        })
    }

    fn get_capabilities() -> SearchCapabilities {
        // Create a minimal provider instance for capabilities (doesn't need actual connection)
        let config = TypesenseConfig {
            endpoint: "http://localhost:8108".to_string(),
            api_key: "dummy".to_string(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
        };
        
        let client = TypesenseClient::new(config).unwrap();
        let provider = TypesenseProvider { client };
        provider.get_capabilities()
    }

    fn batch_upsert(index: String, docs: Vec<Doc>) -> SearchResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = TypesenseProvider::new().await?;
            // Typesense doesn't have native batch upsert, so we'll do sequential upserts
            for doc in docs {
                provider.upsert(&index, &doc).await?;
            }
            Ok(())
        })
    }

    fn health_check() -> SearchResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = TypesenseProvider::new().await?;
            // Simple health check by listing collections
            provider.list_indexes().await.map(|_| ())
        })
    }
}