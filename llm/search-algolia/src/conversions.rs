use std::collections::HashMap;
use anyhow::{anyhow, Result};
use serde_json::Value;
use uuid::Uuid;

use crate::bindings::*;
use crate::client::{AlgoliaIndexSettings, AlgoliaSearchQuery, AlgoliaSearchResults, AlgoliaSearchHit};

/// Convert WIT Schema to Algolia Index Settings
pub fn schema_to_index_settings(schema: &Schema) -> AlgoliaIndexSettings {
    let mut settings = AlgoliaIndexSettings::default();
    
    // Map searchable fields
    let searchable_attrs: Vec<String> = schema.fields
        .iter()
        .filter(|f| f.searchable)
        .map(|f| f.name.clone())
        .collect();
    
    if !searchable_attrs.is_empty() {
        settings.searchable_attributes = Some(searchable_attrs);
    }
    
    // Map facet fields
    let facet_attrs: Vec<String> = schema.fields
        .iter()
        .filter(|f| f.facetable)
        .map(|f| {
            // Use filterOnly for facets to prevent them from being searchable by default
            if f.searchable {
                f.name.clone()
            } else {
                format!("filterOnly({})", f.name)
            }
        })
        .collect();
    
    if !facet_attrs.is_empty() {
        settings.attributes_for_faceting = Some(facet_attrs);
    }
    
    // Map unretrievable fields (opposite of retrievable)
    let unretrievable_attrs: Vec<String> = schema.fields
        .iter()
        .filter(|f| !f.retrievable)
        .map(|f| f.name.clone())
        .collect();
    
    if !unretrievable_attrs.is_empty() {
        settings.unretrievable_attributes = Some(unretrievable_attrs);
    }
    
    // Parse provider-specific parameters
    if let Some(provider_params) = &schema.provider_params {
        if let Ok(params) = serde_json::from_str::<HashMap<String, Value>>(provider_params) {
            // Handle typo tolerance
            if let Some(typo_tolerance) = params.get("typoTolerance") {
                settings.typo_tolerance = Some(typo_tolerance.clone());
            }
            
            // Handle advanced typo tolerance settings
            if let Some(min_1_typo) = params.get("minWordSizefor1Typo") {
                if let Some(size) = min_1_typo.as_u64() {
                    settings.min_word_size_for_1_typo = Some(size as u32);
                }
            }
            
            if let Some(min_2_typos) = params.get("minWordSizefor2Typos") {
                if let Some(size) = min_2_typos.as_u64() {
                    settings.min_word_size_for_2_typos = Some(size as u32);
                }
            }
            
            if let Some(typo_min) = params.get("typoToleranceMin") {
                if let Some(enabled) = typo_min.as_bool() {
                    settings.typo_tolerance_min = Some(enabled);
                }
            }
            
            if let Some(typo_strict) = params.get("typoToleranceStrict") {
                if let Some(enabled) = typo_strict.as_bool() {
                    settings.typo_tolerance_strict = Some(enabled);
                }
            }
            
            // Language-specific settings
            if let Some(remove_stop_words) = params.get("removeStopWords") {
                settings.remove_stop_words = Some(remove_stop_words.clone());
            }
            
            if let Some(ignore_plurals) = params.get("ignorePlurals") {
                settings.ignore_plurals = Some(ignore_plurals.clone());
            }
            
            if let Some(query_languages) = params.get("queryLanguages") {
                if let Some(lang_array) = query_languages.as_array() {
                    let languages: Vec<String> = lang_array
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    if !languages.is_empty() {
                        settings.query_languages = Some(languages);
                    }
                }
            }
            
            if let Some(index_languages) = params.get("indexLanguages") {
                if let Some(lang_array) = index_languages.as_array() {
                    let languages: Vec<String> = lang_array
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    if !languages.is_empty() {
                        settings.index_languages = Some(languages);
                    }
                }
            }
            
            // Synonyms and stop words
            if let Some(synonyms) = params.get("synonyms") {
                if let Some(synonyms_array) = synonyms.as_array() {
                    let synonyms_config: Vec<HashMap<String, Value>> = synonyms_array
                        .iter()
                        .filter_map(|v| {
                            if let Some(obj) = v.as_object() {
                                Some(obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                            } else {
                                None
                            }
                        })
                        .collect();
                    if !synonyms_config.is_empty() {
                        settings.synonyms = Some(synonyms_config);
                    }
                }
            }
            
            if let Some(stop_words) = params.get("stopWords") {
                if let Some(words_array) = stop_words.as_array() {
                    let words: Vec<String> = words_array
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    if !words.is_empty() {
                        settings.stop_words = Some(words);
                    }
                }
            }
            
            // Handle custom ranking
            if let Some(custom_ranking) = params.get("customRanking") {
                if let Some(ranking_array) = custom_ranking.as_array() {
                    let custom_ranking_strs: Vec<String> = ranking_array
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    if !custom_ranking_strs.is_empty() {
                        settings.custom_ranking = Some(custom_ranking_strs);
                    }
                }
            }
            
            // Relevance and ranking settings
            if let Some(distinct) = params.get("distinct") {
                settings.distinct = Some(distinct.clone());
            }
            
            if let Some(min_proximity) = params.get("minProximity") {
                if let Some(proximity) = min_proximity.as_u64() {
                    settings.min_proximity = Some(proximity as u32);
                }
            }
            
            if let Some(separators) = params.get("separatorsToIndex") {
                if let Some(sep_str) = separators.as_str() {
                    settings.separators_to_index = Some(sep_str.to_string());
                }
            }
            
            // Handle highlight tags
            if let Some(pre_tag) = params.get("highlightPreTag") {
                if let Some(tag_str) = pre_tag.as_str() {
                    settings.highlight_pre_tag = Some(tag_str.to_string());
                }
            }
            
            if let Some(post_tag) = params.get("highlightPostTag") {
                if let Some(tag_str) = post_tag.as_str() {
                    settings.highlight_post_tag = Some(tag_str.to_string());
                }
            }
        }
    }
    
    settings
}

/// Convert WIT SearchQuery to Algolia query parameters
pub fn search_query_to_algolia_query(query: &SearchQuery) -> Result<AlgoliaSearchQuery> {
    let mut algolia_query = AlgoliaSearchQuery {
        query: query.query.clone(),
        filters: None,
        facets: None,
        page: query.page,
        hits_per_page: query.per_page,
        highlight_pre_tag: None,
        highlight_post_tag: None,
        attributes_to_retrieve: None,
        sort: None,
        // Initialize advanced features
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
    
    // Convert facet filters to Algolia facet filters (more sophisticated approach)
    if !query.facet_filters.is_empty() {
        // Group facet filters by field for more complex boolean logic
        let mut facet_groups: HashMap<String, Vec<String>> = HashMap::new();
        let mut all_facet_fields = std::collections::HashSet::new();
        
        for filter in &query.facet_filters {
            facet_groups
                .entry(filter.field.clone())
                .or_insert_with(Vec::new)
                .push(format!("{}:{}", filter.field, filter.value));
            all_facet_fields.insert(filter.field.clone());
        }
        
        // Create complex facet filter structure
        if facet_groups.len() == 1 {
            // Simple case: only one field being filtered
            let filters: Vec<String> = facet_groups.into_values().next().unwrap();
            algolia_query.facet_filters = Some(Value::Array(
                filters.into_iter().map(Value::String).collect()
            ));
        } else {
            // Complex case: multiple fields being filtered (AND logic between fields, OR within fields)
            let filter_arrays: Vec<Value> = facet_groups
                .into_values()
                .map(|filters| {
                    if filters.len() == 1 {
                        Value::String(filters[0].clone())
                    } else {
                        Value::Array(filters.into_iter().map(Value::String).collect())
                    }
                })
                .collect();
            algolia_query.facet_filters = Some(Value::Array(filter_arrays));
        }
        
        // Set facets for faceted search
        if !all_facet_fields.is_empty() {
            algolia_query.facets = Some(all_facet_fields.into_iter().collect());
        }
    }
    
    // Convert sort options (support multiple sort criteria)
    if let (Some(sort_by), Some(sort_order)) = (&query.sort_by, &query.sort_order) {
        // Handle multi-attribute sorting if sort_by contains comma-separated fields
        let sort_fields: Vec<&str> = sort_by.split(',').collect();
        let sort_orders: Vec<&str> = sort_order.split(',').collect();
        
        let sort_strings: Vec<String> = sort_fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let order = sort_orders.get(i).unwrap_or(&"asc");
                if order == &"desc" {
                    format!("desc({})", field.trim())
                } else {
                    format!("asc({})", field.trim())
                }
            })
            .collect();
        
        algolia_query.sort = Some(sort_strings);
    }
    
    // Enable advanced features by default for better search experience
    algolia_query.get_ranking_info = Some(true);
    algolia_query.analytics = Some(true);
    algolia_query.synonyms = Some(true);
    
    Ok(algolia_query)
}

