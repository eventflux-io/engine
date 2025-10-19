// SPDX-License-Identifier: MIT OR Apache-2.0

//! Stream Initialization Module
//!
//! Provides the critical integration point between the factory system and stream runtime.
//! This module implements the factory lookup, format validation, and instance creation
//! workflow as specified in M3.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::config::eventflux_context::EventFluxContext;
use crate::core::config::stream_config::{StreamType, StreamTypeConfig};
use crate::core::exception::EventFluxError;
use crate::core::stream::input::mapper::SourceMapper;
use crate::core::stream::input::source::Source;
use crate::core::stream::output::mapper::SinkMapper;
use crate::core::stream::output::sink::Sink;

/// Fully initialized stream with source and mapper components
pub struct InitializedSource {
    pub source: Box<dyn Source>,
    pub mapper: Box<dyn SourceMapper>,
    pub extension: String,
    pub format: String,
}

/// Fully initialized sink stream with sink and mapper components
pub struct InitializedSink {
    pub sink: Box<dyn Sink>,
    pub mapper: Box<dyn SinkMapper>,
    pub extension: String,
    pub format: String,
}

/// Result of stream initialization
pub enum InitializedStream {
    Source(InitializedSource),
    Sink(InitializedSink),
    Internal,
}

impl std::fmt::Debug for InitializedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InitializedSource")
            .field("extension", &self.extension)
            .field("format", &self.format)
            .finish()
    }
}

impl std::fmt::Debug for InitializedSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InitializedSink")
            .field("extension", &self.extension)
            .field("format", &self.format)
            .finish()
    }
}

impl std::fmt::Debug for InitializedStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitializedStream::Source(s) => f.debug_tuple("Source").field(s).finish(),
            InitializedStream::Sink(s) => f.debug_tuple("Sink").field(s).finish(),
            InitializedStream::Internal => write!(f, "Internal"),
        }
    }
}

/// Initialize a stream from configuration using the factory system
///
/// This is the main integration point between the factory registry and stream runtime.
/// It performs the following steps:
///
/// 1. Look up the appropriate factory by extension name
/// 2. Validate that the factory supports the requested format
/// 3. Look up the mapper factory by format name
/// 4. Create fully initialized instances with fail-fast validation
/// 5. Return wired components guaranteed to be in valid state
///
/// # Arguments
///
/// * `context` - EventFlux context containing factory registries
/// * `stream_config` - Typed stream configuration with validation
///
/// # Returns
///
/// * `Ok(InitializedStream)` - Fully initialized and validated stream components
/// * `Err(EventFluxError)` - If factory not found, format unsupported, or initialization fails
///
/// # Example
///
/// ```rust,ignore
/// use eventflux_rust::core::stream::stream_initializer::initialize_stream;
/// use eventflux_rust::core::config::stream_config::StreamTypeConfig;
///
/// let context = EventFluxContext::new();
/// let stream_config = StreamTypeConfig::new(
///     StreamType::Source,
///     Some("kafka".to_string()),
///     Some("json".to_string()),
///     config_map,
/// )?;
///
/// let initialized = initialize_stream(&context, &stream_config)?;
/// match initialized {
///     InitializedStream::Source(source) => {
///         // Source and mapper are ready to use
///         source.source.start(handler);
///     }
///     _ => {}
/// }
/// ```
pub fn initialize_stream(
    context: &EventFluxContext,
    stream_config: &StreamTypeConfig,
) -> Result<InitializedStream, EventFluxError> {
    match stream_config.stream_type {
        StreamType::Source => initialize_source_stream(context, stream_config),
        StreamType::Sink => initialize_sink_stream(context, stream_config),
        StreamType::Internal => Ok(InitializedStream::Internal),
    }
}

