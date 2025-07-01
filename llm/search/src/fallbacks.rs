//! Fallback mechanisms for graceful degradation of unsupported search features
//! 
//! This module implements client-side fallbacks for features that may not be
//! natively supported by all search providers.

use std::collections::HashMap;
use serde_json::Value;
use crate::types::{SearchQuery, SearchResults, SearchHit};
use crate::error::{SearchError, SearchResult};
use crate::capabilities::{FeatureSupport, DegradationStrategy, FacetFallback, HighlightFallback};
use log::{warn, debug};

/// Fallback processor for handling unsupported features
pub struct FallbackProcessor {
    strategy: DegradationStrategy,
}

impl FallbackProcessor {
    /// Create a new fallback processor
    pub fn new(strategy: DegradationStrategy) -> Self {
        Self { strategy }
    }
    
    /// Process search results and apply fallbacks as needed
    pub fn process_search_results(
        &self,
        results: &mut SearchResults,
        original_query: &SearchQuery,
        supported_features: &HashMap<String, FeatureSupport>,
    ) -> SearchResult<()> {
        // Handle faceting fallback
        if !original_query.facets.is_empty() {
            let facet_support = supported_features
                .get("faceted_search")
                .copied()
                .unwrap_or(FeatureSupport::Unsupported);
            
            if facet_support == FeatureSupport::Unsupported || facet_support == FeatureSupport::Emulated {
                self.apply_facet_fallback(results, original_query)?;
            }
        }
        
        // Handle highlighting fallback
        if original_query.highlight.is_some() {
            let highlight_support = supported_features
                .get("highlighting")
                .copied()
                .unwrap_or(FeatureSupport::Unsupported);
            
            if highlight_support == FeatureSupport::Unsupported || highlight_support == FeatureSupport::Emulated {
                self.apply_highlight_fallback(results, original_query)?;
            }
        }
        
        // Apply any post-processing
        self.apply_post_processing(results, original_query)?;
        
        Ok(())
    }
    
    /// Apply faceting fallback when not natively supported
    fn apply_facet_fallback(&self, results: &mut SearchResults, query: &SearchQuery) -> SearchResult<()> {
        match self.strategy.facet_fallback {
            FacetFallback::Empty => {
                if self.strategy.log_unsupported_warnings {
                    warn!("Faceted search not supported by provider - returning empty facets");
                }
                results.facets = Some("{}".to_string());
            }
            
            FacetFallback::ClientSide => {
                if self.strategy.log_unsupported_warnings {
                    warn!("Faceted search not supported by provider - computing facets client-side");
                }
                let facets = self.compute_client_side_facets(&results.hits, &query.facets)?;
                results.facets = Some(serde_json::to_string(&facets)
                    .map_err(|e| SearchError::Internal(e.to_string()))?);
            }
            
            FacetFallback::SeparateQueries => {
                if self.strategy.log_unsupported_warnings {
                    warn!("Faceted search not supported by provider - would require separate queries (not implemented in fallback)");
                }
                results.facets = Some("{}".to_string());
            }
            
            FacetFallback::Error => {
                return Err(SearchError::Unsupported);
            }
        }
        
        Ok(())
    }
    
    /// Apply highlighting fallback when not natively supported
    fn apply_highlight_fallback(&self, results: &mut SearchResults, query: &SearchQuery) -> SearchResult<()> {
        match self.strategy.highlight_fallback {
            HighlightFallback::None => {
                if self.strategy.log_unsupported_warnings {
                    warn!("Highlighting not supported by provider - removing highlights");
                }
                for hit in &mut results.hits {
                    hit.highlights = None;
                }
            }
            
            HighlightFallback::ClientSide => {
                if self.strategy.log_unsupported_warnings {
                    warn!("Highlighting not supported by provider - applying client-side highlighting");
                }
                
                if let Some(highlight_config) = &query.highlight {
                    self.apply_client_side_highlighting(&mut results.hits, query, highlight_config)?;
                }
            }
            
            HighlightFallback::Error => {
                return Err(SearchError::Unsupported);
            }
        }
        
        Ok(())
    }
    
