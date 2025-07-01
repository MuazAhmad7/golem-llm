//! Configuration management for search providers
//!
//! This module provides utilities for loading configuration from environment
//! variables and managing provider-specific settings.

use std::env;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::error::{SearchError, SearchResult};

/// Common configuration for all search providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Provider endpoint URL
    pub endpoint: Option<String>,
    
    /// Request timeout in seconds
    pub timeout: Duration,
    
    /// Maximum number of retries for failed requests
    pub max_retries: u32,
    
    /// Log level for the provider
    pub log_level: String,
    
    /// Provider-specific configuration
    pub provider_config: ProviderConfig,
}

/// Provider-specific configuration variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderConfig {
    Algolia {
        app_id: String,
        api_key: String,
    },
    ElasticSearch {
        username: Option<String>,
        password: Option<String>,
        cloud_id: Option<String>,
        ca_cert: Option<String>,
    },
    OpenSearch {
        username: Option<String>,
        password: Option<String>,
        aws_region: Option<String>,
        aws_access_key: Option<String>,
        aws_secret_key: Option<String>,
    },
    Typesense {
        api_key: String,
        nodes: Vec<String>,
    },
    Meilisearch {
        api_key: Option<String>,
        master_key: Option<String>,
    },
}

impl SearchConfig {
    /// Load configuration from environment variables for the specified provider
    pub fn from_env(provider: &str) -> SearchResult<Self> {
        let endpoint = env::var("SEARCH_PROVIDER_ENDPOINT").ok();
        
        let timeout = env::var("SEARCH_PROVIDER_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .map_err(|e| SearchError::invalid_query(format!("Invalid timeout value: {}", e)))?;
        
        let max_retries = env::var("SEARCH_PROVIDER_MAX_RETRIES")
            .unwrap_or_else(|_| "3".to_string())
            .parse::<u32>()
            .map_err(|e| SearchError::invalid_query(format!("Invalid max_retries value: {}", e)))?;
        
        let log_level = env::var("SEARCH_PROVIDER_LOG_LEVEL")
            .unwrap_or_else(|_| "info".to_string());
        
        let provider_config = match provider.to_lowercase().as_str() {
            "algolia" => Self::load_algolia_config()?,
            "elasticsearch" | "elastic" => Self::load_elasticsearch_config()?,
            "opensearch" => Self::load_opensearch_config()?,
            "typesense" => Self::load_typesense_config()?,
            "meilisearch" => Self::load_meilisearch_config()?,
            _ => return Err(SearchError::invalid_query(format!("Unknown provider: {}", provider))),
        };
        
        Ok(SearchConfig {
            endpoint,
            timeout: Duration::from_secs(timeout),
            max_retries,
            log_level,
            provider_config,
        })
    }
    
    fn load_algolia_config() -> SearchResult<ProviderConfig> {
        let app_id = env::var("ALGOLIA_APP_ID")
            .map_err(|_| SearchError::invalid_query("ALGOLIA_APP_ID environment variable is required"))?;
        
        let api_key = env::var("ALGOLIA_API_KEY")
            .map_err(|_| SearchError::invalid_query("ALGOLIA_API_KEY environment variable is required"))?;
        
        Ok(ProviderConfig::Algolia { app_id, api_key })
    }
    
    fn load_elasticsearch_config() -> SearchResult<ProviderConfig> {
        let username = env::var("ELASTIC_USERNAME").ok();
        let password = env::var("ELASTIC_PASSWORD").ok();
        let cloud_id = env::var("ELASTIC_CLOUD_ID").ok();
        let ca_cert = env::var("ELASTIC_CA_CERT").ok();
        
        Ok(ProviderConfig::ElasticSearch {
            username,
            password,
            cloud_id,
            ca_cert,
        })
    }
    
    fn load_opensearch_config() -> SearchResult<ProviderConfig> {
        let username = env::var("OPENSEARCH_USERNAME").ok();
        let password = env::var("OPENSEARCH_PASSWORD").ok();
        let aws_region = env::var("AWS_REGION").ok();
        let aws_access_key = env::var("AWS_ACCESS_KEY_ID").ok();
        let aws_secret_key = env::var("AWS_SECRET_ACCESS_KEY").ok();
        
        Ok(ProviderConfig::OpenSearch {
            username,
            password,
            aws_region,
            aws_access_key,
            aws_secret_key,
        })
    }
    
    fn load_typesense_config() -> SearchResult<ProviderConfig> {
        let api_key = env::var("TYPESENSE_API_KEY")
            .map_err(|_| SearchError::invalid_query("TYPESENSE_API_KEY environment variable is required"))?;
        
        let nodes = env::var("TYPESENSE_NODES")
            .unwrap_or_else(|_| "http://localhost:8108".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        
        Ok(ProviderConfig::Typesense { api_key, nodes })
    }
    
    fn load_meilisearch_config() -> SearchResult<ProviderConfig> {
        let api_key = env::var("MEILISEARCH_API_KEY").ok();
        let master_key = env::var("MEILISEARCH_MASTER_KEY").ok();
        
        Ok(ProviderConfig::Meilisearch { api_key, master_key })
    }
    
    /// Get the effective endpoint URL for the provider
    pub fn get_endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }
    
    /// Get the timeout duration
    pub fn get_timeout(&self) -> Duration {
        self.timeout
    }
    
    /// Get the maximum number of retries
    pub fn get_max_retries(&self) -> u32 {
        self.max_retries
    }
    
    /// Get the log level
    pub fn get_log_level(&self) -> &str {
        &self.log_level
    }
    
    /// Check if the configuration is valid
    pub fn validate(&self) -> SearchResult<()> {
        match &self.provider_config {
            ProviderConfig::Algolia { app_id, api_key } => {
                if app_id.is_empty() || api_key.is_empty() {
                    return Err(SearchError::invalid_query("Algolia app_id and api_key must not be empty"));
                }
            },
            ProviderConfig::Typesense { api_key, nodes } => {
                if api_key.is_empty() {
                    return Err(SearchError::invalid_query("Typesense api_key must not be empty"));
                }
                if nodes.is_empty() {
                    return Err(SearchError::invalid_query("At least one Typesense node must be specified"));
                }
            },
            _ => {
                // Other providers have optional authentication
            }
        }
        
        Ok(())
    }
}

/// Environment variable helper functions
pub mod env_helpers {
    use super::*;
    
