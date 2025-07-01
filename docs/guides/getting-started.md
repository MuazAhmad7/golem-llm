# Getting Started Guide

This guide will walk you through creating your first search application using the Golem Search Provider Components. In 15 minutes, you'll have a working search application deployed on the Golem platform.

## Prerequisites

- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs/)
- **Golem CLI**: Install from [Golem documentation](https://golem.cloud/docs)
- **cargo-component**: Install with `cargo install cargo-component`
- **A search provider account**: Choose from ElasticSearch, Typesense, Meilisearch, etc.

## Quick Start (5 Minutes)

### 1. Clone and Build

```bash
# Clone the repository
git clone https://github.com/golemcloud/golem-llm
cd golem-llm/llm

# Build all search providers
cargo component build --release

# Verify builds
ls target/wasm32-wasi/release/*.wasm
```

### 2. Configure Your Provider

Choose your preferred search provider and set up environment variables:

#### Option A: Typesense (Recommended for beginners)
```bash
export SEARCH_PROVIDER_ENDPOINT="https://your-typesense-cluster.a1.typesense.net"
export TYPESENSE_API_KEY="your-admin-api-key"
export SEARCH_PROVIDER_TIMEOUT="5000"
```

#### Option B: Meilisearch (Great for development)
```bash
export SEARCH_PROVIDER_ENDPOINT="https://your-meilisearch-instance.com"
export MEILISEARCH_MASTER_KEY="your-master-key"
export SEARCH_PROVIDER_TIMEOUT="5000"
```

#### Option C: ElasticSearch (Enterprise features)
```bash
export SEARCH_PROVIDER_ENDPOINT="https://your-elastic-cluster.com"
export ELASTICSEARCH_API_KEY="your-api-key"
# OR for basic auth:
export ELASTICSEARCH_USERNAME="elastic"
export ELASTICSEARCH_PASSWORD="your-password"
```

### 3. Deploy to Golem

```bash
# Deploy your chosen provider (example with Typesense)
golem-cli component add search-typesense.wasm --component-name search-provider

# Create a worker instance
golem-cli worker add --component-name search-provider --worker-name my-search
```

🎉 **Congratulations!** You now have a search provider running on Golem!

## Detailed Tutorial: Building an E-commerce Search

Let's build a complete e-commerce search application step by step.

### Step 1: Project Setup

Create a new Rust project for your application:

```bash
mkdir my-search-app
cd my-search-app
cargo init --name search-app

# Add dependencies to Cargo.toml
```

Add these dependencies to your `Cargo.toml`:

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"

# Golem worker SDK (for calling search components)
golem-wasm-rpc = "1.0"
```

### Step 2: Define Your Data Model

Create `src/models.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub brand: String,
    pub price: f64,
    pub rating: f64,
    pub in_stock: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub category: Option<String>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub products: Vec<Product>,
    pub total: u64,
    pub facets: std::collections::HashMap<String, Vec<FacetValue>>,
    pub took_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacetValue {
    pub value: String,
    pub count: u64,
}
```

### Step 3: Implement Search Logic

Create `src/search.rs`:

```rust
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;

use crate::models::{Product, SearchRequest, SearchResponse, FacetValue};

pub struct SearchService {
    worker_id: String,
}

impl SearchService {
    pub fn new(worker_id: String) -> Self {
        Self { worker_id }
    }

    pub async fn index_product(&self, product: &Product) -> Result<()> {
        // Convert product to search document
        let doc = json!({
            "id": product.id,
            "title": product.title,
            "description": product.description,
            "category": product.category,
            "brand": product.brand,
            "price": product.price,
            "rating": product.rating,
            "in_stock": product.in_stock,
            "tags": product.tags,
        });

        // Call the search provider via Golem RPC
        let _result = golem_wasm_rpc::call(
            &self.worker_id,
            "upsert",
            &("products", doc.to_string()),
        ).await?;

        Ok(())
    }

    pub async fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        // Build search query
        let mut query = SearchQuery {
            q: Some(request.query),
            facets: vec!["category".to_string(), "brand".to_string()],
            page: request.page,
            per_page: request.per_page.or(Some(20)),
            ..Default::default()
        };

        // Add filters
        let mut filters = Vec::new();
        
        if let Some(category) = request.category {
            filters.push(format!("category:{}", category));
        }
        
        if let Some(min_price) = request.min_price {
            if let Some(max_price) = request.max_price {
                filters.push(format!("price:[{} TO {}]", min_price, max_price));
            } else {
                filters.push(format!("price:>={}", min_price));
            }
        }
        
        query.filters = filters;

        // Execute search
        let results: SearchResults = golem_wasm_rpc::call(
            &self.worker_id,
            "search",
            &("products", query),
        ).await?;

        // Convert results
        let products: Vec<Product> = results.hits
            .into_iter()
            .filter_map(|hit| {
                hit.content
                    .and_then(|content| serde_json::from_str(&content).ok())
            })
            .collect();

        // Convert facets
        let facets: HashMap<String, Vec<FacetValue>> = results.facets
            .into_iter()
            .map(|(key, values)| {
                let facet_values = values
                    .into_iter()
                    .map(|fv| FacetValue {
                        value: fv.value,
                        count: fv.count,
                    })
                    .collect();
                (key, facet_values)
            })
            .collect();

        Ok(SearchResponse {
            products,
            total: results.total_hits,
            facets,
            took_ms: results.took,
        })
    }

    pub async fn bulk_index(&self, products: Vec<Product>) -> Result<()> {
        let docs: Vec<_> = products
            .into_iter()
            .map(|product| {
                json!({
                    "id": product.id,
                    "title": product.title,
                    "description": product.description,
                    "category": product.category,
                    "brand": product.brand,
                    "price": product.price,
                    "rating": product.rating,
                    "in_stock": product.in_stock,
                    "tags": product.tags,
                }).to_string()
            })
            .collect();

        // Batch insert for efficiency
        let _result = golem_wasm_rpc::call(
            &self.worker_id,
            "batch_upsert",
            &("products", docs),
        ).await?;

        Ok(())
    }
}
```

### Step 4: Create Sample Data

Create `src/sample_data.rs`:

```rust
use crate::models::Product;

