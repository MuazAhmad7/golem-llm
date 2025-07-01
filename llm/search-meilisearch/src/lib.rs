//! Meilisearch provider implementation for the golem:search interface
//! 
//! Meilisearch is an ultra-fast search engine with excellent developer experience.
//! It features instant search, typo tolerance, faceted search, and built-in ranking.

use anyhow::Result;
use log::{error, info};
use std::collections::HashMap;
use std::time::Duration;
use reqwest::{Client, Method, header::{HeaderMap, HeaderValue, CONTENT_TYPE, AUTHORIZATION}};
use serde_json::{Value, json};
use url::Url;

// Use the generated WIT types
use golem::search::types::{
    SearchError, Doc, SearchQuery, SearchResults, Schema,
    SearchCapabilities, FieldType, SchemaField,
};

// Helper type alias
type SearchResult<T> = Result<T, SearchError>;

/// Configuration for the Meilisearch client
#[derive(Debug, Clone)]
pub struct MeilisearchConfig {
    pub endpoint: String,
    pub master_key: Option<String>,
    pub timeout: Duration,
    pub max_retries: u32,
}

impl MeilisearchConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let endpoint = std::env::var("SEARCH_PROVIDER_ENDPOINT")
            .or_else(|_| std::env::var("MEILISEARCH_ENDPOINT"))
            .unwrap_or_else(|_| "http://localhost:7700".to_string());

        let master_key = std::env::var("MEILISEARCH_MASTER_KEY")
            .or_else(|_| std::env::var("SEARCH_PROVIDER_API_KEY"))
            .ok(); // Master key is optional for development

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
            master_key,
            timeout: Duration::from_secs(timeout),
            max_retries,
        })
    }
}

/// Meilisearch API client
pub struct MeilisearchClient {
    config: MeilisearchConfig,
    http_client: Client,
    base_url: Url,
}

