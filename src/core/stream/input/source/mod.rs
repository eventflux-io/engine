// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod timer_source;

use crate::core::stream::input::input_handler::InputHandler;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

pub trait Source: Debug + Send + Sync {
    fn start(&mut self, handler: Arc<Mutex<InputHandler>>);
    fn stop(&mut self);
    fn clone_box(&self) -> Box<dyn Source>;

    /// Phase 2 validation: Verify connectivity and external resource availability
    ///
    /// This method is called during application initialization (Phase 2) to validate
    /// that external systems are reachable and properly configured.
    ///
    /// **Fail-Fast Principle**: Application should NOT start if transports are not ready.
    ///
    /// # Default Implementation
    ///
    /// Returns Ok by default - sources that don't need external validation can use this.
    /// Sources with external dependencies (Kafka, HTTP, etc.) MUST override this method.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - External system is reachable and properly configured
    /// * `Err(EventFluxError)` - Validation failed, application should not start
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Kafka source validates broker connectivity
    /// fn validate_connectivity(&self) -> Result<(), EventFluxError> {
    ///     // 1. Validate brokers are reachable
    ///     let metadata = self.consumer.fetch_metadata(None, Duration::from_secs(10))?;
    ///
    ///     // 2. Validate topic exists
    ///     if !metadata.topics().iter().any(|t| t.name() == self.topic) {
    ///         return Err(EventFluxError::configuration(
    ///             format!("Topic '{}' does not exist", self.topic)
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

impl Clone for Box<dyn Source> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
