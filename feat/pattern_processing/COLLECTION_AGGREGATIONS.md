# Collection Aggregations for Pattern Events

**Status**: EXECUTORS COMPLETE (P1 - Parser Integration Pending)
**Date**: 2025-11-27 (Executors Implemented)
**Previous Date**: 2025-11-23

## Overview

Collection aggregations enable aggregate calculations over event collections created by count quantifiers in patterns. This is distinct from window aggregations which operate over time-series data.

## Grammar Examples

```sql
-- Count events in collection
FROM PATTERN (
    e1=FailedLogin{3,5} -> e2=AccountLocked
)
SELECT count(e1) as attempts;

-- Average over collection
FROM PATTERN (
    e1=StockPrice{5} -> e2=Alert
)
SELECT avg(e1.price) as avgPrice;

-- Min/Max over collection
SELECT min(e1.temperature), max(e1.temperature);

-- Array access (for specific event in collection)
SELECT e1[0].userId, e1[last].timestamp;
```

---

## Current Implementation Status

### Summary Table

| Component | Status | Location |
|-----------|--------|----------|
| **Array Access Runtime** (`e[0].attr`, `e[last].attr`) | ✅ COMPLETE | `src/core/executor/indexed_variable_executor.rs` |
| **IndexedVariable Query API** | ✅ COMPLETE | `src/query_api/expression/indexed_variable.rs` |
| **MultiValueVariableFunctionExecutor** | ✅ EXISTS | `src/core/executor/multi_value_variable_function_executor.rs` |
| **StateEvent.get_event_chain()** | ✅ COMPLETE | `src/core/event/state/state_event.rs:261-279` |
| **StateEvent.count_events_at()** | ✅ COMPLETE | `src/core/event/state/state_event.rs:282-284` |
| **Expression::IndexedVariable** | ✅ COMPLETE | `src/query_api/expression/expression.rs:20` |
| **Collection Aggregation Executors** | ✅ COMPLETE | `src/core/executor/collection_aggregation_executor.rs` |
| **Parser Support** | ❌ NOT IMPLEMENTED | - |

### What's Complete ✅

#### 1. Array Access - FULLY IMPLEMENTED

The `IndexedVariableExecutor` provides complete runtime support for accessing specific events in a collection:

```rust
// src/core/executor/indexed_variable_executor.rs (191 lines, 14+ tests)

// Supports:
e1[0].userId      // First event's userId
e1[last].timestamp // Last event's timestamp
e1[2].ipAddress   // Third event's IP address
```

**Test Coverage**: 14 comprehensive unit tests covering:
- Numeric index access (`e[0]`, `e[1]`, `e[2]`)
- Last keyword (`e[last]`)
- Out-of-bounds handling (returns `None`)
- Multiple stream positions
- Different attribute types (INT, LONG, DOUBLE, STRING)

#### 2. StateEvent Event Chain Infrastructure - COMPLETE

```rust
// src/core/event/state/state_event.rs

impl StateEvent {
    /// Get all events in chain at position - WORKS
    pub fn get_event_chain(&self, position: usize) -> Vec<&StreamEvent>

    /// Count events in chain at position - WORKS
    pub fn count_events_at(&self, position: usize) -> usize

    /// Add event to chain (for count quantifiers) - WORKS
    pub fn add_event(&mut self, position: usize, event: StreamEvent)
}
```

#### 3. MultiValueVariableFunctionExecutor - EXISTS

This executor already collects ALL attribute values from an event chain:

```rust
// src/core/executor/multi_value_variable_function_executor.rs:28-48

fn execute(&self, event_opt: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
    let state_event = complex_event.as_any().downcast_ref::<StateEvent>()?;
    let mut results: Vec<AttributeValue> = Vec::new();

    // Walks entire chain, collects all attribute values
    while let Some(stream_event) = stream_event_opt {
        if let Some(val) = stream_event.get_attribute_by_position(&self.attribute_position) {
            results.push(val.clone());
        }
        stream_event_opt = /* walk to next */;
    }
    Some(AttributeValue::Object(Some(Box::new(results))))
}
```

**This can be leveraged** as a building block for collection aggregations.

#### 4. Collection Aggregation Executors - FULLY IMPLEMENTED ✅

All collection aggregation executors are now implemented in `src/core/executor/collection_aggregation_executor.rs`:

