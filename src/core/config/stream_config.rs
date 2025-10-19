// SPDX-License-Identifier: MIT OR Apache-2.0

//! # Stream Configuration Module
//!
//! This module provides configuration management for EventFlux streams with multi-layer
//! property resolution. It implements a priority-based configuration system where settings
//! can come from multiple sources and are merged according to their precedence.
//!
//! ## Configuration Sources (Priority: Low to High)
//!
//! 1. **RustDefault** - Built-in Rust defaults
//! 2. **TomlApplication** - Global application-level TOML config
//! 3. **TomlStream** - Stream-specific TOML config
//! 4. **SqlWith** - SQL WITH clause (highest priority)
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use eventflux_rust::core::config::stream_config::*;
//!
//! // Create configuration from SQL WITH clause
//! let mut config = FlatConfig::new();
//! config.set("type", "source", PropertySource::SqlWith);
//! config.set("extension", "kafka", PropertySource::SqlWith);
//! config.set("format", "json", PropertySource::SqlWith);
//!
//! // Convert to typed configuration
//! let stream_config = StreamTypeConfig::from_flat_config(&config)?;
//! assert_eq!(stream_config.stream_type, StreamType::Source);
//! assert_eq!(stream_config.extension()?, "kafka");
//! assert_eq!(stream_config.format()?, "json");
//! ```

use std::collections::HashMap;

/// Property source identifier with priority ordering
///
/// Higher priority sources override lower priority sources during configuration merging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertySource {
    /// Rust code defaults (priority: 0)
    RustDefault,
    /// TOML [application] section (priority: 1)
    TomlApplication,
    /// TOML [streams.StreamName] section (priority: 2)
    TomlStream,
    /// SQL WITH clause (priority: 3)
    SqlWith,
}

impl PropertySource {
    /// Get numeric priority for comparison (higher = more important)
    #[inline]
    pub const fn priority(&self) -> u8 {
        match self {
            PropertySource::RustDefault => 0,
            PropertySource::TomlApplication => 1,
            PropertySource::TomlStream => 2,
            PropertySource::SqlWith => 3,
        }
    }

    /// Get human-readable description of the source
    #[inline]
    pub const fn description(&self) -> &'static str {
        match self {
            PropertySource::RustDefault => "Rust default",
            PropertySource::TomlApplication => "TOML [application]",
            PropertySource::TomlStream => "TOML [streams.StreamName]",
            PropertySource::SqlWith => "SQL WITH clause",
        }
    }
}

/// Flat key-value configuration with source tracking
///
/// Uses priority-based merging: higher priority sources override lower priority sources.
/// This is the foundation for multi-layer configuration resolution.
#[derive(Debug, Clone)]
pub struct FlatConfig {
    properties: HashMap<String, String>,
    sources: HashMap<String, PropertySource>,
}

impl FlatConfig {
    /// Create a new empty configuration
    #[inline]
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
            sources: HashMap::new(),
        }
    }

    /// Create with initial capacity for better performance
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            properties: HashMap::with_capacity(capacity),
            sources: HashMap::with_capacity(capacity),
        }
    }

    /// Set a property with source tracking and priority-based override
    ///
    /// Only sets the value if the new source has equal or higher priority than the existing source.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>, source: PropertySource) {
        let key = key.into();
        let value = value.into();

        // Check if property exists and compare priorities
        if let Some(existing_source) = self.sources.get(&key) {
            if existing_source.priority() >= source.priority() {
                // Existing source has higher or equal priority, don't override
                return;
            }
        }

        // Set or override the property
        self.properties.insert(key.clone(), value);
        self.sources.insert(key, source);
    }

    /// Get a property value by key
    #[inline]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.properties.get(key)
    }

    /// Get a property value with its source
    #[inline]
    pub fn get_with_source(&self, key: &str) -> Option<(&String, PropertySource)> {
        self.properties.get(key).and_then(|value| {
            self.sources.get(key).map(|source| (value, *source))
        })
    }

    /// Check if a property exists
    #[inline]
    pub fn contains(&self, key: &str) -> bool {
        self.properties.contains_key(key)
    }

    /// Get an iterator over all property keys
    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.properties.keys()
    }

    /// Get immutable reference to all properties
    #[inline]
    pub fn properties(&self) -> &HashMap<String, String> {
        &self.properties
    }

    /// Merge another configuration into this one (respects priorities)
    ///
    /// Properties from `other` will only override if they have higher priority.
    pub fn merge(&mut self, other: &FlatConfig) {
        for (key, value) in &other.properties {
            if let Some(source) = other.sources.get(key) {
                self.set(key.clone(), value.clone(), *source);
            }
        }
    }

    /// Get all properties with a specific prefix
    ///
    /// Useful for extracting extension-specific properties like "kafka.*"
    pub fn get_properties_with_prefix<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = (&'a String, &'a String)> + 'a {
        self.properties
            .iter()
            .filter(move |(key, _)| key.starts_with(prefix))
    }

    /// Get the number of properties
    #[inline]
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Check if configuration is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }
}

