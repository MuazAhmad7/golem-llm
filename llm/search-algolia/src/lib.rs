use anyhow::Result;
use log::{debug, error, info, warn};
use std::sync::OnceLock;

mod client;
mod conversions;

use client::AlgoliaClient;
use conversions::*;

// Import the generated bindings
wit_bindgen::generate!({
    world: "search-library",
    path: "wit",
});

// Re-export the search interface
pub use golem::search::search::*;

/// Configuration for the Algolia search provider
#[derive(Debug, Clone)]
pub struct Config {
    pub app_id: String,
    pub api_key: String,
    pub base_url: String,
}

impl Config {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let app_id = std::env::var("ALGOLIA_APP_ID")
            .map_err(|_| anyhow::anyhow!("ALGOLIA_APP_ID environment variable not set"))?;
        let api_key = std::env::var("ALGOLIA_API_KEY")
            .map_err(|_| anyhow::anyhow!("ALGOLIA_API_KEY environment variable not set"))?;
        let base_url = std::env::var("ALGOLIA_BASE_URL")
            .unwrap_or_else(|_| "https://rest.algolia.net".to_string());

        Ok(Config {
            app_id,
            api_key,
            base_url,
        })
    }
}

/// Global configuration instance
static CONFIG: OnceLock<Config> = OnceLock::new();

/// Get the global configuration, initializing it if necessary
fn get_config() -> Result<&'static Config> {
    CONFIG.get_or_try_init(|| Config::from_env())
}

/// The main Algolia search provider implementation
pub struct AlgoliaSearchProvider {
    client: AlgoliaClient,
}

impl AlgoliaSearchProvider {
    /// Create a new Algolia search provider instance
    pub fn new() -> Result<Self> {
        let config = get_config()?;
        let client = AlgoliaClient::new(
            config.app_id.clone(),
            config.api_key.clone(),
            config.base_url.clone(),
        )?;

        info!("Initialized Algolia search provider");
        Ok(Self { client })
    }

    /// Get a reference to the internal client
    fn get_client(&self) -> &AlgoliaClient {
        &self.client
    }
}

impl Default for AlgoliaSearchProvider {
    fn default() -> Self {
        Self::new().expect("Failed to initialize Algolia search provider")
    }
}

// Implementation of the golem:search interface
impl Guest for AlgoliaSearchProvider {
    type Search = Self;
}

impl GuestSearch for AlgoliaSearchProvider {
    /// Create a new index with the given schema
    fn create_index(&self, name: String, schema: Schema) -> Result<(), Error> {
        info!("Creating index: {}", name);
        debug!("Schema: {:?}", schema);

        match self.get_client().create_index(&name, &schema) {
            Ok(_) => {
                info!("Successfully created index: {}", name);
                Ok(())
            }
            Err(e) => {
                error!("Failed to create index {}: {}", name, e);
                Err(map_error(e))
            }
        }
    }

    /// Delete an index
    fn delete_index(&self, name: String) -> Result<(), Error> {
        info!("Deleting index: {}", name);

        match self.get_client().delete_index(&name) {
            Ok(_) => {
                info!("Successfully deleted index: {}", name);
                Ok(())
            }
            Err(e) => {
                error!("Failed to delete index {}: {}", name, e);
                Err(map_error(e))
            }
        }
    }

    /// List all available indices
    fn list_indices(&self) -> Result<Vec<String>, Error> {
        info!("Listing indices");

        match self.get_client().list_indices() {
            Ok(indices) => {
                debug!("Found {} indices", indices.len());
                Ok(indices)
            }
            Err(e) => {
                error!("Failed to list indices: {}", e);
                Err(map_error(e))
            }
        }
    }

    /// Get the schema for an index
    fn get_schema(&self, index: String) -> Result<Schema, Error> {
        info!("Getting schema for index: {}", index);

        match self.get_client().get_schema(&index) {
            Ok(schema) => {
                debug!("Retrieved schema for index: {}", index);
                Ok(schema)
            }
            Err(e) => {
                error!("Failed to get schema for index {}: {}", index, e);
                Err(map_error(e))
            }
        }
    }

    /// Upsert a single document
    fn upsert_document(&self, index: String, document: Document) -> Result<String, Error> {
        info!("Upserting document to index: {}", index);
        debug!("Document: {:?}", document);

        match self.get_client().upsert_document(&index, &document) {
            Ok(doc_id) => {
                info!("Successfully upserted document with ID: {}", doc_id);
                Ok(doc_id)
            }
            Err(e) => {
                error!("Failed to upsert document to index {}: {}", index, e);
                Err(map_error(e))
            }
        }
    }

    /// Batch upsert multiple documents
    fn batch_upsert_documents(
        &self,
        index: String,
        documents: Vec<Document>,
    ) -> Result<Vec<String>, Error> {
        info!("Batch upserting {} documents to index: {}", documents.len(), index);

        match self.get_client().batch_upsert_documents(&index, &documents) {
            Ok(doc_ids) => {
                info!("Successfully upserted {} documents", doc_ids.len());
                Ok(doc_ids)
            }
            Err(e) => {
                error!("Failed to batch upsert documents to index {}: {}", index, e);
                Err(map_error(e))
            }
        }
    }

    /// Get a document by ID
    fn get_document(&self, index: String, id: String) -> Result<Document, Error> {
        info!("Getting document {} from index: {}", id, index);

        match self.get_client().get_document(&index, &id) {
            Ok(document) => {
                debug!("Retrieved document: {}", id);
                Ok(document)
            }
            Err(e) => {
                error!("Failed to get document {} from index {}: {}", id, index, e);
                Err(map_error(e))
            }
        }
    }

    /// Delete a document by ID
    fn delete_document(&self, index: String, id: String) -> Result<(), Error> {
        info!("Deleting document {} from index: {}", id, index);

        match self.get_client().delete_document(&index, &id) {
            Ok(_) => {
                info!("Successfully deleted document: {}", id);
                Ok(())
            }
            Err(e) => {
                error!("Failed to delete document {} from index {}: {}", id, index, e);
                Err(map_error(e))
            }
        }
    }

    /// Batch delete multiple documents
    fn batch_delete_documents(&self, index: String, ids: Vec<String>) -> Result<(), Error> {
        info!("Batch deleting {} documents from index: {}", ids.len(), index);

        match self.get_client().batch_delete_documents(&index, &ids) {
            Ok(_) => {
                info!("Successfully deleted {} documents", ids.len());
                Ok(())
            }
            Err(e) => {
                error!("Failed to batch delete documents from index {}: {}", index, e);
                Err(map_error(e))
            }
        }
    }

    /// Search documents in an index
    fn search(&self, index: String, query: SearchQuery) -> Result<SearchResults, Error> {
        info!("Searching index: {} with query: {}", index, query.q);
        debug!("Full query: {:?}", query);

        match self.get_client().search(&index, &query) {
            Ok(results) => {
                info!("Search returned {} hits", results.hits.len());
                Ok(results)
            }
            Err(e) => {
                error!("Failed to search index {}: {}", index, e);
                Err(map_error(e))
            }
        }
    }
}

/// Component export
export!(AlgoliaSearchProvider);