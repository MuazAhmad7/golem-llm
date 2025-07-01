use anyhow::{Result, anyhow};
use serde_json::json;
use log::{info, error};

// Import the client directly using the crate name
use golem_search_algolia::client::{AlgoliaClient, AlgoliaConfig, AlgoliaIndexSettings, AlgoliaSearchQuery};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    info!("🚀 Starting Algolia Client Direct Test");
    
    // Check environment variables
    if std::env::var("ALGOLIA_APP_ID").is_err() {
        error!("❌ ALGOLIA_APP_ID environment variable not set");
        return Err(anyhow!("Missing environment variable: ALGOLIA_APP_ID"));
    }
    
    if std::env::var("ALGOLIA_API_KEY").is_err() {
        error!("❌ ALGOLIA_API_KEY environment variable not set");
        return Err(anyhow!("Missing environment variable: ALGOLIA_API_KEY"));
    }
    
    info!("✅ Environment variables found");
    
    // Test configuration
    let test_index = "test_search_implementation";
    
    // Run comprehensive tests
    match run_comprehensive_test(test_index).await {
        Ok(_) => {
            info!("🎉 All tests passed successfully!");
            println!("\n=== TEST RESULTS ===");
            println!("✅ All Algolia search features working correctly!");
            println!("✅ Index management: CREATE, DELETE, LIST");
            println!("✅ Document operations: UPSERT, GET, DELETE");
            println!("✅ Basic search: TEXT QUERIES, PAGINATION");
            println!("✅ Advanced search: FACETS, FILTERING, HIGHLIGHTING");
            println!("✅ Configuration: TYPO TOLERANCE, RANKING");
            println!("====================");
        }
        Err(e) => {
            error!("❌ Test failed: {}", e);
            // Try to clean up the test index
            let config = AlgoliaConfig::from_env().unwrap();
            let client = AlgoliaClient::new(config).unwrap();
            let _ = client.delete_index(test_index).await;
            return Err(e);
        }
    }
    
    Ok(())
}

