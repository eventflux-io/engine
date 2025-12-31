# Pattern Processing

Last Updated: 2025-12-06

---

## 1. Overview

Pattern processing enables Complex Event Processing (CEP) through temporal pattern matching. Events are matched against patterns with count quantifiers, sequences, logical operators, and time constraints.

---

## 2. Current Status

### 2.1 Runtime Complete (Parser Needed)

| Feature | Tests | Location |
|---------|-------|----------|
| Pre/Post Processor Architecture | 195 | `src/core/query/input/stream/state/` |
| Count Quantifiers A{n}, A{m,n} | 52 | `count_pre_state_processor.rs` |
| Pattern Chaining A -> B -> C | 24 | `pattern_chain_builder.rs` |
| Array Access e[0], e[last], e[n] | 14+ | `indexed_variable_executor.rs` |
| Cross-Stream References | 6 | `stream_pre_state_processor.rs` |
| EVERY Multi-Instance | 10 | `stream_post_state_processor.rs` |
| Logical Operators AND, OR | 16 | `pattern_chain_builder.rs` |
| Collection Aggregations | 50+ | `collection_aggregation_executor.rs` |
| Time-based WITHIN | 3+ | `stream_pre_state_processor.rs` |

Total: 370+ tests

### 2.2 Runtime Not Implemented

| Feature | Effort | Notes |
|---------|--------|-------|
| PARTITION BY | 10-15 days | Multi-tenant isolation |
| Absent Patterns | 7-10 days | NOT A FOR duration, requires TimerWheel |
| Event-count WITHIN | 3-4 days | WITHIN 100 EVENTS |
| OUTPUT Event Types | 3-4 days | Wire OutputEventType to runtime |

---

## 3. Architecture

### 3.1 Processor Chain Model

Pattern chains use multi-processor architecture:

```
Event -> PreA -> PostA -> PreB -> PostB -> PreC -> PostC -> Output
```

Each step has:
- PreStateProcessor: Accumulates events, checks conditions
- PostStateProcessor: Forwards matched StateEvent to next step

### 3.2 Key Components

StateEvent: Container for matched events across pattern steps
- `stream_events[i]`: Events at position i (linked list for count quantifiers)
- `get_event_chain(position)`: Returns all events at a position
- `add_event(position, event)`: Adds event to chain

PatternChainBuilder: Factory for creating processor chains
- Creates CountPreStateProcessor per step
- Wires Pre -> Post -> next Pre connections
- Validates: first min>=1, last exact count, all min>=1

StateType: Controls matching semantics
- `Pattern`: Ignores non-matching events, keeps pending states
- `Sequence`: Fails on non-matching events, clears states

### 3.3 Stream Routing

Events route to processors by stream type:
```rust
chain.pre_processors[0].process(event_a);  // Stream A -> Processor 0
chain.update_state();
chain.pre_processors[1].process(event_b);  // Stream B -> Processor 1
chain.update_state();
```

### 3.4 Deadlock Resolution

Problem: Pre->Post->Pre processor chain deadlocked due to circular Arc<Mutex> references.

Solution: Lock-Free Shared State
```rust
struct ProcessorSharedState {
    state_changed: AtomicBool,  // Lock-free atomic flag
}
```
Both Pre and Post processors share this state. PostStateProcessor marks state changed without locking PreStateProcessor Arc.

---

## 4. Feature Reference

### 4.1 Count Quantifiers

Syntax: `A{n}` (exact), `A{m,n}` (range)

Implementation: `CountPreStateProcessor`
- Accumulates events via `add_event()`
- Forwards when count >= min_count
- Completes when count == max_count

Constraints:
- min_count >= 1 (all steps)
- max_count must be explicit (no unbounded A+, A{1,})
- Last step must have exact count (min == max)

### 4.2 Array Access

Syntax: `e[0]`, `e[1]`, `e[last]`

Implementation: `IndexedVariableExecutor`
- Zero-based indexing
- `e[last]` resolves to final event in chain
- Out-of-bounds returns NULL

Usage:
```sql
SELECT e1[0].timestamp as first_attempt,
       e1[last].timestamp as last_attempt
FROM PATTERN (e1=FailedLogin{3,5} -> e2=AccountLocked)
```

> ✅ SQL compiler and runtime support complete (2025-12-06). IndexedVariableExecutor handles array access in pattern queries.

### 4.3 Cross-Stream References

Syntax: `e2[price > e1.price]`

Implementation: Condition function receives `StateEvent`
- Filter can access any matched event via position
- Current event added to StateEvent before filter evaluation
- Uses `VariableExpressionExecutor` with position array

### 4.4 EVERY Patterns

Syntax: `EVERY (e1=A -> e2=B)`

Semantics:
- Each occurrence of first element starts new instance
- Instances overlap and run concurrently
- Only allowed in PATTERN mode (not SEQUENCE)
- Only at top level (not nested)

Implementation:
- `add_every_state()` creates new instance
- `next_every_state_pre_processor` enables loop-back
- `new_and_every_state_event_list` manages instances

### 4.5 Logical Operators

Syntax: `A AND B`, `A OR B`

Implementation: `PatternChainBuilder.add_logical_group()`
- `LogicalGroupConfig` with `and()` / `or()` helpers
- `LogicalPreStateProcessor` / `LogicalPostStateProcessor` for runtime

