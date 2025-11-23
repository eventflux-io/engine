# Array Access for Pattern Event Collections

**Version**: 1.0
**Date**: 2025-11-23
**Status**: Complete (P0 Critical)

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture: StateEvent Chains Explained](#architecture-stateevent-chains-explained)
3. [Requirements](#requirements)
4. [Design](#design)
5. [Implementation Status](#implementation-status)
6. [Test Coverage](#test-coverage)
7. [Programmatic Usage Examples](#programmatic-usage-examples)
8. [Performance Characteristics](#performance-characteristics)
9. [Remaining Work](#remaining-work)

---

## Overview

Array access enables accessing specific events from event collections matched by count quantifiers in pattern queries. This allows expressions like `e[0].userId` (first event), `e[last].timestamp` (last event), and `e[2].price` (third event) to work programmatically without parser.

### Implementation Components

- Query API Layer: `IndexedVariable` struct with `EventIndex` enum
- Runtime Executor: `IndexedVariableExecutor` for evaluation
- Expression Integration: Added to `Expression` enum
- Tests: 21/21 passing
- Documentation: Design and implementation docs

### Quick Example

```rust
// For pattern: e1=FailedLogin{3,5}
// SELECT e1[0].timestamp as firstAttempt,
//        e1[last].timestamp as lastAttempt

let first_timestamp = Expression::indexed_variable_with_index(
    "timestamp".to_string(),
    0,  // index
    Some("e1".to_string()),
    Some(0),  // stream position
);

let last_timestamp = Expression::indexed_variable_with_last(
    "timestamp".to_string(),
    Some("e1".to_string()),
    Some(0),
);
```

---

## Architecture: StateEvent Chains Explained

### The Critical Distinction

Array access does not access items in a ComplexEvent list. It accesses specific StreamEvents within a chain that exists at a particular position inside a StateEvent. The chain is created by count quantifiers (`{n,m}`).

### Data Structure Hierarchy

```rust
StateEvent {
    stream_events: Vec<Option<StreamEvent>>,  // Multiple positions (e1, e2, e3, ...)
    // Position 0 → e1's event chain
    // Position 1 → e2's event chain
    // Position 2 → e3's event chain
}

// Each position can have a CHAIN of StreamEvents linked via `next` pointer:
StreamEvent {
    next: Option<Box<dyn ComplexEvent>>,  // Next event in chain
    before_window_data: Vec<AttributeValue>,
    output_data: Option<Vec<AttributeValue>>,
    // ... other fields
}
```

### How Count Quantifiers Create Chains

When you have a pattern with count quantifiers:

```rust
e1=FailedLogin{3,5}  // Match 3-5 FailedLogin events
```

The runtime builds this structure as events arrive:

```
StateEvent {
    stream_events[0] = Some(StreamEvent) → StreamEvent → StreamEvent → StreamEvent → StreamEvent
                         ↑                    ↑            ↑            ↑            ↑
                       e1[0]               e1[1]        e1[2]        e1[3]        e1[4]
                      (first)                                                    (last)
}
```

Array access navigates the chain at position 0 (e1):
- `e1[0]` → First StreamEvent in the chain at position 0
- `e1[1]` → Second StreamEvent in the chain at position 0
- `e1[last]` → Last StreamEvent in the chain at position 0

### Concrete Multi-Position Example

```rust
// Pattern: e1=FailedLogin{3,5} -> e2=AccountLocked
// Events arrive: FL₁, FL₂, FL₃, FL₄, FL₅, AL

// StateEvent structure after match:
StateEvent {
    stream_events: [
        Some(FL₁ → FL₂ → FL₃ → FL₄ → FL₅),  // Position 0 (e1) - chain of 5 events
        Some(AL),                              // Position 1 (e2) - single event
    ]
}

// Array access examples:
e1[0].userId     → Accesses FL₁ (first in chain at position 0)
e1[2].userId     → Accesses FL₃ (third in chain at position 0)
e1[last].userId  → Accesses FL₅ (last in chain at position 0)
e2[0].timestamp  → Accesses AL (first/only event at position 1)
e2[last].timestamp → Also accesses AL (last = first when only 1 event)
```

### Implementation Flow

```rust
// From IndexedVariableExecutor::execute()
pub fn execute(&self, event_opt: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
    // Step 1: Must be StateEvent (patterns build StateEvents)
    let state_event = complex_event.as_any().downcast_ref::<StateEvent>()?;

    // Step 2: Get the CHAIN at the specified position (e.g., position 0 for e1)
    let event_chain = state_event.get_event_chain(self.state_position);
    //                                             ^^^^^^^^^^^^^^^^^^
    //                                             e1=0, e2=1, e3=2, ...

    // Step 3: Resolve index within that chain
    let resolved_index = match &self.index {
        EventIndex::Numeric(idx) => *idx,          // e[0], e[1], e[2], ...
        EventIndex::Last => event_chain.len() - 1, // e[last]
    };

    // Step 4: Get specific event from the chain
    let stream_event = event_chain.get(resolved_index)?;

    // Step 5: Extract attribute from that event
    let attr = stream_event.before_window_data.get(attr_idx).cloned();

    attr
}
```

### Key Architectural Points

1. Two-Level Indexing:
   - First level: Position in StateEvent (e1, e2, e3, ...)
   - Second level: Index within event chain at that position (0, 1, 2, ..., last)

2. Chain Creation:
   - Count quantifiers (`{n,m}`) create linked lists of events
   - Single events are just chains of length 1

3. Dynamic Resolution:
   - `e[last]` is resolved at runtime to `event_chain.len() - 1`
   - Handles variable-length collections naturally

4. Null Safety:
   - Out-of-bounds returns `None` (SQL NULL semantics)
   - Empty chains return `None`
   - Non-StateEvent returns `None`

---

## Requirements

### Functional Requirements

1. Index-based access: `e[0]`, `e[1]`, `e[2]`, ... for specific events in collection
2. Last event access: `e[last]` dynamically resolves to last event in collection
3. Attribute access: `e[0].userId`, `e[last].timestamp` for attributes of indexed events
4. Null handling: Out-of-bounds access returns `null` (not error)
5. Zero-based indexing: `e[0]` is first event, `e[1]` is second, etc.

### Non-Requirements (Not Supported)

- `e[first]` keyword (use `e[0]` instead)
- Negative indexing `e[-1]`, `e[-2]`
- Range access `e[0:5]`
- Dynamic expressions as index `e[someVar]`

---

## Design

### 1. Query API: IndexedVariable Struct

**Location**: `src/query_api/expression/indexed_variable.rs`

```rust
/// Index specification for array access
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventIndex {
    /// Numeric index (0, 1, 2, ...)
    Numeric(usize),
    /// Special "last" keyword
    Last,
}

impl EventIndex {
    /// Resolve index to concrete value given collection size
    pub fn resolve(&self, collection_size: usize) -> Option<usize> {
        if collection_size == 0 {
            return None;
        }
        match self {
            EventIndex::Numeric(idx) => {
                if *idx < collection_size {
                    Some(*idx)
                } else {
                    None  // Out of bounds
                }
            }
            EventIndex::Last => Some(collection_size - 1),
        }
    }
}

/// Represents indexed access to events in a pattern collection
#[derive(Clone, Debug, PartialEq)]
pub struct IndexedVariable {
    pub eventflux_element: EventFluxElement,
    pub stream_id: Option<String>,
    pub stream_index: Option<i32>,  // Position in StateEvent (e1=0, e2=1, ...)
    pub index: EventIndex,           // Index within chain (0, 1, 2, last)
    pub attribute_name: String,
}

impl IndexedVariable {
    pub fn new_with_index(attribute_name: String, index: usize) -> Self { /* ... */ }
    pub fn new_with_last(attribute_name: String) -> Self { /* ... */ }
    pub fn of_stream_with_index(self, stream_id: String, stream_index: i32) -> Self { /* ... */ }
}
```

Design Decisions:
- EventIndex enum provides type-safe distinction between numeric and "last"
- Separate from Variable due to different semantics (chain navigation vs simple attribute access)
- stream_index required to know which position to access in StateEvent

### 2. Expression Enum Update

**Location**: `src/query_api/expression/expression.rs`

```rust
pub enum Expression {
    Constant(Constant),
    Variable(Variable),
    IndexedVariable(Box<IndexedVariable>),  // NEW
    // ... other variants
}

impl Expression {
    pub fn indexed_variable_with_index(
        attribute_name: String,
        index: usize,
        stream_id: Option<String>,
        stream_index: Option<i32>,
    ) -> Self {
        let mut var = IndexedVariable::new_with_index(attribute_name, index);
        if let (Some(sid), Some(sidx)) = (stream_id, stream_index) {
            var = var.of_stream_with_index(sid, sidx);
        }
        Expression::IndexedVariable(Box::new(var))
    }

    pub fn indexed_variable_with_last(
        attribute_name: String,
        stream_id: Option<String>,
        stream_index: Option<i32>,
    ) -> Self { /* ... */ }
}
```

### 3. Runtime: IndexedVariableExecutor

**Location**: `src/core/executor/indexed_variable_executor.rs`

```rust
pub struct IndexedVariableExecutor {
    /// Position in StateEvent.stream_events[] array (e1=0, e2=1, ...)
    pub state_position: usize,

    /// Index into the event chain (0, 1, 2, ... or "last")
    pub index: EventIndex,

    /// Attribute position within event's data arrays
    /// [0] = data_type_index (BEFORE_WINDOW_DATA_INDEX, OUTPUT_DATA_INDEX, etc.)
    /// [1] = attribute_index (index within that data array)
    pub attribute_position: [i32; 2],

    pub return_type: ApiAttributeType,
    pub attribute_name_for_debug: String,
}

impl ExpressionExecutor for IndexedVariableExecutor {
    fn execute(&self, event_opt: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let complex_event = event_opt?;

        // Must be StateEvent for indexed array access
        let state_event = complex_event.as_any().downcast_ref::<StateEvent>()?;

        // Get event chain at position
        let event_chain = state_event.get_event_chain(self.state_position);
        if event_chain.is_empty() {
            return None;
        }

        // Resolve index (numeric or "last")
        let resolved_index = match &self.index {
            EventIndex::Numeric(idx) => *idx,
            EventIndex::Last => event_chain.len().saturating_sub(1),
        };

        // Get event at index (returns None if out of bounds)
        let stream_event = event_chain.get(resolved_index)?;

        // Extract attribute
        let attr_idx = self.attribute_position[1] as usize;
        stream_event.before_window_data.get(attr_idx).cloned()
        // (with fallback logic to output_data if needed)
    }
}
```

Implementation Decisions:
1. Requires StateEvent: Returns `None` if event is not StateEvent
2. Null on out-of-bounds: `event_chain.get(index)` returns `None` safely
3. Dynamic "last" resolution: Calculates `event_chain.len() - 1` at runtime
4. Reuses attribute extraction: Same logic as VariableExpressionExecutor

---

## Implementation Status

### Completed Components

| Component | Status | Files | Tests |
|-----------|--------|-------|-------|
| Query API | Complete | indexed_variable.rs (220 lines) | 9/9 passing |
| Expression Integration | Complete | expression.rs (updated) | - |
| Runtime Executor | Complete | indexed_variable_executor.rs (512 lines) | 12/12 passing |
| Compilation Fixes | Complete | expression_parser.rs, type_inference.rs | - |

### Files Created/Modified

Created:
- `src/query_api/expression/indexed_variable.rs` (220 lines)
- `src/core/executor/indexed_variable_executor.rs` (512 lines)

Modified:
- `src/query_api/expression/mod.rs` (added module and exports)
- `src/query_api/expression/expression.rs` (added variant, factory methods, 4 match updates)
- `src/core/executor/mod.rs` (added module and export)
- `src/core/util/parser/expression_parser.rs` (added temporary placeholder)
- `src/sql_compiler/type_inference.rs` (added type inference)

### Build Results

```bash
Compilation: Success (256 warnings - unrelated deprecations)
All Tests: 21/21 passing
No errors
```

---

## Test Coverage

### Runtime Executor Tests (12 tests)

| Test | Scenario | Assertion |
|------|----------|-----------|
| `test_indexed_access_first_event` | e[0] returns first event | `assert_eq!(result, Some("user1"))` |
| `test_indexed_access_second_event` | e[1] returns second event | `assert_eq!(result, Some("user2"))` |
| `test_indexed_access_last_keyword` | e[last] returns last event | `assert_eq!(result, Some("user3"))` |
| `test_indexed_access_last_with_single_event` | e[last] with 1 event | `assert_eq!(result, Some(Int(42)))` |
| `test_indexed_access_out_of_bounds` | e[100] with 1 event returns None | `assert_eq!(result, None)` |
| `test_indexed_access_empty_event_chain` | Empty chain returns None | `assert_eq!(result, None)` |
| `test_indexed_access_different_attribute_types` | INT, LONG, DOUBLE types | 3 separate `assert_eq!` |
| `test_indexed_access_multiple_positions` | e1[0] and e2[1] in same StateEvent | 2 separate assertions |
| `test_indexed_access_fallback_to_output_data` | Falls back to output_data | `assert_eq!(result, Some("output_val"))` |
| `test_indexed_access_returns_none_for_non_state_event` | StreamEvent returns None | `assert_eq!(result, None)` |
| `test_indexed_access_no_event` | None event returns None | `assert_eq!(result, None)` |
| `test_clone_executor` | Cloned executor works | `assert_eq!(result, Some("user2"))` |

### Query API Tests (9 tests)

| Test | Scenario | Assertion |
|------|----------|-----------|
| `test_event_index_resolve_numeric` | Numeric index resolves | `assert_eq!(index.resolve(3), Some(0))` |
| `test_event_index_resolve_last` | "last" resolves to size-1 | `assert_eq!(index.resolve(5), Some(4))` |
| `test_event_index_resolve_out_of_bounds` | Out of bounds returns None | `assert_eq!(index.resolve(3), None)` |
| `test_event_index_resolve_last_empty` | "last" on empty returns None | `assert_eq!(index.resolve(0), None)` |
| `test_indexed_variable_creation_numeric` | Creates with numeric index | 4 field assertions |
| `test_indexed_variable_creation_last` | Creates with "last" | 4 field assertions |
| `test_indexed_variable_with_stream` | Binds to stream | 4 field assertions |
| `test_indexed_variable_last_with_stream` | "last" with stream binding | 4 field assertions |
| `test_indexed_variable_multiple_indices` | Multiple indices | 3 index assertions |

### Test Execution Results

```bash
running 21 tests
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured
```

### Coverage Summary

Happy Paths:
- First, middle, last event access
- Multiple data types (INT, LONG, DOUBLE, STRING)
- Multiple stream positions (e1[0], e2[1])

Edge Cases:
- Out of bounds index returns None
- Empty event chain returns None
- Single event collection with e[last]
- Non-StateEvent returns None
- None event returns None

Special Cases:
- Fallback to output_data
- Clone executor preserves functionality
- EventIndex.resolve() boundary checks

---

## Programmatic Usage Examples

### Example 1: Basic Array Access

```rust
use eventflux_rust::query_api::expression::Expression;

// Create e1[0].userId
let first_user = Expression::indexed_variable_with_index(
    "userId".to_string(),      // attribute name
    0,                          // index
    Some("e1".to_string()),     // stream ID
    Some(0),                    // stream position
);

// Create e1[last].timestamp
let last_timestamp = Expression::indexed_variable_with_last(
    "timestamp".to_string(),
    Some("e1".to_string()),
    Some(0),
);
```

### Example 2: With Count Quantifier

```rust
// For pattern: e1=FailedLogin{3,5}
// SELECT e1[0].timestamp as firstAttempt,
//        e1[last].timestamp as lastAttempt,
//        count(e1) as attempts

// Build state event with 5 failed login events
let mut state_event = StateEvent::new(1, 0);  // 1 stream position, 0 output
for i in 0..5 {
    let mut event = StreamEvent::new(0, 2, 0, 0);  // 2 attributes
    event.before_window_data = vec![
        AttributeValue::String(format!("user{}", i)),     // userId
        AttributeValue::Long(timestamp + i * 1000),       // timestamp
    ];
    state_event.add_event(0, event);  // Add to position 0 (e1)
}

// Execute e1[0].timestamp (first event's timestamp)
let first_timestamp_executor = IndexedVariableExecutor::new(
    0,                              // position 0 (e1)
    EventIndex::Numeric(0),         // e[0]
    [BEFORE_WINDOW_DATA_INDEX, 1],  // timestamp at index 1
    ApiAttributeType::LONG,
    "timestamp".to_string(),
);

let first = first_timestamp_executor.execute(Some(&state_event));
// Returns: Some(AttributeValue::Long(timestamp))

// Execute e1[last].timestamp (last event's timestamp)
let last_timestamp_executor = IndexedVariableExecutor::new(
    0,                              // position 0 (e1)
    EventIndex::Last,               // e[last]
    [BEFORE_WINDOW_DATA_INDEX, 1],  // timestamp at index 1
    ApiAttributeType::LONG,
    "timestamp".to_string(),
);

let last = last_timestamp_executor.execute(Some(&state_event));
// Returns: Some(AttributeValue::Long(timestamp + 4000))
```

### Example 3: Multiple Positions

```rust
// For pattern: e1=Login{3} -> e2=DataAccess{5}
// SELECT e1[0].userId, e1[last].timestamp,
//        e2[0].bytes, e2[last].bytes

let mut state_event = StateEvent::new(2, 0);  // 2 positions

// Add 3 Login events to position 0 (e1)
for i in 0..3 {
    let mut event = StreamEvent::new(0, 2, 0, 0);
    event.before_window_data = vec![
        AttributeValue::String(format!("user{}", i)),
        AttributeValue::Long(1000 + i * 100),
    ];
    state_event.add_event(0, event);
}

// Add 5 DataAccess events to position 1 (e2)
for i in 0..5 {
    let mut event = StreamEvent::new(0, 1, 0, 0);
    event.before_window_data = vec![
        AttributeValue::Long(100 * (i + 1)),  // bytes
    ];
    state_event.add_event(1, event);
}

// Access e1[0].userId
let e1_first_user = IndexedVariableExecutor::new(
    0, EventIndex::Numeric(0), [BEFORE_WINDOW_DATA_INDEX, 0],
    ApiAttributeType::STRING, "userId".to_string(),
);
// Returns: Some(AttributeValue::String("user0"))

// Access e2[last].bytes
let e2_last_bytes = IndexedVariableExecutor::new(
    1, EventIndex::Last, [BEFORE_WINDOW_DATA_INDEX, 0],
    ApiAttributeType::LONG, "bytes".to_string(),
);
// Returns: Some(AttributeValue::Long(500))
```

---

## Performance Characteristics

### Time Complexity

| Operation | Complexity | Notes |
|-----------|------------|-------|
| First event (`e[0]`) | O(1) | Direct access via `event_chain[0]` |
| Last event (`e[last]`) | O(n) | Must traverse linked list via `get_event_chain()` |
| Specific index (`e[2]`) | O(n) | Must traverse linked list to build chain |

Explanation of O(n) for last/specific:
- Events are stored as linked list via `next` pointer
- `get_event_chain()` must traverse the entire chain to build a Vec
- Current implementation prioritizes correctness over performance

### Space Complexity

O(1) - No additional allocation in executor, reuses StateEvent's existing event chain

### Optimization Opportunities (Future)

1. Cache event chain Vec in StateEvent:
   - Avoid repeated traversal for multiple array accesses
   - Trade-off: Memory vs CPU

2. Direct indexing without Vec allocation:
   - Traverse linked list only to the required index
   - Avoids building full Vec when only accessing one element

Current implementation prioritizes correctness. Performance optimizations can be added later if needed.

---

## Remaining Work

### Parser Integration (Deferred to Grammar Implementation Phase)

Not implemented (intentionally deferred):
- Parse `e[0]`, `e[last]` syntax
- Parse `e[0].attribute` attribute access
- Conflict resolution with float literals (`e[0]` vs `1.0`)

Reason: User requested "no parser work yet, parser is syntactic sugaring on top of APIs"

### Expression Compiler Integration (Future Work)

Not implemented (requires pattern compiler infrastructure):
- Compile IndexedVariable in pattern expression compiler
- Metadata lookup for attribute position
- Integration tests with full pattern queries

Reason: Requires pattern compiler infrastructure which is separate work

### Current State

Programmatic API: Functional and tested with 21 tests passing
Runtime Execution: IndexedVariableExecutor implemented with 12 passing tests
Query API: IndexedVariable struct and EventIndex enum implemented
Documentation: Design and architecture documentation created

Parser integration is not implemented. This is deferred to Grammar V1.2 parser implementation phase.

---

## Next Steps

### Immediate (P1 Features - Next 2-4 weeks)
1. Verify cross-stream references work with StateEvent context
2. Verify EVERY multi-instance runtime support
3. Verify aggregations work over event collections
4. Implement PARTITION BY multi-tenant isolation

### Future (Grammar Integration)
1. Implement Grammar V1.2 parser
2. Wire IndexedVariableExecutor to pattern expression compiler
3. Add integration tests with full pattern queries
4. Performance optimization if needed (cache event chains)

---

**Implementation Status**: Programmatic API complete, parser not implemented
**Last Updated**: 2025-11-23
**Document Version**: 1.0
