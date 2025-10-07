// Corresponds to io.eventflux.core.config.EventFluxContext
use std::collections::HashMap;
use std::sync::{Arc, RwLockReadGuard}; // For shared, thread-safe components, Added RwLockReadGuard

use super::statistics_configuration::StatisticsConfiguration;
use crate::core::util::executor_service::ExecutorService;

// --- Placeholders for complex types/trait objects ---
// These would be defined in their respective modules (e.g., util::persistence, extension, etc.)
// For now, using simple struct placeholders or type aliases.

// pub trait Extension {} // Example
// pub type ExtensionType = Box<dyn Extension + Send + Sync>; // Example for dynamic dispatch
pub type ExtensionClassPlaceholder = String; // Simplified: Class name as String

// pub trait DataSource {} // Example
// pub type DataSourceType = Arc<dyn DataSource + Send + Sync>; // Example
pub type DataSourcePlaceholder = String; // Simplified

use crate::core::persistence::{IncrementalPersistenceStore, PersistenceStore};

use crate::core::stream::output::error_store::ErrorStore;

use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::persistence::data_source::{DataSource, DataSourceConfig}; // Using actual DataSource trait
                                                                           // use crate::query_api::eventflux_app::EventFluxApp; // TODO: Will be used when implementing app management
                                                                           // pub trait ConfigManager {} // Example
                                                                           // pub type ConfigManagerType = Arc<dyn ConfigManager + Send + Sync>;
pub type ConfigManagerPlaceholder = String; // Simplified

// pub trait SinkHandlerManager {} // Example
// pub type SinkHandlerManagerType = Arc<dyn SinkHandlerManager + Send + Sync>;
pub type SinkHandlerManagerPlaceholder = String; // Simplified

// pub trait SourceHandlerManager {} // Example
// pub type SourceHandlerManagerType = Arc<dyn SourceHandlerManager + Send + Sync>;
pub type SourceHandlerManagerPlaceholder = String; // Simplified

// pub trait RecordTableHandlerManager {} // Example
// pub type RecordTableHandlerManagerType = Arc<dyn RecordTableHandlerManager + Send + Sync>;
pub type RecordTableHandlerManagerPlaceholder = String; // Simplified

// pub trait AbstractExtensionHolder {} // Example
// pub type AbstractExtensionHolderType = Arc<dyn AbstractExtensionHolder + Send + Sync>;
pub type AbstractExtensionHolderPlaceholder = String; // Simplified

// Using String for attributes for now, could be an enum or serde_json::Value
pub type AttributeValuePlaceholder = String;

// Placeholder for Disruptor's ExceptionHandler<Object>
pub type DisruptorExceptionHandlerPlaceholder = String;

use crate::core::executor::function::ScalarFunctionExecutor; // Added
use crate::core::table::Table;
use std::sync::RwLock; // Added for scalar_function_factories and attributes, data_sources

/// Shared context for all EventFlux Apps in a EventFluxManager instance.
// Custom Debug is implemented to avoid requiring Debug on trait objects.
#[repr(C)]
pub struct EventFluxContext {
    pub statistics_configuration: StatisticsConfiguration,
    attributes: Arc<RwLock<HashMap<String, AttributeValuePlaceholder>>>,

    // --- Placeholders mirroring Java fields ---
    eventflux_extensions: HashMap<String, ExtensionClassPlaceholder>,
    deprecated_eventflux_extensions: HashMap<String, ExtensionClassPlaceholder>,
    persistence_store: Arc<RwLock<Option<Arc<dyn PersistenceStore>>>>,
    incremental_persistence_store: Arc<RwLock<Option<Arc<dyn IncrementalPersistenceStore>>>>,
    error_store: Option<Arc<dyn ErrorStore>>,
    extension_holder_map: HashMap<String, AbstractExtensionHolderPlaceholder>,
    config_manager: ConfigManagerPlaceholder,
    sink_handler_manager: Option<SinkHandlerManagerPlaceholder>,
    source_handler_manager: Option<SourceHandlerManagerPlaceholder>,
    record_table_handler_manager: Option<RecordTableHandlerManagerPlaceholder>,
    default_disrupter_exception_handler: DisruptorExceptionHandlerPlaceholder,

