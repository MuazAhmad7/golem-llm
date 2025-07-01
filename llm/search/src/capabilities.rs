//! Capability matrix and graceful degradation system for search providers
//! 
//! This module provides:
//! - Comprehensive capability detection and reporting
//! - Graceful degradation strategies for unsupported features
//! - Feature compatibility checking and fallback mechanisms

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::types::{SearchQuery, FieldType};
use crate::error::{SearchResult};

/// Comprehensive capability matrix for all search providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMatrix {
    /// Provider name
    pub provider_name: String,
    
    /// Provider version
    pub provider_version: Option<String>,
    
    /// Core search capabilities
    pub core_capabilities: CoreCapabilities,
    
    /// Advanced search features
    pub advanced_features: AdvancedFeatures,
    
    /// Performance characteristics
    pub performance_limits: PerformanceLimits,
    
    /// Provider-specific features
    pub provider_specific: HashMap<String, FeatureSupport>,
}

/// Core search capabilities that most providers should support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreCapabilities {
    /// Basic text search
    pub full_text_search: FeatureSupport,
    
    /// Exact keyword matching
    pub keyword_search: FeatureSupport,
    
    /// Index management (create, delete, list)
    pub index_management: FeatureSupport,
    
    /// Document operations (CRUD)
    pub document_operations: FeatureSupport,
    
    /// Schema definition and management
    pub schema_management: FeatureSupport,
    
    /// Basic filtering
    pub filtering: FeatureSupport,
    
    /// Result pagination
    pub pagination: FeatureSupport,
}

/// Advanced search features that may not be universally supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedFeatures {
    /// Faceted search and aggregations
    pub faceted_search: FeatureSupport,
    
    /// Search result highlighting
    pub highlighting: FeatureSupport,
    
    /// Vector/semantic search
    pub vector_search: FeatureSupport,
    
    /// Geospatial search
    pub geo_search: FeatureSupport,
    
    /// Real-time streaming search
    pub streaming_search: FeatureSupport,
    
    /// Autocomplete and suggestions
    pub autocomplete: FeatureSupport,
    
    /// Typo tolerance and fuzzy matching
    pub typo_tolerance: FeatureSupport,
    
    /// Custom scoring and ranking
    pub custom_ranking: FeatureSupport,
    
    /// Multi-language support
    pub multilingual: FeatureSupport,
    
    /// Batch operations
    pub batch_operations: FeatureSupport,
}

/// Performance limits and characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceLimits {
    /// Maximum documents per batch operation
    pub max_batch_size: Option<u32>,
    
    /// Maximum query string length
    pub max_query_length: Option<u32>,
    
    /// Maximum number of facets per query
    pub max_facets: Option<u32>,
    
    /// Maximum number of filter conditions
    pub max_filters: Option<u32>,
    
    /// Maximum results per page
    pub max_results_per_page: Option<u32>,
    
    /// Default timeout for operations (seconds)
    pub default_timeout_seconds: Option<u32>,
    
    /// Rate limiting (requests per second)
    pub rate_limit_rps: Option<u32>,
}

/// Feature support levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureSupport {
    /// Feature is fully supported with native implementation
    Native,
    
    /// Feature is supported but may require workarounds or have limitations
    Limited,
    
    /// Feature is not supported by the provider
    Unsupported,
    
    /// Feature support depends on configuration or external plugins
    Conditional,
    
    /// Feature can be emulated through other means (client-side fallback)
    Emulated,
}

impl FeatureSupport {
    /// Check if the feature is available in any form
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Native | Self::Limited | Self::Conditional | Self::Emulated)
    }
    
    /// Check if the feature has native support
    pub fn is_native(&self) -> bool {
        matches!(self, Self::Native)
    }
    
    /// Check if the feature requires fallback mechanisms
    pub fn needs_fallback(&self) -> bool {
        matches!(self, Self::Limited | Self::Emulated)
    }
}