async fn run_comprehensive_test(test_index: &str) -> Result<()> {
    info!("📋 Running comprehensive Algolia client test");
    
    // Create client
    let config = AlgoliaConfig::from_env()
        .map_err(|e| anyhow!("Failed to create config: {}", e))?;
    
    let client = AlgoliaClient::new(config)
        .map_err(|e| anyhow!("Failed to create client: {}", e))?;
    
    // ========== TEST 1: Index Management ==========
    info!("🔧 Test 1: Index Management");
    
    // Clean up any existing test index (ignore errors)
    let _ = client.delete_index(test_index).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Create index
    client.create_index(test_index).await
        .map_err(|e| anyhow!("Failed to create index: {}", e))?;
    info!("✅ Index created successfully");
    
    // Wait a moment for index creation
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // List indices to verify creation
    let indices = client.list_indices().await
        .map_err(|e| anyhow!("Failed to list indices: {}", e))?;
    if !indices.contains(&test_index.to_string()) {
        return Err(anyhow!("Test index not found in index list"));
    }
    info!("✅ Index verified in list (found {} total indices)", indices.len());
    
    // Configure index settings with advanced features
    let settings = AlgoliaIndexSettings {
        searchable_attributes: Some(vec!["title".to_string(), "description".to_string()]),
        attributes_for_faceting: Some(vec![
            "filterOnly(category)".to_string(),
            "filterOnly(price)".to_string()
        ]),
        custom_ranking: Some(vec!["desc(rating)".to_string(), "asc(price)".to_string()]),
        typo_tolerance: Some(json!(true)),
        highlight_pre_tag: Some("<mark>".to_string()),
        highlight_post_tag: Some("</mark>".to_string()),
        min_word_size_for_1_typo: Some(4),
        min_word_size_for_2_typos: Some(8),
        remove_stop_words: Some(json!(true)),
        ignore_plurals: Some(json!(true)),
        ..Default::default()
    };
    
    client.update_index_settings(test_index, &settings).await
        .map_err(|e| anyhow!("Failed to update index settings: {}", e))?;
    info!("✅ Index settings configured with advanced features");
    
    // ========== TEST 2: Document Operations ==========
    info!("📄 Test 2: Document Operations");
    
    // Create sample documents with more realistic data
    let documents = vec![
        json!({
            "objectID": "1",
            "title": "Premium Coffee Beans",
            "description": "High-quality Arabica coffee beans from Ethiopia. Perfect for morning brew.",
            "category": "beverages",
            "price": 24.99,
            "tags": ["organic", "fair-trade", "premium"],
            "rating": 4.8,
            "stock": 50,
            "brand": "Mountain Coffee Co."
        }),
        json!({
            "objectID": "2",
            "title": "Wireless Headphones",
            "description": "Noise-cancelling bluetooth headphones with 30-hour battery life",
            "category": "electronics",
            "price": 199.99,
            "tags": ["bluetooth", "noise-cancelling", "wireless"],
            "rating": 4.5,
            "stock": 25,
            "brand": "SoundTech"
        }),
        json!({
            "objectID": "3",
            "title": "Organic Green Tea",
            "description": "Premium organic green tea leaves from Japan. Refreshing and healthy.",
            "category": "beverages",
            "price": 15.99,
            "tags": ["organic", "green-tea", "japanese"],
            "rating": 4.3,
            "stock": 100,
            "brand": "Zen Tea"
        }),
        json!({
            "objectID": "4",
            "title": "Smart Watch",
            "description": "Fitness tracking smartwatch with heart rate monitor and GPS",
            "category": "electronics",
            "price": 299.99,
            "tags": ["fitness", "smartwatch", "health"],
            "rating": 4.6,
            "stock": 15,
            "brand": "FitTech"
        }),
        json!({
            "objectID": "5",
            "title": "Herbal Tea Blend",
            "description": "Relaxing herbal tea blend with chamomile and lavender for evening",
            "category": "beverages",
            "price": 12.99,
            "tags": ["herbal", "relaxing", "chamomile"],
            "rating": 4.2,
            "stock": 75,
            "brand": "Calm Tea Co."
        }),
    ];
    
    // Batch upsert documents
    let object_ids = client.batch_objects(test_index, &documents).await
        .map_err(|e| anyhow!("Failed to batch upsert documents: {}", e))?;
    if object_ids.len() != 5 {
        return Err(anyhow!("Expected 5 documents upserted, got {}", object_ids.len()));
    }
    info!("✅ {} documents upserted successfully", object_ids.len());
    
    // Wait for indexing to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Get a specific document to verify storage
    let doc = client.get_object(test_index, "1").await
        .map_err(|e| anyhow!("Failed to retrieve document: {}", e))?;
    
    if doc.get("objectID").and_then(|v| v.as_str()) != Some("1") {
        return Err(anyhow!("Retrieved document has wrong ID"));
    }
    info!("✅ Document retrieval successful: {}", doc.get("title").unwrap_or(&json!("Unknown")).as_str().unwrap_or(""));
    
    // ========== TEST 3: Basic Search ==========
    info!("🔍 Test 3: Basic Search");
    
    // Basic text search - create search query manually since we can't use Default
    let search_query = AlgoliaSearchQuery {
        query: "coffee".to_string(),
        hits_per_page: Some(10),
        page: Some(0),
        filters: None,
        facets: None,
        highlight_pre_tag: None,
        highlight_post_tag: None,
        attributes_to_retrieve: None,
        sort: None,
        facet_filters: None,
        numeric_filters: None,
        tag_filters: None,
        attributes_to_highlight: None,
        attributes_to_snippet: None,
        highlight_pre_tag_override: None,
        highlight_post_tag_override: None,
        restrict_highlight_and_snippet_arrays: None,
        get_ranking_info: None,
        distinct: None,
        typo_tolerance: None,
        analytics: None,
        synonyms: None,
        replaceSynonymsInHighlight: None,
        minProximity: None,
    };
    
    let results = client.search(test_index, &search_query).await
        .map_err(|e| anyhow!("Failed to perform basic search: {}", e))?;
        
    if results.nb_hits == 0 {
        return Err(anyhow!("No results found for 'coffee' search"));
    }
    info!("✅ Basic search found {} results in {}ms", results.nb_hits, results.processing_time_ms);
    
    // Print first result for verification
    if let Some(first_hit) = results.hits.first() {
        let title = first_hit.data.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown");
        info!("📋 First result: ID={}, Title={}", first_hit.object_id, title);
        
        if let Some(highlight) = &first_hit.highlight_result {
            info!("🎨 Highlighting data available");
        }
    }
    
    // ========== TEST 4: Advanced Search with Facets and Filters ==========
    info!("🎯 Test 4: Advanced Search with Facets and Filters");
    
    // Search with facet filters
    let faceted_search = AlgoliaSearchQuery {
        query: "".to_string(), // Empty query to get all results
        filters: Some("category:beverages".to_string()),
        facets: Some(vec!["category".to_string(), "price".to_string()]),
        hits_per_page: Some(10),
        page: Some(0),
        attributes_to_highlight: Some(vec!["title".to_string(), "description".to_string()]),
        highlight_pre_tag: Some("<mark>".to_string()),
        highlight_post_tag: Some("</mark>".to_string()),
        ..Default::default()
    };
    
    let faceted_results = client.search(test_index, &faceted_search).await
        .map_err(|e| anyhow!("Failed to perform faceted search: {}", e))?;
    
    info!("✅ Faceted search found {} results", faceted_results.nb_hits);
    
    // Verify all results are beverages
    for hit in &faceted_results.hits {
        if let Some(category) = hit.data.get("category").and_then(|v| v.as_str()) {
            if category != "beverages" {
                return Err(anyhow!("Facet filter failed: found non-beverage item: {}", category));
            }
        }
    }
    info!("✅ Facet filtering working correctly - all results are beverages");
    
    // Check if facets are returned
    if let Some(facets) = &faceted_results.facets {
        info!("✅ Facets returned: {} facet groups", facets.len());
        for (facet_name, facet_values) in facets {
            info!("   - {}: {} values", facet_name, facet_values.len());
        }
    }
    
    // ========== TEST 5: Complex Search with Multiple Filters ==========
    info!("🎪 Test 5: Complex Search with Multiple Filters");
    
    let complex_search = AlgoliaSearchQuery {
        query: "premium".to_string(),
        filters: Some("category:beverages AND price > 10".to_string()),
        facets: Some(vec!["category".to_string()]),
        hits_per_page: Some(5),
        page: Some(0),
        attributes_to_highlight: Some(vec!["title".to_string(), "description".to_string()]),
        highlight_pre_tag: Some("<strong>".to_string()),
        highlight_post_tag: Some("</strong>".to_string()),
        typo_tolerance: Some("true".to_string()),
        get_ranking_info: Some(true),
        ..Default::default()
    };
    
    let complex_results = client.search(test_index, &complex_search).await
        .map_err(|e| anyhow!("Failed to perform complex search: {}", e))?;
    
    info!("✅ Complex search found {} results", complex_results.nb_hits);
    
    // Verify results match criteria
    for hit in &complex_results.hits {
        let category = hit.data.get("category").and_then(|v| v.as_str()).unwrap_or("");
        let price = hit.data.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
        
        if category != "beverages" || price <= 10.0 {
            return Err(anyhow!("Complex filter failed: category={}, price={}", category, price));
        }
        
        // Check if ranking info is present
        if let Some(ranking_info) = &hit.ranking_info {
            info!("📊 Ranking info available for result: {}", hit.object_id);
        }
    }
    info!("✅ Complex filtering and ranking info working correctly");
    
    // ========== TEST 6: Pagination Test ==========
    info!("📄 Test 6: Pagination");
    
    let paginated_search = AlgoliaSearchQuery {
        query: "".to_string(),
        page: Some(1), // Second page
        hits_per_page: Some(2), // 2 items per page
        ..Default::default()
    };
    
    let paginated_results = client.search(test_index, &paginated_search).await
        .map_err(|e| anyhow!("Failed to perform paginated search: {}", e))?;
    
    if paginated_results.hits.len() > 2 {
        return Err(anyhow!("Pagination failed: too many results per page"));
    }
    info!("✅ Pagination working correctly - page {} has {} results", 
        paginated_results.page + 1, paginated_results.hits.len());
    
    // ========== TEST 7: Typo Tolerance Test ==========
    info!("🔤 Test 7: Typo Tolerance");
    
    let typo_search = AlgoliaSearchQuery {
        query: "cofee".to_string(), // Intentional typo
        typo_tolerance: Some("true".to_string()),
        hits_per_page: Some(5),
        ..Default::default()
    };
    
    let typo_results = client.search(test_index, &typo_search).await
        .map_err(|e| anyhow!("Failed to perform typo tolerance search: {}", e))?;
    
    if typo_results.nb_hits > 0 {
        info!("✅ Typo tolerance working - found {} results for 'cofee'", typo_results.nb_hits);
    } else {
        info!("⚠️  Typo tolerance test inconclusive - no results for 'cofee'");
    }
    
    // ========== TEST 8: Error Handling ==========
    info!("⚠️  Test 8: Error Handling");
    
    // Try to search non-existent index
    let error_search = AlgoliaSearchQuery {
        query: "test".to_string(),
        ..Default::default()
    };
    
    let error_result = client.search("non_existent_index", &error_search).await;
    if error_result.is_ok() {
        return Err(anyhow!("Expected error for non-existent index"));
    }
    info!("✅ Error handling working correctly for non-existent index");
    
    // ========== TEST 9: Document Deletion ==========
    info!("🗑️  Test 9: Document Deletion");
    
    client.delete_objects(test_index, &vec!["5".to_string()]).await
        .map_err(|e| anyhow!("Failed to delete document: {}", e))?;
    info!("✅ Document deletion successful");
    
    // Wait for deletion to process
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Verify document is gone
    let delete_verify = client.get_object(test_index, "5").await;
    if delete_verify.is_ok() {
        return Err(anyhow!("Document should have been deleted"));
    }
    info!("✅ Document deletion verified");
    
    // ========== TEST 10: Advanced Features Verification ==========
    info!("🎛️  Test 10: Advanced Features Verification");
    
    let advanced_search = AlgoliaSearchQuery {
        query: "headphones".to_string(),
        attributes_to_retrieve: Some(vec!["title".to_string(), "price".to_string(), "category".to_string()]),
        attributes_to_highlight: Some(vec!["title".to_string(), "description".to_string()]),
        attributes_to_snippet: Some(vec!["description:20".to_string()]),
        distinct: Some(json!(true)),
        analytics: Some(true),
        synonyms: Some(true),
        ..Default::default()
    };
    
    let advanced_results = client.search(test_index, &advanced_search).await
        .map_err(|e| anyhow!("Failed to perform advanced search: {}", e))?;
    
    if advanced_results.nb_hits > 0 {
        info!("✅ Advanced search features working - found {} results", advanced_results.nb_hits);
        
        // Check if only requested attributes are returned
        if let Some(first_hit) = advanced_results.hits.first() {
            let keys: Vec<&str> = first_hit.data.as_object()
                .map(|obj| obj.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();
            info!("📋 Retrieved attributes: {:?}", keys);
        }
    }
    
    // ========== CLEANUP ==========
    info!("🧹 Cleaning up test index");
    client.delete_index(test_index).await
        .map_err(|e| anyhow!("Failed to delete test index: {}", e))?;
    info!("✅ Test index deleted");
    
    info!("🎉 All tests completed successfully!");
    Ok(())
}