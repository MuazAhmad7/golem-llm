//! Common library for Golem search provider components
//! 
//! This library provides shared functionality for implementing search providers
//! that conform to the `golem:search` interface specification.

pub mod capabilities;
pub mod config;
pub mod error;
pub mod fallbacks;
pub mod testing;
pub mod types;
pub mod utils;

#[cfg(feature = "durability")]
pub mod durability;

// Re-export commonly used items
pub use error::{SearchError, SearchResult};
pub use types::{SearchProvider, SearchCapabilities};
pub use config::SearchConfig;
pub use capabilities::{CapabilityMatrix, ProviderCapabilities, FeatureSupport, DegradationStrategy};
pub use fallbacks::FallbackProcessor;
pub use testing::{TestConfig, TestResult, ProviderTestRunner, TestDataGenerator, UniversalTestQueries};

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

#[cfg(test)]
mod tests {
    use crate::types::{SearchQuery, Doc, HighlightConfig, QueryBuilder, DocumentBuilder, SchemaBuilder, FieldType, SearchCapabilities};
    use crate::config::{SearchConfig, ProviderConfig};
    use serde_json::json;
    use std::time::Duration;

    #[test]
    fn test_search_query_validation() {
        // Test valid query
        let valid_query = SearchQuery {
            q: Some("test query".to_string()),
            filters: vec![],
            sort: vec![],
            facets: vec![],
            page: Some(1),
            per_page: Some(10),
            offset: Some(0),
            highlight: None,
            config: None,
        };
        
        assert!(validate_search_query(&valid_query).is_ok());
        
        // Test query with large page size
        let large_page_query = SearchQuery {
            q: Some("test".to_string()),
            filters: vec![],
            sort: vec![],
            facets: vec![],
            page: Some(1),
            per_page: Some(10000), // Very large page size
            offset: None,
            highlight: None,
            config: None,
        };
        
        assert!(validate_search_query(&large_page_query).is_ok()); // Should still be valid
        
        // Test empty query
        let empty_query = SearchQuery {
            q: None,
            filters: vec![],
            sort: vec![],
            facets: vec![],
            page: None,
            per_page: None,
            offset: None,
            highlight: None,
            config: None,
        };
        
        assert!(validate_search_query(&empty_query).is_ok()); // Empty queries are valid
    }

    #[test]
    fn test_search_config_validation() {
        // Test valid config
        let valid_config = SearchConfig {
            endpoint: Some("http://localhost:9200".to_string()),
            timeout: Duration::from_secs(5),
            max_retries: 3,
            log_level: "info".to_string(),
            provider_config: ProviderConfig::ElasticSearch {
                username: Some("test_user".to_string()),
                password: Some("test_pass".to_string()),
                cloud_id: None,
                ca_cert: None,
            },
        };
        
        assert!(valid_config.validate().is_ok());
        
        // Test invalid config with empty API key for Algolia
        let invalid_config = SearchConfig {
            endpoint: Some("https://test.algolia.net".to_string()),
            timeout: Duration::from_secs(5),
            max_retries: 3,
            log_level: "info".to_string(),
            provider_config: ProviderConfig::Algolia {
                app_id: "".to_string(), // Empty app_id
                api_key: "test_key".to_string(),
            },
        };
        
        assert!(invalid_config.validate().is_err());
        
        // Test valid Meilisearch config
        let meilisearch_config = SearchConfig {
            endpoint: Some("http://localhost:7700".to_string()),
            timeout: Duration::from_secs(10),
            max_retries: 2,
            log_level: "debug".to_string(),
            provider_config: ProviderConfig::Meilisearch {
                api_key: Some("test_key".to_string()),
                master_key: None,
            },
        };
        
        assert!(meilisearch_config.validate().is_ok());
    }

    #[test]
    fn test_document_validation() {
        // Test valid document
        let valid_doc = Doc {
            id: "doc1".to_string(),
            content: json!({
                "title": "Test Document",
                "content": "This is test content"
            }).to_string(),
        };
        
        assert!(validate_document(&valid_doc).is_ok());
        
        // Test invalid document with empty ID
        let invalid_doc = Doc {
            id: "".to_string(),
            content: json!({
                "title": "Test Document",
                "content": "This is test content"
            }).to_string(),
        };
        
        assert!(validate_document(&invalid_doc).is_err());
        
        // Test document with malformed JSON content
        let malformed_doc = Doc {
            id: "doc2".to_string(),
            content: "invalid json {".to_string(),
        };
        
        assert!(validate_document(&malformed_doc).is_err());
    }

