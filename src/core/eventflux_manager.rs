// SPDX-License-Identifier: MIT OR Apache-2.0

// eventflux_rust/src/core/eventflux_manager.rs
use crate::core::config::eventflux_context::EventFluxContext;
use crate::core::eventflux_app_runtime::EventFluxAppRuntime;
use crate::query_api::eventflux_app::EventFluxApp as ApiEventFluxApp;
// EventFluxAppContext is created by EventFluxAppRuntime::new or its internal parser logic
// use crate::core::config::eventflux_app_context::EventFluxAppContext;

// Use SQL compiler for parsing (sqlparser-rs based)
use crate::core::executor::ScalarFunctionExecutor; // Added for UDFs
use crate::core::DataSource;
// Placeholder for actual persistence store trait/type
use crate::core::config::eventflux_context::ExtensionClassPlaceholder;
use crate::core::config::{ApplicationConfig, ConfigManager, EventFluxConfig};
use crate::core::extension;
use crate::core::persistence::PersistenceStore;

use libloading::Library;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
// use rand::Rng; // For generating random app names if not specified

/// Main entry point for managing EventFlux application runtimes.
#[repr(C)]
pub struct EventFluxManager {
    eventflux_context: Arc<EventFluxContext>,
    eventflux_app_runtime_map: Arc<Mutex<HashMap<String, Arc<EventFluxAppRuntime>>>>,
    loaded_libraries: Arc<Mutex<Vec<std::mem::ManuallyDrop<libloading::Library>>>>,
    config_manager: Option<Arc<ConfigManager>>,
    cached_config: Arc<Mutex<Option<EventFluxConfig>>>,
}

impl EventFluxManager {
    pub fn new() -> Self {
        Self {
            eventflux_context: Arc::new(EventFluxContext::new()), // EventFluxContext::new() provides defaults
            eventflux_app_runtime_map: Arc::new(Mutex::new(HashMap::new())),
            loaded_libraries: Arc::new(Mutex::new(Vec::new())),
            config_manager: None,
            cached_config: Arc::new(Mutex::new(None)),
        }
    }

    /// Create EventFluxManager with a specific configuration
    pub fn new_with_config(config: EventFluxConfig) -> Self {
        let manager = Self::new();
        *manager.cached_config.lock().expect("Mutex poisoned") = Some(config);
        manager
    }

    /// Create EventFluxManager with ConfigManager for dynamic configuration loading
    pub fn new_with_config_manager(config_manager: ConfigManager) -> Self {
        Self {
            eventflux_context: Arc::new(EventFluxContext::new()),
            eventflux_app_runtime_map: Arc::new(Mutex::new(HashMap::new())),
            loaded_libraries: Arc::new(Mutex::new(Vec::new())),
            config_manager: Some(Arc::new(config_manager)),
            cached_config: Arc::new(Mutex::new(None)),
        }
    }

    pub fn eventflux_context(&self) -> Arc<EventFluxContext> {
        Arc::clone(&self.eventflux_context)
    }

    pub async fn create_eventflux_app_runtime_from_string(
        &self,
        eventflux_app_string: &str,
    ) -> Result<Arc<EventFluxAppRuntime>, String> {
        // 1. Parse SQL string to SqlApplication (using sql_compiler)
        let sql_app = crate::sql_compiler::parse_sql_application(eventflux_app_string)
            .map_err(|e| format!("SQL parse error: {}", e))?;

        // 2. Determine application name
        // Priority: YAML config > stream-based name > default name
        let app_name = if let Some(config_name) = self.get_config_app_name().await {
            config_name
        } else {
            // Fallback to stream-based naming if no YAML config
            sql_app
                .catalog
                .get_stream_names()
                .first()
                .map(|s| format!("App_{}", s))
                .unwrap_or_else(|| "EventFluxApp".to_string())
        };

        // 3. Convert to EventFluxApp
        let api_eventflux_app = sql_app
            .to_eventflux_app(app_name)
            .map_err(|e| format!("Type inference error: {}", e))?;
        let api_eventflux_app_arc = Arc::new(api_eventflux_app);

        // 4. Create runtime from ApiEventFluxApp
        self.create_eventflux_app_runtime_from_api(
            api_eventflux_app_arc,
            Some(eventflux_app_string.to_string()),
        )
        .await
    }

