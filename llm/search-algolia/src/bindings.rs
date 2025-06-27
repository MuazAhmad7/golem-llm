// Generated WIT bindings for the Algolia search provider

wit_bindgen::generate!({
    path: "wit/algolia.wit",
    world: "search-provider",
    additional_derives: [serde::Deserialize, serde::Serialize],
});

// Re-export the generated bindings for easier access
pub use exports::golem::search::search::*;

// Type aliases for commonly used types
pub type SearchResult<T> = Result<T, Error>;
pub type SearchQuery = exports::golem::search::search::SearchQuery;
pub type SearchResults = exports::golem::search::search::SearchResults;
pub type SearchHit = exports::golem::search::search::SearchHit;
pub type Document = exports::golem::search::search::Document;
pub type Schema = exports::golem::search::search::Schema;
pub type FieldDefinition = exports::golem::search::search::FieldDefinition;
pub type Error = exports::golem::search::search::Error;
pub type ErrorCode = exports::golem::search::search::ErrorCode;