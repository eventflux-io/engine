// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod example_factories;

use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use crate::core::function::script::Script;
use crate::core::store::Store;
use crate::core::stream::input::mapper::SourceMapper;
use crate::core::stream::output::mapper::SinkMapper;

use crate::core::config::{
    eventflux_app_context::EventFluxAppContext, eventflux_query_context::EventFluxQueryContext,
};
use crate::core::query::processor::Processor;
use crate::core::query::selector::attribute::aggregator::AttributeAggregatorExecutor;
use crate::query_api::execution::query::input::handler::WindowHandler;

pub trait WindowProcessorFactory: Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn create(
        &self,
        handler: &WindowHandler,
        app_ctx: Arc<EventFluxAppContext>,
        query_ctx: Arc<EventFluxQueryContext>,
    ) -> Result<Arc<Mutex<dyn Processor>>, String>;
    fn clone_box(&self) -> Box<dyn WindowProcessorFactory>;
}
impl Clone for Box<dyn WindowProcessorFactory> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait AttributeAggregatorFactory: Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn create(&self) -> Box<dyn AttributeAggregatorExecutor>;
    fn clone_box(&self) -> Box<dyn AttributeAggregatorFactory>;
}
impl Clone for Box<dyn AttributeAggregatorFactory> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait SourceFactory: Debug + Send + Sync {
    fn name(&self) -> &'static str;

    /// List supported formats for this source extension
    /// Example: &["json", "avro", "bytes"]
    /// NO DEFAULT - Must be explicitly implemented
    fn supported_formats(&self) -> &[&str];

    /// List required configuration properties
    /// Example: &["kafka.brokers", "kafka.topic"]
    /// NO DEFAULT - Must be explicitly implemented
    fn required_parameters(&self) -> &[&str];

    /// List optional configuration properties
    /// Example: &["kafka.consumer.group", "kafka.security.protocol"]
    /// NO DEFAULT - Must be explicitly implemented
    fn optional_parameters(&self) -> &[&str];

    /// Create a fully initialized, ready-to-use Source instance
    /// Validates configuration and returns error if invalid
    /// Source is guaranteed to be in valid state upon successful return
    /// NO DEFAULT - Must be explicitly implemented
    fn create_initialized(
        &self,
        config: &std::collections::HashMap<String, String>,
    ) -> Result<
        Box<dyn crate::core::stream::input::source::Source>,
        crate::core::exception::EventFluxError,
    >;

    /// Legacy create method for backward compatibility
    /// Deprecated: Use create_initialized instead
    /// Returns None if factory requires configuration parameters
    fn create(&self) -> Option<Box<dyn crate::core::stream::input::source::Source>> {
        self.create_initialized(&std::collections::HashMap::new())
            .ok()
    }

    fn clone_box(&self) -> Box<dyn SourceFactory>;
}
impl Clone for Box<dyn SourceFactory> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait SinkFactory: Debug + Send + Sync {
    fn name(&self) -> &'static str;

    /// List supported formats for this sink extension
    /// Example: &["json", "csv", "bytes"]
    /// NO DEFAULT - Must be explicitly implemented
    fn supported_formats(&self) -> &[&str];

    /// List required configuration properties
    /// Example: &["http.url", "http.method"]
    /// NO DEFAULT - Must be explicitly implemented
    fn required_parameters(&self) -> &[&str];

    /// List optional configuration properties
    /// Example: &["http.headers", "http.timeout"]
    /// NO DEFAULT - Must be explicitly implemented
    fn optional_parameters(&self) -> &[&str];

    /// Create a fully initialized, ready-to-use Sink instance
    /// Validates configuration and returns error if invalid
    /// Sink is guaranteed to be in valid state upon successful return
    /// NO DEFAULT - Must be explicitly implemented
    fn create_initialized(
        &self,
        config: &std::collections::HashMap<String, String>,
    ) -> Result<
        Box<dyn crate::core::stream::output::sink::Sink>,
        crate::core::exception::EventFluxError,
    >;

    /// Legacy create method for backward compatibility
    /// Deprecated: Use create_initialized instead
    /// Returns None if factory requires configuration parameters
    fn create(&self) -> Option<Box<dyn crate::core::stream::output::sink::Sink>> {
        self.create_initialized(&std::collections::HashMap::new())
            .ok()
    }

