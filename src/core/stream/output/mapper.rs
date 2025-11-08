// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::core::event::event::Event;
use std::fmt::Debug;

pub trait SinkMapper: Debug + Send + Sync {
    fn map(&self, events: &[Event]) -> Vec<u8>;
    fn clone_box(&self) -> Box<dyn SinkMapper>;
}

impl Clone for Box<dyn SinkMapper> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Default passthrough mapper for Events
///
/// When no format is specified (no JSON/CSV/XML mapper), this mapper
/// serializes Events to an efficient binary format using bincode.
///
/// This is used for:
/// - Debug sinks like LogSink that need to deserialize back to Events
/// - Internal event passing where no external format is needed
///
/// # Example
///
/// ```ignore
/// let mapper = PassthroughMapper::new();
/// let bytes = mapper.map(&events);
/// let recovered: Vec<Event> = PassthroughMapper::deserialize(&bytes)?;
/// ```
#[derive(Debug, Clone)]
pub struct PassthroughMapper;

impl PassthroughMapper {
    pub fn new() -> Self {
        Self
    }

    /// Deserialize bytes back to Events
    ///
    /// Used by debug sinks that need to recover Events from binary format.
    pub fn deserialize(bytes: &[u8]) -> Result<Vec<Event>, String> {
        bincode::deserialize(bytes).map_err(|e| format!("Failed to deserialize events: {}", e))
    }
}

impl Default for PassthroughMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl SinkMapper for PassthroughMapper {
    fn map(&self, events: &[Event]) -> Vec<u8> {
        // Use bincode for efficient binary serialization
        bincode::serialize(events).unwrap_or_else(|e| {
            log::error!("Failed to serialize events: {}", e);
            vec![]
        })
    }

    fn clone_box(&self) -> Box<dyn SinkMapper> {
        Box::new(self.clone())
    }
}
