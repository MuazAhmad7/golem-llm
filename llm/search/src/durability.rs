//! Durability integration for Golem platform
//!
//! This module provides durability support for search operations,
//! allowing operations to be resumed after interruptions.

#[cfg(feature = "durability")]
use golem_rust::{durability, StateStore};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::{SearchError, SearchResult};
use crate::types::{Doc, SearchQuery, SearchResults};

/// State for tracking batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationState {
    /// Operation type
    pub operation_type: BatchOperationType,
    
    /// Index name
    pub index_name: String,
    
    /// Total number of items to process
    pub total_items: usize,
    
    /// Number of items processed successfully
    pub processed_items: usize,
    
    /// Failed items with their errors
    pub failed_items: Vec<FailedItem>,
    
    /// Checkpoint data for resuming
    pub checkpoint_data: Option<String>,
    
    /// Operation started timestamp
    pub started_at: String,
    
    /// Last checkpoint timestamp
    pub last_checkpoint: Option<String>,
}

/// Types of batch operations that can be made durable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchOperationType {
    UpsertMany,
    DeleteMany,
    BulkImport,
    IndexRebuilding,
}

/// Information about a failed item in a batch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedItem {
    /// Item identifier (document ID or position in batch)
    pub item_id: String,
    
    /// Error message
    pub error_message: String,
    
    /// Whether this item can be retried
    pub retryable: bool,
}

/// State for tracking streaming search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOperationState {
    /// The search query
    pub query: SearchQuery,
    
    /// Index name
    pub index_name: String,
    
    /// Current page/offset position
    pub current_position: u64,
    
    /// Total items streamed so far
    pub streamed_items: u64,
    
    /// Last successful checkpoint
    pub last_checkpoint: String,
    
    /// Stream configuration
    pub config: StreamConfig,
}

/// Configuration for streaming operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Batch size for streaming
    pub batch_size: u32,
    
    /// Checkpoint frequency (number of items)
    pub checkpoint_frequency: u64,
    
    /// Maximum retries for failed batches
    pub max_retries: u32,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            checkpoint_frequency: 1000,
            max_retries: 3,
        }
    }
}

/// Durability manager for search operations
pub struct DurabilityManager {
    #[cfg(feature = "durability")]
    state_store: StateStore,
    
    /// In-memory state for non-durability builds
    #[cfg(not(feature = "durability"))]
    memory_state: HashMap<String, String>,
}

impl DurabilityManager {
    /// Create a new durability manager
    pub fn new() -> SearchResult<Self> {
        #[cfg(feature = "durability")]
        {
            let state_store = StateStore::new()
                .map_err(|e| SearchError::internal(format!("Failed to initialize state store: {}", e)))?;
            
            Ok(Self { state_store })
        }
        
        #[cfg(not(feature = "durability"))]
        {
            Ok(Self {
                memory_state: HashMap::new(),
            })
        }
    }
    
    /// Save batch operation state
    pub async fn save_batch_state(&mut self, operation_id: &str, state: &BatchOperationState) -> SearchResult<()> {
        let state_json = serde_json::to_string(state)
            .map_err(|e| SearchError::internal(format!("Failed to serialize state: {}", e)))?;
        
        #[cfg(feature = "durability")]
        {
            self.state_store.set(operation_id, &state_json)
                .map_err(|e| SearchError::internal(format!("Failed to save state: {}", e)))?;
        }
        
        #[cfg(not(feature = "durability"))]
        {
            self.memory_state.insert(operation_id.to_string(), state_json);
        }
        
        Ok(())
    }
    
