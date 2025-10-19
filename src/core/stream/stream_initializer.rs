// SPDX-License-Identifier: MIT OR Apache-2.0

//! Stream Initialization Module
//!
//! Provides comprehensive stream initialization with topological sorting for dependency
//! management. This module implements:
//!
//! 1. Dependency graph construction (query + DLQ dependencies)
//! 2. Topological sort for initialization ordering
//! 3. Stream handler creation and lifecycle management
//! 4. Integration with factory system for source/sink creation
//!
//! ## Initialization Flow
//!
//! 1. Build dependency graph from queries and DLQ configurations
//! 2. Perform topological sort to determine initialization order
//! 3. Initialize streams in dependency order (dependencies first)
//! 4. Start all source handlers
//!
//! ## Thread Safety
//!
//! All stream handlers are created behind Arc for shared ownership across
//! multiple threads during query processing.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, RwLock};

use crate::core::config::eventflux_context::EventFluxContext;
use crate::core::config::stream_config::{FlatConfig, StreamType, StreamTypeConfig};
use crate::core::exception::EventFluxError;
use crate::core::stream::handler::{SinkStreamHandler, SourceStreamHandler};
use crate::core::stream::input::mapper::SourceMapper;
use crate::core::stream::input::source::Source;
use crate::core::stream::output::mapper::SinkMapper;
use crate::core::stream::output::sink::Sink;
use crate::query_api::definition::stream_definition::StreamDefinition;
use crate::query_api::execution::query::Query;

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

// ============================================================================
// Dependency Graph & Topological Sort
// ============================================================================

/// Build dependency graph for stream initialization order
///
/// Constructs dependencies from two sources:
/// 1. **Query dependencies**: INSERT INTO target FROM source
/// 2. **DLQ dependencies**: Stream depends on its DLQ stream
///
/// # Arguments
///
/// * `parsed_streams` - All defined streams with their configurations
/// * `queries` - All queries defining data flow
///
/// # Returns
///
/// HashMap mapping stream_name → set of streams it depends on
fn build_dependency_graph(
    parsed_streams: &HashMap<String, (StreamDefinition, FlatConfig)>,
    queries: &[Query],
) -> HashMap<String, HashSet<String>> {
    let mut dependencies: HashMap<String, HashSet<String>> = HashMap::new();

    // 1. Add query dependencies
    for query in queries {
        if let Some(target) = query.get_target_stream() {
            let sources = query.get_source_streams();

            dependencies
                .entry(target)
                .or_insert_with(HashSet::new)
                .extend(sources);
        }
    }

    // 2. Add DLQ dependencies
    // Stream depends on its DLQ stream (DLQ must be initialized first)
    for (stream_name, (_stream_def, config)) in parsed_streams {
        if let Some(dlq_stream) = config.get("error.dlq.stream") {
            dependencies
                .entry(stream_name.clone())
                .or_insert_with(HashSet::new)
                .insert(dlq_stream.clone());
        }
    }

    dependencies
}

/// Perform topological sort on dependency graph using DFS
///
/// Returns initialization order where dependencies appear before dependents.
/// Detects cycles which should not occur due to Phase 1 validation.
///
/// # Arguments
///
/// * `dependencies` - Dependency graph (target → [sources])
///
/// # Returns
///
/// * `Ok(Vec<String>)` - Initialization order (dependencies first)
/// * `Err(EventFluxError)` - If cycle detected during initialization
fn topological_sort(
    dependencies: &HashMap<String, HashSet<String>>,
) -> Result<Vec<String>, EventFluxError> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();

    // Collect all nodes (both sources and targets)
    let mut all_nodes = HashSet::new();
    for (target, sources) in dependencies {
        all_nodes.insert(target.clone());
        all_nodes.extend(sources.iter().cloned());
    }

    for node in &all_nodes {
        if !visited.contains(node) {
            dfs_topo_sort(node, dependencies, &mut visited, &mut visiting, &mut result)?;
        }
    }

    // No need to reverse - DFS adds nodes after visiting dependencies,
    // so they're already in correct topological order (dependencies first)
    Ok(result)
}

