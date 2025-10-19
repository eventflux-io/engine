// SPDX-License-Identifier: MIT OR Apache-2.0

//! Mapper Factory System
//!
//! Provides factory traits and implementations for creating mappers from configuration.
//! Follows the `create_initialized` pattern for fully configured mapper instances.
//!
//! ## Factory Pattern
//!
//! All mapper factories implement the same pattern:
//! 1. Parse configuration into internal config struct
//! 2. Validate configuration
//! 3. Create fully initialized mapper
//! 4. Return boxed trait object
//!
//! ## Configuration Format
//!
//! Configuration is provided as a flat HashMap with format-prefixed keys:
//!
//! ```toml
//! json.mapping.orderId = "$.order.id"
//! json.mapping.amount = "$.order.total"
//! json.ignore-parse-errors = "true"
//! json.date-format = "yyyy-MM-dd"
//! ```
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use eventflux_rust::core::stream::mapper::factory::*;
//! use std::collections::HashMap;
//!
//! let mut config = HashMap::new();
//! config.insert("json.mapping.id".to_string(), "$.order.id".to_string());
//!
//! let factory = JsonSourceMapperFactory;
//! let mapper = factory.create_initialized(&config)?;
//! ```

use super::csv_mapper::{CsvSinkMapper, CsvSourceMapper};
use super::json_mapper::{JsonSinkMapper, JsonSourceMapper};
use super::{SinkMapper, SourceMapper};
use crate::core::exception::EventFluxError;
use std::collections::HashMap;

// ============================================================================
// Factory Traits
// ============================================================================

