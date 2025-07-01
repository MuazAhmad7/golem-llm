//! Graceful degradation integration for ElasticSearch provider
//! 
//! This module integrates the graceful degradation framework with the ElasticSearch
//! provider, implementing capability checking and fallback mechanisms.

use std::collections::HashMap;
use golem_search::{
    CapabilityMatrix, ProviderCapabilities, FeatureSupport, DegradationStrategy,
    FallbackProcessor, SearchQuery, SearchResults, SearchResult,
    capabilities::{
        elasticsearch_capability_matrix, QuerySupportResult, CapabilityChecker,
        FacetFallback, HighlightFallback, StreamingFallback, VectorSearchFallback, GeoSearchFallback,
    },
};
use log::{warn, debug, info};

/// ElasticSearch provider with graceful degradation support
pub struct ElasticSearchProviderWithDegradation {
    capability_matrix: CapabilityMatrix,
    degradation_strategy: DegradationStrategy,
    fallback_processor: FallbackProcessor,
}

impl ElasticSearchProviderWithDegradation {
    /// Create a new ElasticSearch provider with degradation support
    pub fn new() -> Self {
        let capability_matrix = elasticsearch_capability_matrix();
        let degradation_strategy = DegradationStrategy {
            facet_fallback: FacetFallback::ClientSide, // ElasticSearch supports facets natively, but fallback for errors
            highlight_fallback: HighlightFallback::ClientSide,
            streaming_fallback: StreamingFallback::Pagination,
            vector_search_fallback: VectorSearchFallback::TextSearch, // ElasticSearch needs plugins for vectors
            geo_search_fallback: GeoSearchFallback::BoundingBox,
            log_unsupported_warnings: true,
            strict_mode: false,
        };
        
        let fallback_processor = FallbackProcessor::new(degradation_strategy.clone());
        
        Self {
            capability_matrix,
            degradation_strategy,
            fallback_processor,
        }
    }
    
    /// Validate a query against ElasticSearch capabilities
    pub fn validate_query(&self, query: &SearchQuery) -> QuerySupportResult {
        let checker = CapabilityChecker::new(
            self.capability_matrix.clone(),
            self.degradation_strategy.clone(),
        );
        checker.check_query_support(query)
    }
    
    /// Process search results with fallback mechanisms
    pub fn process_search_results(
        &self,
        results: &mut SearchResults,
        original_query: &SearchQuery,
    ) -> SearchResult<()> {
        // Create supported features map based on ElasticSearch capabilities
        let mut supported_features = HashMap::new();
        
        // Map capability matrix to feature support map
        supported_features.insert("faceted_search".to_string(), self.capability_matrix.advanced_features.faceted_search);
        supported_features.insert("highlighting".to_string(), self.capability_matrix.advanced_features.highlighting);
        supported_features.insert("vector_search".to_string(), self.capability_matrix.advanced_features.vector_search);
        supported_features.insert("geo_search".to_string(), self.capability_matrix.advanced_features.geo_search);
        supported_features.insert("streaming_search".to_string(), self.capability_matrix.advanced_features.streaming_search);
        
        self.fallback_processor.process_search_results(results, original_query, &supported_features)
    }
    
    /// Check if vector search is available (requires plugins)
    pub fn check_vector_search_availability(&self) -> bool {
        // In a real implementation, this would check if vector search plugins are installed
        // For now, we assume they're not available by default
        false
    }
    
    /// Get feature-specific recommendations for ElasticSearch
    pub fn get_feature_recommendations(&self, query: &SearchQuery) -> Vec<String> {
        let mut recommendations = Vec::new();
        let support_result = self.validate_query(query);
        
        if !support_result.is_fully_supported {
            for issue in &support_result.issues {
                match issue {
                    golem_search::capabilities::CompatibilityIssue::UnsupportedFeature { feature, .. } => {
                        match feature.as_str() {
                            "vector_search" => {
                                recommendations.push(
                                    "Consider installing Elasticsearch vector search plugin for native vector support".to_string()
                                );
                            }
                            _ => {}
                        }
                    }
                    golem_search::capabilities::CompatibilityIssue::LimitedSupport { feature, limitation } => {
                        recommendations.push(format!("Feature '{}' has limitations: {}", feature, limitation));
                    }
                    golem_search::capabilities::CompatibilityIssue::PerformanceLimit { parameter, requested, limit } => {
                        recommendations.push(format!(
                            "Parameter '{}' requested value '{}' exceeds limit '{}'. Consider reducing the value.",
                            parameter, requested, limit
                        ));
                    }
                    _ => {}
                }
            }
        }
        
        recommendations
    }
    
    /// Log capability warnings for debugging
    pub fn log_capability_info(&self, query: &SearchQuery) {
        let support_result = self.validate_query(query);
        
        if !support_result.is_fully_supported {
            warn!("Query not fully supported by ElasticSearch. Issues found:");
            for issue in &support_result.issues {
                warn!("  - {:?}", issue);
            }
        }
        
        if support_result.requires_fallback {
            info!("Query requires fallback mechanisms for optimal results");
        }
        
        debug!("ElasticSearch capability matrix: {:?}", self.capability_matrix);
    }
}

impl ProviderCapabilities for ElasticSearchProviderWithDegradation {
    fn get_capability_matrix(&self) -> CapabilityMatrix {
        self.capability_matrix.clone()
    }
    
