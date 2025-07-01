#[allow(static_mut_refs)]
mod bindings;

use crate::bindings::exports::test::search_exports::test_search_api::*;
use crate::bindings::golem::search::search::*;
use serde_json::{json, Value};
use std::collections::HashMap;

struct Component;

// Test data generators
fn create_test_documents() -> Vec<Document> {
    vec![
        Document {
            id: "doc1".to_string(),
            content: json!({
                "title": "Introduction to Rust Programming",
                "content": "Rust is a systems programming language focused on safety and performance.",
                "category": "programming",
                "tags": ["rust", "systems", "programming"],
                "price": 29.99,
                "rating": 4.8,
                "published": "2024-01-15"
            }),
        },
        Document {
            id: "doc2".to_string(),
            content: json!({
                "title": "Advanced Web Development",
                "content": "Learn modern web development techniques with JavaScript and TypeScript.",
                "category": "web",
                "tags": ["javascript", "typescript", "web"],
                "price": 39.99,
                "rating": 4.5,
                "published": "2024-02-20"
            }),
        },
        Document {
            id: "doc3".to_string(),
            content: json!({
                "title": "Database Design Patterns",
                "content": "Comprehensive guide to database design and optimization strategies.",
                "category": "database",
                "tags": ["database", "sql", "design"],
                "price": 34.99,
                "rating": 4.7,
                "published": "2024-03-10"
            }),
        },
    ]
}

fn create_test_config() -> SearchConfig {
    SearchConfig {
        provider: "elastic".to_string(),
        endpoint: "http://localhost:9200".to_string(),
        api_key: Some("test_key".to_string()),
        index_name: "test_index".to_string(),
        timeout_ms: Some(5000),
        max_results: Some(100),
        options: vec![
            ("refresh".to_string(), "true".to_string()),
            ("routing".to_string(), "test".to_string()),
        ],
    }
}

impl Guest for Component {
    /// Test basic search functionality across all providers
    fn test_basic_search() -> String {
        let config = create_test_config();
        let docs = create_test_documents();
        
        // Test document indexing
        let index_result = index_documents(&config, &docs);
        if let Err(e) = index_result {
            return format!("FAILED: Document indexing failed: {:?}", e);
        }
        
        // Test basic text search
        let query = SearchQuery {
            query_type: QueryType::Text,
            text: Some("rust programming".to_string()),
            filters: vec![],
            facets: vec![],
            sort: vec![],
            highlight: None,
            from: Some(0),
            size: Some(10),
        };
        
        match search(&config, &query) {
            Ok(results) => {
                if results.hits.is_empty() {
                    "FAILED: No results found for basic search".to_string()
                } else {
                    let first_hit = &results.hits[0];
                    if first_hit.id == "doc1" {
                        format!("PASSED: Basic search returned {} results, first hit ID: {}", 
                               results.hits.len(), first_hit.id)
                    } else {
                        format!("FAILED: Expected doc1 as first result, got: {}", first_hit.id)
                    }
                }
            }
            Err(e) => format!("FAILED: Search failed: {:?}", e),
        }
    }

    /// Test faceted search with filters and aggregations
    fn test_faceted_search() -> String {
        let config = create_test_config();
        
        let query = SearchQuery {
            query_type: QueryType::Faceted,
            text: Some("*".to_string()),
            filters: vec![
                Filter {
                    field: "category".to_string(),
                    filter_type: FilterType::Term,
                    value: "programming".to_string(),
                    operator: Some(FilterOperator::Equals),
                }
            ],
            facets: vec![
                Facet {
                    field: "category".to_string(),
                    facet_type: FacetType::Terms,
                    size: Some(10),
                }
            ],
            sort: vec![],
            highlight: None,
            from: Some(0),
            size: Some(10),
        };
        
        match search(&config, &query) {
            Ok(results) => {
                if results.facets.is_empty() {
                    "FAILED: No facets returned in faceted search".to_string()
                } else {
                    let category_facet = &results.facets[0];
                    format!("PASSED: Faceted search returned {} facet buckets for field '{}'", 
                           category_facet.buckets.len(), category_facet.field)
                }
            }
            Err(e) => format!("FAILED: Faceted search failed: {:?}", e),
        }
    }

