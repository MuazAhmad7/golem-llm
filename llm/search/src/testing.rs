//! Comprehensive testing framework for search providers
//! 
//! This module provides a unified testing framework that validates all search providers
//! against the golem:search interface specification, ensuring consistency and compliance.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use crate::types::{
    SearchQuery, SearchResults, SearchHit, Doc, Schema, SchemaField, FieldType,
    HighlightConfig, SearchConfig,
};
use crate::error::{SearchError, SearchResult};
use crate::capabilities::{CapabilityMatrix, FeatureSupport};
use log::{info, warn, debug};

/// Comprehensive test suite for search providers
pub struct SearchProviderTestSuite {
    provider_name: String,
    test_config: TestConfig,
    results: Vec<TestResult>,
}

/// Configuration for test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Test index name prefix
    pub index_prefix: String,
    
    /// Number of test documents to create
    pub document_count: usize,
    
    /// Whether to run performance benchmarks
    pub run_benchmarks: bool,
    
    /// Benchmark timeout in seconds
    pub benchmark_timeout_seconds: u64,
    
    /// Whether to test error conditions
    pub test_error_conditions: bool,
    
    /// Whether to test concurrent operations
    pub test_concurrency: bool,
    
    /// Maximum concurrent operations to test
    pub max_concurrent_ops: usize,
    
    /// Whether to cleanup test data after completion
    pub cleanup_after_tests: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            index_prefix: "test_golem_search".to_string(),
            document_count: 100,
            run_benchmarks: true,
            benchmark_timeout_seconds: 300, // 5 minutes
            test_error_conditions: true,
            test_concurrency: true,
            max_concurrent_ops: 10,
            cleanup_after_tests: true,
        }
    }
}

/// Result of running a test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_name: String,
    pub category: TestCategory,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub error_message: Option<String>,
    pub metrics: HashMap<String, f64>,
    pub assertions: Vec<AssertionResult>,
}

/// Categories of tests
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestCategory {
    /// Basic interface compliance
    InterfaceCompliance,
    
    /// Core functionality tests
    CoreFunctionality,
    
    /// Advanced feature tests
    AdvancedFeatures,
    
    /// Error handling tests
    ErrorHandling,
    
    /// Performance benchmarks
    Performance,
    
    /// Concurrency tests
    Concurrency,
    
    /// Memory and resource usage
    ResourceUsage,
}

/// Test execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Warning,
}

/// Individual assertion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    pub description: String,
    pub passed: bool,
    pub expected: String,
    pub actual: String,
}

/// Performance metrics collected during testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub operations_per_second: f64,
    pub average_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

/// Test data generator for creating consistent test datasets
pub struct TestDataGenerator {
    seed: u64,
}