    /// Default buffer size for StreamJunctions created when parsing EventFlux apps.
    default_junction_buffer_size: usize,
    /// Whether StreamJunctions should run asynchronously by default when no
    /// annotation overrides the mode.
    default_junction_async: bool,

    // Actual fields for extensions and data sources
    /// Stores factories for User-Defined Scalar Functions. Key: "namespace:name" or "name", Value: clonable factory instance.
    scalar_function_factories: Arc<RwLock<HashMap<String, Box<dyn ScalarFunctionExecutor>>>>,
    /// Window processor factories
    window_factories:
        Arc<RwLock<HashMap<String, Box<dyn crate::core::extension::WindowProcessorFactory>>>>,
    /// Attribute aggregator factories
    attribute_aggregator_factories:
        Arc<RwLock<HashMap<String, Box<dyn crate::core::extension::AttributeAggregatorFactory>>>>,
    /// Source factories
    source_factories: Arc<RwLock<HashMap<String, Box<dyn crate::core::extension::SourceFactory>>>>,
    /// Sink factories
    sink_factories: Arc<RwLock<HashMap<String, Box<dyn crate::core::extension::SinkFactory>>>>,
    /// Store factories
    store_factories: Arc<RwLock<HashMap<String, Box<dyn crate::core::extension::StoreFactory>>>>,
    /// Source mapper factories
    source_mapper_factories:
        Arc<RwLock<HashMap<String, Box<dyn crate::core::extension::SourceMapperFactory>>>>,
    /// Sink mapper factories
    sink_mapper_factories:
        Arc<RwLock<HashMap<String, Box<dyn crate::core::extension::SinkMapperFactory>>>>,
    /// Table factories for creating table implementations
    table_factories: Arc<RwLock<HashMap<String, Box<dyn crate::core::extension::TableFactory>>>>,
    /// Configurations for data sources keyed by name
    data_source_configs: Arc<RwLock<HashMap<String, DataSourceConfig>>>,
    /// Stores registered data sources. Key: data source name.
    data_sources: Arc<RwLock<HashMap<String, Arc<dyn DataSource>>>>,
    /// Registered tables available for queries.
    tables: Arc<RwLock<HashMap<String, Arc<dyn Table>>>>,

    /// Named executor services for asynchronous processing.
    pub executor_services: Arc<crate::core::util::executor_service::ExecutorServiceRegistry>,

    pub dummy_field_eventflux_context_extensions: String,
}

impl EventFluxContext {
    pub fn new() -> Self {
        let mut ctx = Self {
            statistics_configuration: StatisticsConfiguration::default(),
            attributes: Arc::new(RwLock::new(HashMap::new())),
            scalar_function_factories: Arc::new(RwLock::new(HashMap::new())),
            window_factories: Arc::new(RwLock::new(HashMap::new())),
            attribute_aggregator_factories: Arc::new(RwLock::new(HashMap::new())),
            source_factories: Arc::new(RwLock::new(HashMap::new())),
            sink_factories: Arc::new(RwLock::new(HashMap::new())),
            store_factories: Arc::new(RwLock::new(HashMap::new())),
            source_mapper_factories: Arc::new(RwLock::new(HashMap::new())),
            sink_mapper_factories: Arc::new(RwLock::new(HashMap::new())),
            table_factories: Arc::new(RwLock::new(HashMap::new())),
            data_source_configs: Arc::new(RwLock::new(HashMap::new())),
            data_sources: Arc::new(RwLock::new(HashMap::new())),
            tables: Arc::new(RwLock::new(HashMap::new())),
            executor_services: Arc::new(
                crate::core::util::executor_service::ExecutorServiceRegistry::new(),
            ),
            eventflux_extensions: HashMap::new(),
            deprecated_eventflux_extensions: HashMap::new(),
            persistence_store: Arc::new(RwLock::new(None)),
            incremental_persistence_store: Arc::new(RwLock::new(None)),
            error_store: None,
            extension_holder_map: HashMap::new(),
            config_manager: "InMemoryConfigManager_Placeholder".to_string(),
            sink_handler_manager: None,
            source_handler_manager: None,
            record_table_handler_manager: None,
            default_disrupter_exception_handler: "DefaultDisruptorExceptionHandler".to_string(),
            // Increased default async buffer to handle high throughput tests
            default_junction_buffer_size: 4096,
            default_junction_async: false,
            dummy_field_eventflux_context_extensions: String::new(),
        };
        let default_threads = crate::core::util::executor_service::pool_size_from_env(
            "default",
            num_cpus::get().max(1),
        );
        ctx.executor_services.register_named(
            "default".to_string(),
            Arc::new(crate::core::util::ExecutorService::new(
                "default",
                default_threads,
            )),
        );
        let part_threads = crate::core::util::executor_service::pool_size_from_env("partition", 2);
        ctx.executor_services.register_named(
            "partition".to_string(),
            Arc::new(crate::core::util::ExecutorService::new(
                "partition",
                part_threads,
            )),
        );
        ctx.register_default_extensions();
        ctx
    }