pub fn generate_sample_products() -> Vec<Product> {
    vec![
        Product {
            id: "laptop_001".to_string(),
            title: "Gaming Laptop Pro 15".to_string(),
            description: "High-performance gaming laptop with RTX 4080 and 32GB RAM".to_string(),
            category: "electronics".to_string(),
            brand: "TechBrand".to_string(),
            price: 2299.99,
            rating: 4.7,
            in_stock: true,
            tags: vec!["gaming".to_string(), "laptop".to_string(), "high-performance".to_string()],
        },
        Product {
            id: "phone_001".to_string(),
            title: "Smartphone Ultra 128GB".to_string(),
            description: "Latest smartphone with advanced camera and 5G connectivity".to_string(),
            category: "electronics".to_string(),
            brand: "PhoneCorp".to_string(),
            price: 899.99,
            rating: 4.5,
            in_stock: true,
            tags: vec!["smartphone".to_string(), "5g".to_string(), "camera".to_string()],
        },
        Product {
            id: "headphones_001".to_string(),
            title: "Wireless Noise-Canceling Headphones".to_string(),
            description: "Premium wireless headphones with active noise cancellation".to_string(),
            category: "audio".to_string(),
            brand: "AudioMax".to_string(),
            price: 299.99,
            rating: 4.8,
            in_stock: false,
            tags: vec!["wireless".to_string(), "noise-canceling".to_string(), "premium".to_string()],
        },
        Product {
            id: "tablet_001".to_string(),
            title: "Professional Tablet 12 inch".to_string(),
            description: "Professional tablet for creative work with stylus support".to_string(),
            category: "electronics".to_string(),
            brand: "CreativeTech".to_string(),
            price: 649.99,
            rating: 4.3,
            in_stock: true,
            tags: vec!["tablet".to_string(), "creative".to_string(), "stylus".to_string()],
        },
        Product {
            id: "mouse_001".to_string(),
            title: "Ergonomic Gaming Mouse".to_string(),
            description: "High-precision gaming mouse with customizable buttons".to_string(),
            category: "accessories".to_string(),
            brand: "GameGear".to_string(),
            price: 79.99,
            rating: 4.6,
            in_stock: true,
            tags: vec!["gaming".to_string(), "mouse".to_string(), "ergonomic".to_string()],
        },
    ]
}
```

### Step 5: Main Application

Update `src/main.rs`:

```rust
mod models;
mod search;
mod sample_data;