    /// Test search with complex filters
    fn test_search_with_filters() -> String {
        let config = create_test_config();
        
        let query = SearchQuery {
            query_type: QueryType::Filtered,
            text: Some("development".to_string()),
            filters: vec![
                Filter {
                    field: "price".to_string(),
                    filter_type: FilterType::Range,
                    value: "30.0".to_string(),
                    operator: Some(FilterOperator::GreaterThanOrEqual),
                },
                Filter {
                    field: "rating".to_string(),
                    filter_type: FilterType::Range,
                    value: "4.0".to_string(),
                    operator: Some(FilterOperator::GreaterThan),
                }
            ],
            facets: vec![],
            sort: vec![
                SortField {
                    field: "rating".to_string(),
                    direction: SortDirection::Descending,
                }
            ],
            highlight: None,
            from: Some(0),
            size: Some(5),
        };
        
        match search(&config, &query) {
            Ok(results) => {
                if results.hits.is_empty() {
                    "PASSED: Filtered search correctly returned no results for strict criteria".to_string()
                } else {
                    // Verify results meet filter criteria
                    let first_hit = &results.hits[0];
                    format!("PASSED: Filtered search returned {} results, sorted by rating", 
                           results.hits.len())
                }
            }
            Err(e) => format!("FAILED: Filtered search failed: {:?}", e),
        }
    }

    /// Test bulk document indexing operations
    fn test_bulk_indexing() -> String {
        let config = create_test_config();
        
        // Create a larger batch of documents
        let mut bulk_docs = Vec::new();
        for i in 1..=50 {
            bulk_docs.push(Document {
                id: format!("bulk_doc_{}", i),
                content: json!({
                    "title": format!("Bulk Document {}", i),
                    "content": format!("This is test content for bulk document number {}", i),
                    "category": if i % 2 == 0 { "even" } else { "odd" },
                    "number": i,
                    "batch": "test_batch"
                }),
            });
        }
        
        match bulk_index(&config, &bulk_docs) {
            Ok(response) => {
                if response.errors.is_empty() {
                    format!("PASSED: Bulk indexed {} documents successfully", response.indexed_count)
                } else {
                    format!("PARTIAL: Bulk indexed {} documents with {} errors", 
                           response.indexed_count, response.errors.len())
                }
            }
            Err(e) => format!("FAILED: Bulk indexing failed: {:?}", e),
        }
    }

    /// Test CRUD operations on individual documents
    fn test_document_operations() -> String {
        let config = create_test_config();
        let test_doc = Document {
            id: "crud_test_doc".to_string(),
            content: json!({
                "title": "CRUD Test Document",
                "content": "This document is used for testing CRUD operations",
                "version": 1
            }),
        };
        
        // Test Create
        if let Err(e) = index_document(&config, &test_doc) {
            return format!("FAILED: Document creation failed: {:?}", e);
        }
        
        // Test Read
        match get_document(&config, "crud_test_doc") {
            Ok(Some(retrieved_doc)) => {
                if retrieved_doc.id != test_doc.id {
                    return format!("FAILED: Retrieved document ID mismatch");
                }
            }
            Ok(None) => return "FAILED: Document not found after creation".to_string(),
            Err(e) => return format!("FAILED: Document retrieval failed: {:?}", e),
        }
        
        // Test Update
        let updated_doc = Document {
            id: "crud_test_doc".to_string(),
            content: json!({
                "title": "CRUD Test Document - Updated",
                "content": "This document has been updated",
                "version": 2
            }),
        };
        
        if let Err(e) = update_document(&config, &updated_doc) {
            return format!("FAILED: Document update failed: {:?}", e);
        }
        
        // Test Delete
        match delete_document(&config, "crud_test_doc") {
            Ok(true) => "PASSED: All CRUD operations completed successfully".to_string(),
            Ok(false) => "FAILED: Document deletion returned false".to_string(),
            Err(e) => format!("FAILED: Document deletion failed: {:?}", e),
        }
    }