    // Example methods translated (simplified)
    pub fn get_eventflux_extensions(&self) -> &HashMap<String, String> {
        &self.eventflux_extensions
    }

    pub fn get_persistence_store(&self) -> Option<Arc<dyn PersistenceStore>> {
        self.persistence_store.read().unwrap().clone()
    }

    pub fn set_persistence_store(&self, persistence_store: Arc<dyn PersistenceStore>) {
        // Mirroring Java logic: only one persistence store allowed
        if self.incremental_persistence_store.read().unwrap().is_some() {
            // In a full implementation this would return an error type
            panic!("Only one type of persistence store can exist. Incremental persistence store already registered!");
        }
        *self.persistence_store.write().unwrap() = Some(persistence_store);
        // self.persistence_store = Some(persistence_store);
    }

    pub fn get_incremental_persistence_store(
        &self,
    ) -> Option<Arc<dyn IncrementalPersistenceStore>> {
        self.incremental_persistence_store.read().unwrap().clone()
    }

    pub fn set_incremental_persistence_store(&self, store: Arc<dyn IncrementalPersistenceStore>) {
        if self.persistence_store.read().unwrap().is_some() {
            panic!("Only one type of persistence store can exist. Persistence store already registered!");
        }
        *self.incremental_persistence_store.write().unwrap() = Some(store);
    }

    pub fn get_error_store(&self) -> Option<Arc<dyn ErrorStore>> {
        self.error_store.as_ref().cloned()
    }

    pub fn set_error_store(&mut self, store: Arc<dyn ErrorStore>) {
        self.error_store = Some(store);
    }

    // ... other getters and setters for various managers and stores ...

    pub fn get_data_source(&self, name: &str) -> Option<Arc<dyn DataSource>> {
        self.data_sources.read().unwrap().get(name).cloned()
    }

    pub fn add_data_source(
        &self,
        name: String,
        mut data_source: Arc<dyn DataSource>,
    ) -> Result<(), String> {
        if let Some(cfg) = self.data_source_configs.read().unwrap().get(&name).cloned() {
            let dummy_ctx = Arc::new(EventFluxAppContext::new(
                Arc::new(self.clone()),
                "__ds_init__".to_string(),
                Arc::new(crate::query_api::eventflux_app::EventFluxApp::default()),
                String::new(),
            ));

            if let Some(ds_mut) = Arc::get_mut(&mut data_source) {
                ds_mut.init(&dummy_ctx, &name, cfg)?;
            } else {
                let mut ds_box = data_source.clone_data_source();
                ds_box.init(&dummy_ctx, &name, cfg.clone())?;
                data_source = Arc::from(ds_box);
            }
        }
        self.data_sources.write().unwrap().insert(name, data_source);
        Ok(())
    }

    pub fn set_data_source_config(&self, name: String, config: DataSourceConfig) {
        self.data_source_configs
            .write()
            .unwrap()
            .insert(name, config);
    }

    pub fn get_data_source_config(&self, name: &str) -> Option<DataSourceConfig> {
        self.data_source_configs.read().unwrap().get(name).cloned()
    }

    pub fn add_table(&self, name: String, table: Arc<dyn Table>) {
        self.tables.write().unwrap().insert(name, table);
    }

    pub fn get_table(&self, name: &str) -> Option<Arc<dyn Table>> {
        self.tables.read().unwrap().get(name).cloned()
    }