/// Initialize a source stream with factory lookup and validation
fn initialize_source_stream(
    context: &EventFluxContext,
    stream_config: &StreamTypeConfig,
) -> Result<InitializedStream, EventFluxError> {
    // 1. Look up source factory by extension
    let extension = stream_config
        .extension()
        .map_err(|e| EventFluxError::configuration(format!("Stream configuration error: {}", e)))?;

    let source_factory = context
        .get_source_factory(extension)
        .ok_or_else(|| EventFluxError::extension_not_found("source", extension))?;

    // 2. Validate format support
    let format = stream_config
        .format()
        .map_err(|e| EventFluxError::configuration(format!("Stream configuration error: {}", e)))?;

    if !source_factory.supported_formats().contains(&format) {
        return Err(EventFluxError::unsupported_format(format, extension));
    }

    // 3. Look up mapper factory by format
    let mapper_factory = context
        .get_source_mapper_factory(format)
        .ok_or_else(|| EventFluxError::extension_not_found("source mapper", format))?;

    // 4. Create fully initialized instances (fail-fast validation)
    let source = source_factory.create_initialized(&stream_config.properties)?;
    let mapper = mapper_factory.create_initialized(&stream_config.properties)?;

    // 5. Return wired components - instances are guaranteed valid
    Ok(InitializedStream::Source(InitializedSource {
        source,
        mapper,
        extension: extension.to_string(),
        format: format.to_string(),
    }))
}

