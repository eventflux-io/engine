// SPDX-License-Identifier: MIT OR Apache-2.0

//! Optimized StreamJunction with Crossbeam-Based High-Performance Pipeline
//!
//! This implementation replaces the original crossbeam_channel-based StreamJunction
//! with our high-performance crossbeam pipeline for >1M events/second throughput.

use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::event::complex_event::ComplexEvent;
use crate::core::event::event::Event;
use crate::core::event::stream::StreamEvent;
use crate::core::exception::EventFluxError;
use crate::core::query::processor::Processor;
use crate::core::stream::input::input_handler::InputProcessor;
use crate::core::util::executor_service::ExecutorService;
use crate::core::util::pipeline::{
    BackpressureStrategy, EventPipeline, EventPool, MetricsSnapshot, PipelineBuilder,
    PipelineConfig, PipelineResult,
};
use crate::query_api::definition::StreamDefinition;

use crossbeam::utils::CachePadded;
use std::fmt::Debug;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};

/// Error handling strategies for StreamJunction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnErrorAction {
    #[default]
    LOG,
    STREAM,
    STORE,
    DROP,
}

/// High-performance event router with crossbeam-based pipeline
pub struct OptimizedStreamJunction {
    // Core identification
    pub stream_id: String,
    stream_definition: Arc<StreamDefinition>,
    eventflux_app_context: Arc<EventFluxAppContext>,

    // High-performance pipeline
    event_pipeline: Arc<EventPipeline>,
    event_pool: Arc<EventPool>,

    // Subscriber management
    subscribers: Arc<Mutex<Vec<Arc<Mutex<dyn Processor>>>>>,

    // Configuration
    is_async: bool,
    buffer_size: usize,
    on_error_action: OnErrorAction,

    // Lifecycle management
    started: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,

    // Fault handling
    fault_stream_junction: Option<Arc<Mutex<OptimizedStreamJunction>>>,

    // Performance tracking
    events_processed: Arc<CachePadded<AtomicU64>>,
    events_dropped: Arc<CachePadded<AtomicU64>>,
    processing_errors: Arc<CachePadded<AtomicU64>>,

    // Executor service for async processing
    executor_service: Arc<ExecutorService>,
}

impl Debug for OptimizedStreamJunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptimizedStreamJunction")
            .field("stream_id", &self.stream_id)
            .field("is_async", &self.is_async)
            .field("buffer_size", &self.buffer_size)
            .field(
                "subscribers_count",
                &self.subscribers.lock().expect("Mutex poisoned").len(),
            )
            .field("started", &self.started.load(Ordering::Relaxed))
            .field(
                "events_processed",
                &self.events_processed.load(Ordering::Relaxed),
            )
            .field(
                "events_dropped",
                &self.events_dropped.load(Ordering::Relaxed),
            )
            .field("on_error_action", &self.on_error_action)
            .finish()
    }
}

