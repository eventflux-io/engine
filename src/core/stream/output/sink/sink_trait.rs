// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::core::stream::output::stream_callback::StreamCallback;
use std::fmt::Debug;

pub trait Sink: StreamCallback + Debug + Send + Sync {
    fn start(&self) {}
    fn stop(&self) {}
    fn clone_box(&self) -> Box<dyn Sink>;

    /// Phase 2 validation: Verify connectivity and external resource availability
    ///
    /// This method is called during application initialization (Phase 2) to validate
    /// that external systems are reachable and properly configured.
    ///
    /// **Fail-Fast Principle**: Application should NOT start if transports are not ready.
    ///
    /// # Default Implementation
    ///
    /// Returns Ok by default - sinks that don't need external validation can use this.
    /// Sinks with external dependencies (Kafka, HTTP, databases) MUST override this method.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - External system is reachable and properly configured
    /// * `Err(EventFluxError)` - Validation failed, application should not start
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // HTTP sink validates URL reachability
    /// fn validate_connectivity(&self) -> Result<(), EventFluxError> {
    ///     // 1. Validate URL is reachable (simple HEAD request)
    ///     let client = reqwest::blocking::Client::new();
    ///     let response = client.head(&self.url)
    ///         .timeout(Duration::from_secs(10))
    ///         .send()?;
    ///
    ///     if !response.status().is_success() && !response.status().is_client_error() {
    ///         return Err(EventFluxError::configuration(
    ///             format!("HTTP endpoint '{}' not reachable: {}", self.url, response.status())
    ///         ));
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    fn validate_connectivity(&self) -> Result<(), crate::core::exception::EventFluxError> {
        Ok(()) // Default: no validation needed
    }
}

impl Clone for Box<dyn Sink> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