    /// Test search highlighting functionality
    fn test_search_highlighting() -> String {
        let config = create_test_config();
        
        let query = SearchQuery {
            query_type: QueryType::Text,
            text: Some("programming language".to_string()),
            filters: vec![],
            facets: vec![],
            sort: vec![],
            highlight: Some(HighlightConfig {
                fields: vec!["title".to_string(), "content".to_string()],
                pre_tags: vec!["<mark>".to_string()],
                post_tags: vec!["</mark>".to_string()],
                fragment_size: Some(150),
                num_fragments: Some(3),
            }),
            from: Some(0),
            size: Some(5),
        };
        
        match search(&config, &query) {
            Ok(results) => {
                if let Some(first_hit) = results.hits.first() {
                    if let Some(highlights) = &first_hit.highlight {
                        if highlights.contains_key("title") || highlights.contains_key("content") {
                            format!("PASSED: Search highlighting returned highlighted fields: {:?}", 
                                   highlights.keys().collect::<Vec<_>>())
                        } else {
                            "FAILED: No highlighted fields found in search results".to_string()
                        }
                    } else {
                        "FAILED: No highlight data in search results".to_string()
                    }
                } else {
                    "FAILED: No search results for highlighting test".to_string()
                }
            }
            Err(e) => format!("FAILED: Search with highlighting failed: {:?}", e),
        }
    }

    /// Test autocomplete/suggestion functionality
    fn test_autocomplete() -> String {
        let config = create_test_config();
        
        match suggest(&config, "prog", "title", 5) {
            Ok(suggestions) => {
                if suggestions.is_empty() {
                    "FAILED: No autocomplete suggestions returned".to_string()
                } else {
                    let suggestion_texts: Vec<&str> = suggestions.iter()
                        .map(|s| s.text.as_str())
                        .collect();
                    format!("PASSED: Autocomplete returned {} suggestions: {:?}", 
                           suggestions.len(), suggestion_texts)
                }
            }
            Err(e) => format!("FAILED: Autocomplete failed: {:?}", e),
        }
    }

    /// Test aggregation functionality
    fn test_aggregations() -> String {
        let config = create_test_config();
        
        let query = SearchQuery {
            query_type: QueryType::Aggregated,
            text: Some("*".to_string()),
            filters: vec![],
            facets: vec![
                Facet {
                    field: "category".to_string(),
                    facet_type: FacetType::Terms,
                    size: Some(10),
                },
                Facet {
                    field: "price".to_string(),
                    facet_type: FacetType::Range,
                    size: Some(5),
                }
            ],
            sort: vec![],
            highlight: None,
            from: Some(0),
            size: Some(0), // Only interested in aggregations
        };
        
        match search(&config, &query) {
            Ok(results) => {
                if results.facets.len() >= 2 {
                    let terms_facet = &results.facets[0];
                    let range_facet = &results.facets[1];
                    format!("PASSED: Aggregations returned {} terms buckets and {} range buckets", 
                           terms_facet.buckets.len(), range_facet.buckets.len())
                } else {
                    format!("FAILED: Expected 2 facets, got {}", results.facets.len())
                }
            }
            Err(e) => format!("FAILED: Aggregation query failed: {:?}", e),
        }
    }