impl TestDataGenerator {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }
    
    /// Generate test documents for a specific domain
    pub fn generate_documents(&self, count: usize, domain: TestDomain) -> Vec<Doc> {
        let mut documents = Vec::new();
        
        for i in 0..count {
            let doc = match domain {
                TestDomain::ECommerce => self.generate_ecommerce_document(i),
                TestDomain::News => self.generate_news_document(i),
                TestDomain::Academic => self.generate_academic_document(i),
                TestDomain::Technical => self.generate_technical_document(i),
            };
            documents.push(doc);
        }
        
        documents
    }
    
    fn generate_ecommerce_document(&self, id: usize) -> Doc {
        let categories = ["electronics", "clothing", "books", "home", "sports"];
        let brands = ["Apple", "Samsung", "Nike", "Adidas", "Sony"];
        
        let content = serde_json::json!({
            "id": format!("product_{}", id),
            "title": format!("Product {} - High Quality Item", id),
            "description": format!("This is a detailed description of product {}. It has many great features and excellent quality.", id),
            "category": categories[id % categories.len()],
            "brand": brands[id % brands.len()],
            "price": 19.99 + (id as f64 * 0.5),
            "rating": 3.0 + ((id % 10) as f64 / 5.0),
            "in_stock": id % 3 != 0,
            "tags": vec![format!("tag{}", id % 5), format!("feature{}", id % 3)],
            "created_at": "2024-01-01T00:00:00Z",
        });
        
        Doc {
            id: format!("product_{}", id),
            content: content.to_string(),
        }
    }
    
    fn generate_news_document(&self, id: usize) -> Doc {
        let categories = ["technology", "politics", "sports", "business", "entertainment"];
        
        let content = serde_json::json!({
            "id": format!("article_{}", id),
            "title": format!("Breaking News: Important Event {} Happens", id),
            "content": format!("This is the full content of news article {}. It contains detailed information about recent events.", id),
            "category": categories[id % categories.len()],
            "author": format!("Author {}", id % 10),
            "published_at": "2024-01-01T00:00:00Z",
            "views": id * 100,
            "likes": id * 5,
        });
        
        Doc {
            id: format!("article_{}", id),
            content: content.to_string(),
        }
    }
    
    fn generate_academic_document(&self, id: usize) -> Doc {
        let subjects = ["computer_science", "mathematics", "physics", "biology", "chemistry"];
        
        let content = serde_json::json!({
            "id": format!("paper_{}", id),
            "title": format!("Research Paper {}: A Comprehensive Study", id),
            "abstract": format!("This paper presents a comprehensive analysis of topic {}. Our findings show significant improvements over existing methods.", id),
            "subject": subjects[id % subjects.len()],
            "authors": vec![format!("Dr. Smith {}", id % 5), format!("Prof. Johnson {}", id % 3)],
            "published_year": 2020 + (id % 5),
            "citations": id * 2,
            "keywords": vec![format!("keyword{}", id % 8), format!("method{}", id % 4)],
        });
        
        Doc {
            id: format!("paper_{}", id),
            content: content.to_string(),
        }
    }
    
    fn generate_technical_document(&self, id: usize) -> Doc {
        let technologies = ["rust", "python", "javascript", "go", "java"];
        let complexity_levels = ["beginner", "intermediate", "advanced"];
        
        let content = serde_json::json!({
            "id": format!("doc_{}", id),
            "title": format!("Technical Documentation: Module {} API", id),
            "content": format!("This documentation describes the API for module {}. It includes detailed examples and usage instructions.", id),
            "technology": technologies[id % technologies.len()],
            "version": format!("v{}.{}.{}", id % 3 + 1, id % 10, id % 5),
            "complexity": complexity_levels[id % 3],
            "last_updated": "2024-01-01T00:00:00Z",
        });
        
        Doc {
            id: format!("doc_{}", id),
            content: content.to_string(),
        }
    }
    
    /// Generate test schema for a domain
    pub fn generate_schema(&self, domain: TestDomain) -> Schema {
        let fields = match domain {
            TestDomain::ECommerce => vec![
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
                SchemaField {
                    name: "price".to_string(),
                    field_type: FieldType::Float,
                    required: false,
                    facet: true,
                    sort: true,
                    index: true,
                },
                SchemaField {
                    name: "rating".to_string(),
                    field_type: FieldType::Float,
                    required: false,
                    facet: false,
                    sort: true,
                    index: true,
                },
                SchemaField {
                    name: "in_stock".to_string(),
                    field_type: FieldType::Boolean,
                    required: false,
                    facet: true,
                    sort: false,
                    index: true,
                },
            ],
            TestDomain::News => vec![
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
                SchemaField {
                    name: "author".to_string(),
                    field_type: FieldType::Keyword,
                    required: false,
                    facet: true,
                    sort: true,
                    index: true,
                },
                SchemaField {
                    name: "published_at".to_string(),
                    field_type: FieldType::Date,
                    required: false,
                    facet: false,
                    sort: true,
                    index: true,
                },
            ],
            TestDomain::Academic => vec![
                SchemaField {
                    name: "title".to_string(),
                    field_type: FieldType::Text,
                    required: true,
                    facet: false,
                    sort: false,
                    index: true,
                },
                SchemaField {
                    name: "subject".to_string(),
                    field_type: FieldType::Keyword,
                    required: false,
                    facet: true,
                    sort: true,
                    index: true,
                },
                SchemaField {
                    name: "published_year".to_string(),
                    field_type: FieldType::Integer,
                    required: false,
                    facet: true,
                    sort: true,
                    index: true,
                },
                SchemaField {
                    name: "citations".to_string(),
                    field_type: FieldType::Integer,
                    required: false,
                    facet: false,
                    sort: true,
                    index: true,
                },
            ],
            TestDomain::Technical => vec![
                SchemaField {
                    name: "title".to_string(),
                    field_type: FieldType::Text,
                    required: true,
                    facet: false,
                    sort: false,
                    index: true,
                },
                SchemaField {
                    name: "technology".to_string(),
                    field_type: FieldType::Keyword,
                    required: false,
                    facet: true,
                    sort: true,
                    index: true,
                },
                SchemaField {
                    name: "complexity".to_string(),
                    field_type: FieldType::Keyword,
                    required: false,
                    facet: true,
                    sort: true,
                    index: true,
                },
            ],
        };
        
        Schema {
            fields,
            primary_key: Some("id".to_string()),
        }
    }
}

