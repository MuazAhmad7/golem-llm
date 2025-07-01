//! Common types and traits for search providers
//!
//! This module defines shared data structures and traits that all search
//! providers must implement to conform to the golem:search interface.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// Re-export WIT-generated types
pub use crate::{
    IndexName, DocumentId, Json, Doc, HighlightConfig, SearchConfig as WitSearchConfig,
    SearchQuery, SearchHit, SearchResults, FieldType, SchemaField, Schema,
    SearchError,
};

/// Capabilities that a search provider supports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCapabilities {
    /// Whether the provider supports index creation
    pub supports_index_creation: bool,
    
    /// Whether the provider supports schema definition
    pub supports_schema_definition: bool,
    
    /// Whether the provider supports faceted search
    pub supports_facets: bool,
    
    /// Whether the provider supports highlighting
    pub supports_highlighting: bool,
    
    /// Whether the provider supports full-text search
    pub supports_full_text_search: bool,
    
    /// Whether the provider supports vector/semantic search
    pub supports_vector_search: bool,
    
    /// Whether the provider supports real-time streaming
    pub supports_streaming: bool,
    
    /// Whether the provider supports geospatial search
    pub supports_geo_search: bool,
    
    /// Whether the provider supports aggregations
    pub supports_aggregations: bool,
    
    /// Maximum number of documents in a batch operation
    pub max_batch_size: Option<u32>,
    
    /// Maximum query size in characters
    pub max_query_size: Option<u32>,
    
    /// Supported field types
    pub supported_field_types: Vec<FieldType>,
    
    /// Provider-specific features
    pub provider_features: HashMap<String, serde_json::Value>,
}

impl Default for SearchCapabilities {
    fn default() -> Self {
        Self {
            supports_index_creation: true,
            supports_schema_definition: true,
            supports_facets: false,
            supports_highlighting: false,
            supports_full_text_search: true,
            supports_vector_search: false,
            supports_streaming: false,
            supports_geo_search: false,
            supports_aggregations: false,
            max_batch_size: Some(100),
            max_query_size: Some(10000),
            supported_field_types: vec![
                FieldType::Text,
                FieldType::Keyword,
                FieldType::Integer,
                FieldType::Float,
                FieldType::Boolean,
            ],
            provider_features: HashMap::new(),
        }
    }
}

/// Search provider statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    /// Total number of indexes
    pub total_indexes: u32,
    
    /// Total number of documents across all indexes
    pub total_documents: u64,
    
    /// Provider uptime in seconds
    pub uptime_seconds: Option<u64>,
    
    /// Provider version
    pub version: Option<String>,
    
    /// Average query response time in milliseconds
    pub avg_query_time_ms: Option<f64>,
    
    /// Current memory usage in bytes
    pub memory_usage_bytes: Option<u64>,
    
    /// Disk usage in bytes
    pub disk_usage_bytes: Option<u64>,
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Index name
    pub name: String,
    
    /// Number of documents in the index
    pub document_count: u64,
    
    /// Index size in bytes
    pub size_bytes: u64,
    
    /// Last update timestamp
    pub last_updated: Option<String>,
    
    /// Index health status
    pub health_status: IndexHealth,
    
    /// Number of shards (if applicable)
    pub shard_count: Option<u32>,
    
    /// Number of replica shards (if applicable)
    pub replica_count: Option<u32>,
}

/// Index health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexHealth {
    Green,
    Yellow,
    Red,
    Unknown,
}

/// Trait that all search providers must implement
pub trait SearchProvider: Send + Sync {
    /// Get the provider's capabilities
    fn get_capabilities(&self) -> SearchCapabilities;
    
    /// Get provider statistics
    fn get_stats(&self) -> crate::error::SearchResult<ProviderStats>;
    
    /// Check if the provider is healthy and ready to accept requests
    fn health_check(&self) -> crate::error::SearchResult<bool>;
    
    /// Get statistics for a specific index
    fn get_index_stats(&self, index_name: &str) -> crate::error::SearchResult<IndexStats>;
    
    /// Validate a query before execution
    fn validate_query(&self, query: &SearchQuery) -> crate::error::SearchResult<()>;
    
    /// Validate a schema before creation/update
    fn validate_schema(&self, schema: &Schema) -> crate::error::SearchResult<()>;
    
    /// Convert provider-specific error to SearchError
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> crate::error::SearchError;
}

/// Query builder utility for constructing search queries
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    query: SearchQuery,
}