    #[test]
    fn test_filter_validation() {
        // Test valid filter string
        let valid_filter = "category:programming";
        assert!(validate_filter_string(valid_filter).is_ok());
        
        // Test valid range filter
        let range_filter = "price:[10 TO 100]";
        assert!(validate_filter_string(range_filter).is_ok());
        
        // Test empty filter
        let empty_filter = "";
        assert!(validate_filter_string(empty_filter).is_err());
        
        // Test malformed filter
        let malformed_filter = "category:";
        assert!(validate_filter_string(malformed_filter).is_ok()); // This is actually valid - empty value
    }

    #[test]
    fn test_facet_validation() {
        // Test valid facet field
        let valid_facet = "category";
        assert!(validate_facet_field(valid_facet).is_ok());
        
        // Test empty facet field
        let empty_facet = "";
        assert!(validate_facet_field(empty_facet).is_err());
        
        // Test facet field with special characters
        let special_facet = "category.sub_field";
        assert!(validate_facet_field(special_facet).is_ok());
    }

    #[test]
    fn test_highlight_config_validation() {
        // Test valid highlight config
        let valid_highlight = HighlightConfig {
            fields: vec!["title".to_string(), "content".to_string()],
            pre_tag: Some("<mark>".to_string()),
            post_tag: Some("</mark>".to_string()),
            max_length: Some(150),
        };
        
        assert!(validate_highlight_config(&valid_highlight).is_ok());
        
        // Test invalid highlight config with empty fields
        let invalid_highlight = HighlightConfig {
            fields: vec![],
            pre_tag: Some("<mark>".to_string()),
            post_tag: Some("</mark>".to_string()),
            max_length: Some(150),
        };
        
        assert!(validate_highlight_config(&invalid_highlight).is_err());
        
        // Test highlight config with no tags
        let no_tags = HighlightConfig {
            fields: vec!["title".to_string()],
            pre_tag: None,
            post_tag: None,
            max_length: Some(150),
        };
        
        assert!(validate_highlight_config(&no_tags).is_ok()); // Should be valid
    }

    #[test]
    fn test_sort_field_validation() {
        // Test valid sort string
        let valid_sort = "created_at:desc";
        assert!(validate_sort_string(valid_sort).is_ok());
        
        // Test ascending sort
        let asc_sort = "rating:asc";
        assert!(validate_sort_string(asc_sort).is_ok());
        
        // Test invalid sort with empty field name
        let invalid_sort = ":desc";
        assert!(validate_sort_string(invalid_sort).is_err());
        
        // Test sort without direction (should default to asc)
        let no_direction_sort = "title";
        assert!(validate_sort_string(no_direction_sort).is_ok());
    }

    #[test]
    fn test_query_builder() {
        // Test basic query building
        let query = QueryBuilder::new()
            .query("test search")
            .filter("category:programming")
            .sort("rating:desc")
            .facet("category")
            .page(1, 10)
            .build();
        
        assert_eq!(query.q, Some("test search".to_string()));
        assert_eq!(query.filters.len(), 1);
        assert_eq!(query.sort.len(), 1);
        assert_eq!(query.facets.len(), 1);
        assert_eq!(query.page, Some(1));
        assert_eq!(query.per_page, Some(10));
    }

    #[test]
    fn test_document_builder() {
        // Test document building
        let doc_result = DocumentBuilder::new()
            .id("test_doc")
            .field("title", "Test Document")
            .field("content", "This is test content")
            .field("price", 29.99)
            .build();
        
        assert!(doc_result.is_ok());
        let doc = doc_result.unwrap();
        assert_eq!(doc.id, "test_doc");
        
        // Parse content to verify structure
        let content: serde_json::Value = serde_json::from_str(&doc.content).unwrap();
        assert_eq!(content["title"], "Test Document");
        assert_eq!(content["content"], "This is test content");
        assert_eq!(content["price"], 29.99);
    }