/// Test domains for generating different types of data
#[derive(Debug, Clone, Copy)]
pub enum TestDomain {
    ECommerce,
    News,
    Academic,
    Technical,
}

/// Trait for implementing provider-specific test runners
pub trait ProviderTestRunner {
    /// Run the complete test suite against a provider
    async fn run_test_suite(&mut self, config: TestConfig) -> Vec<TestResult>;
    
    /// Test basic interface compliance
    async fn test_interface_compliance(&mut self) -> Vec<TestResult>;
    
    /// Test core functionality (CRUD operations)
    async fn test_core_functionality(&mut self) -> Vec<TestResult>;
    
    /// Test advanced features (faceting, highlighting, etc.)
    async fn test_advanced_features(&mut self) -> Vec<TestResult>;
    
    /// Test error handling and edge cases
    async fn test_error_handling(&mut self) -> Vec<TestResult>;
    
    /// Run performance benchmarks
    async fn test_performance(&mut self) -> Vec<TestResult>;
    
    /// Test concurrent operations
    async fn test_concurrency(&mut self) -> Vec<TestResult>;
    
    /// Test memory and resource usage
    async fn test_resource_usage(&mut self) -> Vec<TestResult>;
    
    /// Get provider capabilities for test planning
    fn get_provider_capabilities(&self) -> CapabilityMatrix;
    
    /// Cleanup test data
    async fn cleanup(&mut self) -> SearchResult<()>;
}

/// Universal test queries for consistency across providers
pub struct UniversalTestQueries;

impl UniversalTestQueries {
    /// Basic text search queries
    pub fn basic_text_queries() -> Vec<SearchQuery> {
        vec![
            SearchQuery {
                q: Some("test".to_string()),
                filters: vec![],
                sort: vec![],
                facets: vec![],
                page: None,
                per_page: Some(10),
                offset: None,
                highlight: None,
                config: None,
            },
            SearchQuery {
                q: Some("product quality".to_string()),
                filters: vec![],
                sort: vec![],
                facets: vec![],
                page: None,
                per_page: Some(20),
                offset: None,
                highlight: None,
                config: None,
            },
        ]
    }
    
    /// Faceted search queries
    pub fn faceted_queries() -> Vec<SearchQuery> {
        vec![
            SearchQuery {
                q: Some("*".to_string()),
                filters: vec![],
                sort: vec![],
                facets: vec!["category".to_string()],
                page: None,
                per_page: Some(10),
                offset: None,
                highlight: None,
                config: None,
            },
            SearchQuery {
                q: Some("electronics".to_string()),
                filters: vec!["category:electronics".to_string()],
                sort: vec![],
                facets: vec!["category".to_string(), "brand".to_string()],
                page: None,
                per_page: Some(10),
                offset: None,
                highlight: None,
                config: None,
            },
        ]
    }
    
