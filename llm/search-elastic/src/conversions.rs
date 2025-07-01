//! Conversion utilities between common search types and ElasticSearch-specific types

use std::collections::HashMap;
use anyhow::{anyhow, Result};
use serde_json::{Value, json};
use golem_search::{
    SearchError, Doc, SearchQuery, SearchResults, SearchHit, Schema, SchemaField, FieldType,
    HighlightConfig, SearchConfig as WitSearchConfig
};

/// Convert a WIT Schema to ElasticSearch mapping
pub fn schema_to_elastic_mapping(schema: &Schema) -> Result<Value> {
    let mut properties = serde_json::Map::new();
    
    for field in &schema.fields {
        let field_mapping = match field.field_type {
            FieldType::Text => {
                json!({
                    "type": "text",
                    "index": field.index,
                    "analyzer": "standard"
                })
            }
            FieldType::Keyword => {
                json!({
                    "type": "keyword",
                    "index": field.index
                })
            }
            FieldType::Integer => {
                json!({
                    "type": "integer",
                    "index": field.index
                })
            }
            FieldType::Float => {
                json!({
                    "type": "float",
                    "index": field.index
                })
            }
            FieldType::Boolean => {
                json!({
                    "type": "boolean",
                    "index": field.index
                })
            }
            FieldType::Date => {
                json!({
                    "type": "date",
                    "index": field.index,
                    "format": "strict_date_optional_time||epoch_millis"
                })
            }
            FieldType::GeoPoint => {
                json!({
                    "type": "geo_point",
                    "index": field.index
                })
            }
        };
        
        properties.insert(field.name.clone(), field_mapping);
    }
    
    Ok(json!({
        "mappings": {
            "properties": properties
        }
    }))
}

/// Convert ElasticSearch mapping to WIT Schema
pub fn elastic_mapping_to_schema(mapping: &Value, index_name: &str) -> Result<Schema> {
    let properties = mapping
        .get("mappings")
        .and_then(|m| m.get("properties"))
        .ok_or_else(|| anyhow!("Invalid mapping structure"))?;
    
    let mut fields = Vec::new();
    
    if let Value::Object(props) = properties {
        for (field_name, field_def) in props {
            let field_type = field_def
                .get("type")
                .and_then(|t| t.as_str())
                .ok_or_else(|| anyhow!("Missing field type for {}", field_name))?;
            
            let wit_field_type = match field_type {
                "text" => FieldType::Text,
                "keyword" => FieldType::Keyword,
                "integer" | "long" | "short" | "byte" => FieldType::Integer,
                "float" | "double" | "half_float" | "scaled_float" => FieldType::Float,
                "boolean" => FieldType::Boolean,
                "date" => FieldType::Date,
                "geo_point" => FieldType::GeoPoint,
                _ => FieldType::Text, // Default fallback
            };
            
            let index = field_def
                .get("index")
                .and_then(|i| i.as_bool())
                .unwrap_or(true);
            
            fields.push(SchemaField {
                name: field_name.clone(),
                field_type: wit_field_type,
                required: false, // ElasticSearch doesn't have required fields
                facet: field_type == "keyword", // Only keyword fields can be faceted
                sort: field_type != "text", // Text fields typically can't be sorted
                index,
            });
        }
    }
    
    Ok(Schema {
        fields,
        primary_key: Some("_id".to_string()), // ElasticSearch always has _id
    })
}

/// Convert WIT SearchQuery to ElasticSearch query DSL
pub fn search_query_to_elastic_query(query: &SearchQuery) -> Result<Value> {
    let mut elastic_query = json!({
        "query": {
            "bool": {
                "must": [],
                "filter": []
            }
        }
    });
    
    // Add main query
    if let Some(ref q) = query.q {
        if !q.trim().is_empty() {
            let query_part = json!({
                "multi_match": {
                    "query": q,
                    "type": "best_fields",
                    "operator": "or"
                }
            });
            elastic_query["query"]["bool"]["must"]
                .as_array_mut()
                .unwrap()
                .push(query_part);
        }
    }
    
    // Add filters
    for filter in &query.filters {
        // Simple term filter format: "field:value"
        if let Some((field, value)) = filter.split_once(':') {
            let filter_part = json!({
                "term": {
                    field: value
                }
            });
            elastic_query["query"]["bool"]["filter"]
                .as_array_mut()
                .unwrap()
                .push(filter_part);
        }
    }
    
    // Add sorting
    if !query.sort.is_empty() {
        let mut sort_array = Vec::new();
        for sort_field in &query.sort {
            if sort_field.starts_with('-') {
                // Descending sort
                let field = &sort_field[1..];
                sort_array.push(json!({ field: { "order": "desc" } }));
            } else {
                // Ascending sort
                sort_array.push(json!({ sort_field: { "order": "asc" } }));
            }
        }
        elastic_query["sort"] = json!(sort_array);
    }
    
    // Add pagination
    if let Some(page) = query.page {
        let per_page = query.per_page.unwrap_or(10);
        elastic_query["from"] = json!(page * per_page);
        elastic_query["size"] = json!(per_page);
    } else if let Some(offset) = query.offset {
        let size = query.per_page.unwrap_or(10);
        elastic_query["from"] = json!(offset);
        elastic_query["size"] = json!(size);
    } else {
        elastic_query["size"] = json!(query.per_page.unwrap_or(10));
    }
    
    // Add highlighting
    if let Some(ref highlight_config) = query.highlight {
        let mut highlight = json!({
            "fields": {}
        });
        
        for field in &highlight_config.fields {
            highlight["fields"][field] = json!({});
        }
        
        if let Some(ref pre_tag) = highlight_config.pre_tag {
            highlight["pre_tags"] = json!([pre_tag]);
        }
        
        if let Some(ref post_tag) = highlight_config.post_tag {
            highlight["post_tags"] = json!([post_tag]);
        }
        
        if let Some(max_length) = highlight_config.max_length {
            highlight["fragment_size"] = json!(max_length);
        }
        
        elastic_query["highlight"] = highlight;
    }
    
    // Add aggregations for facets
    if !query.facets.is_empty() {
        let mut aggs = serde_json::Map::new();
        for facet_field in &query.facets {
            aggs.insert(
                format!("{}_facet", facet_field),
                json!({
                    "terms": {
                        "field": facet_field,
                        "size": 100
                    }
                })
            );
        }
        elastic_query["aggs"] = json!(aggs);
    }
    
    Ok(elastic_query)
}