use anyhow::Result;
use search::SearchService;
use models::SearchRequest;
use sample_data::generate_sample_products;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting E-commerce Search Demo");

    // Initialize search service
    let search_service = SearchService::new("my-search".to_string());

    // Create search index with schema
    println!("📋 Creating search index...");
    create_products_index(&search_service).await?;

    // Index sample products
    println!("📦 Indexing sample products...");
    let products = generate_sample_products();
    search_service.bulk_index(products).await?;

    // Wait a moment for indexing
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Perform some example searches
    println!("\n🔍 Running example searches:\n");

    // Basic search
    let results = search_service.search(SearchRequest {
        query: "gaming".to_string(),
        category: None,
        min_price: None,
        max_price: None,
        page: Some(0),
        per_page: Some(10),
    }).await?;

    println!("Search for 'gaming':");
    println!("  Found {} products in {}ms", results.total, results.took_ms);
    for product in &results.products {
        println!("  - {} (${:.2})", product.title, product.price);
    }

    // Category search
    let results = search_service.search(SearchRequest {
        query: "*".to_string(),
        category: Some("electronics".to_string()),
        min_price: None,
        max_price: None,
        page: Some(0),
        per_page: Some(10),
    }).await?;

    println!("\nSearch in 'electronics' category:");
    println!("  Found {} products", results.total);
    for product in &results.products {
        println!("  - {} ({})", product.title, product.category);
    }

    // Price range search
    let results = search_service.search(SearchRequest {
        query: "*".to_string(),
        category: None,
        min_price: Some(100.0),
        max_price: Some(500.0),
        page: Some(0),
        per_page: Some(10),
    }).await?;

    println!("\nSearch for products $100-$500:");
    println!("  Found {} products", results.total);
    for product in &results.products {
        println!("  - {} (${:.2})", product.title, product.price);
    }

    // Show facets
    if !results.facets.is_empty() {
        println!("\nAvailable facets:");
        for (facet_name, values) in &results.facets {
            println!("  {}:", facet_name);
            for value in values {
                println!("    {} ({})", value.value, value.count);
            }
        }
    }

    println!("\n✅ Demo completed successfully!");
    Ok(())
}

async fn create_products_index(search_service: &SearchService) -> Result<()> {
    // Define schema for products
    let schema = json!({
        "fields": [
            {
                "name": "title",
                "type": "text",
                "required": true,
                "facet": false,
                "sort": false,
                "index": true
            },
            {
                "name": "description", 
                "type": "text",
                "required": false,
                "facet": false,
                "sort": false,
                "index": true
            },
            {
                "name": "category",
                "type": "keyword",
                "required": false,
                "facet": true,
                "sort": true,
                "index": true
            },
            {
                "name": "brand",
                "type": "keyword", 
                "required": false,
                "facet": true,
                "sort": true,
                "index": true
            },
            {
                "name": "price",
                "type": "float",
                "required": false,
                "facet": true,
                "sort": true,
                "index": true
            },
            {
                "name": "rating",
                "type": "float",
                "required": false,
                "facet": false,
                "sort": true,
                "index": true
            },
            {
                "name": "in_stock",
                "type": "boolean",
                "required": false,
                "facet": true,
                "sort": false,
                "index": true
            }
        ],
        "primary_key": "id"
    });

    // Create the index
    let _result = golem_wasm_rpc::call(
        &search_service.worker_id,
        "create_index",
        &("products", Some(schema)),
    ).await?;

    Ok(())
}
```

### Step 6: Build and Test

```bash
# Build your application
cargo build

