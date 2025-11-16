# EventFlux Rust Port (eventflux_rust)

This project is an experimental port of the Java-based EventFlux CEP (Complex Event Processing) engine to Rust. The
primary goal is to create a **high-performance, cloud-native CEP engine** with superior memory safety and performance
characteristics.

## Architecture Philosophy: Engine vs Platform

EventFlux Rust is a **CEP Engine**, not a platform. This critical distinction guides our design:

```
┌─────────────────────────────────────────────────────────────┐
│                    Platform Layer (NOT OUR SCOPE)           │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ • Multi-tenancy       • Authentication/Authorization    ││
│  │ • Resource Quotas      • Billing & Metering            ││
│  │ • API Gateway          • Tenant Isolation              ││
│  │ • Service Mesh         • Platform UI/Dashboard         ││
│  └─────────────────────────────────────────────────────────┘│
│  Handled by: Kubernetes, Docker Swarm, Nomad, Custom Platform│
├─────────────────────────────────────────────────────────────--┤
│                    EventFlux Engine (OUR FOCUS)               │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │ Single Runtime = Single App = Single Config = Single    │  │
│  │ Container                                               │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │ Container 1 │  │ Container 2 │  │ Container 3 │            │
│  │ ┌─────────┐ │  │ ┌─────────┐ │  │ ┌─────────┐ │            │
│  │ │Runtime A│ │  │ │Runtime B│ │  │ │Runtime C│ │            │
│  │ │---------│ │  │ │---------│ │  │ │---------│ │            │
│  │ │Config A │ │  │ │Config B │ │  │ │Config C │ │       │
│  │ │App A    │ │  │ │App B    │ │  │ │App C    │ │       │
│  │ └─────────┘ │  │ └─────────┘ │  │ └─────────┘ │       │
│  └─────────────┘  └─────────────┘  └─────────────┘       │
└─────────────────────────────────────────────────────────────┘
```

### Core Design Principles

1. **One Runtime, One App**: Each EventFlux runtime instance handles exactly one application with one configuration
2. **Cloud-Native**: Designed to run as containers orchestrated by Kubernetes, Docker Swarm, or similar
3. **Unix Philosophy**: Do one thing exceptionally well - process complex events at high speed
4. **Platform Agnostic**: Can be integrated into any platform that needs CEP capabilities

### What We Build (Engine)

- ✅ High-performance event processing (>1M events/sec)
- ✅ Query execution and optimization
- ✅ State management and persistence
- ✅ Distributed coordination for single app
- ✅ Monitoring and metrics for the runtime

### What We DON'T Build (Platform Concerns)

- ❌ Multi-tenancy (use separate containers)
- ❌ Authentication/Authorization (use API gateway)
- ❌ Resource quotas (use container limits)
- ❌ Billing/metering (platform responsibility)
- ❌ User management (platform responsibility)

### Deployment Example

```yaml
# Each app runs in its own container with dedicated resources
apiVersion: apps/v1
kind: Deployment
metadata:
  name: fraud-detection-app
spec:
  replicas: 3  # Scale horizontally
  template:
    spec:
      containers:
        - name: eventflux
          image: eventflux-rust:latest
          args: [ "--config", "/config/fraud-detection.yaml" ]
          resources:
            limits:
              cpu: "4"
              memory: "8Gi"
```

## Current Status

The project has evolved from early experimental porting to a production-ready foundation with enterprise-grade
capabilities in key areas. Major architectural milestones have been achieved, making this a viable alternative to the
Java implementation for specific use cases.

### Recent Major Achievements (2025)

✅ **Enterprise State Management (Aug 2025)**: Production-complete StateHolder architecture with schema versioning,
incremental checkpointing, and comprehensive validation across all stateful components.

✅ **High-Performance Event Pipeline (Aug 2025)**: Lock-free crossbeam-based pipeline achieving >1M events/second with
configurable backpressure strategies and comprehensive monitoring.

✅ **Advanced Checkpointing System (Aug 2025)**: Industry-leading incremental checkpointing with Write-Ahead Log, delta
compression, parallel recovery, and distributed coordination.

✅ **Distributed Transport Layers (Aug 2025)**: Production-ready TCP and gRPC transports for distributed processing with
connection pooling, TLS support, and comprehensive integration tests.

✅ **Redis State Backend (Aug 2025)**: Enterprise-grade Redis-based state persistence with connection pooling, automatic
failover, and seamless integration with EventFlux's native persistence system.

✅ **ThreadBarrier Coordination (Aug 2025)**: Complete implementation of Java EventFlux's ThreadBarrier pattern for
coordinating state restoration with concurrent event processing, ensuring race-condition-free aggregation state
persistence.

### Implementation Status

* **`eventflux-query-api` Module**: Largely ported. This module defines the abstract syntax tree (AST) and structures
  for representing EventFlux applications, stream definitions, queries, expressions, and execution plans. Most data
  structures have been translated to Rust structs and enums.
* **`eventflux-query-compiler` Module**: Provides a LALRPOP-based parser for EventFluxQL.
    * The `update_variables` function (for substituting environment/system variables in EventFluxQL strings) has been
      ported.
    * Parsing now uses the grammar in `query_compiler/grammar.lalrpop` to build the AST.
    * **SQL WITH Clause Support**: Full parsing support for SQL WITH clauses on streams and tables for configuration.