impl Default for FlatConfig {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Stream type classification
///
/// Determines validation rules and extension requirements for streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamType {
    /// Source stream - reads from external systems
    Source,
    /// Sink stream - writes to external systems
    Sink,
    /// Internal stream - in-memory processing only
    Internal,
}

impl StreamType {
    /// Parse stream type from string (case-insensitive)
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "source" => Ok(StreamType::Source),
            "sink" => Ok(StreamType::Sink),
            "internal" => Ok(StreamType::Internal),
            _ => Err(format!(
                "Invalid stream type '{}'. Valid values: 'source', 'sink', 'internal'",
                s
            )),
        }
    }

    /// Convert stream type to string representation
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            StreamType::Source => "source",
            StreamType::Sink => "sink",
            StreamType::Internal => "internal",
        }
    }

    /// Check if stream type requires an extension (source/sink do, internal doesn't)
    #[inline]
    pub const fn requires_extension(&self) -> bool {
        matches!(self, StreamType::Source | StreamType::Sink)
    }

    /// Check if stream type requires a format (source/sink do, internal doesn't)
    #[inline]
    pub const fn requires_format(&self) -> bool {
        matches!(self, StreamType::Source | StreamType::Sink)
    }
}

/// Typed stream configuration with validation
///
/// Enforces type-specific validation rules:
/// - Source/Sink streams **must** have extension and format
/// - Internal streams **must not** have extension or format
#[derive(Debug, Clone)]
pub struct StreamTypeConfig {
    pub stream_type: StreamType,
    pub extension: Option<String>,
    pub format: Option<String>,
    pub properties: HashMap<String, String>,
}

impl StreamTypeConfig {
    /// Create a new stream configuration with validation
    pub fn new(
        stream_type: StreamType,
        extension: Option<String>,
        format: Option<String>,
        properties: HashMap<String, String>,
    ) -> Result<Self, String> {
        let config = Self {
            stream_type,
            extension,
            format,
            properties,
        };

        config.validate()?;
        Ok(config)
    }

    /// Create from flat configuration with type inference
    pub fn from_flat_config(flat_config: &FlatConfig) -> Result<Self, String> {
        let stream_type_str = flat_config
            .get("type")
            .ok_or("Stream configuration missing required 'type' property")?;
        let stream_type = StreamType::from_str(stream_type_str)?;

        let extension = flat_config.get("extension").cloned();
        let format = flat_config.get("format").cloned();
        let properties = flat_config.properties().clone();

        Self::new(stream_type, extension, format, properties)
    }

    /// Validate configuration rules based on stream type
    pub fn validate(&self) -> Result<(), String> {
        match self.stream_type {
            StreamType::Source | StreamType::Sink => {
                // External streams require extension and format
                if self.extension.is_none() {
                    return Err(format!(
                        "Stream has type='{}' but missing required 'extension' property",
                        self.stream_type.as_str()
                    ));
                }

                if self.format.is_none() {
                    return Err(format!(
                        "Stream has type='{}' but missing required 'format' property",
                        self.stream_type.as_str()
                    ));
                }
            }
            StreamType::Internal => {
                // Internal streams must NOT have extension or format
                if self.extension.is_some() {
                    return Err(
                        "Stream has type='internal' but specifies 'extension' (not allowed)".to_string()
                    );
                }

                if self.format.is_some() {
                    return Err(
                        "Stream has type='internal' but specifies 'format' (not allowed)".to_string()
                    );
                }
            }
        }

        Ok(())
    }

    /// Get extension with validation (error if not present)
    #[inline]
    pub fn extension(&self) -> Result<&str, String> {
        self.extension.as_deref().ok_or_else(|| {
            format!("Stream type '{}' requires extension", self.stream_type.as_str())
        })
    }

    /// Get format with validation (error if not present)
    #[inline]
    pub fn format(&self) -> Result<&str, String> {
        self.format.as_deref().ok_or_else(|| {
            format!("Stream type '{}' requires format", self.stream_type.as_str())
        })
    }

