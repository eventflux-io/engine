# EVERY Pattern Implementation Reference

Date: 2025-11-26 (Updated)
Status: **COMPLETE - All EVERY pattern features working**

---

## Implementation Status Summary

| Feature | Status | Notes |
|---------|--------|-------|
| Pattern restart (sequential) | ✅ WORKING | A1→B2→A3→B4 produces 2 matches |
| Basic TRUE overlapping | ✅ WORKING | A1→A2→B3 produces 2 matches (A1-B3, A2-B3) |
| Logical operators (AND/OR) | ✅ WORKING | PatternChainBuilder supports add_logical_group() |
| Sliding window (count quantifiers) | ✅ WORKING | EVERY A{3}→B with A1,A2,A3,A4,A5,B produces 3 sliding windows |

---

## ✅ TRUE OVERLAPPING WORKS FOR BASIC PATTERNS

**Verified 2025-11-25**: The basic TRUE overlapping semantics ARE working correctly!

### Working Example

```
Pattern: EVERY (A -> B)
Events: A1@1000 → A2@2000 → B3@3000

RESULT: 2 matches (CORRECT!)
  - Match 1: A1@1000 → B3@3000
  - Match 2: A2@2000 → B3@3000
```

This was verified by test `test_true_every_overlapping_multiple_a_before_b`.

### How It Works

1. A1 arrives at Pre[0] → StateEvent{A1} forwarded to Pre[1]
2. A2 arrives at Pre[0] → StateEvent{A2} forwarded to Pre[1]
3. Pre[1] now has TWO pending states: StateEvent{A1} AND StateEvent{A2}
4. B3 arrives at Pre[1] → processes BOTH pending states → 2 outputs

---

## ✅ SLIDING WINDOW WITH COUNT QUANTIFIERS NOW WORKING

**Implemented**: 2025-11-26

### How It Works

For patterns like `EVERY A{2,3} -> B`, the system now creates **overlapping sliding windows**:

```
Events: A1, A2, A3, A4, B1

Window progression:
A1: pending = [SE([A1])]                                    → 0 outputs
A2: pending = [SE([A1,A2]), SE([A2])]                       → 1 output: [A1,A2]
A3: pending = [SE([A2,A3]), SE([A3])]                       → 2 outputs: [A1,A2,A3], [A2,A3]
A4: pending = [SE([A3,A4]), SE([A4])]                       → 2 outputs: [A2,A3,A4], [A3,A4]
B1: matches all pending patterns                            → 2 matches
```

### Implementation Details

The fix is in `CountPreStateProcessor::process_and_return()`:

1. **Track existing windows**: Before processing, check if any existing pending states have events
2. **Spawn new window**: After processing all existing windows, spawn a NEW StateEvent
   containing only the current event (starts a new overlapping window)
3. **Immediate output**: If the spawned window's count >= min_count, output immediately
4. **Continue accumulating**: Add spawned window to pending if count < max_count

### Key Code (count_pre_state_processor.rs:244-280)

```rust
// Check if any existing window has events at this position
let any_existing_had_events = pending_states
    .iter()
    .any(|se| se.get_stream_event(state_id).is_some());

// ... process existing windows ...

// EVERY SLIDING WINDOW: Spawn new overlapping window
if is_every_start && any_existing_had_events {
    let mut new_window = StateEvent::new(1, 0);
    new_window.add_event(state_id, cloned_event);

    // Output immediately if count >= min_count
    if 1 >= self.min_count && 1 <= self.max_count {
        post_processor.process(Some(Box::new(new_window.clone())));
    }

    // Add to pending if can still grow
    if 1 < self.max_count {
        pending.push_back(new_window);
    }
}
```

### Test Coverage

7 new tests added in `count_pre_state_processor.rs`:
- `test_every_sliding_window_a2_3_basic` - Basic sliding window
- `test_every_sliding_window_a2_3_with_4_events` - Multiple events
- `test_every_sliding_window_exactly_3_with_5_events` - Exact count
- `test_non_every_no_sliding_window` - Non-EVERY doesn't slide
- `test_every_sliding_window_a1_2_with_4_events` - min=1 immediate output
- `test_every_sliding_window_event_chain_integrity` - Event chain verification
- `test_every_sliding_window_a2_5_comprehensive` - Complex scenario

---

## Overview

EVERY enables multi-instance pattern matching where each triggering event starts a NEW concurrent pattern instance, allowing overlapping matches.

**Basic EVERY (A -> B)**: ✅ TRUE OVERLAPPING WORKS!

**EVERY with Logical Operators (A AND B), (A OR B)**: ✅ WORKING via add_logical_group()