/// Convert Algolia search results to WIT SearchResults
pub fn algolia_results_to_search_results(results: AlgoliaSearchResults) -> Result<SearchResults> {
    let hits: Result<Vec<SearchHit>> = results.hits
        .into_iter()
        .map(algolia_hit_to_search_hit)
        .collect();
    
    let hits = hits?;
    
    // Convert facets
    let facets = if let Some(algolia_facets) = results.facets {
        let facet_results: Vec<FacetResult> = algolia_facets
            .into_iter()
            .map(|(field, values)| {
                let facet_values: Vec<FacetValue> = values
                    .into_iter()
                    .map(|(value, count)| FacetValue { value, count })
                    .collect();
                
                FacetResult {
                    field,
                    values: facet_values,
                }
            })
            .collect();
        
        Some(facet_results)
    } else {
        None
    };
    
    Ok(SearchResults {
        hits,
        total_hits: results.nb_hits,
        page: results.page,
        per_page: results.hits_per_page,
        facets: facets.unwrap_or_default(),
        processing_time_ms: Some(results.processing_time_ms),
    })
}

/// Convert Algolia search hit to WIT SearchHit
fn algolia_hit_to_search_hit(hit: AlgoliaSearchHit) -> Result<SearchHit> {
    // Extract the data without the objectID and other Algolia-specific fields
    let mut data = hit.data;
    
    // Remove Algolia-specific fields that shouldn't be in the user data
    if let Some(obj) = data.as_object_mut() {
        obj.remove("objectID");
        obj.remove("_highlightResult");
        obj.remove("_rankingInfo");
        obj.remove("_snippetResult");
    }
    
    let data_str = serde_json::to_string(&data)
        .map_err(|e| anyhow!("Failed to serialize hit data: {}", e))?;
    
    // Enhanced highlighting information extraction
    let highlighted = if let Some(highlight_result) = hit.highlight_result {
        // Process highlighting to create a more comprehensive highlight structure
        let mut highlight_data = serde_json::Map::new();
        
        if let Some(highlight_obj) = highlight_result.as_object() {
            for (field, highlight_info) in highlight_obj {
                if let Some(highlight_detail) = highlight_info.as_object() {
                    // Extract the highlighted value if available
                    if let Some(value) = highlight_detail.get("value") {
                        highlight_data.insert(field.clone(), value.clone());
                    }
                    // Also include match level and other metadata
                    if let Some(match_level) = highlight_detail.get("matchLevel") {
                        highlight_data.insert(
                            format!("{}_matchLevel", field), 
                            match_level.clone()
                        );
                    }
                    if let Some(matched_words) = highlight_detail.get("matchedWords") {
                        highlight_data.insert(
                            format!("{}_matchedWords", field), 
                            matched_words.clone()
                        );
                    }
                }
            }
        }
        
        Some(serde_json::to_string(&highlight_data)
            .map_err(|e| anyhow!("Failed to serialize enhanced highlight result: {}", e))?)
    } else {
        None
    };
    
    // Enhanced ranking score extraction
    let score = if let Some(ranking_info) = &hit.ranking_info {
        // Try to get the most relevant score
        ranking_info.get("score").and_then(|s| s.as_f64())
            .or_else(|| ranking_info.get("userScore").and_then(|s| s.as_f64()))
            .or_else(|| ranking_info.get("geoDistance").and_then(|s| s.as_f64()))
            .or_else(|| {
                // Calculate a combined score from various ranking factors
                let typo_score = ranking_info.get("typoScore").and_then(|s| s.as_f64()).unwrap_or(1.0);
                let geo_score = ranking_info.get("geoScore").and_then(|s| s.as_f64()).unwrap_or(1.0);
                let words_score = ranking_info.get("wordsScore").and_then(|s| s.as_f64()).unwrap_or(1.0);
                let filters_score = ranking_info.get("filtersScore").and_then(|s| s.as_f64()).unwrap_or(1.0);
                
                Some(typo_score * geo_score * words_score * filters_score)
            })
    } else {
        None
    };
    
    Ok(SearchHit {
        id: hit.object_id,
        data: data_str,
        score: score.map(|s| s as f32),
        highlights: highlighted,
    })
}