/// Degradation strategy for handling unsupported features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationStrategy {
    /// Strategy for handling unsupported faceted search
    pub facet_fallback: FacetFallback,
    
    /// Strategy for handling unsupported highlighting
    pub highlight_fallback: HighlightFallback,
    
    /// Strategy for handling unsupported streaming
    pub streaming_fallback: StreamingFallback,
    
    /// Strategy for handling unsupported vector search
    pub vector_search_fallback: VectorSearchFallback,
    
    /// Strategy for handling unsupported geo search
    pub geo_search_fallback: GeoSearchFallback,
    
    /// Whether to log warnings for unsupported features
    pub log_unsupported_warnings: bool,
    
    /// Whether to return errors for unsupported features or attempt fallbacks
    pub strict_mode: bool,
}

/// Faceted search fallback strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FacetFallback {
    /// Return empty facets
    Empty,
    
    /// Post-process search results to generate facets client-side
    ClientSide,
    
    /// Use separate aggregation queries
    SeparateQueries,
    
    /// Return error
    Error,
}

/// Highlighting fallback strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HighlightFallback {
    /// No highlighting
    None,
    
    /// Simple text matching client-side
    ClientSide,
    
    /// Return error
    Error,
}

/// Streaming search fallback strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamingFallback {
    /// Use pagination to simulate streaming
    Pagination,
    
    /// Return error
    Error,
}

/// Vector search fallback strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VectorSearchFallback {
    /// Fall back to text search
    TextSearch,
    
    /// Return error
    Error,
}

/// Geo search fallback strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoSearchFallback {
    /// Use bounding box filtering if available
    BoundingBox,
    
    /// Return error
    Error,
}

impl Default for DegradationStrategy {
    fn default() -> Self {
        Self {
            facet_fallback: FacetFallback::ClientSide,
            highlight_fallback: HighlightFallback::ClientSide,
            streaming_fallback: StreamingFallback::Pagination,
            vector_search_fallback: VectorSearchFallback::TextSearch,
            geo_search_fallback: GeoSearchFallback::BoundingBox,
            log_unsupported_warnings: true,
            strict_mode: false,
        }
    }
}

/// Capability checker for validating queries against provider capabilities
pub struct CapabilityChecker {
    matrix: CapabilityMatrix,
    strategy: DegradationStrategy,
}

impl CapabilityChecker {
    /// Create a new capability checker
    pub fn new(matrix: CapabilityMatrix, strategy: DegradationStrategy) -> Self {
        Self { matrix, strategy }
    }
    
