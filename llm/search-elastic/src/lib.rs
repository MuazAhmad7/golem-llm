//! ElasticSearch provider implementation for the golem:search interface

use anyhow::Result;
use log::{debug, error, info};

mod client;
mod conversions;

use client::{ElasticClient, ElasticConfig};
use conversions::*;
use golem_search::{
    SearchError, SearchResult, Doc, SearchQuery, SearchResults, Schema,
    SearchCapabilities, FieldType,
};

// TODO: Enable WIT bindings when the WIT file structure is fixed
// wit_bindgen::generate!({
//     world: "elasticsearch-provider",
//     path: "wit",
// });

/// The ElasticSearch search provider implementation
pub struct ElasticSearchProvider {
    client: ElasticClient,
}

impl ElasticSearchProvider {
    /// Create a new ElasticSearch provider
    pub async fn new() -> SearchResult<Self> {
        let config = ElasticConfig::from_env()
            .map_err(|e| {
                error!("Failed to load ElasticSearch configuration: {}", e);
                SearchError::Internal(format!("Configuration error: {}", e))
            })?;

        let client = ElasticClient::new(config)
            .map_err(|e| {
                error!("Failed to create ElasticSearch client: {}", e);
                SearchError::Internal(format!("Client initialization error: {}", e))
            })?;

        info!("ElasticSearch search provider initialized successfully");
        Ok(Self { client })
    }

    /// Get ElasticSearch-specific capabilities
    pub fn get_capabilities(&self) -> SearchCapabilities {
        SearchCapabilities {
            supports_index_creation: true,
            supports_schema_definition: true,
            supports_facets: true,
            supports_highlighting: true,
            supports_full_text_search: true,
            supports_vector_search: false, // ElasticSearch supports vectors but requires plugins
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
            provider_features: std::collections::HashMap::new(),
        }
    }

    /// Create an index
    pub async fn create_index(&self, name: &str, schema: Option<&Schema>) -> SearchResult<()> {
        info!("Creating ElasticSearch index: {}", name);

        let settings = if let Some(schema) = schema {
            let mapping = schema_to_elastic_mapping(schema)
                .map_err(|e| SearchError::Internal(e.to_string()))?;
            Some(mapping)
        } else {
            None
        };

        self.client
            .create_index(name, settings)
            .await
            .map_err(|e| {
                error!("Failed to create index {}: {}", name, e);
                map_elastic_error(e)
            })?;

        info!("Successfully created ElasticSearch index: {}", name);
        Ok(())
    }

    /// Delete an index
    pub async fn delete_index(&self, name: &str) -> SearchResult<()> {
        info!("Deleting ElasticSearch index: {}", name);

        self.client
            .delete_index(name)
            .await
            .map_err(|e| {
                error!("Failed to delete index {}: {}", name, e);
                map_elastic_error(e)
            })?;

        info!("Successfully deleted ElasticSearch index: {}", name);
        Ok(())
    }

    /// List all indexes
    pub async fn list_indexes(&self) -> SearchResult<Vec<String>> {
        debug!("Listing ElasticSearch indexes");

        let indexes = self.client
            .list_indexes()
            .await
            .map_err(|e| {
                error!("Failed to list indexes: {}", e);
                map_elastic_error(e)
            })?;

        debug!("Found {} ElasticSearch indexes", indexes.len());
        Ok(indexes)
    }

    /// Upsert a document
    pub async fn upsert(&self, index: &str, doc: &Doc) -> SearchResult<()> {
        debug!("Upserting document {} in index {}", doc.id, index);

        let (doc_id, content) = doc_to_elastic_document(doc)
            .map_err(|e| SearchError::InvalidQuery(e.to_string()))?;

        self.client
            .index_document(index, &doc_id, content)
            .await
            .map_err(|e| {
                error!("Failed to upsert document {}: {}", doc_id, e);
                map_elastic_error(e)
            })?;

        debug!("Successfully upserted document {}", doc_id);
        Ok(())
    }