    /// Load batch operation state
    pub async fn load_batch_state(&self, operation_id: &str) -> SearchResult<Option<BatchOperationState>> {
        #[cfg(feature = "durability")]
        {
            match self.state_store.get(operation_id) {
                Ok(Some(state_json)) => {
                    let state = serde_json::from_str(&state_json)
                        .map_err(|e| SearchError::internal(format!("Failed to deserialize state: {}", e)))?;
                    Ok(Some(state))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(SearchError::internal(format!("Failed to load state: {}", e))),
            }
        }
        
        #[cfg(not(feature = "durability"))]
        {
            match self.memory_state.get(operation_id) {
                Some(state_json) => {
                    let state = serde_json::from_str(state_json)
                        .map_err(|e| SearchError::internal(format!("Failed to deserialize state: {}", e)))?;
                    Ok(Some(state))
                }
                None => Ok(None),
            }
        }
    }
    
    /// Remove batch operation state
    pub async fn remove_batch_state(&mut self, operation_id: &str) -> SearchResult<()> {
        #[cfg(feature = "durability")]
        {
            self.state_store.remove(operation_id)
                .map_err(|e| SearchError::internal(format!("Failed to remove state: {}", e)))?;
        }
        
        #[cfg(not(feature = "durability"))]
        {
            self.memory_state.remove(operation_id);
        }
        
        Ok(())
    }
    
    /// Save stream operation state
    pub async fn save_stream_state(&mut self, stream_id: &str, state: &StreamOperationState) -> SearchResult<()> {
        let state_json = serde_json::to_string(state)
            .map_err(|e| SearchError::internal(format!("Failed to serialize stream state: {}", e)))?;
        
        let key = format!("stream_{}", stream_id);
        
        #[cfg(feature = "durability")]
        {
            self.state_store.set(&key, &state_json)
                .map_err(|e| SearchError::internal(format!("Failed to save stream state: {}", e)))?;
        }
        
        #[cfg(not(feature = "durability"))]
        {
            self.memory_state.insert(key, state_json);
        }
        
        Ok(())
    }
    
    /// Load stream operation state
    pub async fn load_stream_state(&self, stream_id: &str) -> SearchResult<Option<StreamOperationState>> {
        let key = format!("stream_{}", stream_id);
        
        #[cfg(feature = "durability")]
        {
            match self.state_store.get(&key) {
                Ok(Some(state_json)) => {
                    let state = serde_json::from_str(&state_json)
                        .map_err(|e| SearchError::internal(format!("Failed to deserialize stream state: {}", e)))?;
                    Ok(Some(state))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(SearchError::internal(format!("Failed to load stream state: {}", e))),
            }
        }
        
        #[cfg(not(feature = "durability"))]
        {
            match self.memory_state.get(&key) {
                Some(state_json) => {
                    let state = serde_json::from_str(state_json)
                        .map_err(|e| SearchError::internal(format!("Failed to deserialize stream state: {}", e)))?;
                    Ok(Some(state))
                }
                None => Ok(None),
            }
        }
    }
    
    /// Create a checkpoint for the current operation
    pub async fn checkpoint(&mut self, operation_id: &str) -> SearchResult<()> {
        #[cfg(feature = "durability")]
        {
            durability::checkpoint()
                .map_err(|e| SearchError::internal(format!("Failed to create checkpoint: {}", e)))?;
        }
        
        log::debug!("Created checkpoint for operation: {}", operation_id);
        Ok(())
    }
    
    /// List all active batch operations
    pub async fn list_active_operations(&self) -> SearchResult<Vec<String>> {
        #[cfg(feature = "durability")]
        {
            let keys = self.state_store.list_keys()
                .map_err(|e| SearchError::internal(format!("Failed to list keys: {}", e)))?;
            
            Ok(keys.into_iter()
                .filter(|k| !k.starts_with("stream_"))
                .collect())
        }
        
        #[cfg(not(feature = "durability"))]
        {
            Ok(self.memory_state.keys()
                .filter(|k| !k.starts_with("stream_"))
                .cloned()
                .collect())
        }
    }
    
    /// List all active stream operations
    pub async fn list_active_streams(&self) -> SearchResult<Vec<String>> {
        #[cfg(feature = "durability")]
        {
            let keys = self.state_store.list_keys()
                .map_err(|e| SearchError::internal(format!("Failed to list keys: {}", e)))?;
            
            Ok(keys.into_iter()
                .filter_map(|k| {
                    if k.starts_with("stream_") {
                        Some(k[7..].to_string()) // Remove "stream_" prefix
                    } else {
                        None
                    }
                })
                .collect())
        }
        
        #[cfg(not(feature = "durability"))]
        {
            Ok(self.memory_state.keys()
                .filter_map(|k| {
                    if k.starts_with("stream_") {
                        Some(k[7..].to_string()) // Remove "stream_" prefix
                    } else {
                        None
                    }
                })
                .collect())
        }
    }
}

impl Default for DurabilityManager {
    fn default() -> Self {
        Self::new().expect("Failed to create durability manager")
    }
}

/// Durable batch operation executor
pub struct DurableBatchExecutor<'a> {
    durability_manager: &'a mut DurabilityManager,
    operation_id: String,
    state: BatchOperationState,
}

impl<'a> DurableBatchExecutor<'a> {
    /// Create a new durable batch executor
    pub async fn new(
        durability_manager: &'a mut DurabilityManager,
        operation_id: String,
        operation_type: BatchOperationType,
        index_name: String,
        total_items: usize,
    ) -> SearchResult<Self> {
        let state = BatchOperationState {
            operation_type,
            index_name,
            total_items,
            processed_items: 0,
            failed_items: Vec::new(),
            checkpoint_data: None,
            started_at: chrono::Utc::now().to_rfc3339(),
            last_checkpoint: None,
        };
        
        durability_manager.save_batch_state(&operation_id, &state).await?;
        
        Ok(Self {
            durability_manager,
            operation_id,
            state,
        })
    }
    
