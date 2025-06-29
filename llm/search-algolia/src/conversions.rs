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
    };
    
    // Convert facet filters to Algolia filters
    if !query.facet_filters.is_empty() {
        let facet_filter_strings: Vec<String> = query.facet_filters
            .iter()
            .map(|filter| format!("{}:{}", filter.field, filter.value))
            .collect();
        
        algolia_query.filters = Some(facet_filter_strings.join(" AND "));
        
        // Extract unique facet fields for faceted search
        let facet_fields: std::collections::HashSet<String> = query.facet_filters
            .iter()
            .map(|f| f.field.clone())
            .collect();
        
        if !facet_fields.is_empty() {
            algolia_query.facets = Some(facet_fields.into_iter().collect());
        }
    }
    
    // Convert sort options
    if let (Some(sort_by), Some(sort_order)) = (&query.sort_by, &query.sort_order) {
        let sort_string = if sort_order == "desc" {
            format!("desc({})", sort_by)
        } else {
            format!("asc({})", sort_by)
        };
        algolia_query.sort = Some(vec![sort_string]);
    }
    
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
    
    // Extract highlighting information
    let highlighted = if let Some(highlight_result) = hit.highlight_result {
        Some(serde_json::to_string(&highlight_result)
            .map_err(|e| anyhow!("Failed to serialize highlight result: {}", e))?)
    } else {
        None
    };
    
    // Extract ranking score if available
    let score = hit.ranking_info
        .as_ref()
        .and_then(|ranking| ranking.get("score"))
        .and_then(|score| score.as_f64());
    
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
        assert!(matches!(mapped.code, ErrorCode::IndexNotFound));
        
        let error = anyhow!("429 rate limit exceeded");
        let mapped = map_algolia_error(error);
        assert!(matches!(mapped.code, ErrorCode::RateLimited));
        assert!(mapped.retry_after.is_some());
    }
}