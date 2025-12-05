# Implementation Guides

**Last Updated**: 2025-10-02
**Implementation Status**: Complete - Comprehensive Reference
**Related Code**: `src/core/query/processor/`, `src/core/executor/`, `src/core/query/selector/`

---

## Overview

Developer guides and patterns for implementing new features and components in EventFlux Rust. This comprehensive reference covers all major component types with Java-to-Rust translation patterns.

**Key Topics**:
- Window processors and stream functions
- Aggregator executors
- Sources and sinks
- Table implementations
- Extension loading and factory registration
- Performance optimization and testing strategies

---

## Quick Reference

### Component Types

#### 1. Window Processors
**Purpose**: Windowed aggregation and batching

**Implementation Steps**:
1. Create struct implementing `WindowProcessor` trait
2. Implement `process()` for event handling
3. Add `StateHolder` for persistence
4. Register in factory (`window_processor_factory.rs`)
5. Write comprehensive tests

**Template**:
```rust
pub struct MyWindowProcessor {
    meta: CommonProcessorMeta,
    window_size: usize,
    buffer: VecDeque<Arc<StreamEvent>>,
}

impl WindowProcessor for MyWindowProcessor {
    fn process(&mut self, event: Arc<StreamEvent>) -> Vec<Arc<StreamEvent>> {
        // Process event and return output
    }
}

impl StateHolder for MyWindowProcessor {
    fn serialize_state(&self) -> Result<Vec<u8>, StateError> {
        // Serialize window buffer
    }
}
```

**Example**: `src/core/query/processor/stream/window/length_window_processor.rs`

---

#### 2. Stream Functions
**Purpose**: Transform or filter individual events

**Implementation Steps**:
1. Create struct implementing `StreamFunctionProcessor` trait
2. Implement `process()` for transformation
3. Register in factory
4. Add tests for edge cases

**Template**:
```rust
pub struct MyFunctionProcessor {
    meta: CommonProcessorMeta,
    param: String,
}

impl StreamFunctionProcessor for MyFunctionProcessor {
    fn process(&mut self, event: Arc<StreamEvent>) -> Vec<Arc<StreamEvent>> {
        // Transform event
        vec![transformed_event]
    }
}
```

**Example**: `src/core/query/processor/stream/function/log_stream_processor.rs`

---

#### 3. Aggregator Executors
**Purpose**: Incremental aggregation (sum, avg, count, etc.)

**Implementation Steps**:
1. Create struct implementing `Aggregator` trait
2. Implement `process()`, `add()`, `remove()`
3. Add `StateHolder` for checkpointing
4. Register in aggregator factory
5. Test incremental behavior

**Template**:
```rust
pub struct MyAggregator {
    accumulator: f64,
    count: usize,
}

impl Aggregator for MyAggregator {
    fn process(&mut self, value: &AttributeValue) -> AttributeValue {
        self.accumulator += value.as_f64();
        self.count += 1;
        AttributeValue::Double(self.accumulator / self.count as f64)
    }

    fn add(&mut self, value: &AttributeValue) {
        // Incremental add
    }

    fn remove(&mut self, value: &AttributeValue) {
        // Incremental remove
    }
}
```

**Example**: `src/core/query/selector/attribute/aggregator/avg_aggregator_executor.rs`

---

#### 4. Sources
**Purpose**: Ingest events from external systems

**Implementation Steps**:
1. Implement `Source` trait
2. Handle connection lifecycle
3. Error handling and retries
4. Register in source factory
5. Add integration tests

**Template**:
```rust
pub struct MySource {
    url: String,
    client: HttpClient,
}

impl Source for MySource {
    fn start(&mut self, callback: SourceCallback) -> Result<(), IOError> {
        // Start ingestion
        loop {
            let event = self.fetch_event()?;
            callback.on_event(event);
        }
    }

    fn stop(&mut self) -> Result<(), IOError> {
        // Cleanup
    }
}
```

**Example**: `src/core/stream/input/source/timer_source.rs`

---

#### 5. Sinks
**Purpose**: Send events to external systems

**Implementation Steps**:
1. Implement `Sink` trait
2. Handle batching and retries
3. Connection pooling
4. Register in sink factory
5. Test failure scenarios

