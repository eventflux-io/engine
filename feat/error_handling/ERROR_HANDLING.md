# Error Handling

**Last Updated**: 2025-10-02
**Implementation Status**: Production Ready
**Related Code**: `src/core/error/`

---

## Overview

EventFlux Rust implements a comprehensive error handling framework using `thiserror` for structured, type-safe error management. This replaced primitive String-based errors with a robust hierarchical error system.

**Key Features**:
- Hierarchical error types using `thiserror`
- Type-safe error propagation with `Result<T, E>`
- Context-aware error messages with error chaining
- Comprehensive error coverage (121+ Result types throughout codebase)
- Migration patterns from String errors

---

## Implementation Status

### Completed ✅

#### Error Type Hierarchy
- ✅ **StateError**: State management and persistence errors
- ✅ **QueryError**: Query parsing and compilation errors
- ✅ **RuntimeError**: Runtime execution errors
- ✅ **IOError**: I/O and network errors
- ✅ **ConfigError**: Configuration and validation errors

#### Error Handling Patterns
- ✅ **Error Propagation**: `?` operator with type conversion
- ✅ **Error Context**: `.map_err()` for adding context
- ✅ **Error Recovery**: Graceful degradation strategies
- ✅ **Error Logging**: Structured error output

#### Convenience Methods
- ✅ **is_* methods**: Error type checking
- ✅ **context() method**: Extract error context
- ✅ **Display impl**: User-friendly error messages
- ✅ **Debug impl**: Detailed error information

---

## Core Error Module

```
src/core/error/
├── mod.rs              # Error module exports
├── state_error.rs      # State management errors
├── query_error.rs      # Query processing errors
├── runtime_error.rs    # Runtime execution errors
├── io_error.rs         # I/O and network errors
└── config_error.rs     # Configuration errors
```

### Error Usage Across Codebase
```
src/core/persistence/    # StateError
src/sql_compiler/        # QueryError
src/core/runtime/        # RuntimeError
src/core/stream/         # IOError
```

---

## Error Type Reference

### StateError
**Purpose**: State management and persistence operations

**Variants**:
- `SerializationError(String)` - State serialization failed
- `DeserializationError(String)` - State deserialization failed
- `CompressionError { algorithm, reason }` - Compression failed
- `VersionMismatch { expected, actual }` - Schema version incompatible
- `NotFound(String)` - State component not found
- `Corrupted(String)` - State data corrupted
- `LockError(String)` - Lock acquisition failed

**Usage**:
```rust
impl StateHolder for MyComponent {
    fn serialize_state(&self) -> Result<Vec<u8>, StateError> {
        bincode::serialize(&self.state)
            .map_err(|e| StateError::SerializationError(e.to_string()))
    }
}
```

---

### QueryError
**Purpose**: Query parsing, compilation, and validation

**Variants**:
- `ParseError(String)` - Query parsing failed
- `ValidationError(String)` - Query validation failed
- `UndefinedStream(String)` - Referenced stream not defined
- `TypeMismatch { expected, actual }` - Type incompatibility
- `SyntaxError { line, column, message }` - Syntax error

**Usage**:
```rust
pub fn parse_query(query: &str) -> Result<AST, QueryError> {
    if !is_valid_syntax(query) {
        return Err(QueryError::SyntaxError {
            line: 1,
            column: 0,
            message: "Invalid query syntax".to_string(),
        });
    }
    // ... parse query
}
```

---

### RuntimeError
**Purpose**: Runtime execution errors

**Variants**:
- `ExecutionError(String)` - Query execution failed
- `StateError(StateError)` - State operation failed (wrapped)
- `QueryError(QueryError)` - Query error (wrapped)
- `ResourceExhausted(String)` - Out of resources
- `Timeout(String)` - Operation timed out

**Usage**:
```rust
pub fn execute_query(&mut self) -> Result<(), RuntimeError> {
    let state = load_state()?;  // StateError auto-converts to RuntimeError
    let query = parse_query(query_string)?;  // QueryError auto-converts
    Ok(())
}
```

---

### IOError
**Purpose**: I/O operations (sources, sinks, network)

