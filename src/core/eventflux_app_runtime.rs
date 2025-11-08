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

/// Adapter to share sink Arc between handler lifecycle and junction callbacks
///
/// This ensures the SAME sink instance receives both lifecycle calls (start/stop)
/// and event callbacks (receive_events), preventing the bug where cloning creates two
/// separate sink instances.
#[derive(Debug)]
struct SinkCallbackAdapter {
    sink: Arc<Mutex<Box<dyn crate::core::stream::output::sink::Sink>>>,
}

impl crate::core::stream::output::stream_callback::StreamCallback for SinkCallbackAdapter {
    fn receive_events(&self, events: &[crate::core::event::event::Event]) {
        // Forward to the shared sink instance
        // Sink trait extends StreamCallback, so it has receive_events
        self.sink.lock().unwrap().receive_events(events)
    }
}

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

    // Table handlers (tables are passively queried, no lifecycle needed)
    pub table_handlers: Arc<std::sync::RwLock<HashMap<String, Arc<dyn crate::core::table::Table>>>>,
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
        // 1. Create EventFluxAppContext using YAML/TOML configuration (ApplicationConfig)
        let name = api_eventflux_app.name.clone();
        let mut root_metrics =
            crate::core::config::eventflux_app_context::MetricsLevelPlaceholder::OFF;
        let buffer_size = 0i32;

        // Apply configuration-based settings from monitoring
        if let Some(ref monitoring) = app_config.monitoring {
            if monitoring.metrics_enabled {
                root_metrics =
                    crate::core::config::eventflux_app_context::MetricsLevelPlaceholder::BASIC;
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

    /// Register a table handler
    pub fn register_table_handler(
        &self,
        table_name: String,
        table: Arc<dyn crate::core::table::Table>,
    ) {
        self.table_handlers
            .write()
            .unwrap()
            .insert(table_name, table);
    }

    /// Get a table handler by name
    pub fn get_table_handler(
        &self,
        table_name: &str,
    ) -> Option<Arc<dyn crate::core::table::Table>> {
        self.table_handlers.read().unwrap().get(table_name).cloned()
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

        // Get the underlying sink Arc from the handler
        // CRITICAL FIX: Use adapter to share the SAME Arc instead of cloning the sink
        // This ensures lifecycle calls (start/stop) and event callbacks (receive)
        // operate on the SAME sink instance, fixing the bug where SQL sinks never started
        let sink_arc = handler.sink(); // Arc<Mutex<Box<dyn Sink>>>

        // Create adapter that shares the Arc (refcount increases, but same underlying sink)
        let adapter = SinkCallbackAdapter {
            sink: Arc::clone(&sink_arc),
        };

        // Wrap adapter as Box<dyn StreamCallback>
        let callback_box: Box<dyn StreamCallback> = Box::new(adapter);

        // Wrap in Arc<Mutex<>> as expected by CallbackProcessor
        let sink_callback_arc = Arc::new(Mutex::new(callback_box));

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
        let mut errors = Vec::new();

        // Attempt to start all sources, accumulating errors
        for handler in self.source_handlers.read().unwrap().values() {
            if let Err(e) = handler.start() {
                let error_msg = format!("Failed to start source '{}': {}", handler.stream_id(), e);
                log::error!("[EventFluxAppRuntime] {}", error_msg);
                errors.push(error_msg);
            }
        }

        // Fail fast - no partial success allowed
        if !errors.is_empty() {
            Err(format!(
                "Failed to start {} source(s): {}",
                errors.len(),
                errors.join("; ")
            ))
        } else {
            Ok(())
        }
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

    /// Start the EventFlux application runtime
    ///
    /// Attempts to auto-attach configured sources, sinks, and tables, then starts
    /// all components. Errors are logged for debugging and accumulated for reporting.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Runtime started successfully (all attachments succeeded)
    /// * `Err(EventFluxError)` - One or more attachment or startup failures occurred
    ///
    /// # Error Handling
    ///
    /// This method logs all errors as they occur for immediate debugging feedback,
    /// then returns an aggregated error if any operations failed. This allows callers
    /// to detect startup failures while still providing detailed logs.
    ///
    /// Partial success is possible (e.g., some sources attach, others fail). The
    /// returned error will contain details about all failures.
    pub fn start(&self) -> Result<(), crate::core::exception::EventFluxError> {
        use crate::core::exception::EventFluxError;

        let mut all_errors = Vec::new();

        // Auto-attach sources and sinks from configuration (if available)
        if let Some(app_config) = &self.eventflux_app_context.app_config {
            // Auto-attach sources - idempotent operation with error accumulation
            match self.auto_attach_sources_from_config(app_config) {
                Ok(sources) => {
                    if !sources.is_empty() {
                        log::info!(
                            "Successfully attached {} source(s): {}",
                            sources.len(),
                            sources.join(", ")
                        );
                    }
                }
                Err(errors) => {
                    log::error!(
                        "Failed to auto-attach sources ({} error(s)):",
                        errors.len()
                    );
                    for (i, e) in errors.iter().enumerate() {
                        log::error!("  {}. {}", i + 1, e);
                    }
                    all_errors.extend(errors);
                }
            }

            // Auto-attach sinks - idempotent operation
            if let Err(e) = self.auto_attach_sinks_from_config(app_config) {
                log::error!("Failed to auto-attach sinks: {}", e);
                all_errors.push(EventFluxError::app_runtime(e));
            }

            // Auto-attach tables - idempotent operation with error accumulation
            match self.auto_attach_tables_from_config(app_config) {
                Ok(tables) => {
                    if !tables.is_empty() {
                        log::info!(
                            "Successfully attached {} table(s): {}",
                            tables.len(),
                            tables.join(", ")
                        );
                    }
                }
                Err(errors) => {
                    log::error!(
                        "Failed to auto-attach tables ({} error(s)):",
                        errors.len()
                    );
                    for (idx, err) in errors.iter().enumerate() {
                        log::error!("  {}. {}", idx + 1, err);
                    }
                    all_errors.extend(errors);
                }
            }
        }

        // Auto-attach from SQL WITH definitions (higher priority than YAML)
        match self.auto_attach_from_sql_definitions() {
            Ok((sources, sinks)) => {
                if !sources.is_empty() || !sinks.is_empty() {
                    log::info!(
                        "Auto-attached from SQL: {} source(s), {} sink(s)",
                        sources.len(),
                        sinks.len()
                    );
                }
            }
            Err(errors) => {
                log::error!(
                    "Failed to auto-attach from SQL ({} error(s)):",
                    errors.len()
                );
                for (i, e) in errors.iter().enumerate() {
                    log::error!("  {}. {}", i + 1, e);
                }
                all_errors.extend(errors);
            }
        }

        // Start all registered sources (idempotent - no-op if already started)
        if let Err(e) = self.start_all_sources() {
            log::error!("Failed to start sources: {}", e);
            all_errors.push(EventFluxError::app_runtime(e));
        }

        if self.scheduler.is_some() {
            // placeholder: scheduler is kept alive by self
            log::info!(
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

        // Check if any errors occurred during startup
        if !all_errors.is_empty() {
            log::error!(
                "EventFluxAppRuntime '{}' started with {} error(s)",
                self.name,
                all_errors.len()
            );
            return Err(EventFluxError::app_runtime(format!(
                "Runtime startup encountered {} error(s): {}",
                all_errors.len(),
                all_errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            )));
        }

        log::info!("EventFluxAppRuntime '{}' started successfully", self.name);
        Ok(())
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
        log::info!("EventFluxAppRuntime '{}' shutdown", self.name);
    }

    /// Persist the current snapshot using the configured SnapshotService.
    ///
    /// # Returns
    ///
    /// * `Ok(PersistReport)` - Persistence completed with details about successes/failures
    /// * `Err(String)` - Critical failure (no snapshot service or persistence store)
    ///
    /// Callers should check the returned `PersistReport.failure_count` to detect
    /// partial failures where some components failed to persist.
    pub fn persist(&self) -> Result<crate::core::persistence::PersistReport, String> {
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

    /// Auto-attach sources and sinks from SQL WITH definitions
    ///
    /// Processes StreamDefinitions that have SQL WITH clauses and automatically
    /// creates source/sink handlers based on the WITH configuration.
    ///
    /// This completes the end-to-end flow:
    /// SQL parsing → StreamDefinition.with_config → StreamTypeConfig → Factory initialization
    ///
    /// # Priority
    ///
    /// This is called AFTER YAML config processing in start(), so SQL WITH has higher
    /// priority. If a stream is defined in both YAML and SQL WITH, SQL configuration wins
    /// due to idempotent handler registration (first registration stays).
    ///
    /// # Idempotent Operation
    ///
    /// Safe to call multiple times - existing handlers are not recreated.
    ///
    /// # Error Handling
    ///
    /// Uses error accumulation pattern - continues processing all streams even if some fail.
    /// Returns all errors encountered for debugging.
    ///
    /// # Returns
    ///
    /// * `Ok((sources, sinks))` - Lists of successfully attached stream names
    /// * `Err(errors)` - All errors encountered (partial success possible)
    fn auto_attach_from_sql_definitions(
        &self,
    ) -> Result<(Vec<String>, Vec<String>), Vec<crate::core::exception::EventFluxError>> {
        let mut errors = Vec::new();
        let mut attached_sources = Vec::new();
        let mut attached_sinks = Vec::new();

        // Iterate through all stream definitions from the parsed SQL application
        for (stream_id, stream_def) in &self.eventflux_app.stream_definition_map {
            // Skip streams without SQL WITH configuration
            let with_config = match &stream_def.with_config {
                Some(config) => config,
                None => continue, // Internal stream or no WITH clause
            };

            // Determine stream type from configuration
            let stream_type = match with_config.get("type") {
                Some(t) => t.as_str(),
                None => {
                    // No type specified - this is an internal stream, skip
                    continue;
                }
            };

            // Process based on stream type
            match stream_type {
                "source" => {
                    match self.attach_single_stream_from_sql_source(stream_id, with_config) {
                        Ok(()) => {
                            attached_sources.push(stream_id.clone());
                        }
                        Err(e) => {
                            log::error!(
                                "[EventFluxAppRuntime] Failed to attach SQL source '{}': {}",
                                stream_id, e
                            );
                            errors.push(e);
                        }
                    }
                }
                "sink" => match self.attach_single_stream_from_sql_sink(stream_id, with_config) {
                    Ok(()) => {
                        attached_sinks.push(stream_id.clone());
                    }
                    Err(e) => {
                        log::error!(
                            "[EventFluxAppRuntime] Failed to attach SQL sink '{}': {}",
                            stream_id, e
                        );
                        errors.push(e);
                    }
                },
                "internal" => {
                    // Internal streams don't need source/sink attachment
                    continue;
                }
                other => {
                    errors.push(crate::core::exception::EventFluxError::configuration(format!(
                        "Invalid stream type '{}' for stream '{}'. Expected: 'source', 'sink', or 'internal'",
                        other, stream_id
                    )));
                }
            }
        }

        // Fail fast - no partial success allowed
        if !errors.is_empty() {
            Err(errors)
        } else {
            Ok((attached_sources, attached_sinks))
        }
    }

    /// Common helper for attaching source streams
    ///
    /// This method contains the shared logic between YAML and SQL source attachment.
    /// It performs the actual source creation, handler registration, and startup.
    ///
    /// # Arguments
    ///
    /// * `stream_name` - Name of the stream to attach
    /// * `stream_type_config` - Already-constructed StreamTypeConfig
    /// * `context_label` - Label for error messages ("YAML", "SQL", etc.)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Source successfully attached and started
    /// * `Err(EventFluxError)` - Detailed error with context
    fn attach_source_common(
        &self,
        stream_name: &str,
        stream_type_config: &crate::core::config::stream_config::StreamTypeConfig,
        context_label: &str,
    ) -> Result<(), crate::core::exception::EventFluxError> {
        use crate::core::exception::EventFluxError;
        use crate::core::stream::handler::SourceStreamHandler;
        use crate::core::stream::stream_initializer::{initialize_stream, InitializedStream};

        // Check if handler already registered (idempotent operation)
        if let Some(existing_handler) = self.get_source_handler(stream_name) {
            // Handler already exists - just ensure it's started
            existing_handler.start()?;
            return Ok(());
        }

        // Use stream_initializer to create source with mapper
        let initialized = initialize_stream(
            &self.eventflux_app_context.eventflux_context,
            stream_type_config,
        )
        .map_err(|e| {
            EventFluxError::app_creation(format!(
                "Failed to initialize {} source '{}': {}",
                context_label, stream_name, e
            ))
        })?;

        // Extract source and mapper from initialized stream
        let (source, mapper) = match initialized {
            InitializedStream::Source(init_source) => {
                (init_source.source, Some(init_source.mapper))
            }
            _ => {
                return Err(EventFluxError::app_creation(format!(
                    "Expected source stream initialization for {} stream '{}', got different stream type",
                    context_label, stream_name
                )))
            }
        };

        // Get or create InputHandler for this stream
        let input_handler = self
            .input_manager
            .construct_input_handler(stream_name)
            .map_err(|e| {
                EventFluxError::app_creation(format!(
                    "Failed to construct InputHandler for {} source stream '{}': {}",
                    context_label, stream_name, e
                ))
            })?;

        // Create SourceStreamHandler
        let handler = Arc::new(SourceStreamHandler::new(
            source,
            mapper,
            input_handler,
            stream_name.to_string(),
        ));

        // Register handler in runtime
        self.register_source_handler(stream_name.to_string(), Arc::clone(&handler));

        // Start the source
        handler.start().map_err(|e| {
            EventFluxError::app_runtime(format!(
                "Failed to start {} source '{}': {}",
                context_label, stream_name, e
            ))
        })?;

        Ok(())
    }

    /// Attach a single source stream from SQL WITH configuration
    ///
    /// Converts SQL WITH configuration to StreamTypeConfig and delegates to common helper.
    ///
    /// # Arguments
    ///
    /// * `stream_name` - Name of the stream to attach
    /// * `with_config` - SQL WITH configuration from StreamDefinition
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Source successfully attached and started
    /// * `Err(EventFluxError)` - Detailed error with context
    fn attach_single_stream_from_sql_source(
        &self,
        stream_name: &str,
        with_config: &crate::core::config::stream_config::FlatConfig,
    ) -> Result<(), crate::core::exception::EventFluxError> {
        use crate::core::config::stream_config::{StreamType, StreamTypeConfig};
        use crate::core::exception::EventFluxError;

        // Extract required properties from SQL WITH configuration
        let extension = with_config
            .get("extension")
            .ok_or_else(|| {
                EventFluxError::configuration(format!(
                    "Missing 'extension' property in SQL WITH clause for source stream '{}'. \
                     Source streams require: type='source', extension='<name>', format='<type>'",
                    stream_name
                ))
            })?
            .clone();

        let format = with_config.get("format").cloned();

        // Create StreamTypeConfig from SQL WITH properties
        // Key advantage: FlatConfig.properties is already HashMap<String, String>!
        // No conversion needed unlike YAML which requires extract_connection_config()
        let stream_type_config = StreamTypeConfig::new(
            StreamType::Source,
            Some(extension.clone()),
            format.clone(),
            with_config.properties().clone(), // Direct use of properties HashMap via getter
        )
        .map_err(|e| {
            EventFluxError::configuration(format!(
                "Invalid StreamTypeConfig for SQL source '{}' (extension={}, format={:?}): {}",
                stream_name, extension, format, e
            ))
        })?;

        // Delegate to common helper for actual attachment
        self.attach_source_common(stream_name, &stream_type_config, "SQL")?;

        log::info!(
            "[EventFluxAppRuntime] Auto-attached SQL source '{}' (extension={}, format={:?})",
            stream_name, extension, format
        );

        Ok(())
    }

    /// Attach a single sink stream from SQL WITH configuration
    ///
    /// Similar to attach_single_stream_from_sql_source() but for sinks.
    ///
    /// # Arguments
    ///
    /// * `stream_name` - Name of the stream to attach
    /// * `with_config` - SQL WITH configuration from StreamDefinition
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Sink successfully attached
    /// * `Err(EventFluxError)` - Detailed error with context
    fn attach_single_stream_from_sql_sink(
        &self,
        stream_name: &str,
        with_config: &crate::core::config::stream_config::FlatConfig,
    ) -> Result<(), crate::core::exception::EventFluxError> {
        use crate::core::config::stream_config::{StreamType, StreamTypeConfig};
        use crate::core::exception::EventFluxError;
        use crate::core::stream::handler::SinkStreamHandler;
        use crate::core::stream::stream_initializer::{initialize_stream, InitializedStream};

        // Check if handler already registered (idempotent operation)
        if self.get_sink_handler(stream_name).is_some() {
            // Handler already exists - sinks don't need explicit start
            return Ok(());
        }

        // Extract required properties from SQL WITH configuration
        let extension = with_config
            .get("extension")
            .ok_or_else(|| {
                EventFluxError::configuration(format!(
                    "Missing 'extension' property in SQL WITH clause for sink stream '{}'. \
                     Sink streams require: type='sink', extension='<name>', format='<type>'",
                    stream_name
                ))
            })?
            .clone();

        let format = with_config.get("format").cloned();

        // Create StreamTypeConfig from SQL WITH properties
        let stream_type_config = StreamTypeConfig::new(
            StreamType::Sink,
            Some(extension.clone()),
            format.clone(),
            with_config.properties().clone(), // Direct use of properties HashMap via getter
        )
        .map_err(|e| {
            EventFluxError::configuration(format!(
                "Invalid StreamTypeConfig for SQL sink '{}' (extension={}, format={:?}): {}",
                stream_name, extension, format, e
            ))
        })?;

        // Use stream_initializer to create sink with mapper
        let initialized = initialize_stream(
            &self.eventflux_app_context.eventflux_context,
            &stream_type_config,
        )
        .map_err(|e| {
            EventFluxError::app_creation(format!(
                "Failed to initialize SQL sink '{}' (extension={}, format={:?}): {}",
                stream_name, extension, format, e
            ))
        })?;

        // Extract sink and mapper from initialized stream
        let (sink, mapper) = match initialized {
            InitializedStream::Sink(init_sink) => (init_sink.sink, Some(init_sink.mapper)),
            _ => {
                return Err(EventFluxError::app_creation(format!(
                    "Expected sink stream initialization for SQL stream '{}', got different stream type",
                    stream_name
                )))
            }
        };

        // Create SinkStreamHandler
        let handler = Arc::new(SinkStreamHandler::new(
            sink,
            mapper,
            stream_name.to_string(),
        ));

        // Register handler in runtime
        self.register_sink_handler(stream_name.to_string(), Arc::clone(&handler));

        // Attach sink to junction for event delivery
        // Uses existing attach_sink_to_junction which properly converts Sink to StreamCallback
        self.attach_sink_to_junction(stream_name, Arc::clone(&handler))?;

        log::info!(
            "[EventFluxAppRuntime] Auto-attached SQL sink '{}' (extension={}, format={:?})",
            stream_name, extension, format
        );

        Ok(())
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
                        log::info!(
                            "[EventFluxAppRuntime] Successfully attached source '{}' to stream '{}'",
                            source_config.source_type, stream_name
                        );
                    }
                    Err(e) => {
                        log::error!(
                            "[EventFluxAppRuntime] Failed to attach source '{}' to stream '{}': {}",
                            source_config.source_type, stream_name, e
                        );
                        errors.push(e);
                    }
                }
            }
        }

        // Fail fast - no partial success allowed
        if !errors.is_empty() {
            Err(errors)
        } else {
            Ok(successes)
        }
    }

    /// Attach a single source stream handler
    ///
    /// Converts YAML SourceConfig to StreamTypeConfig and delegates to common helper.
    /// All errors are properly typed as EventFluxError with context.
    fn attach_single_source(
        &self,
        stream_name: &str,
        source_config: &crate::core::config::types::application_config::SourceConfig,
    ) -> Result<(), crate::core::exception::EventFluxError> {
        use crate::core::config::stream_config::{StreamType, StreamTypeConfig};
        use crate::core::exception::EventFluxError;

        // Convert SourceConfig to StreamTypeConfig
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

        // Delegate to common helper for actual attachment
        self.attach_source_common(stream_name, &stream_type_config, "YAML")
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
    ///
    /// CRITICAL FIX: Uses stream_initializer with EventFluxContext's registered factories
    /// instead of creating a fresh SinkFactoryRegistry that only has built-in sinks.
    /// This allows custom/extension sinks to be auto-attached from YAML/TOML configuration.
    fn auto_attach_sinks_from_config(&self, app_config: &ApplicationConfig) -> Result<(), String> {
        use crate::core::config::stream_config::{StreamType, StreamTypeConfig};
        use crate::core::exception::EventFluxError;
        use crate::core::stream::handler::SinkStreamHandler;
        use crate::core::stream::stream_initializer::{initialize_stream, InitializedStream};

        // Iterate through all configured streams
        for (stream_name, stream_config) in &app_config.streams {
            // Check if this stream has a sink configuration
            if let Some(ref sink_config) = stream_config.sink {
                // Check if sink handler already registered (idempotent operation)
                if self.get_sink_handler(stream_name).is_some() {
                    continue;
                }

                // Convert SinkConfig to StreamTypeConfig (similar to sources)
                let properties = self
                    .extract_connection_config(&sink_config.connection)
                    .map_err(|e| {
                        format!(
                            "Invalid connection config for sink stream '{}': {}",
                            stream_name, e
                        )
                    })?;

                let stream_type_config = StreamTypeConfig::new(
                    StreamType::Sink,
                    Some(sink_config.sink_type.clone()),
                    sink_config.format.clone(),
                    properties,
                )
                .map_err(|e| {
                    format!(
                        "Invalid StreamTypeConfig for sink '{}' (type={}): {}",
                        stream_name, sink_config.sink_type, e
                    )
                })?;

                // Use stream_initializer to create sink with mapper
                // This properly uses EventFluxContext's registered factories!
                let initialized = initialize_stream(
                    &self.eventflux_app_context.eventflux_context,
                    &stream_type_config,
                )
                .map_err(|e| {
                    format!(
                        "Failed to initialize YAML sink '{}' (type={}): {}",
                        stream_name, sink_config.sink_type, e
                    )
                })?;

                // Extract sink and mapper from initialized stream
                let (sink, mapper) = match initialized {
                    InitializedStream::Sink(init_sink) => {
                        (init_sink.sink, Some(init_sink.mapper))
                    }
                    _ => {
                        return Err(format!(
                            "Expected sink stream initialization for YAML stream '{}', got different stream type",
                            stream_name
                        ))
                    }
                };

                // Create SinkStreamHandler
                let handler = Arc::new(SinkStreamHandler::new(
                    sink,
                    mapper,
                    stream_name.to_string(),
                ));

                // Register handler in runtime
                self.register_sink_handler(stream_name.to_string(), Arc::clone(&handler));

                // Attach sink to junction for event delivery
                self.attach_sink_to_junction(stream_name, Arc::clone(&handler))?;

                log::info!(
                    "[EventFluxAppRuntime] Auto-attached YAML sink '{}' to stream '{}'",
                    sink_config.sink_type, stream_name
                );
            }
        }

        Ok(())
    }

    /// Auto-attach tables from configuration
    ///
    /// Initializes tables from TOML/YAML configuration and registers them in the runtime.
    /// Tables are passively queried and don't require lifecycle management like sources/sinks.
    ///
    /// # Arguments
    ///
    /// * `app_config` - Application configuration containing table definitions
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - List of successfully attached table names
    /// * `Err(Vec<EventFluxError>)` - List of all errors encountered
    fn auto_attach_tables_from_config(
        &self,
        app_config: &ApplicationConfig,
    ) -> Result<Vec<String>, Vec<crate::core::exception::EventFluxError>> {
        use crate::core::config::types::application_config::DefinitionConfig;
        use crate::core::exception::EventFluxError;

        let mut errors = Vec::new();
        let mut successes = Vec::new();

        // Iterate through all definitions, filtering for tables
        for (table_name, def_config) in &app_config.definitions {
            if let DefinitionConfig::Table(table_config) = def_config {
                // Check if handler already registered (idempotent operation)
                if self.get_table_handler(table_name).is_some() {
                    successes.push(table_name.clone());
                    continue;
                }

                // Process this table, collecting errors instead of failing fast
                match self.attach_single_table(table_name, table_config) {
                    Ok(()) => {
                        successes.push(table_name.clone());
                        log::info!(
                            "[EventFluxAppRuntime] Successfully attached table '{}' (extension={})",
                            table_name, table_config.store.store_type
                        );
                    }
                    Err(e) => {
                        log::error!(
                            "[EventFluxAppRuntime] Failed to attach table '{}': {}",
                            table_name, e
                        );
                        errors.push(e);
                    }
                }
            }
        }

        // Fail fast - no partial success allowed
        if !errors.is_empty() {
            Err(errors)
        } else {
            Ok(successes)
        }
    }

    /// Attach a single table handler from TableConfig
    ///
    /// Converts TableConfig (YAML config) to TableTypeConfig (initializer format)
    /// and registers the initialized table in the runtime.
    fn attach_single_table(
        &self,
        table_name: &str,
        table_config: &crate::core::config::types::application_config::TableConfig,
    ) -> Result<(), crate::core::exception::EventFluxError> {
        use crate::core::config::stream_config::TableTypeConfig;
        use crate::core::exception::EventFluxError;
        use crate::core::stream::stream_initializer::{initialize_table, InitializedStream};

        // Convert TableConfig to TableTypeConfig
        let table_type_config = Self::convert_table_config(table_config).map_err(|e| {
            EventFluxError::configuration(format!(
                "Failed to convert TableConfig for '{}': {}",
                table_name, e
            ))
        })?;

        // Initialize table using factory
        let initialized = initialize_table(
            &self.eventflux_app_context.eventflux_context,
            &table_type_config,
            table_name,
        )
        .map_err(|e| {
            EventFluxError::app_creation(format!(
                "Failed to initialize table '{}' (extension={}): {}",
                table_name, table_config.store.store_type, e
            ))
        })?;

        // Extract table from initialized result
        let table = match initialized {
            InitializedStream::Table(init_table) => init_table.table,
            _ => {
                return Err(EventFluxError::app_creation(format!(
                    "Expected table initialization for '{}', got different type",
                    table_name
                )))
            }
        };

        // Register table in runtime
        self.register_table_handler(table_name.to_string(), table);

        Ok(())
    }

    /// Convert TableConfig (YAML) to TableTypeConfig (initializer format)
    ///
    /// Maps the structured TableConfig with store/schema/caching/indexing
    /// to the flattened TableTypeConfig with extension and properties HashMap.
    fn convert_table_config(
        table_config: &crate::core::config::types::application_config::TableConfig,
    ) -> Result<crate::core::config::stream_config::TableTypeConfig, String> {
        use crate::core::config::stream_config::TableTypeConfig;

        // Extension comes from store type
        let extension = table_config.store.store_type.clone();

        // Flatten all configuration into properties HashMap
        let mut properties = HashMap::new();

        // Add extension property
        properties.insert("extension".to_string(), extension.clone());

        // Flatten store connection configuration
        Self::flatten_yaml_value(&table_config.store.connection, "", &mut properties)?;

        // Add pool configuration if present
        if let Some(ref pool) = table_config.store.pool {
            properties.insert(
                format!("{}.pool.max_size", extension),
                pool.max_size.to_string(),
            );
            properties.insert(
                format!("{}.pool.min_size", extension),
                pool.min_size.to_string(),
            );
            properties.insert(
                format!("{}.pool.connection_timeout_ms", extension),
                pool.connection_timeout.as_millis().to_string(),
            );
        }

        // Create TableTypeConfig with validation
        TableTypeConfig::new(extension, properties)
    }

    /// Flatten serde_yaml::Value into HashMap<String, String>
    ///
    /// Recursively flattens nested YAML structures using dot notation.
    /// For example: {mysql: {host: "localhost"}} becomes {"mysql.host": "localhost"}
    fn flatten_yaml_value(
        value: &serde_yaml::Value,
        prefix: &str,
        result: &mut HashMap<String, String>,
    ) -> Result<(), String> {
        match value {
            serde_yaml::Value::Mapping(map) => {
                for (key, val) in map {
                    let key_str = key
                        .as_str()
                        .ok_or_else(|| format!("Non-string key in YAML: {:?}", key))?;

                    let new_prefix = if prefix.is_empty() {
                        key_str.to_string()
                    } else {
                        format!("{}.{}", prefix, key_str)
                    };

                    Self::flatten_yaml_value(val, &new_prefix, result)?;
                }
            }
            serde_yaml::Value::String(s) => {
                result.insert(prefix.to_string(), s.clone());
            }
            serde_yaml::Value::Number(n) => {
                result.insert(prefix.to_string(), n.to_string());
            }
            serde_yaml::Value::Bool(b) => {
                result.insert(prefix.to_string(), b.to_string());
            }
            serde_yaml::Value::Null => {
                result.insert(prefix.to_string(), "null".to_string());
            }
            serde_yaml::Value::Sequence(seq) => {
                // For arrays, join with commas or create indexed keys
                let values: Vec<String> = seq
                    .iter()
                    .filter_map(|v| match v {
                        serde_yaml::Value::String(s) => Some(s.clone()),
                        serde_yaml::Value::Number(n) => Some(n.to_string()),
                        serde_yaml::Value::Bool(b) => Some(b.to_string()),
                        _ => None,
                    })
                    .collect();
                result.insert(prefix.to_string(), values.join(","));
            }
            serde_yaml::Value::Tagged(_) => {
                return Err(format!("Tagged YAML values not supported: {:?}", value));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::types::application_config::{StoreConfig, TableConfig};

    #[test]
    fn test_flatten_yaml_simple_mapping() {
        let mut connection = serde_yaml::Mapping::new();
        connection.insert(
            serde_yaml::Value::String("host".to_string()),
            serde_yaml::Value::String("localhost".to_string()),
        );
        connection.insert(
            serde_yaml::Value::String("port".to_string()),
            serde_yaml::Value::Number(3306.into()),
        );

        let mut result = HashMap::new();
        EventFluxAppRuntime::flatten_yaml_value(
            &serde_yaml::Value::Mapping(connection),
            "",
            &mut result,
        )
        .unwrap();

        assert_eq!(result.get("host"), Some(&"localhost".to_string()));
        assert_eq!(result.get("port"), Some(&"3306".to_string()));
    }

    #[test]
    fn test_flatten_yaml_nested_mapping() {
        let mut inner = serde_yaml::Mapping::new();
        inner.insert(
            serde_yaml::Value::String("host".to_string()),
            serde_yaml::Value::String("localhost".to_string()),
        );

        let mut outer = serde_yaml::Mapping::new();
        outer.insert(
            serde_yaml::Value::String("mysql".to_string()),
            serde_yaml::Value::Mapping(inner),
        );

        let mut result = HashMap::new();
        EventFluxAppRuntime::flatten_yaml_value(
            &serde_yaml::Value::Mapping(outer),
            "",
            &mut result,
        )
        .unwrap();

        assert_eq!(result.get("mysql.host"), Some(&"localhost".to_string()));
    }

    #[test]
    fn test_flatten_yaml_sequence() {
        let seq = vec![
            serde_yaml::Value::String("val1".to_string()),
            serde_yaml::Value::String("val2".to_string()),
            serde_yaml::Value::String("val3".to_string()),
        ];

        let mut result = HashMap::new();
        EventFluxAppRuntime::flatten_yaml_value(
            &serde_yaml::Value::Sequence(seq),
            "list",
            &mut result,
        )
        .unwrap();

        assert_eq!(result.get("list"), Some(&"val1,val2,val3".to_string()));
    }

    #[test]
    fn test_convert_table_config_inmemory() {
        let table_config = TableConfig {
            store: StoreConfig {
                store_type: "inMemory".to_string(),
                connection: serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
                pool: None,
                security: None,
            },
            schema: None,
            caching: None,
            indexing: None,
        };

        let result = EventFluxAppRuntime::convert_table_config(&table_config);
        assert!(result.is_ok());

        let table_type_config = result.unwrap();
        assert_eq!(table_type_config.extension(), "inMemory");
        assert!(table_type_config.properties.contains_key("extension"));
    }

    #[test]
    fn test_convert_table_config_with_connection() {
        let mut connection = serde_yaml::Mapping::new();
        connection.insert(
            serde_yaml::Value::String("host".to_string()),
            serde_yaml::Value::String("localhost".to_string()),
        );
        connection.insert(
            serde_yaml::Value::String("port".to_string()),
            serde_yaml::Value::Number(3306.into()),
        );
        connection.insert(
            serde_yaml::Value::String("database".to_string()),
            serde_yaml::Value::String("testdb".to_string()),
        );

        let table_config = TableConfig {
            store: StoreConfig {
                store_type: "mysql".to_string(),
                connection: serde_yaml::Value::Mapping(connection),
                pool: None,
                security: None,
            },
            schema: None,
            caching: None,
            indexing: None,
        };

        let result = EventFluxAppRuntime::convert_table_config(&table_config);
        assert!(result.is_ok());

        let table_type_config = result.unwrap();
        assert_eq!(table_type_config.extension(), "mysql");
        assert_eq!(
            table_type_config.properties.get("host"),
            Some(&"localhost".to_string())
        );
        assert_eq!(
            table_type_config.properties.get("port"),
            Some(&"3306".to_string())
        );
        assert_eq!(
            table_type_config.properties.get("database"),
            Some(&"testdb".to_string())
        );
    }

    #[test]
    fn test_convert_table_config_with_pool() {
        use crate::core::config::types::application_config::ConnectionPoolConfig;
        use std::time::Duration;

        let pool_config = ConnectionPoolConfig {
            max_size: 20,
            min_size: 5,
            connection_timeout: Duration::from_secs(60),
            idle_timeout: None,
            max_lifetime: None,
        };

        let table_config = TableConfig {
            store: StoreConfig {
                store_type: "postgres".to_string(),
                connection: serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
                pool: Some(pool_config),
                security: None,
            },
            schema: None,
            caching: None,
            indexing: None,
        };

        let result = EventFluxAppRuntime::convert_table_config(&table_config);
        assert!(result.is_ok());

        let table_type_config = result.unwrap();
        assert_eq!(table_type_config.extension(), "postgres");
        assert_eq!(
            table_type_config.properties.get("postgres.pool.max_size"),
            Some(&"20".to_string())
        );
        assert_eq!(
            table_type_config.properties.get("postgres.pool.min_size"),
            Some(&"5".to_string())
        );
        assert_eq!(
            table_type_config
                .properties
                .get("postgres.pool.connection_timeout_ms"),
            Some(&"60000".to_string())
        );
    }
}
