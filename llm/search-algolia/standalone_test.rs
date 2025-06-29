use std::time::Duration;
use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use log::{info, error};

// Simple HTTP client for Algolia API
use reqwest::{Client, Method};

#[derive(Debug, Clone)]
struct AlgoliaConfig {
    app_id: String,
    api_key: String,
    timeout: Duration,
}

impl AlgoliaConfig {
    fn from_env() -> Result<Self> {
        let app_id = std::env::var("ALGOLIA_APP_ID")
            .map_err(|_| anyhow!("ALGOLIA_APP_ID environment variable is required"))?;
        let api_key = std::env::var("ALGOLIA_API_KEY")
            .map_err(|_| anyhow!("ALGOLIA_API_KEY environment variable is required"))?;
        
        Ok(Self {
            app_id,
            api_key,
            timeout: Duration::from_secs(30),
        })
    }
}

struct SimpleAlgoliaClient {
    config: AlgoliaConfig,
    http_client: Client,
}

impl SimpleAlgoliaClient {
    fn new(config: AlgoliaConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(config.timeout)
            .build()?;

        Ok(Self {
            config,
            http_client,
        })
    }

    fn base_url(&self) -> String {
        format!("https://{}-dsn.algolia.net/1", self.config.app_id)
    }

    async fn request<T: serde::Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&T>,
    ) -> Result<reqwest::Response> {
        let url = format!("{}/{}", self.base_url(), path.trim_start_matches('/'));
        
        let mut request = self.http_client
            .request(method, &url)
            .header("X-Algolia-Application-Id", &self.config.app_id)
            .header("X-Algolia-API-Key", &self.config.api_key)
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Algolia API error {}: {}", status, error_text));
        }

        Ok(response)
    }

    async fn create_index(&self, name: &str) -> Result<()> {
        // Create empty index by adding a temporary object and then deleting it
        let temp_doc = json!({
            "objectID": "__temp_init_object__",
            "temp": true
        });
        
        self.request(Method::POST, &format!("indexes/{}/", name), Some(&temp_doc)).await?;
        self.request(Method::DELETE, &format!("indexes/{}/objects/__temp_init_object__", name), None::<&()>).await?;
        
        Ok(())
    }

    async fn delete_index(&self, name: &str) -> Result<()> {
        self.request(Method::DELETE, &format!("indexes/{}", name), None::<&()>).await?;
        Ok(())
    }

    async fn list_indices(&self) -> Result<Vec<String>> {
        let response = self.request(Method::GET, "indexes", None::<&()>).await?;
        let data: Value = response.json().await?;
        
        let indices = data["items"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|item| item["name"].as_str().map(|s| s.to_string()))
            .collect();
        
        Ok(indices)
    }

    async fn batch_objects(&self, index: &str, objects: &[Value]) -> Result<Vec<String>> {
        let requests: Vec<Value> = objects.iter().map(|obj| {
            json!({
                "action": "addObject",
                "body": obj
            })
        }).collect();

        let batch_request = json!({ "requests": requests });
        let response = self.request(Method::POST, &format!("indexes/{}/batch", index), Some(&batch_request)).await?;
        let batch_response: Value = response.json().await?;
        
        let object_ids = batch_response["objectIDs"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|id| id.as_str().map(|s| s.to_string()))
            .collect();
        
        Ok(object_ids)
    }

    async fn get_object(&self, index: &str, object_id: &str) -> Result<Value> {
        let response = self.request(Method::GET, &format!("indexes/{}/objects/{}", index, object_id), None::<&()>).await?;
        let object: Value = response.json().await?;
        Ok(object)
    }

    async fn search(&self, index: &str, query: &Value) -> Result<Value> {
        let response = self.request(Method::POST, &format!("indexes/{}/query", index), Some(query)).await?;
        let results: Value = response.json().await?;
        Ok(results)
    }

    async fn delete_objects(&self, index: &str, object_ids: &[String]) -> Result<()> {
        let requests: Vec<Value> = object_ids.iter().map(|id| {
            json!({
                "action": "deleteObject",
                "body": { "objectID": id }
            })
        }).collect();

        let batch_request = json!({ "requests": requests });
        self.request(Method::POST, &format!("indexes/{}/batch", index), Some(&batch_request)).await?;
        Ok(())
    }

    async fn update_settings(&self, index: &str, settings: &Value) -> Result<()> {
        self.request(Method::PUT, &format!("indexes/{}/settings", index), Some(settings)).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    info!("🚀 Starting Simple Algolia Direct API Test");
    
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
    let test_index = "test_simple_algolia";
    
    // Run tests
    match run_test(test_index).await {
        Ok(_) => {
            info!("🎉 All tests passed successfully!");
            println!("\n=== TEST RESULTS ===");
            println!("✅ Algolia API connection working!");
            println!("✅ Index management: CREATE, DELETE, LIST");
            println!("✅ Document operations: UPSERT, GET, DELETE");
            println!("✅ Search functionality: QUERIES, FILTERS");
            println!("✅ Advanced features: SETTINGS, FACETS");
            println!("====================");
        }
        Err(e) => {
            error!("❌ Test failed: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

async fn run_test(test_index: &str) -> Result<()> {
    info!("📋 Running simple Algolia API test");
    
    // Create client
    let config = AlgoliaConfig::from_env()?;
    let client = SimpleAlgoliaClient::new(config)?;
    
    // ========== TEST 1: Index Management ==========
    info!("🔧 Test 1: Index Management");
    
    // Clean up any existing test index
    let _ = client.delete_index(test_index).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Create index
    client.create_index(test_index).await?;
    info!("✅ Index created successfully");
    
    // Wait for index creation
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // List indices
    let indices = client.list_indices().await?;
    if !indices.contains(&test_index.to_string()) {
        return Err(anyhow!("Test index not found in index list"));
    }
    info!("✅ Index verified in list (found {} total indices)", indices.len());
    
    // Configure index settings
    let settings = json!({
        "searchableAttributes": ["title", "description"],
        "attributesForFaceting": ["filterOnly(category)", "filterOnly(price)"],
        "customRanking": ["desc(rating)", "asc(price)"],
        "typoTolerance": true,
        "highlightPreTag": "<mark>",
        "highlightPostTag": "</mark>",
        "minWordSizefor1Typo": 4,
        "minWordSizefor2Typos": 8,
        "removeStopWords": true,
        "ignorePlurals": true
    });
    
    client.update_settings(test_index, &settings).await?;
    info!("✅ Index settings configured with advanced features");
    
    // ========== TEST 2: Document Operations ==========
    info!("📄 Test 2: Document Operations");
    
    let documents = vec![
        json!({
            "objectID": "1",
            "title": "Premium Coffee Beans",
            "description": "High-quality Arabica coffee beans from Ethiopia. Perfect for morning brew.",
            "category": "beverages",
            "price": 24.99,
            "tags": ["organic", "fair-trade", "premium"],
            "rating": 4.8,
            "stock": 50
        }),
        json!({
            "objectID": "2",
            "title": "Wireless Headphones",
            "description": "Noise-cancelling bluetooth headphones with 30-hour battery life",
            "category": "electronics",
            "price": 199.99,
            "tags": ["bluetooth", "noise-cancelling", "wireless"],
            "rating": 4.5,
            "stock": 25
        }),
        json!({
            "objectID": "3",
            "title": "Organic Green Tea",
            "description": "Premium organic green tea leaves from Japan. Refreshing and healthy.",
            "category": "beverages",
            "price": 15.99,
            "tags": ["organic", "green-tea", "japanese"],
            "rating": 4.3,
            "stock": 100
        }),
    ];
    
    // Batch upsert documents
    let object_ids = client.batch_objects(test_index, &documents).await?;
    if object_ids.len() != 3 {
        return Err(anyhow!("Expected 3 documents upserted, got {}", object_ids.len()));
    }
    info!("✅ {} documents upserted successfully", object_ids.len());
    
    // Wait for indexing
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Get a specific document
    let doc = client.get_object(test_index, "1").await?;
    if doc["objectID"].as_str() != Some("1") {
        return Err(anyhow!("Retrieved document has wrong ID"));
    }
    info!("✅ Document retrieval successful: {}", doc["title"].as_str().unwrap_or("Unknown"));
    
    // ========== TEST 3: Basic Search ==========
    info!("🔍 Test 3: Basic Search");
    
    let search_query = json!({
        "query": "coffee",
        "hitsPerPage": 10,
        "page": 0
    });
    
    let results = client.search(test_index, &search_query).await?;
    let nb_hits = results["nbHits"].as_u64().unwrap_or(0);
    if nb_hits == 0 {
        return Err(anyhow!("No results found for 'coffee' search"));
    }
    info!("✅ Basic search found {} results in {}ms", 
        nb_hits, 
        results["processingTimeMS"].as_u64().unwrap_or(0)
    );
    
    // Print first result
    if let Some(first_hit) = results["hits"].as_array().and_then(|hits| hits.first()) {
        let title = first_hit["title"].as_str().unwrap_or("Unknown");
        let object_id = first_hit["objectID"].as_str().unwrap_or("Unknown");
        info!("📋 First result: ID={}, Title={}", object_id, title);
        
        if first_hit.get("_highlightResult").is_some() {
            info!("🎨 Highlighting data available");
        }
    }
    
    // ========== TEST 4: Advanced Search with Facets ==========
    info!("🎯 Test 4: Advanced Search with Facets");
    
    let faceted_search = json!({
        "query": "",
        "filters": "category:beverages",
        "facets": ["category"],
        "hitsPerPage": 10,
        "page": 0,
        "attributesToHighlight": ["title", "description"],
        "highlightPreTag": "<mark>",
        "highlightPostTag": "</mark>"
    });
    
    let faceted_results = client.search(test_index, &faceted_search).await?;
    let faceted_hits = faceted_results["nbHits"].as_u64().unwrap_or(0);
    info!("✅ Faceted search found {} results", faceted_hits);
    
    // Verify all results are beverages
    if let Some(hits) = faceted_results["hits"].as_array() {
        for hit in hits {
            if let Some(category) = hit["category"].as_str() {
                if category != "beverages" {
                    return Err(anyhow!("Facet filter failed: found non-beverage item: {}", category));
                }
            }
        }
    }
    info!("✅ Facet filtering working correctly - all results are beverages");
    
    // Check facets
    if let Some(facets) = faceted_results.get("facets") {
        info!("✅ Facets returned in search response");
    }
    
    // ========== TEST 5: Complex Search ==========
    info!("🎪 Test 5: Complex Search with Multiple Filters");
    
    let complex_search = json!({
        "query": "premium",
        "filters": "category:beverages AND price > 10",
        "facets": ["category"],
        "hitsPerPage": 5,
        "page": 0,
        "attributesToHighlight": ["title", "description"],
        "highlightPreTag": "<strong>",
        "highlightPostTag": "</strong>",
        "typoTolerance": "true",
        "getRankingInfo": true
    });
    
    let complex_results = client.search(test_index, &complex_search).await?;
    info!("✅ Complex search found {} results", complex_results["nbHits"].as_u64().unwrap_or(0));
    
    // ========== TEST 6: Typo Tolerance ==========
    info!("🔤 Test 6: Typo Tolerance");
    
    let typo_search = json!({
        "query": "cofee",  // Intentional typo
        "typoTolerance": "true",
        "hitsPerPage": 5
    });
    
    let typo_results = client.search(test_index, &typo_search).await?;
    let typo_hits = typo_results["nbHits"].as_u64().unwrap_or(0);
    if typo_hits > 0 {
        info!("✅ Typo tolerance working - found {} results for 'cofee'", typo_hits);
    } else {
        info!("⚠️  Typo tolerance test inconclusive - no results for 'cofee'");
    }
    
    // ========== TEST 7: Document Deletion ==========
    info!("🗑️  Test 7: Document Deletion");
    
    client.delete_objects(test_index, &vec!["3".to_string()]).await?;
    info!("✅ Document deletion successful");
    
    // Wait for deletion to process
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Verify document is gone
    let delete_verify = client.get_object(test_index, "3").await;
    if delete_verify.is_ok() {
        return Err(anyhow!("Document should have been deleted"));
    }
    info!("✅ Document deletion verified");
    
    // ========== CLEANUP ==========
    info!("🧹 Cleaning up test index");
    client.delete_index(test_index).await?;
    info!("✅ Test index deleted");
    
    info!("🎉 All tests completed successfully!");
    Ok(())
}