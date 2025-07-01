# Core Interface API Reference

The `golem:search` interface provides a unified API for all search providers. This document details every function, type, and behavior defined in the interface.

## Interface Overview

```wit
// golem:search@1.0.0
package golem:search@1.0.0;

world search-provider {
  import golem:search/types@1.0.0;
  export golem:search/core@1.0.0;
}
```

## Core Types

### SearchQuery

The primary structure for defining search operations.

```rust
pub struct SearchQuery {
    /// Search query string (optional for match-all queries)
    pub q: Option<String>,
    
    /// Filter expressions (e.g., "category:electronics", "price:[10 TO 100]")
    pub filters: Vec<String>,
    
    /// Sort criteria (e.g., "price:asc", "rating:desc")
    pub sort: Vec<String>,
    
    /// Fields to facet on for aggregations
    pub facets: Vec<String>,
    
    /// Page number (0-based)
    pub page: Option<u32>,
    
    /// Number of results per page
    pub per_page: Option<u32>,
    
    /// Offset for pagination (alternative to page)
    pub offset: Option<u32>,
    
    /// Highlighting configuration
    pub highlight: Option<HighlightConfig>,
    
    /// Additional search configuration
    pub config: Option<SearchConfig>,
}
```

#### Usage Examples

```rust
// Basic text search
let query = SearchQuery {
    q: Some("laptop computers".to_string()),
    per_page: Some(20),
    ..Default::default()
};

// Complex search with filtering and faceting
let query = SearchQuery {
    q: Some("gaming laptop".to_string()),
    filters: vec![
        "category:electronics".to_string(),
        "price:[500 TO 2000]".to_string(),
        "in_stock:true".to_string(),
    ],
    sort: vec!["rating:desc".to_string(), "price:asc".to_string()],
    facets: vec!["brand".to_string(), "category".to_string()],
    page: Some(0),
    per_page: Some(25),
    highlight: Some(HighlightConfig {
        fields: vec!["title".to_string(), "description".to_string()],
        pre_tag: Some("<mark>".to_string()),
        post_tag: Some("</mark>".to_string()),
        max_length: Some(200),
    }),
    ..Default::default()
};
```

### SearchResults

Contains the complete search response with results and metadata.

```rust
pub struct SearchResults {
    /// Array of search hits
    pub hits: Vec<SearchHit>,
    
    /// Total number of matching documents
    pub total_hits: u64,
    
    /// Query execution time in milliseconds
    pub took: u64,
    
    /// Facet aggregations (if requested)
    pub facets: HashMap<String, Vec<FacetValue>>,
    
    /// Search suggestions (if available)
    pub suggestions: Vec<String>,
    
    /// Provider-specific metadata
    pub metadata: HashMap<String, String>,
}
```

### SearchHit

Individual search result with scoring and highlighting.

```rust
pub struct SearchHit {
    /// Document identifier
    pub id: String,
    
    /// Relevance score (higher = more relevant)
    pub score: Option<f64>,
    
    /// Document content (JSON string)
    pub content: Option<String>,
    
    /// Highlighted snippets (HTML)
    pub highlights: Option<String>,
}
```

### Document Operations

#### Doc

Structure for document indexing operations.

```rust
pub struct Doc {
    /// Unique document identifier
    pub id: String,
    
    /// Document content as JSON string
    pub content: String,
}
```

#### Schema

Defines the structure and indexing behavior for documents.

```rust
pub struct Schema {
    /// Field definitions
    pub fields: Vec<SchemaField>,
    
    /// Primary key field name
    pub primary_key: Option<String>,
}

pub struct SchemaField {
    /// Field name
    pub name: String,
    
    /// Data type
    pub field_type: FieldType,
    
    /// Whether field is required
    pub required: bool,
    
    /// Whether field can be faceted
    pub facet: bool,
    
    /// Whether field can be sorted
    pub sort: bool,
    
    /// Whether field is searchable
    pub index: bool,
}
```

#### Field Types