    /// Get a required environment variable
    pub fn get_required_env(key: &str) -> SearchResult<String> {
        env::var(key).map_err(|_| {
            SearchError::invalid_query(format!("Required environment variable {} is not set", key))
        })
    }
    
    /// Get an optional environment variable with a default value
    pub fn get_env_or_default(key: &str, default: &str) -> String {
        env::var(key).unwrap_or_else(|_| default.to_string())
    }
    
    /// Get an environment variable as a parsed type
    pub fn get_env_parsed<T>(key: &str) -> SearchResult<Option<T>>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        match env::var(key) {
            Ok(value) => {
                let parsed = value.parse::<T>()
                    .map_err(|e| SearchError::invalid_query(format!("Failed to parse {}: {}", key, e)))?;
                Ok(Some(parsed))
            },
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_validation() {
        let config = SearchConfig {
            endpoint: None,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            log_level: "info".to_string(),
            provider_config: ProviderConfig::Algolia {
                app_id: "test_app".to_string(),
                api_key: "test_key".to_string(),
            },
        };
        
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_invalid_config() {
        let config = SearchConfig {
            endpoint: None,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            log_level: "info".to_string(),
            provider_config: ProviderConfig::Algolia {
                app_id: "".to_string(),
                api_key: "test_key".to_string(),
            },
        };
        
        assert!(config.validate().is_err());
    }
}