**Implemented Executors:**
- `CollectionCountExecutor` - for `count(e1)` - counts events in collection
- `CollectionSumExecutor` - for `sum(e1.price)` - sums attribute over collection
- `CollectionAvgExecutor` - for `avg(e1.price)` - averages attribute over collection
- `CollectionMinMaxExecutor` - for `min(e1.price)` / `max(e1.price)` - finds min/max
- `CollectionStdDevExecutor` - for `stdDev(e1.price)` - population standard deviation (bonus)

**Key Features:**
- Stateless batch computation (no incremental state needed)
- Proper null handling (SQL-like: nulls are excluded)
- Type preservation for min/max/sum (INT→INT, LONG→LONG, etc.)
- Comprehensive test coverage (42 tests)

**Test Coverage:**
- Basic aggregation tests for each executor
- Empty collection handling
- Single value collections
- Null value handling
- Mixed type handling (INT, LONG, DOUBLE)
- Multiple stream positions
- Edge cases (negative values, out of bounds, etc.)

### What's Missing ❌

#### Parser Support

No SQL parser for collection aggregation syntax:
- `count(e1)` vs `count(column_name)` distinction
- `avg(e1.price)` collection attribute aggregation

---

## Key Differences from Window Aggregations

### Window Aggregations (Implemented)

- Operate over streams of events across time
- Maintain incremental state (e.g., running sum, count)
- Use `process_add()` / `process_remove()` semantics
- State holders: `CountAggregatorStateHolder`, `SumAggregatorStateHolder`, etc.
- Used in: `SELECT count() WINDOW timeLength(5 minutes)`
- Location: `src/core/query/selector/attribute/aggregator/`

### Collection Aggregations (Executors NOT Implemented)

- Operate over bounded event collections within a single StateEvent
- **Batch computation** - no incremental state needed
- Collection is complete at evaluation time
- Should work like: count events in e1 chain, sum attribute over e1 chain
- Needed for: `SELECT count(e1), avg(e1.price)` where e1 is from pattern

| Aspect | Window Aggregators | Collection Aggregators (Needed) |
|--------|-------------------|-------------------------------|
| Input | Single values | Complete event chain |
| Semantics | Incremental (add/remove) | Batch computation |
| State | Maintains running state | Stateless |
| Trait | `AttributeAggregatorExecutor` | New `ExpressionExecutor` impl |
| Use case | Streaming windows | Pattern count quantifiers |

---

## Architecture Requirements

### Recommended Approach: Dedicated Collection Executors

Create new executor types specifically for collection aggregations (separate from window aggregators):

```rust
// New file: src/core/executor/collection_aggregation_executor.rs

use crate::core::event::complex_event::ComplexEvent;
use crate::core::event::state::state_event::StateEvent;
use crate::core::event::value::AttributeValue;
use crate::core::executor::expression_executor::ExpressionExecutor;

/// Executor for count(e1) - counts events in collection
#[derive(Debug, Clone)]
pub struct CollectionCountExecutor {
    /// Position in StateEvent.stream_events[] (e1=0, e2=1, ...)
    pub chain_index: usize,
}

impl ExpressionExecutor for CollectionCountExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let state_event = event?.as_any().downcast_ref::<StateEvent>()?;
        let count = state_event.count_events_at(self.chain_index);
        Some(AttributeValue::Long(count as i64))
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::LONG
    }
    // ...
}

/// Executor for sum(e1.price) - sums attribute over collection
#[derive(Debug, Clone)]
pub struct CollectionSumExecutor {
    pub chain_index: usize,
    pub attribute_position: [i32; 2],  // [data_type_index, attr_index]
    pub return_type: ApiAttributeType,
}

impl ExpressionExecutor for CollectionSumExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let state_event = event?.as_any().downcast_ref::<StateEvent>()?;
        let chain = state_event.get_event_chain(self.chain_index);

        if chain.is_empty() {
            return None;
        }

        let mut sum = 0.0f64;
        for stream_event in chain {
            if let Some(val) = stream_event.get_attribute_by_position(&self.attribute_position) {
                sum += value_as_f64(&val).unwrap_or(0.0);
            }
        }

        match self.return_type {
            ApiAttributeType::LONG => Some(AttributeValue::Long(sum as i64)),
            _ => Some(AttributeValue::Double(sum)),
        }
    }
    // ...
}

/// Executor for avg(e1.price) - averages attribute over collection
#[derive(Debug, Clone)]
pub struct CollectionAvgExecutor {
    pub chain_index: usize,
    pub attribute_position: [i32; 2],
}

impl ExpressionExecutor for CollectionAvgExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let state_event = event?.as_any().downcast_ref::<StateEvent>()?;
        let chain = state_event.get_event_chain(self.chain_index);

        if chain.is_empty() {
            return None;
        }

        let mut sum = 0.0f64;
        let mut count = 0usize;

        for stream_event in chain {
            if let Some(val) = stream_event.get_attribute_by_position(&self.attribute_position) {
                if let Some(num) = value_as_f64(&val) {
                    sum += num;
                    count += 1;
                }
            }
        }

        if count > 0 {
            Some(AttributeValue::Double(sum / count as f64))
        } else {
            None
        }
    }
    // ...
}

/// Executor for min(e1.price) / max(e1.price)
#[derive(Debug, Clone)]
pub struct CollectionMinMaxExecutor {
    pub chain_index: usize,
    pub attribute_position: [i32; 2],
    pub is_min: bool,  // true for min, false for max
}

impl ExpressionExecutor for CollectionMinMaxExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let state_event = event?.as_any().downcast_ref::<StateEvent>()?;
        let chain = state_event.get_event_chain(self.chain_index);

        if chain.is_empty() {
            return None;
        }

        let mut result: Option<f64> = None;

        for stream_event in chain {
            if let Some(val) = stream_event.get_attribute_by_position(&self.attribute_position) {
                if let Some(num) = value_as_f64(&val) {
                    result = Some(match result {
                        None => num,
                        Some(current) => {
                            if self.is_min { current.min(num) } else { current.max(num) }
                        }
                    });
                }
            }
        }

        result.map(AttributeValue::Double)
    }
    // ...
}
```

