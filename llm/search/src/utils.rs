//! Utility functions and streaming support for search providers
//!
//! This module provides common utilities and streaming implementations
//! that can be shared across different search providers.

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use tokio::sync::mpsc;
use crate::types::{SearchHit, SearchQuery, SearchResults};
use crate::error::{SearchError, SearchResult};

/// Stream implementation for search hits
pub struct SearchHitStream {
    receiver: Option<mpsc::Receiver<SearchResult<SearchHit>>>,
    buffer: VecDeque<SearchHit>,
    finished: bool,
}

impl SearchHitStream {
    /// Create a new search hit stream
    pub fn new() -> (Self, mpsc::Sender<SearchResult<SearchHit>>) {
        let (sender, receiver) = mpsc::channel(100);
        (
            Self {
                receiver: Some(receiver),
                buffer: VecDeque::new(),
                finished: false,
            },
            sender,
        )
    }
    
    /// Create a stream from paginated results
    pub fn from_paginated<F>(
        query: SearchQuery,
        page_size: u32,
        fetch_page: F,
    ) -> Self
    where
        F: Fn(SearchQuery, u32) -> SearchResult<SearchResults> + Send + 'static,
    {
        let (stream, sender) = Self::new();
        
        tokio::spawn(async move {
            let mut page = query.page.unwrap_or(0);
            let per_page = query.per_page.unwrap_or(page_size);
            
            loop {
                let mut page_query = query.clone();
                page_query.page = Some(page);
                page_query.per_page = Some(per_page);
                
                match fetch_page(page_query, page) {
                    Ok(results) => {
                        let has_more = results.hits.len() == per_page as usize;
                        
                        for hit in results.hits {
                            if sender.send(Ok(hit)).await.is_err() {
                                return; // Receiver dropped
                            }
                        }
                        
                        if !has_more {
                            break;
                        }
                        
                        page += 1;
                    }
                    Err(e) => {
                        let _ = sender.send(Err(e)).await;
                        break;
                    }
                }
            }
        });
        
        stream
    }
    
    /// Get the next batch of search hits
    pub async fn next_batch(&mut self, size: usize) -> Option<Vec<SearchHit>> {
        if self.finished && self.buffer.is_empty() {
            return None;
        }
        
        let mut batch = Vec::with_capacity(size);
        
        // First, drain from buffer
        while batch.len() < size && !self.buffer.is_empty() {
            if let Some(hit) = self.buffer.pop_front() {
                batch.push(hit);
            }
        }
        
        // If we need more items and stream is still active, fetch from receiver
        if batch.len() < size && !self.finished {
            if let Some(ref mut receiver) = self.receiver {
                while batch.len() < size {
                    match receiver.recv().await {
                        Some(Ok(hit)) => batch.push(hit),
                        Some(Err(_)) => {
                            self.finished = true;
                            break;
                        }
                        None => {
                            self.finished = true;
                            break;
                        }
                    }
                }
            }
        }
        
        if batch.is_empty() {
            None
        } else {
            Some(batch)
        }
    }
}

impl Default for SearchHitStream {
    fn default() -> Self {
        let (stream, _) = Self::new();
        stream
    }
}

/// Retry utility for handling transient failures
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Execute a function with retry logic
pub async fn retry_with_backoff<F, Fut, T>(
    config: &RetryConfig,
    operation: F,
) -> SearchResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = SearchResult<T>>,
{
    let mut last_error = None;
    let mut delay_ms = config.base_delay_ms;
    
    for attempt in 0..config.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                last_error = Some(error.clone());
                
                // Don't retry on certain errors
                match error {
                    SearchError::InvalidQuery(_) | SearchError::Unsupported => {
                        return Err(error);
                    }
                    _ => {}
                }
                
                // If this isn't the last attempt, wait before retrying
                if attempt < config.max_attempts - 1 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    delay_ms = std::cmp::min(
                        (delay_ms as f64 * config.backoff_multiplier) as u64,
                        config.max_delay_ms,
                    );
                }
            }
        }
    }
    
    Err(last_error.unwrap_or_else(|| SearchError::Internal("Retry failed".to_string())))
}

