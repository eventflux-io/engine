// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::core::config::{ConfigValue, ProcessorConfigReader};
use crate::core::event::event::Event;
use crate::core::stream::output::sink::sink_trait::Sink;
use crate::core::stream::output::stream_callback::StreamCallback;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

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

impl StreamCallback for LogSink {
    fn receive_events(&self, events: &[Event]) {
        let prefix = self.get_prefix();
        for e in events {
            log::info!("{} {e:?}", prefix);
            self.events.lock().unwrap().push(e.clone());
        }
    }
}

impl Sink for LogSink {
    fn clone_box(&self) -> Box<dyn Sink> {
        Box::new(self.clone())
    }
}
