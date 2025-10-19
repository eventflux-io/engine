// SPDX-License-Identifier: MIT OR Apache-2.0

//! # Source Error Handling Support
//!
//! This module provides helper utilities for Source implementations to integrate
//! error handling with minimal boilerplate.
//!
//! ## Overview
//!
//! The `SourceErrorContext` encapsulates error handling state and provides
//! convenient methods for sources to handle errors in their event loops.
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! pub struct MySource {
//!     error_ctx: Option<SourceErrorContext>,
//!     // ... other fields
//! }
//!
//! impl Source for MySource {
//!     fn start(&mut self, handler: Arc<Mutex<InputHandler>>) {
//!         loop {
//!             match self.fetch_event() {
//!                 Ok(event) => {
//!                     // Success - send event and reset error counter
//!                     handler.lock().unwrap().send_event(event).ok();
//!                     if let Some(ctx) = &mut self.error_ctx {
//!                         ctx.reset_errors();
//!                     }
//!                 }
//!                 Err(e) => {
//!                     // Error - use context to handle
//!                     if let Some(ctx) = &mut self.error_ctx {
//!                         if !ctx.handle_error(Some(&event), &e) {
//!                             return; // Stop source
//!                         }
//!                     }
//!                 }
//!             }
//!         }
//!     }
//! }
//! ```

use super::handler::{ErrorAction, ErrorHandler};
use crate::core::config::FlatConfig;
use crate::core::error::ErrorConfig;
use crate::core::event::Event;
use crate::core::exception::EventFluxError;
use crate::core::stream::input::input_handler::InputHandler;
use std::sync::{Arc, Mutex};
use std::thread;

/// Source error handling context
///
/// Encapsulates error handling state and provides convenient methods
/// for sources to integrate error handling.
pub struct SourceErrorContext {
    /// The error handler
    handler: ErrorHandler,
}

impl SourceErrorContext {
    /// Create a new source error context
    ///
    /// # Arguments
    /// * `error_config` - Error handling configuration
    /// * `dlq_junction` - Optional DLQ stream junction
    /// * `stream_name` - Name of the source stream
    pub fn new(
        error_config: ErrorConfig,
        dlq_junction: Option<Arc<Mutex<InputHandler>>>,
        stream_name: String,
    ) -> Self {
        Self {
            handler: ErrorHandler::new(error_config, dlq_junction, stream_name),
        }
    }

    /// Create from FlatConfig (convenience method)
    ///
    /// # Arguments
    /// * `config` - Flat configuration with error.* properties
    /// * `dlq_junction` - Optional DLQ stream junction
    /// * `stream_name` - Name of the source stream
    pub fn from_config(
        config: &FlatConfig,
        dlq_junction: Option<Arc<Mutex<InputHandler>>>,
        stream_name: String,
    ) -> Result<Self, String> {
        let error_config = ErrorConfig::from_flat_config(config)?;
        Ok(Self::new(error_config, dlq_junction, stream_name))
    }

    /// Handle an error and return whether to continue processing
    ///
    /// This method applies the error handling strategy and performs any
    /// necessary actions (retry delays, DLQ sending, etc.).
    ///
    /// # Arguments
    /// * `event` - The event that failed (if available)
    /// * `error` - The error that occurred
    ///
    /// # Returns
    /// * `true` - Continue processing (error was handled, drop/retry/dlq)
    /// * `false` - Stop processing (fail strategy or unrecoverable error)
    pub fn handle_error(&mut self, event: Option<&Event>, error: &EventFluxError) -> bool {
        let action = self.handler.handle_error(event, error);

        match action {
            ErrorAction::Retry { delay } => {
                // Sleep for retry delay
                thread::sleep(delay);
                true // Continue processing (retry)
            }
            ErrorAction::Drop => {
                true // Continue processing (skip this event)
            }
            ErrorAction::SendToDlq => {
                true // Continue processing (event sent to DLQ)
            }
            ErrorAction::Fail => {
                false // Stop processing
            }
        }
    }

    /// Reset error counters (call after successful event processing)
    #[inline]
    pub fn reset_errors(&mut self) {
        self.handler.reset_consecutive_errors();
    }

    /// Get consecutive error count
    #[inline]
    pub fn error_count(&self) -> usize {
        self.handler.consecutive_error_count()
    }

    /// Get the underlying error handler (for advanced use cases)
    #[inline]
    pub fn handler(&self) -> &ErrorHandler {
        &self.handler
    }

    /// Get mutable reference to error handler (for advanced use cases)
    #[inline]
    pub fn handler_mut(&mut self) -> &mut ErrorHandler {
        &mut self.handler
    }
}

impl std::fmt::Debug for SourceErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceErrorContext")
            .field("handler", &self.handler)
            .finish()
    }
}

/// Builder for creating ErrorConfig from configuration properties
///
/// This provides a convenient way for factories to extract error configuration
/// during source/sink creation.
pub struct ErrorConfigBuilder {
    config: FlatConfig,
}