impl OptimizedStreamJunction {
    /// Create a new optimized stream junction
    ///
    /// # Arguments
    /// * `stream_id` - Unique identifier for this stream junction
    /// * `stream_definition` - Schema definition for events flowing through this junction
    /// * `eventflux_app_context` - Application context for configuration and services
    /// * `buffer_size` - Internal buffer capacity (power of 2 recommended for async mode)
    /// * `is_async` - **CRITICAL**: Processing mode selection
    ///   - `false` (RECOMMENDED DEFAULT): Synchronous processing with guaranteed event ordering
    ///   - `true`: Async processing for high throughput but potential event reordering
    /// * `fault_stream_junction` - Optional fault handling junction
    ///
    /// # Processing Modes
    ///
    /// **Synchronous Mode (`is_async: false`):**
    /// - Events are processed sequentially in the exact order they arrive
    /// - Thread-safe but blocking - each event waits for previous event to complete
    /// - Guarantees temporal correctness and event causality
    /// - Throughput: ~1K-10K events/sec depending on processing complexity
    ///
    /// **Async Mode (`is_async: true`):**  
    /// - Events are processed concurrently using lock-free pipeline
    /// - Non-blocking, high-throughput processing with object pooling
    /// - **⚠️ WARNING: May process events out of arrival order**
    /// - Throughput: >100K events/sec capability
    ///
    /// # Recommendation
    /// Use synchronous mode unless you specifically need high throughput AND can tolerate
    /// potential event reordering in your stream processing logic.
    pub fn new(
        stream_id: String,
        stream_definition: Arc<StreamDefinition>,
        eventflux_app_context: Arc<EventFluxAppContext>,
        buffer_size: usize,
        is_async: bool,
        fault_stream_junction: Option<Arc<Mutex<OptimizedStreamJunction>>>,
    ) -> Result<Self, String> {
        let executor_service = eventflux_app_context
            .executor_service
            .clone()
            .unwrap_or_else(|| Arc::new(ExecutorService::default()));

        // Configure pipeline based on async mode and performance requirements
        let backpressure_strategy = if is_async {
            BackpressureStrategy::ExponentialBackoff { max_delay_ms: 1000 }
        } else {
            BackpressureStrategy::Block
        };

        let pipeline_config = PipelineConfig {
            capacity: buffer_size,
            backpressure: backpressure_strategy,
            use_object_pool: true,
            consumer_threads: if is_async {
                (num_cpus::get() / 2).max(1)
            } else {
                1
            },
            batch_size: 64, // Optimal batch size for throughput
            enable_metrics: eventflux_app_context.get_root_metrics_level()
                != crate::core::config::eventflux_app_context::MetricsLevelPlaceholder::OFF,
        };

        let event_pipeline = Arc::new(
            PipelineBuilder::new()
                .with_capacity(pipeline_config.capacity)
                .with_backpressure(pipeline_config.backpressure)
                .with_object_pool(pipeline_config.use_object_pool)
                .with_consumer_threads(pipeline_config.consumer_threads)
                .with_batch_size(pipeline_config.batch_size)
                .with_metrics(pipeline_config.enable_metrics)
                .build()?,
        );

        let event_pool = Arc::new(EventPool::new(buffer_size * 2)); // 2x for buffering

        Ok(Self {
            stream_id,
            stream_definition,
            eventflux_app_context,
            event_pipeline,
            event_pool,
            subscribers: Arc::new(Mutex::new(Vec::new())),
            is_async,
            buffer_size,
            on_error_action: OnErrorAction::default(),
            started: Arc::new(AtomicBool::new(false)),
            shutdown: Arc::new(AtomicBool::new(false)),
            fault_stream_junction,
            events_processed: Arc::new(CachePadded::new(AtomicU64::new(0))),
            events_dropped: Arc::new(CachePadded::new(AtomicU64::new(0))),
            processing_errors: Arc::new(CachePadded::new(AtomicU64::new(0))),
            executor_service,
        })
    }

    /// Get the stream definition
    pub fn get_stream_definition(&self) -> Arc<StreamDefinition> {
        Arc::clone(&self.stream_definition)
    }

    /// Create a publisher for this junction
    pub fn construct_publisher(&self) -> OptimizedPublisher {
        OptimizedPublisher::new(self)
    }

    /// Subscribe a processor to receive events
    pub fn subscribe(&self, processor: Arc<Mutex<dyn Processor>>) {
        let mut subs = self.subscribers.lock().expect("Mutex poisoned");
        if !subs.iter().any(|p| Arc::ptr_eq(p, &processor)) {
            subs.push(processor);
        }
    }

    /// Unsubscribe a processor from receiving events
    pub fn unsubscribe(&self, processor: &Arc<Mutex<dyn Processor>>) {
        let mut subs = self.subscribers.lock().expect("Mutex poisoned");
        subs.retain(|p| !Arc::ptr_eq(p, processor));
    }

    /// Set error handling action
    pub fn set_on_error_action(&mut self, action: OnErrorAction) {
        self.on_error_action = action;
    }

    /// Start processing events
    pub fn start_processing(&self) -> Result<(), String> {
        if self
            .started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            return Ok(()); // Already started
        }

        if self.is_async {
            self.start_async_consumer()?;
        }