    /// Compute facets client-side from search results
    fn compute_client_side_facets(
        &self,
        hits: &[SearchHit],
        facet_fields: &[String],
    ) -> SearchResult<HashMap<String, HashMap<String, u32>>> {
        let mut facets = HashMap::new();
        
        for field_name in facet_fields {
            let mut field_facets = HashMap::new();
            
            for hit in hits {
                if let Some(content) = &hit.content {
                    if let Ok(doc) = serde_json::from_str::<Value>(content) {
                        if let Some(field_value) = doc.get(field_name) {
                            let value_str = match field_value {
                                Value::String(s) => s.clone(),
                                Value::Number(n) => n.to_string(),
                                Value::Bool(b) => b.to_string(),
                                Value::Array(arr) => {
                                    // Handle array fields by counting each element
                                    for item in arr {
                                        let item_str = match item {
                                            Value::String(s) => s.clone(),
                                            _ => item.to_string(),
                                        };
                                        *field_facets.entry(item_str).or_insert(0) += 1;
                                    }
                                    continue;
                                }
                                _ => field_value.to_string(),
                            };
                            
                            *field_facets.entry(value_str).or_insert(0) += 1;
                        }
                    }
                }
            }
            
            if !field_facets.is_empty() {
                facets.insert(field_name.clone(), field_facets);
            }
        }
        
        debug!("Computed client-side facets for {} fields", facets.len());
        Ok(facets)
    }
    
    /// Apply client-side highlighting to search results
    fn apply_client_side_highlighting(
        &self,
        hits: &mut [SearchHit],
        query: &SearchQuery,
        highlight_config: &crate::types::HighlightConfig,
    ) -> SearchResult<()> {
        let search_terms = self.extract_search_terms(query)?;
        let pre_tag = highlight_config.pre_tag.as_deref().unwrap_or("<mark>");
        let post_tag = highlight_config.post_tag.as_deref().unwrap_or("</mark>");
        let hits_len = hits.len();
        
        for hit in hits {
            if let Some(content) = &hit.content {
                if let Ok(doc) = serde_json::from_str::<Value>(content) {
                    let highlights = self.generate_highlights(
                        &doc,
                        &highlight_config.fields,
                        &search_terms,
                        pre_tag,
                        post_tag,
                        highlight_config.max_length,
                    )?;
                    
                    if !highlights.is_empty() {
                        hit.highlights = Some(serde_json::to_string(&highlights)
                            .map_err(|e| SearchError::Internal(e.to_string()))?);
                    }
                }
            }
        }
        
        debug!("Applied client-side highlighting to {} hits", hits_len);
        Ok(())
    }
    
    /// Extract search terms from query for highlighting
    fn extract_search_terms(&self, query: &SearchQuery) -> SearchResult<Vec<String>> {
        let mut terms = Vec::new();
        
        if let Some(q) = &query.q {
            // Simple term extraction - split on whitespace and remove punctuation
            for term in q.split_whitespace() {
                let clean_term = term
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
                    .to_lowercase();
                
                if !clean_term.is_empty() && clean_term.len() > 2 {
                    terms.push(clean_term);
                }
            }
        }
        
        Ok(terms)
    }
    
    /// Generate highlights for a document
    fn generate_highlights(
        &self,
        doc: &Value,
        highlight_fields: &[String],
        search_terms: &[String],
        pre_tag: &str,
        post_tag: &str,
        max_length: Option<u32>,
    ) -> SearchResult<HashMap<String, Vec<String>>> {
        let mut highlights = HashMap::new();
        
        for field_name in highlight_fields {
            if let Some(field_value) = doc.get(field_name) {
                if let Some(text) = field_value.as_str() {
                    let highlighted_snippets = self.highlight_text(
                        text,
                        search_terms,
                        pre_tag,
                        post_tag,
                        max_length,
                    );
                    
                    if !highlighted_snippets.is_empty() {
                        highlights.insert(field_name.clone(), highlighted_snippets);
                    }
                }
            }
        }
        
        Ok(highlights)
    }
    