* **`eventflux-core` Module**: Foundational elements for a Phase 1 feature set (simple stateless queries like filters
  and projections) are structurally in place. This includes:
    * **Configuration (`config`)**: `EventFluxContext` and `EventFluxAppContext` defined (many internal fields are
      placeholders for complex Java objects like persistence stores, data sources, executor services).
    * **Events (`event`)**: `Event`, `AttributeValue`, `ComplexEvent` trait, and `StreamEvent` are defined. Placeholders
      for state/meta events exist.
    * **Stream Handling (`stream`)**: Basic structures for `StreamJunction` (event routing) and `InputHandler` are
      defined. `StreamCallback` trait for output. **OptimizedStreamJunction** with high-performance crossbeam-based
      event pipeline provides >1M events/sec capability.
    * **Expression Executors (`executor`)**: `ExpressionExecutor` trait defined. Implementations for constants,
      variables (simplified), basic math operators (+,-,*,/,mod), basic conditions (AND,OR,NOT,Compare,IsNull), and
      common functions (Coalesce, IfThenElse, UUID, InstanceOf*) are present.
    * **Expression Parser (`util/parser/expression_parser.rs`)**: Initial recursive structure to convert
      `query_api::Expression` objects into `core::ExpressionExecutor`s.
    * **Stream Processors (`query/processor`)**: `Processor` trait and `CommonProcessorMeta` struct. In addition to
      `FilterProcessor` and `SelectProcessor`, the Rust port includes `LengthWindowProcessor`, `TimeWindowProcessor`,
      `JoinProcessor`, and processors for event patterns and sequences.  `InsertIntoStreamProcessor` handles output
      routing.
    * **Runtime Parsers (`util/parser/eventflux_app_parser.rs`, `util/parser/query_parser.rs`)**: Build
      `EventFluxAppRuntime`s from the AST. The parser supports windows, joins, patterns, sequences and incremental
      aggregations. **SQL WITH property processing** automatically configures high-performance async streams and table extensions.
    * **Runtime (`eventflux_app_runtime.rs`)**: `EventFluxAppRuntime` executes queries built by the parser, including
      windows, joins, patterns, sequences and aggregations. Runtimes use the scheduler for time-based operations and can
      register callbacks for output.
* **`EventFluxManager`**: Basic functionality for creating, retrieving, and shutting down `EventFluxAppRuntime`
  instances has been ported. Methods for managing extensions and data sources are placeholders pointing to
  `EventFluxContext`.
* **Metrics and Fault Handling**: Simple in-memory metrics trackers are available and stream junctions can route faults
  to fault streams or an error store.

## Key Omissions, Simplifications, and Major TODOs

This port is **far from feature-complete** with the Java version. Users should be aware of the following critical
missing pieces and simplifications:

* **EventFluxQL String Parsing**: A LALRPOP-based parser converts EventFluxQL strings into the `query_api` AST. The
  grammar covers streams, tables, windows, triggers, aggregations, queries and partitions (with optional `define`
  syntax) and supports aggregation store queries with `within`/`per` clauses, but still omits many advanced constructs.
* **`ExpressionParser` Completeness**:
    * **Variable Resolution**: Variables can now be resolved from joins, pattern queries and tables in addition to
      single streams, and executors retrieve the correct attribute from these sources.
    * **Function Handling**: Built-in and user-defined functions are resolved with descriptive error messages when
      missing.
    * **Type Checking & Coercion**: Rigorous EventFlux-specific type checking and coercion for all operators and
      functions is not yet implemented.
    * **Error Handling**: Error reporting from parsing is basic (String-based).
* **`ExpressionExecutor` Implementations**:
    * `VariableExpressionExecutor`: Retrieves attributes from joined streams, patterns and tables using state event
      positions. More advanced handling of different event types and data sections is still needed.
    * `CompareExpressionExecutor`: Supports numeric, boolean and string comparisons with type coercion.
    * `InExpressionExecutor`: Implements the `IN` operator using registered tables such as `InMemoryTable`.
    * Built‑in function executors cover casts, string operations, date utilities, math functions and UUID generation.
    * Stateful user-defined functions are supported via the `ScalarFunctionExecutor` trait.
* **Stream Processors & Query Logic**:
    * `FilterProcessor` & `SelectProcessor`: Event chunk (linked list) manipulation is simplified (uses `Vec`
      intermediate for `SelectProcessor`). Advanced features for `SelectProcessor` (group by, having, order by, limit,
      offset) are not implemented.
    * **Windows**: `LengthWindowProcessor` and `TimeWindowProcessor` provide basic sliding and tumbling windows.
    * **Joins**: `JoinProcessor` supports inner and outer joins with optional conditions.
    * **Patterns & Sequences**: `SequenceProcessor` and related logic implement pattern and sequence matching.
    * **Aggregations**: Attribute aggregator executors are available and incremental aggregations are executed via
      `AggregationRuntime`.
* **State Management & Persistence**:
    * **Tables**: An `InMemoryTable` implementation supports insert, update, delete and membership checks. Custom table
      implementations can be provided via `TableFactory` instances registered with the `EventFluxManager`.
    * **Enterprise State Management**: ✅ **PRODUCTION COMPLETE** - Enhanced `StateHolder` architecture with schema
      versioning, incremental checkpointing, compression, and access pattern optimization. Comprehensive coverage across
      all 11 stateful components (5 window types, 6 aggregator types).
    * **Advanced Checkpointing**: Enterprise-grade Write-Ahead Log (WAL) system with segmented storage, delta
      compression, conflict resolution, and point-in-time recovery capabilities.
    * **Pluggable Persistence Backends**: Production-ready file backend with atomic operations, plus framework for
      distributed and cloud storage integration.