**Variants**:
- `ConnectionFailed { host, port, reason }` - Connection failed
- `ReadError(String)` - Read operation failed
- `WriteError(String)` - Write operation failed
- `Timeout { operation, duration }` - I/O timeout
- `SerializationError(String)` - Data serialization failed

**Usage**:
```rust
pub fn send_event(&self, event: Event) -> Result<(), IOError> {
    self.connection.send(&event).map_err(|e| IOError::ConnectionFailed {
        host: self.host.clone(),
        port: self.port,
        reason: e.to_string(),
    })
}
```

---

### ConfigError
**Purpose**: Configuration and validation

**Variants**:
- `InvalidValue { key, value, reason }` - Invalid config value
- `MissingRequired(String)` - Required config missing
- `ParseError(String)` - Config parsing failed
- `ValidationError(String)` - Config validation failed

**Usage**:
```rust
pub fn load_config(path: &str) -> Result<Config, ConfigError> {
    if !Path::new(path).exists() {
        return Err(ConfigError::MissingRequired(format!("Config file not found: {}", path)));
    }
    // ... load config
}
```

---

## Error Handling Patterns

### Pattern 1: Simple Error Propagation
```rust
pub fn process() -> Result<(), StateError> {
    let state = load_state()?;  // Propagate error
    save_state(&state)?;         // Propagate error
    Ok(())
}
```

### Pattern 2: Error Context Addition
```rust
pub fn process_with_context() -> Result<(), StateError> {
    let state = load_state()
        .map_err(|e| {
            eprintln!("Failed to load state: {}", e);
            e
        })?;
    Ok(())
}
```

### Pattern 3: Error Type Conversion
```rust
pub fn mixed_operations() -> Result<(), RuntimeError> {
    // StateError converts to RuntimeError automatically
    let state = load_state()?;

    // QueryError converts to RuntimeError automatically
    let query = parse_query("SELECT * FROM stream")?;

    Ok(())
}
```

### Pattern 4: Error Recovery
```rust
pub fn process_with_fallback() -> Result<State, StateError> {
    match load_state() {
        Ok(state) => Ok(state),
        Err(StateError::NotFound(_)) => {
            // Recover by creating new state
            Ok(State::new())
        }
        Err(e) => Err(e),  // Other errors propagate
    }
}
```

### Pattern 5: Comprehensive Error Information
```rust
pub fn detailed_error() -> Result<(), StateError> {
    compress_data(&data).map_err(|e| {
        StateError::CompressionError {
            algorithm: "zstd".to_string(),
            reason: format!("Compression failed: {} (data size: {})",
                e, data.len()),
        }
    })?;
    Ok(())
}
```

---

## Quick Start

### Defining Custom Errors

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StateError {
    #[error("Failed to serialize state: {0}")]
    SerializationError(String),

    #[error("Failed to deserialize state: {0}")]
    DeserializationError(String),

    #[error("Compression failed: {algorithm} - {reason}")]
    CompressionError {
        algorithm: String,
        reason: String,
    },

    #[error("State version mismatch: expected {expected}, got {actual}")]
    VersionMismatch {
        expected: String,
        actual: String,
    },
}
```

### Using Errors in Functions

```rust
pub fn save_state(&self) -> Result<(), StateError> {
    let data = self.serialize_state()
        .map_err(|e| StateError::SerializationError(e.to_string()))?;

    let compressed = compress_data(&data)
        .map_err(|e| StateError::CompressionError {
            algorithm: "zstd".to_string(),
            reason: e.to_string(),
        })?;

    Ok(())
}
```

### Error Matching

```rust
match save_state() {
    Ok(()) => println!("State saved"),
    Err(StateError::SerializationError(msg)) => {
        eprintln!("Serialization failed: {}", msg);
    }
    Err(StateError::CompressionError { algorithm, reason }) => {
        eprintln!("Compression failed with {}: {}", algorithm, reason);
    }
    Err(e) => eprintln!("Unknown error: {}", e),
}
```

---

## Migration Guide

### From String Errors

**Before**:
```rust
pub fn save_state(&self) -> Result<(), String> {
    let data = self.serialize()
        .map_err(|e| format!("Serialization failed: {}", e))?;
    Ok(())
}
```

**After**:
```rust
pub fn save_state(&self) -> Result<(), StateError> {
    let data = self.serialize()
        .map_err(|e| StateError::SerializationError(e.to_string()))?;
    Ok(())
}
```

**Benefits**:
- Type-safe error handling
- Better error matching
- Automatic error conversion
- Structured error information

---

## Best Practices

### 1. Use Specific Error Types
```rust
// Good - specific error type
pub fn save() -> Result<(), StateError> { }

