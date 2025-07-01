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

// TODO: WIT bindings will be generated here when the WIT file is properly configured
// wit_bindgen::generate!({
//     world: "search-provider",
//     path: "wit",
// });

// For now, we'll export the types that will be used by individual provider implementations
pub use types::{
    Doc, SearchQuery, SearchResults, Schema, SearchHit, FieldType, SchemaField,
    HighlightConfig, SearchConfig as SearchConfigType,
    QueryBuilder, DocumentBuilder, SchemaBuilder,
    IndexName, DocumentId, Json,
};

/// Placeholder component struct for future WIT implementation
pub struct Component;

// Future implementation will include the WIT Guest trait implementation
// This will be uncommented when WIT bindings are working:
/*
use exports::golem::search::core::Guest;

impl Guest for Component {
    type SearchHitStream = utils::SearchHitStream;
    
    // All the interface methods will be implemented here
    // For now, they would return Unsupported errors
}
*/