**Template**:
```rust
pub struct MySink {
    url: String,
    client: HttpClient,
}

impl Sink for MySink {
    fn publish(&mut self, event: Arc<Event>) -> Result<(), IOError> {
        // Send event
        self.client.post(&self.url, event)?;
        Ok(())
    }
}
```

**Example**: `src/core/stream/output/sink/log_sink.rs`

---

#### 6. Tables
**Purpose**: Queryable in-memory or persistent data stores

**Implementation Steps**:
1. Implement `Table` trait
2. Add CRUD operations
3. Implement indexes
4. StateHolder for persistence
5. Test concurrency

**Template**:
```rust
pub struct MyTable {
    data: HashMap<String, Event>,
    indexes: HashMap<String, BTreeMap<String, String>>,
}

impl Table for MyTable {
    fn add(&mut self, event: Event) -> Result<(), StateError> {
        // Insert with index updates
    }

    fn find(&self, condition: CompiledCondition) -> Vec<Event> {
        // Query with index optimization
    }
}
```

**Example**: `src/core/table/in_memory_table.rs`

---

## Implementation Patterns

### Pattern 1: Java to Rust Translation

**Java Pattern**:
```java
public class LengthWindowProcessor extends WindowProcessor {
    private int length;
    private Queue<StreamEvent> events = new LinkedList<>();

    @Override
    protected void process(StreamEvent event) {
        events.add(event);
        if (events.size() > length) {
            StreamEvent expired = events.poll();
            // emit expired
        }
    }
}
```

**Rust Translation**:
```rust
pub struct LengthWindowProcessor {
    meta: CommonProcessorMeta,
    length: usize,
    events: VecDeque<Arc<StreamEvent>>,
}

impl WindowProcessor for LengthWindowProcessor {
    fn process(&mut self, event: Arc<StreamEvent>) -> Vec<Arc<StreamEvent>> {
        self.events.push_back(Arc::clone(&event));
        let mut expired = Vec::new();
        while self.events.len() > self.length {
            if let Some(exp) = self.events.pop_front() {
                expired.push(exp);
            }
        }
        expired
    }
}
```

**Key Changes**:
- `Queue<T>` → `VecDeque<Arc<T>>` (shared ownership)
- `null` → `Option<T>` (explicit null handling)
- Exceptions → `Result<T, E>` (explicit error handling)
- Inheritance → Traits (composition over inheritance)

---

### Pattern 2: StateHolder Integration

**Every stateful component needs**:
```rust
impl StateHolder for MyComponent {
    fn component_id(&self) -> String {
        "my_component".to_string()
    }

    fn schema_version(&self) -> semver::Version {
        semver::Version::new(1, 0, 0)
    }

    fn serialize_state(&self) -> Result<Vec<u8>, StateError> {
        bincode::serialize(&self.state)
            .map_err(|e| StateError::SerializationError(e.to_string()))
    }

    fn deserialize_state(&mut self, data: &[u8]) -> Result<(), StateError> {
        self.state = bincode::deserialize(data)
            .map_err(|e| StateError::DeserializationError(e.to_string()))?;
        Ok(())
    }
}

// Get automatic compression
impl CompressibleStateHolder for MyComponent {}
```

**Benefits**:
- Automatic checkpointing
- 90-95% compression
- Schema versioning
- Point-in-time recovery

---

### Pattern 3: Factory Registration

**Window Factory**:
```rust
// In window_processor_factory.rs
pub fn create_window_processor(
    name: &str,
    handler: FunctionWindowHandler,
) -> Result<Arc<Mutex<dyn WindowProcessor>>, String> {
    match name {
        "length" => Ok(Arc::new(Mutex::new(
            LengthWindowProcessor::from_handler(handler)?
        ))),
        "myWindow" => Ok(Arc::new(Mutex::new(
            MyWindowProcessor::from_handler(handler)?
        ))),
        _ => Err(format!("Unknown window: {}", name)),
    }
}
```