/// Factory trait for creating source mappers
pub trait SourceMapperFactory: Send + Sync {
    /// Create a fully initialized source mapper from configuration
    ///
    /// # Arguments
    /// * `config` - Flat configuration map with format-prefixed keys
    ///
    /// # Returns
    /// * `Ok(Box<dyn SourceMapper>)` - Fully configured mapper ready to use
    /// * `Err(EventFluxError)` - Configuration parsing or validation failed
    fn create_initialized(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<Box<dyn SourceMapper>, EventFluxError>;

    /// Get the format name this factory supports (e.g., "json", "csv")
    fn format_name(&self) -> &str;
}

/// Factory trait for creating sink mappers
pub trait SinkMapperFactory: Send + Sync {
    /// Create a fully initialized sink mapper from configuration
    ///
    /// # Arguments
    /// * `config` - Flat configuration map with format-prefixed keys
    ///
    /// # Returns
    /// * `Ok(Box<dyn SinkMapper>)` - Fully configured mapper ready to use
    /// * `Err(EventFluxError)` - Configuration parsing or validation failed
    fn create_initialized(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<Box<dyn SinkMapper>, EventFluxError>;

    /// Get the format name this factory supports (e.g., "json", "csv")
    fn format_name(&self) -> &str;
}

// ============================================================================
// JSON Mapper Factories
// ============================================================================

/// Factory for creating JSON source mappers
#[derive(Debug, Clone, Default)]
pub struct JsonSourceMapperFactory;

/// Internal configuration for JSON source mapper
#[derive(Debug, Clone)]
struct JsonSourceMapperConfig {
    mappings: HashMap<String, String>,
    ignore_parse_errors: bool,
    date_format: Option<String>,
}

impl JsonSourceMapperConfig {
    fn parse(config: &HashMap<String, String>) -> Result<Self, EventFluxError> {
        let mut mappings = HashMap::new();

        // Extract mapping configuration
        for (key, value) in config {
            if let Some(field_name) = key.strip_prefix("json.mapping.") {
                mappings.insert(field_name.to_string(), value.clone());
            }
        }

        // Extract format options
        let ignore_parse_errors = config
            .get("json.ignore-parse-errors")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let date_format = config.get("json.date-format").cloned();

        Ok(JsonSourceMapperConfig {
            mappings,
            ignore_parse_errors,
            date_format,
        })
    }
}

impl SourceMapperFactory for JsonSourceMapperFactory {
    fn create_initialized(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<Box<dyn SourceMapper>, EventFluxError> {
        // Parse and validate configuration
        let mapper_config = JsonSourceMapperConfig::parse(config)?;

        // Create fully initialized mapper
        let mut mapper = if mapper_config.mappings.is_empty() {
            JsonSourceMapper::new()
        } else {
            JsonSourceMapper::with_mappings(mapper_config.mappings)
        };

        mapper.set_ignore_parse_errors(mapper_config.ignore_parse_errors);
        mapper.set_date_format(mapper_config.date_format);

        Ok(Box::new(mapper))
    }

    fn format_name(&self) -> &str {
        "json"
    }
}

/// Factory for creating JSON sink mappers
#[derive(Debug, Clone, Default)]
pub struct JsonSinkMapperFactory;

/// Internal configuration for JSON sink mapper
#[derive(Debug, Clone)]
struct JsonSinkMapperConfig {
    template: Option<String>,
    pretty_print: bool,
}

impl JsonSinkMapperConfig {
    fn parse(config: &HashMap<String, String>) -> Result<Self, EventFluxError> {
        let template = config.get("json.template").cloned();

        let pretty_print = config
            .get("json.pretty-print")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        Ok(JsonSinkMapperConfig {
            template,
            pretty_print,
        })
    }
}

impl SinkMapperFactory for JsonSinkMapperFactory {
    fn create_initialized(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<Box<dyn SinkMapper>, EventFluxError> {
        // Parse and validate configuration
        let mapper_config = JsonSinkMapperConfig::parse(config)?;

        // Create fully initialized mapper
        let mut mapper = if let Some(template) = mapper_config.template {
            JsonSinkMapper::with_template(template)
        } else {
            JsonSinkMapper::new()
        };

        mapper.set_pretty_print(mapper_config.pretty_print);

        Ok(Box::new(mapper))
    }

    fn format_name(&self) -> &str {
        "json"
    }
}

// ============================================================================
// CSV Mapper Factories
// ============================================================================

/// Factory for creating CSV source mappers
#[derive(Debug, Clone, Default)]
pub struct CsvSourceMapperFactory;

/// Internal configuration for CSV source mapper
#[derive(Debug, Clone)]
struct CsvSourceMapperConfig {
    mappings: HashMap<String, usize>,
    delimiter: char,
    has_header: bool,
    ignore_parse_errors: bool,
}

impl CsvSourceMapperConfig {
    fn parse(config: &HashMap<String, String>) -> Result<Self, EventFluxError> {
        let mut mappings = HashMap::new();

        // Extract mapping configuration (field name → column index)
        for (key, value) in config {
            if let Some(field_name) = key.strip_prefix("csv.mapping.") {
                let col_idx =
                    value
                        .parse::<usize>()
                        .map_err(|e| EventFluxError::Configuration {
                            message: format!(
                                "Invalid column index for {}: '{}' ({})",
                                field_name, value, e
                            ),
                            config_key: Some(key.clone()),
                        })?;
                mappings.insert(field_name.to_string(), col_idx);
            }
        }

        // Extract delimiter (default: comma)
        let delimiter = config
            .get("csv.delimiter")
            .and_then(|s| s.chars().next())
            .unwrap_or(',');

        // Extract header flag
        let has_header = config
            .get("csv.has-header")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        // Extract ignore errors flag
        let ignore_parse_errors = config
            .get("csv.ignore-parse-errors")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        Ok(CsvSourceMapperConfig {
            mappings,
            delimiter,
            has_header,
            ignore_parse_errors,
        })
    }
}

impl SourceMapperFactory for CsvSourceMapperFactory {
    fn create_initialized(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<Box<dyn SourceMapper>, EventFluxError> {
        // Parse and validate configuration
        let mapper_config = CsvSourceMapperConfig::parse(config)?;

        // Create fully initialized mapper
        let mut mapper = if mapper_config.mappings.is_empty() {
            CsvSourceMapper::new()
        } else {
            CsvSourceMapper::with_mappings(mapper_config.mappings)
        };

        mapper.set_delimiter(mapper_config.delimiter);
        mapper.set_has_header(mapper_config.has_header);
        mapper.set_ignore_parse_errors(mapper_config.ignore_parse_errors);

        Ok(Box::new(mapper))
    }

    fn format_name(&self) -> &str {
        "csv"
    }
}

/// Factory for creating CSV sink mappers
#[derive(Debug, Clone, Default)]
pub struct CsvSinkMapperFactory;

/// Internal configuration for CSV sink mapper
#[derive(Debug, Clone)]
struct CsvSinkMapperConfig {
    delimiter: char,
    include_header: bool,
    header_names: Option<Vec<String>>,
}

impl CsvSinkMapperConfig {
    fn parse(config: &HashMap<String, String>) -> Result<Self, EventFluxError> {
        // Extract delimiter (default: comma)
        let delimiter = config
            .get("csv.delimiter")
            .and_then(|s| s.chars().next())
            .unwrap_or(',');

        // Extract header inclusion flag
        let include_header = config
            .get("csv.include-header")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        // Extract custom header names (comma-separated)
        let header_names = config
            .get("csv.header-names")
            .map(|s| s.split(',').map(|name| name.trim().to_string()).collect());

        Ok(CsvSinkMapperConfig {
            delimiter,
            include_header,
            header_names,
        })
    }
}

impl SinkMapperFactory for CsvSinkMapperFactory {
    fn create_initialized(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<Box<dyn SinkMapper>, EventFluxError> {
        // Parse and validate configuration
        let mapper_config = CsvSinkMapperConfig::parse(config)?;

        // Create fully initialized mapper
        let mut mapper = CsvSinkMapper::new();
        mapper.set_delimiter(mapper_config.delimiter);
        mapper.set_include_header(mapper_config.include_header);

        if let Some(headers) = mapper_config.header_names {
            mapper.set_header_names(headers);
        }

        Ok(Box::new(mapper))
    }

    fn format_name(&self) -> &str {
        "csv"
    }
}

// ============================================================================
// Mapper Factory Registry
// ============================================================================

/// Registry for mapper factories
pub struct MapperFactoryRegistry {
    source_factories: HashMap<String, Box<dyn SourceMapperFactory>>,
    sink_factories: HashMap<String, Box<dyn SinkMapperFactory>>,
}

impl MapperFactoryRegistry {
    /// Create a new registry with default factories
    pub fn new() -> Self {
        let mut registry = Self {
            source_factories: HashMap::new(),
            sink_factories: HashMap::new(),
        };

        // Register default factories
        registry.register_source_factory(Box::new(JsonSourceMapperFactory));
        registry.register_source_factory(Box::new(CsvSourceMapperFactory));
        registry.register_sink_factory(Box::new(JsonSinkMapperFactory));
        registry.register_sink_factory(Box::new(CsvSinkMapperFactory));

        registry
    }

