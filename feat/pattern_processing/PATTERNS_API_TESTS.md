# Pattern Processing API Tests - Comprehensive Test Design

**Purpose**: Define comprehensive tests for pattern matching that cover real-world CEP scenarios, not just basic counting.

**Status**: Design phase - awaiting review before implementation

**Last Updated**: 2025-11-04

---

## Test Coverage Analysis

### Current Test Coverage (52 tests) - STATUS: INCOMPLETE

**What Current Tests Cover:**
- ✅ Basic exact counts: A{3}, A{5}
- ✅ state_changed() flag behavior for single patterns
- ✅ Simple consecutive events (2-10 events)
- ✅ Min/max boundary conditions for single patterns

**CRITICAL GAPS - What We DON'T Test:**
- ❌ **Pattern chaining** (A > B > C) - Core sequence feature
- ❌ **Optional middle patterns** (B{0,2}) - State machine transitions
- ❌ **Forward-only state machine** - Cannot go backwards
- ❌ **Non-greedy matching** - Stop at minimum
- ❌ **State transitions** between patterns
- ❌ Patterns with non-matching events interleaved
- ❌ Event chains with 50+ events
- ❌ Attribute selection and filtering accuracy
- ❌ Multiple pattern instances in parallel (PARTITION BY)
- ❌ Aggregation on pattern results
- ❌ Memory usage under load
- ❌ Pattern failure and state cleanup

**VERDICT:** Current tests validate only ~30% of pattern matching semantics. Pattern chaining, state machines, and non-greedy behavior are completely untested.

### Most Critical Tests (Must Pass for Production)

**Priority 1 - State Machine Semantics (BLOCKER):**
1. **Test 1.4**: Optional Middle Pattern (B{0,2}) ← **MOST CRITICAL**
   - Validates forward-only transitions
   - Validates optional patterns (min=0)
   - Validates failure on backward/wrong events

2. **Test 1.2**: Long chains (A{50} > B{100})
   - Validates multi-pattern sequencing
   - Validates state transitions
   - Validates non-greedy (single output at end)

**Priority 2 - Real-World Scenarios:**
3. **Test 1.1**: Interleaved non-matching events
4. **Test 2.1**: Attribute preservation in long chains
5. **Test 1.3**: Parallel pattern instances (PARTITION BY)

**Current Status:** ❌ **None of Priority 1 tests implemented yet**

---

## Test Category 1: Complex Event Chains

### Test 1.1: Pattern Match with Interleaved Non-Matching Events

**Scenario**: Find pattern A{3} where matching events arrive with non-matching events in between

**Setup**:
```
Pattern: A{3} where A = StockPrice[symbol == "AAPL"]
Event stream:
  1. AAPL (match)
  2. GOOGL (no match)
  3. MSFT (no match)
  4. AAPL (match)
  5. TSLA (no match)
  6. AAPL (match)
  7. META (no match)
```

**Expected**:
- Output at event 6 (3 AAPL events matched)
- Output contains events 1, 4, 6 in chain
- Events 2, 3, 5, 7 ignored
- Attributes preserved: symbol="AAPL" in all matched events

**Validates**:
- Pattern matching filters correctly
- Non-matching events don't break state
- Event chains preserve only matching events
- Attribute selection works across gaps

---

### Test 1.2: Long Event Chain (100+ events)

**Scenario**: Pattern chain with long accumulation: A{50} > B{100} with 200+ events

**Setup**:
```
Pattern: A{50} > B{100} (two-stage pattern)
Event stream: 200 events
  - Events 1-49: A events (count < 50)
  - Event 50: A event → A pattern complete, transition to B
  - Events 51-149: B events (count < 100)
  - Event 150: B event → B pattern complete (count=100) → MATCH
  - Events 151-200: Additional events (should not affect completed pattern)
```

**Expected**:
- Events 1-49: A state accumulating, no output
- Event 50: A complete, transition to B state
- Events 51-149: B state accumulating, no output
- Event 150: B complete (count=100), **pattern matched**, state removed
- Events 151-200: Ignored (pattern already completed)
- **Single output** at event 150 (non-greedy: only when complete)

**Validates**:
- Long chains handled correctly (150 events in chain)
- Count tracking accurate over 100+ events
- Memory efficiency (no leaks)
- State transition between patterns
- State cleanup after completion
- Non-greedy behavior (single output when complete)