**Source Factory**:
```rust
// In source_factory.rs
pub fn create_source(
    source_type: &str,
    config: SourceConfig,
) -> Result<Box<dyn Source>, IOError> {
    match source_type {
        "timer" => Ok(Box::new(TimerSource::new(config)?)),
        "mySource" => Ok(Box::new(MySource::new(config)?)),
        _ => Err(IOError::UnknownType(source_type.to_string())),
    }
}
```

---

### Pattern 4: Error Handling

**Use specific error types**:
```rust
pub fn process() -> Result<Output, ProcessorError> {
    let state = load_state()
        .map_err(|e| ProcessorError::StateError(e))?;

    let result = compute(&state)
        .map_err(|e| ProcessorError::ComputationError {
            reason: e.to_string(),
        })?;

    Ok(result)
}
```

**Provide context**:
```rust
.map_err(|e| StateError::CompressionError {
    algorithm: "zstd".to_string(),
    reason: format!("Failed to compress {} bytes: {}", data.len(), e),
})?
```

---

### Pattern 5: Testing Strategy

**Unit Tests**:
```rust
#[test]
fn test_window_basic() {
    let mut window = LengthWindowProcessor::new(3);
    let e1 = create_event(1);
    let e2 = create_event(2);

    let expired = window.process(e1);
    assert_eq!(expired.len(), 0);

    // Add more events...
}
```

**Integration Tests** (use AppRunner):
```rust
#[test]
fn test_window_integration() {
    let app = "@app:name('Test')
        define stream In (id string, val double);
        from In#window.length(3)
        select id, val
        insert into Out;";

    let runner = AppRunner::new(app, "Out");
    runner.send("In", vec![("1", 10.0), ("2", 20.0)]);
    let results = runner.shutdown();
    assert_eq!(results.len(), 2);
}
```

**State Tests**:
```rust
#[test]
fn test_state_persistence() {
    let window = create_window();
    // Process events
    let state = window.serialize_state().unwrap();

    let mut restored = create_window();
    restored.deserialize_state(&state).unwrap();
    // Verify state matches
}
```

---

## Grammar Extension Patterns

### Adding New Annotation Support

#### @Async Annotation Implementation Pattern

The @Async annotation implementation demonstrates the complete pattern for adding annotation support to EventFlux Rust. This includes grammar rules, parser integration, and runtime configuration.

**Grammar Pattern for Minimal and Parameterized Annotations (legacy LALRPOP illustration; current parser uses `sql_compiler`/sqlparser-rs):**
```lalrpop
// Support both minimal and parameterized annotations
pub AnnotationStmt: Annotation = {
    "@" <name:Ident> "(" <pairs:KeyValuePairs> ")" => {
        let mut a = Annotation::new(name);
        for (k, v) in pairs { a = a.element(Some(k), v); }
        a
    },
    "@" <name:Ident> ":" <key:Ident> "(" <val:STRING> ")" => {
        Annotation::new(name).element(Some(key), val)
    },
    "@" <name:Ident> => {
        Annotation::new(name)  // Minimal annotation support (e.g., @Async)
    }
};

// Use underscore notation to avoid grammar conflicts
KeyValue: (String, String) = { <k:Ident> "=" <v:STRING> => (k, v) };
```

**Parser Integration Pattern:**
```rust
// In eventflux_app_parser.rs - Complete @Async annotation processing
for ann in &stream_def_arc.abstract_definition.annotations {
    match ann.name.to_lowercase().as_str() {
        "async" => {
            use_optimized = true;
            config = config.with_async(true);

            // Parse @Async annotation parameters with underscore notation
            for el in &ann.elements {
                match el.key.to_lowercase().as_str() {
                    "buffer_size" | "buffersize" => {
                        if let Ok(sz) = el.value.parse::<usize>() {
                            config = config.with_buffer_size(sz);
                        }
                    }
                    "workers" => {
                        if let Ok(workers) = el.value.parse::<u64>() {
                            let estimated_throughput = workers * 10000;
                            config = config.with_expected_throughput(estimated_throughput);
                        }
                    }
                    "batch_size_max" | "batchsizemax" => {
                        // Compatibility with Java EventFlux
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}
```