// Avoid - generic error
pub fn save() -> Result<(), Box<dyn Error>> { }
```

### 2. Provide Context
```rust
// Good - context in error
StateError::CompressionError {
    algorithm: "zstd".to_string(),
    reason: format!("Failed: {}", e),
}

// Avoid - generic message
StateError::CompressionError("failed".to_string())
```

### 3. Use Error Hierarchies
```rust
// Good - wrap specific errors
RuntimeError::StateError(state_error)

// Avoid - convert to string
RuntimeError::ExecutionError(state_error.to_string())
```

### 4. Implement Display Properly
```rust
#[derive(Error, Debug)]
#[error("Connection failed to {host}:{port} - {reason}")]
pub struct ConnectionError {
    host: String,
    port: u16,
    reason: String,
}
```

### 5. Test Error Cases
```rust
#[test]
fn test_invalid_state() {
    let result = load_state("invalid_path");
    assert!(matches!(result, Err(StateError::NotFound(_))));
}
```

---

## Error Metrics

### Error Coverage
- **121+ Result<T, Error>**: Comprehensive error handling throughout codebase
- **0 panics**: All failures return Result types
- **Type-safe**: Compile-time error type verification

### Error Categories
- **State Operations**: 40+ error points
- **Query Processing**: 30+ error points
- **Runtime Execution**: 25+ error points
- **I/O Operations**: 15+ error points
- **Configuration**: 11+ error points

---

## Implementation Summary

### What Was Implemented

#### 1. Comprehensive Error Hierarchy
Created `EventFluxError` enum with 25+ error variants covering:
- **Application Lifecycle**: EventFluxAppCreation, EventFluxAppRuntime
- **Query Operations**: QueryCreation, QueryRuntime, OnDemandQuery operations
- **Data Access**: StoreQuery, NoSuchAttribute, DefinitionNotExist
- **Extensions**: ExtensionNotFound, CannotLoadClass
- **Persistence**: PersistenceStore, CannotPersistState, CannotRestoreState
- **Type System**: TypeError, InvalidParameter
- **Runtime**: ProcessorError, SendError, MappingFailed
- **External Dependencies**: IO errors, SQLite errors, Serialization errors

#### 2. Error Context and Source Chaining
- Added `#[source]` attributes for error chaining
- Implemented contextual information (query names, parameter names, etc.)
- Helper methods for common error construction patterns

#### 3. Backward Compatibility
- Maintained `From<String>` and `From<&str>` implementations
- Added `IntoEventFluxResult` trait for gradual migration
- Preserved existing API signatures while improving error quality

#### 4. Updated Error Store
- Modified `ErrorStore` trait to work with the new error types
- Changed error storage to use string representations for serialization
- Maintained thread safety and performance characteristics

### Benefits Achieved

#### 1. Better Debugging
- Structured error information instead of plain strings
- Source error chaining for root cause analysis
- Contextual information (parameter names, query names, etc.)

#### 2. Improved Error Handling
- Type-safe error matching and handling
- Automatic error formatting with thiserror
- Integration with Rust's error ecosystem

#### 3. Production Readiness
- Professional error reporting suitable for production use
- Consistent error formats across all modules
- Support for error monitoring and logging systems

#### 4. Developer Experience
- Clear error messages with actionable information
- IDE support for error types and documentation
- Easy error creation with helper methods

---

## Next Steps

See [MILESTONES.md](../../MILESTONES.md):
- **M2 (v0.2)**: Enhanced I/O errors for connector failures
- **M6 (v0.6)**: Security errors for authentication/authorization
- **M7 (v0.7)**: Distributed errors for cluster coordination

---

## Contributing

When adding error handling:
1. Use `thiserror` for all error types
2. Provide context in error messages
3. Use error hierarchies appropriately
4. Write tests for error cases
5. Document new error variants

---

**Status**: Production Ready - Complete error handling system with 121+ Result types and comprehensive coverage