    /// Check if a query is fully supported by the provider
    pub fn check_query_support(&self, query: &SearchQuery) -> QuerySupportResult {
        let mut issues = Vec::new();
        let mut requires_fallback = false;
        
        // Check faceting support
        if !query.facets.is_empty() {
            match self.matrix.advanced_features.faceted_search {
                FeatureSupport::Native => {},
                FeatureSupport::Limited => {
                    issues.push(CompatibilityIssue::LimitedSupport {
                        feature: "faceted_search".to_string(),
                        limitation: "May have performance or accuracy limitations".to_string(),
                    });
                },
                FeatureSupport::Unsupported => {
                    requires_fallback = true;
                    issues.push(CompatibilityIssue::UnsupportedFeature {
                        feature: "faceted_search".to_string(),
                        fallback: format!("{:?}", self.strategy.facet_fallback),
                    });
                },
                FeatureSupport::Emulated => {
                    requires_fallback = true;
                    issues.push(CompatibilityIssue::RequiresFallback {
                        feature: "faceted_search".to_string(),
                        method: "Client-side post-processing".to_string(),
                    });
                },
                FeatureSupport::Conditional => {
                    issues.push(CompatibilityIssue::ConditionalSupport {
                        feature: "faceted_search".to_string(),
                        condition: "Depends on index configuration".to_string(),
                    });
                },
            }
        }
        
        // Check highlighting support
        if query.highlight.is_some() {
            match self.matrix.advanced_features.highlighting {
                FeatureSupport::Native => {},
                FeatureSupport::Limited => {
                    issues.push(CompatibilityIssue::LimitedSupport {
                        feature: "highlighting".to_string(),
                        limitation: "May not support all highlight options".to_string(),
                    });
                },
                FeatureSupport::Unsupported => {
                    requires_fallback = true;
                    issues.push(CompatibilityIssue::UnsupportedFeature {
                        feature: "highlighting".to_string(),
                        fallback: format!("{:?}", self.strategy.highlight_fallback),
                    });
                },
                FeatureSupport::Emulated => {
                    requires_fallback = true;
                    issues.push(CompatibilityIssue::RequiresFallback {
                        feature: "highlighting".to_string(),
                        method: "Client-side text processing".to_string(),
                    });
                },
                FeatureSupport::Conditional => {
                    issues.push(CompatibilityIssue::ConditionalSupport {
                        feature: "highlighting".to_string(),
                        condition: "Depends on field configuration".to_string(),
                    });
                },
            }
        }
        
        // Check performance limits
        if let Some(per_page) = query.per_page {
            if let Some(max_per_page) = self.matrix.performance_limits.max_results_per_page {
                if per_page > max_per_page {
                    issues.push(CompatibilityIssue::PerformanceLimit {
                        parameter: "per_page".to_string(),
                        requested: per_page.to_string(),
                        limit: max_per_page.to_string(),
                    });
                }
            }
        }
        
        // Check query length
        if let Some(query_text) = &query.q {
            if let Some(max_length) = self.matrix.performance_limits.max_query_length {
                if query_text.len() > max_length as usize {
                    issues.push(CompatibilityIssue::PerformanceLimit {
                        parameter: "query_length".to_string(),
                        requested: query_text.len().to_string(),
                        limit: max_length.to_string(),
                    });
                }
            }
        }
        
        // Check filter count
        if !query.filters.is_empty() {
            if let Some(max_filters) = self.matrix.performance_limits.max_filters {
                if query.filters.len() > max_filters as usize {
                    issues.push(CompatibilityIssue::PerformanceLimit {
                        parameter: "filter_count".to_string(),
                        requested: query.filters.len().to_string(),
                        limit: max_filters.to_string(),
                    });
                }
            }
        }
        
        QuerySupportResult {
            is_fully_supported: issues.is_empty(),
            requires_fallback,
            issues,
        }
    }
    
    /// Get the capability matrix
    pub fn get_matrix(&self) -> &CapabilityMatrix {
        &self.matrix
    }
    
    /// Get the degradation strategy
    pub fn get_strategy(&self) -> &DegradationStrategy {
        &self.strategy
    }
}

/// Result of checking query support
#[derive(Debug, Clone)]
pub struct QuerySupportResult {
    /// Whether the query is fully supported without any issues
    pub is_fully_supported: bool,
    
    /// Whether the query requires fallback mechanisms
    pub requires_fallback: bool,
    
    /// List of compatibility issues found
    pub issues: Vec<CompatibilityIssue>,
}

/// Types of compatibility issues
#[derive(Debug, Clone)]
pub enum CompatibilityIssue {
    /// Feature is not supported at all
    UnsupportedFeature {
        feature: String,
        fallback: String,
    },
    
    /// Feature has limited support
    LimitedSupport {
        feature: String,
        limitation: String,
    },
    
    /// Feature requires fallback mechanism
    RequiresFallback {
        feature: String,
        method: String,
    },
    
    /// Feature support is conditional
    ConditionalSupport {
        feature: String,
        condition: String,
    },
    
    /// Performance or size limit exceeded
    PerformanceLimit {
        parameter: String,
        requested: String,
        limit: String,
    },
}

/// Trait for providers to declare their capabilities
pub trait ProviderCapabilities {
    /// Get the provider's capability matrix
    fn get_capability_matrix(&self) -> CapabilityMatrix;
    
    /// Check if a specific feature is supported
    fn supports_feature(&self, feature: &str) -> FeatureSupport;
    
    /// Get recommended degradation strategy for this provider
    fn get_degradation_strategy(&self) -> DegradationStrategy;
    
