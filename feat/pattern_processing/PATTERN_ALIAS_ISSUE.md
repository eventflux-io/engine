# Pattern Alias Runtime Resolution Issue

## Status: FIXED ✅

**Date Identified**: 2024-12-19
**Date Fixed**: 2024-12-20
**Component**: Runtime Expression Parser + SequenceProcessor + SameStreamSequenceAdapter
**Severity**: Resolved - Both two-stream and same-stream patterns now work correctly

## Summary

Pattern aliases (e.g., `e1`, `e2` in `FROM PATTERN (e1=Stream -> e2=Stream)`) now work correctly for:
- **Two-stream patterns** (A -> B) - Fixed via alias registration in ExpressionParserContext
- **Same-stream patterns** (A -> A) - Fixed via SameStreamSequenceAdapter

## What Was Fixed (2024-12-19)

### Root Cause 1: Pattern aliases not registered in ExpressionParserContext

The `query_parser.rs` was using stream **names** (e.g., "RawTrades") in `stream_meta_map` and `stream_positions` but not the **aliases** (e.g., "e1", "e2"). When SELECT referenced `e1.symbol`, the expression parser couldn't find `e1` in either map.

### Fix Applied

1. **Added alias extraction function** in `query_parser.rs`:
   ```rust
   fn extract_stream_state_with_count_and_alias(se: &StateElement)
       -> Option<(StreamStateElement, min, max, Option<String>)>
   ```
   This extracts the `stream_reference_id` (alias) from `SingleInputStream`.

2. **Updated `StateRuntimeKind` enum** to include alias fields:
   ```rust
   enum StateRuntimeKind {
       Sequence {
           first_id: String,
           second_id: String,
           first_alias: Option<String>,  // Pattern alias (e.g., "e1")
           second_alias: Option<String>, // Pattern alias (e.g., "e2")
           // ...
       },
       // Similar for Logical
   }
   ```

3. **Registered aliases in both maps**:
   ```rust
   // stream_meta_map
   if let Some(ref alias) = first_alias_opt {
       stream_meta_map.insert(alias.clone(), Arc::clone(&first_meta));
   }
   if let Some(ref alias) = second_alias_opt {
       stream_meta_map.insert(alias.clone(), Arc::clone(&second_meta));
   }

   // stream_positions
   if let Some(ref alias) = first_alias_opt {
       stream_positions_map.insert(alias.clone(), 0);
   }
   if let Some(ref alias) = second_alias_opt {
       stream_positions_map.insert(alias.clone(), 1);
   }
   ```

4. **Proper offset application**: `second_meta` has `apply_attribute_offset(first_len)` applied, so `e2.price` correctly resolves to the offset index in the flattened event.

### Root Cause 2: Same-stream patterns caused self-matching

When both pattern elements reference the same stream (e.g., `e1=RawTrades -> e2=RawTrades`), both `first_junction` and `second_junction` resolve to the same junction. This caused every event to be delivered to BOTH buffers, resulting in self-matches.

### Fix Applied: SameStreamSequenceAdapter

Added a `SameStreamSequenceAdapter` that:
1. Subscribes once to the junction when `first_id == second_id`
2. Routes the **first event** to `first_side` only (adds to first_buffer)
3. Routes **subsequent events** to `second_side` only (adds to second_buffer, triggers matching)

This adapter is used for **both** pattern types:
- **Sequence patterns** (A -> B): Ensures temporal ordering - first event to first_buffer, subsequent to second_buffer
- **Logical patterns** (A AND B, A OR B): Prevents self-matching and duplicate outputs

```rust
/// Adapter for same-stream sequence/pattern processing.
/// Routes first event to first_side only, subsequent events to second_side only.
/// This prevents self-matching that occurs when both sides subscribe to the same junction.
struct SameStreamSequenceAdapter {
    first_side: Arc<Mutex<dyn Processor>>,
    second_side: Arc<Mutex<dyn Processor>>,
    first_event_received: AtomicBool,
}
```

## Working Examples

### Two Different Streams

```sql
CREATE STREAM StreamA (price DOUBLE, symbol VARCHAR);
CREATE STREAM StreamB (price DOUBLE, symbol VARCHAR);
CREATE STREAM TrendSignal (first_symbol VARCHAR, first_price DOUBLE, second_price DOUBLE, change DOUBLE);

-- This query works correctly
INSERT INTO TrendSignal
SELECT
    e1.symbol AS first_symbol,
    e1.price AS first_price,
    e2.price AS second_price,
    e2.price - e1.price AS change
FROM PATTERN (e1=StreamA -> e2=StreamB);
```

Output: `[AAPL, 100.0, 110.0, 10.0]`

### Same Stream (Now Working!)

```sql
CREATE STREAM RawTrades (price DOUBLE, symbol VARCHAR);
CREATE STREAM TrendSignal (first_symbol VARCHAR, first_price DOUBLE, second_price DOUBLE, change DOUBLE);

-- This query now works correctly
INSERT INTO TrendSignal
SELECT e1.symbol, e1.price, e2.price, e2.price - e1.price
FROM PATTERN (e1=RawTrades -> e2=RawTrades);
```

With events: `[100.0, "AAPL"]` then `[110.0, "AAPL"]`
Output: `[AAPL, 100.0, 110.0, 10.0]` ✅

## Files Changed

| File | Change |
|------|--------|
| `src/core/util/parser/query_parser.rs` | Added alias extraction, registration, and SameStreamSequenceAdapter |

## Tests

| Test | Status | Purpose |
|------|--------|---------|
| `pattern_alias_two_streams` | ✅ PASSES | Verifies pattern alias access works with two different streams |
| `pattern_alias_same_stream` | ✅ PASSES | Verifies pattern alias access works with same stream |
| `sequence_api` | ✅ PASSES | Verifies basic sequence API functionality (no regression) |
| `sequence_with_timeout` | ✅ PASSES | Verifies sequence with timeout (no regression) |
| `kleene_star_pattern` | ✅ PASSES | Verifies Kleene star patterns (no regression) |

## Architecture Notes

The fix maintains compatibility with the existing `SequenceProcessor` architecture by adding the `SameStreamSequenceAdapter` at the subscription level. This is a minimal, targeted fix that:

1. Doesn't require migrating to `StreamPreStateProcessor`/`PatternChainBuilder`
2. Works with the existing flattened `StreamEvent` model
3. Only activates for same-stream patterns (`first_id == second_id`)
4. Correctly handles the event routing to prevent self-matching

## Related Files

| File | Purpose |
|------|---------|
| `src/core/query/input/stream/state/sequence_processor.rs` | Current processor for sequence/pattern matching |
| `src/core/query/input/stream/state/stream_pre_state_processor.rs` | Alternative StateEvent-based architecture |
| `src/core/query/input/stream/state/pattern_chain_builder.rs` | Helper for building pattern processor chains |