    fn clone_box(&self) -> Box<dyn SinkFactory>;
}
impl Clone for Box<dyn SinkFactory> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait StoreFactory: Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn create(&self) -> Box<dyn Store>;
    fn clone_box(&self) -> Box<dyn StoreFactory>;
}
impl Clone for Box<dyn StoreFactory> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait SourceMapperFactory: Debug + Send + Sync {
    fn name(&self) -> &'static str;

    /// List required configuration properties
    /// Example: &[] for JSON (no required config), &["avro.schema"] for Avro
    /// NO DEFAULT - Must be explicitly implemented
    fn required_parameters(&self) -> &[&str];

    /// List optional configuration properties
    /// Example: &["json.ignore-parse-errors", "json.date-format"]
    /// NO DEFAULT - Must be explicitly implemented
    fn optional_parameters(&self) -> &[&str];

    /// Create a fully initialized, ready-to-use SourceMapper instance
    /// Validates configuration and returns error if invalid
    /// NO DEFAULT - Must be explicitly implemented
    fn create_initialized(
        &self,
        config: &std::collections::HashMap<String, String>,
    ) -> Result<Box<dyn SourceMapper>, crate::core::exception::EventFluxError>;

    /// Legacy create method for backward compatibility
    /// Deprecated: Use create_initialized instead
    /// Returns None if factory requires configuration parameters
    fn create(&self) -> Option<Box<dyn SourceMapper>> {
        self.create_initialized(&std::collections::HashMap::new())
            .ok()
    }

    fn clone_box(&self) -> Box<dyn SourceMapperFactory>;
}
impl Clone for Box<dyn SourceMapperFactory> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait SinkMapperFactory: Debug + Send + Sync {
    fn name(&self) -> &'static str;

    /// List required configuration properties
    /// Example: &[] for JSON, &["csv.delimiter"] for CSV
    /// NO DEFAULT - Must be explicitly implemented
    fn required_parameters(&self) -> &[&str];

    /// List optional configuration properties
    /// Example: &["json.pretty-print", "json.template"]
    /// NO DEFAULT - Must be explicitly implemented
    fn optional_parameters(&self) -> &[&str];

    /// Create a fully initialized, ready-to-use SinkMapper instance
    /// Validates configuration and returns error if invalid
    /// NO DEFAULT - Must be explicitly implemented
    fn create_initialized(
        &self,
        config: &std::collections::HashMap<String, String>,
    ) -> Result<Box<dyn SinkMapper>, crate::core::exception::EventFluxError>;

    /// Legacy create method for backward compatibility
    /// Deprecated: Use create_initialized instead
    /// Returns None if factory requires configuration parameters
    fn create(&self) -> Option<Box<dyn SinkMapper>> {
        self.create_initialized(&std::collections::HashMap::new())
            .ok()
    }

    fn clone_box(&self) -> Box<dyn SinkMapperFactory>;
}
impl Clone for Box<dyn SinkMapperFactory> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait TableFactory: Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn create(
        &self,
        table_name: String,
        properties: std::collections::HashMap<String, String>,
        ctx: Arc<crate::core::config::eventflux_context::EventFluxContext>,
    ) -> Result<Arc<dyn crate::core::table::Table>, String>;
    fn clone_box(&self) -> Box<dyn TableFactory>;
}
impl Clone for Box<dyn TableFactory> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait ScriptFactory: Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn create(&self) -> Box<dyn Script>;
    fn clone_box(&self) -> Box<dyn ScriptFactory>;
}
impl Clone for Box<dyn ScriptFactory> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Debug, Clone)]
pub struct TimerSourceFactory;

impl SourceFactory for TimerSourceFactory {
    fn name(&self) -> &'static str {
        "timer"
    }

    fn supported_formats(&self) -> &[&str] {
        &["json", "text", "bytes"]
    }

    fn required_parameters(&self) -> &[&str] {
        &[] // Timer has no required parameters
    }

    fn optional_parameters(&self) -> &[&str] {
        &["timer.interval"] // Interval in milliseconds
    }

    fn create_initialized(
        &self,
        config: &std::collections::HashMap<String, String>,
    ) -> Result<
        Box<dyn crate::core::stream::input::source::Source>,
        crate::core::exception::EventFluxError,
    > {
        // Parse optional interval parameter
        let interval_ms = if let Some(interval_str) = config.get("timer.interval") {
            interval_str.parse::<u64>().map_err(|_| {
                crate::core::exception::EventFluxError::invalid_parameter_with_details(
                    "timer.interval must be a valid integer",
                    "timer.interval",
                    "positive integer (milliseconds)",
                )
            })?
        } else {
            1000 // Default 1 second
        };

        Ok(Box::new(
            crate::core::stream::input::source::timer_source::TimerSource::new(interval_ms),
        ))
    }

    fn clone_box(&self) -> Box<dyn SourceFactory> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct LogSinkFactory;