impl MeilisearchClient {
    /// Create a new Meilisearch client
    pub fn new(config: MeilisearchConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        
        // Add authorization header if master key is provided
        if let Some(ref master_key) = config.master_key {
            headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", master_key))?);
        }

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

    /// Create an index
    pub async fn create_index(&self, index_name: &str, primary_key: Option<&str>) -> Result<Value> {
        let mut body = json!({
            "uid": index_name
        });

        if let Some(pk) = primary_key {
            body["primaryKey"] = json!(pk);
        }

        let response = self.request_sync(Method::POST, "indexes", Some(body))?;
        
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
    pub async fn delete_index(&self, index_name: &str) -> Result<Value> {
        let path = format!("indexes/{}", index_name);
        let response = self.request_sync(Method::DELETE, &path, None)?;
        
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
        let response = self.request_sync(Method::GET, "indexes", None)?;
        
        if response.status().is_success() {
            let indexes_response: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            
            let empty_vec = vec![];
            let results = indexes_response.get("results")
                .and_then(|r| r.as_array())
                .unwrap_or(&empty_vec);
            
            let names = results.iter()
                .filter_map(|index| {
                    index.get("uid")
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

    /// Get index information
    pub async fn get_index(&self, index_name: &str) -> Result<Value> {
        let path = format!("indexes/{}", index_name);
        let response = self.request_sync(Method::GET, &path, None)?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to get index: {}", error_text))
        }
    }

    /// Update index settings
    pub async fn update_settings(&self, index_name: &str, settings: Value) -> Result<Value> {
        let path = format!("indexes/{}/settings", index_name);
        let response = self.request_sync(Method::PATCH, &path, Some(settings))?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to update settings: {}", error_text))
        }
    }

    /// Get index settings
    pub async fn get_settings(&self, index_name: &str) -> Result<Value> {
        let path = format!("indexes/{}/settings", index_name);
        let response = self.request_sync(Method::GET, &path, None)?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to get settings: {}", error_text))
        }
    }

    /// Add or update documents
    pub async fn add_documents(&self, index_name: &str, documents: Value) -> Result<Value> {
        let path = format!("indexes/{}/documents", index_name);
        let response = self.request_sync(Method::POST, &path, Some(documents))?;
        
        if response.status().is_success() || response.status().as_u16() == 202 {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to add documents: {}", error_text))
        }
    }

    /// Get a document by ID
    pub async fn get_document(&self, index_name: &str, id: &str) -> Result<Option<Value>> {
        let path = format!("indexes/{}/documents/{}", index_name, id);
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
    pub async fn delete_document(&self, index_name: &str, id: &str) -> Result<Value> {
        let path = format!("indexes/{}/documents/{}", index_name, id);
        let response = self.request_sync(Method::DELETE, &path, None)?;
        
        if response.status().is_success() || response.status().as_u16() == 202 {
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
    pub async fn search(&self, index_name: &str, query: Value) -> Result<Value> {
        let path = format!("indexes/{}/search", index_name);
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

    /// Get stats for the instance
    pub async fn get_stats(&self) -> Result<Value> {
        let response = self.request_sync(Method::GET, "stats", None)?;
        
        if response.status().is_success() {
            let result: Value = response.json()
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            Ok(result)
        } else {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to get stats: {}", error_text))
        }
    }
}

/// Map Meilisearch errors to SearchError
pub fn map_meilisearch_error(error: anyhow::Error) -> SearchError {
    let error_string = error.to_string();
    
    if error_string.contains("index_not_found") || error_string.contains("404") {
        SearchError::IndexNotFound(error_string)
    } else if error_string.contains("invalid_request") || error_string.contains("400") {
        SearchError::InvalidQuery(error_string)
    } else if error_string.contains("timeout") {
        SearchError::Timeout
    } else if error_string.contains("rate") || error_string.contains("429") {
        SearchError::RateLimited
    } else {
        SearchError::Internal(error_string)
    }
}

/// The Meilisearch search provider implementation
pub struct MeilisearchProvider {
    client: MeilisearchClient,
}

impl MeilisearchProvider {
    /// Create a new Meilisearch provider
    pub async fn new() -> SearchResult<Self> {
        let config = MeilisearchConfig::from_env()
            .map_err(|e| {
                error!("Failed to load Meilisearch configuration: {}", e);
                SearchError::Internal(format!("Configuration error: {}", e))
            })?;

        let client = MeilisearchClient::new(config)
            .map_err(|e| {
                error!("Failed to create Meilisearch client: {}", e);
                SearchError::Internal(format!("Client initialization error: {}", e))
            })?;

        info!("Meilisearch search provider initialized successfully");
        Ok(Self { client })
    }

    /// Get Meilisearch-specific capabilities
    pub fn get_capabilities(&self) -> SearchCapabilities {
        SearchCapabilities {
            supports_index_creation: true,
            supports_schema_definition: true,
            supports_facets: true,
            supports_highlighting: true,
            supports_full_text_search: true,
            supports_vector_search: true, // Meilisearch supports vector search
            supports_streaming: false, // Meilisearch doesn't have streaming search
            supports_geo_search: true,
            supports_aggregations: false, // Meilisearch doesn't support aggregations
            max_batch_size: Some(1000), // Meilisearch supports large batches
            max_query_size: Some(1000),
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
                features.insert("typo_tolerance".to_string(), serde_json::Value::String("advanced".to_string()));
                features.insert("instant_search".to_string(), serde_json::Value::String("ultra_fast".to_string()));
                features.insert("geo_search".to_string(), serde_json::Value::String("native".to_string()));
                features.insert("custom_ranking".to_string(), serde_json::Value::String("advanced".to_string()));
                features.insert("facet_search".to_string(), serde_json::Value::String("native".to_string()));
                serde_json::to_string(&features).unwrap_or_default()
            },
        }
    }

    /// Convert WIT Schema to Meilisearch settings
    fn schema_to_meilisearch_settings(&self, schema: &Schema) -> SearchResult<Value> {
        let mut searchable_attributes = Vec::new();
        let mut filterable_attributes = Vec::new();
        let mut sortable_attributes = Vec::new();
        
        for field in &schema.fields {
            // Add to searchable attributes if it's a text field
            if matches!(field.field_type, FieldType::Text) && field.index {
                searchable_attributes.push(&field.name);
            }
            
            // Add to filterable attributes if facet is enabled
            if field.facet {
                filterable_attributes.push(&field.name);
            }
            
            // Add to sortable attributes if sort is enabled
            if field.sort {
                sortable_attributes.push(&field.name);
            }
        }
        
        let mut settings = json!({});
        
        if !searchable_attributes.is_empty() {
            settings["searchableAttributes"] = json!(searchable_attributes);
        }
        
        if !filterable_attributes.is_empty() {
            settings["filterableAttributes"] = json!(filterable_attributes);
        }
        
        if !sortable_attributes.is_empty() {
            settings["sortableAttributes"] = json!(sortable_attributes);
        }
        
        Ok(settings)
    }

    /// Convert Meilisearch settings to WIT Schema
    fn meilisearch_settings_to_schema(&self, settings: &Value, index_info: &Value) -> SearchResult<Schema> {
        let mut fields = Vec::new();
        
        // Get searchable attributes
        let empty_vec1 = vec![];
        let searchable_attrs = settings.get("searchableAttributes")
            .and_then(|s| s.as_array())
            .unwrap_or(&empty_vec1);
        
        // Get filterable attributes
        let empty_vec2 = vec![];
        let filterable_attrs = settings.get("filterableAttributes")
            .and_then(|f| f.as_array())
            .unwrap_or(&empty_vec2);
        
        // Get sortable attributes
        let empty_vec3 = vec![];
        let sortable_attrs = settings.get("sortableAttributes")
            .and_then(|s| s.as_array())
            .unwrap_or(&empty_vec3);
        
        // Collect all unique field names
        let mut field_names = std::collections::HashSet::new();
        
        for attr in searchable_attrs {
            if let Some(name) = attr.as_str() {
                field_names.insert(name);
            }
        }
        
        for attr in filterable_attrs {
            if let Some(name) = attr.as_str() {
                field_names.insert(name);
            }
        }
        
        for attr in sortable_attrs {
            if let Some(name) = attr.as_str() {
                field_names.insert(name);
            }
        }
        
        // Create schema fields
        for field_name in field_names {
            let is_searchable = searchable_attrs.iter()
                .any(|attr| attr.as_str() == Some(field_name));
            let is_filterable = filterable_attrs.iter()
                .any(|attr| attr.as_str() == Some(field_name));
            let is_sortable = sortable_attrs.iter()
                .any(|attr| attr.as_str() == Some(field_name));
            
            // Determine field type based on name and usage
            let field_type = if is_filterable && !is_searchable {
                FieldType::Keyword
            } else if field_name.contains("date") || field_name.contains("time") {
                FieldType::Date
            } else if field_name.contains("geo") || field_name.contains("location") {
                FieldType::GeoPoint
            } else if field_name.contains("price") || field_name.contains("score") {
                FieldType::Float
            } else if field_name.contains("count") || field_name.contains("number") {
                FieldType::Integer
            } else if field_name.contains("enabled") || field_name.contains("active") {
                FieldType::Boolean
            } else {
                FieldType::Text
            };
            
            fields.push(SchemaField {
                name: field_name.to_string(),
                field_type,
                required: false, // Meilisearch doesn't enforce required fields
                facet: is_filterable,
                sort: is_sortable,
                index: is_searchable,
            });
        }
        
        let primary_key = index_info.get("primaryKey")
            .and_then(|p| p.as_str())
            .map(|s| s.to_string());
        
        Ok(Schema {
            fields,
            primary_key,
        })
    }

    /// Convert WIT SearchQuery to Meilisearch query
    fn query_to_meilisearch(&self, query: &SearchQuery) -> Value {
        let mut meilisearch_query = json!({});
        
        // Main query
        if let Some(ref q) = query.q {
            if !q.trim().is_empty() {
                meilisearch_query["q"] = json!(q);
            }
        }
        
        // Filters
        if !query.filters.is_empty() {
            let filter_str = query.filters.join(" AND ");
            meilisearch_query["filter"] = json!(filter_str);
        }
        
        // Sorting
        if !query.sort.is_empty() {
            meilisearch_query["sort"] = json!(query.sort);
        }
        
        // Pagination
        let limit = query.per_page.unwrap_or(10);
        meilisearch_query["limit"] = json!(limit);
        
        if let Some(page) = query.page {
            let offset = page * limit;
            meilisearch_query["offset"] = json!(offset);
        } else if let Some(offset) = query.offset {
            meilisearch_query["offset"] = json!(offset);
        }
        
        // Facets
        if !query.facets.is_empty() {
            meilisearch_query["facets"] = json!(query.facets);
        }
        
        // Highlighting
        if let Some(ref highlight_config) = query.highlight {
            if !highlight_config.fields.is_empty() {
                meilisearch_query["attributesToHighlight"] = json!(highlight_config.fields);
                
                if let Some(ref pre_tag) = highlight_config.pre_tag {
                    if let Some(ref post_tag) = highlight_config.post_tag {
                        meilisearch_query["highlightPreTag"] = json!(pre_tag);
                        meilisearch_query["highlightPostTag"] = json!(post_tag);
                    }
                }
            }
        }
        
        meilisearch_query
    }

    /// Convert Meilisearch search response to WIT SearchResults
    fn response_to_results(&self, response: &Value) -> SearchResult<SearchResults> {
        let estimated_total_hits = response
            .get("estimatedTotalHits")
            .and_then(|f| f.as_u64())
            .map(|f| f as u32);
        
        let hits_array = response
            .get("hits")
            .and_then(|h| h.as_array())
            .ok_or_else(|| SearchError::Internal("Missing hits array in response".to_string()))?;
        
        let mut hits = Vec::new();
        for hit in hits_array {
            let id = hit
                .get("id")
                .and_then(|id| id.as_str())
                .unwrap_or_else(|| {
                    // Try to find any field that could be an ID
                    hit.as_object()
                        .and_then(|obj| obj.keys().next())
                        .and_then(|first_key| hit.get(first_key))
                        .and_then(|val| val.as_str())
                        .unwrap_or("unknown")
                })
                .to_string();
            
            let content = serde_json::to_string(hit)
                .map_err(|e| SearchError::Internal(e.to_string()))?;
            
            // Meilisearch doesn't provide scores in the same way, but we can use ranking score if available
            let score = hit.get("_rankingScore").and_then(|s| s.as_f64());
            
            let highlights = hit.get("_formatted")
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
        
        let facets = response.get("facetDistribution")
            .map(|f| serde_json::to_string(f).unwrap_or_default());
        
        let took_ms = response
            .get("processingTimeMs")
            .and_then(|t| t.as_u64())
            .map(|t| t as u32);
        
        Ok(SearchResults {
            total: estimated_total_hits,
            page: None,
            per_page: None,
            hits,
            facets,
            took_ms,
        })
    }

    /// Basic CRUD and search operations
    pub async fn create_index(&self, name: &str, schema: Option<&Schema>) -> SearchResult<()> {
        info!("Creating Meilisearch index: {}", name);

        let primary_key = schema.as_ref()
            .and_then(|s| s.primary_key.as_ref())
            .map(|s| s.as_str());

        self.client
            .create_index(name, primary_key)
            .await
            .map_err(map_meilisearch_error)?;

        // Update settings if schema is provided
        if let Some(schema) = schema {
            let settings = self.schema_to_meilisearch_settings(schema)?;
            self.client
                .update_settings(name, settings)
                .await
                .map_err(map_meilisearch_error)?;
        }

        info!("Successfully created Meilisearch index: {}", name);
        Ok(())
    }

    pub async fn delete_index(&self, name: &str) -> SearchResult<()> {
        self.client.delete_index(name).await.map_err(map_meilisearch_error)?;
        Ok(())
    }

    pub async fn list_indexes(&self) -> SearchResult<Vec<String>> {
        self.client.list_indexes().await.map_err(map_meilisearch_error)
    }

    pub async fn upsert(&self, index: &str, doc: &Doc) -> SearchResult<()> {
        let mut content: Value = serde_json::from_str(&doc.content)
            .map_err(|e| SearchError::InvalidQuery(e.to_string()))?;
        
        // Ensure the document has an id field
        content["id"] = json!(doc.id);
        
        // Meilisearch expects an array of documents
        let documents = json!([content]);
        
        self.client.add_documents(index, documents).await
            .map_err(map_meilisearch_error)?;
        Ok(())
    }

    pub async fn get(&self, index: &str, id: &str) -> SearchResult<Option<Doc>> {
        let result = self.client.get_document(index, id).await
            .map_err(map_meilisearch_error)?;
        
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
            .map_err(map_meilisearch_error)?;
        Ok(())
    }

    pub async fn search(&self, index: &str, query: &SearchQuery) -> SearchResult<SearchResults> {
        let meilisearch_query = self.query_to_meilisearch(query);
        
        let response = self.client.search(index, meilisearch_query).await
            .map_err(map_meilisearch_error)?;
        
        self.response_to_results(&response)
    }

    pub async fn get_schema(&self, index: &str) -> SearchResult<Schema> {
        let settings = self.client.get_settings(index).await
            .map_err(map_meilisearch_error)?;
        
        let index_info = self.client.get_index(index).await
            .map_err(map_meilisearch_error)?;
        
        self.meilisearch_settings_to_schema(&settings, &index_info)
    }
}

// WIT bindings
wit_bindgen::generate!({
    world: "meilisearch-provider",
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
            let provider = MeilisearchProvider::new().await?;
            provider.search(&index, &query).await
        })
    }

    fn upsert(index: String, doc: Doc) -> SearchResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = MeilisearchProvider::new().await?;
            provider.upsert(&index, &doc).await
        })
    }

    fn get(index: String, id: String) -> SearchResult<Option<Doc>> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = MeilisearchProvider::new().await?;
            provider.get(&index, &id).await
        })
    }

    fn delete(index: String, id: String) -> SearchResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = MeilisearchProvider::new().await?;
            provider.delete(&index, &id).await
        })
    }

    fn create_index(name: String, schema: Option<Schema>) -> SearchResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = MeilisearchProvider::new().await?;
            provider.create_index(&name, schema.as_ref()).await
        })
    }

    fn delete_index(name: String) -> SearchResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = MeilisearchProvider::new().await?;
            provider.delete_index(&name).await
        })
    }

    fn list_indexes() -> SearchResult<Vec<String>> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = MeilisearchProvider::new().await?;
            provider.list_indexes().await
        })
    }

    fn get_schema(index: String) -> SearchResult<Schema> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = MeilisearchProvider::new().await?;
            provider.get_schema(&index).await
        })
    }

    fn get_capabilities() -> SearchCapabilities {
        // Create a minimal provider instance for capabilities (doesn't need actual connection)
        let config = MeilisearchConfig {
            endpoint: "http://localhost:7700".to_string(),
            master_key: None,
            timeout: Duration::from_secs(30),
            max_retries: 3,
        };
        
        let client = MeilisearchClient::new(config).unwrap();
        let provider = MeilisearchProvider { client };
        provider.get_capabilities()
    }

    fn batch_upsert(index: String, docs: Vec<Doc>) -> SearchResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = MeilisearchProvider::new().await?;
            
            // Meilisearch supports native batch operations
            let mut documents = Vec::new();
            for doc in docs {
                let mut content: Value = serde_json::from_str(&doc.content)
                    .map_err(|e| SearchError::InvalidQuery(e.to_string()))?;
                content["id"] = json!(doc.id);
                documents.push(content);
            }
            
            let documents_array = json!(documents);
            provider.client.add_documents(&index, documents_array).await
                .map_err(map_meilisearch_error)?;
            
            Ok(())
        })
    }

    fn health_check() -> SearchResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SearchError::Internal(format!("Failed to create async runtime: {}", e)))?;
        
        rt.block_on(async {
            let provider = MeilisearchProvider::new().await?;
            // Simple health check by getting stats
            provider.client.get_stats().await.map_err(map_meilisearch_error).map(|_| ())
        })
    }
}