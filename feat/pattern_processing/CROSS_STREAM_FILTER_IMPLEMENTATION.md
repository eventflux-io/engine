# Cross-Stream References in Pattern Filters

**Status**: Runtime Complete | Parser Not Implemented
**Date**: 2025-11-23
**Last Updated**: 2025-11-23

## Implementation Status

✅ **Runtime**: Complete and tested (6 tests passing)
❌ **Parser**: Not implemented (parser is syntactic sugar on top of programmatic API)

## Overview

Pattern filter conditions can access previous events in the StateEvent context, enabling conditional matching where later events compare attributes against earlier events.

**Example**: `e2[price > e1.price * 1.1]` - e2 must have price > 10% higher than e1

## Architecture

### Condition Function Signature

**File**: `src/core/query/input/stream/state/stream_pre_state_processor.rs`

**Signature**:
```rust
condition_fn: Option<Arc<dyn Fn(&StateEvent) -> bool + Send + Sync>>
```

Filter receives full `StateEvent` with all matched events (e1, e2, e3, ...). Current event is already added to StateEvent at position `self.state_id` before filter evaluation.

### Key Implementation Points

**Lines**: 535-548

```rust
// Ensure StateEvent has enough positions (critical bug fix)
candidate_state.expand_to_size(self.state_id + 1);

// Add current event to StateEvent at this processor's position
candidate_state.set_event(self.state_id, cloned_stream);

// Evaluate filter with complete StateEvent
if self.matches_condition(&candidate_state) {
    // ... process match
}
```

**Critical**: Event is added to StateEvent BEFORE condition check, ensuring filters can access it.

###  matches_condition Method

**Lines**: 269-274

```rust
fn matches_condition(&self, state_event: &StateEvent) -> bool {
    match &self.condition_fn {
        Some(f) => f(state_event),
        None => true, // No condition = match all
    }
}
```

## Test Coverage

**File**: `tests/pattern_filter_cross_stream_test.rs`
**Total**: 6 tests, all passing

| Test | Pattern | Coverage |
|------|---------|----------|
| `test_filter_with_cross_stream_reference_simple` | `e2[price > e1.price]` | Basic cross-stream comparison |
| `test_filter_cross_stream_percentage` | `e2[price > e1.price * 1.1]` | Arithmetic in filter |
| `test_filter_cross_stream_string_equality` | `e2[userId == e1.userId]` | String attribute matching |
| `test_filter_cross_stream_three_events` | `e3[value > e1.value AND value > e2.value]` | Multi-stream compound conditions |
| `test_filter_cross_stream_null_handling` | Missing stream reference | NULL safety |
| `test_filter_without_cross_stream_reference` | `e2[price > 100]` | Backward compatibility |

## Programmatic Usage

**Simple cross-stream filter**:
```rust
// Pattern: e1=StockPrice -> e2=StockPrice[price > e1.price]
e2_processor.set_condition(|state_event| {
    // Access e1 from position 0
    if let Some(e1) = state_event.get_stream_event(0) {
        if let Some(AttributeValue::Float(e1_price)) = e1.before_window_data.get(0) {
            // Access e2 (current event) from position 1
            if let Some(e2) = state_event.get_stream_event(1) {
                if let Some(AttributeValue::Float(e2_price)) = e2.before_window_data.get(0) {
                    return e2_price > e1_price;
                }
            }
        }
    }
    false
});
```

**Multi-stream filter**:
```rust
// Pattern: e1 -> e2 -> e3[value > e1.value AND value > e2.value]
e3_processor.set_condition(|state_event| {
    if let Some(e3) = state_event.get_stream_event(2) {
        if let Some(AttributeValue::Int(e3_val)) = e3.before_window_data.get(0) {
            // Check against e1
            if let Some(e1) = state_event.get_stream_event(0) {
                if let Some(AttributeValue::Int(e1_val)) = e1.before_window_data.get(0) {
                    if e3_val <= e1_val { return false; }
                }
            }
            // Check against e2
            if let Some(e2) = state_event.get_stream_event(1) {
                if let Some(AttributeValue::Int(e2_val)) = e2.before_window_data.get(0) {
                    return e3_val > e2_val;
                }
            }
        }
    }
    false
});
```

## Implementation Files

| Component | File | Lines |
|-----------|------|-------|
| Condition signature | `src/core/query/input/stream/state/stream_pre_state_processor.rs` | 269-274, 535-548 |
| Tests | `tests/pattern_filter_cross_stream_test.rs` | (6 tests) |

## Design Decisions

1. **Single Parameter (StateEvent Only)**: Clean, single source of truth
   - StateEvent contains ALL events including current at position `state_id`
   - No redundancy - current event is in StateEvent before filter

2. **Automatic Expansion** (Line 541): StateEvents grow automatically
   - Prevents silent failures in multi-stream patterns
   - Critical fix: `expand_to_size()` before `set_event()`

3. **NULL Safety**: All access returns Option
   - Missing streams return None
   - Filters choose how to handle missing data

4. **Event Ordering Guarantee**: Current event added BEFORE filter check
   - Line 544: `set_event()` adds event
   - Line 548: `matches_condition()` evaluates with complete StateEvent

## Grammar Design (Not Implemented)

❌ **Parser**: Not implemented (parser is syntactic sugar on top of programmatic API)

### Intended Syntax

```sql
-- Cross-stream filter in pattern matching
FROM PATTERN (
    e1=StockPrice[symbol == 'AAPL'] ->
    e2=StockPrice[symbol == 'AAPL' AND price > e1.price * 1.1]
)
SELECT e1.price as startPrice, e2.price as endPrice
INSERT INTO PriceJumps;

-- Multi-stream cross-reference
FROM PATTERN (
    e1=Login ->
    e2=DataAccess[userId == e1.userId AND bytes > 1000000] ->
    e3=Logout[userId == e1.userId AND timestamp - e1.timestamp > 3600000]
)
SELECT
    e1.userId,
    e2.bytes,
    e3.timestamp - e1.timestamp as sessionDuration
```

### Supported Filter Patterns (Design)

| Pattern | Example |
|---------|---------|
| Simple comparison | `e2[price > e1.price]` |
| Arithmetic | `e2[price > e1.price * 1.1]` |
| Multiple references | `e3[value > e1.value AND value > e2.value]` |
| String equality | `e2[userId == e1.userId]` |
| Attribute chains | `e2[user.location == e1.user.location]` |

## Performance

- **Time**: O(1) to access any stream in StateEvent
- **Space**: O(n) where n = number of streams in pattern
- **No Allocation**: Uses existing StateEvent, just expands vector