    /// Resume an existing batch operation
    pub async fn resume(
        durability_manager: &'a mut DurabilityManager,
        operation_id: String,
    ) -> SearchResult<Option<Self>> {
        match durability_manager.load_batch_state(&operation_id).await? {
            Some(state) => Ok(Some(Self {
                durability_manager,
                operation_id,
                state,
            })),
            None => Ok(None),
        }
    }
    
    /// Process a batch of items with automatic checkpointing
    pub async fn process_batch<T, F, Fut>(
        &mut self,
        items: Vec<T>,
        process_fn: F,
    ) -> SearchResult<Vec<T>>
    where
        F: Fn(T) -> Fut,
        Fut: std::future::Future<Output = SearchResult<()>>,
    {
        let mut remaining_items = Vec::new();
        
        for item in items {
            match process_fn(item).await {
                Ok(()) => {
                    self.state.processed_items += 1;
                }
                Err(e) => {
                    self.state.failed_items.push(FailedItem {
                        item_id: self.state.processed_items.to_string(),
                        error_message: e.to_string(),
                        retryable: matches!(e, SearchError::Timeout | SearchError::RateLimited | SearchError::Internal(_)),
                    });
                    
                    // For retryable errors, add to remaining items
                    if matches!(e, SearchError::Timeout | SearchError::RateLimited | SearchError::Internal(_)) {
                        remaining_items.push(item);
                    }
                }
            }
            
            // Checkpoint every 100 items
            if self.state.processed_items % 100 == 0 {
                self.checkpoint().await?;
            }
        }
        
        self.checkpoint().await?;
        Ok(remaining_items)
    }
    
    /// Create a checkpoint
    pub async fn checkpoint(&mut self) -> SearchResult<()> {
        self.state.last_checkpoint = Some(chrono::Utc::now().to_rfc3339());
        self.durability_manager.save_batch_state(&self.operation_id, &self.state).await?;
        self.durability_manager.checkpoint(&self.operation_id).await?;
        Ok(())
    }
    
    /// Complete the operation and clean up state
    pub async fn complete(mut self) -> SearchResult<BatchOperationState> {
        self.durability_manager.remove_batch_state(&self.operation_id).await?;
        Ok(self.state)
    }
    
    /// Get the current state
    pub fn get_state(&self) -> &BatchOperationState {
        &self.state
    }
    
    /// Check if the operation is complete
    pub fn is_complete(&self) -> bool {
        self.state.processed_items >= self.state.total_items
    }
    
    /// Get progress percentage
    pub fn progress_percentage(&self) -> f64 {
        if self.state.total_items == 0 {
            100.0
        } else {
            (self.state.processed_items as f64 / self.state.total_items as f64) * 100.0
        }
    }
}

/// Utility functions for durability
pub mod utils {
    use super::*;
    
    /// Generate a unique operation ID
    pub fn generate_operation_id(prefix: &str) -> String {
        format!("{}_{}", prefix, uuid::Uuid::new_v4())
    }
    
    /// Calculate optimal checkpoint frequency based on operation size
    pub fn calculate_checkpoint_frequency(total_items: usize, target_checkpoints: usize) -> usize {
        std::cmp::max(1, total_items / target_checkpoints)
    }
    
    /// Estimate memory usage for batch operation state
    pub fn estimate_state_memory_usage(state: &BatchOperationState) -> usize {
        std::mem::size_of::<BatchOperationState>() +
        state.index_name.len() +
        state.failed_items.iter().map(|item| item.item_id.len() + item.error_message.len()).sum::<usize>() +
        state.checkpoint_data.as_ref().map(|s| s.len()).unwrap_or(0)
    }
}