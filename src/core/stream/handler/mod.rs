// SPDX-License-Identifier: MIT OR Apache-2.0

//! # Stream Handler Module
//!
//! Provides lifecycle management for source and sink streams with proper
//! initialization, startup, and shutdown handling.
//!
//! ## Architecture
//!
//! - **SourceStreamHandler**: Manages source streams with mapper integration
//! - **SinkStreamHandler**: Manages sink streams with mapper integration
//! - Both handlers provide start/stop lifecycle control
//!
//! ## Thread Safety
//!
//! All handlers are designed to be used behind Arc for shared ownership across
//! multiple threads during query processing.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::core::exception::EventFluxError;
use crate::core::stream::input::input_handler::InputHandler;
use crate::core::stream::input::mapper::SourceMapper;
use crate::core::stream::input::source::Source;
use crate::core::stream::output::mapper::SinkMapper;
use crate::core::stream::output::sink::Sink;

/// Handler for source streams with lifecycle management
///
/// Manages a source with its associated mapper and input handler.
/// Provides thread-safe start/stop operations.
#[derive(Debug)]
pub struct SourceStreamHandler {
    source: Arc<Mutex<Box<dyn Source>>>,
    mapper: Option<Arc<Mutex<Box<dyn SourceMapper>>>>,
    input_handler: Arc<Mutex<InputHandler>>,
    stream_id: String,
    is_running: AtomicBool,
}

impl SourceStreamHandler {
    /// Create a new source stream handler
    ///
    /// # Arguments
    ///
    /// * `source` - The source implementation
    /// * `mapper` - Optional source mapper for data transformation
    /// * `input_handler` - Input handler for processing events
    /// * `stream_id` - Unique identifier for this stream
    pub fn new(
        source: Box<dyn Source>,
        mapper: Option<Box<dyn SourceMapper>>,
        input_handler: Arc<Mutex<InputHandler>>,
        stream_id: String,
    ) -> Self {
        Self {
            source: Arc::new(Mutex::new(source)),
            mapper: mapper.map(|m| Arc::new(Mutex::new(m))),
            input_handler,
            stream_id,
            is_running: AtomicBool::new(false),
        }
    }

    /// Start the source stream
    ///
    /// Begins processing events from the source. Returns error if already running.
    pub fn start(&self) -> Result<(), EventFluxError> {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return Err(EventFluxError::app_runtime(format!(
                "Source '{}' is already running",
                self.stream_id
            )));
        }

        self.source
            .lock()
            .unwrap()
            .start(Arc::clone(&self.input_handler));
        Ok(())
    }

    /// Stop the source stream
    ///
    /// Gracefully stops event processing from the source.
    pub fn stop(&self) {
        if self.is_running.swap(false, Ordering::SeqCst) {
            self.source.lock().unwrap().stop();
        }
    }

    /// Check if the source is currently running
    #[inline]
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Get the stream identifier
    #[inline]
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }

    /// Get reference to the source
    #[inline]
    pub fn source(&self) -> Arc<Mutex<Box<dyn Source>>> {
        Arc::clone(&self.source)
    }

    /// Get reference to the mapper (if any)
    #[inline]
    pub fn mapper(&self) -> Option<Arc<Mutex<Box<dyn SourceMapper>>>> {
        self.mapper.as_ref().map(Arc::clone)
    }

    /// Get reference to the input handler
    #[inline]
    pub fn input_handler(&self) -> Arc<Mutex<InputHandler>> {
        Arc::clone(&self.input_handler)
    }
}

/// Handler for sink streams with lifecycle management
///
/// Manages a sink with its associated mapper.
/// Provides thread-safe start/stop operations.
#[derive(Debug)]
pub struct SinkStreamHandler {
    sink: Arc<Mutex<Box<dyn Sink>>>,
    mapper: Option<Arc<Mutex<Box<dyn SinkMapper>>>>,
    stream_id: String,
    is_running: AtomicBool,
}

impl SinkStreamHandler {
    /// Create a new sink stream handler
    ///
    /// # Arguments
    ///
    /// * `sink` - The sink implementation
    /// * `mapper` - Optional sink mapper for data transformation
    /// * `stream_id` - Unique identifier for this stream
    pub fn new(
        sink: Box<dyn Sink>,
        mapper: Option<Box<dyn SinkMapper>>,
        stream_id: String,
    ) -> Self {
        Self {
            sink: Arc::new(Mutex::new(sink)),
            mapper: mapper.map(|m| Arc::new(Mutex::new(m))),
            stream_id,
            is_running: AtomicBool::new(false),
        }
    }