    /// Highlighting queries
    pub fn highlighting_queries() -> Vec<SearchQuery> {
        vec![
            SearchQuery {
                q: Some("important".to_string()),
                filters: vec![],
                sort: vec![],
                facets: vec![],
                page: None,
                per_page: Some(10),
                offset: None,
                highlight: Some(HighlightConfig {
                    fields: vec!["title".to_string(), "description".to_string()],
                    pre_tag: Some("<mark>".to_string()),
                    post_tag: Some("</mark>".to_string()),
                    max_length: Some(200),
                }),
                config: None,
            },
        ]
    }
    
    /// Complex queries combining multiple features
    pub fn complex_queries() -> Vec<SearchQuery> {
        vec![
            SearchQuery {
                q: Some("quality product".to_string()),
                filters: vec!["price:[10 TO 100]".to_string(), "in_stock:true".to_string()],
                sort: vec!["price:asc".to_string(), "rating:desc".to_string()],
                facets: vec!["category".to_string(), "brand".to_string()],
                page: Some(0),
                per_page: Some(25),
                offset: None,
                highlight: Some(HighlightConfig {
                    fields: vec!["title".to_string()],
                    pre_tag: Some("<em>".to_string()),
                    post_tag: Some("</em>".to_string()),
                    max_length: Some(150),
                }),
                config: Some(SearchConfig {
                    timeout_ms: Some(5000),
                    boost_fields: vec![("title".to_string(), 2.0)],
                    attributes_to_retrieve: vec!["title".to_string(), "price".to_string()],
                    language: Some("en".to_string()),
                    typo_tolerance: Some(true),
                    exact_match_boost: Some(1.5),
                    provider_params: None,
                }),
            },
        ]
    }
    
    /// Edge case queries that might cause issues
    pub fn edge_case_queries() -> Vec<SearchQuery> {
        vec![
            // Empty query
            SearchQuery {
                q: Some("".to_string()),
                filters: vec![],
                sort: vec![],
                facets: vec![],
                page: None,
                per_page: Some(10),
                offset: None,
                highlight: None,
                config: None,
            },
            // Very long query
            SearchQuery {
                q: Some("a".repeat(1000)),
                filters: vec![],
                sort: vec![],
                facets: vec![],
                page: None,
                per_page: Some(10),
                offset: None,
                highlight: None,
                config: None,
            },
            // Large page size
            SearchQuery {
                q: Some("test".to_string()),
                filters: vec![],
                sort: vec![],
                facets: vec![],
                page: None,
                per_page: Some(10000),
                offset: None,
                highlight: None,
                config: None,
            },
            // Special characters
            SearchQuery {
                q: Some("test@#$%^&*()".to_string()),
                filters: vec![],
                sort: vec![],
                facets: vec![],
                page: None,
                per_page: Some(10),
                offset: None,
                highlight: None,
                config: None,
            },
        ]
    }
}

/// Test result analysis and reporting
pub struct TestReportGenerator;