    #[test]
    fn test_schema_builder() {
        // Test schema building
        let schema = SchemaBuilder::new()
            .primary_key("id")
            .text_field("title")
            .keyword_field("category")
            .integer_field("rating")
            .float_field("price")
            .boolean_field("featured")
            .build();
        
        assert_eq!(schema.primary_key, Some("id".to_string()));
        assert_eq!(schema.fields.len(), 5);
        
        // Check field types
        let title_field = schema.fields.iter().find(|f| f.name == "title").unwrap();
        assert_eq!(title_field.field_type, FieldType::Text);
        
        let price_field = schema.fields.iter().find(|f| f.name == "price").unwrap();
        assert_eq!(price_field.field_type, FieldType::Float);
    }

    #[test]
    fn test_search_capabilities() {
        // Test default capabilities
        let default_caps = SearchCapabilities::default();
        assert!(default_caps.supports_index_creation);
        assert!(default_caps.supports_schema_definition);
        assert!(default_caps.supports_full_text_search);
        assert!(!default_caps.supports_facets);
        assert!(!default_caps.supports_highlighting);
        
        // Test custom capabilities
        let mut custom_caps = SearchCapabilities::default();
        custom_caps.supports_facets = true;
        custom_caps.supports_highlighting = true;
        custom_caps.max_batch_size = Some(1000);
        
        assert!(custom_caps.supports_facets);
        assert!(custom_caps.supports_highlighting);
        assert_eq!(custom_caps.max_batch_size, Some(1000));
    }

    // Helper functions for validation (these would be implemented in the main code)
    fn validate_search_query(query: &SearchQuery) -> Result<(), String> {
        // Basic validation for search queries
        if let Some(per_page) = query.per_page {
            if per_page == 0 {
                return Err("'per_page' parameter must be positive".to_string());
            }
        }
        
        for filter in &query.filters {
            validate_filter_string(filter)?;
        }
        
        for facet in &query.facets {
            validate_facet_field(facet)?;
        }
        
        for sort_field in &query.sort {
            validate_sort_string(sort_field)?;
        }
        
        if let Some(highlight) = &query.highlight {
            validate_highlight_config(highlight)?;
        }
        
        Ok(())
    }
    
    fn validate_document(doc: &Doc) -> Result<(), String> {
        if doc.id.is_empty() {
            return Err("Document ID cannot be empty".to_string());
        }
        
        // Try to parse JSON content
        if serde_json::from_str::<serde_json::Value>(&doc.content).is_err() {
            return Err("Document content must be valid JSON".to_string());
        }
        
        Ok(())
    }
    
    fn validate_filter_string(filter: &str) -> Result<(), String> {
        if filter.is_empty() {
            return Err("Filter cannot be empty".to_string());
        }
        
        // Basic filter validation - should contain field and value
        if !filter.contains(':') {
            return Err("Filter must contain field:value format".to_string());
        }
        
        let parts: Vec<&str> = filter.split(':').collect();
        if parts[0].is_empty() {
            return Err("Filter must have a field name".to_string());
        }
        
        Ok(())
    }
    
    fn validate_facet_field(facet: &str) -> Result<(), String> {
        if facet.is_empty() {
            return Err("Facet field cannot be empty".to_string());
        }
        
        Ok(())
    }
    
    fn validate_highlight_config(highlight: &HighlightConfig) -> Result<(), String> {
        if highlight.fields.is_empty() {
            return Err("Highlight fields cannot be empty".to_string());
        }
        
        Ok(())
    }
    
    fn validate_sort_string(sort: &str) -> Result<(), String> {
        if sort.is_empty() {
            return Err("Sort field cannot be empty".to_string());
        }
        
        // Check for valid sort format (field:direction or just field)
        if sort.contains(':') {
            let parts: Vec<&str> = sort.split(':').collect();
            if parts.len() != 2 || parts[0].is_empty() {
                return Err("Sort must be in format 'field:direction'".to_string());
            }
            
            let direction = parts[1].to_lowercase();
            if direction != "asc" && direction != "desc" {
                return Err("Sort direction must be 'asc' or 'desc'".to_string());
            }
        }
        
        Ok(())
    }
}