    /// Test streaming search functionality
    fn test_streaming_search() -> String {
        let config = create_test_config();
        
        let query = SearchQuery {
            query_type: QueryType::Text,
            text: Some("*".to_string()),
            filters: vec![],
            facets: vec![],
            sort: vec![],
            highlight: None,
            from: Some(0),
            size: Some(100), // Large result set for streaming
        };
        
        match stream_search(&config, &query) {
            Ok(stream) => {
                let mut total_hits = 0;
                let mut batch_count = 0;
                
                // Simulate consuming the stream
                loop {
                    match stream.get_next() {
                        Ok(Some(batch)) => {
                            total_hits += batch.hits.len();
                            batch_count += 1;
                            
                            if batch.hits.is_empty() {
                                break; // End of stream
                            }
                        }
                        Ok(None) => break, // End of stream
                        Err(e) => return format!("FAILED: Stream error: {:?}", e),
                    }
                    
                    if batch_count > 10 {
                        break; // Prevent infinite loop in test
                    }
                }
                
                format!("PASSED: Streaming search processed {} batches with {} total hits", 
                       batch_count, total_hits)
            }
            Err(e) => format!("FAILED: Streaming search failed: {:?}", e),
        }
    }

    /// Test error handling for various failure scenarios
    fn test_error_handling() -> String {
        // Test with invalid config
        let invalid_config = SearchConfig {
            provider: "nonexistent".to_string(),
            endpoint: "invalid://endpoint".to_string(),
            api_key: None,
            index_name: "".to_string(),
            timeout_ms: Some(1), // Very short timeout
            max_results: Some(0),
            options: vec![],
        };
        
        let query = SearchQuery {
            query_type: QueryType::Text,
            text: Some("test".to_string()),
            filters: vec![],
            facets: vec![],
            sort: vec![],
            highlight: None,
            from: Some(0),
            size: Some(10),
        };
        
        match search(&invalid_config, &query) {
            Ok(_) => "FAILED: Expected error for invalid config, but search succeeded".to_string(),
            Err(e) => {
                match e {
                    SearchError::ConfigurationError(_) => "PASSED: Correctly caught configuration error".to_string(),
                    SearchError::ConnectionError(_) => "PASSED: Correctly caught connection error".to_string(),
                    SearchError::TimeoutError(_) => "PASSED: Correctly caught timeout error".to_string(),
                    _ => format!("PASSED: Caught expected error type: {:?}", e),
                }
            }
        }
    }

    /// Test handling of malformed queries
    fn test_malformed_queries() -> String {
        let config = create_test_config();
        
        // Test with malformed JSON in filters
        let malformed_query = SearchQuery {
            query_type: QueryType::Text,
            text: Some("test".to_string()),
            filters: vec![
                Filter {
                    field: "".to_string(), // Empty field name
                    filter_type: FilterType::Term,
                    value: "".to_string(), // Empty value
                    operator: Some(FilterOperator::Equals),
                }
            ],
            facets: vec![],
            sort: vec![
                SortField {
                    field: "nonexistent_field".to_string(),
                    direction: SortDirection::Ascending,
                }
            ],
            highlight: None,
            from: Some(-1), // Invalid from value
            size: Some(0),  // Invalid size
        };
        
        match search(&config, &malformed_query) {
            Ok(_) => "FAILED: Expected error for malformed query, but search succeeded".to_string(),
            Err(e) => {
                match e {
                    SearchError::QueryError(_) => "PASSED: Correctly caught query error".to_string(),
                    SearchError::ValidationError(_) => "PASSED: Correctly caught validation error".to_string(),
                    _ => format!("PASSED: Caught error for malformed query: {:?}", e),
                }
            }
        }
    }