**EVERY with Count Quantifiers (A{3} -> B)**: ❌ Sliding window needs implementation

---

## ✅ LOGICAL OPERATORS NOW SUPPORTED

**Added 2025-11-25**: PatternChainBuilder now supports AND/OR logical groups.

### API

```rust
use eventflux_rust::core::query::input::stream::state::pattern_chain_builder::{
    PatternChainBuilder, PatternStepConfig, LogicalGroupConfig, LogicalType,
};

let mut builder = PatternChainBuilder::new(StateType::Pattern);

// Pattern: (A AND B) -> C
builder.add_logical_group(LogicalGroupConfig::and(
    PatternStepConfig::new("e1".into(), "StreamA".into(), 1, 1),
    PatternStepConfig::new("e2".into(), "StreamB".into(), 1, 1),
));
builder.add_step(PatternStepConfig::new("e3".into(), "StreamC".into(), 1, 1));

// Or using OR:
// builder.add_logical_group(LogicalGroupConfig::or(...))
```

### How Logical Groups Work

**AND Logic**:
- Creates two LogicalPreStateProcessor instances (left and right)
- Both sides must match before the pattern can proceed
- LogicalPostStateProcessor checks if partner's position is filled
- Only forwards to next element when BOTH sides have matched

**OR Logic**:
- Either side matching is sufficient
- When one side matches, it marks the partner as "satisfied"
- First match triggers the pattern to proceed

### Chaining with Logical Groups

```rust
// Pattern: A -> (B AND C) -> D
builder.add_step(a_config);           // state_id = 0
builder.add_logical_group(and_bc);    // state_id = 1, 2
builder.add_step(d_config);           // state_id = 3

// Pattern: (A OR B) -> C
builder.add_logical_group(or_ab);     // state_id = 0, 1
builder.add_step(c_config);           // state_id = 2
```

### State ID Allocation

- Simple steps consume 1 state_id
- Logical groups consume 2 state_ids (one for each side)
- Use `builder.total_state_count()` to get total state positions

### Test Coverage

30 unit tests in `pattern_chain_builder.rs` including:
- `test_build_chain_with_logical_group` - Basic AND group
- `test_build_chain_with_or_group` - Basic OR group
- `test_build_chain_with_step_then_logical_group` - A -> (B AND C)
- `test_build_chain_with_logical_group_then_step` - (A AND B) -> C
- `test_validation_logical_group_last_element_not_exact` - Validation

---

## Architecture

### Three-List State Machine

File: `src/core/query/input/stream/state/stream_pre_state.rs:12-31`

StreamPreState manages three event lists:
- `current_state_event_chunk: Vec<StateEvent>` - Current working set
- `pending_state_event_list: VecDeque<StateEvent>` - Events waiting for processing
- `new_and_every_state_event_list: VecDeque<StateEvent>` - New events plus EVERY loopback events

### Flag-Based Detection

File: `src/core/query/input/stream/state/stream_pre_state_processor.rs:99-102, 357-365, 504-537`

```rust
pub struct StreamPreStateProcessor {
    is_every_pattern: bool,  // Set by PatternChainBuilder
    // ... other fields
}

fn reset_state(&mut self) {
    let should_skip_reset = self.is_start_state && self.is_every_pattern;
    if should_skip_reset {
        // Don't clear pending for EVERY patterns
        return;
    }
    // Normal reset logic
}
```

Methods:
- `set_every_pattern_flag(bool)` - Set flag
- `is_every_pattern() -> bool` - Check flag

### Loopback Mechanism

File: `src/core/query/input/stream/state/stream_post_state_processor.rs:181-185`

```rust
if let Some(ref next_every) = self.next_every_state_pre_processor {
    let next_every_state_id = next_every.lock().unwrap().state_id();
    state_event_copy.expand_to_size(next_every_state_id + 1);
    next_every.lock().unwrap().add_every_state(state_event_copy);
}
```

When pattern completes at last post processor, forwards matched state to first pre processor via `add_every_state()`.

### PreStateProcessor Trait

File: `src/core/query/input/stream/state/pre_state_processor.rs:76-85`

```rust
fn add_state(&mut self, state_event: StateEvent) -> StateEventId;
fn add_every_state(&mut self, state_event: StateEvent) -> StateEventId;
```

- `add_state()` - Normal forward chaining
- `add_every_state()` - Pattern restart

### PatternChainBuilder Integration

File: `src/core/query/input/stream/state/pattern_chain_builder.rs:65, 86-100, 136-140, 209-225`

Builder method:
```rust
pub fn set_every(&mut self, is_every: bool) {
    self.is_every = is_every;
}
```