/// Rate limiter for controlling request frequency
pub struct RateLimiter {
    permits: Arc<Mutex<u32>>,
    max_permits: u32,
    refill_rate: u32, // permits per second
    last_refill: Arc<Mutex<std::time::Instant>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_permits: u32, refill_rate: u32) -> Self {
        Self {
            permits: Arc::new(Mutex::new(max_permits)),
            max_permits,
            refill_rate,
            last_refill: Arc::new(Mutex::new(std::time::Instant::now())),
        }
    }
    
    /// Try to acquire a permit (non-blocking)
    pub fn try_acquire(&self) -> bool {
        self.refill_permits();
        
        let mut permits = self.permits.lock().unwrap();
        if *permits > 0 {
            *permits -= 1;
            true
        } else {
            false
        }
    }
    
    /// Acquire a permit (blocking until available)
    pub async fn acquire(&self) -> SearchResult<()> {
        loop {
            if self.try_acquire() {
                return Ok(());
            }
            
            // Calculate how long to wait for next permit
            let wait_time = 1000 / std::cmp::max(self.refill_rate, 1); // milliseconds
            tokio::time::sleep(tokio::time::Duration::from_millis(wait_time as u64)).await;
        }
    }
    
    fn refill_permits(&self) {
        let now = std::time::Instant::now();
        let mut last_refill = self.last_refill.lock().unwrap();
        let elapsed = now.duration_since(*last_refill);
        
        if elapsed.as_secs() >= 1 {
            let permits_to_add = (elapsed.as_secs() as u32) * self.refill_rate;
            let mut permits = self.permits.lock().unwrap();
            *permits = std::cmp::min(*permits + permits_to_add, self.max_permits);
            *last_refill = now;
        }
    }
}

/// Utility functions for working with search queries
pub mod query_utils {
    use super::*;
    use crate::types::{SearchQuery, HighlightConfig};
    
    /// Validate that a query is well-formed
    pub fn validate_query(query: &SearchQuery) -> SearchResult<()> {
        // Check for empty or invalid query string
        if let Some(ref q) = query.q {
            if q.trim().is_empty() {
                return Err(SearchError::invalid_query("Query string cannot be empty"));
            }
            
            if q.len() > 10000 {
                return Err(SearchError::invalid_query("Query string too long"));
            }
        }
        
        // Validate pagination parameters
        if let (Some(page), Some(per_page)) = (query.page, query.per_page) {
            if per_page == 0 {
                return Err(SearchError::invalid_query("per_page must be greater than 0"));
            }
            
            if per_page > 1000 {
                return Err(SearchError::invalid_query("per_page cannot exceed 1000"));
            }
            
            if page > 10000 {
                return Err(SearchError::invalid_query("page cannot exceed 10000"));
            }
        }
        
        // Validate offset parameters
        if let Some(offset) = query.offset {
            if offset > 100000 {
                return Err(SearchError::invalid_query("offset cannot exceed 100000"));
            }
        }
        
        // Validate filters
        for filter in &query.filters {
            if filter.trim().is_empty() {
                return Err(SearchError::invalid_query("Filter cannot be empty"));
            }
        }
        
        // Validate sorts
        for sort in &query.sort {
            if sort.trim().is_empty() {
                return Err(SearchError::invalid_query("Sort field cannot be empty"));
            }
        }
        
        Ok(())
    }
    
    /// Extract highlights from a query
    pub fn extract_highlight_fields(query: &SearchQuery) -> Vec<String> {
        query.highlight
            .as_ref()
            .map(|h| h.fields.clone())
            .unwrap_or_default()
    }
    
    /// Create a basic highlight configuration
    pub fn create_basic_highlight(fields: Vec<String>) -> HighlightConfig {
        HighlightConfig {
            fields,
            pre_tag: Some("<mark>".to_string()),
            post_tag: Some("</mark>".to_string()),
            max_length: Some(200),
        }
    }
    