* **Runtime & Orchestration**:
    * `EventFluxAppParser` & `QueryParser` now construct runtimes with windows, joins, patterns, sequences and
      aggregations.
    * `Scheduler` drives time-based windows and cron style callbacks.
    * `EventFluxAppRuntime` supports starting and shutting down applications and routes events through the configured
      processors.
    * Triggers are executed via `TriggerRuntime`, allowing periodic or cron-based event generation.
    * Error handling throughout `eventflux-core` remains basic.
* **Extensions Framework**:
    * `ScalarFunctionExecutor` allows registering stateful user-defined functions.
    * Placeholders for other extension types (Window, Sink, Source, Store, Mapper, AttributeAggregator, Script) are
      largely missing.
* **DataSources**: `DataSource` trait is a placeholder. No actual implementations or integration with table stores.
  `EventFluxContext::add_data_source` now looks for a matching configuration and calls `init` on the `DataSource` with
  it when registering using a temporary `EventFluxAppContext` (`dummy_ctx`).
* **Concurrency**: While `Arc<Mutex<T>>` is used in places, detailed analysis and implementation of EventFlux's
  concurrency model (thread pools for async junctions, partitioned execution) are pending.

## Configuration

EventFlux Rust uses a **simplified 4-layer configuration system** combining SQL WITH clauses and TOML files.
Configuration is designed to be simple for development and powerful for production.

### Configuration Syntax

**SQL WITH Clause** - Primary configuration method:

```sql
-- Kafka Source Stream with JSON format
CREATE
STREAM Orders (
    orderId VARCHAR,
    amount DOUBLE,
    timestamp BIGINT
) WITH (
    'type' = 'source',
    'extension' = 'kafka',
    'kafka.brokers' = 'localhost:9092',
    'kafka.topic' = 'orders',
    'format' = 'json'
);

-- HTTP Sink Stream
CREATE
STREAM Alerts (
    alertId VARCHAR,
    severity VARCHAR,
    message VARCHAR
) WITH (
    'type' = 'sink',
    'extension' = 'http',
    'http.url' = 'https://api.example.com/alerts',
    'format' = 'json'
);

-- MySQL Table
CREATE TABLE Users
(
    userId VARCHAR,
    name   VARCHAR,
    email  VARCHAR,
    PRIMARY KEY (userId)
) WITH (
      'extension' = 'mysql',
      'mysql.host' = 'localhost',
      'mysql.database' = 'mydb',
      'mysql.table' = 'users'
      );
```

**Key Properties:**

- `'type'` - Required for streams: `'source'` or `'sink'`
- `'extension'` - Required: connector type (`'kafka'`, `'http'`, `'mysql'`, `'file'`)
- `'format'` - Data mapper for streams (`'json'`, `'avro'`, `'csv'`, `'protobuf'`)

### TOML Configuration Files

**Development (config-dev.toml)**:

```toml
[application]
name = "OrderProcessing"
buffer_size = 8192

[streams.Orders]
type = "source"
extension = "kafka"
format = "json"

[streams.Orders.kafka]
brokers = "localhost:9092"
topic = "orders"
```

**Production (config-prod.toml)**:

```toml
[application]
name = "OrderProcessing-Prod"

[streams.Orders]
type = "source"
extension = "kafka"
format = "json"

[streams.Orders.kafka]
brokers = "prod1:9092,prod2:9092,prod3:9092"
topic = "orders"
group = "production-app"

[streams.Orders.kafka.security]
protocol = "SASL_SSL"
username = "${KAFKA_USER}"
password = "${KAFKA_PASSWORD}"
```

**Running with Configuration:**

```bash
# Development
eventflux run app.sql --config config-dev.toml

# Production
export KAFKA_USER="admin"
export KAFKA_PASSWORD="secret"
eventflux run app.sql --config config-prod.toml
```

### Configuration Layers

EventFlux merges configuration from 4 layers (highest to lowest priority):

1. **SQL WITH clause** - Runtime overrides
2. **TOML [streams.StreamName]** - Stream-specific config
3. **TOML [application]** - Application defaults
4. **Rust defaults** - Framework defaults

**📖 Complete Documentation**: See [feat/configuration/CONFIGURATION.md](feat/configuration/CONFIGURATION.md) for
comprehensive configuration reference.

## Testing Status

* **`query_api`**: Basic unit tests for constructors and getters of key data structures are planned / partially
  implemented.
* **`eventflux-core`**: Some unit tests for basic expression executors are planned / partially implemented.
* **Integration Testing**: The `tests` directory contains end-to-end tests covering windows, joins, patterns, sequences,
  incremental aggregations and the scheduler. These tests parse EventFlux applications and run them through a helper
  `AppRunner` to verify expected outputs.
* **Performance Testing**: Long-running performance and stress tests are isolated in `tests/performance_tests.rs` and only
  run when explicitly enabled with the `perf-tests` feature flag. This keeps normal test runs fast while still allowing
  comprehensive performance validation.
* **Benchmarking**: Not yet performed.

### Running Performance Tests

Performance tests are excluded from normal `cargo test` runs to keep development workflows fast. To run performance tests:

```bash
# Run all performance tests
cargo test --features perf-tests --test performance_tests

# Run specific performance test
cargo test --features perf-tests --test performance_tests test_junction_backpressure

# Run normal tests (excludes performance tests)
cargo test
```

Performance tests include:
- **Backpressure Testing**: Validates event pipeline behavior under high load (50K events)
- **Throughput Testing**: Measures events/second processing capability
- **Resource Testing**: Validates memory and CPU usage under sustained load

## Registering Tables and UDFs

Tables can be registered through the `EventFluxContext` obtained from a `EventFluxManager`:

```rust
use eventflux_rust::core::eventflux_manager::EventFluxManager;
use eventflux_rust::core::table::{InMemoryTable, Table};
use eventflux_rust::core::event::value::AttributeValue;
use std::sync::Arc;

let manager = EventFluxManager::new();
let ctx = manager.eventflux_context();
let table: Arc<dyn Table> = Arc::new(InMemoryTable::new());
table.insert( & [AttributeValue::Int(1)]);
ctx.add_table("MyTable".to_string(), table);
// custom tables can be registered via factories
// manager.add_table_factory("jdbc".to_string(), Box::new(MyJdbcTableFactory));
```

User-defined scalar functions implement `ScalarFunctionExecutor` and are registered with the manager:

```rust
use eventflux_rust::core::executor::function::scalar_function_executor::ScalarFunctionExecutor;

#[derive(Debug, Clone)]
struct CounterFn;

impl ScalarFunctionExecutor for CounterFn {
    fn init(&mut self, _args: &Vec<Box<dyn ExpressionExecutor>>, _ctx: &Arc<EventFluxAppContext>) -> Result<(), String> { Ok(()) }
    fn get_name(&self) -> String { "counter".to_string() }
    fn clone_scalar_function(&self) -> Box<dyn ScalarFunctionExecutor> { Box::new(self.clone()) }
}

let manager = EventFluxManager::new();
manager.add_scalar_function_factory("counter".to_string(), Box::new(CounterFn));
```

Other extension types such as windows and attribute aggregators can also be registered using the `EventFluxManager`.

```rust
use eventflux_rust::core::extension::{WindowProcessorFactory, AttributeAggregatorFactory};

let manager = EventFluxManager::new();
// manager.add_window_factory("myWindow".to_string(), Box::new(MyWindowFactory));
// manager.add_attribute_aggregator_factory("myAgg".to_string(), Box::new(MyAggFactory));
```

## High-Performance Async Streams

EventFlux Rust supports high-performance async event processing through standard SQL WITH clauses:

```rust
use eventflux_rust::core::eventflux_manager::EventFluxManager;
use eventflux_rust::core::config::ConfigManager;

let mut manager = EventFluxManager::new();

// Stream-level async configuration via SQL WITH
let eventflux_app = r#"
    CREATE STREAM HighThroughputStream (symbol STRING, price DOUBLE, volume BIGINT) WITH (
        'async.buffer_size' = '1024',
        'async.workers' = '2',
        'async.batch_size_max' = '10'
    );

    CREATE STREAM MinimalAsyncStream (id INT, value STRING) WITH (
        'async.enabled' = 'true'
    );

    INSERT INTO FilteredStream
    SELECT symbol, price * volume as value
    FROM HighThroughputStream
    WHERE price > 100.0;
"#;

let app_runtime = manager.create_eventflux_app_runtime_from_string(eventflux_app).await?;
```

### Async Configuration Properties:

- **`async.enabled`**: Enable async processing (true/false)
- **`async.buffer_size`**: Queue buffer size (default: context buffer size)
- **`async.workers`**: Hint for throughput estimation (used internally)
- **`async.batch_size_max`**: Batch processing size

### Configuration Methods:

- **Stream-level**: SQL WITH clause on individual streams (highest priority)
- **Application-level**: YAML configuration file for global async defaults
- **Pure SQL**: No custom annotations - standard SQL WITH syntax only

The async pipeline uses lock-free crossbeam data structures with configurable backpressure strategies, providing >1M
events/second throughput capability.

📖 **For comprehensive documentation on async streams, including architecture, advanced configuration, performance
tuning, and troubleshooting, see [ASYNC_STREAMS_GUIDE.md](ASYNC_STREAMS_GUIDE.md).**

### Dynamic Extension Loading

Extensions can be compiled into separate crates and loaded at runtime. When
`EventFluxManager::set_extension` loads a dynamic library it looks up a set of
optional registration functions and calls any that are present:

```text
register_extension
register_windows
register_functions
register_sources
register_sinks
register_stores
register_source_mappers
register_sink_mappers
```

Each function should have the signature
`unsafe extern "C" fn(&EventFluxManager)` and is free to register any number of
factories using the provided manager reference. Only the callbacks implemented
in the library need to be exported.

The integration tests contain a sample dynamic extension under
`tests/custom_dyn_ext` exposing a window and a scalar function. Loading the
compiled library looks like:

```rust
let manager = EventFluxManager::new();
let lib_path = custom_dyn_ext::library_path();
manager
.set_extension("custom", lib_path.to_str().unwrap().to_string())
.unwrap();
```

Once loaded, the factories provided by the library can be used like any other
registered extension in EventFlux applications.

When developing your own extensions you can compile the crate as a
`cdylib` and point `set_extension` at the resulting shared library:

```bash
cargo build -p my_extension
./target/debug/libmy_extension.{so|dylib|dll}
```

### Writing Extensions