    pub fn get_statistics_configuration(&self) -> &StatisticsConfiguration {
        &self.statistics_configuration
    }

    pub fn set_statistics_configuration(&mut self, stat_config: StatisticsConfiguration) {
        self.statistics_configuration = stat_config;
    }

    pub fn get_config_manager(&self) -> &String {
        &self.config_manager
        // &self.config_manager
    }

    pub fn set_config_manager(&mut self, config_manager: String) {
        self.config_manager = config_manager;
        // self.config_manager = config_manager;
    }

    pub fn get_sink_handler_manager(&self) -> Option<&String> {
        self.sink_handler_manager.as_ref()
    }

    pub fn set_sink_handler_manager(&mut self, manager: String) {
        self.sink_handler_manager = Some(manager);
    }

    pub fn get_source_handler_manager(&self) -> Option<&String> {
        self.source_handler_manager.as_ref()
    }

    pub fn set_source_handler_manager(&mut self, manager: String) {
        self.source_handler_manager = Some(manager);
    }

    pub fn get_record_table_handler_manager(&self) -> Option<&String> {
        self.record_table_handler_manager.as_ref()
    }

    pub fn set_record_table_handler_manager(&mut self, manager: String) {
        self.record_table_handler_manager = Some(manager);
    }

    pub fn get_default_disruptor_exception_handler(&self) -> &String {
        &self.default_disrupter_exception_handler
    }

    pub fn get_default_junction_buffer_size(&self) -> usize {
        self.default_junction_buffer_size
    }

    pub fn set_default_junction_buffer_size(&mut self, size: usize) {
        self.default_junction_buffer_size = size;
    }

    pub fn get_default_async_mode(&self) -> bool {
        self.default_junction_async
    }

    pub fn set_default_async_mode(&mut self, mode: bool) {
        self.default_junction_async = mode;
    }