---

### Test 1.3: Multiple Pattern Instances in Parallel

**Scenario**: Two separate pattern instances matching simultaneously (partitioned by customerId)

**Setup**:
```
Pattern: A{3} where A = Order[customerId]
PARTITION BY customerId
Event stream:
  1. Order(customerId=1, amount=100)  → Instance 1: count=1
  2. Order(customerId=2, amount=200)  → Instance 2: count=1
  3. Order(customerId=1, amount=150)  → Instance 1: count=2
  4. Order(customerId=2, amount=250)  → Instance 2: count=2
  5. Order(customerId=1, amount=175)  → Instance 1: count=3 → MATCH
  6. Order(customerId=2, amount=275)  → Instance 2: count=3 → MATCH
```

**Expected**:
- Two independent pattern instances (one per customerId)
- Instance 1 (customerId=1): Matches at event 5 (exact count=3)
- Instance 2 (customerId=2): Matches at event 6 (exact count=3)
- Output 1: Chain contains [event1, event3, event5] with customerId=1
- Output 2: Chain contains [event2, event4, event6] with customerId=2
- No cross-contamination between instances
- **Non-greedy**: Each instance outputs once when complete

**Validates**:
- Parallel pattern matching
- State isolation per partition
- Attribute grouping (PARTITION BY)
- Independent completion tracking
- Non-greedy behavior per instance

---

### Test 1.4: Optional Middle Pattern (B{0,2}) - CRITICAL

**Scenario**: Forward-only state machine with optional middle pattern

**Setup**:
```
Pattern: A{2} > B{0,2} > C{2}
```

**Test 1.4a: Skip B entirely (min=0)**
```
Events: A, A, C, C
State progression:
  - Event 1 (A): [A:1]
  - Event 2 (A): [A:2] → Transition to B state
  - Event 3 (C): [B:0, got C] → Transition to C state (B optional)
  - Event 4 (C): [C:2] → MATCH
Output: Chain [A1, A2, C1, C2] (no B events)
```

**Test 1.4b: One B event**
```
Events: A, A, B, C, C
State progression:
  - Event 1 (A): [A:1]
  - Event 2 (A): [A:2] → Transition to B state
  - Event 3 (B): [B:1]
  - Event 4 (C): [B:1, got C] → Transition to C state
  - Event 5 (C): [C:2] → MATCH
Output: Chain [A1, A2, B1, C1, C2]
```

**Test 1.4c: Two B events (max reached)**
```
Events: A, A, B, B, C, C
State progression:
  - Event 1 (A): [A:1]
  - Event 2 (A): [A:2] → Transition to B state
  - Event 3 (B): [B:1]
  - Event 4 (B): [B:2] → Max reached, transition to C state
  - Event 5 (C): [C:1]
  - Event 6 (C): [C:2] → MATCH
Output: Chain [A1, A2, B1, B2, C1, C2]
```

**Test 1.4d: Three B events (FAIL - forward-only)**
```
Events: A, A, B, B, B, ...
State progression:
  - Event 1 (A): [A:1]
  - Event 2 (A): [A:2] → Transition to B state
  - Event 3 (B): [B:1]
  - Event 4 (B): [B:2] → Max reached, transition to C state
  - Event 5 (B): [C state, got B] → FAIL (expected C, got B)
Result: Pattern fails, state removed
```

**Test 1.4e: A after transition (FAIL - cannot go backwards)**
```
Events: A, A, A, ...
State progression:
  - Event 1 (A): [A:1]
  - Event 2 (A): [A:2] → Transition to B state
  - Event 3 (A): [B state, got A] → FAIL (cannot return to A)
Result: Pattern fails, state removed
```

**Test 1.4f: C arrives too early (FAIL - A not satisfied)**
```
Events: A, C, ...
State progression:
  - Event 1 (A): [A:1]
  - Event 2 (C): [A:1, got C] → FAIL (A needs 2 events, only has 1)
Result: Pattern fails, state removed
```

**Validates**:
- Optional middle pattern (min=0) can be skipped
- Forward-only state transitions (no backwards)
- Max count triggers automatic transition
- Pattern failure on invalid event type for current state
- State cleanup on failure