    /// Validate query against provider capabilities
    fn validate_query_compatibility(&self, query: &SearchQuery) -> QuerySupportResult {
        let matrix = self.get_capability_matrix();
        let strategy = self.get_degradation_strategy();
        let checker = CapabilityChecker::new(matrix, strategy);
        checker.check_query_support(query)
    }
}

// Provider-specific capability matrices

/// ElasticSearch capability matrix
pub fn elasticsearch_capability_matrix() -> CapabilityMatrix {
    CapabilityMatrix {
        provider_name: "elasticsearch".to_string(),
        provider_version: None,
        core_capabilities: CoreCapabilities {
            full_text_search: FeatureSupport::Native,
            keyword_search: FeatureSupport::Native,
            index_management: FeatureSupport::Native,
            document_operations: FeatureSupport::Native,
            schema_management: FeatureSupport::Native,
            filtering: FeatureSupport::Native,
            pagination: FeatureSupport::Native,
        },
        advanced_features: AdvancedFeatures {
            faceted_search: FeatureSupport::Native,
            highlighting: FeatureSupport::Native,
            vector_search: FeatureSupport::Conditional, // Requires plugins
            geo_search: FeatureSupport::Native,
            streaming_search: FeatureSupport::Native, // Via scroll API
            autocomplete: FeatureSupport::Native,
            typo_tolerance: FeatureSupport::Limited, // Via fuzzy queries
            custom_ranking: FeatureSupport::Native,
            multilingual: FeatureSupport::Native,
            batch_operations: FeatureSupport::Native,
        },
        performance_limits: PerformanceLimits {
            max_batch_size: Some(1000),
            max_query_length: Some(32768),
            max_facets: Some(100),
            max_filters: Some(256),
            max_results_per_page: Some(10000),
            default_timeout_seconds: Some(30),
            rate_limit_rps: None, // Depends on configuration
        },
        provider_specific: {
            let mut features = HashMap::new();
            features.insert("scroll_api".to_string(), FeatureSupport::Native);
            features.insert("percolator".to_string(), FeatureSupport::Native);
            features.insert("machine_learning".to_string(), FeatureSupport::Conditional);
            features.insert("security".to_string(), FeatureSupport::Conditional);
            features
        },
    }
}

/// OpenSearch capability matrix
pub fn opensearch_capability_matrix() -> CapabilityMatrix {
    let mut matrix = elasticsearch_capability_matrix();
    matrix.provider_name = "opensearch".to_string();
    
    // OpenSearch has better vector search support
    matrix.advanced_features.vector_search = FeatureSupport::Native;
    
    // Add OpenSearch-specific features
    matrix.provider_specific.insert("neural_search".to_string(), FeatureSupport::Native);
    matrix.provider_specific.insert("anomaly_detection".to_string(), FeatureSupport::Native);
    
    matrix
}

/// Typesense capability matrix
pub fn typesense_capability_matrix() -> CapabilityMatrix {
    CapabilityMatrix {
        provider_name: "typesense".to_string(),
        provider_version: None,
        core_capabilities: CoreCapabilities {
            full_text_search: FeatureSupport::Native,
            keyword_search: FeatureSupport::Native,
            index_management: FeatureSupport::Native,
            document_operations: FeatureSupport::Native,
            schema_management: FeatureSupport::Native,
            filtering: FeatureSupport::Native,
            pagination: FeatureSupport::Native,
        },
        advanced_features: AdvancedFeatures {
            faceted_search: FeatureSupport::Native,
            highlighting: FeatureSupport::Native,
            vector_search: FeatureSupport::Native,
            geo_search: FeatureSupport::Native,
            streaming_search: FeatureSupport::Unsupported, // No scroll API
            autocomplete: FeatureSupport::Native,
            typo_tolerance: FeatureSupport::Native, // Built-in
            custom_ranking: FeatureSupport::Native,
            multilingual: FeatureSupport::Limited,
            batch_operations: FeatureSupport::Limited, // Sequential only
        },
        performance_limits: PerformanceLimits {
            max_batch_size: Some(100), // Prefers smaller batches
            max_query_length: Some(2048),
            max_facets: Some(50),
            max_filters: Some(100),
            max_results_per_page: Some(250),
            default_timeout_seconds: Some(30),
            rate_limit_rps: None,
        },
        provider_specific: {
            let mut features = HashMap::new();
            features.insert("instant_search".to_string(), FeatureSupport::Native);
            features.insert("collection_aliases".to_string(), FeatureSupport::Native);
            features.insert("curation".to_string(), FeatureSupport::Native);
            features
        },
    }
}

