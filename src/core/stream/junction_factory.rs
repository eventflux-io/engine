// SPDX-License-Identifier: MIT OR Apache-2.0

//! StreamJunction Factory for Event Routing
//!
//! Provides configuration and creation of high-performance StreamJunctions
//! using crossbeam pipeline-based implementation.

use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::stream::StreamJunction;
use crate::query_api::definition::StreamDefinition;
use std::sync::{Arc, Mutex};

/// Configuration for StreamJunction creation
#[derive(Debug, Clone)]
pub struct JunctionConfig {
    pub stream_id: String,
    pub buffer_size: usize,
    pub is_async: bool,
    pub expected_throughput: Option<u64>, // events/second
    pub subscriber_count: Option<usize>,
}

impl JunctionConfig {
    /// Create a new junction configuration with synchronous processing by default
    ///
    /// **Default Mode: Synchronous (is_async: false)**
    /// - Guarantees strict event ordering
    /// - Events are processed sequentially in the order they arrive
    /// - Suitable for scenarios where event order is critical
    ///
    /// **Async Mode Option: Use with_async(true) for high-throughput scenarios**
    /// - Trades event ordering guarantees for higher performance
    /// - Events may be processed out of order due to concurrent processing
    /// - Suitable for scenarios where throughput > ordering
    pub fn new(stream_id: String) -> Self {
        Self {
            stream_id,
            buffer_size: 4096,
            is_async: false, // DEFAULT: Synchronous to guarantee event ordering
            expected_throughput: None,
            subscriber_count: None,
        }
    }

    /// Set buffer size
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Enable or disable async processing mode [CRITICAL ORDERING TRADE-OFF]
    ///
    /// **⚠️  IMPORTANT: Enabling async mode may break event ordering guarantees!**
    ///
    /// **Synchronous Mode (false - DEFAULT):**
    /// - ✅ **Strict event ordering preserved**
    /// - ✅ Events processed sequentially in arrival order
    /// - ✅ Predictable, deterministic behavior
    /// - ❌ Lower throughput (~thousands events/sec)
    /// - **Use when**: Event order is critical for correctness
    ///
    /// **Async Mode (true):**
    /// - ✅ **High throughput** (>100K events/sec capability)
    /// - ✅ Better resource utilization with concurrent processing
    /// - ✅ Non-blocking, scalable performance
    /// - ❌ **Events may be processed out of order**
    /// - ❌ Less predictable timing behavior
    /// - **Use when**: Throughput > strict ordering requirements
    ///
    /// # Example
    /// ```
    /// use eventflux_rust::core::stream::JunctionConfig;
    ///
    /// // Default: Synchronous processing (guaranteed ordering)
    /// let sync_config = JunctionConfig::new("stream".to_string());
    ///
    /// // High-throughput async processing (potential reordering)
    /// let async_config = JunctionConfig::new("stream".to_string())
    ///     .with_async(true)
    ///     .with_expected_throughput(100_000);
    /// ```
    pub fn with_async(mut self, async_mode: bool) -> Self {
        self.is_async = async_mode;
        self
    }

    /// Set expected throughput hint
    pub fn with_expected_throughput(mut self, throughput: u64) -> Self {
        self.expected_throughput = Some(throughput);
        self
    }

    /// Set expected subscriber count hint
    pub fn with_subscriber_count(mut self, count: usize) -> Self {
        self.subscriber_count = Some(count);
        self
    }
}

/// Factory for creating StreamJunctions
pub struct StreamJunctionFactory;

impl StreamJunctionFactory {
    /// Create a StreamJunction with the given configuration
    pub fn create(
        config: JunctionConfig,
        stream_definition: Arc<StreamDefinition>,
        eventflux_app_context: Arc<EventFluxAppContext>,
        fault_stream_junction: Option<Arc<Mutex<StreamJunction>>>,
    ) -> Result<Arc<Mutex<StreamJunction>>, String> {
        Self::create_junction(
            config,
            stream_definition,
            eventflux_app_context,
            fault_stream_junction,
        )
    }

    /// Create a StreamJunction (internal implementation)
    pub fn create_junction(
        config: JunctionConfig,
        stream_definition: Arc<StreamDefinition>,
        eventflux_app_context: Arc<EventFluxAppContext>,
        fault_stream_junction: Option<Arc<Mutex<StreamJunction>>>,
    ) -> Result<Arc<Mutex<StreamJunction>>, String> {
        let junction = StreamJunction::new(
            config.stream_id,
            stream_definition,
            eventflux_app_context,
            config.buffer_size,
            config.is_async,
            fault_stream_junction,
        )?;

        Ok(Arc::new(Mutex::new(junction)))
    }