---

## Test Category 2: Attribute Selection and Accuracy

### Test 2.1: Attribute Preservation in Long Chain

**Scenario**: Verify all attributes preserved correctly in 50-event chain

**Setup**:
```
Pattern: A{50} where A = SensorReading
Events: 50 sensor readings with:
  - timestamp: 1000 + i*100 (incrementing)
  - sensorId: "sensor-" + i
  - value: 20.0 + i*0.5 (incrementing)
  - status: i % 2 == 0 ? "ok" : "warn"
```

**Expected**:
- Output at event 50
- Chain contains all 50 events
- Walk chain and verify:
  - All timestamps correct: [1000, 1100, 1200, ..., 5900]
  - All sensorIds correct: ["sensor-0", "sensor-1", ..., "sensor-49"]
  - All values correct: [20.0, 20.5, 21.0, ..., 44.5]
  - All statuses correct: alternating "ok"/"warn"

**Validates**:
- No attribute corruption in long chains
- All event data preserved
- Chain traversal works correctly
- Memory layout correct

---

### Test 2.2: Attribute Selection with Conditions

**Scenario**: Pattern matching with attribute-based filtering

**Setup**:
```
Pattern: A{3} where A = Trade[price > 100 AND volume > 1000]
Event stream (20 events):
  1. Trade(price=90, volume=1500)   → No match (price too low)
  2. Trade(price=110, volume=500)   → No match (volume too low)
  3. Trade(price=120, volume=1200)  → Match 1
  4. Trade(price=95, volume=2000)   → No match (price too low)
  5. Trade(price=130, volume=1500)  → Match 2
  6. Trade(price=140, volume=1100)  → Match 3 → OUTPUT
  ... 14 more events (mixed matching/non-matching)
```

**Expected**:
- Output at event 6 (3 matching trades)
- Chain contains only events 3, 5, 6
- Verify each event in chain satisfies conditions:
  - Event 3: price=120 > 100 ✓, volume=1200 > 1000 ✓
  - Event 5: price=130 > 100 ✓, volume=1500 > 1000 ✓
  - Event 6: price=140 > 100 ✓, volume=1100 > 1000 ✓
- Non-matching events (1,2,4) not in chain

**Validates**:
- Condition evaluation accuracy
- Filtered chain correctness
- Attribute-based filtering

---

### Test 2.3: Aggregation on Pattern Results

**Scenario**: Compute aggregation (sum, avg, max) on matched pattern

**Setup**:
```
Pattern: A{5} where A = Sale[amount]
Events: 5 sales with amounts [100, 200, 150, 300, 250]
```

**Expected**:
- Output at event 5
- Chain contains 5 events with amounts [100, 200, 150, 300, 250]
- Computed aggregations:
  - SUM(amount) = 1000
  - AVG(amount) = 200
  - MAX(amount) = 300
  - MIN(amount) = 100
  - COUNT(*) = 5

**Validates**:
- Aggregation functions on pattern results
- Correct traversal of event chain
- Accurate attribute extraction
- Numeric computation correctness

---

## Test Category 3: Pattern State Lifecycle

### Test 3.1: State Cleanup After Max Reached

**Scenario**: Verify state removed from pending after reaching max

**Setup**:
```
Pattern: A{3,5}
Events: Send 10 events
```

**Expected**:
- Events 1-2: state in pending, count < min
- Event 3: output, state still in pending (count=3, max=5)
- Event 4: output, state still in pending (count=4)
- Event 5: output, state REMOVED from pending (count=5, max reached)
- Events 6-10: no effect (state already completed)

**Validation**:
- Inspect pending_list size:
  - After event 2: 1 state
  - After event 4: 1 state
  - After event 5: 0 states (cleaned up)
  - After event 10: 0 states

**Validates**:
- State cleanup on max
- No memory leaks
- No processing after completion

---

### Test 3.2: Multiple States with Different Progress

**Scenario**: Multiple pattern instances at different stages

