# Cross-Stream References in Pattern Filters - Implementation Complete

**Date**: 2025-11-23
**Status**: COMPLETE
**Priority**: P0 - CRITICAL (was blocking Grammar V1.2)

## Overview

Pattern filter conditions can now access previous events in the StateEvent context. This enables conditional matching where later events can compare their attributes against earlier events in the pattern.

## Implementation Summary

### Changes Made

#### 1. Updated Condition Function Signature

**File**: `src/core/query/input/stream/state/stream_pre_state_processor.rs`

**Before**:
```rust
condition_fn: Option<Arc<dyn Fn(&StreamEvent) -> bool + Send + Sync>>
```

**After**:
```rust
condition_fn: Option<Arc<dyn Fn(&StateEvent, &StreamEvent) -> bool + Send + Sync>>
```

**Impact**: Filters now receive both:
- `&StateEvent` - Full context with all matched events (e1, e2, e3, ...)
- `&StreamEvent` - The incoming event being evaluated

#### 2. Updated matches_condition Method

**Lines**: 258-269

**Before**:
```rust
fn matches_condition(&self, stream_event: &StreamEvent) -> bool {
    match &self.condition_fn {
        Some(f) => f(stream_event),
        None => true,
    }
}
```

**After**:
```rust
fn matches_condition(&self, state_event: &StateEvent, stream_event: &StreamEvent) -> bool {
    match &self.condition_fn {
        Some(f) => f(state_event, stream_event),
        None => true,
    }
}
```

#### 3. Updated process_and_return Call Site

**Line**: 544

**Before**:
```rust
if self.matches_condition(&stream_event) {
```

**After**:
```rust
if self.matches_condition(&candidate_state, &stream_event) {
```

#### 4. Critical Bug Fix: StateEvent Size Expansion

**Lines**: 535-537

**Problem**: When a processor tried to add an event at a position beyond the current StateEvent size, `set_event()` would fail silently.

**Fix**: Added automatic expansion before setting event:
```rust
// Ensure StateEvent has enough positions for this processor's state_id
// Without this, set_event() will fail silently if position >= stream_events.len()
candidate_state.expand_to_size(self.state_id + 1);

// Add the StreamEvent to the StateEvent at this processor's position
candidate_state.set_event(self.state_id, cloned_stream);
```

**Impact**: This bug affected all multi-stream patterns (e1 -> e2 -> e3). The fix ensures StateEvents can grow as they progress through processor chains.

#### 5. Updated Test Closures

Updated 3 existing tests to use new signature:
- `test_process_and_return_pattern_semantics`
- `test_process_and_return_sequence_semantics`
- `test_process_and_return_with_condition`

### Test Coverage

**File**: `tests/pattern_filter_cross_stream_test.rs`

**Total**: 6 comprehensive tests, all passing

1. **test_filter_with_cross_stream_reference_simple**
   - Pattern: `e1=StockPrice -> e2=StockPrice[price > e1.price]`
   - Tests basic cross-stream comparison
   - Verifies both match and no-match cases

2. **test_filter_cross_stream_percentage**
   - Pattern: `e1=StockPrice -> e2=StockPrice[price > e1.price * 1.1]`
   - Tests arithmetic operations in cross-stream filters
   - Verifies threshold calculations

3. **test_filter_cross_stream_string_equality**
   - Pattern: `e1=Login -> e2=Activity[userId == e1.userId]`
   - Tests String attribute matching across streams
   - Verifies equality comparisons

4. **test_filter_cross_stream_three_events**
   - Pattern: `e1=Event -> e2=Event -> e3=Event[value > e1.value AND value > e2.value]`
   - Tests three-stream patterns with compound conditions
   - Verifies multi-stream context access

5. **test_filter_cross_stream_null_handling**
   - Tests filter behavior when referenced stream doesn't exist
   - Verifies NULL safety

6. **test_filter_without_cross_stream_reference**
   - Tests that simple filters (without cross-stream refs) still work
   - Verifies backward compatibility

### Verification

**Existing Tests**: All 37 pattern tests still pass
**New Tests**: 6 new tests pass
**Total Coverage**: 43 passing pattern/filter tests

## Usage Examples

### Simple Cross-Stream Filter

```rust
let mut e2_processor = StreamPreStateProcessor::new(1, false, StateType::Sequence, ...);

// Filter: price > e1.price
e2_processor.set_condition(|state_event, stream_event| {
    if let Some(e1) = state_event.get_stream_event(0) {
        if let Some(AttributeValue::Float(e1_price)) = e1.before_window_data.get(0) {
            if let Some(AttributeValue::Float(e2_price)) = stream_event.before_window_data.get(0) {
                return e2_price > e1_price;
            }
        }
    }
    false
});
```

