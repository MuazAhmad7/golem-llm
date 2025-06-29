use anyhow::Result;
use log::{error, info, warn};

mod bindings;
pub mod client;
mod conversions;

use bindings::*;
use client::{AlgoliaClient, AlgoliaConfig};
use conversions::*;

/// The main Algolia search provider implementation
pub struct AlgoliaSearchProvider {
    client: AlgoliaClient,
}

impl AlgoliaSearchProvider {
    /// Create a new Algolia search provider
    pub fn new() -> Result<Self, Error> {
        let config = AlgoliaConfig::from_env()
            .map_err(|e| {
                error!("Failed to load Algolia configuration: {}", e);
                Error {
                    code: ErrorCode::AuthenticationFailed,
                    message: format!("Configuration error: {}", e),
                    retry_after: None,
                }
            })?;

        let client = AlgoliaClient::new(config)
            .map_err(|e| {
                error!("Failed to create Algolia client: {}", e);
                Error {
                    code: ErrorCode::InternalError,
                    message: format!("Client initialization error: {}", e),
                    retry_after: None,
                }
            })?;

        info!("Algolia search provider initialized successfully");
        Ok(Self { client })
    }

    /// Get the client for internal use
    fn get_client(&self) -> &AlgoliaClient {
        &self.client
    }
}

/// Implementation of the golem:search interface
impl exports::golem::search_algolia::search::Guest for AlgoliaSearchProvider {
    // Index Management

    fn create_index(name: String, schema: Schema) -> Result<(), Error> {
        let provider = Self::new()?;
        
        info!("Creating index: {}", name);
        
        // Convert schema to Algolia settings
        let settings = schema_to_index_settings(&schema);
        
        // Create the index
        if let Err(e) = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(provider.client.create_index(&name))
        }) {
            error!("Failed to create index {}: {}", name, e);
            return Err(map_algolia_error(e));
        }
        
        // Apply the settings
        if let Err(e) = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(provider.client.update_index_settings(&name, &settings))
        }) {
            warn!("Index created but failed to apply settings: {}", e);
            // Don't fail entirely if settings can't be applied
        }
        
        info!("Successfully created index: {}", name);
        Ok(())
    }

    fn delete_index(name: String) -> Result<(), Error> {
        let provider = Self::new()?;
        
        info!("Deleting index: {}", name);
        
        if let Err(e) = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(provider.client.delete_index(&name))
        }) {
            error!("Failed to delete index {}: {}", name, e);
            return Err(map_algolia_error(e));
        }
        
        info!("Successfully deleted index: {}", name);
        Ok(())
    }

    fn list_indices() -> Result<Vec<String>, Error> {
        let provider = Self::new()?;
        
        info!("Listing indices");
        
        match tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(provider.client.list_indices())
        }) {
            Ok(indices) => {
                info!("Found {} indices", indices.len());
                Ok(indices)
            }
            Err(e) => {
                error!("Failed to list indices: {}", e);
                Err(map_algolia_error(e))
            }
        }
    }

    // Document Operations

    fn upsert_documents(index: String, documents: Vec<Document>) -> Result<u32, Error> {
        let provider = Self::new()?;
        
        info!("Upserting {} documents in index {}", documents.len(), index);
        
        // Convert all documents to Algolia objects
        let mut algolia_objects = Vec::new();
        let mut object_ids = Vec::new();
        
        for document in documents {
            let (object_id, algolia_object) = document_to_algolia_object(&document)
                .map_err(map_algolia_error)?;
            object_ids.push(object_id);
            algolia_objects.push(algolia_object);
        }
        
        // Batch upsert
        match tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(provider.client.batch_objects(&index, &algolia_objects))
        }) {
            Ok(_) => {
                info!("Successfully upserted {} documents in index {}", object_ids.len(), index);
                Ok(object_ids.len() as u32)
            }
            Err(e) => {
                error!("Failed to batch upsert documents in index {}: {}", index, e);
                Err(map_algolia_error(e))
            }
        }
    }

    fn get_document(index: String, id: String) -> Result<Document, Error> {
        let provider = Self::new()?;
        
        info!("Getting document {} from index {}", id, index);
        
        match tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(provider.client.get_object(&index, &id))
        }) {
            Ok(algolia_object) => {
                let document = algolia_object_to_document(id.clone(), algolia_object)
                    .map_err(map_algolia_error)?;
                info!("Successfully retrieved document {} from index {}", id, index);
                Ok(document)
            }
            Err(e) => {
                error!("Failed to get document {} from index {}: {}", id, index, e);
                Err(map_algolia_error(e))
            }
        }
    }

    fn delete_documents(index: String, ids: Vec<String>) -> Result<u32, Error> {
        let provider = Self::new()?;
        
        info!("Deleting {} documents from index {}", ids.len(), index);
        
        if let Err(e) = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(provider.client.delete_objects(&index, &ids))
        }) {
            error!("Failed to delete {} documents from index {}: {}", ids.len(), index, e);
            return Err(map_algolia_error(e));
        }
        
        info!("Successfully deleted {} documents from index {}", ids.len(), index);
        Ok(ids.len() as u32)
    }

    // Search Operations

    fn search(index: String, query: SearchQuery) -> Result<SearchResults, Error> {
        let provider = Self::new()?;
        
        info!("Searching index {} with query: '{}'", index, query.query);
        
        let algolia_query = search_query_to_algolia_query(&query)
            .map_err(map_algolia_error)?;
        
        match tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(provider.client.search(&index, &algolia_query))
        }) {
            Ok(algolia_results) => {
                let search_results = algolia_results_to_search_results(algolia_results)
                    .map_err(map_algolia_error)?;
                
                info!("Search completed. Found {} hits in {} ms", 
                    search_results.total_hits, 
                    search_results.processing_time_ms.unwrap_or(0)
                );
                
                Ok(search_results)
            }
            Err(e) => {
                error!("Search failed for index {} with query '{}': {}", index, query.query, e);
                Err(map_algolia_error(e))
            }
        }
    }
}

// Export the component implementation
bindings::export!(AlgoliaSearchProvider with_types_in bindings);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        // This test will fail without proper environment variables
        // but it validates the code structure
        std::env::set_var("ALGOLIA_APP_ID", "test");
        std::env::set_var("ALGOLIA_API_KEY", "test");
        
        let result = AlgoliaSearchProvider::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_schema_conversion() {
        use bindings::{FieldDefinition, FieldType};
        
        let schema = Schema {
            primary_key: "id".to_string(),
            fields: vec![
                FieldDefinition {
                    name: "title".to_string(),
                    field_type: FieldType::Text,
                    searchable: true,
                    facetable: false,
                    retrievable: true,
                    sortable: true,
                }
            ],
            provider_params: None,
        };
        
        let settings = schema_to_index_settings(&schema);
        assert!(settings.searchable_attributes.is_some());
    }
}