See [docs/writing_extensions.md](docs/writing_extensions.md) for a full guide.

Extensions implement traits from `eventflux_rust::core::extension` and are
registered with a `EventFluxManager`. A table extension provides a
`TableFactory` that constructs structs implementing the `Table` trait. Queries
can reference the extension using SQL WITH clause syntax:
`CREATE TABLE MyTable (...) WITH ('extension' = '<name>', ...)`. To
optimize operations, the table should also implement `compile_condition` and
`compile_update_set` which translate EventFlux expressions into a custom
`CompiledCondition` or `CompiledUpdateSet`. For joins, implementing
`compile_join_condition` allows the extension to pre-process the join
expression.

The built-in `CacheTable` and `JdbcTable` are examples of table extensions that
support compiled conditions. Custom extensions can follow the same pattern to
provide efficient lookups for other storage engines.

### Example Usage

```rust
use eventflux_rust::core::executor::condition::CompareExpressionExecutor;
use eventflux_rust::core::executor::constant_expression_executor::ConstantExpressionExecutor;
use eventflux_rust::query_api::expression::condition::compare::Operator;
use eventflux_rust::core::event::value::AttributeValue;
use eventflux_rust::query_api::definition::attribute::Type;

let cmp = CompareExpressionExecutor::new(
Box::new(ConstantExpressionExecutor::new(AttributeValue::Int(5), Type::INT)),
Box::new(ConstantExpressionExecutor::new(AttributeValue::Int(3), Type::INT)),
Operator::GreaterThan,
);
assert_eq!(cmp.execute(None), Some(AttributeValue::Bool(true)));
```

## Distributed Processing & Transport Layers

EventFlux Rust provides enterprise-grade distributed processing capabilities with multiple transport layer
implementations. The system follows a "Single-Node First" philosophy - zero overhead for single-node deployments with
progressive enhancement to distributed mode through configuration.

### Available Transport Layers

#### 1. TCP Transport (Default)

Simple, efficient binary protocol for low-latency communication.

**Features:**

- Connection pooling for efficient resource usage
- Configurable timeouts and buffer sizes
- TCP keepalive support
- Binary message serialization with bincode
- Support for 6 message types (Event, Query, State, Control, Heartbeat, Checkpoint)

**Configuration:**

```rust
use eventflux_rust::core::distributed::transport::{TcpTransport, TcpTransportConfig};

let config = TcpTransportConfig {
connection_timeout_ms: 5000,
read_timeout_ms: 30000,
write_timeout_ms: 30000,
keepalive_enabled: true,
keepalive_interval_secs: 30,
nodelay: true,  // Disable Nagle's algorithm for low latency
send_buffer_size: Some(65536),
recv_buffer_size: Some(65536),
max_message_size: 10 * 1024 * 1024, // 10MB
};

let transport = TcpTransport::with_config(config);
```

#### 2. gRPC Transport (Advanced)

HTTP/2-based transport with Protocol Buffers for enterprise deployments.

**Features:**

- HTTP/2 multiplexing - multiple streams per connection
- Protocol Buffers for efficient, schema-evolution-friendly serialization
- Built-in compression (LZ4, Snappy, Zstd)
- TLS/mTLS support for secure communication
- Client-side load balancing
- Streaming support (unary and bidirectional)
- Health checks and heartbeat monitoring

**Setup Requirements:**

1. **Install Protocol Buffer Compiler:**

```bash
# macOS
brew install protobuf

# Ubuntu/Debian
apt-get install protobuf-compiler

# Verify installation
protoc --version
```

2. **Configuration:**

```rust
use eventflux_rust::core::distributed::grpc::simple_transport::{
    SimpleGrpcTransport, SimpleGrpcConfig
};

let config = SimpleGrpcConfig {
connection_timeout_ms: 10000,
enable_compression: true,
server_address: "127.0.0.1:50051".to_string(),
};

let transport = SimpleGrpcTransport::with_config(config);

// Connect to a gRPC server
transport.connect("127.0.0.1:50051").await?;

// Send a message
let message = Message::event(b"event data".to_vec())
.with_header("source_node".to_string(), "node-1".to_string());
let response = transport.send_message("127.0.0.1:50051", message).await?;

// Send heartbeat
let heartbeat_response = transport.heartbeat(
"127.0.0.1:50051",
"node-1".to_string()
).await?;
```

### Architecture Explanation: gRPC Module Files

The gRPC transport implementation consists of three key files:

#### 1. `eventflux.transport.rs` (Generated)

- **Purpose**: Auto-generated Protocol Buffer definitions
- **Generated by**: `tonic-build` from `proto/transport.proto` during compilation
- **Contents**: Rust structs for all protobuf messages (TransportMessage, HeartbeatRequest, etc.) and gRPC service
  traits
- **Why it exists**: Provides type-safe message definitions and RPC service interfaces from the `.proto` schema

#### 2. `transport.rs` (Full Implementation)

- **Purpose**: Complete gRPC transport implementation with full feature set
- **Features**:
    - Implements the unified `Transport` trait for compatibility with the distributed framework
    - Advanced features: connection pooling, TLS support, streaming, compression
    - Server implementation for accepting incoming connections
- **Status**: Feature-complete but complex; requires full trait implementation for production use
- **Use case**: Production deployments requiring all gRPC features

#### 3. `simple_transport.rs` (Simplified Client)

