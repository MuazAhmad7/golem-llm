//! Common library for Golem search provider components
//! 
//! This library provides shared functionality for implementing search providers
//! that conform to the `golem:search` interface specification.

pub mod config;
pub mod error;
pub mod types;
pub mod utils;

#[cfg(feature = "durability")]
pub mod durability;

// Re-export commonly used items
pub use error::{SearchError, SearchResult};
pub use types::{SearchProvider, SearchCapabilities};
pub use config::SearchConfig;

// Generate WIT bindings
wit_bindgen::generate!({
    world: "search-provider",
    path: "wit",
    exports: {
        "golem:search/core": Component,
    },
});

use exports::golem::search::core::Guest;

/// The main component struct that implements the search interface
pub struct Component;

impl Guest for Component {
    type SearchHitStream = utils::SearchHitStream;
    
    fn create_index(
        name: types::IndexName, 
        schema: Option<types::Schema>
    ) -> Result<(), types::SearchError> {
        // This will be implemented by individual providers
        Err(types::SearchError::Unsupported)
    }
    
    fn delete_index(name: types::IndexName) -> Result<(), types::SearchError> {
        // This will be implemented by individual providers
        Err(types::SearchError::Unsupported)
    }
    
    fn list_indexes() -> Result<Vec<types::IndexName>, types::SearchError> {
        // This will be implemented by individual providers
        Err(types::SearchError::Unsupported)
    }
    
    fn upsert(
        index: types::IndexName, 
        doc: types::Doc
    ) -> Result<(), types::SearchError> {
        // This will be implemented by individual providers
        Err(types::SearchError::Unsupported)
    }
    
    fn upsert_many(
        index: types::IndexName, 
        docs: Vec<types::Doc>
    ) -> Result<(), types::SearchError> {
        // This will be implemented by individual providers
        Err(types::SearchError::Unsupported)
    }
    
    fn delete(
        index: types::IndexName, 
        id: types::DocumentId
    ) -> Result<(), types::SearchError> {
        // This will be implemented by individual providers
        Err(types::SearchError::Unsupported)
    }
    
    fn delete_many(
        index: types::IndexName, 
        ids: Vec<types::DocumentId>
    ) -> Result<(), types::SearchError> {
        // This will be implemented by individual providers
        Err(types::SearchError::Unsupported)
    }
    
    fn get(
        index: types::IndexName, 
        id: types::DocumentId
    ) -> Result<Option<types::Doc>, types::SearchError> {
        // This will be implemented by individual providers
        Err(types::SearchError::Unsupported)
    }
    
    fn search(
        index: types::IndexName, 
        query: types::SearchQuery
    ) -> Result<types::SearchResults, types::SearchError> {
        // This will be implemented by individual providers
        Err(types::SearchError::Unsupported)
    }
    
    fn stream_search(
        index: types::IndexName, 
        query: types::SearchQuery
    ) -> Result<Self::SearchHitStream, types::SearchError> {
        // This will be implemented by individual providers
        Err(types::SearchError::Unsupported)
    }
    
    fn get_schema(index: types::IndexName) -> Result<types::Schema, types::SearchError> {
        // This will be implemented by individual providers
        Err(types::SearchError::Unsupported)
    }
    
    fn update_schema(
        index: types::IndexName, 
        schema: types::Schema
    ) -> Result<(), types::SearchError> {
        // This will be implemented by individual providers
        Err(types::SearchError::Unsupported)
    }
}