    /// Get all properties with a specific prefix
    ///
    /// Useful for extracting extension-specific properties like "kafka.*"
    pub fn get_properties_with_prefix(&self, prefix: &str) -> HashMap<String, String> {
        self.properties
            .iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // PropertySource Tests
    // ========================================================================

    #[test]
    fn test_property_source_priority_ordering() {
        assert!(PropertySource::SqlWith.priority() > PropertySource::TomlStream.priority());
        assert!(PropertySource::TomlStream.priority() > PropertySource::TomlApplication.priority());
        assert!(PropertySource::TomlApplication.priority() > PropertySource::RustDefault.priority());
    }

    #[test]
    fn test_property_source_priority_values() {
        assert_eq!(PropertySource::RustDefault.priority(), 0);
        assert_eq!(PropertySource::TomlApplication.priority(), 1);
        assert_eq!(PropertySource::TomlStream.priority(), 2);
        assert_eq!(PropertySource::SqlWith.priority(), 3);
    }

    #[test]
    fn test_property_source_descriptions() {
        assert_eq!(PropertySource::RustDefault.description(), "Rust default");
        assert_eq!(PropertySource::TomlApplication.description(), "TOML [application]");
        assert_eq!(PropertySource::TomlStream.description(), "TOML [streams.StreamName]");
        assert_eq!(PropertySource::SqlWith.description(), "SQL WITH clause");
    }

    // ========================================================================
    // FlatConfig Tests
    // ========================================================================

    #[test]
    fn test_flat_config_creation() {
        let config = FlatConfig::new();
        assert!(config.is_empty());
        assert_eq!(config.len(), 0);
    }

    #[test]
    fn test_flat_config_set_and_get() {
        let mut config = FlatConfig::new();
        config.set("key1", "value1", PropertySource::RustDefault);

        assert_eq!(config.get("key1"), Some(&"value1".to_string()));
        assert!(config.contains("key1"));
        assert_eq!(config.len(), 1);
    }

    #[test]
    fn test_flat_config_priority_override() {
        let mut config = FlatConfig::new();

        // Set with low priority
        config.set("buffer_size", "1024", PropertySource::RustDefault);
        assert_eq!(config.get("buffer_size"), Some(&"1024".to_string()));

        // Override with higher priority
        config.set("buffer_size", "2048", PropertySource::TomlApplication);
        assert_eq!(config.get("buffer_size"), Some(&"2048".to_string()));

        // Override with even higher priority
        config.set("buffer_size", "4096", PropertySource::SqlWith);
        assert_eq!(config.get("buffer_size"), Some(&"4096".to_string()));
    }

    #[test]
    fn test_flat_config_priority_no_override() {
        let mut config = FlatConfig::new();

        // Set with high priority
        config.set("buffer_size", "4096", PropertySource::SqlWith);
        assert_eq!(config.get("buffer_size"), Some(&"4096".to_string()));

        // Try to override with lower priority (should fail)
        config.set("buffer_size", "1024", PropertySource::RustDefault);
        assert_eq!(config.get("buffer_size"), Some(&"4096".to_string()));

        // Try to override with same priority (should also not override due to >= check)
        config.set("buffer_size", "2048", PropertySource::SqlWith);
        assert_eq!(config.get("buffer_size"), Some(&"4096".to_string()));
    }

    #[test]
    fn test_flat_config_get_with_source() {
        let mut config = FlatConfig::new();
        config.set("key1", "value1", PropertySource::TomlStream);

        let result = config.get_with_source("key1");
        assert!(result.is_some());
        let (value, source) = result.unwrap();
        assert_eq!(value, &"value1".to_string());
        assert_eq!(source, PropertySource::TomlStream);
    }

    #[test]
    fn test_flat_config_merge() {
        let mut config1 = FlatConfig::new();
        config1.set("key1", "value1", PropertySource::RustDefault);
        config1.set("key2", "value2", PropertySource::TomlApplication);

        let mut config2 = FlatConfig::new();
        config2.set("key2", "new_value2", PropertySource::SqlWith); // Higher priority
        config2.set("key3", "value3", PropertySource::TomlStream);

        config1.merge(&config2);

        assert_eq!(config1.get("key1"), Some(&"value1".to_string()));
        assert_eq!(config1.get("key2"), Some(&"new_value2".to_string())); // Overridden
        assert_eq!(config1.get("key3"), Some(&"value3".to_string()));
    }

    #[test]
    fn test_flat_config_prefix_filtering() {
        let mut config = FlatConfig::new();
        config.set("kafka_bootstrap_servers", "localhost:9092", PropertySource::SqlWith);
        config.set("kafka_topic", "events", PropertySource::SqlWith);
        config.set("type", "source", PropertySource::SqlWith);
        config.set("format", "json", PropertySource::SqlWith);

        let kafka_props: Vec<_> = config
            .get_properties_with_prefix("kafka_")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        assert_eq!(kafka_props.len(), 2);
        assert!(kafka_props.iter().any(|(k, _)| k == "kafka_bootstrap_servers"));
        assert!(kafka_props.iter().any(|(k, _)| k == "kafka_topic"));
    }

    #[test]
    fn test_flat_config_keys_iterator() {
        let mut config = FlatConfig::new();
        config.set("key1", "value1", PropertySource::RustDefault);
        config.set("key2", "value2", PropertySource::RustDefault);
        config.set("key3", "value3", PropertySource::RustDefault);

        let keys: Vec<_> = config.keys().collect();
        assert_eq!(keys.len(), 3);
    }

    // ========================================================================
    // StreamType Tests
    // ========================================================================

    #[test]
    fn test_stream_type_from_str() {
        assert_eq!(StreamType::from_str("source").unwrap(), StreamType::Source);
        assert_eq!(StreamType::from_str("Source").unwrap(), StreamType::Source);
        assert_eq!(StreamType::from_str("SOURCE").unwrap(), StreamType::Source);

        assert_eq!(StreamType::from_str("sink").unwrap(), StreamType::Sink);
        assert_eq!(StreamType::from_str("Sink").unwrap(), StreamType::Sink);

        assert_eq!(StreamType::from_str("internal").unwrap(), StreamType::Internal);
        assert_eq!(StreamType::from_str("Internal").unwrap(), StreamType::Internal);
    }

    #[test]
    fn test_stream_type_from_str_invalid() {
        assert!(StreamType::from_str("invalid").is_err());
        assert!(StreamType::from_str("").is_err());
        assert!(StreamType::from_str("stream").is_err());
    }

    #[test]
    fn test_stream_type_as_str() {
        assert_eq!(StreamType::Source.as_str(), "source");
        assert_eq!(StreamType::Sink.as_str(), "sink");
        assert_eq!(StreamType::Internal.as_str(), "internal");
    }

    #[test]
    fn test_stream_type_requires_extension() {
        assert!(StreamType::Source.requires_extension());
        assert!(StreamType::Sink.requires_extension());
        assert!(!StreamType::Internal.requires_extension());
    }

    #[test]
    fn test_stream_type_requires_format() {
        assert!(StreamType::Source.requires_format());
        assert!(StreamType::Sink.requires_format());
        assert!(!StreamType::Internal.requires_format());
    }

    // ========================================================================
    // StreamTypeConfig Tests
    // ========================================================================

    #[test]
    fn test_stream_type_config_source_valid() {
        let mut props = HashMap::new();
        props.insert("type".to_string(), "source".to_string());
        props.insert("extension".to_string(), "kafka".to_string());
        props.insert("format".to_string(), "json".to_string());

        let config = StreamTypeConfig::new(
            StreamType::Source,
            Some("kafka".to_string()),
            Some("json".to_string()),
            props,
        );

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.stream_type, StreamType::Source);
        assert_eq!(config.extension().unwrap(), "kafka");
        assert_eq!(config.format().unwrap(), "json");
    }