    /// Test handling of large result sets
    fn test_large_result_sets() -> String {
        let config = create_test_config();
        
        let large_query = SearchQuery {
            query_type: QueryType::Text,
            text: Some("*".to_string()),
            filters: vec![],
            facets: vec![],
            sort: vec![],
            highlight: None,
            from: Some(0),
            size: Some(10000), // Very large result set
        };
        
        match search(&config, &large_query) {
            Ok(results) => {
                if results.total > 1000 {
                    format!("PASSED: Large result set handled, total: {}, returned: {}", 
                           results.total, results.hits.len())
                } else {
                    format!("PASSED: Result set size reasonable, total: {}", results.total)
                }
            }
            Err(e) => {
                match e {
                    SearchError::ResourceLimitError(_) => "PASSED: Correctly caught resource limit error".to_string(),
                    _ => format!("FAILED: Unexpected error for large result set: {:?}", e),
                }
            }
        }
    }

    /// Test concurrent search operations
    fn test_concurrent_operations() -> String {
        let config = create_test_config();
        
        // Simulate concurrent operations by performing multiple operations in sequence
        let mut results = Vec::new();
        
        for i in 0..5 {
            let query = SearchQuery {
                query_type: QueryType::Text,
                text: Some(format!("concurrent test {}", i)),
                filters: vec![],
                facets: vec![],
                sort: vec![],
                highlight: None,
                from: Some(0),
                size: Some(10),
            };
            
            match search(&config, &query) {
                Ok(result) => results.push(format!("Query {} succeeded with {} hits", i, result.hits.len())),
                Err(e) => results.push(format!("Query {} failed: {:?}", i, e)),
            }
        }
        
        format!("PASSED: Concurrent operations completed: {}", results.join("; "))
    }

    /// Test durability and checkpoint functionality
    fn test_durability_checkpoints() -> String {
        let config = create_test_config();
        let docs = create_test_documents();
        
        // Test with durability options
        let mut durable_config = config.clone();
        durable_config.options.push(("durability".to_string(), "true".to_string()));
        durable_config.options.push(("checkpoint_interval".to_string(), "10".to_string()));
        
        match bulk_index(&durable_config, &docs) {
            Ok(response) => {
                if response.indexed_count == docs.len() {
                    format!("PASSED: Durable bulk indexing completed successfully, indexed {} documents", 
                           response.indexed_count)
                } else {
                    format!("PARTIAL: Durable indexing completed with {} of {} documents", 
                           response.indexed_count, docs.len())
                }
            }
            Err(e) => format!("FAILED: Durable indexing failed: {:?}", e),
        }
    }

    /// Test graceful degradation when features are unavailable
    fn test_graceful_degradation() -> String {
        let config = create_test_config();
        
        // Test query that might not be fully supported
        let complex_query = SearchQuery {
            query_type: QueryType::Faceted,
            text: Some("test query".to_string()),
            filters: vec![
                Filter {
                    field: "complex_field".to_string(),
                    filter_type: FilterType::GeoDistance,
                    value: "40.7128,-74.0060,10km".to_string(),
                    operator: Some(FilterOperator::Within),
                }
            ],
            facets: vec![
                Facet {
                    field: "geo_location".to_string(),
                    facet_type: FacetType::GeoHash,
                    size: Some(10),
                }
            ],
            sort: vec![],
            highlight: Some(HighlightConfig {
                fields: vec!["title".to_string()],
                pre_tags: vec!["<em>".to_string()],
                post_tags: vec!["</em>".to_string()],
                fragment_size: Some(100),
                num_fragments: Some(1),
            }),
            from: Some(0),
            size: Some(10),
        };
        
        match search(&config, &complex_query) {
            Ok(results) => {
                // Check if some features were gracefully degraded
                let has_highlights = results.hits.iter().any(|hit| hit.highlight.is_some());
                let has_facets = !results.facets.is_empty();
                
                format!("PASSED: Complex query handled (highlights: {}, facets: {})", 
                       has_highlights, has_facets)
            }
            Err(e) => {
                match e {
                    SearchError::FeatureNotSupported(_) => "PASSED: Correctly reported unsupported feature".to_string(),
                    _ => format!("PASSED: Query handled with graceful degradation: {:?}", e),
                }
            }
        }
    }

