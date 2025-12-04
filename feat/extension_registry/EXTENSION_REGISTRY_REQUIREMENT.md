# Extension Registry System Requirement

**Status**: ✅ IMPLEMENTED
**Priority**: P1 (Architectural Foundation)
**Date**: 2025-11-27
**Updated**: 2025-11-29

## Problem Statement

The current EventFlux codebase has **hardcoded switches** scattered throughout for selecting extensions:

```rust
// CURRENT: Hardcoded, not extensible
match func_name {
    "sum" => Box::new(SumAttributeAggregatorExecutor::default()),
    "avg" => Box::new(AvgAttributeAggregatorExecutor::default()),
    "count" => Box::new(CountAttributeAggregatorExecutor::default()),
    // Adding new aggregator = modify this file
    _ => return Err("Unknown aggregator")
}

match window_name {
    "length" => create_length_window(...),
    "time" => create_time_window(...),
    // Adding new window = modify this file
    _ => return Err("Unknown window")
}
```

**Problems with this approach:**

1. **Not Extensible**: Adding any new extension requires modifying core code
2. **Scattered Logic**: Extension selection logic spread across multiple files
3. **No Dynamic Registration**: Cannot add extensions at runtime
4. **Testing Difficulty**: Hard to mock/stub extensions for testing
5. **Code Coupling**: Core compiler/runtime tightly coupled to specific implementations
6. **Violates Open/Closed Principle**: Must modify existing code to extend functionality

## Affected Extension Types

| Extension Type | Current Location | Example |
|---------------|------------------|---------|
| Window Aggregators | `aggregator/mod.rs` | sum, avg, count, min, max |
| Collection Aggregators | TBD | sum(e1.price), avg(e1.price) |
| Window Processors | `window/mod.rs` | length, time, batch, session |
| Functions | `function/mod.rs` | convert, coalesce, ifThenElse |
| Sources | `source/mod.rs` | kafka, http, file |
| Sinks | `sink/mod.rs` | kafka, http, log |
| Stream Processors | Various | filter, passThrough |
| Table Types | `table/mod.rs` | inMemory, redis, jdbc |

## Proposed Solution: Centralized Extension Registry

### Core Design

```rust
/// Central registry for all extension types
pub struct ExtensionRegistry {
    // Aggregators (for windows)
    window_aggregators: HashMap<String, Arc<dyn AttributeAggregatorFactory>>,

    // Collection aggregation functions (for patterns)
    collection_aggregations: HashMap<String, Arc<dyn CollectionAggregationFunction>>,

    // Window processors
    windows: HashMap<String, Arc<dyn WindowProcessorFactory>>,

    // Scalar functions
    functions: HashMap<String, Arc<dyn FunctionExecutorFactory>>,

    // Sources and Sinks
    sources: HashMap<String, Arc<dyn SourceFactory>>,
    sinks: HashMap<String, Arc<dyn SinkFactory>>,

    // Tables
    tables: HashMap<String, Arc<dyn TableFactory>>,

    // Stream processors
    stream_processors: HashMap<String, Arc<dyn StreamProcessorFactory>>,
}
```

### Registration API

```rust
impl ExtensionRegistry {
    /// Create with built-in extensions pre-registered
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register_builtins();
        registry
    }

    /// Register a window aggregator
    pub fn register_aggregator<F: AttributeAggregatorFactory + 'static>(&mut self, factory: F) {
        self.window_aggregators.insert(factory.name().to_string(), Arc::new(factory));
    }

    /// Register a collection aggregation function
    pub fn register_collection_aggregation<F: CollectionAggregationFunction + 'static>(&mut self, func: F) {
        self.collection_aggregations.insert(func.name().to_string(), Arc::new(func));
    }

    /// Register a window processor
    pub fn register_window<F: WindowProcessorFactory + 'static>(&mut self, factory: F) {
        self.windows.insert(factory.name().to_string(), Arc::new(factory));
    }

    /// Register a scalar function
    pub fn register_function<F: FunctionExecutorFactory + 'static>(&mut self, factory: F) {
        self.functions.insert(factory.name().to_string(), Arc::new(factory));
    }

    // ... similar for sources, sinks, tables, etc.
}
```