/// Convert WIT Document to Algolia object with proper ID handling
pub fn document_to_algolia_object(document: &Document) -> Result<(String, Value)> {
    // Parse the document data
    let mut data: Value = serde_json::from_str(&document.data)
        .map_err(|e| anyhow!("Failed to parse document data: {}", e))?;
    
    // Generate or extract the object ID
    let object_id = if let Some(id) = &document.id {
        id.clone()
    } else {
        // Generate a new UUID if no ID is provided
        Uuid::new_v4().to_string()
    };
    
    // Ensure the objectID is set in the document data
    if let Some(obj) = data.as_object_mut() {
        obj.insert("objectID".to_string(), Value::String(object_id.clone()));
    }
    
    Ok((object_id, data))
}

/// Convert Algolia object back to WIT Document
pub fn algolia_object_to_document(object_id: String, mut data: Value) -> Result<Document> {
    // Remove the objectID from the data since it's handled separately
    if let Some(obj) = data.as_object_mut() {
        obj.remove("objectID");
    }
    
    let data_str = serde_json::to_string(&data)
        .map_err(|e| anyhow!("Failed to serialize document data: {}", e))?;
    
    Ok(Document {
        id: Some(object_id),
        data: data_str,
    })
}

/// Create complex boolean filter expression for Algolia
pub fn create_complex_filter(filters: &[(&str, &str)], operator: &str) -> String {
    let filter_strings: Vec<String> = filters
        .iter()
        .map(|(field, value)| format!("{}:{}", field, value))
        .collect();
    
    match operator.to_lowercase().as_str() {
        "and" => filter_strings.join(" AND "),
        "or" => filter_strings.join(" OR "),
        _ => filter_strings.join(" AND "), // Default to AND
    }
}

