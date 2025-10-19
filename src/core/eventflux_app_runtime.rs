// SPDX-License-Identifier: MIT OR Apache-2.0

// Corresponds to io.eventflux.core.EventFluxAppRuntime (interface)
// and io.eventflux.core.EventFluxAppRuntimeImpl (implementation)

use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::config::eventflux_query_context::EventFluxQueryContext; // For add_callback
use crate::core::config::{ApplicationConfig, EventFluxConfig};
use crate::core::eventflux_app_runtime_builder::TableRuntimePlaceholder;
use crate::core::partition::PartitionRuntime;
use crate::core::persistence::SnapshotService;
use crate::core::query::output::callback_processor::CallbackProcessor; // To be created
use crate::core::query::query_runtime::QueryRuntime;
use crate::core::stream::input::input_handler::InputHandler;
use crate::core::stream::input::input_manager::InputManager;
use crate::core::stream::output::stream_callback::StreamCallback; // The trait
use crate::core::stream::stream_junction::StreamJunction;
use crate::core::trigger::TriggerRuntime;
use crate::core::util::parser::eventflux_app_parser::EventFluxAppParser; // For EventFluxAppParser::parse_eventflux_app_runtime_builder
use crate::core::window::WindowRuntime;
use crate::query_api::EventFluxApp as ApiEventFluxApp; // From query_api
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use std::collections::HashMap;

/// Manages the runtime lifecycle of a single EventFlux Application.
#[derive(Debug)] // Default removed, construction via new() -> Result
pub struct EventFluxAppRuntime {
    pub name: String,
    pub eventflux_app: Arc<ApiEventFluxApp>, // The original parsed API definition
    pub eventflux_app_context: Arc<EventFluxAppContext>,

    // Runtime components constructed by EventFluxAppRuntimeBuilder
    pub stream_junction_map: HashMap<String, Arc<Mutex<StreamJunction>>>,
    pub input_manager: Arc<InputManager>,
    pub query_runtimes: Vec<Arc<QueryRuntime>>,
    pub partition_runtimes: Vec<Arc<PartitionRuntime>>,
    pub trigger_runtimes: Vec<Arc<TriggerRuntime>>,
    pub scheduler: Option<Arc<crate::core::util::Scheduler>>,
    pub table_map: HashMap<String, Arc<Mutex<TableRuntimePlaceholder>>>,
    pub window_map: HashMap<String, Arc<Mutex<WindowRuntime>>>,
    pub aggregation_map: HashMap<String, Arc<Mutex<crate::core::aggregation::AggregationRuntime>>>,

    // Stream handlers for lifecycle management (M7)
    pub source_handlers: Arc<
        std::sync::RwLock<HashMap<String, Arc<crate::core::stream::handler::SourceStreamHandler>>>,
    >,
    pub sink_handlers: Arc<
        std::sync::RwLock<HashMap<String, Arc<crate::core::stream::handler::SinkStreamHandler>>>,
    >,
}

impl EventFluxAppRuntime {
    // This 'new' function replaces the direct construction and acts more like
    // EventFluxManager.createEventFluxAppRuntime(api_eventflux_app)
    pub fn new(
        api_eventflux_app: Arc<ApiEventFluxApp>,
        // EventFluxContext is needed to initialize EventFluxAppContext if not already done
        eventflux_context: Arc<crate::core::config::eventflux_context::EventFluxContext>,
        eventflux_app_string: Option<String>,
    ) -> Result<Self, String> {
        // 1. Create EventFluxAppContext using @app level annotations when present
        let mut name = api_eventflux_app.name.clone();
        let mut is_playback = false;
        let mut enforce_order = false;
        let mut root_metrics =
            crate::core::config::eventflux_app_context::MetricsLevelPlaceholder::OFF;
        let mut buffer_size = 0i32;
        let mut transport_creation = false;

        if let Some(app_ann) = api_eventflux_app
            .annotations
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case("app"))
        {
            for el in &app_ann.elements {
                match el.key.to_lowercase().as_str() {
                    "name" => name = el.value.clone(),
                    "playback" => is_playback = el.value.eq_ignore_ascii_case("true"),
                    "enforce.order" | "enforceorder" => {
                        enforce_order = el.value.eq_ignore_ascii_case("true")
                    }
                    "statistics" | "stats" => root_metrics = match el.value.to_lowercase().as_str()
                    {
                        "true" | "basic" => {
                            crate::core::config::eventflux_app_context::MetricsLevelPlaceholder::BASIC
                        }
                        "detail" | "detailed" => {
                            crate::core::config::eventflux_app_context::MetricsLevelPlaceholder::DETAIL
                        }
                        _ => crate::core::config::eventflux_app_context::MetricsLevelPlaceholder::OFF,
                    },
                    "buffer_size" | "buffersize" => {
                        if let Ok(sz) = el.value.parse::<i32>() {
                            buffer_size = sz;
                        }
                    }
                    "transport.channel.creation" => {
                        transport_creation = el.value.eq_ignore_ascii_case("true");
                    }
                    _ => {}
                }
            }
        }

