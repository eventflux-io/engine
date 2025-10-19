// SPDX-License-Identifier: MIT OR Apache-2.0

//! Example Factory Implementations with Typed Configurations
//!
//! This module demonstrates the single-phase construction pattern for factories
//! as described in M3: Factory System & Registry

use crate::core::event::event::Event;
use crate::core::exception::EventFluxError;
use crate::core::extension::{SinkFactory, SinkMapperFactory, SourceFactory, SourceMapperFactory};
use crate::core::stream::input::mapper::SourceMapper;
use crate::core::stream::input::source::Source;
use crate::core::stream::output::mapper::SinkMapper;
use crate::core::stream::output::sink::Sink;
use std::collections::HashMap;

// ============================================================================
// Kafka Source Factory with Typed Config
// ============================================================================

/// Kafka-specific validated configuration (INTERNAL to KafkaSourceFactory)
#[derive(Debug, Clone)]
struct KafkaSourceConfig {
    bootstrap_servers: Vec<String>,
    topic: String,
    consumer_group: String,
    timeout_ms: u64,
}

impl KafkaSourceConfig {
    /// Parse and validate raw config into typed config (PRIVATE helper)
    fn parse(raw_config: &HashMap<String, String>) -> Result<Self, EventFluxError> {
        // 1. Validate required parameters present
        let brokers_str = raw_config
            .get("kafka.bootstrap.servers")
            .ok_or_else(|| EventFluxError::missing_parameter("kafka.bootstrap.servers"))?;

        let topic = raw_config
            .get("kafka.topic")
            .ok_or_else(|| EventFluxError::missing_parameter("kafka.topic"))?;

        // 2. Parse comma-separated brokers list
        let bootstrap_servers: Vec<String> = brokers_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if bootstrap_servers.is_empty() {
            return Err(EventFluxError::configuration_with_key(
                "kafka.bootstrap.servers cannot be empty",
                "kafka.bootstrap.servers",
            ));
        }

        // 3. Parse optional integer
        let timeout_ms = raw_config
            .get("kafka.timeout")
            .map(|s| s.parse::<u64>())
            .transpose()
            .map_err(|_| {
                EventFluxError::invalid_parameter_with_details(
                    "kafka.timeout must be a valid integer",
                    "kafka.timeout",
                    "positive integer (milliseconds)",
                )
            })?
            .unwrap_or(30000);

        // 4. Parse consumer group (with default)
        let consumer_group = raw_config
            .get("kafka.consumer.group")
            .cloned()
            .unwrap_or_else(|| format!("eventflux-{}", topic));

        // 5. Return typed config
        Ok(KafkaSourceConfig {
            bootstrap_servers,
            topic: topic.clone(),
            consumer_group,
            timeout_ms,
        })
    }
}

/// Placeholder Kafka Source (actual implementation would use rdkafka)
#[derive(Debug)]
struct KafkaSource {
    _topic: String,
    _bootstrap_servers: Vec<String>,
}

impl Source for KafkaSource {
    fn start(
        &mut self,
        _handler: std::sync::Arc<
            std::sync::Mutex<crate::core::stream::input::input_handler::InputHandler>,
        >,
    ) {
        // Placeholder: actual implementation would start Kafka consumer
    }

    fn stop(&mut self) {
        // Placeholder: actual implementation would stop Kafka consumer
    }