/// Convert ElasticSearch search response to WIT SearchResults
pub fn elastic_response_to_search_results(response: &Value) -> Result<SearchResults> {
    let hits_obj = response
        .get("hits")
        .ok_or_else(|| anyhow!("Missing hits in response"))?;
    
    let total = hits_obj
        .get("total")
        .and_then(|t| {
            // Handle both old format (number) and new format (object with value)
            if t.is_number() {
                t.as_u64()
            } else {
                t.get("value").and_then(|v| v.as_u64())
            }
        })
        .map(|t| t as u32);
    
    let hits_array = hits_obj
        .get("hits")
        .and_then(|h| h.as_array())
        .ok_or_else(|| anyhow!("Missing hits array in response"))?;
    
    let mut hits = Vec::new();
    for hit in hits_array {
        let id = hit
            .get("_id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| anyhow!("Missing document ID"))?
            .to_string();
        
        let source = hit.get("_source");
        let content = if let Some(source) = source {
            Some(serde_json::to_string(source)?)
        } else {
            None
        };
        
        let score = hit.get("_score").and_then(|s| s.as_f64());
        
        let highlights = hit.get("highlight").map(|h| serde_json::to_string(h)).transpose()?;
        
        hits.push(SearchHit {
            id,
            score,
            content,
            highlights,
        });
    }
    
    // Extract facets from aggregations
    let facets = response.get("aggregations").map(|aggs| {
        serde_json::to_string(aggs).unwrap_or_default()
    });
    
    let took_ms = response
        .get("took")
        .and_then(|t| t.as_u64())
        .map(|t| t as u32);
    
    Ok(SearchResults {
        total,
        page: None, // Will be calculated from request context
        per_page: None, // Will be calculated from request context
        hits,
        facets,
        took_ms,
    })
}

/// Convert WIT Doc to ElasticSearch document
pub fn doc_to_elastic_document(doc: &Doc) -> Result<(String, Value)> {
    let content: Value = serde_json::from_str(&doc.content)
        .map_err(|e| anyhow!("Invalid JSON in document content: {}", e))?;
    
    Ok((doc.id.clone(), content))
}

/// Convert ElasticSearch document response to WIT Doc
pub fn elastic_document_to_doc(response: &Value) -> Result<Doc> {
    let id = response
        .get("_id")
        .and_then(|id| id.as_str())
        .ok_or_else(|| anyhow!("Missing document ID"))?
        .to_string();
    
    let source = response
        .get("_source")
        .ok_or_else(|| anyhow!("Missing document source"))?;
    
    let content = serde_json::to_string(source)
        .map_err(|e| anyhow!("Failed to serialize document content: {}", e))?;
    
    Ok(Doc { id, content })
}

/// Convert bulk operations to ElasticSearch bulk format
pub fn docs_to_bulk_operations(index: &str, docs: &[Doc], operation: &str) -> Result<Vec<Value>> {
    let mut operations = Vec::new();
    
    for doc in docs {
        // Add operation header
        let op_header = match operation {
            "index" => json!({
                "index": {
                    "_index": index,
                    "_id": doc.id
                }
            }),
            "delete" => json!({
                "delete": {
                    "_index": index,
                    "_id": doc.id
                }
            }),
            _ => return Err(anyhow!("Unsupported bulk operation: {}", operation)),
        };
        
        operations.push(op_header);
        
        // Add document body for index operations
        if operation == "index" {
            let content: Value = serde_json::from_str(&doc.content)
                .map_err(|e| anyhow!("Invalid JSON in document content: {}", e))?;
            operations.push(content);
        }
    }
    
    Ok(operations)
}

/// Map ElasticSearch errors to SearchError
pub fn map_elastic_error(error: anyhow::Error) -> SearchError {
    let error_string = error.to_string();
    
    if error_string.contains("index_not_found") || error_string.contains("404") {
        SearchError::IndexNotFound(error_string)
    } else if error_string.contains("parsing_exception") || error_string.contains("400") {
        SearchError::InvalidQuery(error_string)
    } else if error_string.contains("timeout") {
        SearchError::Timeout
    } else if error_string.contains("rate") || error_string.contains("429") {
        SearchError::RateLimited
    } else {
        SearchError::Internal(error_string)
    }
}