    /// Highlight search terms in text
    fn highlight_text(
        &self,
        text: &str,
        search_terms: &[String],
        pre_tag: &str,
        post_tag: &str,
        max_length: Option<u32>,
    ) -> Vec<String> {
        let mut snippets = Vec::new();
        let text_lower = text.to_lowercase();
        
        for term in search_terms {
            if let Some(pos) = text_lower.find(term) {
                let snippet_start = pos.saturating_sub(50);
                let snippet_end = if let Some(max_len) = max_length {
                    std::cmp::min(pos + term.len() + 50, snippet_start + max_len as usize)
                } else {
                    pos + term.len() + 50
                };
                
                let snippet_end = std::cmp::min(snippet_end, text.len());
                
                if snippet_start < text.len() {
                    let mut snippet = text[snippet_start..snippet_end].to_string();
                    
                    // Apply highlighting to the term (case-insensitive)
                    let term_regex = regex::Regex::new(&format!(r"(?i)\b{}\b", regex::escape(term)))
                        .unwrap_or_else(|_| regex::Regex::new(term).unwrap());
                    
                    snippet = term_regex.replace_all(&snippet, |caps: &regex::Captures| {
                        format!("{}{}{}", pre_tag, &caps[0], post_tag)
                    }).to_string();
                    
                    snippets.push(snippet);
                }
            }
        }
        
        // Remove duplicates and limit to reasonable number
        snippets.sort_unstable();
        snippets.dedup();
        snippets.truncate(3);
        
        snippets
    }
    
    /// Apply any final post-processing to results
    fn apply_post_processing(&self, results: &mut SearchResults, _query: &SearchQuery) -> SearchResult<()> {
        // Ensure we have reasonable defaults for missing fields
        if results.total.is_none() {
            results.total = Some(results.hits.len() as u32);
        }
        
        if results.took_ms.is_none() {
            results.took_ms = Some(0); // Indicate processing was instant (fallback)
        }
        
        Ok(())
    }
}

/// Streaming search fallback using pagination
pub struct StreamingFallback {
    page_size: u32,
    max_pages: Option<u32>,
}

impl StreamingFallback {
    /// Create a new streaming fallback processor
    pub fn new(page_size: u32, max_pages: Option<u32>) -> Self {
        Self { page_size, max_pages }
    }
    
    /// Convert a streaming search request to paginated queries
    pub fn paginate_query(&self, query: &SearchQuery) -> Vec<SearchQuery> {
        let max_pages = self.max_pages.unwrap_or(10); // Default limit to prevent runaway queries
        let mut queries = Vec::new();
        
        for page in 0..max_pages {
            let mut paginated_query = query.clone();
            paginated_query.page = Some(page);
            paginated_query.per_page = Some(self.page_size);
            queries.push(paginated_query);
        }
        
        queries
    }
    
    /// Combine paginated results into a single result set
    pub fn combine_results(&self, page_results: Vec<SearchResults>) -> SearchResult<SearchResults> {
        if page_results.is_empty() {
            return Ok(SearchResults {
                total: Some(0),
                page: Some(0),
                per_page: Some(self.page_size),
                hits: Vec::new(),
                facets: None,
                took_ms: Some(0),
            });
        }
        
        let first_result = &page_results[0];
        let mut combined_hits = Vec::new();
        let mut total_time = 0;
        
        for result in &page_results {
            combined_hits.extend(result.hits.clone());
            if let Some(time) = result.took_ms {
                total_time += time;
            }
        }
        
        Ok(SearchResults {
            total: first_result.total,
            page: Some(0),
            per_page: Some(combined_hits.len() as u32),
            hits: combined_hits,
            facets: first_result.facets.clone(),
            took_ms: Some(total_time),
        })
    }
}

/// Feature detection utilities
pub struct FeatureDetector;

impl FeatureDetector {
    /// Detect if a query uses vector search features
    pub fn uses_vector_search(query: &SearchQuery) -> bool {
        // Check if query contains vector-specific parameters
        if let Some(config) = &query.config {
            if let Some(provider_params) = &config.provider_params {
                if let Ok(params) = serde_json::from_str::<Value>(provider_params) {
                    return params.get("vector").is_some()
                        || params.get("embedding").is_some()
                        || params.get("semantic").is_some();
                }
            }
        }
        false
    }
    