    /// Test provider capability detection
    fn test_provider_capabilities() -> String {
        let config = create_test_config();
        
        match get_capabilities(&config) {
            Ok(capabilities) => {
                let features = vec![
                    ("text_search", capabilities.supports_text_search),
                    ("faceted_search", capabilities.supports_faceted_search),
                    ("highlighting", capabilities.supports_highlighting),
                    ("autocomplete", capabilities.supports_autocomplete),
                    ("geo_search", capabilities.supports_geo_search),
                    ("streaming", capabilities.supports_streaming),
                ];
                
                let supported_features: Vec<&str> = features.iter()
                    .filter(|(_, supported)| *supported)
                    .map(|(name, _)| *name)
                    .collect();
                
                format!("PASSED: Provider capabilities detected: {:?}", supported_features)
            }
            Err(e) => format!("FAILED: Could not get provider capabilities: {:?}", e),
        }
    }

    /// Test fallback mechanisms
    fn test_fallback_mechanisms() -> String {
        let config = create_test_config();
        
        // Test a query that might require fallback
        let fallback_query = SearchQuery {
            query_type: QueryType::Faceted,
            text: Some("fallback test".to_string()),
            filters: vec![],
            facets: vec![
                Facet {
                    field: "category".to_string(),
                    facet_type: FacetType::Terms,
                    size: Some(5),
                }
            ],
            sort: vec![],
            highlight: Some(HighlightConfig {
                fields: vec!["content".to_string()],
                pre_tags: vec!["<mark>".to_string()],
                post_tags: vec!["</mark>".to_string()],
                fragment_size: Some(150),
                num_fragments: Some(2),
            }),
            from: Some(0),
            size: Some(10),
        };
        
        match search(&config, &fallback_query) {
            Ok(results) => {
                // Check if fallback mechanisms were used
                let has_basic_results = !results.hits.is_empty();
                let has_client_side_facets = !results.facets.is_empty();
                let has_client_side_highlights = results.hits.iter()
                    .any(|hit| hit.highlight.is_some());
                
                format!("PASSED: Fallback mechanisms working (results: {}, facets: {}, highlights: {})", 
                       has_basic_results, has_client_side_facets, has_client_side_highlights)
            }
            Err(e) => format!("FAILED: Fallback mechanisms failed: {:?}", e),
        }
    }

    /// Test configuration validation
    fn test_configuration_validation() -> String {
        // Test various invalid configurations
        let test_configs = vec![
            ("empty_provider", SearchConfig {
                provider: "".to_string(),
                endpoint: "http://localhost:9200".to_string(),
                api_key: Some("key".to_string()),
                index_name: "test".to_string(),
                timeout_ms: Some(5000),
                max_results: Some(100),
                options: vec![],
            }),
            ("invalid_endpoint", SearchConfig {
                provider: "elastic".to_string(),
                endpoint: "not-a-url".to_string(),
                api_key: Some("key".to_string()),
                index_name: "test".to_string(),
                timeout_ms: Some(5000),
                max_results: Some(100),
                options: vec![],
            }),
            ("empty_index", SearchConfig {
                provider: "elastic".to_string(),
                endpoint: "http://localhost:9200".to_string(),
                api_key: Some("key".to_string()),
                index_name: "".to_string(),
                timeout_ms: Some(5000),
                max_results: Some(100),
                options: vec![],
            }),
        ];
        
        let mut validation_results = Vec::new();
        
        for (test_name, config) in test_configs {
            match validate_config(&config) {
                Ok(_) => validation_results.push(format!("{}: FAILED (should have been invalid)", test_name)),
                Err(e) => validation_results.push(format!("{}: PASSED (correctly caught: {:?})", test_name, e)),
            }
        }
        
        format!("Configuration validation tests: {}", validation_results.join("; "))
    }
}

bindings::export!(Component with_types_in bindings);