/// Create advanced facet filter configuration with nested boolean logic
pub fn create_advanced_facet_filters(
    filters: &HashMap<String, Vec<String>>,
    inter_field_logic: &str, // "and" or "or" between different fields
    intra_field_logic: &str  // "and" or "or" within same field values
) -> Value {
    if filters.is_empty() {
        return Value::Null;
    }
    
    let filter_groups: Vec<Value> = filters
        .iter()
        .map(|(field, values)| {
            if values.len() == 1 {
                Value::String(format!("{}:{}", field, values[0]))
            } else {
                // Multiple values for the same field
                let field_filters: Vec<Value> = values
                    .iter()
                    .map(|value| Value::String(format!("{}:{}", field, value)))
                    .collect();
                
                if intra_field_logic == "and" {
                    // Each value should be in a separate array for AND logic
                    Value::Array(field_filters.into_iter().map(|f| Value::Array(vec![f])).collect())
                } else {
                    // All values in the same array for OR logic
                    Value::Array(field_filters)
                }
            }
        })
        .collect();
    
    if inter_field_logic == "or" && filter_groups.len() > 1 {
        // OR logic between fields - flatten into a single array
        let mut flattened = Vec::new();
        for group in filter_groups {
            match group {
                Value::Array(arr) => flattened.extend(arr),
                other => flattened.push(other),
            }
        }
        Value::Array(flattened)
    } else {
        // AND logic between fields (default)
        Value::Array(filter_groups)
    }
}

/// Configure advanced highlighting options
pub fn configure_advanced_highlighting(
    query: &mut AlgoliaSearchQuery,
    highlight_fields: Option<&[String]>,
    snippet_fields: Option<&[String]>,
    pre_tag: Option<&str>,
    post_tag: Option<&str>,
    restrict_arrays: bool
) {
    if let Some(fields) = highlight_fields {
        query.attributes_to_highlight = Some(fields.to_vec());
    }
    
    if let Some(fields) = snippet_fields {
        query.attributes_to_snippet = Some(fields.to_vec());
    }
    
    if let Some(tag) = pre_tag {
        query.highlight_pre_tag_override = Some(tag.to_string());
    }
    
    if let Some(tag) = post_tag {
        query.highlight_post_tag_override = Some(tag.to_string());
    }
    
    query.restrict_highlight_and_snippet_arrays = Some(restrict_arrays);
}