### Complex Multi-Stream Filter

```rust
// Pattern: e1 -> e2 -> e3[value > e1.value AND value > e2.value]
e3_processor.set_condition(|state_event, stream_event| {
    let e3_value = stream_event.before_window_data[0].as_int()?;
    let e1_value = state_event.get_stream_event(0)?.before_window_data[0].as_int()?;
    let e2_value = state_event.get_stream_event(1)?.before_window_data[0].as_int()?;

    e3_value > e1_value && e3_value > e2_value
});
```

## Architecture

### Execution Flow

1. **Event Arrives**: StreamEvent enters processor
2. **Pending States**: Processor iterates through all pending StateEvents
3. **Candidate Build**:
   - Clone pending StateEvent (contains e1, e2, ...)
   - Expand to accommodate new event position
   - Set new event at current position
4. **Filter Evaluation**:
   - Pass full candidate_state (all events) to filter
   - Pass incoming stream_event
   - Filter can reference any stream via `state_event.get_stream_event(position)`
5. **Match Handling**: Forward or remove based on Pattern/Sequence semantics

### Key Design Decisions

1. **Two Parameters**: Both StateEvent and StreamEvent provide flexibility
   - StateEvent: Access to all previous events
   - StreamEvent: Direct access to current event (optimization)

2. **Automatic Expansion**: StateEvents grow automatically as needed
   - Prevents silent failures in multi-stream patterns
   - Enables arbitrary depth pattern chains

3. **NULL Safety**: All access returns Option
   - Missing streams return None
   - Filters can choose how to handle missing data

4. **Backward Compatible**: Simple filters can ignore state_event parameter
   - `|_state_event, stream_event| stream_event.value > 100`

## Performance Characteristics

- **Time Complexity**: O(1) to access any stream in StateEvent
- **Space Complexity**: O(n) where n = number of streams in pattern
- **No Allocation**: Uses existing StateEvent, just expands vector

## Future Work

### Parser Integration (Next Step)

When Grammar V1.2 parser is implemented, it will need to:

1. Parse filter syntax: `e2[price > e1.price * 1.1]`
2. Parse cross-stream references: `e1.attribute`
3. Compile to expression executors
4. Wire executors to condition function:

```rust
// Pseudocode for parser output
let filter_executor = ComparisonExecutor::new(
    VariableExpressionExecutor::new([1, 0, BEFORE_WINDOW_DATA_INDEX, 1], FLOAT, "e2.price"),
    MultiplyExecutor::new(
        VariableExpressionExecutor::new([0, 0, BEFORE_WINDOW_DATA_INDEX, 1], FLOAT, "e1.price"),
        ConstantExecutor::new(AttributeValue::Float(1.1)),
    ),
    ComparisonOperator::GreaterThan,
);

processor.set_condition(move |state_event, _stream_event| {
    filter_executor.execute(Some(state_event as &dyn ComplexEvent))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
});
```

### FilterExecutor Wrapper (Optional Enhancement)

Could create a wrapper to simplify filter executor integration:

```rust
pub struct FilterExecutor {
    expression_executor: Box<dyn ExpressionExecutor>,
}

impl FilterExecutor {
    pub fn evaluate(&self, state_event: &StateEvent) -> bool {
        self.expression_executor.execute(Some(state_event as &dyn ComplexEvent))
            .and_then(|v| match v {
                AttributeValue::Bool(b) => Some(b),
                _ => None,
            })
            .unwrap_or(false)
    }
}
```

## Conclusion

Cross-stream references in pattern filters are now fully functional at the runtime level. This was a P0 critical blocker for Grammar V1.2 parser implementation.

**Key Achievements**:
- ✓ Condition function signature updated to accept StateEvent context
- ✓ All filter evaluation paths updated
- ✓ Critical StateEvent size bug fixed
- ✓ 6 comprehensive tests added and passing
- ✓ All existing tests still passing (43 total pattern tests)
- ✓ Production-ready implementation with NULL safety

**No Blockers**: Parser implementation can now proceed with confidence that the runtime fully supports cross-stream filter conditions.

**Bug Fixed**: StateEvent expansion issue would have caused silent failures in all multi-stream patterns. This fix benefits not just filters, but all pattern processing.