impl SinkFactory for LogSinkFactory {
    fn name(&self) -> &'static str {
        "log"
    }

    fn supported_formats(&self) -> &[&str] {
        &["json", "text", "csv", "bytes"] // Log sink accepts all formats
    }

    fn required_parameters(&self) -> &[&str] {
        &[] // Log sink has no required parameters
    }

    fn optional_parameters(&self) -> &[&str] {
        &["log.prefix", "log.level"] // Optional logging parameters
    }

    fn create_initialized(
        &self,
        _config: &std::collections::HashMap<String, String>,
    ) -> Result<
        Box<dyn crate::core::stream::output::sink::Sink>,
        crate::core::exception::EventFluxError,
    > {
        // LogSink doesn't need configuration validation for now
        // Future: could add log level validation, prefix customization, etc.
        Ok(Box::new(crate::core::stream::output::sink::LogSink::new()))
    }

    fn clone_box(&self) -> Box<dyn SinkFactory> {
        Box::new(self.clone())
    }
}

/// FFI callback type used when dynamically loading extensions.
pub type RegisterFn = unsafe extern "C" fn(&crate::core::eventflux_manager::EventFluxManager);

/// Symbol names looked up by [`EventFluxManager::set_extension`].
pub const REGISTER_EXTENSION_FN: &[u8] = b"register_extension";
pub const REGISTER_WINDOWS_FN: &[u8] = b"register_windows";
pub const REGISTER_FUNCTIONS_FN: &[u8] = b"register_functions";
pub const REGISTER_SOURCES_FN: &[u8] = b"register_sources";
pub const REGISTER_SINKS_FN: &[u8] = b"register_sinks";
pub const REGISTER_STORES_FN: &[u8] = b"register_stores";
pub const REGISTER_SOURCE_MAPPERS_FN: &[u8] = b"register_source_mappers";
pub const REGISTER_SINK_MAPPERS_FN: &[u8] = b"register_sink_mappers";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::eventflux_context::EventFluxContext;
    use crate::core::extension::example_factories::*;

    #[test]
    fn test_factory_registration() {
        let context = EventFluxContext::new();

        // Test built-in factory registration
        let factory = context.get_source_factory("timer");
        assert!(factory.is_some());
        assert_eq!(factory.unwrap().name(), "timer");

        let sink_factory = context.get_sink_factory("log");
        assert!(sink_factory.is_some());
        assert_eq!(sink_factory.unwrap().name(), "log");
    }

    #[test]
    fn test_kafka_source_factory_registration() {
        let context = EventFluxContext::new();
        context.add_source_factory("kafka".to_string(), Box::new(KafkaSourceFactory));

        let factory = context.get_source_factory("kafka");
        assert!(factory.is_some());
        assert_eq!(factory.unwrap().name(), "kafka");
    }

    #[test]
    fn test_format_support_validation() {
        let factory = TimerSourceFactory;
        assert!(factory.supported_formats().contains(&"json"));
        assert!(factory.supported_formats().contains(&"text"));
        assert!(!factory.supported_formats().contains(&"xml"));

        let kafka_factory = KafkaSourceFactory;
        assert!(kafka_factory.supported_formats().contains(&"json"));
        assert!(kafka_factory.supported_formats().contains(&"avro"));
        assert!(!kafka_factory.supported_formats().contains(&"xml"));
    }

    #[test]
    fn test_create_initialized_missing_params() {
        let factory = KafkaSourceFactory;
        let config = std::collections::HashMap::new();

        let result = factory.create_initialized(&config);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("Missing required parameter"));
    }

    #[test]
    fn test_create_initialized_invalid_config() {
        let factory = KafkaSourceFactory;
        let mut config = std::collections::HashMap::new();
        config.insert("kafka.bootstrap.servers".to_string(), "".to_string());
        config.insert("kafka.topic".to_string(), "test".to_string());

        let result = factory.create_initialized(&config);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("cannot be empty"));
    }

    #[test]
    fn test_create_initialized_valid_config() {
        let factory = KafkaSourceFactory;
        let mut config = std::collections::HashMap::new();
        config.insert(
            "kafka.bootstrap.servers".to_string(),
            "localhost:9092".to_string(),
        );
        config.insert("kafka.topic".to_string(), "test-topic".to_string());

        let result = factory.create_initialized(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_timer_source_factory_interval_config() {
        let factory = TimerSourceFactory;
        let mut config = std::collections::HashMap::new();
        config.insert("timer.interval".to_string(), "5000".to_string());

        let result = factory.create_initialized(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_timer_source_factory_invalid_interval() {
        let factory = TimerSourceFactory;
        let mut config = std::collections::HashMap::new();
        config.insert("timer.interval".to_string(), "invalid".to_string());

        let result = factory.create_initialized(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_http_sink_factory_format_support() {
        let factory = HttpSinkFactory;
        assert!(factory.supported_formats().contains(&"json"));
        assert!(factory.supported_formats().contains(&"xml"));
        assert!(!factory.supported_formats().contains(&"avro"));
    }

    #[test]
    fn test_http_sink_factory_required_params() {
        let factory = HttpSinkFactory;
        assert_eq!(factory.required_parameters(), &["http.url"]);
    }

    #[test]
    fn test_http_sink_factory_create() {
        let factory = HttpSinkFactory;
        let mut config = std::collections::HashMap::new();
        config.insert(
            "http.url".to_string(),
            "http://localhost:8080/events".to_string(),
        );

        let result = factory.create_initialized(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_source_mapper_factory() {
        let factory = JsonSourceMapperFactory;
        assert_eq!(factory.name(), "json");
        assert_eq!(factory.required_parameters(), &[] as &[&str]);

        let config = std::collections::HashMap::new();
        let result = factory.create_initialized(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_csv_sink_mapper_factory() {
        let factory = CsvSinkMapperFactory;
        assert_eq!(factory.name(), "csv");

        let mut config = std::collections::HashMap::new();
        config.insert("csv.delimiter".to_string(), ";".to_string());
        config.insert("csv.header".to_string(), "false".to_string());

        let result = factory.create_initialized(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_csv_sink_mapper_invalid_delimiter() {
        let factory = CsvSinkMapperFactory;
        let mut config = std::collections::HashMap::new();
        config.insert("csv.delimiter".to_string(), ";;".to_string());

        let result = factory.create_initialized(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_factory_lookup_and_validation() {
        let context = EventFluxContext::new();
        context.add_source_factory("kafka".to_string(), Box::new(KafkaSourceFactory));
        context.add_source_mapper_factory("json".to_string(), Box::new(JsonSourceMapperFactory));

        // 1. Look up source factory by extension
        let source_factory = context.get_source_factory("kafka");
        assert!(source_factory.is_some());
        let source_factory = source_factory.unwrap();

        // 2. Validate format support
        let format = "json";
        assert!(source_factory.supported_formats().contains(&format));

        // 3. Look up mapper factory by format
        let mapper_factory = context.get_source_mapper_factory(format);
        assert!(mapper_factory.is_some());

        // 4. Create fully initialized instances
        let mut source_config = std::collections::HashMap::new();
        source_config.insert(
            "kafka.bootstrap.servers".to_string(),
            "localhost:9092".to_string(),
        );
        source_config.insert("kafka.topic".to_string(), "test".to_string());

        let source = source_factory.create_initialized(&source_config);
        assert!(source.is_ok());

        let mapper_config = std::collections::HashMap::new();
        let mapper = mapper_factory.unwrap().create_initialized(&mapper_config);
        assert!(mapper.is_ok());
    }

    #[test]
    fn test_unsupported_format_detection() {
        let factory = KafkaSourceFactory;
        let format = "xml";

        // Kafka doesn't support XML format
        assert!(!factory.supported_formats().contains(&format));
    }

    #[test]
    fn test_sink_factory_parameters() {
        let factory = LogSinkFactory;
        assert_eq!(factory.required_parameters(), &[] as &[&str]);
        assert!(factory.optional_parameters().contains(&"log.prefix"));
        assert!(factory.optional_parameters().contains(&"log.level"));
    }

    #[test]
    fn test_source_factory_parameters() {
        let factory = KafkaSourceFactory;
        assert!(factory
            .required_parameters()
            .contains(&"kafka.bootstrap.servers"));
        assert!(factory.required_parameters().contains(&"kafka.topic"));
        assert!(factory
            .optional_parameters()
            .contains(&"kafka.consumer.group"));
        assert!(factory.optional_parameters().contains(&"kafka.timeout"));
    }

    #[test]
    fn test_factory_clone() {
        let factory = TimerSourceFactory;
        let cloned = factory.clone_box();
        assert_eq!(cloned.name(), "timer");
        assert_eq!(cloned.supported_formats(), factory.supported_formats());
    }
}