    #[test]
    fn test_stream_type_config_sink_valid() {
        let mut props = HashMap::new();
        props.insert("type".to_string(), "sink".to_string());
        props.insert("extension".to_string(), "log".to_string());
        props.insert("format".to_string(), "text".to_string());

        let config = StreamTypeConfig::new(
            StreamType::Sink,
            Some("log".to_string()),
            Some("text".to_string()),
            props,
        );

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.stream_type, StreamType::Sink);
    }

    #[test]
    fn test_stream_type_config_internal_valid() {
        let mut props = HashMap::new();
        props.insert("type".to_string(), "internal".to_string());

        let config = StreamTypeConfig::new(
            StreamType::Internal,
            None,
            None,
            props,
        );

        assert!(config.is_ok());
    }

    #[test]
    fn test_stream_type_config_source_missing_extension() {
        let props = HashMap::new();
        let config = StreamTypeConfig::new(
            StreamType::Source,
            None, // Missing extension
            Some("json".to_string()),
            props,
        );

        assert!(config.is_err());
        assert!(config.unwrap_err().contains("missing required 'extension'"));
    }

    #[test]
    fn test_stream_type_config_source_missing_format() {
        let props = HashMap::new();
        let config = StreamTypeConfig::new(
            StreamType::Source,
            Some("kafka".to_string()),
            None, // Missing format
            props,
        );

        assert!(config.is_err());
        assert!(config.unwrap_err().contains("missing required 'format'"));
    }

    #[test]
    fn test_stream_type_config_internal_with_extension() {
        let props = HashMap::new();
        let config = StreamTypeConfig::new(
            StreamType::Internal,
            Some("kafka".to_string()), // Not allowed for internal
            None,
            props,
        );

        assert!(config.is_err());
        assert!(config.unwrap_err().contains("type='internal' but specifies 'extension'"));
    }

    #[test]
    fn test_stream_type_config_internal_with_format() {
        let props = HashMap::new();
        let config = StreamTypeConfig::new(
            StreamType::Internal,
            None,
            Some("json".to_string()), // Not allowed for internal
            props,
        );

        assert!(config.is_err());
        assert!(config.unwrap_err().contains("type='internal' but specifies 'format'"));
    }

    #[test]
    fn test_stream_type_config_from_flat_config() {
        let mut flat_config = FlatConfig::new();
        flat_config.set("type", "source", PropertySource::SqlWith);
        flat_config.set("extension", "kafka", PropertySource::SqlWith);
        flat_config.set("format", "json", PropertySource::SqlWith);
        flat_config.set("bootstrap_servers", "localhost:9092", PropertySource::SqlWith);

        let config = StreamTypeConfig::from_flat_config(&flat_config);
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.stream_type, StreamType::Source);
        assert_eq!(config.extension().unwrap(), "kafka");
        assert_eq!(config.format().unwrap(), "json");
        assert_eq!(
            config.properties.get("bootstrap_servers"),
            Some(&"localhost:9092".to_string())
        );
    }

    #[test]
    fn test_stream_type_config_from_flat_config_missing_type() {
        let flat_config = FlatConfig::new();
        let config = StreamTypeConfig::from_flat_config(&flat_config);

        assert!(config.is_err());
        assert!(config.unwrap_err().contains("missing required 'type'"));
    }

    #[test]
    fn test_stream_type_config_get_properties_with_prefix() {
        let mut props = HashMap::new();
        props.insert("type".to_string(), "source".to_string());
        props.insert("extension".to_string(), "kafka".to_string());
        props.insert("format".to_string(), "json".to_string());
        props.insert("kafka_bootstrap_servers".to_string(), "localhost:9092".to_string());
        props.insert("kafka_topic".to_string(), "events".to_string());
        props.insert("redis_host".to_string(), "localhost".to_string());

        let config = StreamTypeConfig::new(
            StreamType::Source,
            Some("kafka".to_string()),
            Some("json".to_string()),
            props,
        ).unwrap();

        let kafka_props = config.get_properties_with_prefix("kafka_");
        assert_eq!(kafka_props.len(), 2);
        assert_eq!(kafka_props.get("kafka_bootstrap_servers"), Some(&"localhost:9092".to_string()));
        assert_eq!(kafka_props.get("kafka_topic"), Some(&"events".to_string()));

        let redis_props = config.get_properties_with_prefix("redis_");
        assert_eq!(redis_props.len(), 1);
    }

    // ========================================================================
    // Integration Tests
    // ========================================================================

    #[test]
    fn test_complete_configuration_flow() {
        // Simulate multi-layer configuration merge
        let mut config = FlatConfig::new();

        // Layer 1: Rust defaults
        config.set("buffer_size", "1024", PropertySource::RustDefault);
        config.set("timeout", "30000", PropertySource::RustDefault);

        // Layer 2: TOML application config
        config.set("buffer_size", "2048", PropertySource::TomlApplication);
        config.set("retry_count", "3", PropertySource::TomlApplication);

        // Layer 3: TOML stream config
        config.set("type", "source", PropertySource::TomlStream);
        config.set("extension", "kafka", PropertySource::TomlStream);

        // Layer 4: SQL WITH clause
        config.set("extension", "http", PropertySource::SqlWith); // Override extension
        config.set("format", "json", PropertySource::SqlWith);

        // Verify final configuration
        assert_eq!(config.get("buffer_size"), Some(&"2048".to_string())); // From TOML app
        assert_eq!(config.get("timeout"), Some(&"30000".to_string())); // From Rust default
        assert_eq!(config.get("retry_count"), Some(&"3".to_string())); // From TOML app
        assert_eq!(config.get("type"), Some(&"source".to_string())); // From TOML stream
        assert_eq!(config.get("extension"), Some(&"http".to_string())); // Overridden by SQL WITH
        assert_eq!(config.get("format"), Some(&"json".to_string())); // From SQL WITH
    }

    #[test]
    fn test_with_capacity_optimization() {
        let config = FlatConfig::with_capacity(10);
        assert_eq!(config.len(), 0);
        assert!(config.is_empty());
    }
}