/// Configure custom ranking and multi-attribute sorting
pub fn configure_custom_ranking(
    query: &mut AlgoliaSearchQuery,
    sort_attributes: &[&str],
    custom_ranking_formula: Option<&str>,
    distinct_attribute: Option<&str>,
    typo_tolerance: Option<&str>
) {
    // Multi-attribute sorting
    if !sort_attributes.is_empty() {
        let sort_strings: Vec<String> = sort_attributes
            .iter()
            .map(|attr| {
                if attr.starts_with("desc(") || attr.starts_with("asc(") {
                    attr.to_string()
                } else {
                    format!("asc({})", attr)
                }
            })
            .collect();
        query.sort = Some(sort_strings);
    }
    
    // Set distinct attribute
    if let Some(distinct) = distinct_attribute {
        query.distinct = Some(Value::String(distinct.to_string()));
    }
    
    // Set typo tolerance
    if let Some(tolerance) = typo_tolerance {
        query.typo_tolerance = Some(tolerance.to_string());
    }
    
    // Enable ranking info for debugging
    query.get_ranking_info = Some(true);
}

/// Configure attribute retrieval control
pub fn configure_attribute_retrieval(
    query: &mut AlgoliaSearchQuery,
    attributes_to_retrieve: Option<&[String]>,
    restrict_sources: bool
) {
    if let Some(attributes) = attributes_to_retrieve {
        query.attributes_to_retrieve = Some(attributes.to_vec());
    }
    
    // Disable analytics for performance if restricting sources
    if restrict_sources {
        query.analytics = Some(false);
    }
}

/// Apply provider-specific query parameters for advanced features
pub fn apply_provider_query_params(
    query: &mut AlgoliaSearchQuery,
    provider_params: Option<&str>
) -> Result<()> {
    if let Some(params_str) = provider_params {
        if let Ok(params) = serde_json::from_str::<HashMap<String, Value>>(params_str) {
            // Advanced filter configuration
            if let Some(numeric_filters) = params.get("numericFilters") {
                if let Some(filters_array) = numeric_filters.as_array() {
                    let numeric_filter_strings: Vec<String> = filters_array
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    if !numeric_filter_strings.is_empty() {
                        query.numeric_filters = Some(numeric_filter_strings);
                    }
                }
            }
            
            // Tag filters
            if let Some(tag_filters) = params.get("tagFilters") {
                query.tag_filters = Some(tag_filters.clone());
            }
            
            // Query-level typo tolerance
            if let Some(typo_tolerance) = params.get("typoTolerance") {
                if let Some(tolerance_str) = typo_tolerance.as_str() {
                    query.typo_tolerance = Some(tolerance_str.to_string());
                }
            }
            
            // Synonyms configuration
            if let Some(synonyms) = params.get("synonyms") {
                if let Some(enabled) = synonyms.as_bool() {
                    query.synonyms = Some(enabled);
                }
            }
            
            // Replace synonyms in highlight
            if let Some(replace_synonyms) = params.get("replaceSynonymsInHighlight") {
                if let Some(enabled) = replace_synonyms.as_bool() {
                    query.replaceSynonymsInHighlight = Some(enabled);
                }
            }
            
            // Minimum proximity
            if let Some(min_proximity) = params.get("minProximity") {
                if let Some(proximity) = min_proximity.as_u64() {
                    query.minProximity = Some(proximity as u32);
                }
            }
            
            // Distinct configuration
            if let Some(distinct) = params.get("distinct") {
                query.distinct = Some(distinct.clone());
            }
        }
    }
    
    Ok(())
}