impl QueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            query: SearchQuery {
                q: None,
                filters: Vec::new(),
                sort: Vec::new(),
                facets: Vec::new(),
                page: None,
                per_page: None,
                offset: None,
                highlight: None,
                config: None,
            },
        }
    }
    
    /// Set the query string
    pub fn query<S: Into<String>>(mut self, q: S) -> Self {
        self.query.q = Some(q.into());
        self
    }
    
    /// Add a filter
    pub fn filter<S: Into<String>>(mut self, filter: S) -> Self {
        self.query.filters.push(filter.into());
        self
    }
    
    /// Add multiple filters
    pub fn filters<I, S>(mut self, filters: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.query.filters.extend(filters.into_iter().map(|f| f.into()));
        self
    }
    
    /// Add a sort field
    pub fn sort<S: Into<String>>(mut self, sort: S) -> Self {
        self.query.sort.push(sort.into());
        self
    }
    
    /// Add multiple sort fields
    pub fn sorts<I, S>(mut self, sorts: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.query.sort.extend(sorts.into_iter().map(|s| s.into()));
        self
    }
    
    /// Add a facet field
    pub fn facet<S: Into<String>>(mut self, facet: S) -> Self {
        self.query.facets.push(facet.into());
        self
    }
    
    /// Set pagination
    pub fn page(mut self, page: u32, per_page: u32) -> Self {
        self.query.page = Some(page);
        self.query.per_page = Some(per_page);
        self
    }
    
    /// Set offset-based pagination
    pub fn offset(mut self, offset: u32, limit: u32) -> Self {
        self.query.offset = Some(offset);
        self.query.per_page = Some(limit);
        self
    }
    
    /// Set highlighting configuration
    pub fn highlight(mut self, config: HighlightConfig) -> Self {
        self.query.highlight = Some(config);
        self
    }
    
    /// Set search configuration
    pub fn config(mut self, config: WitSearchConfig) -> Self {
        self.query.config = Some(config);
        self
    }
    
    /// Build the final query
    pub fn build(self) -> SearchQuery {
        self.query
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Document builder utility for constructing documents
#[derive(Debug, Clone)]
pub struct DocumentBuilder {
    id: Option<String>,
    content: serde_json::Map<String, serde_json::Value>,
}

impl DocumentBuilder {
    /// Create a new document builder
    pub fn new() -> Self {
        Self {
            id: None,
            content: serde_json::Map::new(),
        }
    }
    
    /// Set the document ID
    pub fn id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = Some(id.into());
        self
    }
    
    /// Add a field to the document
    pub fn field<K, V>(mut self, key: K, value: V) -> Self 
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        self.content.insert(key.into(), value.into());
        self
    }
    
    /// Add multiple fields from a map
    pub fn fields<K, V, I>(mut self, fields: I) -> Self 
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
        I: IntoIterator<Item = (K, V)>,
    {
        for (k, v) in fields {
            self.content.insert(k.into(), v.into());
        }
        self
    }
    
    /// Build the final document
    pub fn build(self) -> crate::error::SearchResult<Doc> {
        let content = serde_json::to_string(&self.content)
            .map_err(crate::error::SearchError::from)?;
        
        Ok(Doc {
            id: self.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            content,
        })
    }
}

impl Default for DocumentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Schema builder utility for constructing schemas
#[derive(Debug, Clone)]
pub struct SchemaBuilder {
    fields: Vec<SchemaField>,
    primary_key: Option<String>,
}

impl SchemaBuilder {
    /// Create a new schema builder
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            primary_key: None,
        }
    }
    
    /// Set the primary key field
    pub fn primary_key<S: Into<String>>(mut self, key: S) -> Self {
        self.primary_key = Some(key.into());
        self
    }
    
    /// Add a field to the schema
    pub fn field(
        mut self,
        name: String,
        field_type: FieldType,
        required: bool,
        facet: bool,
        sort: bool,
        index: bool,
    ) -> Self {
        self.fields.push(SchemaField {
            name,
            r#type: field_type,
            required,
            facet,
            sort,
            index,
        });
        self
    }
    
    /// Add a text field
    pub fn text_field<S: Into<String>>(self, name: S) -> Self {
        self.field(name.into(), FieldType::Text, false, false, false, true)
    }
    
    /// Add a keyword field
    pub fn keyword_field<S: Into<String>>(self, name: S) -> Self {
        self.field(name.into(), FieldType::Keyword, false, true, true, true)
    }
    
    /// Add an integer field
    pub fn integer_field<S: Into<String>>(self, name: S) -> Self {
        self.field(name.into(), FieldType::Integer, false, true, true, true)
    }
    
    /// Add a float field
    pub fn float_field<S: Into<String>>(self, name: S) -> Self {
        self.field(name.into(), FieldType::Float, false, true, true, true)
    }
    
    /// Add a boolean field
    pub fn boolean_field<S: Into<String>>(self, name: S) -> Self {
        self.field(name.into(), FieldType::Boolean, false, true, false, true)
    }
    
    /// Add a date field
    pub fn date_field<S: Into<String>>(self, name: S) -> Self {
        self.field(name.into(), FieldType::Date, false, true, true, true)
    }
    
    /// Add a geo-point field
    pub fn geo_field<S: Into<String>>(self, name: S) -> Self {
        self.field(name.into(), FieldType::GeoPoint, false, false, false, true)
    }
    
    /// Build the final schema
    pub fn build(self) -> Schema {
        Schema {
            fields: self.fields,
            primary_key: self.primary_key,
        }
    }
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}