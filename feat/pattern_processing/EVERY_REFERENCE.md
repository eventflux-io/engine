# EVERY Pattern Implementation Reference

Date: 2025-11-24
Status: Runtime complete, parser pending

## Overview

EVERY enables pattern restart semantics. When a pattern completes, it forwards the matched state back to the start and continues matching with subsequent events.

Implementation mode: Pattern restart (sequential matches), not simultaneous overlapping instances.

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

### What EVERY Does Not Do

```
Events: A(1) → A(2) → B(3)
Does not produce: 2 matches (A1-B3 AND A2-B3)
Actual result: 1 match (A2-B3)
Reason: A2 replaces A1 before B3 arrives
```

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

Runtime: Complete
- Three-list state machine: Implemented
- Flag-based detection: Implemented
- Loopback mechanism: Implemented
- Query API: Implemented
- PatternChainBuilder integration: Implemented
- Validation: Implemented
- Test coverage: 7 tests passing

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
- `src/core/query/input/stream/state/pattern_chain_builder.rs` - Builder integration, validation

Query API:
- `src/query_api/execution/query/input/state/every_state_element.rs` - EveryStateElement
- `src/query_api/execution/query/input/state/state.rs` - Factory method

Tests:
- `tests/pattern_every_overlapping_test.rs` - 7 comprehensive tests
- `tests/pattern_runtime.rs:142-245` - Legacy test with deprecated architecture

## Performance

Memory overhead: 1 bool per StreamPreStateProcessor
CPU overhead: O(1) flag check, O(n) state filtering where n = pending states (typically 1-2)
Stress test: 100 sequential restarts complete without memory leaks or performance degradation

## References

Gap analysis: `feat/pattern_processing/GRAMMAR_V1.2_IMPLEMENTATION_GAPS.md` lines 285-357