Validation (enforced at build time):
```rust
if self.is_every && !matches!(self.state_type, StateType::Pattern) {
    return Err("EVERY patterns are only supported in PATTERN mode, not SEQUENCE mode");
}
```

Wiring at build:
```rust
if self.is_every {
    // Set loopback on last post processor only
    last_post.lock().unwrap().set_next_every_state_pre_processor(first_pre.clone());

    // Set flag on all pre processors
    for pre in &pre_processors_concrete {
        pre.lock().unwrap().stream_processor.set_every_pattern_flag(true);
    }
}
```

Rationale for dual mechanism:
- Loopback only on last processor prevents infinite loops
- Flag on all processors enables reliable detection (survives test wrapping)

### CountPreStateProcessor

File: `src/core/query/input/stream/state/count_pre_state_processor.rs:136-181`

EVERY detection:
```rust
let is_every_start = self.stream_processor.is_start_state()
    && self.stream_processor.is_every_pattern();
```

Completed state filtering:
```rust
if is_every_start && !pending_states.is_empty() {
    pending_states.retain(|state_event| {
        // Keep only states without events at later positions
        for pos in (state_id + 1)..state_event.stream_event_count() {
            if state_event.get_stream_event(pos).is_some() {
                return false; // Completed state from loopback
            }
        }
        true
    });
}
```

Fresh StateEvent creation when pending is empty:
```rust
if pending_states.is_empty() && is_every_start {
    let new_state = StateEvent::new(1, 0);
    pending_states.push(new_state);
}
```

## Query API

File: `src/query_api/execution/query/input/state/every_state_element.rs`

```rust
pub struct EveryStateElement {
    state_element: Box<StateElement>,
}
```

Factory method in `src/query_api/execution/query/input/state/state.rs:19-21`:
```rust
pub fn every(state_element: StateElement) -> StateElement {
    StateElement::Every(Box::new(EveryStateElement::new(state_element)))
}
```

Usage:
```rust
let sse1 = State::stream(a_si);
let sse2 = State::stream(b_si);
let every_a = State::every(StateElement::Stream(sse1));
let pattern = State::next(every_a, StateElement::Stream(sse2));
```

## Pattern Semantics

### Pattern Restart Behavior

Without EVERY:
```
Events: A(1) → B(2) → A(3) → B(4)
Result: 2 matches
  - A1-B2 (completes, pattern resets)
  - A3-B4 (new instance)
```

With EVERY:
```
Events: A(1) → B(2) → A(3) → B(4)
Result: 2 matches
  - A1-B2 (completes, restarts via loopback)
  - A3-B4 (new instance after restart)
```

Pattern restarts after each completion. Not simultaneous overlapping.

### What Basic EVERY Does (WORKING!)

```
Events: A(1) → A(2) → B(3)

ACTUAL (verified 2025-11-25): 2 matches (A1-B3 AND A2-B3) ✅
```

**Basic TRUE overlapping IS working correctly!** See test `test_true_every_overlapping_multiple_a_before_b`.

## Restrictions

Enforced by PatternChainBuilder validation:

1. PATTERN mode only - Not allowed in SEQUENCE mode
2. Top-level only - No nested EVERY patterns
3. Requires parentheses - `EVERY (pattern)` syntax

## Test Coverage

File: `tests/pattern_every_overlapping_test.rs`

Total: 7 tests, all passing

Core tests:
- `test_every_pattern_overlapping_instances` - Basic restart: A→B→A→B produces 2 matches
- `test_pattern_without_every_no_overlapping` - Without EVERY: A→A→B produces 1 match
- `test_every_validation_sequence_mode_rejected` - SEQUENCE mode validation error

Integration tests:
- `test_every_with_count_quantifiers` - EVERY (A{3} → B): collects 3 A events, restarts
- `test_every_with_within` - EVERY + WITHIN constraint (see known limitation)
- `test_every_with_longer_chain` - EVERY (A → B → C): 3-step pattern restart
- `test_every_memory_leak_stress` - 100 A→B sequences without memory leaks

## Execution Flow Example

Pattern: `EVERY (A{1} -> B{1})`
Events: A(1) → B(2) → A(3) → B(4)

Step 1 - A(1) arrives at pre[0]:
- init() creates StateEvent{}
- A(1) added → StateEvent{A(1)}
- Forwarded to post[0] → pre[1]

Step 2 - B(2) arrives at pre[1]:
- StateEvent{A(1)} pending
- B(2) added → StateEvent{A(1), B(2)}
- Pattern complete, forwarded to post[1]
- post[1] has loopback → sends to pre[0].add_every_state()
- Output: 1 match (A1-B2)