    /// Create a junction with performance hints
    pub fn create_with_hints(
        stream_id: String,
        stream_definition: Arc<StreamDefinition>,
        eventflux_app_context: Arc<EventFluxAppContext>,
        expected_throughput: Option<u64>,
        subscriber_count: Option<usize>,
    ) -> Result<Arc<Mutex<StreamJunction>>, String> {
        let config = JunctionConfig::new(stream_id)
            .with_expected_throughput(expected_throughput.unwrap_or(0))
            .with_subscriber_count(subscriber_count.unwrap_or(1));

        Self::create(config, stream_definition, eventflux_app_context, None)
    }

    /// Create a high-performance junction for known high-throughput scenarios
    pub fn create_high_performance(
        stream_id: String,
        stream_definition: Arc<StreamDefinition>,
        eventflux_app_context: Arc<EventFluxAppContext>,
        buffer_size: usize,
    ) -> Result<Arc<Mutex<StreamJunction>>, String> {
        let config = JunctionConfig::new(stream_id)
            .with_buffer_size(buffer_size)
            .with_async(true);

        Self::create(config, stream_definition, eventflux_app_context, None)
    }
}

/// Performance benchmark for comparing junction implementations
pub struct JunctionBenchmark;

impl JunctionBenchmark {
    /// Run a simple throughput benchmark
    pub fn benchmark_throughput(
        junction: &Arc<Mutex<StreamJunction>>,
        num_events: usize,
        num_threads: usize,
    ) -> Result<BenchmarkResult, String> {
        use crate::core::event::{value::AttributeValue, Event};
        use std::thread;
        use std::time::Instant;

        let start = Instant::now();
        let mut handles = Vec::new();

        for thread_id in 0..num_threads {
            let events_per_thread = num_events / num_threads;
            let junction_clone = Arc::clone(junction);

            handles.push(thread::spawn(move || {
                for i in 0..events_per_thread {
                    let event = Event::new_with_data(
                        i as i64,
                        vec![AttributeValue::Int(thread_id as i32 * 1000 + i as i32)],
                    );
                    let _ = junction_clone.lock().unwrap().send_event(event);
                }
            }));
        }

        for handle in handles {
            handle.join().map_err(|_| "Thread join failed")?;
        }

        let duration = start.elapsed();
        let throughput = num_events as f64 / duration.as_secs_f64();

        Ok(BenchmarkResult {
            events_sent: num_events,
            duration,
            throughput,
            implementation: "StreamJunction".to_string(),
        })
    }
}

/// Benchmark result for junction performance testing
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub events_sent: usize,
    pub duration: std::time::Duration,
    pub throughput: f64,
    pub implementation: String,
}

impl BenchmarkResult {
    /// Print benchmark results
    pub fn print(&self) {
        println!("Junction Benchmark Results:");
        println!("  Implementation: {}", self.implementation);
        println!("  Events sent: {}", self.events_sent);
        println!("  Duration: {:.2?}", self.duration);
        println!("  Throughput: {:.0} events/sec", self.throughput);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::eventflux_context::EventFluxContext;
    use crate::query_api::definition::attribute::Type as AttrType;

    fn create_test_context() -> Arc<EventFluxAppContext> {
        let eventflux_context = Arc::new(EventFluxContext::new());
        let app = Arc::new(crate::query_api::eventflux_app::EventFluxApp::new(
            "TestApp".to_string(),
        ));
        Arc::new(EventFluxAppContext::new(
            eventflux_context,
            "TestApp".to_string(),
            app,
            String::new(),
        ))
    }

    fn create_test_stream_definition() -> Arc<StreamDefinition> {
        Arc::new(
            StreamDefinition::new("TestStream".to_string())
                .attribute("id".to_string(), AttrType::INT),
        )
    }

    #[test]
    fn test_create_with_config() {
        let config = JunctionConfig::new("TestStream".to_string())
            .with_expected_throughput(100)
            .with_buffer_size(4096);

        let context = create_test_context();
        let stream_def = create_test_stream_definition();

        let junction = StreamJunctionFactory::create(config, stream_def, context, None).unwrap();
        assert_eq!(junction.lock().unwrap().stream_id, "TestStream");
    }

    #[test]
    fn test_high_performance_factory_method() {
        let context = create_test_context();
        let stream_def = create_test_stream_definition();

        let junction = StreamJunctionFactory::create_high_performance(
            "HighPerfStream".to_string(),
            stream_def,
            context,
            32768,
        )
        .unwrap();

        assert_eq!(junction.lock().unwrap().stream_id, "HighPerfStream");
    }

    #[test]
    fn test_junction_with_hints() {
        let context = create_test_context();
        let stream_def = create_test_stream_definition();

        // High throughput hint should create junction successfully
        let junction = StreamJunctionFactory::create_with_hints(
            "HintedStream".to_string(),
            stream_def,
            context,
            Some(150000), // High throughput
            Some(4),      // Multiple subscribers
        )
        .unwrap();

        assert_eq!(junction.lock().unwrap().stream_id, "HintedStream");
    }
}