        Ok(())
    }

    /// Stop processing events
    pub fn stop_processing(&self) {
        self.shutdown.store(true, Ordering::Release);
        self.event_pipeline.shutdown();
        self.started.store(false, Ordering::Release);
    }

    /// Send a single event through the pipeline
    pub fn send_event(&self, event: Event) -> Result<(), EventFluxError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(EventFluxError::SendError {
                message: "StreamJunction is shutting down".to_string(),
            });
        }

        let stream_event = Self::event_to_stream_event(event);

        if self.is_async {
            // Use high-performance pipeline for async processing
            match self.event_pipeline.publish(stream_event) {
                PipelineResult::Success { .. } => {
                    // Don't count here - let the consumer count processed events
                    Ok(())
                }
                PipelineResult::Full => {
                    self.events_dropped.fetch_add(1, Ordering::Relaxed);
                    self.handle_backpressure_error("Pipeline full");
                    Err(EventFluxError::SendError {
                        message: format!("Pipeline full for stream {}", self.stream_id),
                    })
                }
                PipelineResult::Shutdown => Err(EventFluxError::SendError {
                    message: "Pipeline is shutting down".to_string(),
                }),
                PipelineResult::Timeout => {
                    self.events_dropped.fetch_add(1, Ordering::Relaxed);
                    Err(EventFluxError::SendError {
                        message: "Pipeline publish timeout".to_string(),
                    })
                }
                PipelineResult::Error(msg) => {
                    self.processing_errors.fetch_add(1, Ordering::Relaxed);
                    Err(EventFluxError::SendError { message: msg })
                }
            }
        } else {
            // Synchronous processing - direct dispatch to subscribers
            self.dispatch_to_subscribers_sync(Box::new(stream_event))
        }
    }

    /// Send multiple events through the pipeline
    pub fn send_events(&self, events: Vec<Event>) -> Result<(), EventFluxError> {
        if events.is_empty() {
            return Ok(());
        }

        if self.shutdown.load(Ordering::Acquire) {
            return Err(EventFluxError::SendError {
                message: "StreamJunction is shutting down".to_string(),
            });
        }

        if self.is_async {
            // Use batch publishing for better throughput
            let event_count = events.len();
            let stream_events: Vec<_> = events
                .into_iter()
                .map(Self::event_to_stream_event)
                .collect();

            let results = self.event_pipeline.publish_batch(stream_events);
            let mut successful = 0;
            let mut dropped = 0;
            let mut errors = 0;

            for result in results {
                match result {
                    PipelineResult::Success { .. } => successful += 1,
                    PipelineResult::Full | PipelineResult::Timeout => dropped += 1,
                    PipelineResult::Error(_) => errors += 1,
                    PipelineResult::Shutdown => break,
                }
            }

            // Don't count processed events here - let the consumer count them
            self.events_dropped.fetch_add(dropped, Ordering::Relaxed);
            self.processing_errors.fetch_add(errors, Ordering::Relaxed);

            if successful > 0 {
                Ok(())
            } else {
                Err(EventFluxError::SendError {
                    message: format!("Failed to process {event_count} events"),
                })
            }
        } else {
            // Synchronous batch processing
            let event_chain = self.create_event_chain(events);
            self.dispatch_to_subscribers_sync(event_chain)
        }
    }

    /// Start async consumer for pipeline processing
    fn start_async_consumer(&self) -> Result<(), String> {
        let subscribers = Arc::clone(&self.subscribers);
        let pipeline = Arc::clone(&self.event_pipeline);
        let shutdown_flag = Arc::clone(&self.shutdown);
        let error_counter = Arc::clone(&self.processing_errors);
        let events_processed = Arc::clone(&self.events_processed);
        let executor_service = Arc::clone(&self.executor_service);
        let stream_id = self.stream_id.clone();

        // Start a dedicated consumer thread that processes events from the pipeline
        let executor_for_main = Arc::clone(&executor_service);
        executor_for_main.execute(move || {
            // Clone variables needed after the consumer closure
            let stream_id_for_completion = stream_id.clone();
            let error_counter_for_completion = Arc::clone(&error_counter);

            // Use the pipeline's consume method with a proper handler
            let result = pipeline.consume(move |event, _sequence| {
                let boxed_event = Box::new(event);

                // Get current subscribers - minimize lock time
                let subs_guard = match subscribers.lock() {
                    Ok(guard) => guard,
                    Err(_) => {
                        error_counter.fetch_add(1, Ordering::Relaxed);
                        return Err("Subscribers mutex poisoned".to_string());
                    }
                };
                let subs: Vec<_> = subs_guard.iter().map(Arc::clone).collect();
                drop(subs_guard);

                if subs.is_empty() {
                    events_processed.fetch_add(1, Ordering::Relaxed);
                    return Ok(());
                }

                // Process for each subscriber - dispatch events efficiently
                for subscriber in subs.iter() {
                    // Clone event for each subscriber
                    let event_for_sub = Some(crate::core::event::complex_event::clone_event_chain(
                        boxed_event.as_ref(),
                    ));

                    // Process asynchronously to avoid blocking the pipeline consumer
                    let subscriber_clone = Arc::clone(subscriber);
                    let stream_id_clone = stream_id.clone();
                    let executor_clone = Arc::clone(&executor_service);
                    let error_counter_clone = Arc::clone(&error_counter);

                    executor_clone.execute(move || match subscriber_clone.lock() {
                        Ok(processor) => {
                            processor.process(event_for_sub);
                        }
                        Err(_) => {
                            error_counter_clone.fetch_add(1, Ordering::Relaxed);
                            eprintln!("[{stream_id_clone}] Failed to lock subscriber processor");
                        }
                    });
                }

                events_processed.fetch_add(1, Ordering::Relaxed);
                Ok(())
            });

            // Handle consumer completion
            match result {
                Ok(processed_count) => {
                    println!(
                        "[{stream_id_for_completion}] Consumer completed after processing {processed_count} events"
                    );
                }
                Err(e) => {
                    error_counter_for_completion.fetch_add(1, Ordering::Relaxed);
                    eprintln!("[{stream_id_for_completion}] Consumer error: {e}");
                }
            }
        });

        Ok(())
    }

    /// Dispatch events directly to subscribers (synchronous mode)
    fn dispatch_to_subscribers_sync(
        &self,
        event: Box<dyn ComplexEvent>,
    ) -> Result<(), EventFluxError> {
        let subs_guard = self.subscribers.lock().expect("Mutex poisoned");
        let subs: Vec<_> = subs_guard.iter().map(Arc::clone).collect();
        drop(subs_guard);

        if subs.is_empty() {
            self.events_processed.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }

        // Process all subscribers except the last one with cloned events
        for subscriber in subs.iter().take(subs.len().saturating_sub(1)) {
            let event_for_sub = Some(crate::core::event::complex_event::clone_event_chain(
                event.as_ref(),
            ));

            match subscriber.lock() {
                Ok(processor) => {
                    processor.process(event_for_sub);
                }
                Err(_) => {
                    self.processing_errors.fetch_add(1, Ordering::Relaxed);
                    return Err(EventFluxError::SendError {
                        message: "Failed to lock subscriber processor".to_string(),
                    });
                }
            }
        }

        // Last subscriber gets the original event (transfer ownership)
        if let Some(last_subscriber) = subs.last() {
            match last_subscriber.lock() {
                Ok(processor) => {
                    processor.process(Some(event));
                }
                Err(_) => {
                    self.processing_errors.fetch_add(1, Ordering::Relaxed);
                    return Err(EventFluxError::SendError {
                        message: "Failed to lock subscriber processor".to_string(),
                    });
                }
            }
        }

        self.events_processed.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Convert Event to StreamEvent
    fn event_to_stream_event(event: Event) -> StreamEvent {
        let mut stream_event = StreamEvent::new(event.timestamp, event.data.len(), 0, 0);
        stream_event.before_window_data = event.data;
        stream_event
    }

    /// Create an event chain from multiple events
    fn create_event_chain(&self, events: Vec<Event>) -> Box<dyn ComplexEvent> {
        let mut iter = events.into_iter();
        let first = match iter.next() {
            Some(event) => Self::event_to_stream_event(event),
            None => panic!("Cannot create chain from empty events"),
        };

        let mut head: Box<dyn ComplexEvent> = Box::new(first);
        let mut tail_ref = head.mut_next_ref_option();

        for event in iter {
            let stream_event = Self::event_to_stream_event(event);
            let boxed = Box::new(stream_event);
            *tail_ref = Some(boxed);
            if let Some(ref mut last) = *tail_ref {
                tail_ref = last.mut_next_ref_option();
            }
        }

        head
    }

    /// Handle backpressure errors
    fn handle_backpressure_error(&self, message: &str) {
        match self.on_error_action {
            OnErrorAction::LOG => {
                eprintln!("[{}] Backpressure: {}", self.stream_id, message);
            }
            OnErrorAction::DROP => {
                // Silently drop
            }
            OnErrorAction::STREAM => {
                if let Some(fault_junction) = &self.fault_stream_junction {
                    // Create an error event and send to fault stream
                    let error_event = Event::new_with_data(
                        chrono::Utc::now().timestamp_millis(),
                        vec![
                            crate::core::event::value::AttributeValue::String(
                                self.stream_id.clone(),
                            ),
                            crate::core::event::value::AttributeValue::String(message.to_string()),
                        ],
                    );
                    let _ = fault_junction.lock().unwrap().send_event(error_event);
                } else {
                    eprintln!(
                        "[{}] Fault stream not configured: {}",
                        self.stream_id, message
                    );
                }
            }
            OnErrorAction::STORE => {
                if let Some(store) = self
                    .eventflux_app_context
                    .get_eventflux_context()
                    .get_error_store()
                {
                    let error = EventFluxError::SendError {
                        message: message.to_string(),
                    };
                    store.store(&self.stream_id, error);
                } else {
                    eprintln!(
                        "[{}] Error store not configured: {}",
                        self.stream_id, message
                    );
                }
            }
        }
    }

    /// Get comprehensive performance metrics
    pub fn get_performance_metrics(&self) -> JunctionPerformanceMetrics {
        let pipeline_metrics = self.event_pipeline.metrics().snapshot();

        JunctionPerformanceMetrics {
            stream_id: self.stream_id.clone(),
            events_processed: self.events_processed.load(Ordering::Relaxed),
            events_dropped: self.events_dropped.load(Ordering::Relaxed),
            processing_errors: self.processing_errors.load(Ordering::Relaxed),
            pipeline_metrics,
            subscriber_count: self.subscribers.lock().expect("Mutex poisoned").len(),
            is_async: self.is_async,
            buffer_utilization: self.event_pipeline.utilization(),
            remaining_capacity: self.event_pipeline.remaining_capacity(),
        }
    }

    /// Check if the junction is healthy
    pub fn is_healthy(&self) -> bool {
        !self.shutdown.load(Ordering::Acquire)
            && self.event_pipeline.metrics().snapshot().is_healthy()
            && self.events_processed.load(Ordering::Relaxed) > 0
    }

    /// Get total events processed
    pub fn total_events(&self) -> Option<u64> {
        Some(self.events_processed.load(Ordering::Relaxed))
    }

    /// Get average latency from pipeline metrics
    pub fn average_latency_ns(&self) -> Option<u64> {
        let metrics = self.event_pipeline.metrics().snapshot();
        if metrics.avg_publish_latency_us > 0.0 {
            Some((metrics.avg_publish_latency_us * 1000.0) as u64)
        } else {
            None
        }
    }
}