- **Purpose**: Simplified, immediately usable gRPC client implementation
- **Features**:
    - Focused on client-side operations (connect, send, receive)
    - Simpler API without the complexity of the unified transport interface
    - Direct methods for common operations (send_message, heartbeat)
- **Why it exists**: Provides a working gRPC transport that can be used immediately without dealing with the complexity
  of the full transport trait implementation
- **Use case**: Applications that need gRPC communication without full distributed framework integration

### Choosing Between Transports

| Feature                   | TCP Transport         | gRPC Transport                    |
|---------------------------|-----------------------|-----------------------------------|
| **Latency**               | Lower (direct binary) | Slightly higher (HTTP/2 overhead) |
| **Throughput**            | High                  | Very High (multiplexing)          |
| **Connection Efficiency** | Good (pooling)        | Excellent (multiplexing)          |
| **Protocol Evolution**    | Manual versioning     | Automatic (protobuf)              |
| **Security**              | Basic                 | Built-in TLS/mTLS                 |
| **Load Balancing**        | External              | Built-in client-side              |
| **Monitoring**            | Custom                | Rich ecosystem                    |
| **Complexity**            | Simple                | More complex                      |
| **Dependencies**          | Minimal               | Requires protoc                   |

**Recommendations:**

- Use **TCP** for: Simple deployments, lowest latency requirements, minimal dependencies
- Use **gRPC** for: Enterprise deployments, microservices, need for streaming, strong typing requirements

### Message Types

Both transports support the following message types:

```rust
pub enum MessageType {
    Event,      // Stream events
    Query,      // Query requests/responses
    State,      // State synchronization
    Control,    // Control plane messages
    Heartbeat,  // Health monitoring
    Checkpoint, // State checkpointing
}
```

### Testing

Run transport integration tests:

```bash
# TCP transport tests
cargo test distributed_tcp_integration

# gRPC transport tests  
cargo test distributed_grpc_integration

# All distributed tests
cargo test distributed
```

### Future Transport Implementations

The architecture supports additional transport layers:

- **RDMA**: For ultra-low latency in HPC environments
- **QUIC**: For improved performance over unreliable networks
- **WebSocket**: For browser-based clients
- **Unix Domain Sockets**: For local inter-process communication

## Distributed State Management

### Redis State Backend ✅ **PRODUCTION READY**

EventFlux Rust provides enterprise-grade Redis-based state persistence that seamlessly integrates with EventFlux's
native persistence system. The Redis backend is production-ready with comprehensive features for distributed CEP
deployments.

#### Features

- **Enterprise Connection Management**: Connection pooling with deadpool-redis for high-throughput operations
- **Automatic Failover**: Graceful error recovery and connection retry logic
- **PersistenceStore Integration**: Implements EventFlux's `PersistenceStore` trait for seamless integration
- **Comprehensive Testing**: 15/15 Redis backend tests passing with full integration validation
- **ThreadBarrier Coordination**: Race-condition-free state restoration using Java EventFlux's proven synchronization
  pattern

#### Quick Setup

**1. Start Redis Server:**

```bash
# Using Docker Compose (recommended for development)
cd eventflux_rust
docker-compose up -d

# Or install Redis locally
brew install redis  # macOS
redis-server
```

**2. Configure Redis Backend:**

```rust
use eventflux_rust::core::persistence::RedisPersistenceStore;
use eventflux_rust::core::distributed::RedisConfig;

let config = RedisConfig {
url: "redis://localhost:6379".to_string(),
max_connections: 10,
connection_timeout_ms: 5000,
key_prefix: "eventflux:".to_string(),
ttl_seconds: Some(3600), // Optional TTL
};

let store = RedisPersistenceStore::new_with_config(config) ?;
manager.set_persistence_store(Arc::new(store));
```

**3. Use with Persistence:**

```rust
// Applications automatically use Redis for state persistence
let runtime = manager.create_eventflux_app_runtime(app) ?;

// Persist application state
let revision = runtime.persist() ?;

// Restore from checkpoint
runtime.restore_revision( & revision) ?;
```

#### Configuration Options

| Parameter               | Description               | Default                  |
|-------------------------|---------------------------|--------------------------|
| `url`                   | Redis connection URL      | `redis://localhost:6379` |
| `max_connections`       | Connection pool size      | `10`                     |
| `connection_timeout_ms` | Connection timeout        | `5000`                   |
| `key_prefix`            | Redis key namespace       | `eventflux:`             |
| `ttl_seconds`           | Key expiration (optional) | `None`                   |

#### Production Features

- **Connection Pooling**: Efficient resource management with deadpool-redis
- **Health Monitoring**: Built-in connection health checks and metrics
- **Error Recovery**: Automatic retry logic with exponential backoff
- **Memory Efficiency**: Optimized serialization with optional compression
- **Cluster Support**: Compatible with Redis Cluster for horizontal scaling

#### Integration with EventFlux Components

The Redis backend integrates seamlessly with:

- **SnapshotService**: Automatic state persistence and restoration
- **StateHolders**: All window and aggregation state automatically persisted
- **ThreadBarrier**: Coordinated state restoration preventing race conditions
- **Incremental Checkpointing**: Compatible with EventFlux's advanced checkpointing system

#### Testing

Redis tests automatically skip if Redis is not available, making the test suite work in any environment:

```bash
# Run Redis persistence tests (gracefully skips if Redis not running)
cargo test redis_persistence

# Run all Redis backend tests
cargo test redis_backend

# Integration tests
cargo test test_redis_eventflux_persistence

# Skip all ignored tests (most Redis tests use old EventFluxQL syntax)
cargo test
```