/// Map Algolia API errors to WIT error types
pub fn map_algolia_error(error: anyhow::Error) -> Error {
    let error_message = error.to_string();
    
    // Analyze the error message to determine the appropriate error code
    let (code, message) = if error_message.contains("404") || error_message.contains("not found") {
        (ErrorCode::InternalError, "Resource not found".to_string())
    } else if error_message.contains("401") || error_message.contains("403") || error_message.contains("authentication") {
        (ErrorCode::AuthenticationFailed, "Authentication failed".to_string())
    } else if error_message.contains("429") || error_message.contains("rate limit") {
        (ErrorCode::RateLimitExceeded, "Rate limit exceeded".to_string())
    } else if error_message.contains("400") || error_message.contains("invalid") {
        (ErrorCode::InvalidRequest, "Invalid query or request".to_string())
    } else if error_message.contains("unsupported") {
        (ErrorCode::Unsupported, "Operation not supported".to_string())
    } else {
        (ErrorCode::InternalError, format!("Internal error: {}", error_message))
    };
    
    Error {
        code,
        message,
        retry_after: if matches!(code, ErrorCode::RateLimitExceeded) { 
            Some(60) // Suggest retrying after 60 seconds for rate limits
        } else { 
            None 
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::{FieldType, FieldDefinition};

    #[test]
    fn test_schema_to_index_settings() {
        let schema = Schema {
            primary_key: "id".to_string(),
            fields: vec![
                FieldDefinition {
                    name: "title".to_string(),
                    field_type: FieldType::Text,
                    searchable: true,
                    facetable: false,
                    retrievable: true,
                    sortable: false,
                },
                FieldDefinition {
                    name: "category".to_string(),
                    field_type: FieldType::Text,
                    searchable: false,
                    facetable: true,
                    retrievable: true,
                    sortable: false,
                },
            ],
            provider_params: Some(r#"{"typoTolerance": true}"#.to_string()),
        };

        let settings = schema_to_index_settings(&schema);
        
        assert_eq!(settings.searchable_attributes, Some(vec!["title".to_string()]));
        assert_eq!(settings.attributes_for_faceting, Some(vec!["filterOnly(category)".to_string()]));
        assert!(settings.typo_tolerance.is_some());
    }

    #[test]
    fn test_document_conversion() {
        let document = Document {
            id: Some("test-id".to_string()),
            data: r#"{"title": "Test Document", "content": "Test content"}"#.to_string(),
        };

        let (object_id, algolia_object) = document_to_algolia_object(&document).unwrap();
        
        assert_eq!(object_id, "test-id");
        assert_eq!(algolia_object["objectID"], "test-id");
        assert_eq!(algolia_object["title"], "Test Document");
        
        // Convert back
        let converted_doc = algolia_object_to_document(object_id, algolia_object).unwrap();
        assert_eq!(converted_doc.id, Some("test-id".to_string()));
        assert!(converted_doc.data.contains("Test Document"));
        assert!(!converted_doc.data.contains("objectID")); // Should be removed
    }

    #[test]
    fn test_error_mapping() {
        let error = anyhow!("404 index not found");
        let mapped = map_algolia_error(error);
        assert!(matches!(mapped.code, ErrorCode::InternalError));
        
        let error = anyhow!("429 rate limit exceeded");
        let mapped = map_algolia_error(error);
        assert!(matches!(mapped.code, ErrorCode::RateLimitExceeded));
        assert!(mapped.retry_after.is_some());
    }

    #[test]
    fn test_advanced_schema_configuration() {
        let schema = Schema {
            primary_key: "id".to_string(),
            fields: vec![
                FieldDefinition {
                    name: "title".to_string(),
                    field_type: FieldType::Text,
                    searchable: true,
                    facetable: false,
                    retrievable: true,
                    sortable: true,
                },
                FieldDefinition {
                    name: "category".to_string(),
                    field_type: FieldType::Text,
                    searchable: false,
                    facetable: true,
                    retrievable: true,
                    sortable: false,
                },
            ],
            provider_params: Some(r#"{
                "typoTolerance": true,
                "minWordSizefor1Typo": 4,
                "minWordSizefor2Typos": 8,
                "removeStopWords": ["en", "fr"],
                "ignorePlurals": true,
                "queryLanguages": ["en"],
                "customRanking": ["desc(popularity)", "asc(date)"],
                "synonyms": [{"type": "synonym", "synonyms": ["car", "automobile"]}],
                "stopWords": ["the", "and", "or"],
                "distinct": true,
                "minProximity": 1
            }"#.to_string()),
        };

        let settings = schema_to_index_settings(&schema);
        
        assert_eq!(settings.searchable_attributes, Some(vec!["title".to_string()]));
        assert_eq!(settings.attributes_for_faceting, Some(vec!["filterOnly(category)".to_string()]));
        assert!(settings.typo_tolerance.is_some());
        assert_eq!(settings.min_word_size_for_1_typo, Some(4));
        assert_eq!(settings.min_word_size_for_2_typos, Some(8));
        assert!(settings.remove_stop_words.is_some());
        assert!(settings.ignore_plurals.is_some());
        assert_eq!(settings.query_languages, Some(vec!["en".to_string()]));
        assert_eq!(settings.custom_ranking, Some(vec!["desc(popularity)".to_string(), "asc(date)".to_string()]));
        assert!(settings.synonyms.is_some());
        assert_eq!(settings.stop_words, Some(vec!["the".to_string(), "and".to_string(), "or".to_string()]));
        assert!(settings.distinct.is_some());
        assert_eq!(settings.min_proximity, Some(1));
    }

    #[test]
    fn test_advanced_search_query_conversion() {
        use crate::bindings::FacetFilter;
        
        let query = SearchQuery {
            query: "test query".to_string(),
            facet_filters: vec![
                FacetFilter {
                    field: "category".to_string(),
                    value: "electronics".to_string(),
                },
                FacetFilter {
                    field: "category".to_string(),
                    value: "computers".to_string(),
                },
                FacetFilter {
                    field: "brand".to_string(),
                    value: "apple".to_string(),
                },
            ],
            page: Some(1),
            per_page: Some(20),
            sort_by: Some("price,popularity".to_string()),
            sort_order: Some("asc,desc".to_string()),
        };

        let algolia_query = search_query_to_algolia_query(&query).unwrap();
        
        assert_eq!(algolia_query.query, "test query");
        assert_eq!(algolia_query.page, Some(1));
        assert_eq!(algolia_query.hits_per_page, Some(20));
        
        // Check facet filters structure
        assert!(algolia_query.facet_filters.is_some());
        assert!(algolia_query.facets.is_some());
        
        // Check multi-attribute sorting
        assert_eq!(algolia_query.sort, Some(vec![
            "asc(price)".to_string(),
            "desc(popularity)".to_string()
        ]));
        
        // Check advanced features are enabled
        assert_eq!(algolia_query.get_ranking_info, Some(true));
        assert_eq!(algolia_query.analytics, Some(true));
        assert_eq!(algolia_query.synonyms, Some(true));
    }

    #[test]
    fn test_complex_filter_creation() {
        let filters = [("category", "electronics"), ("brand", "apple"), ("price", "100")];
        
        let and_filter = create_complex_filter(&filters, "and");
        assert_eq!(and_filter, "category:electronics AND brand:apple AND price:100");
        
        let or_filter = create_complex_filter(&filters, "or");
        assert_eq!(or_filter, "category:electronics OR brand:apple OR price:100");
    }

    #[test]
    fn test_advanced_facet_filters() {
        let mut filters = HashMap::new();
        filters.insert("category".to_string(), vec!["electronics".to_string(), "computers".to_string()]);
        filters.insert("brand".to_string(), vec!["apple".to_string()]);
        
        let facet_filters = create_advanced_facet_filters(&filters, "and", "or");
        assert!(facet_filters.is_array());
        
        // Test OR logic between fields
        let facet_filters_or = create_advanced_facet_filters(&filters, "or", "or");
        assert!(facet_filters_or.is_array());
    }

    #[test]
    fn test_provider_query_params() {
        let mut query = AlgoliaSearchQuery {
            query: "test".to_string(),
            filters: None,
            facets: None,
            page: None,
            hits_per_page: None,
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
        
        let provider_params = r#"{
            "numericFilters": ["price > 100", "rating >= 4"],
            "typoTolerance": "strict",
            "synonyms": false,
            "minProximity": 2
        }"#;
        
        apply_provider_query_params(&mut query, Some(provider_params)).unwrap();
        
        assert_eq!(query.numeric_filters, Some(vec!["price > 100".to_string(), "rating >= 4".to_string()]));
        assert_eq!(query.typo_tolerance, Some("strict".to_string()));
        assert_eq!(query.synonyms, Some(false));
        assert_eq!(query.minProximity, Some(2));
    }
}