# Run the demo
cargo run
```

You should see output like:

```
🚀 Starting E-commerce Search Demo
📋 Creating search index...
📦 Indexing sample products...

🔍 Running example searches:

Search for 'gaming':
  Found 2 products in 15ms
  - Gaming Laptop Pro 15 ($2299.99)
  - Ergonomic Gaming Mouse ($79.99)

Search in 'electronics' category:
  Found 3 products
  - Gaming Laptop Pro 15 (electronics)
  - Smartphone Ultra 128GB (electronics)
  - Professional Tablet 12 inch (electronics)

✅ Demo completed successfully!
```

## Next Steps

### 1. Advanced Features

Add more sophisticated search features:

```rust
// Highlighting
let query = SearchQuery {
    q: Some("gaming laptop".to_string()),
    highlight: Some(HighlightConfig {
        fields: vec!["title".to_string(), "description".to_string()],
        pre_tag: Some("<mark>".to_string()),
        post_tag: Some("</mark>".to_string()),
        max_length: Some(200),
    }),
    ..Default::default()
};

// Boost certain fields
let query = SearchQuery {
    config: Some(SearchConfig {
        boost_fields: vec![
            ("title".to_string(), 2.0),
            ("description".to_string(), 1.0),
        ],
        typo_tolerance: Some(true),
        ..Default::default()
    }),
    ..Default::default()
};
```

### 2. Error Handling

Implement robust error handling:

```rust
use golem_search::SearchError;

match search_service.search(request).await {
    Ok(results) => process_results(results),
    Err(SearchError::RateLimit(_)) => {
        // Implement retry with backoff
        tokio::time::sleep(Duration::from_secs(1)).await;
        search_service.search(request).await?
    },
    Err(SearchError::Unsupported(feature)) => {
        // Fall back to simpler query
        fallback_search(request).await?
    },
    Err(e) => return Err(e.into()),
}
```

### 3. Performance Optimization

- **Implement caching** for frequent queries
- **Use batch operations** for bulk indexing
- **Monitor query performance** and optimize slow queries
- **Configure appropriate page sizes** for your use case

### 4. Production Deployment

- **Set up monitoring** and alerting
- **Configure proper logging** levels
- **Implement health checks** 
- **Set up backup and recovery** procedures

## Troubleshooting

### Common Issues

**Build Errors:**
```bash
# Update Rust and cargo-component
rustup update
cargo install cargo-component --force
```

**Connection Errors:**
```bash
# Verify environment variables
echo $SEARCH_PROVIDER_ENDPOINT
echo $TYPESENSE_API_KEY

# Test provider connectivity
curl -H "X-TYPESENSE-API-KEY: $TYPESENSE_API_KEY" \
     "$SEARCH_PROVIDER_ENDPOINT/health"
```

**Worker Not Found:**
```bash
# List workers
golem-cli worker list

# Check worker status
golem-cli worker get --worker-name my-search
```

## What's Next?

- **[Provider Selection Guide](provider-selection.md)**: Choose the best search engine for your needs
- **[Advanced Features](../examples/advanced-features.md)**: Explore faceting, highlighting, and more
- **[Performance Optimization](performance-optimization.md)**: Optimize for production workloads
- **[Deployment Guide](deployment.md)**: Production deployment strategies

---

**Need Help?** Join the [Golem Discord](https://discord.gg/golem) community for support and discussions!