    /// Normalize query string for consistent processing
    pub fn normalize_query_string(query: &str) -> String {
        query
            .trim()
            .replace('\n', " ")
            .replace('\r', " ")
            .replace('\t', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Utility functions for working with documents
pub mod document_utils {
    use super::*;
    use crate::types::Doc;
    use serde_json::Value;
    
    /// Validate that a document is well-formed
    pub fn validate_document(doc: &Doc) -> SearchResult<()> {
        if doc.id.trim().is_empty() {
            return Err(SearchError::invalid_query("Document ID cannot be empty"));
        }
        
        // Try to parse the content as JSON
        serde_json::from_str::<Value>(&doc.content)
            .map_err(|e| SearchError::invalid_query(format!("Invalid JSON content: {}", e)))?;
        
        Ok(())
    }
    
    /// Extract a field value from a document's JSON content
    pub fn extract_field(doc: &Doc, field: &str) -> SearchResult<Option<Value>> {
        let content: Value = serde_json::from_str(&doc.content)?;
        Ok(content.get(field).cloned())
    }
    
    /// Set a field value in a document's JSON content
    pub fn set_field(doc: &mut Doc, field: &str, value: Value) -> SearchResult<()> {
        let mut content: Value = serde_json::from_str(&doc.content)?;
        
        if let Value::Object(ref mut map) = content {
            map.insert(field.to_string(), value);
            doc.content = serde_json::to_string(&content)?;
        } else {
            return Err(SearchError::invalid_query("Document content is not a JSON object"));
        }
        
        Ok(())
    }
    
    /// Calculate the size of a document in bytes
    pub fn document_size(doc: &Doc) -> usize {
        doc.id.len() + doc.content.len()
    }
    
    /// Batch documents for efficient processing
    pub fn batch_documents(docs: Vec<Doc>, max_batch_size: usize, max_bytes: usize) -> Vec<Vec<Doc>> {
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();
        let mut current_size = 0;
        
        for doc in docs {
            let doc_size = document_size(&doc);
            
            if current_batch.len() >= max_batch_size || 
               (current_size + doc_size > max_bytes && !current_batch.is_empty()) {
                batches.push(current_batch);
                current_batch = Vec::new();
                current_size = 0;
            }
            
            current_batch.push(doc);
            current_size += doc_size;
        }
        
        if !current_batch.is_empty() {
            batches.push(current_batch);
        }
        
        batches
    }
}

/// Utility functions for working with indexes
pub mod index_utils {
    use super::*;
    use crate::types::{Schema, SchemaField, FieldType};
    
    /// Validate an index name
    pub fn validate_index_name(name: &str) -> SearchResult<()> {
        if name.trim().is_empty() {
            return Err(SearchError::invalid_query("Index name cannot be empty"));
        }
        
        if name.len() > 255 {
            return Err(SearchError::invalid_query("Index name too long"));
        }
        
        // Check for valid characters (alphanumeric, hyphens, underscores)
        if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(SearchError::invalid_query(
                "Index name can only contain alphanumeric characters, hyphens, and underscores"
            ));
        }
        
        // Cannot start with hyphen or underscore
        if name.starts_with('-') || name.starts_with('_') {
            return Err(SearchError::invalid_query(
                "Index name cannot start with hyphen or underscore"
            ));
        }
        
        Ok(())
    }
    
    /// Validate a schema
    pub fn validate_schema(schema: &Schema) -> SearchResult<()> {
        if schema.fields.is_empty() {
            return Err(SearchError::invalid_query("Schema must have at least one field"));
        }
        
        // Check for duplicate field names
        let mut field_names = std::collections::HashSet::new();
        for field in &schema.fields {
            if !field_names.insert(&field.name) {
                return Err(SearchError::invalid_query(
                    format!("Duplicate field name: {}", field.name)
                ));
            }
            
            validate_field(field)?;
        }
        
        // Validate primary key if specified
        if let Some(ref primary_key) = schema.primary_key {
            if !field_names.contains(primary_key) {
                return Err(SearchError::invalid_query(
                    "Primary key field must be defined in schema"
                ));
            }
        }
        
        Ok(())
    }
    
    fn validate_field(field: &SchemaField) -> SearchResult<()> {
        if field.name.trim().is_empty() {
            return Err(SearchError::invalid_query("Field name cannot be empty"));
        }
        
        if field.name.len() > 255 {
            return Err(SearchError::invalid_query("Field name too long"));
        }
        
        // Validate field type constraints
        match field.field_type {
            FieldType::GeoPoint => {
                if field.facet {
                    return Err(SearchError::invalid_query(
                        "Geo-point fields cannot be faceted"
                    ));
                }
                if field.sort {
                    return Err(SearchError::invalid_query(
                        "Geo-point fields cannot be sorted"
                    ));
                }
            }
            FieldType::Text => {
                if field.sort {
                    return Err(SearchError::invalid_query(
                        "Text fields are typically not suitable for sorting. Consider using keyword type."
                    ));
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}