**Setup**:
```
Pattern: A{3,5}
Event stream:
  1. Event(groupId=1) → State1 created, count=0
  2. Event(groupId=2) → State2 created, count=0
  3. Event(groupId=1) → State1 count=1
  4. Event(groupId=1) → State1 count=2
  5. Event(groupId=2) → State2 count=1
  6. Event(groupId=1) → State1 count=3 → OUTPUT (min reached)
  7. Event(groupId=2) → State2 count=2
  8. Event(groupId=1) → State1 count=4 → OUTPUT
  9. Event(groupId=1) → State1 count=5 → OUTPUT, COMPLETE
  10. Event(groupId=2) → State2 count=3 → OUTPUT (min reached)
```

**Expected**:
- Two independent states in pending
- State1 outputs: events 6, 8, 9 (3 outputs)
- State1 completes at event 9
- State2 outputs: event 10 (1 output so far)
- State2 still in pending after event 10 (count=3 < max=5)

**Validates**:
- Multiple states managed correctly
- Independent progress tracking
- Correct completion per state

---

### Test 3.3: State Expiry with Within Time

**Scenario**: Pattern with time window constraint

**Setup**:
```
Pattern: A{3} WITHIN 10 seconds
Events:
  1. Event(time=1000) → State created
  2. Event(time=3000) → State count=1
  3. Event(time=6000) → State count=2
  4. Event(time=15000) → Time check: 15000 - 1000 = 14s > 10s → EXPIRE
  5. Event(time=16000) → New state created
```

**Expected**:
- State expires at event 4 (time window exceeded)
- No output (count=2 < min=3)
- State removed from pending
- Event 5 starts fresh pattern

**Validates**:
- Time window enforcement
- State expiry logic
- Cleanup of expired states
- Fresh start after expiry

---

## Test Category 4: Performance and Scale

### Test 4.1: Memory Usage with Long Chains

**Scenario**: Measure memory usage for pattern with 1000 events

**Setup**:
```
Pattern: A{1000}
Events: Send 1000 events with full attributes
```

**Expected**:
- Track memory usage:
  - Before: X bytes
  - After 500 events: Y bytes
  - After 1000 events: Z bytes
- Memory should be linear: Z ≈ 2*Y (linear growth)
- After completion: Memory released
- No memory leaks

**Validates**:
- Memory efficiency
- Linear memory growth (not exponential)
- Proper cleanup

---

### Test 4.2: Throughput Test (10,000 events/sec)

**Scenario**: Pattern matching at high event rate

**Setup**:
```
Pattern: A{100}
Events: Send 10,000 events as fast as possible
```

**Expected**:
- 100 outputs (at counts 100, 200, 300, ..., 10000)
- Processing time: < 1 second
- Throughput: > 10,000 events/sec
- All outputs correct

**Validates**:
- Performance under load
- Correctness at high speed
- No dropped events

---

### Test 4.3: Many Parallel Pattern Instances

**Scenario**: 1000 pattern instances in parallel

**Setup**:
```
Pattern: A{5} partitioned by userId
Events: 5000 events from 1000 different users (5 each)
```

**Expected**:
- 1000 independent pattern instances
- 1000 outputs (one per user)
- All outputs correct
- Memory usage reasonable
- Processing time acceptable

**Validates**:
- Scalability to many instances
- Parallel state management
- Memory efficiency with many states

---

## Test Category 5: Edge Cases and Error Conditions

### Test 5.1: Zero Events After State Creation

**Scenario**: State created but no events arrive

**Setup**:
```
Pattern: A{3}
Actions:
  1. Create initial state
  2. update_state() (move to pending)
  3. Don't send any events
  4. Check pending_list
```

**Expected**:
- State remains in pending
- No outputs
- No crashes
- state_changed = false

**Validates**:
- Graceful handling of no events
- State persistence

---

### Test 5.2: Events Arriving Out of Order (by timestamp)

**Scenario**: Events arrive with non-monotonic timestamps

**Setup**:
```
Pattern: A{3}
Events:
  1. Event(timestamp=1000)
  2. Event(timestamp=3000)
  3. Event(timestamp=2000)  ← Out of order!
```

**Expected**:
- All events processed
- Chain preserves arrival order (not timestamp order)
- Output at event 3
- Chain: [event1, event2, event3] (arrival order)

**Validates**:
- Arrival order vs timestamp order
- Handling of out-of-order timestamps

---

### Test 5.3: Pattern with Max = i32::MAX (Unbounded)

**Scenario**: A+ pattern that never completes