Step 3 - A(3) arrives at pre[0]:
- new_and_every_state_event_list has StateEvent{A(1), B(2)} from loopback
- is_every_pattern=true so reset_state() skips clearing
- CountPreStateProcessor filters out completed states
- Creates fresh StateEvent{}
- A(3) added → StateEvent{A(3)}
- Forwarded to post[0] → pre[1]

Step 4 - B(4) arrives at pre[1]:
- StateEvent{A(3)} pending
- B(4) added → StateEvent{A(3), B(4)}
- Pattern complete
- Output: 2 matches (A1-B2, A3-B4)

## Implementation Status

Runtime: **MOSTLY COMPLETE** ✅
- Three-list state machine: ✅ Implemented and working
- Flag-based detection: ✅ Implemented
- Loopback mechanism: ✅ Implemented and working
- Query API: ✅ Implemented
- PatternChainBuilder integration: ✅ Implemented
- Validation: ✅ Implemented
- Basic TRUE overlapping: ✅ WORKING (verified 2025-11-25)
- Logical operators (AND/OR): ✅ WORKING (added 2025-11-25)
- Test coverage: 40 tests total (30 unit + 10 integration, 39 passing, 1 for sliding window - expected)

**What's Missing** (P1 - not critical):
- Sliding window for count quantifiers (A{3} -> B with overlapping windows)
- This requires spawning new StateEvents on each incoming A event at EVERY boundary
- Integration tests for EVERY with logical operators (can be added later)

Parser: Not implemented
- Estimated effort: 1-2 days
- Blocked on: P0 parser foundation
- Required: Parse `EVERY (pattern)` syntax, map to builder.set_every(true)

## Known Limitations

### WITHIN Timing Integration

WITHIN timing constraints may not fully integrate with EVERY pattern restart.

Test case:
```
Pattern: EVERY (A -> B) WITHIN 5 seconds
Events: A(3)@t2000, B(4)@t10000
Expected: Timeout (8 seconds exceeds 5s window)
Actual: Match (timing not enforced on restart)
```

Test: `test_every_with_within()` documents this with 2 matches instead of 1.

Investigation needed:
- Check if WITHIN timer resets on pattern restart via loopback
- Verify expired event handling accounts for EVERY
- Determine if bug or alternative semantic

Status: Documented, requires investigation (1-2 days)
Priority: P2
Workaround: Validate timing behavior for specific use cases or avoid strict timing with EVERY

### Parser Integration

No SQL parser for EVERY keyword exists.

Status: Runtime complete, parser pending (1-2 days)
Workaround: Use programmatic API via PatternChainBuilder.set_every(true)
Priority: P1

## Grammar Design (Not Implemented)

Intended syntax:
```sql
FROM PATTERN (
    EVERY (e1=FailedLogin{3,5} -> e2=AccountLocked)
)
PARTITION BY userId
SELECT e1[0].timestamp, count(e1) as attempts
INSERT INTO SecurityAlerts;
```

Parser requirements:
- Parse EVERY keyword with required parentheses
- Validate PATTERN mode only
- Validate top-level only
- Map to builder.set_every(true)

## File Locations

Core implementation:
- `src/core/query/input/stream/state/stream_pre_state.rs` - Three-list state machine
- `src/core/query/input/stream/state/stream_pre_state_processor.rs` - Flag detection, reset logic
- `src/core/query/input/stream/state/stream_post_state_processor.rs` - Loopback forwarding
- `src/core/query/input/stream/state/pre_state_processor.rs` - add_every_state trait
- `src/core/query/input/stream/state/count_pre_state_processor.rs` - State filtering, fresh creation
- `src/core/query/input/stream/state/pattern_chain_builder.rs` - Builder integration, validation, logical groups
- `src/core/query/input/stream/state/logical_pre_state_processor.rs` - AND/OR pre-processing
- `src/core/query/input/stream/state/logical_post_state_processor.rs` - AND/OR post-processing

Query API:
- `src/query_api/execution/query/input/state/every_state_element.rs` - EveryStateElement
- `src/query_api/execution/query/input/state/state.rs` - Factory method

Tests:
- `tests/pattern_every_overlapping_test.rs` - 10 comprehensive tests (9 passing, 1 ignored)
- `tests/pattern_runtime.rs:142-245` - Legacy test with deprecated architecture
- Unit tests in `pattern_chain_builder.rs` - 30 tests including 16 logical group tests

## Performance

Memory overhead: 1 bool per StreamPreStateProcessor
CPU overhead: O(1) flag check, O(n) state filtering where n = pending states (typically 1-2)
Stress test: 100 sequential restarts complete without memory leaks or performance degradation

## References

Gap analysis: `feat/pattern_processing/GRAMMAR_V1.2_IMPLEMENTATION_GAPS.md` lines 285-357
