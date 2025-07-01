//! Error handling for search operations
//!
//! This module provides error types and conversion utilities for mapping
//! provider-specific errors to the unified search-error interface.

use thiserror::Error;

/// Unified search error type that maps to the WIT search-error variant
#[derive(Debug, Error, Clone)]
pub enum SearchError {
    #[error("Index not found: {0}")]
    IndexNotFound(String),
    
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    
    #[error("Unsupported operation")]
    Unsupported,
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Operation timed out")]
    Timeout,
    
    #[error("Rate limited")]
    RateLimited,
}

/// Result type alias for search operations
pub type SearchResult<T> = Result<T, SearchError>;

impl SearchError {
    /// Create an internal error from any error type
    pub fn internal<E: std::fmt::Display>(err: E) -> Self {
        Self::Internal(err.to_string())
    }
    
    /// Create an invalid query error
    pub fn invalid_query<S: Into<String>>(msg: S) -> Self {
        Self::InvalidQuery(msg.into())
    }
    
    /// Create an index not found error
    pub fn index_not_found<S: Into<String>>(index_name: S) -> Self {
        Self::IndexNotFound(index_name.into())
    }
}

// Conversion from anyhow::Error
impl From<anyhow::Error> for SearchError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

// Conversion from serde_json::Error
impl From<serde_json::Error> for SearchError {
    fn from(err: serde_json::Error) -> Self {
        Self::InvalidQuery(format!("JSON parsing error: {}", err))
    }
}

// Conversion from reqwest::Error
impl From<reqwest::Error> for SearchError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout
        } else if err.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS) {
            Self::RateLimited
        } else if err.status() == Some(reqwest::StatusCode::NOT_FOUND) {
            Self::IndexNotFound("HTTP 404".to_string())
        } else if err.status() == Some(reqwest::StatusCode::BAD_REQUEST) {
            Self::InvalidQuery(format!("HTTP 400: {}", err))
        } else {
            Self::Internal(err.to_string())
        }
    }
}

// Conversion from tokio::time::error::Elapsed (timeout)
impl From<tokio::time::error::Elapsed> for SearchError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        Self::Timeout
    }
}

// Conversion from url::ParseError
impl From<url::ParseError> for SearchError {
    fn from(err: url::ParseError) -> Self {
        Self::InvalidQuery(format!("URL parsing error: {}", err))
    }
}

// Convert to WIT types
impl From<SearchError> for crate::types::SearchError {
    fn from(err: SearchError) -> Self {
        match err {
            SearchError::IndexNotFound(_) => Self::IndexNotFound,
            SearchError::InvalidQuery(msg) => Self::InvalidQuery(msg),
            SearchError::Unsupported => Self::Unsupported,
            SearchError::Internal(msg) => Self::Internal(msg),
            SearchError::Timeout => Self::Timeout,
            SearchError::RateLimited => Self::RateLimited,
        }
    }
}

// Convert from WIT types
impl From<crate::types::SearchError> for SearchError {
    fn from(err: crate::types::SearchError) -> Self {
        match err {
            crate::types::SearchError::IndexNotFound => Self::IndexNotFound("Unknown index".to_string()),
            crate::types::SearchError::InvalidQuery(msg) => Self::InvalidQuery(msg),
            crate::types::SearchError::Unsupported => Self::Unsupported,
            crate::types::SearchError::Internal(msg) => Self::Internal(msg),
            crate::types::SearchError::Timeout => Self::Timeout,
            crate::types::SearchError::RateLimited => Self::RateLimited,
        }
    }
}

/// Utility macro for creating provider-specific error mappings
#[macro_export]
macro_rules! map_provider_error {
    ($provider:ident, $error:expr) => {
        match $error {
            // Provider-specific mappings can be added here
            _ => $crate::error::SearchError::internal($error),
        }
    };
}

/// Context extension trait for adding error context
pub trait ErrorContext<T> {
    /// Add context to an error
    fn with_context<F>(self, f: F) -> SearchResult<T>
    where
        F: FnOnce() -> String;
    
    /// Add context to an error with a static string
    fn context(self, msg: &'static str) -> SearchResult<T>;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Into<SearchError>,
{
    fn with_context<F>(self, f: F) -> SearchResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let original = e.into();
            match original {
                SearchError::Internal(msg) => SearchError::Internal(format!("{}: {}", f(), msg)),
                other => other,
            }
        })
    }
    
    fn context(self, msg: &'static str) -> SearchResult<T> {
        self.with_context(|| msg.to_string())
    }
}