    /// Register a source mapper factory
    pub fn register_source_factory(&mut self, factory: Box<dyn SourceMapperFactory>) {
        let format = factory.format_name().to_string();
        self.source_factories.insert(format, factory);
    }

    /// Register a sink mapper factory
    pub fn register_sink_factory(&mut self, factory: Box<dyn SinkMapperFactory>) {
        let format = factory.format_name().to_string();
        self.sink_factories.insert(format, factory);
    }

    /// Create a source mapper for the given format
    pub fn create_source_mapper(
        &self,
        format: &str,
        config: &HashMap<String, String>,
    ) -> Result<Box<dyn SourceMapper>, EventFluxError> {
        let factory =
            self.source_factories
                .get(format)
                .ok_or_else(|| EventFluxError::Configuration {
                    message: format!("Unknown source mapper format: {}", format),
                    config_key: Some("format".to_string()),
                })?;

        factory.create_initialized(config)
    }

    /// Create a sink mapper for the given format
    pub fn create_sink_mapper(
        &self,
        format: &str,
        config: &HashMap<String, String>,
    ) -> Result<Box<dyn SinkMapper>, EventFluxError> {
        let factory =
            self.sink_factories
                .get(format)
                .ok_or_else(|| EventFluxError::Configuration {
                    message: format!("Unknown sink mapper format: {}", format),
                    config_key: Some("format".to_string()),
                })?;

        factory.create_initialized(config)
    }
}

impl Default for MapperFactoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_source_factory_auto_mapping() {
        let config = HashMap::new();
        let factory = JsonSourceMapperFactory;