### Lookup API

```rust
impl ExtensionRegistry {
    /// Get aggregator factory by name (case-insensitive)
    pub fn get_aggregator(&self, name: &str) -> Option<Arc<dyn AttributeAggregatorFactory>> {
        self.window_aggregators.get(&name.to_lowercase()).cloned()
    }

    /// Get collection aggregation function by name
    pub fn get_collection_aggregation(&self, name: &str) -> Option<Arc<dyn CollectionAggregationFunction>> {
        self.collection_aggregations.get(&name.to_lowercase()).cloned()
    }

    /// Get window processor factory by name
    pub fn get_window(&self, name: &str) -> Option<Arc<dyn WindowProcessorFactory>> {
        self.windows.get(&name.to_lowercase()).cloned()
    }

    /// List all registered extensions of a type
    pub fn list_aggregators(&self) -> Vec<&str> {
        self.window_aggregators.keys().map(|s| s.as_str()).collect()
    }
}
```

### Built-in Registration

```rust
impl ExtensionRegistry {
    fn register_builtins(&mut self) {
        // Window Aggregators
        self.register_aggregator(SumAttributeAggregatorFactory);
        self.register_aggregator(AvgAttributeAggregatorFactory);
        self.register_aggregator(CountAttributeAggregatorFactory);
        self.register_aggregator(MinAttributeAggregatorFactory);
        self.register_aggregator(MaxAttributeAggregatorFactory);
        self.register_aggregator(DistinctCountAttributeAggregatorFactory);

        // Collection Aggregations (share logic with window aggregators where possible)
        self.register_collection_aggregation(SumFunction);
        self.register_collection_aggregation(AvgFunction);
        self.register_collection_aggregation(CountFunction);
        self.register_collection_aggregation(MinFunction);
        self.register_collection_aggregation(MaxFunction);
        self.register_collection_aggregation(StdDevFunction);

        // Windows
        self.register_window(LengthWindowFactory);
        self.register_window(TimeWindowFactory);
        self.register_window(LengthBatchWindowFactory);
        self.register_window(TimeBatchWindowFactory);
        self.register_window(SessionWindowFactory);
        // ...

        // Functions
        self.register_function(ConvertFunctionFactory);
        self.register_function(CoalesceFunctionFactory);
        self.register_function(IfThenElseFunctionFactory);
        // ...
    }
}
```

### Usage in Compiler (No More Hardcoded Switches!)

```rust
// BEFORE (bad):
fn create_aggregator(name: &str) -> Result<Box<dyn AttributeAggregatorExecutor>> {
    match name {
        "sum" => Ok(Box::new(SumAttributeAggregatorExecutor::default())),
        "avg" => Ok(Box::new(AvgAttributeAggregatorExecutor::default())),
        _ => Err(format!("Unknown aggregator: {}", name))
    }
}

// AFTER (good):
fn create_aggregator(name: &str, registry: &ExtensionRegistry) -> Result<Box<dyn AttributeAggregatorExecutor>> {
    registry.get_aggregator(name)
        .map(|factory| factory.create())
        .ok_or_else(|| format!("Unknown aggregator: {}. Available: {:?}", name, registry.list_aggregators()))
}
```

### Collection Aggregation with Registry

```rust
// BEFORE (bad):
fn create_collection_aggregation(func: &str, chain_idx: usize, attr_pos: [i32; 2])
    -> Result<Box<dyn ExpressionExecutor>>
{
    match func {
        "sum" => Ok(Box::new(CollectionSumExecutor::new(chain_idx, attr_pos, ...))),
        "avg" => Ok(Box::new(CollectionAvgExecutor::new(chain_idx, attr_pos))),
        _ => Err("Unknown")
    }
}

// AFTER (good):
fn create_collection_aggregation(
    func: &str,
    chain_idx: usize,
    attr_pos: [i32; 2],
    registry: &ExtensionRegistry
) -> Result<Box<dyn ExpressionExecutor>> {
    let agg_fn = registry.get_collection_aggregation(func)
        .ok_or_else(|| format!("Unknown collection aggregation: {}", func))?;

    Ok(Box::new(CollectionAggregator::new(chain_idx, Some(attr_pos), agg_fn)))
}
```