/// DFS helper for topological sort
///
/// Recursively visits nodes in depth-first order, detecting cycles.
fn dfs_topo_sort(
    node: &String,
    dependencies: &HashMap<String, HashSet<String>>,
    visited: &mut HashSet<String>,
    visiting: &mut HashSet<String>,
    result: &mut Vec<String>,
) -> Result<(), EventFluxError> {
    if visiting.contains(node) {
        // Cycle detected (should not happen - Phase 1 prevents this)
        return Err(EventFluxError::app_creation(format!(
            "Unexpected cycle detected during initialization: {}",
            node
        )));
    }

    if visited.contains(node) {
        return Ok(());
    }

    visiting.insert(node.clone());

    // Visit all dependencies first
    if let Some(deps) = dependencies.get(node) {
        for dep in deps {
            dfs_topo_sort(dep, dependencies, visited, visiting, result)?;
        }
    }

    visiting.remove(node);
    visited.insert(node.clone());
    result.push(node.clone());

    Ok(())
}

// ============================================================================
// Stream Initialization
// ============================================================================

/// Initialize a single source stream with handler creation
///
/// Creates a fully initialized source with:
/// 1. Source instance from factory
/// 2. Mapper instance from factory
/// 3. Input handler for event processing (from InputManager)
/// 4. SourceStreamHandler for lifecycle management
fn initialize_source_stream_with_handler(
    stream_def: &StreamDefinition,
    stream_config: &StreamTypeConfig,
    context: &EventFluxContext,
    input_manager: &crate::core::stream::input::InputManager,
    stream_name: &str,
) -> Result<Arc<SourceStreamHandler>, EventFluxError> {
    // Use existing initialization logic
    let initialized = initialize_source_stream(context, stream_config)?;

    match initialized {
        InitializedStream::Source(source) => {
            // Get or create InputHandler for this stream using InputManager
            // This properly integrates with the junction system
            let input_handler =
                input_manager
                    .construct_input_handler(stream_name)
                    .map_err(|e| {
                        EventFluxError::app_creation(format!(
                            "Failed to construct input handler for stream '{}': {}",
                            stream_name, e
                        ))
                    })?;

            // Create SourceStreamHandler
            let handler = Arc::new(SourceStreamHandler::new(
                source.source,
                Some(source.mapper),
                input_handler,
                stream_name.to_string(),
            ));

            Ok(handler)
        }
        _ => Err(EventFluxError::app_creation(
            "Expected source stream initialization",
        )),
    }
}

/// Initialize a single sink stream with handler creation
///
/// Creates a fully initialized sink with:
/// 1. Sink instance from factory
/// 2. Mapper instance from factory
/// 3. SinkStreamHandler for lifecycle management
fn initialize_sink_stream_with_handler(
    stream_def: &StreamDefinition,
    stream_config: &StreamTypeConfig,
    context: &EventFluxContext,
    stream_name: &str,
) -> Result<Arc<SinkStreamHandler>, EventFluxError> {
    // Use existing initialization logic
    let initialized = initialize_sink_stream(context, stream_config)?;

    match initialized {
        InitializedStream::Sink(sink) => {
            // Create SinkStreamHandler
            let handler = Arc::new(SinkStreamHandler::new(
                sink.sink,
                Some(sink.mapper),
                stream_name.to_string(),
            ));

            Ok(handler)
        }
        _ => Err(EventFluxError::app_creation(
            "Expected sink stream initialization",
        )),
    }
}