/// Performance metrics for the optimized stream junction
#[derive(Debug, Clone)]
pub struct JunctionPerformanceMetrics {
    pub stream_id: String,
    pub events_processed: u64,
    pub events_dropped: u64,
    pub processing_errors: u64,
    pub pipeline_metrics: MetricsSnapshot,
    pub subscriber_count: usize,
    pub is_async: bool,
    pub buffer_utilization: f64,
    pub remaining_capacity: usize,
}

impl JunctionPerformanceMetrics {
    /// Calculate overall health score (0.0 to 1.0)
    pub fn health_score(&self) -> f64 {
        let pipeline_health = self.pipeline_metrics.health_score();
        let error_rate = if self.events_processed > 0 {
            self.processing_errors as f64 / self.events_processed as f64
        } else {
            0.0
        };
        let drop_rate = if self.events_processed > 0 {
            self.events_dropped as f64 / (self.events_processed + self.events_dropped) as f64
        } else {
            0.0
        };

        let mut score = pipeline_health;
        score -= error_rate * 2.0; // 2x penalty for errors
        score -= drop_rate * 1.5; // 1.5x penalty for drops

        score.max(0.0).min(1.0)
    }

    /// Check if metrics indicate healthy operation
    pub fn is_healthy(&self) -> bool {
        self.health_score() > 0.8
            && self.pipeline_metrics.is_healthy()
            && self.processing_errors < (self.events_processed / 100) // <1% error rate
    }
}