    /// Upsert multiple documents
    pub async fn upsert_many(&self, index: &str, docs: &[Doc]) -> SearchResult<()> {
        info!("Bulk upserting {} documents in index {}", docs.len(), index);

        let operations = docs_to_bulk_operations(index, docs, "index")
            .map_err(|e| SearchError::InvalidQuery(e.to_string()))?;

        self.client
            .bulk(operations)
            .await
            .map_err(|e| {
                error!("Failed to bulk upsert documents: {}", e);
                map_elastic_error(e)
            })?;

        info!("Successfully bulk upserted {} documents", docs.len());
        Ok(())
    }

    /// Delete a document
    pub async fn delete(&self, index: &str, id: &str) -> SearchResult<()> {
        debug!("Deleting document {} from index {}", id, index);

        self.client
            .delete_document(index, id)
            .await
            .map_err(|e| {
                error!("Failed to delete document {}: {}", id, e);
                map_elastic_error(e)
            })?;

        debug!("Successfully deleted document {}", id);
        Ok(())
    }

    /// Delete multiple documents
    pub async fn delete_many(&self, index: &str, ids: &[String]) -> SearchResult<()> {
        info!("Bulk deleting {} documents from index {}", ids.len(), index);

        let docs: Vec<Doc> = ids.iter().map(|id| Doc {
            id: id.clone(),
            content: "{}".to_string(), // Empty content for delete operations
        }).collect();

        let operations = docs_to_bulk_operations(index, &docs, "delete")
            .map_err(|e| SearchError::InvalidQuery(e.to_string()))?;

        self.client
            .bulk(operations)
            .await
            .map_err(|e| {
                error!("Failed to bulk delete documents: {}", e);
                map_elastic_error(e)
            })?;

        info!("Successfully bulk deleted {} documents", docs.len());
        Ok(())
    }

    /// Get a document by ID
    pub async fn get(&self, index: &str, id: &str) -> SearchResult<Option<Doc>> {
        debug!("Getting document {} from index {}", id, index);

        let result = self.client
            .get_document(index, id)
            .await
            .map_err(|e| {
                error!("Failed to get document {}: {}", id, e);
                map_elastic_error(e)
            })?;

        if let Some(response) = result {
            let doc = elastic_document_to_doc(&response)
                .map_err(|e| SearchError::Internal(e.to_string()))?;
            
            debug!("Successfully retrieved document {}", id);
            Ok(Some(doc))
        } else {
            debug!("Document {} not found", id);
            Ok(None)
        }
    }

    /// Search documents
    pub async fn search(&self, index: &str, query: &SearchQuery) -> SearchResult<SearchResults> {
        debug!("Searching index {} with query: {:?}", index, query.q);

        let elastic_query = search_query_to_elastic_query(query)
            .map_err(|e| SearchError::InvalidQuery(e.to_string()))?;

        let response = self.client
            .search(index, elastic_query)
            .await
            .map_err(|e| {
                error!("Search failed for index {}: {}", index, e);
                map_elastic_error(e)
            })?;

        let results = elastic_response_to_search_results(&response)
            .map_err(|e| SearchError::Internal(e.to_string()))?;

        debug!("Search completed. Found {} hits", results.hits.len());
        Ok(results)
    }

    /// Get schema for an index
    pub async fn get_schema(&self, index: &str) -> SearchResult<Schema> {
        debug!("Getting schema for index {}", index);

        let mapping = self.client
            .get_mapping(index)
            .await
            .map_err(|e| {
                error!("Failed to get mapping for index {}: {}", index, e);
                map_elastic_error(e)
            })?;

        let schema = elastic_mapping_to_schema(&mapping, index)
            .map_err(|e| SearchError::Internal(e.to_string()))?;

        debug!("Successfully retrieved schema for index {}", index);
        Ok(schema)
    }

    /// Update schema for an index
    pub async fn update_schema(&self, index: &str, schema: &Schema) -> SearchResult<()> {
        info!("Updating schema for index {}", index);

        let mapping = schema_to_elastic_mapping(schema)
            .map_err(|e| SearchError::Internal(e.to_string()))?;

        self.client
            .put_mapping(index, mapping)
            .await
            .map_err(|e| {
                error!("Failed to update mapping for index {}: {}", index, e);
                map_elastic_error(e)
            })?;

        info!("Successfully updated schema for index {}", index);
        Ok(())
    }
}