**Setup**:
```
Pattern: A{1, i32::MAX}  (A+)
Events: Send 10,000 events
```

**Expected**:
- 10,000 outputs
- state_changed always false (never reaches max)
- State remains in pending indefinitely
- Memory usage grows linearly

**Question**: How should unbounded patterns be cleaned up?

**Validates**:
- Unbounded pattern handling
- Memory behavior with no completion
- Need for explicit cleanup mechanism?

---

### Test 5.4: Count Overflow Protection

**Scenario**: Ensure count doesn't overflow

**Setup**:
```
Pattern: A{100, i32::MAX}
Events: Send i32::MAX + 1000 events (if possible in test)
```

**Expected**:
- Count saturates at i32::MAX (or u32::MAX)
- No integer overflow
- No panics
- Graceful handling

**Validates**:
- Integer overflow protection
- Safe counting

---

## Test Category 6: Integration Scenarios

### Test 6.1: Pattern Chaining (A -> B)

**Scenario**: Sequential pattern matching A{2} followed by B{3}

**Setup**:
```
Pattern: (A{2} -> B{3})
Events:
  1. A event
  2. A event → A{2} matches
  3. B event → Start B{3}
  4. B event
  5. B event → B{3} matches → COMPLETE
```

**Expected**:
- A pattern completes at event 2
- B pattern starts at event 3
- Full pattern completes at event 5
- Output contains chain: [A1, A2, B1, B2, B3]

**Validates**:
- Pattern sequencing
- State transition between patterns
- Combined output

---

### Test 6.2: Logical Pattern (A{2} AND B{2})

**Scenario**: Both patterns must match

**Setup**:
```
Pattern: (A{2} AND B{2})
Events:
  1. A event
  2. B event
  3. A event → A{2} complete
  4. B event → B{2} complete → OUTPUT (both satisfied)
```

**Expected**:
- Output when both A{2} and B{2} satisfied
- Combined output with events from both streams
- Correct logical combination

**Validates**:
- Logical AND combination
- Multi-stream pattern matching
- Coordinated state tracking

---

### Test 6.3: Pattern with Aggregation in SELECT

**Scenario**: Pattern match with aggregation in output

**Setup**:
```
FROM TemperatureStream
SELECT avg(temp) as avgTemp, count(*) as eventCount
PATTERN (HIGH{5,10})
WHERE HIGH.temp > 30
```

**Events**: 8 temperature readings: [32, 31, 35, 33, 34, 36, 37, 38]

**Expected**:
- Output at event 5 (min reached): avgTemp=33.0, eventCount=5
- Output at event 6: avgTemp=33.5, eventCount=6
- Output at event 7: avgTemp=34.0, eventCount=7
- Output at event 8: avgTemp=34.375, eventCount=8

**Validates**:
- Aggregation on pattern results
- Incremental aggregation updates
- Correct computation

---

## Test Category 7: Semantic Correctness

### Test 7.1: Greedy vs Non-Greedy Matching

**Scenario**: Verify patterns are greedy by default

**Setup**:
```
Pattern: A{2,5}
Events: Send exactly 3 matching events
```

**Expected**:
- Output at event 2 (count=2, min reached)
- Output at event 3 (count=3, still valid)
- State remains in pending (waiting for more, greedy behavior)
- No automatic completion at min

**Validates**:
- Greedy matching semantics
- State continuation beyond min

---

### Test 7.2: Event Order Preservation

**Scenario**: Verify events in chain maintain order

**Setup**:
```
Pattern: A{5}
Events: 5 events with unique IDs [e1, e2, e3, e4, e5]
```

**Expected**:
- Output chain order: e1 -> e2 -> e3 -> e4 -> e5
- Walk chain: verify order matches arrival order
- get_next() traversal: e1.next=e2, e2.next=e3, etc.

**Validates**:
- Chain order correctness
- Linked list integrity

---

### Test 7.3: State Isolation Between Patterns

**Scenario**: Two different patterns don't interfere

**Setup**:
```
Query 1: A{3}
Query 2: A{5}
Events: Send 6 A events
```

**Expected**:
- Query 1 output at event 3
- Query 2 output at event 5
- No cross-contamination
- Independent state tracking

**Validates**:
- Pattern isolation
- Multiple queries on same stream

---

## Implementation Priority