```rust
pub enum FieldType {
    Text,      // Full-text searchable
    Keyword,   // Exact-match only
    Integer,   // Numeric integer
    Float,     // Numeric decimal
    Boolean,   // True/false
    Date,      // ISO 8601 datetime
    GeoPoint,  // Geographic coordinates
}
```

## Core Functions

### Search Operations

#### search

Execute a search query against an index.

```rust
fn search(index: String, query: SearchQuery) -> Result<SearchResults, SearchError>
```

**Parameters:**
- `index`: Name of the search index
- `query`: Search query configuration

**Returns:**
- `SearchResults`: Complete search response
- `SearchError`: Error if search fails

**Example:**
```rust
let results = provider.search("products", SearchQuery {
    q: Some("laptop".to_string()),
    per_page: Some(10),
    ..Default::default()
}).await?;

println!("Found {} results in {}ms", 
    results.total_hits, results.took);
    
for hit in results.hits {
    println!("Document {}: score {:.2}", 
        hit.id, hit.score.unwrap_or(0.0));
}
```

### Document Management

#### upsert

Insert or update a single document.

```rust
fn upsert(index: String, doc: Doc) -> Result<(), SearchError>
```

**Parameters:**
- `index`: Target index name
- `doc`: Document to insert/update

**Example:**
```rust
let doc = Doc {
    id: "product_123".to_string(),
    content: serde_json::json!({
        "title": "Gaming Laptop",
        "price": 1299.99,
        "category": "electronics",
        "in_stock": true
    }).to_string(),
};

provider.upsert("products", doc).await?;
```

#### get

Retrieve a document by ID.

```rust
fn get(index: String, id: String) -> Result<Option<Doc>, SearchError>
```

**Example:**
```rust
if let Some(doc) = provider.get("products", "product_123".to_string()).await? {
    let product: serde_json::Value = serde_json::from_str(&doc.content)?;
    println!("Product: {}", product["title"]);
}
```

#### delete

Remove a document by ID.

```rust
fn delete(index: String, id: String) -> Result<(), SearchError>
```

### Batch Operations

#### batch-upsert

Insert or update multiple documents efficiently.

```rust
fn batch_upsert(index: String, docs: Vec<Doc>) -> Result<(), SearchError>
```

**Example:**
```rust
let docs = vec![
    Doc {
        id: "1".to_string(),
        content: json!({"title": "Product 1"}).to_string(),
    },
    Doc {
        id: "2".to_string(),
        content: json!({"title": "Product 2"}).to_string(),
    },
];

provider.batch_upsert("products", docs).await?;
```

### Index Management

#### create-index

Create a new search index with optional schema.

```rust
fn create_index(name: String, schema: Option<Schema>) -> Result<(), SearchError>
```

**Example:**
```rust
let schema = Schema {
    fields: vec![
        SchemaField {
            name: "title".to_string(),
            field_type: FieldType::Text,
            required: true,
            facet: false,
            sort: false,
            index: true,
        },
        SchemaField {
            name: "category".to_string(),
            field_type: FieldType::Keyword,
            required: false,
            facet: true,
            sort: true,
            index: true,
        },
    ],
    primary_key: Some("id".to_string()),
};

provider.create_index("products".to_string(), Some(schema)).await?;
```

#### delete-index

Remove an index and all its documents.

```rust
fn delete_index(name: String) -> Result<(), SearchError>
```

#### list-indexes

Get all available index names.

```rust
fn list_indexes() -> Result<Vec<String>, SearchError>
```

#### get-schema

Retrieve the schema for an index.

```rust
fn get_schema(index: String) -> Result<Schema, SearchError>
```

### Provider Information

#### get-capabilities

Get provider capability information.

```rust
fn get_capabilities() -> SearchCapabilities
```

**Returns:**
```rust
pub struct SearchCapabilities {
    pub provider_name: String,
    pub version: String,
    pub features: HashMap<String, bool>,
    pub limits: HashMap<String, u64>,
}
```

#### health-check

Verify provider connectivity and health.

```rust
fn health_check() -> Result<(), SearchError>
```

## Error Handling

### SearchError