/// Initialize a single stream based on its type
///
/// Delegates to type-specific initialization functions.
/// Returns handlers for registration in the runtime.
fn initialize_single_stream(
    stream_def: &StreamDefinition,
    stream_config: &StreamTypeConfig,
    context: &EventFluxContext,
    input_manager: &crate::core::stream::input::InputManager,
    stream_name: &str,
) -> Result<InitializedStreamHandler, EventFluxError> {
    match stream_config.stream_type {
        StreamType::Source => {
            let handler = initialize_source_stream_with_handler(
                stream_def,
                stream_config,
                context,
                input_manager,
                stream_name,
            )?;

            Ok(InitializedStreamHandler::Source(handler))
        }
        StreamType::Sink => {
            let handler = initialize_sink_stream_with_handler(
                stream_def,
                stream_config,
                context,
                stream_name,
            )?;

            Ok(InitializedStreamHandler::Sink(handler))
        }
        StreamType::Internal => {
            // Internal streams only need junction (no external I/O)
            // Junction creation happens automatically in runtime
            Ok(InitializedStreamHandler::Internal)
        }
    }
}

/// Result of stream handler initialization
pub enum InitializedStreamHandler {
    Source(Arc<SourceStreamHandler>),
    Sink(Arc<SinkStreamHandler>),
    Internal,
}

// ============================================================================
// Full Initialization Flow
// ============================================================================

/// Initialize all streams in topological order
///
/// This is the main entry point for stream initialization during application startup.
///
/// # Flow
///
/// 1. Build dependency graph (query + DLQ dependencies)
/// 2. Validate DLQ stream references (Phase 1 validation)
/// 3. Perform topological sort (dependencies first)
/// 4. Initialize each stream in dependency order
/// 5. Return initialized handlers for runtime registration
///
/// # Arguments
///
/// * `parsed_streams` - Map of stream_name → (StreamDefinition, FlatConfig)
/// * `queries` - All queries defining data flow
/// * `context` - EventFlux context with factory registries
/// * `input_manager` - InputManager for creating input handlers
///
/// # Returns
///
/// * `Ok(StreamHandlers)` - All initialized handlers
/// * `Err(EventFluxError)` - Initialization failed
///
/// # Example
///
/// ```rust,ignore
/// let mut streams = HashMap::new();
/// streams.insert("InputStream".to_string(), (stream_def, config));
///
/// let queries = vec![query];
///
/// let handlers = initialize_streams(streams, queries, &context, &input_manager)?;
/// ```
pub fn initialize_streams(
    parsed_streams: HashMap<String, (StreamDefinition, FlatConfig)>,
    queries: Vec<Query>,
    context: &EventFluxContext,
    input_manager: &crate::core::stream::input::InputManager,
) -> Result<StreamHandlers, EventFluxError> {
    // 1. Build dependency graph
    let dependencies = build_dependency_graph(&parsed_streams, &queries);

    // 2. Validate DLQ stream names exist (Phase 1 validation)
    for (stream_name, (_stream_def, config)) in &parsed_streams {
        if let Some(dlq_stream) = config.get("error.dlq.stream") {
            if !parsed_streams.contains_key(dlq_stream) {
                return Err(EventFluxError::configuration(format!(
                    "DLQ stream '{}' referenced by stream '{}' does not exist",
                    dlq_stream, stream_name
                )));
            }
        }
    }

    // 3. Topological sort
    let init_order = topological_sort(&dependencies)?;

    // 4. Initialize in dependency order
    let mut source_handlers = HashMap::new();
    let mut sink_handlers = HashMap::new();

    for stream_name in &init_order {
        if let Some((stream_def, flat_config)) = parsed_streams.get(stream_name) {
            let stream_config = StreamTypeConfig::from_flat_config(flat_config).map_err(|e| {
                EventFluxError::configuration(format!(
                    "Invalid stream configuration for '{}': {}",
                    stream_name, e
                ))
            })?;

            // Validate DLQ schema if configured (Phase 2 validation)
            if let Some(dlq_stream) = flat_config.get("error.dlq.stream") {
                validate_dlq_schema(stream_name, dlq_stream, &parsed_streams)?;
            }

            match initialize_single_stream(
                stream_def,
                &stream_config,
                context,
                input_manager,
                stream_name,
            )? {
                InitializedStreamHandler::Source(handler) => {
                    source_handlers.insert(stream_name.clone(), handler);
                }
                InitializedStreamHandler::Sink(handler) => {
                    sink_handlers.insert(stream_name.clone(), handler);
                }
                InitializedStreamHandler::Internal => {
                    // Internal streams don't need handlers
                }
            }
        }
    }

    Ok(StreamHandlers {
        source_handlers,
        sink_handlers,
    })
}