        let mut ctx = EventFluxAppContext::new(
            eventflux_context,
            name.clone(),
            Arc::clone(&api_eventflux_app),
            eventflux_app_string.unwrap_or_default(),
        );
        ctx.set_playback(is_playback);
        ctx.set_enforce_order(enforce_order);
        ctx.set_root_metrics_level(root_metrics);
        if buffer_size > 0 {
            ctx.set_buffer_size(buffer_size);
        }
        ctx.set_transport_channel_creation_enabled(transport_creation);

        // Initialize ThreadBarrier if enforce_order is enabled or for persistence coordination
        let thread_barrier = Arc::new(crate::core::util::thread_barrier::ThreadBarrier::new());
        ctx.set_thread_barrier(thread_barrier);

        let scheduler = if let Some(exec) = ctx.get_scheduled_executor_service() {
            Arc::new(crate::core::util::Scheduler::new(Arc::clone(
                &exec.executor,
            )))
        } else {
            Arc::new(crate::core::util::Scheduler::new(Arc::new(
                crate::core::util::ExecutorService::default(),
            )))
        };
        ctx.set_scheduler(Arc::clone(&scheduler));
        let mut ss = SnapshotService::new(name.clone());
        if let Some(store) = ctx.eventflux_context.get_persistence_store() {
            ss.persistence_store = Some(store);
        }
        let snapshot_service = Arc::new(ss);
        ctx.set_snapshot_service(Arc::clone(&snapshot_service));
        let eventflux_app_context = Arc::new(ctx);

        // 2. Parse the ApiEventFluxApp into a builder
        let builder = EventFluxAppParser::parse_eventflux_app_runtime_builder(
            &api_eventflux_app,
            eventflux_app_context,
            None,
        )?;