    // Added eventflux_app_str_opt for cases where it's available (from string parsing)
    // or not (when an ApiEventFluxApp is provided directly).
    pub async fn create_eventflux_app_runtime_from_api(
        &self,
        api_eventflux_app: Arc<ApiEventFluxApp>,
        eventflux_app_str_opt: Option<String>,
    ) -> Result<Arc<EventFluxAppRuntime>, String> {
        // Extract app name from annotations or generate one
        // This logic is similar to Java's EventFluxAppParser.
        let app_name = api_eventflux_app
            .annotations
            .iter()
            .find(|ann| ann.name == crate::query_api::constants::ANNOTATION_INFO)
            .and_then(|ann| {
                ann.elements
                    .iter()
                    .find(|el| el.key == crate::query_api::constants::ANNOTATION_ELEMENT_NAME)
            })
            .map(|el| el.value.clone())
            .unwrap_or_else(|| format!("eventflux_app_{}", uuid::Uuid::new_v4().hyphenated()));

        // Check if app with this name already exists
        if self
            .eventflux_app_runtime_map
            .lock()
            .expect("Mutex poisoned")
            .contains_key(&app_name)
        {
            return Err(format!(
                "EventFluxApp with name '{app_name}' already exists."
            ));
        }

        // Get application-specific configuration
        let app_config = self.get_application_config(&app_name).await?;

        // EventFluxAppRuntime::new now handles creation of EventFluxAppContext and parsing
        let runtime = Arc::new(EventFluxAppRuntime::new_with_config(
            Arc::clone(&api_eventflux_app),
            Arc::clone(&self.eventflux_context),
            eventflux_app_str_opt.clone(),
            app_config,
        )?);
        runtime.start();

        self.eventflux_app_runtime_map
            .lock()
            .expect("Mutex poisoned")
            .insert(app_name.clone(), Arc::clone(&runtime)); // Use app_name for map key
        Ok(runtime)
    }

    pub fn get_eventflux_app_runtime(&self, app_name: &str) -> Option<Arc<EventFluxAppRuntime>> {
        self.eventflux_app_runtime_map
            .lock()
            .expect("Mutex poisoned")
            .get(app_name)
            .map(Arc::clone)
    }

    pub fn get_eventflux_app_runtimes_keys(&self) -> Vec<String> {
        self.eventflux_app_runtime_map
            .lock()
            .expect("Mutex poisoned")
            .keys()
            .cloned()
            .collect()
    }

    pub fn shutdown_app_runtime(&self, app_name: &str) -> bool {
        if let Some(runtime) = self
            .eventflux_app_runtime_map
            .lock()
            .expect("Mutex poisoned")
            .remove(app_name)
        {
            runtime.shutdown();
            true
        } else {
            false
        }
    }

    pub fn shutdown_all_app_runtimes(&self) {
        let mut map = self
            .eventflux_app_runtime_map
            .lock()
            .expect("Mutex poisoned");
        for runtime in map.values() {
            runtime.shutdown();
        }
        map.clear();
    }

    // --- Setters for EventFluxContext components (placeholders) ---
    // `set_extension` typically refers to general extensions (FunctionExecutor, StreamProcessor etc.)
    // For scalar functions, we use `add_scalar_function_factory`.
    pub fn set_extension(
        &self,
        name: &str,
        library_path: ExtensionClassPlaceholder,
    ) -> Result<(), String> {
        unsafe {
            let lib = Library::new(&library_path).map_err(|e| e.to_string())?;

            macro_rules! call_if_exists {
                ($sym:expr) => {
                    if let Ok(f) = lib.get::<extension::RegisterFn>($sym) {
                        f(self);
                    }
                };
            }

            call_if_exists!(extension::REGISTER_EXTENSION_FN);
            call_if_exists!(extension::REGISTER_WINDOWS_FN);
            call_if_exists!(extension::REGISTER_FUNCTIONS_FN);
            call_if_exists!(extension::REGISTER_SOURCES_FN);
            call_if_exists!(extension::REGISTER_SINKS_FN);
            call_if_exists!(extension::REGISTER_STORES_FN);
            call_if_exists!(extension::REGISTER_SOURCE_MAPPERS_FN);
            call_if_exists!(extension::REGISTER_SINK_MAPPERS_FN);

            self.loaded_libraries
                .lock()
                .unwrap()
                .push(std::mem::ManuallyDrop::new(lib));
        }

        println!("[EventFluxManager] dynamically loaded extension '{name}' from {library_path}");
        Ok(())
    }

    // Specific method for adding scalar function factories
    pub fn add_scalar_function_factory(
        &self,
        name: String,
        function_factory: Box<dyn ScalarFunctionExecutor>,
    ) {
        self.eventflux_context
            .add_scalar_function_factory(name, function_factory);
    }