## Factory Trait Definitions

### Aggregator Factory

```rust
pub trait AttributeAggregatorFactory: Send + Sync + Debug {
    /// Unique name for this aggregator (e.g., "sum", "avg")
    fn name(&self) -> &str;

    /// Create a new instance of the aggregator
    fn create(&self) -> Box<dyn AttributeAggregatorExecutor>;

    /// Clone the factory (for registry copying)
    fn clone_box(&self) -> Box<dyn AttributeAggregatorFactory>;

    /// Description for documentation/help
    fn description(&self) -> &str { "" }

    /// Parameter specifications for validation
    fn parameters(&self) -> &[ParameterSpec] { &[] }
}
```

### Collection Aggregation Function

```rust
pub trait CollectionAggregationFunction: Send + Sync + Debug {
    /// Unique name (e.g., "sum", "avg")
    fn name(&self) -> &str;

    /// Aggregate over numeric values
    fn aggregate(&self, values: &[f64]) -> Option<f64>;

    /// Whether this is a count-only function (no attribute needed)
    fn supports_count_only(&self) -> bool { false }

    /// Determine return type based on input type
    fn return_type(&self, input_type: ApiAttributeType) -> ApiAttributeType;

    /// Clone for registry
    fn clone_box(&self) -> Box<dyn CollectionAggregationFunction>;
}
```

### Window Processor Factory

```rust
pub trait WindowProcessorFactory: Send + Sync + Debug {
    /// Unique name (e.g., "length", "time", "session")
    fn name(&self) -> &str;

    /// Create window processor from parsed parameters
    fn create(
        &self,
        params: &[AttributeValue],
        app_ctx: &Arc<EventFluxAppContext>,
        query_ctx: &Arc<EventFluxQueryContext>,
    ) -> Result<Arc<Mutex<dyn WindowProcessor>>, String>;

    /// Parameter specifications for validation
    fn parameters(&self) -> &[ParameterSpec];

    fn clone_box(&self) -> Box<dyn WindowProcessorFactory>;
}
```

### Function Executor Factory

```rust
pub trait FunctionExecutorFactory: Send + Sync + Debug {
    /// Unique name (e.g., "convert", "coalesce")
    fn name(&self) -> &str;

    /// Create function executor
    fn create(
        &self,
        args: Vec<Box<dyn ExpressionExecutor>>,
        ctx: &EventFluxQueryContext,
    ) -> Result<Box<dyn ExpressionExecutor>, String>;

    /// Parameter specifications
    fn parameters(&self) -> &[ParameterSpec];

    fn clone_box(&self) -> Box<dyn FunctionExecutorFactory>;
}
```

## Integration with EventFluxManager

```rust
impl EventFluxManager {
    pub fn new() -> Self {
        Self {
            extension_registry: ExtensionRegistry::with_builtins(),
            // ...
        }
    }

    /// Register custom extension
    pub fn register_extension<E: Extension>(&mut self, extension: E) {
        extension.register(&mut self.extension_registry);
    }

    /// Get registry for query compilation
    pub fn extension_registry(&self) -> &ExtensionRegistry {
        &self.extension_registry
    }
}

// User code:
let mut manager = EventFluxManager::new();
manager.register_extension(MyCustomAggregator);
manager.register_extension(MyCustomWindow);
```

## Benefits

1. **Open/Closed Principle**: Add extensions without modifying core code
2. **Centralized Discovery**: One place to find all available extensions
3. **Runtime Extensibility**: Register extensions dynamically
4. **Better Error Messages**: "Unknown aggregator 'foo'. Available: [sum, avg, count, min, max]"
5. **Testing**: Easy to mock registry with specific extensions
6. **Documentation**: Registry can generate docs for available extensions
7. **Validation**: Centralized parameter validation per extension type
8. **Namespacing**: Support for `namespace:function` syntax (e.g., `math:sin`)