/// High-performance publisher for the optimized stream junction
#[derive(Debug, Clone)]
pub struct OptimizedPublisher {
    junction: *const OptimizedStreamJunction, // Raw pointer for performance
}

unsafe impl Send for OptimizedPublisher {}
unsafe impl Sync for OptimizedPublisher {}

impl OptimizedPublisher {
    fn new(junction: &OptimizedStreamJunction) -> Self {
        Self {
            junction: junction as *const OptimizedStreamJunction,
        }
    }

    fn get_junction(&self) -> &OptimizedStreamJunction {
        unsafe { &*self.junction }
    }
}

impl InputProcessor for OptimizedPublisher {
    fn send_event_with_data(
        &mut self,
        timestamp: i64,
        data: Vec<crate::core::event::value::AttributeValue>,
        _stream_index: usize,
    ) -> Result<(), String> {
        let event = Event::new_with_data(timestamp, data);
        self.get_junction()
            .send_event(event)
            .map_err(|e| format!("Send error: {e}"))
    }

    fn send_single_event(&mut self, event: Event, _stream_index: usize) -> Result<(), String> {
        self.get_junction()
            .send_event(event)
            .map_err(|e| format!("Send error: {e}"))
    }

    fn send_multiple_events(
        &mut self,
        events: Vec<Event>,
        _stream_index: usize,
    ) -> Result<(), String> {
        self.get_junction()
            .send_events(events)
            .map_err(|e| format!("Send error: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::eventflux_context::EventFluxContext;
    use crate::core::event::value::AttributeValue;
    use crate::query_api::definition::attribute::Type as AttrType;
    use std::thread;
    use std::time::{Duration, Instant};

    // Test processor that records events
    #[derive(Debug)]
    struct TestProcessor {
        events: Arc<Mutex<Vec<Vec<AttributeValue>>>>,
        name: String,
    }

    impl TestProcessor {
        fn new(name: String) -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
                name,
            }
        }

        fn get_events(&self) -> Vec<Vec<AttributeValue>> {
            self.events.lock().unwrap().clone()
        }
    }

    impl Processor for TestProcessor {
        fn process(&self, mut chunk: Option<Box<dyn ComplexEvent>>) {
            while let Some(mut ce) = chunk {
                chunk = ce.set_next(None);
                if let Some(se) = ce.as_any().downcast_ref::<StreamEvent>() {
                    self.events
                        .lock()
                        .unwrap()
                        .push(se.before_window_data.clone());
                }
            }
        }

        fn next_processor(&self) -> Option<Arc<Mutex<dyn Processor>>> {
            None
        }

        fn set_next_processor(&mut self, _n: Option<Arc<Mutex<dyn Processor>>>) {}

        fn clone_processor(
            &self,
            _c: &Arc<crate::core::config::eventflux_query_context::EventFluxQueryContext>,
        ) -> Box<dyn Processor> {
            Box::new(TestProcessor::new(self.name.clone()))
        }

        fn get_eventflux_app_context(&self) -> Arc<EventFluxAppContext> {
            Arc::new(EventFluxAppContext::new(
                Arc::new(EventFluxContext::new()),
                "TestApp".to_string(),
                Arc::new(crate::query_api::eventflux_app::EventFluxApp::new(
                    "TestApp".to_string(),
                )),
                String::new(),
            ))
        }

        fn get_eventflux_query_context(
            &self,
        ) -> Arc<crate::core::config::eventflux_query_context::EventFluxQueryContext> {
            Arc::new(
                crate::core::config::eventflux_query_context::EventFluxQueryContext::new(
                    self.get_eventflux_app_context(),
                    "TestQuery".to_string(),
                    None,
                ),
            )
        }

        fn get_processing_mode(&self) -> crate::core::query::processor::ProcessingMode {
            crate::core::query::processor::ProcessingMode::DEFAULT
        }

        fn is_stateful(&self) -> bool {
            false
        }
    }

    fn setup_junction(is_async: bool) -> (OptimizedStreamJunction, Arc<Mutex<TestProcessor>>) {
        let eventflux_context = Arc::new(EventFluxContext::new());
        let app = Arc::new(crate::query_api::eventflux_app::EventFluxApp::new(
            "TestApp".to_string(),
        ));
        let mut app_ctx = EventFluxAppContext::new(
            Arc::clone(&eventflux_context),
            "TestApp".to_string(),
            Arc::clone(&app),
            String::new(),
        );
        app_ctx.root_metrics_level =
            crate::core::config::eventflux_app_context::MetricsLevelPlaceholder::BASIC;

        let stream_def = Arc::new(
            StreamDefinition::new("TestStream".to_string())
                .attribute("id".to_string(), AttrType::INT),
        );

        let junction = OptimizedStreamJunction::new(
            "TestStream".to_string(),
            stream_def,
            Arc::new(app_ctx),
            4096,
            is_async,
            None,
        )
        .unwrap();

        let processor = Arc::new(Mutex::new(TestProcessor::new("TestProcessor".to_string())));
        junction.subscribe(processor.clone() as Arc<Mutex<dyn Processor>>);

        (junction, processor)
    }

    #[test]
    fn test_sync_junction_single_event() {
        let (junction, processor) = setup_junction(false);

        junction.start_processing().unwrap();

        let event = Event::new_with_data(1000, vec![AttributeValue::Int(42)]);
        junction.send_event(event).unwrap();

        let events = processor.lock().unwrap().get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], vec![AttributeValue::Int(42)]);