impl ErrorConfigBuilder {
    /// Create a new builder from properties
    pub fn from_properties(properties: &std::collections::HashMap<String, String>) -> Self {
        use crate::core::config::PropertySource;

        let mut config = FlatConfig::new();
        for (key, value) in properties {
            if key.starts_with("error.") {
                config.set(key.clone(), value.clone(), PropertySource::SqlWith);
            }
        }

        Self { config }
    }

    /// Check if error handling is configured
    pub fn is_configured(&self) -> bool {
        self.config.contains("error.strategy")
    }

    /// Build ErrorConfig (returns None if not configured)
    pub fn build(&self) -> Result<Option<ErrorConfig>, String> {
        if !self.is_configured() {
            return Ok(None);
        }

        let error_config = ErrorConfig::from_flat_config(&self.config)?;
        Ok(Some(error_config))
    }

    /// Build ErrorConfig with default if not configured
    pub fn build_or_default(&self) -> Result<ErrorConfig, String> {
        Ok(self.build()?.unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::{BackoffStrategy, RetryConfig};
    use crate::core::event::AttributeValue;
    use std::collections::HashMap;

    fn create_test_event() -> Event {
        Event::new_with_data(
            123,
            vec![
                AttributeValue::String("test".to_string()),
                AttributeValue::Int(42),
            ],
        )
    }

    #[test]
    fn test_source_error_context_creation() {
        let error_config = ErrorConfig::default();
        let ctx = SourceErrorContext::new(error_config, None, "TestStream".to_string());

        assert_eq!(ctx.error_count(), 0);
    }

    #[test]
    fn test_source_error_context_handle_drop() {
        let error_config = ErrorConfig::default(); // Default is Drop
        let mut ctx = SourceErrorContext::new(error_config, None, "TestStream".to_string());

        let error = EventFluxError::Other("Test error".to_string());

        // Should continue processing (drop)
        assert!(ctx.handle_error(Some(&create_test_event()), &error));
    }

    #[test]
    fn test_source_error_context_handle_fail() {
        use crate::core::error::{ErrorStrategy, FailConfig, LogLevel};

        let error_config = ErrorConfig::new(
            ErrorStrategy::Fail,
            LogLevel::Error,
            None,
            None,
            Some(FailConfig::default()),
        )
        .unwrap();

        let mut ctx = SourceErrorContext::new(error_config, None, "TestStream".to_string());

        let error = EventFluxError::Other("Fatal error".to_string());

        // Should stop processing (fail)
        assert!(!ctx.handle_error(Some(&create_test_event()), &error));
    }

    #[test]
    fn test_source_error_context_reset() {
        use crate::core::error::{ErrorStrategy, LogLevel};

        let retry_config = RetryConfig {
            max_attempts: 3,
            backoff: BackoffStrategy::Fixed,
            initial_delay: std::time::Duration::from_millis(1),
            max_delay: std::time::Duration::from_secs(1),
        };

        let error_config = ErrorConfig::new(
            ErrorStrategy::Retry,
            LogLevel::Warn,
            Some(retry_config),
            None,
            None,
        )
        .unwrap();

        let mut ctx = SourceErrorContext::new(error_config, None, "TestStream".to_string());

        let error = EventFluxError::ConnectionUnavailable {
            message: "Test".to_string(),
            source: None,
        };

        // Trigger error
        ctx.handle_error(Some(&create_test_event()), &error);
        assert_eq!(ctx.error_count(), 1);

        // Reset
        ctx.reset_errors();
        assert_eq!(ctx.error_count(), 0);
    }

    #[test]
    fn test_error_config_builder_empty() {
        let properties = HashMap::new();
        let builder = ErrorConfigBuilder::from_properties(&properties);

        assert!(!builder.is_configured());
        assert!(builder.build().unwrap().is_none());
    }

    #[test]
    fn test_error_config_builder_with_config() {
        let mut properties = HashMap::new();
        properties.insert("error.strategy".to_string(), "drop".to_string());
        properties.insert("error.log-level".to_string(), "warn".to_string());

        let builder = ErrorConfigBuilder::from_properties(&properties);

        assert!(builder.is_configured());

        let config = builder.build().unwrap().unwrap();
        assert_eq!(config.strategy, crate::core::error::ErrorStrategy::Drop);
        assert_eq!(config.log_level, crate::core::error::LogLevel::Warn);
    }

    #[test]
    fn test_error_config_builder_default() {
        let properties = HashMap::new();
        let builder = ErrorConfigBuilder::from_properties(&properties);

        let config = builder.build_or_default().unwrap();
        assert_eq!(config.strategy, crate::core::error::ErrorStrategy::Drop);
    }

    #[test]
    fn test_from_config() {
        use crate::core::config::PropertySource;

        let mut config = FlatConfig::new();
        config.set("error.strategy", "retry", PropertySource::SqlWith);
        config.set("error.retry.max-attempts", "5", PropertySource::SqlWith);

        let ctx = SourceErrorContext::from_config(&config, None, "TestStream".to_string());
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        assert_eq!(ctx.error_count(), 0);
    }
}