/// Collection of initialized stream handlers
pub struct StreamHandlers {
    pub source_handlers: HashMap<String, Arc<SourceStreamHandler>>,
    pub sink_handlers: HashMap<String, Arc<SinkStreamHandler>>,
}

/// Validate DLQ schema compatibility
///
/// Ensures the DLQ stream has the required schema for error events.
///
/// # Required DLQ Schema
///
/// - `originalEvent` STRING
/// - `errorMessage` STRING
/// - `errorType` STRING
/// - `timestamp` BIGINT
/// - `attemptCount` INT
/// - `streamName` STRING
fn validate_dlq_schema(
    stream_name: &str,
    dlq_stream: &str,
    parsed_streams: &HashMap<String, (StreamDefinition, FlatConfig)>,
) -> Result<(), EventFluxError> {
    // For now, just check that DLQ stream exists
    // Full schema validation would be implemented in Phase 2
    if !parsed_streams.contains_key(dlq_stream) {
        return Err(EventFluxError::configuration(format!(
            "DLQ stream '{}' for stream '{}' does not exist",
            dlq_stream, stream_name
        )));
    }

    Ok(())
}

// ============================================================================
// Helper Trait Extensions
// ============================================================================

/// Trait for extracting query dependencies
trait QuerySourceExtractor {
    fn get_source_streams(&self) -> Vec<String>;
    fn get_target_stream(&self) -> Option<String>;
}

impl QuerySourceExtractor for Query {
    fn get_source_streams(&self) -> Vec<String> {
        crate::core::validation::query_helpers::QuerySourceExtractor::get_source_streams(self)
    }

    fn get_target_stream(&self) -> Option<String> {
        crate::core::validation::query_helpers::QuerySourceExtractor::get_target_stream(self)
    }
}