**Testing Pattern for Grammar Extensions:**
```rust
#[test]
fn test_async_annotation_with_parameters() {
    let mut manager = EventFluxManager::new();
    let eventflux_app_string = r#"
        @Async(buffer_size='1024', workers='2', batch_size_max='10')
        define stream TestStream (id int, value string);
    "#;

    let result = manager.create_eventflux_app_runtime_from_string(eventflux_app_string);
    assert!(result.is_ok());
}
```

---

## Performance Optimization

### 1. Use Arc<T> for Shared Data
```rust
// Good - shared ownership
let event = Arc::new(Event::new());
processor.process(Arc::clone(&event));

// Avoid - unnecessary cloning
let event = Event::new();
processor.process(event.clone());
```

### 2. Pre-allocate Collections
```rust
// Good - pre-allocated
let mut buffer = Vec::with_capacity(1000);

// Avoid - grows dynamically
let mut buffer = Vec::new();
```

### 3. Use Object Pools
```rust
// Good - reuse from pool
let event = event_pool.acquire();

// Avoid - new allocation
let event = Event::new();
```

### 4. Minimize Lock Contention
```rust
// Good - short critical section
{
    let data = state.lock().unwrap();
    let value = data.get_value();
    // Release lock
}
process(value);

// Avoid - long critical section
let mut data = state.lock().unwrap();
process(&mut data);  // Holds lock during processing
```

---

## Best Practices

### 1. Follow Rust Idioms
- Use `Option<T>` instead of null
- Use `Result<T, E>` instead of exceptions
- Prefer traits over inheritance
- Use lifetime annotations correctly

### 2. Maintain Type Safety
- Avoid `unwrap()` in production code
- Use proper error types
- Validate inputs at boundaries

### 3. Document Public APIs
```rust
/// Creates a new length-based window processor.
///
/// # Arguments
/// * `length` - Maximum number of events in window
///
/// # Examples
/// ```
/// let window = LengthWindowProcessor::new(100);
/// ```
pub fn new(length: usize) -> Self { }
```

### 4. Test Edge Cases
- Empty inputs
- Null/None values
- Boundary conditions
- Error scenarios
- Concurrent access

### 5. Profile Before Optimizing
```bash
cargo build --release
perf record --call-graph=dwarf ./target/release/benchmark
perf report
```

---

## Common Pitfalls

### 1. Deadlocks
```rust
// BAD - potential deadlock
let lock1 = self.state.lock().unwrap();
let lock2 = self.other.lock().unwrap();

// GOOD - use try_lock or specific ordering
let lock1 = self.state.try_lock()
    .map_err(|_| StateError::LockError("Failed to acquire lock".to_string()))?;
```

### 2. Memory Leaks
```rust
// BAD - circular references
struct Node {
    next: Arc<Mutex<Node>>,
    prev: Arc<Mutex<Node>>,  // Cycle!
}

// GOOD - use Weak for back-references
struct Node {
    next: Arc<Mutex<Node>>,
    prev: Weak<Mutex<Node>>,
}
```

### 3. Unnecessary Clones
```rust
// BAD - clone when not needed
fn process(data: Vec<u8>) -> Vec<u8> {
    data.clone()  // Unnecessary
}

// GOOD - consume and return
fn process(data: Vec<u8>) -> Vec<u8> {
    data  // Move ownership
}
```

---

## Getting Help

### Resources
1. **Implementation Guide**: This document
2. **ROADMAP.md**: Feature priorities and technical details
3. **CLAUDE.md**: Architecture and development guidelines
4. **Feature Docs**: `feat/*/` for specific features

### Example Code
- `src/core/query/processor/stream/window/` - Window examples
- `src/core/query/selector/attribute/aggregator/` - Aggregator examples
- `src/core/stream/input/source/` - Source examples
- `src/core/stream/output/sink/` - Sink examples

### Testing
```bash
# Run specific component tests
cargo test window
cargo test aggregator
cargo test source

# Run with output
cargo test -- --nocapture

# Run benchmarks
cargo bench
```

---

## Contributing

When implementing new features:
1. Read this guide and relevant examples
2. Follow established patterns
3. Implement StateHolder for stateful components
4. Write comprehensive tests
5. Document public APIs
6. Profile performance if critical path
7. Update relevant documentation

---

**Status**: Complete - Comprehensive implementation reference for all major component types