### Phase 1: Critical Correctness Tests (Week 1)
- Test 1.1: Interleaved non-matching events
- Test 2.1: Attribute preservation in long chains
- Test 2.2: Attribute selection with conditions
- Test 3.1: State cleanup after max
- Test 7.2: Event order preservation

### Phase 2: Scale and Performance Tests (Week 2)
- Test 1.2: Long event chains (100+)
- Test 4.1: Memory usage
- Test 4.2: Throughput test
- Test 4.3: Many parallel instances

### Phase 3: Integration Tests (Week 3)
- Test 1.3: Multiple pattern instances
- Test 2.3: Aggregation on results
- Test 6.1: Pattern chaining
- Test 6.2: Logical patterns

### Phase 4: Edge Cases (Week 4)
- Test 3.2: Multiple states with different progress
- Test 3.3: State expiry with time
- Test 5.1-5.4: All edge cases
- Test 7.1, 7.3: Semantic correctness

---

## Test Infrastructure Needed

### Helper Functions
```rust
fn create_event_stream(count: usize, filter: fn(i32) -> bool) -> Vec<StreamEvent>
fn verify_chain_order(output: &StateEvent, expected_ids: &[i32]) -> bool
fn measure_memory_usage() -> usize
fn create_partitioned_events(num_partitions: usize, events_per_partition: usize) -> Vec<StreamEvent>
```

### Assertion Helpers
```rust
fn assert_chain_contains_exactly(chain: &StateEvent, expected: &[EventId])
fn assert_attribute_values(chain: &StateEvent, field: &str, expected: &[Value])
fn assert_aggregation(chain: &StateEvent, agg: AggregationType, expected: f64, tolerance: f64)
```

### Performance Helpers
```rust
fn measure_throughput(pattern: &Pattern, events: Vec<StreamEvent>) -> f64
fn measure_memory_growth(pattern: &Pattern, event_counts: &[usize]) -> Vec<usize>
```

---

## Success Criteria

**Before declaring Task 2.1 production-ready, ALL of the following must pass:**

1. ✅ All Phase 1 tests (critical correctness)
2. ✅ At least 80% of Phase 2 tests (scale/performance)
3. ✅ At least 70% of Phase 3 tests (integration)
4. ✅ At least 60% of Phase 4 tests (edge cases)

**Performance Requirements:**
- Throughput: > 10,000 events/sec for simple patterns
- Memory: Linear growth, no leaks
- Latency: < 1ms per event for patterns with < 100 events

**Correctness Requirements:**
- 100% attribute preservation accuracy
- 100% event order preservation
- 100% correct output count
- 100% correct state_changed behavior

---

## Pattern Semantics - FINALIZED

### Rule 1: Matching Strategy
**Non-Greedy Matching**: Patterns match the minimum required and transition immediately.

### Rule 2: Pattern Constraints

**Single Patterns (not in a chain):**
```
✅ A{3}      (exact count, min ≥ 1)
❌ A{2,5}    (invalid - must be exact)
❌ A+        (invalid - unbounded)
❌ A{0,3}    (invalid - min=0)
```

**Pattern Chains:**
```
✅ A{2} > B{0,2} > C{3}     (valid)
❌ A{0,2} > B{2}            (invalid - first has min=0)
❌ A{2} > B{1,5}            (invalid - last has range)
❌ A{2} > B+                (invalid - last unbounded)
```

**Constraints:**
- **First pattern**: min ≥ 1 (must have triggering event)
- **Last pattern**: min = max (must be exact count)
- **Middle patterns**: Can have ranges, can have min=0 (optional)
- **Single patterns**: Treated as both first and last (min ≥ 1 AND exact)

### Rule 3: State Transitions

**Forward-Only State Machine:**
- After pattern X reaches min, transition to pattern X+1
- Cannot return to previous patterns
- Receiving previous pattern's event after transition = FAIL

**Example: A{2} > B{0,2} > C{2}**

Scenario 1: Skip B (min=0 allows it)
```
Events: A, A, C, C
States: [A:1] → [A:2,transition] → [B:0,got C,transition] → [C:1] → [C:2,MATCH]
```

Scenario 2: One B
```
Events: A, A, B, C, C
States: [A:1] → [A:2,transition] → [B:1,got C,transition] → [C:1] → [C:2,MATCH]
```