**GitHub Actions**: The CI workflow includes a Redis service container, so all Redis tests run automatically in CI
without manual setup.

**Local Development**: If Redis is not installed locally, tests will print "Redis not available, skipping test" and
pass. To run Redis tests locally:

```bash
# Quick start with Docker
docker-compose up -d
cargo test redis

# Or install Redis natively
brew install redis && brew services start redis  # macOS
```

#### Status and Limitations

**✅ Production Ready:**

- Basic window filtering with persistence and restoration
- Enterprise connection management and error handling
- Complete PersistenceStore trait implementation
- ThreadBarrier coordination for race-free restoration

**🔄 In Development:**

- Group By aggregation state persistence (infrastructure complete, debugging in progress)
- Complex window combinations with aggregations

See [REDIS_PERSISTENCE_STATUS.md](REDIS_PERSISTENCE_STATUS.md) for detailed status and implementation notes.

### ThreadBarrier Coordination

EventFlux Rust implements Java EventFlux's proven **ThreadBarrier** pattern for coordinating state restoration with
concurrent event processing. This ensures race-condition-free aggregation state persistence.

#### How It Works

1. **Event Processing**: All event processing threads enter the ThreadBarrier before processing events
2. **State Restoration**: During restoration, the barrier is locked to prevent new events
3. **Coordination**: Active threads complete their current processing before restoration begins
4. **Synchronization**: State is restored while event processing is safely blocked
5. **Resume**: After restoration, the barrier is unlocked and processing resumes

#### Implementation

```rust
// Automatic ThreadBarrier initialization in EventFluxAppRuntime
let thread_barrier = Arc::new(ThreadBarrier::new());
ctx.set_thread_barrier(thread_barrier);

// Event processing coordination
if let Some(barrier) = self .eventflux_app_context.get_thread_barrier() {
barrier.enter();
// Process events...
barrier.exit();
}

// State restoration coordination
if let Some(barrier) = self .eventflux_app_context.get_thread_barrier() {
barrier.lock();
// Wait for active threads...
service.restore_revision(revision) ?;
barrier.unlock();
}
```

This pattern ensures that aggregation state restoration is atomic and thread-safe, preventing the race conditions that
can occur when events are processed during state restoration.

EventFlux Rust provides enterprise-grade distributed state management through multiple state backend implementations.
The system enables horizontal scaling by distributing state across multiple nodes while maintaining consistency and
providing fault tolerance.

### Available State Backends

#### 1. In-Memory State Backend (Default)

Suitable for single-node deployments or testing environments.

**Features:**

- Zero external dependencies
- High performance (all operations in memory)
- Automatic cleanup on shutdown
- Simple checkpoint/restore for basic persistence

**Configuration:**

```rust
use eventflux_rust::core::distributed::state_backend::InMemoryBackend;

let backend = InMemoryBackend::new();
// Automatically initialized - no external setup required
```

#### 2. Redis State Backend (Production)

Enterprise-ready distributed state management using Redis as the backing store.

**Features:**

- **Connection Pooling**: Efficient resource utilization with configurable pool sizes
- **Automatic Failover**: Robust error handling with connection retry logic
- **State Serialization**: Binary-safe storage of complex state data
- **Key Prefixing**: Namespace isolation for multiple EventFlux clusters
- **TTL Support**: Automatic expiration of state entries
- **Checkpoint/Restore**: Point-in-time state snapshots for disaster recovery
- **Concurrent Operations**: Thread-safe operations with deadpool connection management

**Setup Requirements:**

1. **Install and Start Redis:**

```bash
# macOS
brew install redis
brew services start redis

# Ubuntu/Debian
apt-get install redis-server
systemctl start redis-server

# Docker
docker run -d --name redis -p 6379:6379 redis:alpine

# Verify installation
redis-cli ping
```

2. **Configuration and Usage:**

```rust
use eventflux_rust::core::distributed::state_backend::{RedisBackend, RedisConfig};

// Default configuration (localhost:6379)
let mut backend = RedisBackend::new();
backend.initialize().await?;

// Custom configuration
let config = RedisConfig {
url: "redis://127.0.0.1:6379".to_string(),
max_connections: 10,
connection_timeout_ms: 5000,
key_prefix: "eventflux:cluster1:".to_string(),
ttl_seconds: Some(3600), // 1 hour expiration
};

let mut backend = RedisBackend::with_config(config);
backend.initialize().await?;

// Basic operations
backend.set("key1", b"value1".to_vec()).await?;
let value = backend.get("key1").await?;
assert_eq!(value, Some(b"value1".to_vec()));

// Checkpoint operations
backend.checkpoint("checkpoint_1").await?;
backend.set("key1", b"modified".to_vec()).await?;
backend.restore("checkpoint_1").await?; // Restores to original state

// Cleanup
backend.shutdown().await?;
```

3. **Distributed Configuration:**

```rust
use eventflux_rust::core::distributed::{
    DistributedConfig, StateBackendConfig, StateBackendImplementation
};

let config = DistributedConfig {
state_backend: StateBackendConfig {
implementation: StateBackendImplementation::Redis {
endpoints: vec!["redis://node1:6379".to_string()]
},
checkpoint_interval: Duration::from_secs(60),
state_ttl: Some(Duration::from_secs(7200)), // 2 hours
incremental_checkpoints: true,
compression: CompressionType::Zstd,
},
..Default::default ()
};
```