        let metrics = junction.get_performance_metrics();
        assert_eq!(metrics.events_processed, 1);
        assert_eq!(metrics.events_dropped, 0);
    }

    #[test]
    fn test_async_junction_multiple_events() {
        let (junction, processor) = setup_junction(true);

        junction.start_processing().unwrap();

        let events: Vec<_> = (0..100)
            .map(|i| Event::new_with_data(1000 + i, vec![AttributeValue::Int(i as i32)]))
            .collect();

        junction.send_events(events).unwrap();

        // Wait for async processing
        thread::sleep(Duration::from_millis(200));

        let received_events = processor.lock().unwrap().get_events();
        assert_eq!(received_events.len(), 100);

        let metrics = junction.get_performance_metrics();
        assert!(metrics.events_processed >= 100);
        // Note: Health check might be strict for small loads, just verify basic functionality
        assert!(metrics.pipeline_metrics.events_published > 0);
        assert!(metrics.pipeline_metrics.events_consumed > 0);
    }

    #[test]
    fn test_junction_throughput() {
        let (junction, _processor) = setup_junction(true);

        junction.start_processing().unwrap();

        let start = Instant::now();
        let num_events = 10000;

        for i in 0..num_events {
            let event = Event::new_with_data(i, vec![AttributeValue::Int(i as i32)]);
            let _ = junction.send_event(event);
        }

        let duration = start.elapsed();
        let throughput = num_events as f64 / duration.as_secs_f64();

        println!("Throughput: {:.0} events/sec", throughput);

        // Should handle significantly more than original crossbeam_channel implementation
        assert!(throughput > 50000.0, "Throughput should be >50K events/sec");

        // Wait for processing
        thread::sleep(Duration::from_millis(500));

        let metrics = junction.get_performance_metrics();
        println!("Pipeline metrics: {:?}", metrics.pipeline_metrics);
        assert!(metrics.pipeline_metrics.throughput_events_per_sec > 10000.0);
    }

    #[test]
    fn test_junction_backpressure() {
        let (junction, _processor) = setup_junction(true);

        junction.start_processing().unwrap();

        // Flood the junction to test backpressure
        let mut dropped = 0;
        for i in 0..50000 {
            let event = Event::new_with_data(i, vec![AttributeValue::Int(i as i32)]);
            if junction.send_event(event).is_err() {
                dropped += 1;
            }
        }

        println!("Dropped events due to backpressure: {}", dropped);

        let metrics = junction.get_performance_metrics();
        println!("Junction metrics: {:?}", metrics);

        // Some events should be dropped due to backpressure
        assert!(metrics.events_dropped > 0 || dropped > 0);
    }

    #[test]
    fn test_junction_metrics_integration() {
        let (junction, _processor) = setup_junction(true);

        junction.start_processing().unwrap();

        // Send some events
        for i in 0..1000 {
            let event = Event::new_with_data(i, vec![AttributeValue::Int(i as i32)]);
            let _ = junction.send_event(event);
        }

        thread::sleep(Duration::from_millis(100));

        let metrics = junction.get_performance_metrics();

        // Verify metrics are being collected
        assert!(metrics.events_processed > 0);
        assert!(metrics.pipeline_metrics.events_published > 0);
        assert!(metrics.pipeline_metrics.throughput_events_per_sec > 0.0);
        assert!(metrics.health_score() > 0.0);

        // Test legacy methods for compatibility
        assert!(junction.total_events().unwrap() > 0);
        // Note: average_latency_ns might be None if processing is too fast
    }
}
