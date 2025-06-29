// Generated WIT bindings for the Algolia search provider

wit_bindgen::generate!({
    path: "wit/algolia.wit",
    world: "search-provider",
    additional_derives: [serde::Deserialize, serde::Serialize],
});

// Re-export the generated bindings for easier access
pub use exports::golem::search_algolia::search::*;

// Type aliases for commonly used types
pub type SearchResult<T> = Result<T, Error>;
pub type SearchQuery = exports::golem::search_algolia::search::SearchQuery;
pub type SearchResults = exports::golem::search_algolia::search::SearchResults;
pub type SearchHit = exports::golem::search_algolia::search::SearchHit;
pub type Document = exports::golem::search_algolia::search::Document;
pub type Schema = exports::golem::search_algolia::search::Schema;
pub type FieldDefinition = exports::golem::search_algolia::search::FieldDefinition;
pub type Error = exports::golem::search_algolia::search::Error;
pub type ErrorCode = exports::golem::search_algolia::search::ErrorCode;
pub type FacetResult = exports::golem::search_algolia::search::FacetResult;
pub type FacetValue = exports::golem::search_algolia::search::FacetValue;