    pub fn get_attributes(
        &self,
    ) -> RwLockReadGuard<'_, HashMap<String, AttributeValuePlaceholder>> {
        // Return guard
        self.attributes.read().unwrap()
    }

    pub fn set_attribute(&self, key: String, value: AttributeValuePlaceholder) {
        // Takes &self due to RwLock
        self.attributes.write().unwrap().insert(key, value);
    }

    /// Register a new executor service under the given name.
    pub fn add_executor_service(&self, name: String, exec: Arc<ExecutorService>) {
        self.executor_services.register_named(name, exec);
    }

    /// Retrieve an executor service by name.
    pub fn get_executor_service(&self, name: &str) -> Option<Arc<ExecutorService>> {
        self.executor_services.get(name)
    }

    // --- Scalar Function Methods ---
    pub fn add_scalar_function_factory(
        &self,
        name: String,
        function_factory: Box<dyn ScalarFunctionExecutor>,
    ) {
        self.scalar_function_factories
            .write()
            .unwrap()
            .insert(name, function_factory);
    }

    pub fn get_scalar_function_factory(
        &self,
        name: &str,
    ) -> Option<Box<dyn ScalarFunctionExecutor>> {
        // .clone() on Box<dyn ScalarFunctionExecutor> calls the trait's clone_scalar_function()
        self.scalar_function_factories
            .read()
            .unwrap()
            .get(name)
            .cloned()
    }

    fn register_default_extensions(&mut self) {
        use crate::core::executor::function::builtin_wrapper::register_builtin_scalar_functions;
        use crate::core::extension::{LogSinkFactory, TimerSourceFactory};
        use crate::core::query::processor::stream::window::{
            CronWindowFactory, ExternalTimeBatchWindowFactory, ExternalTimeWindowFactory,
            LengthBatchWindowFactory, LengthWindowFactory, LossyCountingWindowFactory,
            TimeBatchWindowFactory, TimeWindowFactory,
        };
        use crate::core::query::selector::attribute::aggregator::{
            AvgAttributeAggregatorFactory, CountAttributeAggregatorFactory,
            DistinctCountAttributeAggregatorFactory, MaxAttributeAggregatorFactory,
            MaxForeverAttributeAggregatorFactory, MinAttributeAggregatorFactory,
            MinForeverAttributeAggregatorFactory, SumAttributeAggregatorFactory,
        };
        use crate::core::table::{CacheTableFactory, InMemoryTableFactory, JdbcTableFactory};

        self.add_window_factory("length".to_string(), Box::new(LengthWindowFactory));
        self.add_window_factory("time".to_string(), Box::new(TimeWindowFactory));
        self.add_window_factory(
            "lengthBatch".to_string(),
            Box::new(LengthBatchWindowFactory),
        );
        self.add_window_factory("timeBatch".to_string(), Box::new(TimeBatchWindowFactory));
        self.add_window_factory(
            "externalTime".to_string(),
            Box::new(ExternalTimeWindowFactory),
        );
        self.add_window_factory(
            "externalTimeBatch".to_string(),
            Box::new(ExternalTimeBatchWindowFactory),
        );
        self.add_window_factory(
            "lossyCounting".to_string(),
            Box::new(LossyCountingWindowFactory),
        );
        self.add_window_factory("cron".to_string(), Box::new(CronWindowFactory));

        self.add_attribute_aggregator_factory(
            "sum".to_string(),
            Box::new(SumAttributeAggregatorFactory),
        );
        self.add_attribute_aggregator_factory(
            "avg".to_string(),
            Box::new(AvgAttributeAggregatorFactory),
        );
        self.add_attribute_aggregator_factory(
            "count".to_string(),
            Box::new(CountAttributeAggregatorFactory),
        );
        self.add_attribute_aggregator_factory(
            "distinctCount".to_string(),
            Box::new(DistinctCountAttributeAggregatorFactory),
        );
        self.add_attribute_aggregator_factory(
            "min".to_string(),
            Box::new(MinAttributeAggregatorFactory),
        );
        self.add_attribute_aggregator_factory(
            "max".to_string(),
            Box::new(MaxAttributeAggregatorFactory),
        );
        self.add_attribute_aggregator_factory(
            "minForever".to_string(),
            Box::new(MinForeverAttributeAggregatorFactory),
        );
        self.add_attribute_aggregator_factory(
            "maxForever".to_string(),
            Box::new(MaxForeverAttributeAggregatorFactory),
        );

        self.add_table_factory("inMemory".to_string(), Box::new(InMemoryTableFactory));
        self.add_table_factory("jdbc".to_string(), Box::new(JdbcTableFactory));
        self.add_table_factory("cache".to_string(), Box::new(CacheTableFactory));
        self.add_source_factory("timer".to_string(), Box::new(TimerSourceFactory));
        self.add_sink_factory("log".to_string(), Box::new(LogSinkFactory));

        register_builtin_scalar_functions(self);
    }

    // --- Extension Factory Methods ---
    pub fn add_window_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::WindowProcessorFactory>,
    ) {
        self.window_factories.write().unwrap().insert(name, factory);
    }

    pub fn get_window_factory(
        &self,
        name: &str,
    ) -> Option<Box<dyn crate::core::extension::WindowProcessorFactory>> {
        self.window_factories.read().unwrap().get(name).cloned()
    }

    pub fn add_attribute_aggregator_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::AttributeAggregatorFactory>,
    ) {
        self.attribute_aggregator_factories
            .write()
            .unwrap()
            .insert(name, factory);
    }

    pub fn get_attribute_aggregator_factory(
        &self,
        name: &str,
    ) -> Option<Box<dyn crate::core::extension::AttributeAggregatorFactory>> {
        self.attribute_aggregator_factories
            .read()
            .unwrap()
            .get(name)
            .cloned()
    }

    /// List the names of all registered scalar functions.
    pub fn list_scalar_function_names(&self) -> Vec<String> {
        self.scalar_function_factories
            .read()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }

    /// List the names of all registered attribute aggregators.
    pub fn list_attribute_aggregator_names(&self) -> Vec<String> {
        self.attribute_aggregator_factories
            .read()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }

    pub fn add_source_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::SourceFactory>,
    ) {
        self.source_factories.write().unwrap().insert(name, factory);
    }
    pub fn get_source_factory(
        &self,
        name: &str,
    ) -> Option<Box<dyn crate::core::extension::SourceFactory>> {
        self.source_factories.read().unwrap().get(name).cloned()
    }
    pub fn add_sink_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::SinkFactory>,
    ) {
        self.sink_factories.write().unwrap().insert(name, factory);
    }
    pub fn get_sink_factory(
        &self,
        name: &str,
    ) -> Option<Box<dyn crate::core::extension::SinkFactory>> {
        self.sink_factories.read().unwrap().get(name).cloned()
    }
    pub fn add_store_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::StoreFactory>,
    ) {
        self.store_factories.write().unwrap().insert(name, factory);
    }
    pub fn get_store_factory(
        &self,
        name: &str,
    ) -> Option<Box<dyn crate::core::extension::StoreFactory>> {
        self.store_factories.read().unwrap().get(name).cloned()
    }
    pub fn add_source_mapper_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::SourceMapperFactory>,
    ) {
        self.source_mapper_factories
            .write()
            .unwrap()
            .insert(name, factory);
    }
    pub fn get_source_mapper_factory(
        &self,
        name: &str,
    ) -> Option<Box<dyn crate::core::extension::SourceMapperFactory>> {
        self.source_mapper_factories
            .read()
            .unwrap()
            .get(name)
            .cloned()
    }
    pub fn add_sink_mapper_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::SinkMapperFactory>,
    ) {
        self.sink_mapper_factories
            .write()
            .unwrap()
            .insert(name, factory);
    }
    pub fn get_sink_mapper_factory(
        &self,
        name: &str,
    ) -> Option<Box<dyn crate::core::extension::SinkMapperFactory>> {
        self.sink_mapper_factories
            .read()
            .unwrap()
            .get(name)
            .cloned()
    }

    pub fn add_table_factory(
        &self,
        name: String,
        factory: Box<dyn crate::core::extension::TableFactory>,
    ) {
        self.table_factories.write().unwrap().insert(name, factory);
    }

    pub fn get_table_factory(
        &self,
        name: &str,
    ) -> Option<Box<dyn crate::core::extension::TableFactory>> {
        self.table_factories.read().unwrap().get(name).cloned()
    }
}

