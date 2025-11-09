// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::core::event::event::Event;
use std::fmt::Debug;

pub trait SourceMapper: Debug + Send + Sync {
    fn map(&self, input: &[u8]) -> Vec<Event>;
    fn clone_box(&self) -> Box<dyn SourceMapper>;
}

impl Clone for Box<dyn SourceMapper> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Default passthrough mapper for Sources
///
/// When no format is specified (no JSON/CSV/XML mapper), this mapper
/// deserializes Events from an efficient binary format using bincode.
///
/// This is the inverse of output::mapper::PassthroughMapper and is used for:
/// - Debug sources like TimerSource that produce Events directly
/// - Internal event passing where no external format is needed
///
/// # Example
///
/// ```ignore
/// let mapper = PassthroughMapper::new();
/// let bytes = bincode::serialize(&events)?;
/// let recovered: Vec<Event> = mapper.map(&bytes);
/// ```
#[derive(Debug, Clone)]
pub struct PassthroughMapper;

impl PassthroughMapper {
    pub fn new() -> Self {
        Self
    }

    /// Serialize Events to bytes
    ///
    /// Used by sources that generate Events internally and need to convert
    /// them to binary format for the mapper pipeline.
    pub fn serialize(events: &[Event]) -> Result<Vec<u8>, String> {
        bincode::serialize(events).map_err(|e| format!("Failed to serialize events: {}", e))
    }
}

impl Default for PassthroughMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceMapper for PassthroughMapper {
    fn map(&self, input: &[u8]) -> Vec<Event> {
        // Use bincode for efficient binary deserialization
        bincode::deserialize(input).unwrap_or_else(|e| {
            log::error!("Failed to deserialize events: {}", e);
            vec![]
        })
    }

    fn clone_box(&self) -> Box<dyn SourceMapper> {
        Box::new(self.clone())
    }
}
