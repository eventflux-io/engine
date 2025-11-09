// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::core::config::{ConfigValue, ProcessorConfigReader};
use crate::core::event::event::Event;
use crate::core::exception::EventFluxError;
use crate::core::stream::output::mapper::PassthroughMapper;
use crate::core::stream::output::sink::sink_trait::Sink;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

/// LogSink - Debug sink that logs events to stdout/log system
///
/// This sink deserializes binary event payloads and logs them for debugging.
/// Used primarily for development and testing.
#[derive(Debug, Clone)]
pub struct LogSink {
    pub events: Arc<Mutex<Vec<Event>>>,
    pub config_reader: Option<Arc<ProcessorConfigReader>>,
    pub sink_name: String,
}

impl Default for LogSink {
    fn default() -> Self {
        Self::new()
    }
}

impl LogSink {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            config_reader: None,
            sink_name: "log".to_string(),
        }
    }

    /// Create a new LogSink with configuration reader support
    pub fn new_with_config(
        config_reader: Option<Arc<ProcessorConfigReader>>,
        sink_name: String,
    ) -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            config_reader,
            sink_name,
        }
    }

    /// Get the configured prefix for this sink
    fn get_prefix(&self) -> String {
        if let Some(ref config_reader) = self.config_reader {
            if let Some(ConfigValue::String(prefix)) =
                config_reader.get_sink_config(&self.sink_name, "prefix")
            {
                return prefix;
            }
        }
        "[LOG]".to_string() // Default prefix
    }
}

impl Sink for LogSink {
    fn publish(&self, payload: &[u8]) -> Result<(), EventFluxError> {
        // Deserialize events from binary format
        let events = PassthroughMapper::deserialize(payload)
            .map_err(|e| EventFluxError::app_runtime(format!("LogSink deserialization failed: {}", e)))?;

        let prefix = self.get_prefix();
        for e in &events {
            log::info!("{} {:?}", prefix, e);
            self.events.lock().unwrap().push(e.clone());
        }

        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Sink> {
        Box::new(self.clone())
    }
}