/// Initialize a sink stream with factory lookup and validation
fn initialize_sink_stream(
    context: &EventFluxContext,
    stream_config: &StreamTypeConfig,
) -> Result<InitializedStream, EventFluxError> {
    // 1. Look up sink factory by extension
    let extension = stream_config
        .extension()
        .map_err(|e| EventFluxError::configuration(format!("Stream configuration error: {}", e)))?;

    let sink_factory = context
        .get_sink_factory(extension)
        .ok_or_else(|| EventFluxError::extension_not_found("sink", extension))?;

    // 2. Validate format support
    let format = stream_config
        .format()
        .map_err(|e| EventFluxError::configuration(format!("Stream configuration error: {}", e)))?;

    if !sink_factory.supported_formats().contains(&format) {
        return Err(EventFluxError::unsupported_format(format, extension));
    }

    // 3. Look up mapper factory by format
    let mapper_factory = context
        .get_sink_mapper_factory(format)
        .ok_or_else(|| EventFluxError::extension_not_found("sink mapper", format))?;

    // 4. Create fully initialized instances (fail-fast validation)
    let sink = sink_factory.create_initialized(&stream_config.properties)?;
    let mapper = mapper_factory.create_initialized(&stream_config.properties)?;

    // 5. Return wired components - instances are guaranteed valid
    Ok(InitializedStream::Sink(InitializedSink {
        sink,
        mapper,
        extension: extension.to_string(),
        format: format.to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::stream_config::{FlatConfig, PropertySource};
    use crate::core::extension::example_factories::*;

    #[test]
    fn test_initialize_source_stream_success() {
        let context = EventFluxContext::new();
        context.add_source_factory("kafka".to_string(), Box::new(KafkaSourceFactory));
        context.add_source_mapper_factory("json".to_string(), Box::new(JsonSourceMapperFactory));

        let mut config = HashMap::new();
        config.insert(
            "kafka.bootstrap.servers".to_string(),
            "localhost:9092".to_string(),
        );
        config.insert("kafka.topic".to_string(), "test-topic".to_string());

        let stream_config = StreamTypeConfig::new(
            StreamType::Source,
            Some("kafka".to_string()),
            Some("json".to_string()),
            config,
        )
        .unwrap();

        let result = initialize_stream(&context, &stream_config);
        assert!(result.is_ok());

        match result.unwrap() {
            InitializedStream::Source(source) => {
                assert_eq!(source.extension, "kafka");
                assert_eq!(source.format, "json");
            }
            _ => panic!("Expected Source stream"),
        }
    }

    #[test]
    fn test_initialize_source_stream_extension_not_found() {
        let context = EventFluxContext::new();

        let stream_config = StreamTypeConfig::new(
            StreamType::Source,
            Some("nonexistent".to_string()),
            Some("json".to_string()),
            HashMap::new(),
        )
        .unwrap();

        let result = initialize_stream(&context, &stream_config);
        assert!(result.is_err());

        match result.unwrap_err() {
            EventFluxError::ExtensionNotFound {
                extension_type,
                name,
            } => {
                assert_eq!(extension_type, "source");
                assert_eq!(name, "nonexistent");
            }
            e => panic!("Expected ExtensionNotFound error, got {:?}", e),
        }
    }

    #[test]
    fn test_initialize_source_stream_unsupported_format() {
        let context = EventFluxContext::new();
        context.add_source_factory("kafka".to_string(), Box::new(KafkaSourceFactory));

        let stream_config = StreamTypeConfig::new(
            StreamType::Source,
            Some("kafka".to_string()),
            Some("xml".to_string()), // Kafka doesn't support XML
            HashMap::new(),
        )
        .unwrap();

        let result = initialize_stream(&context, &stream_config);
        assert!(result.is_err());

        match result.unwrap_err() {
            EventFluxError::Configuration { message, .. } => {
                assert!(message.contains("xml"));
                assert!(message.contains("kafka"));
            }
            e => panic!(
                "Expected Configuration error for unsupported format, got {:?}",
                e
            ),
        }
    }

    #[test]
    fn test_initialize_source_stream_mapper_not_found() {
        let context = EventFluxContext::new();
        context.add_source_factory("kafka".to_string(), Box::new(KafkaSourceFactory));
        // Don't register json mapper

        let mut config = HashMap::new();
        config.insert(
            "kafka.bootstrap.servers".to_string(),
            "localhost:9092".to_string(),
        );
        config.insert("kafka.topic".to_string(), "test".to_string());

        let stream_config = StreamTypeConfig::new(
            StreamType::Source,
            Some("kafka".to_string()),
            Some("json".to_string()),
            config,
        )
        .unwrap();

        let result = initialize_stream(&context, &stream_config);
        assert!(result.is_err());

        match result.unwrap_err() {
            EventFluxError::ExtensionNotFound {
                extension_type,
                name,
            } => {
                assert_eq!(extension_type, "source mapper");
                assert_eq!(name, "json");
            }
            e => panic!("Expected ExtensionNotFound for mapper, got {:?}", e),
        }
    }

    #[test]
    fn test_initialize_sink_stream_success() {
        let context = EventFluxContext::new();
        context.add_sink_factory("http".to_string(), Box::new(HttpSinkFactory));
        context.add_sink_mapper_factory("json".to_string(), Box::new(CsvSinkMapperFactory)); // Using CSV as placeholder

        let mut config = HashMap::new();
        config.insert(
            "http.url".to_string(),
            "http://localhost:8080/events".to_string(),
        );

        let stream_config = StreamTypeConfig::new(
            StreamType::Sink,
            Some("http".to_string()),
            Some("json".to_string()),
            config,
        )
        .unwrap();

        let result = initialize_stream(&context, &stream_config);
        assert!(result.is_ok());

        match result.unwrap() {
            InitializedStream::Sink(sink) => {
                assert_eq!(sink.extension, "http");
                assert_eq!(sink.format, "json");
            }
            _ => panic!("Expected Sink stream"),
        }
    }

    #[test]
    fn test_initialize_internal_stream() {
        let context = EventFluxContext::new();

        let stream_config =
            StreamTypeConfig::new(StreamType::Internal, None, None, HashMap::new()).unwrap();

        let result = initialize_stream(&context, &stream_config);
        assert!(result.is_ok());

        match result.unwrap() {
            InitializedStream::Internal => {}
            _ => panic!("Expected Internal stream"),
        }
    }

    #[test]
    fn test_initialize_source_stream_invalid_config() {
        let context = EventFluxContext::new();
        context.add_source_factory("kafka".to_string(), Box::new(KafkaSourceFactory));
        context.add_source_mapper_factory("json".to_string(), Box::new(JsonSourceMapperFactory));

        // Missing required kafka.bootstrap.servers
        let config = HashMap::new();

        let stream_config = StreamTypeConfig::new(
            StreamType::Source,
            Some("kafka".to_string()),
            Some("json".to_string()),
            config,
        )
        .unwrap();

        let result = initialize_stream(&context, &stream_config);
        assert!(result.is_err());

        // Should get InvalidParameter error from factory (for missing parameter)
        match result.unwrap_err() {
            EventFluxError::InvalidParameter { parameter, .. } => {
                if let Some(param) = parameter {
                    assert!(param.contains("kafka.bootstrap.servers"));
                }
            }
            e => panic!(
                "Expected InvalidParameter error for missing parameter, got {:?}",
                e
            ),
        }
    }
}