### Why Dedicated Executors (Not Extending Window Aggregators)

**Pros of Dedicated Executors:**
- Clean separation from window aggregators
- No risk of breaking existing window aggregation functionality
- Simpler to test in isolation
- Clearer semantics (batch vs incremental)
- No need for `process_add`/`process_remove` complexity

**Cons:**
- Some code duplication with window aggregators (minimal)

---

## Required Aggregation Functions

Minimum set for Grammar V1.2:

1. `count(e)` - Count events in collection
2. `sum(e.attribute)` - Sum attribute over collection
3. `avg(e.attribute)` - Average attribute over collection
4. `min(e.attribute)` - Minimum attribute in collection
5. `max(e.attribute)` - Maximum attribute in collection

---

## Implementation Plan

### Phase 1: Collection Aggregation Executors (2-3 days)

1. Create `src/core/executor/collection_aggregation_executor.rs`
2. Implement executors:
   - `CollectionCountExecutor`
   - `CollectionSumExecutor`
   - `CollectionAvgExecutor`
   - `CollectionMinMaxExecutor`
3. Add to executor module exports
4. Comprehensive unit tests

**Note**: Infrastructure already exists (`StateEvent.get_event_chain()`, `count_events_at()`), so this is straightforward.

### Phase 2: Parser Integration (1-2 days)

1. Parse `count(e1)` - detect when argument is collection alias
2. Parse `avg(e1.price)` - detect collection attribute aggregation
3. Distinguish collection references from regular variables
4. Wire to collection executors in compiler
5. Validation: Reject invalid uses (e.g., `count(e1.price)` should error)

### Phase 3: Testing (1 day)

1. Unit tests for each executor
2. Integration tests with full pattern queries
3. Edge cases (nulls, empty collections, type coercion)

**Total Effort**: 4-6 days (reduced from original 7-10 days estimate due to existing infrastructure)

---

## Test Coverage Needed

### Basic count tests:

```rust
#[test]
fn test_collection_count_exact() {
    // Pattern: e1{3}
    // Events: [e1, e2, e3]
    // Assert: count(e1) == 3

    let executor = CollectionCountExecutor { chain_index: 0 };
    let state_event = create_state_event_with_n_events(0, 3);

    let result = executor.execute(Some(&state_event as &dyn ComplexEvent));
    assert_eq!(result, Some(AttributeValue::Long(3)));
}

#[test]
fn test_collection_count_range() {
    // Pattern: e1{2,5}
    // Input: 4 events
    // Assert: count(e1) == 4
}

#[test]
fn test_collection_count_empty() {
    // Empty collection
    // Assert: count(e1) == 0 or None
}
```