/// Meilisearch capability matrix
pub fn meilisearch_capability_matrix() -> CapabilityMatrix {
    CapabilityMatrix {
        provider_name: "meilisearch".to_string(),
        provider_version: None,
        core_capabilities: CoreCapabilities {
            full_text_search: FeatureSupport::Native,
            keyword_search: FeatureSupport::Native,
            index_management: FeatureSupport::Native,
            document_operations: FeatureSupport::Native,
            schema_management: FeatureSupport::Native,
            filtering: FeatureSupport::Native,
            pagination: FeatureSupport::Native,
        },
        advanced_features: AdvancedFeatures {
            faceted_search: FeatureSupport::Native,
            highlighting: FeatureSupport::Native,
            vector_search: FeatureSupport::Limited, // Experimental
            geo_search: FeatureSupport::Native,
            streaming_search: FeatureSupport::Unsupported,
            autocomplete: FeatureSupport::Native,
            typo_tolerance: FeatureSupport::Native, // Excellent built-in
            custom_ranking: FeatureSupport::Native,
            multilingual: FeatureSupport::Native,
            batch_operations: FeatureSupport::Native,
        },
        performance_limits: PerformanceLimits {
            max_batch_size: Some(1000),
            max_query_length: Some(4096),
            max_facets: Some(100),
            max_filters: Some(200),
            max_results_per_page: Some(1000),
            default_timeout_seconds: Some(30),
            rate_limit_rps: None,
        },
        provider_specific: {
            let mut features = HashMap::new();
            features.insert("stop_words".to_string(), FeatureSupport::Native);
            features.insert("synonyms".to_string(), FeatureSupport::Native);
            features.insert("ranking_rules".to_string(), FeatureSupport::Native);
            features.insert("distinct".to_string(), FeatureSupport::Native);
            features
        },
    }
}

/// Algolia capability matrix
pub fn algolia_capability_matrix() -> CapabilityMatrix {
    CapabilityMatrix {
        provider_name: "algolia".to_string(),
        provider_version: None,
        core_capabilities: CoreCapabilities {
            full_text_search: FeatureSupport::Native,
            keyword_search: FeatureSupport::Native,
            index_management: FeatureSupport::Native,
            document_operations: FeatureSupport::Native,
            schema_management: FeatureSupport::Native,
            filtering: FeatureSupport::Native,
            pagination: FeatureSupport::Native,
        },
        advanced_features: AdvancedFeatures {
            faceted_search: FeatureSupport::Native,
            highlighting: FeatureSupport::Native,
            vector_search: FeatureSupport::Limited, // Via Recommend API
            geo_search: FeatureSupport::Native,
            streaming_search: FeatureSupport::Unsupported,
            autocomplete: FeatureSupport::Native,
            typo_tolerance: FeatureSupport::Native, // Industry-leading
            custom_ranking: FeatureSupport::Native,
            multilingual: FeatureSupport::Native,
            batch_operations: FeatureSupport::Native,
        },
        performance_limits: PerformanceLimits {
            max_batch_size: Some(1000),
            max_query_length: Some(512),
            max_facets: Some(100),
            max_filters: Some(100),
            max_results_per_page: Some(1000),
            default_timeout_seconds: Some(30),
            rate_limit_rps: Some(1000), // Depends on plan
        },
        provider_specific: {
            let mut features = HashMap::new();
            features.insert("analytics".to_string(), FeatureSupport::Native);
            features.insert("ab_testing".to_string(), FeatureSupport::Native);
            features.insert("personalization".to_string(), FeatureSupport::Native);
            features.insert("recommend".to_string(), FeatureSupport::Native);
            features
        },
    }
}