    /// Detect if a query uses geospatial search features
    pub fn uses_geo_search(query: &SearchQuery) -> bool {
        // Check for geo-related filters
        for filter in &query.filters {
            if filter.contains("geo_distance") 
                || filter.contains("geo_bounding_box")
                || filter.contains("latitude")
                || filter.contains("longitude") {
                return true;
            }
        }
        false
    }
    
    /// Detect if a query requires advanced aggregations
    pub fn uses_advanced_aggregations(query: &SearchQuery) -> bool {
        // This would need more sophisticated detection based on the query structure
        query.facets.len() > 5 || query.facets.iter().any(|f| f.contains("nested"))
    }
    
    /// Get estimated performance impact of using fallbacks
    pub fn estimate_fallback_performance_impact(
        query: &SearchQuery,
        unsupported_features: &[String],
    ) -> PerformanceImpact {
        let mut impact = PerformanceImpact::Low;
        
        for feature in unsupported_features {
            match feature.as_str() {
                "faceted_search" => {
                    if query.facets.len() > 10 {
                        impact = impact.max(PerformanceImpact::High);
                    } else if query.facets.len() > 3 {
                        impact = impact.max(PerformanceImpact::Medium);
                    } else {
                        impact = impact.max(PerformanceImpact::Low);
                    }
                }
                "highlighting" => {
                    impact = impact.max(PerformanceImpact::Low);
                }
                "streaming_search" => {
                    impact = impact.max(PerformanceImpact::Medium);
                }
                "vector_search" => {
                    impact = impact.max(PerformanceImpact::High);
                }
                _ => {
                    impact = impact.max(PerformanceImpact::Medium);
                }
            }
        }
        
        impact
    }
}

/// Performance impact levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PerformanceImpact {
    /// Minimal performance impact
    Low,
    /// Moderate performance impact
    Medium,
    /// Significant performance impact
    High,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HighlightConfig};
    
    #[test]
    fn test_client_side_facets() {
        let processor = FallbackProcessor::new(DegradationStrategy::default());
        
        let hits = vec![
            SearchHit {
                id: "1".to_string(),
                score: Some(1.0),
                content: Some(r#"{"category": "books", "price": 10}"#.to_string()),
                highlights: None,
            },
            SearchHit {
                id: "2".to_string(),
                score: Some(0.8),
                content: Some(r#"{"category": "books", "price": 15}"#.to_string()),
                highlights: None,
            },
            SearchHit {
                id: "3".to_string(),
                score: Some(0.6),
                content: Some(r#"{"category": "electronics", "price": 100}"#.to_string()),
                highlights: None,
            },
        ];
        
        let facets = processor.compute_client_side_facets(&hits, &["category".to_string()]).unwrap();
        
        assert_eq!(facets.len(), 1);
        assert_eq!(facets["category"]["books"], 2);
        assert_eq!(facets["category"]["electronics"], 1);
    }
    
    #[test]
    fn test_client_side_highlighting() {
        let processor = FallbackProcessor::new(DegradationStrategy::default());
        
        let terms = vec!["rust".to_string(), "programming".to_string()];
        let highlighted = processor.highlight_text(
            "Rust is a great programming language for systems programming",
            &terms,
            "<mark>",
            "</mark>",
            Some(100),
        );
        
        assert!(!highlighted.is_empty());
        
        // Check that we have highlighting for both terms (may be in different snippets)
        let all_highlighted = highlighted.join(" ");
        assert!(all_highlighted.contains("<mark>Rust</mark>"));
        assert!(all_highlighted.contains("<mark>programming</mark>"));
    }
    
    #[test]
    fn test_feature_detection() {
        let query = SearchQuery {
            q: Some("test query".to_string()),
            filters: vec!["geo_distance(location, 10km)".to_string()],
            sort: vec![],
            facets: vec!["category".to_string()],
            page: None,
            per_page: None,
            offset: None,
            highlight: None,
            config: None,
        };
        
        assert!(FeatureDetector::uses_geo_search(&query));
        assert!(!FeatureDetector::uses_vector_search(&query));
    }
}