impl TestReportGenerator {
    /// Generate a comprehensive test report
    pub fn generate_report(
        provider_name: &str,
        results: &[TestResult],
        capabilities: &CapabilityMatrix,
    ) -> TestReport {
        let total_tests = results.len();
        let passed_tests = results.iter().filter(|r| matches!(r.status, TestStatus::Passed)).count();
        let failed_tests = results.iter().filter(|r| matches!(r.status, TestStatus::Failed)).count();
        let skipped_tests = results.iter().filter(|r| matches!(r.status, TestStatus::Skipped)).count();
        
        let success_rate = if total_tests > 0 {
            (passed_tests as f64 / total_tests as f64) * 100.0
        } else {
            0.0
        };
        
        let avg_duration = if total_tests > 0 {
            results.iter().map(|r| r.duration_ms).sum::<u64>() as f64 / total_tests as f64
        } else {
            0.0
        };
        
        // Group results by category
        let mut category_results = HashMap::new();
        for result in results {
            category_results
                .entry(result.category.clone())
                .or_insert_with(Vec::new)
                .push(result.clone());
        }
        
        TestReport {
            provider_name: provider_name.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            total_tests,
            passed_tests,
            failed_tests,
            skipped_tests,
            success_rate,
            average_duration_ms: avg_duration,
            category_results,
            capabilities: capabilities.clone(),
            recommendations: Self::generate_recommendations(results, capabilities),
        }
    }
    
    fn generate_recommendations(
        results: &[TestResult],
        capabilities: &CapabilityMatrix,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Analyze failed tests
        let failed_results: Vec<_> = results.iter()
            .filter(|r| matches!(r.status, TestStatus::Failed))
            .collect();
        
        if !failed_results.is_empty() {
            recommendations.push(format!(
                "Address {} failed tests to improve provider reliability",
                failed_results.len()
            ));
        }
        
        // Check performance
        let slow_tests: Vec<_> = results.iter()
            .filter(|r| r.duration_ms > 1000) // > 1 second
            .collect();
        
        if !slow_tests.is_empty() {
            recommendations.push(format!(
                "Optimize performance for {} slow operations",
                slow_tests.len()
            ));
        }
        
        // Check capability gaps
        if capabilities.advanced_features.vector_search == FeatureSupport::Unsupported {
            recommendations.push("Consider adding vector search support for semantic similarity".to_string());
        }
        
        if capabilities.advanced_features.streaming_search == FeatureSupport::Unsupported {
            recommendations.push("Implement streaming search for large result sets".to_string());
        }
        
        recommendations
    }
}

/// Final test report structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestReport {
    pub provider_name: String,
    pub timestamp: String,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub success_rate: f64,
    pub average_duration_ms: f64,
    pub category_results: HashMap<TestCategory, Vec<TestResult>>,
    pub capabilities: CapabilityMatrix,
    pub recommendations: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_data_generator() {
        let generator = TestDataGenerator::new(42);
        let docs = generator.generate_documents(5, TestDomain::ECommerce);
        
        assert_eq!(docs.len(), 5);
        assert!(docs[0].id.starts_with("product_"));
        
        // Test that documents have consistent structure
        for doc in &docs {
            let content: serde_json::Value = serde_json::from_str(&doc.content).unwrap();
            assert!(content.get("title").is_some());
            assert!(content.get("category").is_some());
            assert!(content.get("price").is_some());
        }
    }
    
    #[test]
    fn test_schema_generation() {
        let generator = TestDataGenerator::new(42);
        let schema = generator.generate_schema(TestDomain::ECommerce);
        
        assert!(!schema.fields.is_empty());
        assert_eq!(schema.primary_key, Some("id".to_string()));
        
        // Check that we have expected fields
        let field_names: Vec<_> = schema.fields.iter().map(|f| &f.name).collect();
        assert!(field_names.contains(&&"title".to_string()));
        assert!(field_names.contains(&&"category".to_string()));
        assert!(field_names.contains(&&"price".to_string()));
    }
    
    #[test]
    fn test_universal_queries() {
        let basic_queries = UniversalTestQueries::basic_text_queries();
        assert!(!basic_queries.is_empty());
        
        let faceted_queries = UniversalTestQueries::faceted_queries();
        assert!(!faceted_queries.is_empty());
        assert!(faceted_queries.iter().any(|q| !q.facets.is_empty()));
        
        let highlighting_queries = UniversalTestQueries::highlighting_queries();
        assert!(!highlighting_queries.is_empty());
        assert!(highlighting_queries.iter().any(|q| q.highlight.is_some()));
    }
}