### Attribute aggregation tests:

```rust
#[test]
fn test_sum_over_collection() {
    // Pattern: e1{3} with values [10, 20, 30]
    // Assert: sum(e1.value) == 60
}

#[test]
fn test_avg_over_collection() {
    // Pattern: e1{3} with prices [100.0, 110.0, 120.0]
    // Assert: avg(e1.price) == 110.0
}

#[test]
fn test_min_over_collection() {
    // Pattern: e1{5} with temps [10, 5, 20, 15, 8]
    // Assert: min(e1.temp) == 5
}

#[test]
fn test_max_over_collection() {
    // Pattern: e1{5} with temps [10, 5, 20, 15, 8]
    // Assert: max(e1.temp) == 20
}
```

### Edge cases:

```rust
#[test]
fn test_collection_aggregation_null_handling() {
    // Some events have null attributes
    // Assert: Aggregation ignores nulls (like SQL)
}

#[test]
fn test_collection_aggregation_type_coercion() {
    // Mix of INT and LONG values
    // Assert: Proper numeric promotion
}

#[test]
fn test_collection_aggregation_single_event() {
    // Pattern: e1{1} with single event
    // Assert: avg == value, min == max == value
}
```

---

## Parser Gap

Currently NO parser support for collection aggregations. The parser needs to:

1. **Recognize collection alias** - `count(e1)` where `e1` is a pattern alias
2. **Parse collection attribute access** - `avg(e1.price)`
3. **Distinguish from window aggregations** - Context-aware parsing
4. **Validate usage**:
   - `count(e1)` - valid (count events)
   - `count(e1.price)` - invalid (cannot count attribute)
   - `avg(e1)` - invalid (needs attribute for numeric aggregation)
   - `avg(e1.price)` - valid

---

## Blocker Status

This is a P1 feature. Current blockers:

1. ~~No StateEvent event chain infrastructure~~ ✅ RESOLVED
2. ~~No array access executor~~ ✅ RESOLVED (IndexedVariableExecutor works)
3. ~~No collection aggregation executors~~ ✅ RESOLVED (CollectionAggregationExecutors implemented)
4. ❌ No parser for collection aggregation syntax
5. ~~No tests for collection aggregations~~ ✅ RESOLVED (42 comprehensive tests)

---

## Dependencies

### Already Complete ✅

- `StateEvent.get_event_chain()` - returns `Vec<&StreamEvent>`
- `StateEvent.count_events_at()` - returns count
- `IndexedVariableExecutor` - array access works
- `MultiValueVariableFunctionExecutor` - collects all values (can be leveraged)

### Required for Collection Aggregations

- Collection aggregation executors (this feature)
- Parser support (separate work item)

---

## Recommendation

**IMPLEMENT** collection aggregation executors now. Reasons:

1. Infrastructure is already complete (`StateEvent`, event chains, etc.)
2. Executors can be tested independently of parser
3. Low risk - doesn't affect existing window aggregations
4. Enables future parser work to wire directly to working executors

**Implementation order:**
1. `CollectionCountExecutor` (simplest, uses existing `count_events_at()`)
2. `CollectionSumExecutor` and `CollectionAvgExecutor`
3. `CollectionMinMaxExecutor`
4. Tests for all executors

Parser integration can follow once pattern grammar parser work begins.

---

## Conclusion

Collection aggregations are **EXECUTORS COMPLETE**:

| Feature | Status |
|---------|--------|
| Array access (`e[0].attr`, `e[last].attr`) | ✅ RUNTIME COMPLETE |
| StateEvent event chain infrastructure | ✅ COMPLETE |
| Collection aggregation executors | ✅ COMPLETE |
| Parser support | ❌ NOT IMPLEMENTED |

**Current Status**: All executors implemented and tested, parser integration pending
**Priority**: P2 (parser integration)
**Remaining Effort**: 1-2 days (parser integration only)
**Next Step**: Implement parser support to wire SQL syntax to executors

---

**Document Version**: 3.0
**Last Updated**: 2025-11-27
**Change Summary**:
- Collection aggregation executors FULLY IMPLEMENTED
- 5 executors: Count, Sum, Avg, MinMax, StdDev
- 42 comprehensive unit tests
- Proper null handling, type preservation
- Ready for parser integration
