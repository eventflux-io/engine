# Array Access for Pattern Event Collections

**Status**: Runtime Complete | Parser Not Implemented
**Date**: 2025-11-23
**Last Updated**: 2025-11-23

## Implementation Status

✅ **Runtime**: Complete and tested (21 tests passing)
❌ **Parser**: Not implemented (parser is syntactic sugar on top of programmatic API)

## Overview

Array access enables accessing specific events from event collections matched by count quantifiers: `e[0]` (first), `e[last]` (last), `e[2]` (specific index).

**Example**:
```rust
// Pattern: e1=FailedLogin{3,5}
let first_timestamp = Expression::indexed_variable_with_index(
    "timestamp".to_string(), 0, Some("e1".to_string()), Some(0)
);
let last_timestamp = Expression::indexed_variable_with_last(
    "timestamp".to_string(), Some("e1".to_string()), Some(0)
);
```

## Architecture: StateEvent Event Chains

### Core Concept

Array access navigates event chains created by count quantifiers (`{n,m}`). Count quantifiers link multiple StreamEvents via `next` pointers at a specific position in the StateEvent.

**Structure**:
```rust
StateEvent {
    stream_events: Vec<Option<StreamEvent>>,  // e1=position 0, e2=position 1, ...
}

StreamEvent {
    next: Option<Box<dyn ComplexEvent>>,  // Next in chain
    before_window_data: Vec<AttributeValue>,
    // ...
}
```

**Example**: `e1=FailedLogin{5}` creates chain of 5 events at position 0:
```
stream_events[0] → FL₁ → FL₂ → FL₃ → FL₄ → FL₅
                   e1[0] e1[1] e1[2] e1[3] e1[last]
```

**Execution**:
```rust
pub fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
    let state_event = event?.as_any().downcast_ref::<StateEvent>()?;
    let event_chain = state_event.get_event_chain(self.state_position);
    let resolved_index = match &self.index {
        EventIndex::Numeric(idx) => *idx,
        EventIndex::Last => event_chain.len().saturating_sub(1),
    };
    let stream_event = event_chain.get(resolved_index)?;
    stream_event.before_window_data.get(attr_idx).cloned()
}
```

**Key Points**:
- Two-level indexing: position (e1, e2, ...) then index (0, 1, ..., last)
- `e[last]` resolved at runtime to `event_chain.len() - 1`
- Out-of-bounds returns `None` (SQL NULL)

## Query API Design

**File**: `src/query_api/expression/indexed_variable.rs`

```rust
pub enum EventIndex {
    Numeric(usize),  // 0, 1, 2, ...
    Last,            // Resolved to collection.len() - 1
}

pub struct IndexedVariable {
    pub eventflux_element: EventFluxElement,
    pub stream_id: Option<String>,
    pub stream_index: Option<i32>,  // Position in StateEvent (e1=0, e2=1, ...)
    pub index: EventIndex,
    pub attribute_name: String,
}
```

**Expression Integration**:
```rust
pub enum Expression {
    // ... other variants
    IndexedVariable(Box<IndexedVariable>),
}

// Factory methods
impl Expression {
    pub fn indexed_variable_with_index(...) -> Self { ... }
    pub fn indexed_variable_with_last(...) -> Self { ... }
}
```

## Runtime Executor

**File**: `src/core/executor/indexed_variable_executor.rs`

```rust
pub struct IndexedVariableExecutor {
    pub state_position: usize,             // e1=0, e2=1, ...
    pub index: EventIndex,                 // 0, 1, ..., last
    pub attribute_position: [i32; 2],      // [data_type, attr_idx]
    pub return_type: ApiAttributeType,
    pub attribute_name_for_debug: String,
}
```

## Test Coverage

**Total**: 21 tests passing

| Category | Tests | Status |
|----------|-------|--------|
| Runtime executor | 12 | ✅ All passing |
| Query API | 9 | ✅ All passing |

**Coverage**:
- First/middle/last event access
- Multiple data types (INT, LONG, DOUBLE, STRING)
- Multiple stream positions (e1[0], e2[1])
- Out-of-bounds returns None
- Empty chains return None
- Non-StateEvent returns None

## Implementation Files

| Component | File | Lines |
|-----------|------|-------|
| Query API | `src/query_api/expression/indexed_variable.rs` | 220 |
| Runtime executor | `src/core/executor/indexed_variable_executor.rs` | 512 |
| Expression enum | `src/query_api/expression/expression.rs` | (updated) |
| Tests | `src/core/executor/indexed_variable_executor.rs` | (tests section) |

## Programmatic Usage

```rust
// e1[0].timestamp
let first_ts = Expression::indexed_variable_with_index(
    "timestamp".to_string(), 0, Some("e1".to_string()), Some(0)
);

// e1[last].timestamp
let last_ts = Expression::indexed_variable_with_last(
    "timestamp".to_string(), Some("e1".to_string()), Some(0)
);

// Create executor and execute
let executor = IndexedVariableExecutor::new(
    0,                              // position
    EventIndex::Numeric(0),         // index
    [BEFORE_WINDOW_DATA_INDEX, 1],  // attribute position
    ApiAttributeType::LONG,
    "timestamp".to_string(),
);
let result = executor.execute(Some(&state_event));
```

## Grammar Design (Not Implemented)

❌ **Parser**: Not implemented (parser is syntactic sugar on top of programmatic API)

### Intended Syntax

```sql
-- Array access in SELECT clause
SELECT
    e1[0].timestamp as firstAttempt,
    e1[last].timestamp as lastAttempt,
    e1[2].userId,
    count(e1) as attempts
FROM PATTERN (
    e1=FailedLogin{3,5} -> e2=AccountLocked
)
```

### Supported Index Forms (Design)

| Syntax | Meaning | Example |
|--------|---------|---------|
| `e[0]` | First event | `e1[0].userId` |
| `e[1]`, `e[2]`, ... | Specific index | `e1[1].timestamp` |
| `e[last]` | Last event (dynamic) | `e1[last].price` |

### Not Supported (Design Decision)

| Syntax | Reason |
|--------|--------|
| `e[first]` | Use `e[0]` instead |
| `e[-1]` | Negative indexing not needed |
| `e[0:5]` | Range access not needed |
| `e[someVar]` | Dynamic expressions add complexity |

## Performance

| Operation | Complexity |
|-----------|------------|
| `e[0]` | O(1) |
| `e[last]` | O(n) - must traverse chain |
| `e[2]` | O(n) - must traverse chain |

**Note**: Events stored as linked list via `next` pointer. `get_event_chain()` traverses to build Vec.