Scenario 3: Two B (max reached)
```
Events: A, A, B, B, C, C
States: [A:1] → [A:2,transition] → [B:1] → [B:2,transition] → [C:1] → [C:2,MATCH]
```

Scenario 4: Three B (FAIL)
```
Events: A, A, B, B, B
States: [A:1] → [A:2,transition] → [B:1] → [B:2,transition] → [C:got B,FAIL]
```

### Rule 4: Additional Constraints
- **Unbounded patterns**: Must use `WITHIN` with event count or time limit
- **Out-of-order timestamps**: Process by arrival order (not timestamp order)
- **Memory limits**: Covered by `WITHIN` constraints
- **Aggregations**: Computed eagerly (per event)

---

## Critical Realization

### Current Implementation Status

**What We Have:**
- ✅ CountPreStateProcessor for single patterns (A{n})
- ✅ 52 tests for basic counting
- ✅ state_changed() semantics correct

**What We're Missing (CRITICAL):**
- ❌ **Pattern chaining** (A > B > C) - Not implemented
- ❌ **State transitions** between patterns - Not implemented
- ❌ **Optional middle patterns** (B{0,2}) - Not implemented
- ❌ **Forward-only state machine** - Not implemented
- ❌ **Non-greedy matching** - Current behavior is greedy
- ❌ **Pattern failure handling** - Not implemented

### Honest Assessment

**Current confidence in CountPreStateProcessor: 95%**
- Works perfectly for single exact-count patterns (A{3}, A{5})
- Handles state_changed() correctly
- Memory management solid

**Current confidence for production use: 40%**
- Single patterns work ✅
- Pattern chaining completely untested ❌
- State machine semantics not validated ❌
- Non-greedy matching not implemented ❌

### The Real Problem

We tested and validated **only CountPreStateProcessor**, which handles single patterns.

But the finalized semantics require:
1. **SequencePreStateProcessor** (or equivalent) for chaining A > B > C
2. State transition logic between patterns
3. Optional pattern handling (min=0)
4. Forward-only validation (reject backwards events)
5. Non-greedy completion

**These components either don't exist or haven't been tested.**

---

## Next Steps - Revised

### Immediate (Week 1):
1. ✅ **Review and finalize test design** (this document)
2. **Implement Test 1.4** (Optional Middle Pattern)
   - Will likely reveal: pattern chaining not implemented
   - Will likely reveal: state transitions not working
3. **Implement pattern sequencing infrastructure**
   - SequencePreStateProcessor for A > B chains
   - State transition logic
   - Pattern completion detection

### Short-term (Weeks 2-3):
4. Implement Test 1.2 (Long chains)
5. Implement Test 1.1 (Interleaved events)
6. Fix all bugs discovered
7. Implement non-greedy matching

### Medium-term (Week 4):
8. Performance and scale tests
9. Edge cases
10. Integration scenarios

**Expected timeline**: 4-6 weeks for production-ready pattern matching

---

## Decision Point

**Before proceeding with test implementation, we need to decide:**

**Option A: Implement tests now, expect many failures**
- Pro: Tests will guide implementation
- Pro: We'll discover all gaps
- Con: Will see 80%+ test failures initially

**Option B: Implement pattern chaining first, then test**
- Pro: Better test pass rate initially
- Pro: Can validate as we build
- Con: Might miss edge cases

**Recommendation**: **Option A** - Implement Test 1.4 first as acceptance test, then build to pass it.

---

## Summary for Review

**This document defines:**
1. ✅ Pattern semantics (finalized based on your input)
2. ✅ 30+ comprehensive test scenarios
3. ✅ Implementation phases
4. ✅ Success criteria
5. ✅ Honest gap analysis

**Next action after your approval:**
- Implement Test 1.4 (Optional Middle Pattern) as acceptance test
- Will reveal what needs to be built to support pattern chaining
- Iteratively implement and fix until Test 1.4 passes

**Current Status**:
- Task 2.1 (CountPreStateProcessor): ✅ 95% complete for single patterns
- Task 2.1 (Pattern Chaining): ❌ 10% complete (design only, no implementation)
- **Overall Task 2.1**: 50% complete

**Awaiting your review and approval to proceed.**