    fn supports_feature(&self, feature: &str) -> FeatureSupport {
        match feature {
            "full_text_search" => self.capability_matrix.core_capabilities.full_text_search,
            "keyword_search" => self.capability_matrix.core_capabilities.keyword_search,
            "index_management" => self.capability_matrix.core_capabilities.index_management,
            "document_operations" => self.capability_matrix.core_capabilities.document_operations,
            "schema_management" => self.capability_matrix.core_capabilities.schema_management,
            "filtering" => self.capability_matrix.core_capabilities.filtering,
            "pagination" => self.capability_matrix.core_capabilities.pagination,
            "faceted_search" => self.capability_matrix.advanced_features.faceted_search,
            "highlighting" => self.capability_matrix.advanced_features.highlighting,
            "vector_search" => {
                // Dynamic check for vector search availability
                if self.check_vector_search_availability() {
                    FeatureSupport::Native
                } else {
                    FeatureSupport::Conditional
                }
            }
            "geo_search" => self.capability_matrix.advanced_features.geo_search,
            "streaming_search" => self.capability_matrix.advanced_features.streaming_search,
            "autocomplete" => self.capability_matrix.advanced_features.autocomplete,
            "typo_tolerance" => self.capability_matrix.advanced_features.typo_tolerance,
            "custom_ranking" => self.capability_matrix.advanced_features.custom_ranking,
            "multilingual" => self.capability_matrix.advanced_features.multilingual,
            "batch_operations" => self.capability_matrix.advanced_features.batch_operations,
            _ => {
                // Check provider-specific features
                self.capability_matrix.provider_specific
                    .get(feature)
                    .copied()
                    .unwrap_or(FeatureSupport::Unsupported)
            }
        }
    }
    
    fn get_degradation_strategy(&self) -> DegradationStrategy {
        self.degradation_strategy.clone()
    }
}

/// Utility functions for ElasticSearch-specific degradation handling
pub mod elasticsearch_utils {
    use super::*;
    
    /// Check if an ElasticSearch cluster has vector search plugins installed
    pub async fn check_vector_plugins(client: &crate::client::ElasticClient) -> bool {
        // In a real implementation, this would call the _cat/plugins API
        // For now, we return false as a safe default
        false
    }
    
    /// Get ElasticSearch cluster information for capability detection
    pub async fn get_cluster_capabilities(client: &crate::client::ElasticClient) -> HashMap<String, bool> {
        let mut capabilities = HashMap::new();
        
        // In a real implementation, this would check cluster features
        capabilities.insert("vector_search".to_string(), false);
        capabilities.insert("machine_learning".to_string(), false);
        capabilities.insert("security".to_string(), false);
        
        capabilities
    }
    
    /// Suggest ElasticSearch configuration improvements based on query patterns
    pub fn suggest_configuration_improvements(query_patterns: &[SearchQuery]) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Analyze query patterns for optimization suggestions
        let has_facets = query_patterns.iter().any(|q| !q.facets.is_empty());
        let has_highlighting = query_patterns.iter().any(|q| q.highlight.is_some());
        let large_results = query_patterns.iter().any(|q| q.per_page.unwrap_or(10) > 100);
        
        if has_facets {
            suggestions.push("Consider enabling aggressive caching for faceted queries".to_string());
        }
        
        if has_highlighting {
            suggestions.push("Configure highlight field limits to improve performance".to_string());
        }
        
        if large_results {
            suggestions.push("Consider using scroll API for large result sets".to_string());
        }
        
        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use golem_search::types::{HighlightConfig};
    
    #[test]
    fn test_elasticsearch_capability_matrix() {
        let provider = ElasticSearchProviderWithDegradation::new();
        let matrix = provider.get_capability_matrix();
        
        assert_eq!(matrix.provider_name, "elasticsearch");
        assert_eq!(matrix.core_capabilities.full_text_search, FeatureSupport::Native);
        assert_eq!(matrix.advanced_features.faceted_search, FeatureSupport::Native);
        assert_eq!(matrix.advanced_features.vector_search, FeatureSupport::Conditional);
    }
    
    #[test]
    fn test_query_validation() {
        let provider = ElasticSearchProviderWithDegradation::new();
        
        let query = SearchQuery {
            q: Some("test query".to_string()),
            filters: vec![],
            sort: vec![],
            facets: vec!["category".to_string()],
            page: None,
            per_page: Some(50),
            offset: None,
            highlight: Some(HighlightConfig {
                fields: vec!["title".to_string()],
                pre_tag: Some("<mark>".to_string()),
                post_tag: Some("</mark>".to_string()),
                max_length: Some(200),
            }),
            config: None,
        };
        
        let result = provider.validate_query(&query);
        
        // This query should be fully supported by ElasticSearch
        assert!(result.is_fully_supported);
        assert!(!result.requires_fallback);
    }
    
    #[test]
    fn test_unsupported_feature_query() {
        let provider = ElasticSearchProviderWithDegradation::new();
        
        let query = SearchQuery {
            q: Some("test query".to_string()),
            filters: vec![],
            sort: vec![],
            facets: vec![],
            page: None,
            per_page: Some(20000), // Exceeds max_results_per_page limit
            offset: None,
            highlight: None,
            config: None,
        };
        
        let result = provider.validate_query(&query);
        
        // This query should have performance limit issues
        assert!(!result.is_fully_supported);
        assert!(!result.issues.is_empty());
    }
    
    #[test]
    fn test_feature_support_check() {
        let provider = ElasticSearchProviderWithDegradation::new();
        
        assert_eq!(provider.supports_feature("full_text_search"), FeatureSupport::Native);
        assert_eq!(provider.supports_feature("faceted_search"), FeatureSupport::Native);
        assert_eq!(provider.supports_feature("vector_search"), FeatureSupport::Conditional);
        assert_eq!(provider.supports_feature("nonexistent_feature"), FeatureSupport::Unsupported);
    }
}