    pub fn add_window_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::WindowProcessorFactory>,
    ) {
        self.eventflux_context.add_window_factory(name, factory);
    }

    pub fn add_attribute_aggregator_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::AttributeAggregatorFactory>,
    ) {
        self.eventflux_context
            .add_attribute_aggregator_factory(name, factory);
    }

    pub fn add_source_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::SourceFactory>,
    ) {
        self.eventflux_context.add_source_factory(name, factory);
    }

    pub fn add_sink_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::SinkFactory>,
    ) {
        self.eventflux_context.add_sink_factory(name, factory);
    }

    pub fn add_store_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::StoreFactory>,
    ) {
        self.eventflux_context.add_store_factory(name, factory);
    }

    pub fn add_source_mapper_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::SourceMapperFactory>,
    ) {
        self.eventflux_context
            .add_source_mapper_factory(name, factory);
    }

    pub fn add_sink_mapper_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::SinkMapperFactory>,
    ) {
        self.eventflux_context
            .add_sink_mapper_factory(name, factory);
    }

    pub fn add_table_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::TableFactory>,
    ) {
        self.eventflux_context.add_table_factory(name, factory);
    }

    // Specific method for adding data sources
    pub fn add_data_source(
        &self,
        name: String,
        data_source: Arc<dyn DataSource>,
    ) -> Result<(), String> {
        self.eventflux_context.add_data_source(name, data_source)
    }

    // set_data_source was a placeholder, replaced by add_data_source
    // pub fn set_data_source(&self, name: &str, ds_placeholder: DataSourcePlaceholder) -> Result<(), String> { ... }

    pub fn set_persistence_store(&self, store: Arc<dyn PersistenceStore>) {
        self.eventflux_context.set_persistence_store(store);
    }

    /// Set the configuration manager for dynamic configuration loading
    pub fn set_config_manager(&mut self, config_manager: ConfigManager) -> Result<(), String> {
        self.config_manager = Some(Arc::new(config_manager));
        // Clear cached config to force reload on next access
        *self
            .cached_config
            .lock()
            .map_err(|e| format!("Mutex poisoned: {}", e))? = None;
        Ok(())
    }

    /// Get the current configuration, loading it if necessary
    pub async fn get_config(&self) -> Result<EventFluxConfig, String> {
        // Check if we have a cached config
        {
            let cached = self
                .cached_config
                .lock()
                .map_err(|e| format!("Mutex poisoned: {}", e))?;
            if let Some(ref config) = *cached {
                return Ok(config.clone());
            }
        }

        // Load configuration using ConfigManager if available
        if let Some(ref config_manager) = self.config_manager {
            let config = config_manager
                .load_unified_config()
                .await
                .map_err(|e| format!("Failed to load configuration: {}", e))?;

            // Cache the loaded configuration
            *self
                .cached_config
                .lock()
                .map_err(|e| format!("Mutex poisoned: {}", e))? = Some(config.clone());

            Ok(config)
        } else {
            // Return default configuration if no ConfigManager is set
            let default_config = EventFluxConfig::default();
            *self
                .cached_config
                .lock()
                .map_err(|e| format!("Mutex poisoned: {}", e))? = Some(default_config.clone());
            Ok(default_config)
        }
    }

    /// Get application-specific configuration by name
    pub async fn get_application_config(
        &self,
        app_name: &str,
    ) -> Result<Option<ApplicationConfig>, String> {
        let config = self.get_config().await?;
        Ok(config.applications.get(app_name).cloned())
    }

    /// Get the application name from YAML configuration
    ///
    /// Returns the application name from `eventflux.application.name` if configured,
    /// otherwise returns None.
    async fn get_config_app_name(&self) -> Option<String> {
        if let Ok(config) = self.get_config().await {
            config
                .eventflux
                .application
                .as_ref()
                .and_then(|app| app.name.clone())
        } else {
            None
        }
    }

    // --- Validation ---
    pub fn validate_eventflux_app_string(&self, eventflux_app_string: &str) -> Result<(), String> {
        // Parse SQL string to SqlApplication
        let sql_app = crate::sql_compiler::parse_sql_application(eventflux_app_string)
            .map_err(|e| format!("SQL parse error: {}", e))?;

        // Convert to EventFluxApp
        let app_name = sql_app
            .catalog
            .get_stream_names()
            .first()
            .map(|s| format!("App_{}", s))
            .unwrap_or_else(|| "EventFluxApp".to_string());
        let api_eventflux_app = sql_app
            .to_eventflux_app(app_name)
            .map_err(|e| format!("Type inference error: {}", e))?;

        self.validate_eventflux_app_api(
            &Arc::new(api_eventflux_app),
            Some(eventflux_app_string.to_string()),
        )
    }

    pub fn validate_eventflux_app_api(
        &self,
        api_eventflux_app: &Arc<ApiEventFluxApp>,
        eventflux_app_str_opt: Option<String>,
    ) -> Result<(), String> {
        // Create a temporary runtime to validate
        // Note: This doesn't add it to the main eventflux_app_runtime_map
        let temp_runtime = EventFluxAppRuntime::new(
            Arc::clone(api_eventflux_app),
            Arc::clone(&self.eventflux_context),
            eventflux_app_str_opt,
        )?;
        temp_runtime.start(); // This would internally build the plan, run initializations
        temp_runtime.shutdown(); // Clean up
        Ok(())
    }

    // --- Persistence ---
    pub fn persist_app(&self, app_name: &str) -> Result<String, String> {
        let map = self
            .eventflux_app_runtime_map
            .lock()
            .expect("Mutex poisoned");
        let rt = map
            .get(app_name)
            .ok_or_else(|| format!("EventFluxApp '{app_name}' not found"))?;
        rt.persist()
    }

    pub fn restore_app_revision(&self, app_name: &str, revision: &str) -> Result<(), String> {
        let map = self
            .eventflux_app_runtime_map
            .lock()
            .expect("Mutex poisoned");
        let rt = map
            .get(app_name)
            .ok_or_else(|| format!("EventFluxApp '{app_name}' not found"))?;
        rt.restore_revision(revision)
    }

    pub fn persist_all(&self) {
        for runtime in self
            .eventflux_app_runtime_map
            .lock()
            .expect("Mutex poisoned")
            .values()
        {
            let _ = runtime.persist();
        }
    }

    pub fn restore_all_last_state(&self) {
        for runtime in self
            .eventflux_app_runtime_map
            .lock()
            .expect("Mutex poisoned")
            .values()
        {
            if let Some(service) = runtime.eventflux_app_context.get_snapshot_service() {
                if let Some(store) = &service.persistence_store {
                    if let Some(last) = store.get_last_revision(&runtime.name) {
                        let _ = runtime.restore_revision(&last);
                    }
                }
            }
        }
    }

    /// Level 3 API: Create EventFluxAppRuntime from SQL application string
    ///
    /// This is the high-level API for creating a runtime directly from SQL.
    /// Internally uses the SQL compiler to parse and convert to EventFluxApp.
    ///
    /// # Example
    /// ```rust,ignore
    /// let manager = EventFluxManager::new();
    /// let sql = r#"
    ///     CREATE STREAM StockStream (symbol VARCHAR, price DOUBLE);
    ///
    ///     SELECT symbol, price
    ///     FROM StockStream
    ///     WHERE price > 100;
    /// "#;
    ///
    /// let runtime = manager.create_runtime_from_sql(sql, Some("MyApp".to_string())).await?;
    /// runtime.start();
    /// ```
    pub async fn create_runtime_from_sql(
        &self,
        sql: &str,
        app_name: Option<String>,
    ) -> Result<Arc<EventFluxAppRuntime>, String> {
        // Parse SQL application
        let sql_app = crate::sql_compiler::parse_sql_application(sql)
            .map_err(|e| format!("SQL parse error: {}", e))?;

        // Convert to EventFluxApp
        let app_name = app_name.unwrap_or_else(|| "SqlApp".to_string());
        let eventflux_app = sql_app
            .to_eventflux_app(app_name)
            .map_err(|e| format!("Type inference error: {}", e))?;

        // Create runtime using existing method
        self.create_eventflux_app_runtime_from_api(Arc::new(eventflux_app), Some(sql.to_string()))
            .await
    }
}

impl Default for EventFluxManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for EventFluxManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventFluxManager")
            .field("eventflux_context", &"EventFluxContext")
            .field(
                "app_runtimes",
                &self.eventflux_app_runtime_map.lock().unwrap().len(),
            )
            .finish()
    }
}

// Helper on ApiEventFluxApp for getting name (if not already part of query_api)
// This is more robust than assuming api_eventflux_app.name
impl ApiEventFluxApp {
    fn get_name(&self) -> Option<String> {
        self.annotations
            .iter()
            .find(|ann| ann.name == crate::query_api::constants::ANNOTATION_INFO)
            .and_then(|ann| {
                ann.elements
                    .iter()
                    .find(|el| el.key == crate::query_api::constants::ANNOTATION_ELEMENT_NAME)
            })
            .map(|el| el.value.clone())
    }
}