        let mapper = factory.create_initialized(&config).unwrap();
        let json_str = r#"{"orderId": "123", "amount": 100.0}"#;
        let events = mapper.map(json_str.as_bytes()).unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data.len(), 2);
    }

    #[test]
    fn test_json_source_factory_explicit_mapping() {
        let mut config = HashMap::new();
        config.insert("json.mapping.orderId".to_string(), "$.order.id".to_string());
        config.insert(
            "json.mapping.amount".to_string(),
            "$.order.total".to_string(),
        );

        let factory = JsonSourceMapperFactory;
        let mapper = factory.create_initialized(&config).unwrap();

        let json_str = r#"{"order": {"id": "123", "total": 100.0}}"#;
        let events = mapper.map(json_str.as_bytes()).unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data.len(), 2);
    }

    #[test]
    fn test_json_sink_factory_simple() {
        let config = HashMap::new();
        let factory = JsonSinkMapperFactory;

        let mapper = factory.create_initialized(&config).unwrap();
        let event = crate::core::event::Event::new_with_data(
            123,
            vec![
                crate::core::event::AttributeValue::String("test".to_string()),
                crate::core::event::AttributeValue::Int(42),
            ],
        );

        let result = mapper.map(&[event]).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_json_sink_factory_with_template() {
        let mut config = HashMap::new();
        config.insert(
            "json.template".to_string(),
            r#"{"id":"{{field_0}}","value":{{field_1}}}"#.to_string(),
        );

        let factory = JsonSinkMapperFactory;
        let mapper = factory.create_initialized(&config).unwrap();

        let event = crate::core::event::Event::new_with_data(
            123,
            vec![
                crate::core::event::AttributeValue::String("test-id".to_string()),
                crate::core::event::AttributeValue::Double(99.5),
            ],
        );

        let result = mapper.map(&[event]).unwrap();
        let json_str = String::from_utf8(result).unwrap();

        assert!(json_str.contains("\"id\":\"test-id\""));
        assert!(json_str.contains("\"value\":99.5"));
    }

    #[test]
    fn test_csv_source_factory() {
        let mut config = HashMap::new();
        config.insert("csv.delimiter".to_string(), ",".to_string());
        config.insert("csv.has-header".to_string(), "false".to_string());

        let factory = CsvSourceMapperFactory;
        let mapper = factory.create_initialized(&config).unwrap();

        let csv_str = "123,100.0,customer1";
        let events = mapper.map(csv_str.as_bytes()).unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data.len(), 3);
    }

    #[test]
    fn test_csv_sink_factory() {
        let mut config = HashMap::new();
        config.insert("csv.delimiter".to_string(), ",".to_string());
        config.insert("csv.include-header".to_string(), "true".to_string());

        let factory = CsvSinkMapperFactory;
        let mapper = factory.create_initialized(&config).unwrap();

        let event = crate::core::event::Event::new_with_data(
            123,
            vec![
                crate::core::event::AttributeValue::Int(42),
                crate::core::event::AttributeValue::String("test".to_string()),
            ],
        );

        let result = mapper.map(&[event]).unwrap();
        let csv_str = String::from_utf8(result).unwrap();

        assert!(csv_str.contains("field_0"));
        assert!(csv_str.contains("42"));
        assert!(csv_str.contains("test"));
    }

    #[test]
    fn test_registry() {
        let registry = MapperFactoryRegistry::new();
        let config = HashMap::new();

        // Test JSON source mapper creation
        let json_source = registry.create_source_mapper("json", &config).unwrap();
        assert!(json_source.map(r#"{"test": 1}"#.as_bytes()).is_ok());

        // Test CSV source mapper creation
        let csv_source = registry.create_source_mapper("csv", &config).unwrap();
        assert!(csv_source.map("1,2,3".as_bytes()).is_ok());

        // Test unknown format
        assert!(registry.create_source_mapper("unknown", &config).is_err());
    }
}