## Implementation Plan

### Phase 1: Core Registry (2-3 days)
1. Create `ExtensionRegistry` struct
2. Define factory traits for each extension type
3. Implement registration and lookup methods
4. Add to `EventFluxManager`

### Phase 2: Migrate Aggregators (1-2 days)
1. Refactor window aggregators to use registry
2. Implement `CollectionAggregationFunction` trait
3. Create single `CollectionAggregator` executor
4. Register all built-in aggregators

### Phase 3: Migrate Windows (1-2 days)
1. Create `WindowProcessorFactory` implementations
2. Register all built-in windows
3. Update window creation code to use registry

### Phase 4: Migrate Functions (1-2 days)
1. Create `FunctionExecutorFactory` implementations
2. Register all built-in functions
3. Update function creation code

### Phase 5: Migrate Sources/Sinks (2-3 days)
1. Create factory traits for sources/sinks
2. Register built-ins
3. Update creation code

### Phase 6: Documentation & Testing (1-2 days)
1. Add registry introspection for help/docs
2. Comprehensive testing
3. Update CLAUDE.md with extension patterns

**Total Effort**: 8-14 days

## Example: Adding a New Aggregation

**Before (requires modifying core code):**
```rust
// Must modify aggregator/mod.rs
// Must modify compiler switch statement
// Must add to factory registration
// Multiple files changed
```

**After (single file, no core changes):**
```rust
// my_extension.rs
#[derive(Debug, Clone)]
pub struct MedianAggregatorFactory;

impl AttributeAggregatorFactory for MedianAggregatorFactory {
    fn name(&self) -> &str { "median" }
    fn create(&self) -> Box<dyn AttributeAggregatorExecutor> {
        Box::new(MedianAggregatorExecutor::default())
    }
    fn clone_box(&self) -> Box<dyn AttributeAggregatorFactory> {
        Box::new(self.clone())
    }
}

// Also works for collection aggregations automatically:
#[derive(Debug, Clone)]
pub struct MedianFunction;

impl CollectionAggregationFunction for MedianFunction {
    fn name(&self) -> &str { "median" }
    fn aggregate(&self, values: &[f64]) -> Option<f64> {
        if values.is_empty() { return None; }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            Some((sorted[mid - 1] + sorted[mid]) / 2.0)
        } else {
            Some(sorted[mid])
        }
    }
    fn return_type(&self, _: ApiAttributeType) -> ApiAttributeType {
        ApiAttributeType::DOUBLE
    }
    fn clone_box(&self) -> Box<dyn CollectionAggregationFunction> {
        Box::new(self.clone())
    }
}

// Registration (one line):
manager.extension_registry_mut().register_aggregator(MedianAggregatorFactory);
manager.extension_registry_mut().register_collection_aggregation(MedianFunction);
```

## Automatic Registration Alternatives Considered

### Rust Does Not Have Runtime Reflection

Unlike Java or C#, Rust is a statically compiled language **without runtime type information (RTTI)**. There is no built-in mechanism to automatically discover all types implementing a trait at runtime.

### Alternative Approaches Evaluated

#### 1. `inventory` Crate

Uses linker section tricks to collect items at compile time:

```rust
use inventory;

// Define a registry
inventory::collect!(Box<dyn WindowProcessorFactory>);

// Each factory registers itself (can be in separate files/crates)
inventory::submit! {
    Box::new(LengthWindowFactory) as Box<dyn WindowProcessorFactory>
}

// At runtime, iterate all registered factories
for factory in inventory::iter::<Box<dyn WindowProcessorFactory>> {
    ctx.add_window_factory(factory.name().to_string(), factory.clone_box());
}
```

#### 2. `linkme` Crate

Similar approach using distributed slices:

```rust
use linkme::distributed_slice;

#[distributed_slice]
pub static WINDOW_FACTORIES: [fn() -> Box<dyn WindowProcessorFactory>] = [..];

#[distributed_slice(WINDOW_FACTORIES)]
fn length_factory() -> Box<dyn WindowProcessorFactory> {
    Box::new(LengthWindowFactory)
}
```