    fn clone_box(&self) -> Box<dyn Source> {
        Box::new(KafkaSource {
            _topic: self._topic.clone(),
            _bootstrap_servers: self._bootstrap_servers.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct KafkaSourceFactory;

impl SourceFactory for KafkaSourceFactory {
    fn name(&self) -> &'static str {
        "kafka"
    }

    fn supported_formats(&self) -> &[&str] {
        &["json", "avro", "bytes"]
    }

    fn required_parameters(&self) -> &[&str] {
        &["kafka.bootstrap.servers", "kafka.topic"]
    }

    fn optional_parameters(&self) -> &[&str] {
        &["kafka.consumer.group", "kafka.timeout"]
    }

    fn create_initialized(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<Box<dyn Source>, EventFluxError> {
        // 1. Parse and validate configuration
        let parsed = KafkaSourceConfig::parse(config)?;

        // 2. Create Kafka source (in real implementation, would create rdkafka consumer)
        // This is a placeholder - actual implementation would:
        // - Create rdkafka consumer with parsed config
        // - Test connectivity (fail-fast)
        // - Return fully initialized Source

        // 3. Return fully initialized Source
        Ok(Box::new(KafkaSource {
            _topic: parsed.topic,
            _bootstrap_servers: parsed.bootstrap_servers,
        }))
    }

    fn clone_box(&self) -> Box<dyn SourceFactory> {
        Box::new(self.clone())
    }
}

// ============================================================================
// HTTP Sink Factory with Typed Config
// ============================================================================

/// HTTP-specific validated configuration
#[derive(Debug, Clone)]
struct HttpSinkConfig {
    url: String,
    method: String,
    headers: HashMap<String, String>,
    timeout_secs: u64,
}

impl HttpSinkConfig {
    fn parse(raw_config: &HashMap<String, String>) -> Result<Self, EventFluxError> {
        let url = raw_config
            .get("http.url")
            .ok_or_else(|| EventFluxError::missing_parameter("http.url"))?;

        let method = raw_config
            .get("http.method")
            .cloned()
            .unwrap_or_else(|| "POST".to_string());

        // Validate HTTP method
        if !["GET", "POST", "PUT", "DELETE", "PATCH"].contains(&method.to_uppercase().as_str()) {
            return Err(EventFluxError::invalid_parameter_with_details(
                format!("Invalid HTTP method: {}", method),
                "http.method",
                "one of: GET, POST, PUT, DELETE, PATCH",
            ));
        }

        let timeout_secs = raw_config
            .get("http.timeout")
            .map(|s| s.parse::<u64>())
            .transpose()
            .map_err(|_| {
                EventFluxError::invalid_parameter_with_details(
                    "http.timeout must be a valid integer",
                    "http.timeout",
                    "positive integer (seconds)",
                )
            })?
            .unwrap_or(30);

        // Parse headers (simple implementation)
        let headers = HashMap::new(); // Placeholder for header parsing

        Ok(HttpSinkConfig {
            url: url.clone(),
            method: method.to_uppercase(),
            headers,
            timeout_secs,
        })
    }
}

/// Placeholder HTTP Sink
#[derive(Debug)]
struct HttpSink {
    _url: String,
    _method: String,
}

impl crate::core::stream::output::stream_callback::StreamCallback for HttpSink {
    fn receive_events(&self, _events: &[Event]) {
        // Placeholder: actual implementation would send HTTP request
    }
}

impl Sink for HttpSink {
    fn clone_box(&self) -> Box<dyn Sink> {
        Box::new(HttpSink {
            _url: self._url.clone(),
            _method: self._method.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct HttpSinkFactory;

impl SinkFactory for HttpSinkFactory {
    fn name(&self) -> &'static str {
        "http"
    }

    fn supported_formats(&self) -> &[&str] {
        &["json", "xml", "text"]
    }

    fn required_parameters(&self) -> &[&str] {
        &["http.url"]
    }

    fn optional_parameters(&self) -> &[&str] {
        &["http.method", "http.headers", "http.timeout"]
    }

    fn create_initialized(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<Box<dyn Sink>, EventFluxError> {
        let parsed = HttpSinkConfig::parse(config)?;

        Ok(Box::new(HttpSink {
            _url: parsed.url,
            _method: parsed.method,
        }))
    }

    fn clone_box(&self) -> Box<dyn SinkFactory> {
        Box::new(self.clone())
    }
}

// ============================================================================
// JSON Source Mapper Factory
// ============================================================================

/// Placeholder JSON Source Mapper
#[derive(Debug, Clone)]
struct JsonSourceMapper;

impl SourceMapper for JsonSourceMapper {
    fn map(&self, _input: &[u8]) -> Vec<Event> {
        // Placeholder: actual implementation would parse JSON
        vec![]
    }

    fn clone_box(&self) -> Box<dyn SourceMapper> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct JsonSourceMapperFactory;

impl SourceMapperFactory for JsonSourceMapperFactory {
    fn name(&self) -> &'static str {
        "json"
    }

    fn required_parameters(&self) -> &[&str] {
        &[] // JSON has no required parameters
    }

    fn optional_parameters(&self) -> &[&str] {
        &["json.fail-on-missing-attribute", "json.enclosing-element"]
    }

    fn create_initialized(
        &self,
        _config: &HashMap<String, String>,
    ) -> Result<Box<dyn SourceMapper>, EventFluxError> {
        Ok(Box::new(JsonSourceMapper))
    }

    fn clone_box(&self) -> Box<dyn SourceMapperFactory> {
        Box::new(self.clone())
    }
}

// ============================================================================
// CSV Sink Mapper Factory
// ============================================================================

/// CSV-specific configuration
#[derive(Debug, Clone)]
struct CsvSinkMapperConfig {
    delimiter: String,
    header: bool,
}

impl CsvSinkMapperConfig {
    fn parse(raw_config: &HashMap<String, String>) -> Result<Self, EventFluxError> {
        let delimiter = raw_config
            .get("csv.delimiter")
            .cloned()
            .unwrap_or_else(|| ",".to_string());

        if delimiter.len() != 1 {
            return Err(EventFluxError::invalid_parameter_with_details(
                "CSV delimiter must be a single character",
                "csv.delimiter",
                "single character",
            ));
        }

        let header = raw_config
            .get("csv.header")
            .map(|s| s.parse::<bool>())
            .transpose()
            .map_err(|_| {
                EventFluxError::invalid_parameter_with_details(
                    "csv.header must be 'true' or 'false'",
                    "csv.header",
                    "boolean",
                )
            })?
            .unwrap_or(true);

        Ok(CsvSinkMapperConfig { delimiter, header })
    }
}

/// Placeholder CSV Sink Mapper
#[derive(Debug)]
struct CsvSinkMapper {
    _delimiter: String,
    _header: bool,
}

impl SinkMapper for CsvSinkMapper {
    fn map(&self, _events: &[Event]) -> Vec<u8> {
        // Placeholder: actual implementation would generate CSV
        vec![]
    }

    fn clone_box(&self) -> Box<dyn SinkMapper> {
        Box::new(CsvSinkMapper {
            _delimiter: self._delimiter.clone(),
            _header: self._header,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CsvSinkMapperFactory;

impl SinkMapperFactory for CsvSinkMapperFactory {
    fn name(&self) -> &'static str {
        "csv"
    }

    fn required_parameters(&self) -> &[&str] {
        &[] // CSV has no required parameters
    }

    fn optional_parameters(&self) -> &[&str] {
        &["csv.delimiter", "csv.header"]
    }

    fn create_initialized(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<Box<dyn SinkMapper>, EventFluxError> {
        let parsed = CsvSinkMapperConfig::parse(config)?;

        Ok(Box::new(CsvSinkMapper {
            _delimiter: parsed.delimiter,
            _header: parsed.header,
        }))
    }

    fn clone_box(&self) -> Box<dyn SinkMapperFactory> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kafka_source_config_parse() {
        let mut config = HashMap::new();
        config.insert(
            "kafka.bootstrap.servers".to_string(),
            "localhost:9092".to_string(),
        );
        config.insert("kafka.topic".to_string(), "test-topic".to_string());

        let parsed = KafkaSourceConfig::parse(&config).unwrap();
        assert_eq!(parsed.bootstrap_servers, vec!["localhost:9092"]);
        assert_eq!(parsed.topic, "test-topic");
        assert_eq!(parsed.consumer_group, "eventflux-test-topic");
        assert_eq!(parsed.timeout_ms, 30000);
    }

    #[test]
    fn test_kafka_source_config_missing_required() {
        let config = HashMap::new();
        let result = KafkaSourceConfig::parse(&config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EventFluxError::InvalidParameter { .. }
        ));
    }

    #[test]
    fn test_kafka_source_config_empty_brokers() {
        let mut config = HashMap::new();
        config.insert("kafka.bootstrap.servers".to_string(), "".to_string());
        config.insert("kafka.topic".to_string(), "test".to_string());

        let result = KafkaSourceConfig::parse(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_kafka_factory_supported_formats() {
        let factory = KafkaSourceFactory;
        assert!(factory.supported_formats().contains(&"json"));
        assert!(factory.supported_formats().contains(&"avro"));
        assert!(!factory.supported_formats().contains(&"xml"));
    }

    #[test]
    fn test_http_sink_config_parse() {
        let mut config = HashMap::new();
        config.insert(
            "http.url".to_string(),
            "http://localhost:8080/api".to_string(),
        );
        config.insert("http.method".to_string(), "POST".to_string());

        let parsed = HttpSinkConfig::parse(&config).unwrap();
        assert_eq!(parsed.url, "http://localhost:8080/api");
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.timeout_secs, 30);
    }

    #[test]
    fn test_http_sink_config_invalid_method() {
        let mut config = HashMap::new();
        config.insert("http.url".to_string(), "http://localhost:8080".to_string());
        config.insert("http.method".to_string(), "INVALID".to_string());

        let result = HttpSinkConfig::parse(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_csv_sink_mapper_config_parse() {
        let mut config = HashMap::new();
        config.insert("csv.delimiter".to_string(), ";".to_string());
        config.insert("csv.header".to_string(), "false".to_string());

        let parsed = CsvSinkMapperConfig::parse(&config).unwrap();
        assert_eq!(parsed.delimiter, ";");
        assert_eq!(parsed.header, false);
    }

    #[test]
    fn test_csv_sink_mapper_invalid_delimiter() {
        let mut config = HashMap::new();
        config.insert("csv.delimiter".to_string(), ";;".to_string());

        let result = CsvSinkMapperConfig::parse(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_factory_create_initialized() {
        let factory = KafkaSourceFactory;
        let mut config = HashMap::new();
        config.insert(
            "kafka.bootstrap.servers".to_string(),
            "localhost:9092".to_string(),
        );
        config.insert("kafka.topic".to_string(), "test".to_string());

        let result = factory.create_initialized(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_factory_create_initialized_missing_params() {
        let factory = KafkaSourceFactory;
        let config = HashMap::new();

        let result = factory.create_initialized(&config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Missing required parameter"));
    }
}