impl Default for EventFluxContext {
    fn default() -> Self {
        Self::new()
    }
}

// Clone implementation for EventFluxContext needs to handle Arc fields by cloning the Arc, not the underlying data.
impl Clone for EventFluxContext {
    fn clone(&self) -> Self {
        Self {
            statistics_configuration: self.statistics_configuration.clone(),
            attributes: Arc::clone(&self.attributes),
            scalar_function_factories: Arc::clone(&self.scalar_function_factories),
            window_factories: Arc::clone(&self.window_factories),
            attribute_aggregator_factories: Arc::clone(&self.attribute_aggregator_factories),
            source_factories: Arc::clone(&self.source_factories),
            sink_factories: Arc::clone(&self.sink_factories),
            store_factories: Arc::clone(&self.store_factories),
            source_mapper_factories: Arc::clone(&self.source_mapper_factories),
            sink_mapper_factories: Arc::clone(&self.sink_mapper_factories),
            table_factories: Arc::clone(&self.table_factories),
            data_source_configs: Arc::clone(&self.data_source_configs),
            data_sources: Arc::clone(&self.data_sources),
            tables: Arc::clone(&self.tables),
            executor_services: Arc::clone(&self.executor_services),
            eventflux_extensions: self.eventflux_extensions.clone(),
            deprecated_eventflux_extensions: self.deprecated_eventflux_extensions.clone(),
            persistence_store: self.persistence_store.clone(),
            incremental_persistence_store: self.incremental_persistence_store.clone(),
            error_store: self.error_store.clone(),
            extension_holder_map: self.extension_holder_map.clone(),
            config_manager: self.config_manager.clone(),
            sink_handler_manager: self.sink_handler_manager.clone(),
            source_handler_manager: self.source_handler_manager.clone(),
            record_table_handler_manager: self.record_table_handler_manager.clone(),
            default_disrupter_exception_handler: self.default_disrupter_exception_handler.clone(),
            default_junction_buffer_size: self.default_junction_buffer_size,
            default_junction_async: self.default_junction_async,
            dummy_field_eventflux_context_extensions: self
                .dummy_field_eventflux_context_extensions
                .clone(),
        }
    }
}

impl std::fmt::Debug for EventFluxContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventFluxContext")
            .field("statistics_configuration", &self.statistics_configuration)
            .finish()
    }
}