// ============================================================================
// Tests
// ============================================================================

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

    // ========================================================================
    // Topological Sort Tests
    // ========================================================================

    #[test]
    fn test_topological_sort_linear() {
        let mut deps = HashMap::new();
        deps.insert("B".to_string(), HashSet::from(["A".to_string()]));
        deps.insert("C".to_string(), HashSet::from(["B".to_string()]));
        deps.insert("D".to_string(), HashSet::from(["C".to_string()]));

        let order = topological_sort(&deps).unwrap();

        // A should come before B, B before C, C before D
        let a_pos = order.iter().position(|x| x == "A").unwrap();
        let b_pos = order.iter().position(|x| x == "B").unwrap();
        let c_pos = order.iter().position(|x| x == "C").unwrap();
        let d_pos = order.iter().position(|x| x == "D").unwrap();

        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
        assert!(c_pos < d_pos);
    }

    #[test]
    fn test_topological_sort_with_dlq() {
        let mut deps = HashMap::new();
        deps.insert(
            "Orders".to_string(),
            HashSet::from(["OrderErrors".to_string()]),
        );
        deps.insert(
            "Processed".to_string(),
            HashSet::from(["Orders".to_string()]),
        );

        let order = topological_sort(&deps).unwrap();

        // OrderErrors must be before Orders, Orders before Processed
        let errors_pos = order.iter().position(|x| x == "OrderErrors").unwrap();
        let orders_pos = order.iter().position(|x| x == "Orders").unwrap();
        let processed_pos = order.iter().position(|x| x == "Processed").unwrap();

        assert!(errors_pos < orders_pos);
        assert!(orders_pos < processed_pos);
    }

    #[test]
    fn test_topological_sort_multiple_dependencies() {
        let mut deps = HashMap::new();
        // C depends on both A and B
        deps.insert(
            "C".to_string(),
            HashSet::from(["A".to_string(), "B".to_string()]),
        );

        let order = topological_sort(&deps).unwrap();

        // Both A and B must come before C
        let a_pos = order.iter().position(|x| x == "A").unwrap();
        let b_pos = order.iter().position(|x| x == "B").unwrap();
        let c_pos = order.iter().position(|x| x == "C").unwrap();

        assert!(a_pos < c_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn test_topological_sort_diamond_dependency() {
        let mut deps = HashMap::new();
        // Diamond: D depends on B and C, B and C both depend on A
        deps.insert("B".to_string(), HashSet::from(["A".to_string()]));
        deps.insert("C".to_string(), HashSet::from(["A".to_string()]));
        deps.insert(
            "D".to_string(),
            HashSet::from(["B".to_string(), "C".to_string()]),
        );

        let order = topological_sort(&deps).unwrap();

        // A must be before B and C, B and C must be before D
        let a_pos = order.iter().position(|x| x == "A").unwrap();
        let b_pos = order.iter().position(|x| x == "B").unwrap();
        let c_pos = order.iter().position(|x| x == "C").unwrap();
        let d_pos = order.iter().position(|x| x == "D").unwrap();

        assert!(a_pos < b_pos);
        assert!(a_pos < c_pos);
        assert!(b_pos < d_pos);
        assert!(c_pos < d_pos);
    }

    #[test]
    fn test_topological_sort_cycle_detection() {
        let mut deps = HashMap::new();
        // Create a cycle: A -> B -> C -> A
        deps.insert("A".to_string(), HashSet::from(["C".to_string()]));
        deps.insert("B".to_string(), HashSet::from(["A".to_string()]));
        deps.insert("C".to_string(), HashSet::from(["B".to_string()]));

        let result = topological_sort(&deps);
        assert!(result.is_err());

        // Should get an error about cycle detection
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("cycle") || err_msg.contains("Cycle"));
    }

    #[test]
    fn test_topological_sort_empty_graph() {
        let deps = HashMap::new();
        let order = topological_sort(&deps).unwrap();
        assert!(order.is_empty());
    }

    #[test]
    fn test_topological_sort_independent_nodes() {
        let mut deps = HashMap::new();
        deps.insert("A".to_string(), HashSet::new());
        deps.insert("B".to_string(), HashSet::new());
        deps.insert("C".to_string(), HashSet::new());

        let order = topological_sort(&deps).unwrap();
        assert_eq!(order.len(), 3);
        assert!(order.contains(&"A".to_string()));
        assert!(order.contains(&"B".to_string()));
        assert!(order.contains(&"C".to_string()));
    }

    // ========================================================================
    // Dependency Graph Tests
    // ========================================================================

    #[test]
    fn test_build_dependency_graph_query_only() {
        use crate::query_api::execution::query::input::stream::{InputStream, SingleInputStream};
        use crate::query_api::execution::query::output::output_stream::{
            InsertIntoStreamAction, OutputStream, OutputStreamAction,
        };
        use crate::query_api::execution::query::selection::Selector;

        let mut parsed_streams = HashMap::new();
        let stream_a = StreamDefinition::new("A".to_string());
        let stream_b = StreamDefinition::new("B".to_string());

        parsed_streams.insert("A".to_string(), (stream_a, FlatConfig::new()));
        parsed_streams.insert("B".to_string(), (stream_b, FlatConfig::new()));

        // Create query: FROM A INSERT INTO B
        let input_stream = InputStream::Single(SingleInputStream::new_basic(
            "A".to_string(),
            false,
            false,
            None,
            Vec::new(),
        ));

        let output_stream = OutputStream {
            eventflux_element: Default::default(),
            action: OutputStreamAction::InsertInto(InsertIntoStreamAction {
                target_id: "B".to_string(),
                is_inner_stream: false,
                is_fault_stream: false,
            }),
            output_event_type: None,
        };

        let query = Query {
            eventflux_element: Default::default(),
            input_stream: Some(input_stream),
            selector: Selector::new(),
            output_stream,
            output_rate: None,
            annotations: Vec::new(),
        };

        let queries = vec![query];
        let deps = build_dependency_graph(&parsed_streams, &queries);

        // B depends on A
        assert_eq!(deps.len(), 1);
        assert!(deps.get("B").unwrap().contains("A"));
    }

    #[test]
    fn test_build_dependency_graph_with_dlq() {
        let mut parsed_streams = HashMap::new();

        let stream_orders = StreamDefinition::new("Orders".to_string());
        let stream_errors = StreamDefinition::new("OrderErrors".to_string());

        let mut config_orders = FlatConfig::new();
        config_orders.set(
            "error.dlq.stream",
            "OrderErrors",
            PropertySource::TomlStream,
        );

        parsed_streams.insert("Orders".to_string(), (stream_orders, config_orders));
        parsed_streams.insert(
            "OrderErrors".to_string(),
            (stream_errors, FlatConfig::new()),
        );

        let queries = vec![];
        let deps = build_dependency_graph(&parsed_streams, &queries);

        // Orders depends on OrderErrors (DLQ dependency)
        assert_eq!(deps.len(), 1);
        assert!(deps.get("Orders").unwrap().contains("OrderErrors"));
    }

    #[test]
    fn test_build_dependency_graph_combined() {
        use crate::query_api::execution::query::input::stream::{InputStream, SingleInputStream};
        use crate::query_api::execution::query::output::output_stream::{
            InsertIntoStreamAction, OutputStream, OutputStreamAction,
        };
        use crate::query_api::execution::query::selection::Selector;

        let mut parsed_streams = HashMap::new();

        let stream_a = StreamDefinition::new("A".to_string());
        let stream_b = StreamDefinition::new("B".to_string());
        let stream_errors = StreamDefinition::new("Errors".to_string());

        let mut config_b = FlatConfig::new();
        config_b.set("error.dlq.stream", "Errors", PropertySource::TomlStream);

        parsed_streams.insert("A".to_string(), (stream_a, FlatConfig::new()));
        parsed_streams.insert("B".to_string(), (stream_b, config_b));
        parsed_streams.insert("Errors".to_string(), (stream_errors, FlatConfig::new()));

        // Create query: FROM A INSERT INTO B
        let input_stream = InputStream::Single(SingleInputStream::new_basic(
            "A".to_string(),
            false,
            false,
            None,
            Vec::new(),
        ));

        let output_stream = OutputStream {
            eventflux_element: Default::default(),
            action: OutputStreamAction::InsertInto(InsertIntoStreamAction {
                target_id: "B".to_string(),
                is_inner_stream: false,
                is_fault_stream: false,
            }),
            output_event_type: None,
        };

        let query = Query {
            eventflux_element: Default::default(),
            input_stream: Some(input_stream),
            selector: Selector::new(),
            output_stream,
            output_rate: None,
            annotations: Vec::new(),
        };

        let queries = vec![query];
        let deps = build_dependency_graph(&parsed_streams, &queries);

        // B depends on both A (query) and Errors (DLQ)
        assert_eq!(deps.len(), 1);
        let b_deps = deps.get("B").unwrap();
        assert_eq!(b_deps.len(), 2);
        assert!(b_deps.contains("A"));
        assert!(b_deps.contains("Errors"));
    }
}