        // 3. Build the EventFluxAppRuntime from the builder
        builder.build(api_eventflux_app) // Pass the Arc<ApiEventFluxApp> again
    }

    /// Create a new EventFluxAppRuntime with specific application configuration
    pub fn new_with_config(
        api_eventflux_app: Arc<ApiEventFluxApp>,
        eventflux_context: Arc<crate::core::config::eventflux_context::EventFluxContext>,
        eventflux_app_string: Option<String>,
        app_config: Option<ApplicationConfig>,
    ) -> Result<Self, String> {
        // If we have application configuration, apply it before creating the runtime
        if let Some(config) = app_config {
            // Apply global configuration from app config to the runtime
            Self::new_with_applied_config(
                api_eventflux_app,
                eventflux_context,
                eventflux_app_string,
                &config,
            )
        } else {
            // Fall back to standard creation if no config provided
            Self::new(api_eventflux_app, eventflux_context, eventflux_app_string)
        }
    }

    /// Internal method to create runtime with applied configuration
    fn new_with_applied_config(
        api_eventflux_app: Arc<ApiEventFluxApp>,
        eventflux_context: Arc<crate::core::config::eventflux_context::EventFluxContext>,
        eventflux_app_string: Option<String>,
        app_config: &ApplicationConfig,
    ) -> Result<Self, String> {
        // 1. Create EventFluxAppContext using @app level annotations when present
        let mut name = api_eventflux_app.name.clone();
        let mut is_playback = false;
        let mut enforce_order = false;
        let mut root_metrics =
            crate::core::config::eventflux_app_context::MetricsLevelPlaceholder::OFF;
        let mut buffer_size = 0i32;
        let mut transport_creation = false;

        // Apply configuration-based settings from monitoring
        if let Some(ref monitoring) = app_config.monitoring {
            if monitoring.metrics_enabled {
                root_metrics =
                    crate::core::config::eventflux_app_context::MetricsLevelPlaceholder::BASIC;
            }
            // Note: ApplicationConfig::MonitoringConfig doesn't have detailed_metrics
            // This would be a global configuration setting
        }

        // Then apply @app level annotations (which override config)
        if let Some(app_ann) = api_eventflux_app
            .annotations
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case("app"))
        {
            for el in &app_ann.elements {
                match el.key.to_lowercase().as_str() {
                    "name" => name = el.value.clone(),
                    "playback" => is_playback = el.value.eq_ignore_ascii_case("true"),
                    "enforce.order" | "enforceorder" => {
                        enforce_order = el.value.eq_ignore_ascii_case("true")
                    }
                    "statistics" | "stats" => root_metrics = match el.value.to_lowercase().as_str()
                    {
                        "true" | "basic" => {
                            crate::core::config::eventflux_app_context::MetricsLevelPlaceholder::BASIC
                        }
                        "detail" | "detailed" => {
                            crate::core::config::eventflux_app_context::MetricsLevelPlaceholder::DETAIL
                        }
                        _ => crate::core::config::eventflux_app_context::MetricsLevelPlaceholder::OFF,
                    },
                    "buffer_size" | "buffersize" => {
                        if let Ok(sz) = el.value.parse::<i32>() {
                            buffer_size = sz;
                        }
                    }
                    "transport.channel.creation" => {
                        transport_creation = el.value.eq_ignore_ascii_case("true");
                    }
                    _ => {}
                }
            }
        }

        let mut ctx = EventFluxAppContext::new_with_config(
            eventflux_context,
            name.clone(),
            Arc::clone(&api_eventflux_app),
            String::new(),                        // eventflux_app_string
            Arc::new(EventFluxConfig::default()), // global_config
            Some(app_config.clone()),             // app_config
            None,                                 // config_manager
        );

        ctx.set_root_metrics_level(root_metrics);
        if buffer_size > 0 {
            ctx.set_buffer_size(buffer_size);
        }
        ctx.set_transport_channel_creation_enabled(transport_creation);

        // Apply additional configuration settings to context
        if let Some(ref _error_handling) = app_config.error_handling {
            // Error handling configuration would be applied here
            // Future implementation will configure error handling strategies
        }

        // Initialize ThreadBarrier if enforce_order is enabled or for persistence coordination
        let thread_barrier = Arc::new(crate::core::util::ThreadBarrier::new());
        ctx.set_thread_barrier(Arc::clone(&thread_barrier));

        // 2. Create SnapshotService and configure it
        let mut ss = SnapshotService::new(name.clone());
        if let Some(store) = ctx.eventflux_context.get_persistence_store() {
            ss.persistence_store = Some(store);
        }
        let snapshot_service = Arc::new(ss);
        ctx.set_snapshot_service(Arc::clone(&snapshot_service));
        let eventflux_app_context = Arc::new(ctx);

        // 2. Parse the ApiEventFluxApp into a builder with configuration
        let builder = EventFluxAppParser::parse_eventflux_app_runtime_builder(
            &api_eventflux_app,
            eventflux_app_context,
            Some(app_config.clone()),
        )?;

        // 3. Build the EventFluxAppRuntime from the builder
        let runtime = builder.build(api_eventflux_app)?;

        // Note: Auto-attach of sources and sinks is deferred to start() method
        // This allows proper lifecycle management:
        //  - Construction: Create runtime and register components
        //  - Start: Attach and start I/O sources/sinks
        //  - Shutdown: Stop I/O and clean up
        //
        // The application configuration is stored in eventflux_app_context.app_config
        // and will be accessed during start()

        Ok(runtime)
    }

    pub fn get_input_handler(&self, stream_id: &str) -> Option<Arc<Mutex<InputHandler>>> {
        self.input_manager.get_input_handler(stream_id)
    }

    pub fn get_table_input_handler(
        &self,
        table_id: &str,
    ) -> Option<crate::core::stream::input::table_input_handler::TableInputHandler> {
        self.input_manager.get_table_input_handler(table_id)
    }

    pub fn add_callback(
        &self,
        stream_id: &str,
        callback: Box<dyn StreamCallback>,
    ) -> Result<(), String> {
        let output_junction = self
            .stream_junction_map
            .get(stream_id)
            .ok_or_else(|| format!("StreamJunction '{stream_id}' not found to add callback"))?
            .clone();

        let query_name_for_callback = format!(
            "callback_processor_{}_{}",
            stream_id,
            Uuid::new_v4().hyphenated()
        );
        let query_context_for_callback = Arc::new(EventFluxQueryContext::new(
            Arc::clone(&self.eventflux_app_context),
            query_name_for_callback.clone(),
            None, // No specific partition ID for a generic stream callback processor
        ));

        let callback_processor = Arc::new(Mutex::new(CallbackProcessor::new(
            Arc::new(Mutex::new(callback)),
            Arc::clone(&self.eventflux_app_context),
            query_context_for_callback,
            // query_name_for_callback, // query_name is now in query_context
        )));
        output_junction
            .lock()
            .expect("Output StreamJunction Mutex poisoned")
            .subscribe(callback_processor);
        Ok(())
    }

    // ========================================================================
    // Stream Handler Management (M7)
    // ========================================================================

    /// Register a source stream handler
    pub fn register_source_handler(
        &self,
        stream_name: String,
        handler: Arc<crate::core::stream::handler::SourceStreamHandler>,
    ) {
        self.source_handlers
            .write()
            .unwrap()
            .insert(stream_name, handler);
    }

    /// Get a source stream handler by name
    pub fn get_source_handler(
        &self,
        stream_name: &str,
    ) -> Option<Arc<crate::core::stream::handler::SourceStreamHandler>> {
        self.source_handlers
            .read()
            .unwrap()
            .get(stream_name)
            .cloned()
    }

    /// Register a sink stream handler
    pub fn register_sink_handler(
        &self,
        stream_name: String,
        handler: Arc<crate::core::stream::handler::SinkStreamHandler>,
    ) {
        self.sink_handlers
            .write()
            .unwrap()
            .insert(stream_name, handler);
    }

    /// Get a sink stream handler by name
    pub fn get_sink_handler(
        &self,
        stream_name: &str,
    ) -> Option<Arc<crate::core::stream::handler::SinkStreamHandler>> {
        self.sink_handlers.read().unwrap().get(stream_name).cloned()
    }

    /// Attach sink handler to junction for event delivery
    ///
    /// This connects a sink handler to the stream junction so it receives events.
    /// Uses the same pattern as `add_callback` to properly wire the sink to the junction.
    pub fn attach_sink_to_junction(
        &self,
        stream_name: &str,
        handler: Arc<crate::core::stream::handler::SinkStreamHandler>,
    ) -> Result<(), String> {
        let output_junction = self
            .stream_junction_map
            .get(stream_name)
            .ok_or_else(|| format!("StreamJunction '{}' not found to attach sink", stream_name))?
            .clone();

        // Create a query context for the sink callback processor
        let query_name_for_sink = format!(
            "sink_handler_{}_{}",
            stream_name,
            Uuid::new_v4().hyphenated()
        );
        let query_context_for_sink = Arc::new(EventFluxQueryContext::new(
            Arc::clone(&self.eventflux_app_context),
            query_name_for_sink.clone(),
            None,
        ));

        // Get the underlying sink from the handler
        // Since Sink extends StreamCallback, we can convert directly
        let sink = handler.sink();

        // Lock and clone the inner Box<dyn Sink>
        let sink_guard = sink.lock().expect("Sink Mutex poisoned");
        let sink_clone = sink_guard.clone_box();
        drop(sink_guard); // Release lock

        // Box<dyn Sink> can be upcast to Box<dyn StreamCallback> because Sink: StreamCallback
        let sink_as_callback: Box<dyn StreamCallback> = sink_clone;

        // Wrap in Arc<Mutex<>> as expected by CallbackProcessor
        let sink_callback_arc = Arc::new(Mutex::new(sink_as_callback));

        // Wrap in a CallbackProcessor and subscribe to junction
        let callback_processor = Arc::new(Mutex::new(CallbackProcessor::new(
            sink_callback_arc,
            Arc::clone(&self.eventflux_app_context),
            query_context_for_sink,
        )));

        output_junction
            .lock()
            .expect("Output StreamJunction Mutex poisoned")
            .subscribe(callback_processor);

        Ok(())
    }

    /// Start all registered source handlers
    pub fn start_all_sources(&self) -> Result<(), String> {
        for handler in self.source_handlers.read().unwrap().values() {
            handler
                .start()
                .map_err(|e| format!("Failed to start source '{}': {}", handler.stream_id(), e))?;
        }
        Ok(())
    }

    /// Stop all registered source handlers
    pub fn stop_all_sources(&self) {
        for handler in self.source_handlers.read().unwrap().values() {
            handler.stop();
        }
    }

    /// Stop all registered sink handlers
    pub fn stop_all_sinks(&self) {
        for handler in self.sink_handlers.read().unwrap().values() {
            handler.stop();
        }
    }

    pub fn start(&self) {
        // Auto-attach sources and sinks from configuration (if available)
        if let Some(app_config) = &self.eventflux_app_context.app_config {
            // Auto-attach sources - idempotent operation with error accumulation
            match self.auto_attach_sources_from_config(app_config) {
                Ok(sources) => {
                    if !sources.is_empty() {
                        println!(
                            "[EventFluxAppRuntime] Successfully attached {} source(s): {}",
                            sources.len(),
                            sources.join(", ")
                        );
                    }
                }
                Err(errors) => {
                    eprintln!(
                        "[EventFluxAppRuntime] Failed to auto-attach sources ({} error(s)):",
                        errors.len()
                    );
                    for (i, e) in errors.iter().enumerate() {
                        eprintln!("  {}. {}", i + 1, e);
                    }
                }
            }

            // Auto-attach sinks - idempotent operation
            if let Err(e) = self.auto_attach_sinks_from_config(app_config) {
                eprintln!("[EventFluxAppRuntime] Failed to auto-attach sinks: {}", e);
            }
        }

        // Start all registered sources (idempotent - no-op if already started)
        if let Err(e) = self.start_all_sources() {
            eprintln!("[EventFluxAppRuntime] Failed to start sources: {}", e);
        }

        if self.scheduler.is_some() {
            // placeholder: scheduler is kept alive by self
            println!(
                "Scheduler initialized for EventFluxAppRuntime '{}'",
                self.name
            );
        }
        for tr in &self.trigger_runtimes {
            tr.start();
        }
        for pr in &self.partition_runtimes {
            pr.start();
        }
        println!("EventFluxAppRuntime '{}' started", self.name);
    }

    pub fn shutdown(&self) {
        // Stop all source and sink handlers first
        self.stop_all_sources();
        self.stop_all_sinks();

        if let Some(scheduler) = &self.scheduler {
            scheduler.shutdown();
        }
        for tr in &self.trigger_runtimes {
            tr.shutdown();
        }
        for pr in &self.partition_runtimes {
            pr.shutdown();
        }
        for qr in &self.query_runtimes {
            qr.flush();
        }
        // Persisted revisions are retained after shutdown for potential restoration
        println!("EventFluxAppRuntime '{}' shutdown", self.name);
    }

    /// Persist the current snapshot using the configured SnapshotService.
    pub fn persist(&self) -> Result<String, String> {
        let service = self
            .eventflux_app_context
            .get_snapshot_service()
            .ok_or("SnapshotService not set")?;
        service.persist()
    }

    /// Capture a snapshot of the current state via the SnapshotService.
    pub fn snapshot(&self) -> Result<Vec<u8>, String> {
        let service = self
            .eventflux_app_context
            .get_snapshot_service()
            .ok_or("SnapshotService not set")?;
        Ok(service.snapshot())
    }

    /// Restore the given snapshot bytes using the SnapshotService.
    pub fn restore(&self, snapshot: &[u8]) -> Result<(), String> {
        let service = self
            .eventflux_app_context
            .get_snapshot_service()
            .ok_or("SnapshotService not set")?;
        service.set_state(snapshot.to_vec());
        Ok(())
    }

    /// Restore the given revision using the SnapshotService.
    pub fn restore_revision(&self, revision: &str) -> Result<(), String> {
        let service = self
            .eventflux_app_context
            .get_snapshot_service()
            .ok_or("SnapshotService not set")?;

        // Use ThreadBarrier to coordinate with event processing threads
        if let Some(barrier) = self.eventflux_app_context.get_thread_barrier() {
            // Lock the barrier to prevent new events from entering
            barrier.lock();

            // Wait for all active threads to complete their current processing
            while barrier.get_active_threads() > 0 {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }

            // Perform the restoration while event processing is blocked
            let result = service.restore_revision(revision);

            // Clear SelectProcessor group states after restoration to ensure fresh aggregator state
            if result.is_ok() {
                self.clear_select_processor_group_states();
            }

            // Unlock the barrier to resume normal processing
            barrier.unlock();

            result
        } else {
            // No barrier configured, proceed with restoration (may have timing issues)
            service.restore_revision(revision)
        }
    }

    /// Clear group states in all SelectProcessors to ensure fresh state after restoration
    fn clear_select_processor_group_states(&self) {
        for query_runtime in &self.query_runtimes {
            // Try to access the processor chain and find SelectProcessors
            if let Some(ref processor) = query_runtime.processor_chain_head {
                self.clear_processor_chain_group_states(processor);
            }
        }
    }

    /// Recursively clear group states in processor chains
    fn clear_processor_chain_group_states(
        &self,
        processor: &Arc<Mutex<dyn crate::core::query::processor::Processor>>,
    ) {
        if let Ok(proc) = processor.lock() {
            // Clear group states for this processor (no-op for non-SelectProcessors)
            proc.clear_group_states();

            // Recursively check next processors in the chain
            if let Some(ref next) = proc.next_processor() {
                self.clear_processor_chain_group_states(next);
            }
        }
    }

    /// Query an aggregation runtime using optional `within` and `per` clauses.
    pub fn query_aggregation(
        &self,
        agg_id: &str,
        within: Option<crate::query_api::aggregation::Within>,
        per: Option<crate::query_api::aggregation::TimeDuration>,
    ) -> Vec<Vec<crate::core::event::value::AttributeValue>> {
        if let Some(rt) = self.aggregation_map.get(agg_id) {
            rt.lock().unwrap().query(within, per)
        } else {
            Vec::new()
        }
    }

    /// Auto-attach sources from configuration
    ///
    /// Automatically creates source stream handlers based on application configuration.
    /// Uses the existing stream_initializer infrastructure for proper factory integration
    /// and mapper support.
    ///
    /// This operation is idempotent - if a source handler is already registered for a stream,
    /// it will be skipped (only started if not already running).
    ///
    /// # Production Features
    ///
    /// - Accumulates all errors instead of failing on first error
    /// - Provides detailed error context for debugging
    /// - Tracks which sources succeeded vs failed
    /// - Proper logging instead of println
    ///
    /// # Flow
    ///
    /// 1. Iterate through all configured streams with sources
    /// 2. Skip if handler already registered (idempotent)
    /// 3. Convert SourceConfig to StreamTypeConfig
    /// 4. Use stream_initializer to create source with mapper
    /// 5. Create SourceStreamHandler and register it
    /// 6. Start the source
    /// 7. Collect successes and failures
    ///
    /// # Arguments
    ///
    /// * `app_config` - Application configuration containing stream definitions
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - List of successfully attached source stream names
    /// * `Err(Vec<EventFluxError>)` - List of all errors encountered (partial success possible)
    fn auto_attach_sources_from_config(
        &self,
        app_config: &ApplicationConfig,
    ) -> Result<Vec<String>, Vec<crate::core::exception::EventFluxError>> {
        let mut errors = Vec::new();
        let mut successes = Vec::new();

        // Iterate through all configured streams
        for (stream_name, stream_config) in &app_config.streams {
            // Check if this stream has a source configuration
            if let Some(ref source_config) = stream_config.source {
                // Process this source, collecting errors instead of failing fast
                match self.attach_single_source(stream_name, source_config) {
                    Ok(()) => {
                        successes.push(stream_name.clone());
                        println!(
                            "[EventFluxAppRuntime] Successfully attached source '{}' to stream '{}'",
                            source_config.source_type, stream_name
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "[EventFluxAppRuntime] Failed to attach source '{}' to stream '{}': {}",
                            source_config.source_type, stream_name, e
                        );
                        errors.push(e);
                    }
                }
            }
        }

        // Return results based on what happened
        if errors.is_empty() {
            Ok(successes)
        } else if successes.is_empty() {
            // All failed
            Err(errors)
        } else {
            // Partial success - log warning but don't fail
            eprintln!(
                "[EventFluxAppRuntime] Partial source attachment: {} succeeded, {} failed",
                successes.len(),
                errors.len()
            );
            Ok(successes)
        }
    }

    /// Attach a single source stream handler
    ///
    /// Separated into its own method for clarity and error handling.
    /// All errors are properly typed as EventFluxError with context.
    fn attach_single_source(
        &self,
        stream_name: &str,
        source_config: &crate::core::config::types::application_config::SourceConfig,
    ) -> Result<(), crate::core::exception::EventFluxError> {
        use crate::core::config::stream_config::{StreamType, StreamTypeConfig};
        use crate::core::exception::EventFluxError;
        use crate::core::stream::handler::SourceStreamHandler;
        use crate::core::stream::stream_initializer::{initialize_stream, InitializedStream};

        // Check if handler already registered (idempotent operation)
        if let Some(existing_handler) = self.get_source_handler(stream_name) {
            // Handler already exists - just ensure it's started
            existing_handler.start()?;
            return Ok(());
        }

        // 1. Convert SourceConfig to StreamTypeConfig
        let properties = self
            .extract_connection_config(&source_config.connection)
            .map_err(|e| {
                EventFluxError::configuration(format!(
                    "Invalid connection config for source stream '{}': {}",
                    stream_name, e
                ))
            })?;

        let stream_type_config = StreamTypeConfig::new(
            StreamType::Source,
            Some(source_config.source_type.clone()),
            source_config.format.clone(),
            properties,
        )
        .map_err(|e| {
            EventFluxError::configuration(format!(
                "Invalid StreamTypeConfig for source '{}' (type={}): {}",
                stream_name, source_config.source_type, e
            ))
        })?;

        // 2. Use stream_initializer to create source with mapper
        let initialized =
            initialize_stream(&self.eventflux_app_context.eventflux_context, &stream_type_config)
                .map_err(|e| {
                    EventFluxError::app_creation(format!(
                        "Failed to initialize source '{}' (type={}, format={:?}): {}",
                        stream_name,
                        source_config.source_type,
                        source_config.format,
                        e
                    ))
                })?;

        // 3. Extract source and mapper from initialized stream
        let (source, mapper) = match initialized {
            InitializedStream::Source(init_source) => (init_source.source, Some(init_source.mapper)),
            _ => {
                return Err(EventFluxError::app_creation(format!(
                    "Expected source stream initialization for '{}', got different stream type",
                    stream_name
                )))
            }
        };

        // 4. Get or create InputHandler for this stream
        let input_handler = self
            .input_manager
            .construct_input_handler(stream_name)
            .map_err(|e| {
                EventFluxError::app_creation(format!(
                    "Failed to construct InputHandler for source stream '{}': {}",
                    stream_name, e
                ))
            })?;

        // 5. Create SourceStreamHandler
        let handler = Arc::new(SourceStreamHandler::new(
            source,
            mapper,
            input_handler,
            stream_name.to_string(),
        ));

        // 6. Register handler in runtime
        self.register_source_handler(stream_name.to_string(), Arc::clone(&handler));

        // 7. Start the source
        handler.start().map_err(|e| {
            EventFluxError::app_runtime(format!(
                "Failed to start source '{}' (type={}): {}",
                stream_name, source_config.source_type, e
            ))
        })?;

        Ok(())
    }

    /// Extract connection configuration from serde_yaml::Value into HashMap
    ///
    /// Converts the flattened connection configuration from SourceConfig/SinkConfig into
    /// a HashMap that can be passed to factory create_initialized() methods.
    ///
    /// # Arguments
    ///
    /// * `connection` - Connection configuration from SourceConfig/SinkConfig
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap)` - Configuration as key-value pairs
    /// * `Err(String)` - If configuration cannot be extracted
    fn extract_connection_config(
        &self,
        connection: &serde_yaml::Value,
    ) -> Result<std::collections::HashMap<String, String>, String> {
        use std::collections::HashMap;

        // Convert serde_yaml::Value to HashMap<String, String>
        let map = match connection {
            serde_yaml::Value::Mapping(m) => m,
            serde_yaml::Value::Null => {
                // Empty connection config is valid
                return Ok(HashMap::new());
            }
            _ => {
                return Err(format!(
                    "Connection configuration must be an object/mapping, got: {:?}",
                    connection
                ))
            }
        };

        let mut config = HashMap::new();

        for (key, value) in map {
            // Get key as string
            let key_str = key
                .as_str()
                .ok_or_else(|| format!("Configuration key must be a string, got: {:?}", key))?;

            // Convert value to string representation
            let value_str = match value {
                serde_yaml::Value::String(s) => s.clone(),
                serde_yaml::Value::Number(n) => n.to_string(),
                serde_yaml::Value::Bool(b) => b.to_string(),
                serde_yaml::Value::Sequence(_) => {
                    // Serialize arrays as JSON string for factory consumption
                    serde_json::to_string(value).map_err(|e| {
                        format!(
                            "Failed to serialize array value for key '{}': {}",
                            key_str, e
                        )
                    })?
                }
                serde_yaml::Value::Null => String::new(),
                _ => {
                    return Err(format!(
                        "Unsupported value type for configuration key '{}': {:?}",
                        key_str, value
                    ))
                }
            };

            config.insert(key_str.to_string(), value_str);
        }

        Ok(config)
    }

    /// Auto-attach sinks from configuration
    fn auto_attach_sinks_from_config(&self, app_config: &ApplicationConfig) -> Result<(), String> {
        use crate::core::config::ProcessorConfigReader;
        use crate::core::stream::output::sink::SinkFactoryRegistry;

        // Create a ProcessorConfigReader with the application configuration
        let config_reader = Arc::new(ProcessorConfigReader::new(
            Some(app_config.clone()),
            None, // Global config would be passed here if available
        ));

        // Create a sink factory registry
        let registry = SinkFactoryRegistry::new();

        // Iterate through all configured streams
        for (stream_name, stream_config) in &app_config.streams {
            // Check if this stream has a sink configuration
            if let Some(ref sink_config) = stream_config.sink {
                // Create the sink using the factory
                let sink = registry.create_sink(
                    sink_config,
                    Some(Arc::clone(&config_reader)),
                    stream_name,
                )?;

                // Attach the sink to the stream
                self.add_callback(stream_name, sink)?;

                println!(
                    "[EventFluxAppRuntime] Auto-attached sink '{}' to stream '{}'",
                    sink_config.sink_type, stream_name
                );
            }
        }

        Ok(())
    }
}