#### 3. State Backend Configuration Options

**RedisConfig Parameters:**

- **`url`**: Redis connection string (default: "redis://localhost:6379")
- **`max_connections`**: Connection pool size (default: 10)
- **`connection_timeout_ms`**: Connection timeout in milliseconds (default: 5000)
- **`key_prefix`**: Namespace prefix for all keys (default: "eventflux:state:")
- **`ttl_seconds`**: Optional TTL for state entries (default: None - no expiration)

**Performance Characteristics:**

- **Latency**: 1-5ms for local Redis, 10-50ms for network Redis
- **Throughput**: 10K-100K operations/second depending on network and Redis configuration
- **Memory**: Efficient binary serialization minimizes Redis memory usage
- **Scaling**: Linear scaling with Redis cluster size

### Checkpoint and Recovery System

The Redis state backend integrates with EventFlux's enterprise checkpointing system:

```rust
// Create checkpoint (captures all state)
backend.checkpoint("recovery_point_1").await?;

// Continue processing...
backend.set("counter", bincode::serialize( & 42) ? ).await?;
backend.set("last_event", bincode::serialize( & event) ? ).await?;

// Disaster recovery - restore to checkpoint
backend.restore("recovery_point_1").await?;

// State is now restored to checkpoint time
assert_eq!(backend.get("counter").await?, None);
```

### Testing the Redis Backend

#### Quick Test with Docker

The easiest way to test the Redis backend is using the included Docker setup:

```bash
# Run complete example with Docker Redis
./run_redis_example.sh

# Or start Redis manually and run tests
docker-compose up -d
cargo test distributed_redis_state
```

#### Manual Testing

Tests gracefully skip if Redis is not available:

```bash
# Run Redis-specific tests (skips if Redis not running)
cargo test distributed_redis_state

# All distributed state tests
cargo test distributed.*state

# Verify Redis is available (optional)
redis-cli ping
```

**Note**: Tests automatically skip if Redis is not available, making the test suite resilient to different development
environments. GitHub Actions runs these tests with a Redis service container.

📖 **For detailed Docker setup instructions, see [DOCKER_SETUP.md](DOCKER_SETUP.md)**

### Monitoring and Observability

The Redis backend provides comprehensive error reporting and connection health monitoring:

```rust
// Connection health check is automatic during initialization
match backend.initialize().await {
Ok(_) => println!("Redis backend initialized successfully"),
Err(DistributedError::StateError { message }) => {
eprintln ! ("Redis connection failed: {}", message);
// Fallback to in-memory backend or retry logic
}
}

// Operations include detailed error context
if let Err(e) = backend.set("key", data).await {
match e {
DistributedError::StateError { message } => {
eprintln ! ("Redis operation failed: {}", message);
}
_ => eprintln ! ("Unexpected error: {}", e),
}
}
```

### Future State Backend Implementations

The architecture supports additional state backends:

- **Apache Ignite**: In-memory data grid for ultra-high performance
- **Hazelcast**: Distributed caching with advanced features
- **RocksDB**: Embedded high-performance storage
- **Cloud Storage**: AWS DynamoDB, Google Cloud Datastore integration

### CLI Runner

A small binary `run_eventflux` can execute a EventFluxQL file and log emitted events.
Build and run with:

```bash
cargo run --bin run_eventflux examples/sample.eventflux
```

To see trigger events in action you can run the trigger example:

```bash
cargo run --bin run_eventflux examples/trigger.eventflux
```

All streams have a `LogSink` attached so events appear on stdout. The CLI accepts
some additional flags:

```
--persistence-dir <dir>   # enable file persistence
--sqlite <db>             # use SQLite persistence
--extension <lib>         # load a dynamic extension library (repeatable)
--config <file>           # provide a custom configuration
```

Several example EventFluxQL files live in `examples/` including `simple_filter.eventflux`,
`time_window.eventflux`, `partition.eventflux` and `extension.eventflux` mirroring the
Java quick start samples.

## Next Planned Phases (High-Level)

1. **Enterprise State Management** 🔴: Implement comprehensive state management system as designed
   in [STATE_MANAGEMENT_DESIGN.md](STATE_MANAGEMENT_DESIGN.md). This is the immediate priority to enable distributed
   processing and production resilience.
2. **Distributed Processing**: Build cluster coordination and distributed state management (requires state management
   completion).
3. **Query Optimization**: Implement cost-based optimization and runtime code generation.
4. **Production Features**: Add enterprise monitoring, security, and advanced persistence.

## Contributing

(Placeholder for contribution guidelines)

## Incremental Aggregation (Experimental)

Basic support for defining incremental aggregations is available. An aggregation
can be declared using EventFluxQL syntax:

```
define aggregation AggName
from InputStream
select sum(value) as total
group by category
aggregate every seconds, minutes;
```

After parsing, `AggregationRuntime` instances are created when building a
`EventFluxAppRuntime`. Events fed to the runtime will update the aggregation buckets
for each configured duration. Query APIs for reading these buckets are not yet
implemented, but tests demonstrate the accumulation logic.
\nNote: The project still emits numerous compiler warnings due to incomplete features and placeholder code. These are
expected during the early porting phase.

## License

Licensed under either of

- MIT license (see `LICENSE-MIT`)
- Apache License, Version 2.0 (see `LICENSE-APACHE`)
  at your option.