    /// Start the sink stream
    ///
    /// Begins accepting events at the sink.
    pub fn start(&self) {
        if !self.is_running.swap(true, Ordering::SeqCst) {
            self.sink.lock().unwrap().start();
        }
    }

    /// Stop the sink stream
    ///
    /// Gracefully stops the sink and flushes any pending events.
    pub fn stop(&self) {
        if self.is_running.swap(false, Ordering::SeqCst) {
            self.sink.lock().unwrap().stop();
        }
    }

    /// Check if the sink is currently running
    #[inline]
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Get the stream identifier
    #[inline]
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }

    /// Get reference to the sink
    #[inline]
    pub fn sink(&self) -> Arc<Mutex<Box<dyn Sink>>> {
        Arc::clone(&self.sink)
    }

    /// Get reference to the mapper (if any)
    #[inline]
    pub fn mapper(&self) -> Option<Arc<Mutex<Box<dyn SinkMapper>>>> {
        self.mapper.as_ref().map(Arc::clone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::eventflux_app_context::EventFluxAppContext;
    use crate::core::event::event::Event;
    use crate::core::stream::input::input_handler::InputProcessor;
    use crate::core::stream::input::source::timer_source::TimerSource;
    use crate::core::stream::output::sink::log_sink::LogSink;

    /// Mock InputProcessor for testing
    #[derive(Debug)]
    struct MockInputProcessor;

    impl InputProcessor for MockInputProcessor {
        fn send_event_with_data(
            &mut self,
            _timestamp: i64,
            _data: Vec<crate::core::event::value::AttributeValue>,
            _stream_index: usize,
        ) -> Result<(), String> {
            Ok(())
        }

        fn send_single_event(&mut self, _event: Event, _stream_index: usize) -> Result<(), String> {
            Ok(())
        }

        fn send_multiple_events(
            &mut self,
            _events: Vec<Event>,
            _stream_index: usize,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    /// Helper to create a test InputHandler
    fn create_test_input_handler(stream_id: String) -> Arc<Mutex<InputHandler>> {
        use crate::core::config::eventflux_context::EventFluxContext;
        use crate::query_api::eventflux_app::EventFluxApp;

        // Create minimal EventFluxContext
        let eventflux_context = Arc::new(EventFluxContext::default());

        // Create minimal EventFluxApp
        let eventflux_app = Arc::new(EventFluxApp::default());

        // Create EventFluxAppContext
        let app_context = Arc::new(EventFluxAppContext::new(
            eventflux_context,
            "TestApp".to_string(),
            eventflux_app,
            String::new(), // empty app string for tests
        ));

        let processor: Arc<Mutex<dyn InputProcessor>> = Arc::new(Mutex::new(MockInputProcessor));
        Arc::new(Mutex::new(InputHandler::new(
            stream_id,
            0,
            processor,
            app_context,
        )))
    }

    #[test]
    fn test_source_handler_creation() {
        let source = Box::new(TimerSource::new(100));
        let input_handler = create_test_input_handler("TestStream".to_string());

        let handler =
            SourceStreamHandler::new(source, None, input_handler, "TestStream".to_string());

        assert_eq!(handler.stream_id(), "TestStream");
        assert!(!handler.is_running());
    }

    #[test]
    fn test_source_handler_start_stop() {
        let source = Box::new(TimerSource::new(100));
        let input_handler = create_test_input_handler("TestStream".to_string());

        let handler =
            SourceStreamHandler::new(source, None, input_handler, "TestStream".to_string());

        assert!(handler.start().is_ok());
        assert!(handler.is_running());

        // Starting again should fail
        assert!(handler.start().is_err());

        handler.stop();
        assert!(!handler.is_running());
    }

    #[test]
    fn test_sink_handler_creation() {
        let sink = Box::new(LogSink::new());

        let handler = SinkStreamHandler::new(sink, None, "TestSink".to_string());

        assert_eq!(handler.stream_id(), "TestSink");
        assert!(!handler.is_running());
    }

    #[test]
    fn test_sink_handler_start_stop() {
        let sink = Box::new(LogSink::new());

        let handler = SinkStreamHandler::new(sink, None, "TestSink".to_string());

        handler.start();
        assert!(handler.is_running());

        handler.stop();
        assert!(!handler.is_running());
    }
}