### 4.6 Collection Aggregations

Syntax: `count(e1)`, `sum(e1.price)`, `avg(e1.price)`, `min(e1.price)`, `max(e1.price)`, `stdDev(e1.price)`

Implementation: Collection aggregation executors in `src/core/executor/collection_aggregation_executor.rs`

Executors:
- `CollectionCountExecutor`: Counts events in chain
- `CollectionSumExecutor`: Sums attribute values
- `CollectionAvgExecutor`: Averages attribute values
- `CollectionMinMaxExecutor`: Finds min/max
- `CollectionStdDevExecutor`: Standard deviation

Registry: `CollectionAggregationFunction` trait in `src/core/extension/mod.rs`

Semantics:
- NULL values skipped (SQL semantics)
- Type preservation (INT -> LONG, FLOAT -> DOUBLE)
- Overflow detection with fallback to DOUBLE
- Empty collections return NULL

### 4.7 WITHIN Constraints

Syntax: `... WITHIN 10 MINUTES`

Implementation:
- `set_within_time()` on first processor
- `expire_events()` removes expired states
- Reactive validation (checks on event arrival)

Proactive expiry (via TimerWheel) deferred to Phase 3.

---

## 5. Validation Rules

### 5.1 PatternChainBuilder Validation

1. First step: min_count >= 1
2. Last step: min_count == max_count (exact)
3. All steps: min_count >= 1 (no zero-count)
4. All steps: min_count <= max_count
5. All steps: max_count explicit (no unbounded)

### 5.2 EVERY Restrictions

- Only in PATTERN mode
- Only at top level
- Requires parentheses
- No multiple EVERY in one pattern

### 5.3 Unsupported Patterns

The following are rejected to prevent memory overflow:
- `A+` or `A{1,}` - One or more (unbounded)
- `A*` or `A{0,}` - Zero or more (unbounded + zero-count)
- `A?` or `A{0,1}` - Zero or one (zero-count)
- `A{0,n}` - Zero to n (zero-count)

---

## 6. File Locations

Core processors:
```
src/core/query/input/stream/state/
├── pre_state_processor.rs           # PreStateProcessor trait
├── post_state_processor.rs          # PostStateProcessor trait
├── stream_pre_state_processor.rs    # Base Pre implementation
├── stream_post_state_processor.rs   # Base Post implementation
├── count_pre_state_processor.rs     # Count quantifier Pre
├── count_post_state_processor.rs    # Count quantifier Post
├── pattern_chain_builder.rs         # Factory and validation
├── logical_pre_state_processor.rs   # AND/OR Pre
├── logical_post_state_processor.rs  # AND/OR Post
├── stream_pre_state.rs              # Three-list state management
└── shared_processor_state.rs        # Lock-free shared state
```

Executors:
```
src/core/executor/
├── indexed_variable_executor.rs         # Array access
└── collection_aggregation_executor.rs   # Collection aggregations
```

State management:
```
src/core/event/state/
├── state_event.rs          # StateEvent structure
├── state_event_cloner.rs   # Cloning with 'every' support
└── state_event_factory.rs  # StateEvent creation
```

Preserved for Phase 3:
```
src/core/query/input/stream/state/timers/
└── timer_wheel.rs          # O(1) timer scheduling (313 lines, 8 tests)
```

---

## 7. Parser Status

### 7.1 Complete (2025-12-06)

SQL compiler integration complete for core pattern features:

| Feature | Status | Location |
|---------|--------|----------|
| FROM PATTERN / FROM SEQUENCE | ✅ | `converter.rs:convert_pattern_input` |
| Sequence operator `->` | ✅ | `converter.rs:convert_pattern_expression` |
| Count quantifiers `{n,m}` | ✅ | Validation + conversion |
| Event aliases `e1=StreamName` | ✅ | StreamStateElement with alias |
| Filter conditions `[expression]` | ✅ | Filter in SingleInputStream |
| Cross-stream references `e1.attr` | ✅ | CompoundIdentifier handling |
| Array access `e[0].attr`, `e[last].attr` | ✅ | IndexedVariable + IndexedVariableExecutor |
| Time expressions and WITHIN | ✅ | Time-based WITHIN supported |
| Logical operators AND, OR | ✅ | LogicalStateElement |
| EVERY keyword with validation | ✅ | PatternValidator + EveryStateElement |
| Collection aggregation detection | ✅ | Registry lookup at runtime |

Tests: 17 integration tests + 109 sql_compiler tests

### 7.2 Known Limitations

| Feature | Status | Notes |
|---------|--------|-------|
| PATTERN/SEQUENCE in JOINs | ❌ Not supported | Explicit error returned |
| Event-count WITHIN | ❌ Not supported | Use time-based WITHIN |
| PARTITION BY | ❌ Not implemented | Runtime support needed |
| Absent patterns (NOT ... FOR) | ❌ Not implemented | Requires TimerWheel |
| OUTPUT event types | ❌ Not implemented | Wire to runtime needed |

See PATTERN_GRAMMAR_V1.2.md "Known Limitations" section for details.

---

## 8. Related Documents

- PATTERN_GRAMMAR_V1.2.md: Grammar specification for parser implementation
- PATTERN_TEST_SPEC.md: Test specifications for pattern processing