All operations return a `SearchResult<T>` which is `Result<T, SearchError>`.

```rust
pub enum SearchError {
    /// Connection or network error
    Connection(String),
    
    /// Authentication or authorization error  
    Authentication(String),
    
    /// Invalid request or parameters
    InvalidRequest(String),
    
    /// Requested resource not found
    NotFound(String),
    
    /// Feature not supported by provider
    Unsupported(String),
    
    /// Rate limit exceeded
    RateLimit(String),
    
    /// Internal provider error
    Internal(String),
    
    /// Request timeout
    Timeout(String),
}
```

### Error Handling Patterns

```rust
use golem_search::SearchError;

match provider.search("products", query).await {
    Ok(results) => {
        // Handle successful search
        process_results(results);
    },
    Err(SearchError::NotFound(msg)) => {
        println!("Index not found: {}", msg);
        // Maybe create the index?
    },
    Err(SearchError::Unsupported(feature)) => {
        println!("Feature not supported: {}", feature);
        // Use fallback or alternative approach
    },
    Err(SearchError::RateLimit(_)) => {
        // Implement retry with backoff
        tokio::time::sleep(Duration::from_secs(1)).await;
        // Retry operation
    },
    Err(e) => {
        eprintln!("Search error: {:?}", e);
        return Err(e);
    }
}
```

## Configuration

### SearchConfig

Advanced search configuration options.

```rust
pub struct SearchConfig {
    /// Request timeout in milliseconds
    pub timeout_ms: Option<u32>,
    
    /// Field boost factors for relevance scoring
    pub boost_fields: Vec<(String, f64)>,
    
    /// Specific fields to retrieve (empty = all)
    pub attributes_to_retrieve: Vec<String>,
    
    /// Language for search operations
    pub language: Option<String>,
    
    /// Enable/disable typo tolerance
    pub typo_tolerance: Option<bool>,
    
    /// Boost factor for exact matches
    pub exact_match_boost: Option<f64>,
    
    /// Provider-specific parameters
    pub provider_params: Option<HashMap<String, serde_json::Value>>,
}
```

### HighlightConfig

Configuration for search result highlighting.

```rust
pub struct HighlightConfig {
    /// Fields to highlight
    pub fields: Vec<String>,
    
    /// HTML tag to wrap highlighted terms (default: "<em>")
    pub pre_tag: Option<String>,
    
    /// HTML tag to close highlighted terms (default: "</em>")  
    pub post_tag: Option<String>,
    
    /// Maximum length of highlighted snippets
    pub max_length: Option<u32>,
}
```

## Best Practices

### Performance Optimization

1. **Use appropriate page sizes**: 10-50 results per page for UI, larger for processing
2. **Limit facet fields**: Only request facets you'll display
3. **Use filters effectively**: Filters are faster than query matches
4. **Cache frequent queries**: Implement application-level caching

### Error Resilience

1. **Handle unsupported features**: Use capability checking before advanced features
2. **Implement retries**: Especially for rate limits and timeouts
3. **Graceful degradation**: Fall back to simpler queries when advanced features fail
4. **Monitor provider health**: Regular health checks for early problem detection

### Security Considerations

1. **Validate input**: Sanitize search queries and filters
2. **Limit query complexity**: Prevent expensive operations
3. **Use authentication**: Configure API keys and access controls
4. **Monitor usage**: Track query patterns for abuse detection

## Provider Compatibility

Different providers may have varying levels of support for features:

| Feature | Notes |
|---------|-------|
| **Complex Filters** | Syntax varies between providers |
| **Geo Queries** | Not all providers support geo search |
| **Vector Search** | Limited availability, different implementations |
| **Facet Limits** | Providers have different maximum facet counts |
| **Sort Options** | Some providers limit sortable fields |

Always check `get_capabilities()` and handle `SearchError::Unsupported` gracefully.

---

**Next Steps:**
- [Provider APIs](provider-apis.md) - Provider-specific features and limitations
- [Configuration Guide](configuration.md) - Environment setup and advanced configuration
- [Error Handling](error-handling.md) - Comprehensive error handling strategies