#### 3. Procedural Macros

Custom derive macros to generate registration code:

```rust
#[register_extension]  // Custom derive macro
pub struct LengthWindowFactory;
```

### Platform Compatibility

| Platform | `inventory` | `linkme` | Manual |
|----------|-------------|----------|--------|
| Linux | ✅ | ✅ | ✅ |
| macOS | ✅ | ✅ | ✅ |
| Windows | ✅ | ✅ | ✅ |
| FreeBSD | ✅ | ✅ | ✅ |
| Docker | ✅ | ✅ | ✅ |
| iOS/Android | ⚠️ Limited | ✅ | ✅ |
| **WASM** | ❌ **No** | ❌ **No** | ✅ |

Both `inventory` and `linkme` rely on **linker section tricks** that WebAssembly does not support.

### Decision: Manual Registration

**We chose manual registration** for the following reasons:

1. **WASM Compatibility**: EventFlux may target browser-based CEP or edge computing scenarios where WASM is required. Neither `inventory` nor `linkme` work on WASM.

2. **Portability**: Manual registration works on every platform Rust compiles to, with no platform-specific surprises.

3. **Explicitness**: No "magic" - developers can clearly see what gets registered and when.

4. **Simplicity**: No additional dependencies or complex linker configurations.

5. **Debuggability**: Registration flow is straightforward to trace and debug.

### Current Implementation

Built-in extensions are registered in `EventFluxContext::register_default_extensions()`:

```rust
fn register_default_extensions(&mut self) {
    // Windows
    self.add_window_factory("length".to_string(), Box::new(LengthWindowFactory));
    self.add_window_factory("time".to_string(), Box::new(TimeWindowFactory));
    // ...

    // Aggregators
    self.add_attribute_aggregator_factory("sum".to_string(), Box::new(SumAttributeAggregatorFactory));
    // ...

    // Collection aggregation functions
    self.add_collection_aggregation_function("sum".to_string(), Box::new(CollectionSumFunction));
    // ...
}
```

Custom extensions can be registered via `EventFluxContext` methods:

```rust
let manager = EventFluxManager::new();
manager.get_context().add_window_factory("myWindow".to_string(), Box::new(MyWindowFactory));
```

### Future Consideration

If WASM support is not required, we could revisit `inventory` or `linkme` for automatic discovery. This would require:

```rust
#[cfg(not(target_arch = "wasm32"))]
use inventory;

#[cfg(target_arch = "wasm32")]
// Fall back to manual registration
```

However, the added complexity is not justified at this time.

---

## Related Work

This pattern is common in extensible systems:
- **Apache Flink**: `FunctionDefinition` registry
- **Apache Spark**: `FunctionRegistry` for UDFs
- **Trino/Presto**: Plugin-based function registration
- **PostgreSQL**: Extension system with `CREATE EXTENSION`

## Conclusion

The Extension Registry is a **foundational architectural improvement** that:
1. Eliminates hardcoded switches throughout the codebase
2. Enables true extensibility without core code changes
3. Provides better error messages and discoverability
4. Unifies the pattern for all extension types
5. Simplifies testing and mocking

## Implementation Status

✅ **Completed on 2025-11-29**

| Component | Status | Notes |
|-----------|--------|-------|
| `CollectionAggregationFunction` trait | ✅ Done | `src/core/extension/mod.rs` |
| Built-in collection functions | ✅ Done | count, sum, avg, min, max, stdDev |
| Collection aggregation registry | ✅ Done | `EventFluxContext` |
| Remove hardcoded aggregators | ✅ Done | `expression_parser.rs` refactored |
| Remove hardcoded windows | ✅ Done | `window/mod.rs` refactored |
| Register session/sort windows | ✅ Done | Previously missing |
| Registry introspection methods | ✅ Done | `list_*_names()` methods |
| Tests | ✅ Done | 1182 tests passing |

---

**Document Version**: 2.0
**Created**: 2025-11-27
**Updated**: 2025-11-29
**Author**: Architecture Review
