# Grammar V1.2 Implementation Gap Analysis

**Version**: 1.2
**Date**: 2025-12-04 (Updated)
**Previous Date**: 2025-11-27
**Status**: Gap Analysis Blueprint
**Purpose**: Identify gaps between Grammar V1.2 requirements and current implementation before grammar integration

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Feature-by-Feature Gap Analysis](#feature-by-feature-gap-analysis)
3. [Implementation Priorities](#implementation-priorities)
4. [Architecture Decisions Required](#architecture-decisions-required)
5. [Migration Concerns](#migration-concerns)
6. [Test Coverage Gaps](#test-coverage-gaps)
7. [Implementation Roadmap](#implementation-roadmap)

---

## Executive Summary

### Overall Readiness: ~70% Complete (updated from 45%)

**Current Implementation Status**:
- Phase 1 COMPLETE: Pre/Post state processor architecture (195 tests)
- Phase 2a COMPLETE: Count quantifiers for single patterns (52 tests)
- Phase 2b COMPLETE: Pattern chaining with `->` operator (24 tests)
- Phase 2c COMPLETE: Array access runtime (14+ tests)
- Phase 2d COMPLETE: Cross-stream references runtime (6 tests)
- Phase 2e COMPLETE: EVERY multi-instance runtime (10 tests)
- Phase 2f COMPLETE: Collection aggregation executors (50+ tests)
- Phase 3 NOT STARTED: Absent patterns
- Phase 4 NOT STARTED: Advanced features (PARTITION BY, OUTPUT types)

**Total Tests Passing**: 370+ tests (pattern-specific)

**Status for Grammar Integration**:
1. Parser does not exist - No SQL parser for Pattern Grammar V1.2
2. Array access runtime - COMPLETE. `IndexedVariableExecutor` works. Parser needed.
3. Logical operators runtime - COMPLETE. AND, OR via PatternChainBuilder.add_logical_group(). Parser needed.
4. Cross-stream references runtime - COMPLETE. Condition function receives StateEvent. Parser needed.
5. EVERY multi-instance - COMPLETE. Overlapping and sliding window working. Parser needed.
6. Collection aggregations - COMPLETE. All executors implemented (count, sum, avg, min, max, stdDev). Parser needed.
7. PARTITION BY runtime - Not implemented. Multi-tenant isolation not available.
8. Event-count WITHIN - Not implemented. Only time-based WITHIN exists.
9. Absent patterns runtime - Not implemented. AbsentStreamStateElement exists but no processor.
10. OUTPUT event types - Not implemented. OutputEventType enum exists but not wired to pattern runtime.

---

## Feature-by-Feature Gap Analysis

### 1. PATTERN vs SEQUENCE Modes

**Grammar Requirement**:
```sql
FROM PATTERN (...)   -- Relaxed matching (allows gaps)
FROM SEQUENCE (...)  -- Strict consecutive matching (no gaps)
```

**Query API Status**: ✅ COMPLETE
- `StateType` enum exists in `src/core/query/input/stream/state/stream_pre_state_processor.rs`
- `StateType::Pattern` and `StateType::Sequence` defined

**Runtime Status**: ✅ COMPLETE
- Phase 2b tests confirm both modes work
- `reset_and_update()` in SEQUENCE mode clears state after match
- PATTERN mode preserves pending states

**Parser Status**: ❌ NOT IMPLEMENTED
- No SQL parser for `FROM PATTERN (...)` or `FROM SEQUENCE (...)`

**Expression Evaluator Status**: N/A

**Gap**:
- **Parser only** - Need to parse `FROM PATTERN/SEQUENCE (...)` and map to StateType enum

**Implementation Effort**: **Medium** (2-3 days)
- Add Pattern/Sequence keywords to EventFluxDialect
- Parse pattern_mode → StateType mapping
- Validate mode restrictions (EVERY only in PATTERN)

**Dependencies**: None

**Priority**: **P0** - Critical for any pattern query

---

### 2. Sequence Operator (`->`)

**Grammar Requirement**:
```sql
e1=LoginStream -> e2=DataAccessStream -> e3=LogoutStream
```

**Query API Status**: ✅ COMPLETE
- `NextStateElement` exists in `src/query_api/execution/query/input/state/next_state_element.rs`

**Runtime Status**: ✅ COMPLETE
- Phase 2b: Pattern chaining (24 tests passing)
- `PatternChainBuilder` factory creates multi-processor chains
- PreA → PostA → PreB → PostB → PreC architecture validated

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for `->` operator

**Expression Evaluator Status**: N/A

**Gap**:
- **Parser only** - Need to parse `->` and map to NextStateElement

**Implementation Effort**: **Small** (1 day)
- Add `->` operator to pattern expression parser
- Map to `NextStateElement(left, right)`

**Dependencies**: Basic pattern expression parser

**Priority**: **P0** - Critical for sequences

---

### 3. Count Quantifiers

**Grammar Requirement** (SUPPORTED):
```sql
A{3}        -- Exactly 3                    ✅ SUPPORTED
A{2,5}      -- Between 2 and 5 (bounded)    ✅ SUPPORTED
```

**Grammar Requirement** (NOT SUPPORTED - by design):
```sql
A{1,} or A+ -- One or more (unbounded)     ❌ NOT SUPPORTED
A{0,} or A* -- Zero or more (unbounded)    ❌ NOT SUPPORTED
A{0,1} or A? -- Zero or one (zero-count)   ❌ NOT SUPPORTED
```

**Why Not Supported**:
- Unbounded patterns (A+, A{n,}) can cause memory overflow
- Zero-count patterns (A*, A?) break pattern step semantics (every step must match >= 1 event)
- Explicit bounds required: use `A{min,max}` with both values specified

**Query API Status**: ✅ COMPLETE
- `CountStateElement` exists in `src/query_api/execution/query/input/state/count_state_element.rs`

**Runtime Status**: ✅ COMPLETE
- Phase 2a: Count quantifiers (52 tests passing)
- `StateEvent` event chaining methods validated (43 tests)
- Methods: `add_event()`, `remove_last_event()`, `get_event_chain()`, `count_events_at()`
- Validation: `UNBOUNDED_MAX_COUNT` sentinel rejects unbounded patterns

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for `{n,m}` syntax

**Expression Evaluator Status**: N/A

**Gap**:
- **Parser only** - Need to parse bounded count quantifier syntax `{n}` and `{n,m}`

**Implementation Effort**: **Low** (1 day)
- Parse `{n}`, `{n,m}` syntax only (bounded)
- Map to `CountStateElement(pattern, min, max)`
- Validation: reject unbounded (`{n,}`), zero-count (`{0,n}`), invalid (`{max < min}`)

**Dependencies**: Basic pattern expression parser

**Priority**: **P0** - Critical for count-based patterns

---

### 4. Array Access (`e[0]`, `e[last]`)

**Grammar Requirement**:
```sql
SELECT e1[0].userId,        -- First event
       e1[last].timestamp,  -- Last event
       count(e1)            -- Total count
```

**Query API Status**: ✅ COMPLETE (updated 2025-11-27)
- `IndexedVariable` AST node exists in `src/query_api/expression/indexed_variable.rs`
- `EventIndex` enum with `Numeric(usize)` and `Last` variants
- `Expression::IndexedVariable` variant exists

**Runtime Status**: ✅ COMPLETE (updated 2025-11-27)
- `IndexedVariableExecutor` fully implemented in `src/core/executor/indexed_variable_executor.rs`
- 14+ comprehensive unit tests passing
- Supports:
  - `e[0]` → first event ✅
  - `e[1]`, `e[2]`, ... → specific index ✅
  - `e[last]` → last event (dynamic resolution) ✅
  - Out-of-bounds → returns `None` (safe handling) ✅
- Uses `StateEvent.get_event_chain()` for event chain traversal

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for array index syntax

**Expression Evaluator Status**: ✅ COMPLETE
- `IndexedVariableExecutor` implements `ExpressionExecutor` trait
- Works with `StateEvent` for pattern queries
- Handles all attribute types (INT, LONG, DOUBLE, STRING, etc.)

**Gap**:
- **Parser only**: Parse `alias[index].attribute` syntax
  1. Parse `e1[0]`, `e1[last]`, `e1[n]` syntax
  2. Map to `IndexedVariable` AST node
  3. Compiler wires to existing `IndexedVariableExecutor`

**Implementation Effort**: **Small** (1-2 days) - reduced from 5-7 days
- Parser: 1-2 days (conflict resolution with float literals like `1.0`)
- Query API: ✅ ALREADY COMPLETE
- Executor: ✅ ALREADY COMPLETE (14+ tests passing)
- Integration: Minimal wiring needed

**Dependencies**:
- StateEvent infrastructure (COMPLETE ✅)
- Expression evaluator framework (COMPLETE ✅)
- IndexedVariableExecutor (COMPLETE ✅)

**Priority**: **P0** - Required for SELECT clause with count quantifiers

**See**: `feat/pattern_processing/COLLECTION_AGGREGATIONS.md` for detailed status

---

### 5. Logical Operators (AND, OR)

**Grammar Requirement**:
```sql
FROM PATTERN (
    (e1=Login AND e2=VPNConnect) -> e3=DataExport
)
```

**Query API Status**: ✅ COMPLETE
- `LogicalStateElement` exists in `src/query_api/execution/query/input/state/logical_state_element.rs`

**Runtime Status**: ✅ COMPLETE (updated 2025-11-25)
- `LogicalPreStateProcessor` and `LogicalPostStateProcessor` exist in `src/core/query/input/stream/state/`
- Implements partner processor pattern for AND/OR coordination
- Both `LogicalType::And` and `LogicalType::Or` supported
- Thread-safe coordination via shared locks
- **PatternChainBuilder now supports `add_logical_group()`** - Full API for building logical patterns
- 16 new unit tests for logical group functionality

**PatternChainBuilder API** (added 2025-11-25):
```rust
// Pattern: (A AND B) -> C
builder.add_logical_group(LogicalGroupConfig::and(
    PatternStepConfig::new("e1".into(), "A".into(), 1, 1),
    PatternStepConfig::new("e2".into(), "B".into(), 1, 1),
));
builder.add_step(PatternStepConfig::new("e3".into(), "C".into(), 1, 1));
```

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for `AND`, `OR` operators in pattern expressions

**Expression Evaluator Status**: N/A (pattern-level operators, not expression-level)

**Gap**:
- **Parser only**: Parse `AND`, `OR` operators and map to LogicalGroupConfig/LogicalStateElement

**Implementation Effort**: **Small** (1 day)
- Parser: 1 day (precedence: AND > OR)
- Runtime: ✅ COMPLETE - PatternChainBuilder.add_logical_group() ready

**Dependencies**: Pattern expression parser

**Priority**: **P1** - Important for complex patterns, runtime fully ready

---

### 6. Absent Patterns (NOT ... FOR)

**Grammar Requirement**:
```sql
FROM PATTERN (
    e1=Order -> NOT Shipping FOR 24 hours
)
```

**Query API Status**: ✅ COMPLETE
- `AbsentStreamStateElement` exists in `src/query_api/execution/query/input/state/absent_stream_state_element.rs`

**Runtime Status**: ❌ NOT IMPLEMENTED (Phase 3)
- Documentation explicitly states Phase 3 not started
- Requires TimerWheel for temporal tracking
- 1 test ignored waiting for Phase 3

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for `NOT ... FOR duration` syntax

**Expression Evaluator Status**: N/A

**Gap**:
- **CRITICAL RUNTIME MISSING**:
  1. AbsentStreamPreStateProcessor (temporal tracking)
  2. TimerWheel integration for timeout detection
  3. Parser for `NOT ... FOR duration` syntax
  4. Validation: Reject absent in logical combinations

**Implementation Effort**: **Large** (7-10 days)
- TimerWheel: 2-3 days
- AbsentStreamProcessor: 3-4 days
- Parser: 1-2 days
- Tests: 2-3 days

**Dependencies**:
- TimerWheel infrastructure
- Time expression parser

**Priority**: **P2** - Advanced feature, not blocking basic patterns

---

### 7. EVERY Patterns (Multi-Instance)

**Grammar Requirement**:
```sql
FROM PATTERN (
    EVERY (e1=FailedLogin{3,5} -> e2=AccountLocked)
)
```

**Restrictions**:
- ✅ Only in PATTERN mode (NOT SEQUENCE)
- ✅ Only at top level (NOT nested)
- ✅ Requires parentheses: `EVERY (...)`

**Query API Status**: ✅ COMPLETE
- `EveryStateElement` exists in `src/query_api/execution/query/input/state/every_state_element.rs`

**Runtime Status**: ✅ **MOSTLY COMPLETE** (updated 2025-11-25)
- ✅ Three-list architecture exists and WORKING
- ✅ Loopback mechanism exists and WORKING
- ✅ `add_every_state()` trait method exists in PreStateProcessor
- ✅ **PatternChainBuilder**: Loopback wired correctly
- ✅ **Validation**: PATTERN-mode-only restriction enforced
- ✅ **Basic TRUE OVERLAPPING**: WORKING! (verified with test)
  - A1→A2→B3 produces 2 matches (A1-B3, A2-B3) ✅
- ✅ **Sliding window with count quantifiers**: WORKING (2025-11-26)
  - EVERY A{2,3}→B with 4 A events produces 5 overlapping window outputs
  - EVERY A{3}→B with 5 A events produces 3 sliding windows

**Working Examples** (verified 2025-11-26):

Basic EVERY overlapping:
```
Pattern: EVERY (A -> B)
Events: A1@1000 → A2@2000 → B3@3000

RESULT: 2 matches (CORRECT!)
  - Match 1: A1 → B3
  - Match 2: A2 → B3

Test: test_true_every_overlapping_multiple_a_before_b
```

Sliding window with count quantifiers (NEW):
```
Pattern: EVERY (A{3} -> B)
Events: [A1, A2, A3, A4, A5, B6]
RESULT: 3 matches (sliding window) ✅
  - [A1,A2,A3] → B6
  - [A2,A3,A4] → B6
  - [A3,A4,A5] → B6

Test: test_every_sliding_window_exactly_3_with_5_events
```

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for `EVERY (...)` syntax
- Validation logic exists in PatternChainBuilder, ready for parser integration

**Expression Evaluator Status**: N/A

**Gap**:
1. **Parser**: Parse `EVERY (pattern)` syntax

**Implementation Effort**: **Small** (1 day)
- Parser (EVERY syntax): 1 day

**Implementation Details** (2025-11-26):
- `pattern_chain_builder.rs` - EVERY flag wiring and loopback
- `count_pre_state_processor.rs` - Sliding window spawning (lines 149-155, 257-280)
- 7 sliding window tests in `count_pre_state_processor.rs`
- See `feat/pattern_processing/EVERY_REFERENCE.md` for full documentation

**Dependencies**:
- Pattern expression parser (for `EVERY (...)` syntax)

**Priority**: **P0** - Runtime COMPLETE, only parser needed

**Status Update** (2025-11-26):
- ✅ Basic TRUE overlapping VERIFIED WORKING
- ✅ Sliding window with count quantifiers VERIFIED WORKING
- All runtime features complete, only parser needed

---

### 8. WITHIN Constraints

#### 8a. Time-Based WITHIN

**Grammar Requirement**:
```sql
... WITHIN 10 minutes
... WITHIN 24 hours
```

**Query API Status**: ✅ COMPLETE
- Time expressions exist (milliseconds, seconds, minutes, hours, days)

**Runtime Status**: ✅ PARTIAL
- `set_within_time(duration_ms)` method exists on PreStateProcessor
- Tested in basic scenarios

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for `WITHIN duration` clause
- No time expression parser

**Expression Evaluator Status**: N/A

**Gap**:
- **Parser only**:
  1. Parse `WITHIN n time_unit` syntax
  2. Parse time units (milliseconds, seconds, minutes, hours, days)
  3. Convert to milliseconds
  4. Map to `set_within_time()` call

**Implementation Effort**: **Small** (1-2 days)
- Time expression parser: 1 day
- WITHIN clause parser: 0.5 days
- Tests: 0.5 days

**Dependencies**: Pattern statement parser

**Priority**: **P0** - Common requirement for time-bounded patterns

#### 8b. Event-Count WITHIN

**Grammar Requirement**:
```sql
... WITHIN 100 EVENTS
```

**Query API Status**: ⚠️ UNCLEAR
- No explicit event-count WITHIN in Query API

**Runtime Status**: ❌ NOT IMPLEMENTED
- `set_within_event_count(count)` method does NOT exist on PreStateProcessor
- Requires event counter per pattern instance

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for `WITHIN n EVENTS` syntax

**Expression Evaluator Status**: N/A

**Gap**:
- **CRITICAL RUNTIME MISSING**:
  1. Add `set_within_event_count(count)` to PreStateProcessor
  2. Event counter tracking per pattern instance
  3. Fail pattern when event count exceeded
  4. Parser for `WITHIN n EVENTS` syntax

**Implementation Effort**: **Medium** (3-4 days)
- Runtime: 2-3 days (event counting, failure logic)
- Parser: 0.5 days
- Tests: 1 day

**Dependencies**:
- PreStateProcessor architecture
- WITHIN clause parser

**Priority**: **P2** - Nice-to-have, time-based WITHIN covers most use cases

---

### 9. PARTITION BY Clause

**Grammar Requirement**:
```sql
FROM PATTERN (...)
PARTITION BY userId, deviceId
SELECT ...
```

**Query API Status**: ⚠️ UNCLEAR
- No explicit PARTITION BY in Query API state elements

**Runtime Status**: ❌ NOT IMPLEMENTED
- No multi-tenant pattern isolation
- No processor-per-partition mechanism
- Requires partition key extraction and instance routing

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for `PARTITION BY col1, col2, ...`

**Expression Evaluator Status**: ⚠️ NEEDED
- Need to evaluate partition key attributes from events

**Gap**:
- **MAJOR RUNTIME ARCHITECTURE**:
  1. Partition key extractor (evaluate attributes from event)
  2. Processor instance manager (create/route per partition)
  3. State isolation per partition
  4. Cleanup for expired partitions
  5. Parser for PARTITION BY clause
  6. Validation: No duplicate columns, at least one column

**Implementation Effort**: **Very Large** (10-15 days)
- Architecture design: 2-3 days
- Partition key extraction: 2 days
- Instance manager: 3-4 days
- State isolation: 2-3 days
- Parser: 1 day
- Tests: 3-5 days

**Dependencies**:
- Expression evaluator (for partition key attributes)
- Pattern runtime architecture

**Priority**: **P1** - Critical for multi-tenant CEP, but single partition works for simple cases

---

### 10. OUTPUT Event Types

**Grammar Requirement**:
```sql
INSERT ALL EVENTS INTO stream       -- Both arrivals and expirations
INSERT CURRENT EVENTS INTO stream   -- Default, arrivals only
INSERT EXPIRED EVENTS INTO stream   -- Expirations only
```

**Query API Status**: ✅ COMPLETE
- `OutputEventType` enum exists in `src/query_api/execution/query/output/output_stream.rs`
- Variants: `CurrentEvents`, `ExpiredEvents`, `AllEvents`, `AllRawEvents`, `ExpiredRawEvents`

**Runtime Status**: ❌ NOT WIRED
- OutputEventType enum exists but not wired to pattern processors
- Pattern processors likely only emit CURRENT events
- Need to emit EXPIRED events on timeout/window removal

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for `INSERT ALL EVENTS INTO ...`

**Expression Evaluator Status**: N/A

**Gap**:
1. **Wire OutputEventType to pattern runtime**:
   - PostStateProcessor: Check output_event_type config
   - Emit EXPIRED events on pattern timeout
   - Emit EXPIRED events on window eviction
   - Filter events based on output_event_type
2. **Parser**: Parse `ALL EVENTS`, `CURRENT EVENTS`, `EXPIRED EVENTS` in INSERT clause

**Implementation Effort**: **Medium** (3-4 days)
- Runtime wiring: 2-3 days
- Parser: 0.5 days
- Tests: 1 day

**Dependencies**:
- Pattern runtime
- Window eviction logic

**Priority**: **P2** - Useful for debugging, but CURRENT EVENTS sufficient for most cases

---

### 11. Cross-Stream References

**Grammar Requirement**:
```sql
FROM PATTERN (
    e1=StockPrice[symbol == 'AAPL'] ->
    e2=StockPrice[symbol == 'AAPL' AND price > e1.price * 1.1]
)
```

**Query API Status**: COMPLETE
- VariableExpressionExecutor supports cross-stream references via position array

**Runtime Status**: COMPLETE (2025-11-23)
- Condition function signature: `Fn(&StateEvent) -> bool`
- Filter receives full StateEvent with all matched events
- Current event added to StateEvent before filter evaluation
- 6 tests passing in `tests/pattern_filter_cross_stream_test.rs`

**Parser Status**: NOT IMPLEMENTED
- No parser for filter conditions `[expression]`
- No parser for cross-stream references `e1.attribute`

**Gap**:
- Parser only: Parse `[expression]` filter syntax and `alias.attribute` cross-stream references

**Implementation Effort**: 2-3 days (parser only)

**Dependencies**: None - runtime complete

**Priority**: P0 - Required for conditional pattern matching

**See**: `feat/pattern_processing/CROSS_STREAM_FILTER_IMPLEMENTATION.md` for implementation details

---

### 12. Filter Conditions

**Grammar Requirement**:
```sql
e1=TemperatureStream[temp > 100]
e2=LoginStream[userId == 'admin']
```

**Query API Status**: ✅ COMPLETE
- `StreamStateElement` has `filter: Option<Expression>` field

**Runtime Status**: ⚠️ UNCLEAR
- Need to verify if filter expressions are evaluated in PreStateProcessor
- Likely works for simple cases

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for `[expression]` syntax

**Expression Evaluator Status**: ✅ LIKELY COMPLETE
- Expression evaluators exist, likely wired to stream filters

**Gap**:
1. **Verify runtime filter evaluation** - Test that filters work
2. **Parser**: Parse `[expression]` filter syntax

**Implementation Effort**: **Small-Medium** (1-3 days)
- Investigation: 0.5 days
- Parser: 1-2 days
- Tests: 0.5-1 day

**Dependencies**:
- Expression evaluator framework

**Priority**: **P0** - Critical for selective event matching

---

### 13. Time Expressions

**Grammar Requirement**:
```sql
10 milliseconds, 5 seconds, 30 minutes, 2 hours, 7 days
```

**Query API Status**: ⚠️ UNCLEAR
- Time expressions likely exist as `ExpressionConstant::Time(ms)`

**Runtime Status**: ✅ LIKELY COMPLETE
- WITHIN time constraints work, suggesting time expressions are handled

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for `n time_unit` syntax

**Expression Evaluator Status**: N/A

**Gap**:
- **Parser only**: Parse time expressions and convert to milliseconds

**Implementation Effort**: **Small** (1 day)
- Parse `integer time_unit`
- Support: milliseconds, seconds, minutes, hours, days
- Convert to milliseconds

**Dependencies**: None

**Priority**: **P0** - Required for WITHIN, FOR clauses

---

### 14. Event Aliases

**Grammar Requirement**:
```sql
e1=LoginStream
e2=DataAccessStream AS data
```

**Query API Status**: ✅ COMPLETE
- `StreamStateElement` has stream_id and likely supports aliasing

**Runtime Status**: ✅ COMPLETE
- Phase 2b tests use multi-event aliases (e1, e2, e3)
- Aliases map to positions in StateEvent.stream_events[]

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for `alias=StreamName` or `StreamName AS alias`

**Expression Evaluator Status**: N/A

**Gap**:
- **Parser only**: Parse event alias syntax

**Implementation Effort**: **Small** (1 day)
- Parse `alias=StreamName` (assignment style)
- Parse `StreamName AS alias` (SQL style)
- Map to StreamStateElement

**Dependencies**: Basic pattern expression parser

**Priority**: **P0** - Required for multi-event patterns

---

### 15. Aggregation Functions (Collection Aggregations)

**Grammar Requirement**:
```sql
SELECT count(e1) as attempts,        -- Count events in collection
       avg(e1.price) as avgPrice,    -- Average over collection
       max(e1.temp) - min(e1.temp) as range
```

**Important Distinction**:
- Window Aggregations: Incremental aggregation over streaming windows (process_add/process_remove)
- Collection Aggregations: Batch aggregation over pattern event collections

**Query API Status**: COMPLETE
- `CollectionAggregationFunction` trait defined in `src/core/extension/mod.rs`
- 6 built-in functions registered: count, sum, avg, min, max, stdDev

**Runtime Status**: COMPLETE (2025-12-04)
- All collection aggregation executors implemented in `src/core/executor/collection_aggregation_executor.rs`
- Executors: `CollectionCountExecutor`, `CollectionSumExecutor`, `CollectionAvgExecutor`, `CollectionMinMaxExecutor`, `CollectionStdDevExecutor`
- Uses `StateEvent.get_event_chain()` for event chain traversal
- Null handling: nulls skipped (SQL semantics)
- Type preservation: INT inputs preserve LONG output, overflow detection
- 50+ unit tests passing
- Registry integration: functions registered in `EventFluxContext`

**Parser Status**: NOT IMPLEMENTED
- No parser for collection aggregation syntax
- Need to distinguish `count(e1)` (collection) from `count(column)` (window)

**Gap**:
- Parser only: Parse aggregation functions and detect collection vs window context
- Compiler: Wire to collection executors when argument is pattern alias

**Implementation Effort**: 1-2 days (parser integration only)

**Dependencies**: None - runtime complete

**Priority**: P1 - Important for analytics on pattern matches

**See**: `feat/pattern_processing/COLLECTION_AGGREGATIONS.md` for implementation details

---

## Implementation Priorities

### Priority 0 (P0): Parser Work - All Runtime Complete

**Timeline**: 2 weeks

| Feature | Parser Effort | Runtime Status | Notes |
|---------|---------------|----------------|-------|
| Parser Foundation | 1 week | N/A | EventFluxDialect, basic pattern statement parser |
| PATTERN/SEQUENCE modes | 2-3 days | COMPLETE | Parse FROM PATTERN/SEQUENCE |
| Sequence operator (->) | 1 day | COMPLETE | Parse -> and map to NextStateElement |
| Count quantifiers | 2 days | COMPLETE | Parse {n,m} syntax (bounded only) |
| Event aliases | 1 day | COMPLETE | Parse e1=StreamName, StreamName AS e1 |
| Filter conditions | 1-3 days | COMPLETE | Parse [expression] syntax |
| Cross-stream references | 2-3 days | COMPLETE | Parse e1.attr in e2 filters (6 tests) |
| Time expressions | 1 day | COMPLETE | Parse n time_unit |
| Time-based WITHIN | 1-2 days | COMPLETE | Parse WITHIN duration |
| Array access expressions | 1-2 days | COMPLETE | Parser only - IndexedVariableExecutor works (14+ tests) |

**Total P0 Effort**: ~2 weeks (parser implementation only)

**Deliverable**: Parse and execute pattern queries with count quantifiers, sequences, filters, cross-stream references, and array access.

---

### Priority 1 (P1): Additional Parser + Minor Runtime

**Timeline**: 1-2 weeks

| Feature | Parser Effort | Runtime Status | Notes |
|---------|---------------|----------------|-------|
| Logical operators (AND, OR) | 1 day | COMPLETE | PatternChainBuilder.add_logical_group() ready (16 tests) |
| EVERY multi-instance | 1-2 days | COMPLETE | Overlapping + sliding window working (10 tests) |
| Collection aggregations | 1-2 days | COMPLETE | All executors implemented (50+ tests) |

**Total P1 Effort**: ~1 week (parser integration only)

**Deliverable**: Pattern queries with logical operators, EVERY patterns, and collection aggregations.

---

### Priority 2 (P2): Runtime Not Implemented

**Timeline**: 4-6 weeks (requires runtime implementation)

| Feature | Effort | Runtime Status | Notes |
|---------|--------|----------------|-------|
| PARTITION BY | 10-15 days | NOT IMPLEMENTED | Multi-tenant isolation, major architecture |
| Absent patterns (NOT ... FOR) | 7-10 days | NOT IMPLEMENTED | Requires TimerWheel, Phase 3 |
| Event-count WITHIN | 3-4 days | NOT IMPLEMENTED | Event counter per instance |
| OUTPUT event types | 3-4 days | NOT IMPLEMENTED | Wire OutputEventType to pattern runtime |

**Total P2 Effort**: 4-6 weeks (runtime + parser)

**Deliverable**: Advanced CEP features for debugging and absence detection.

---

## Architecture Decisions Required

### Decision 1: Parser Architecture

**Question**: Integrate into existing query_parser.rs or create dedicated pattern_parser.rs?

**Options**:
- **Option A**: Extend query_parser.rs with pattern statement parsing
  - ✅ Unified parser
  - ❌ query_parser.rs uses deprecated processors (migration blocked)
  - ❌ Larger file, harder to maintain

- **Option B**: Create pattern_parser.rs with EventFluxDialect
  - ✅ Clean separation
  - ✅ Can use new PatternChainBuilder directly
  - ✅ Easier testing
  - ❌ Duplication of some expression parsing

**Recommendation**: **Option B** - Create pattern_parser.rs
- Avoids migration blockers in query_parser.rs
- Clean architecture
- Can be integrated later when query_parser.rs migrated

---

### Decision 2: Array Access Conflict Resolution

**Question**: How to handle conflict between array access `e[0]` and float literals `1.0`?

**Background**: `e[0].price` could be confused with `e[0.price]` if not carefully parsed.

**Options**:
- **Option A**: Require parentheses for array access: `e[(0)].price`
  - ✅ No conflicts
  - ❌ Ugly syntax, not user-friendly

- **Option B**: Context-aware parsing (lookahead for `]` after integer)
  - ✅ Clean syntax: `e[0].price`
  - ✅ Matches user expectations
  - ⚠️ More complex parser

- **Option C**: Restrict array index to identifier only (no expressions)
  - ✅ Simple parser
  - ❌ Can't use `e[0]`, must use named constant

**Recommendation**: **Option B** - Context-aware parsing
- User-friendly syntax
- Worth the parser complexity
- Already proven in other SQL parsers

---

### Decision 3: PARTITION BY Implementation Strategy

**Question**: How to implement processor-per-partition architecture?

**Options**:
- **Option A**: PartitionedStreamProcessor wrapper
  - Create processor instances dynamically per partition key
  - Route events to correct partition
  - Cleanup expired partitions
  - ✅ Clean separation
  - ✅ Doesn't pollute existing processors

- **Option B**: Add partitioning to every processor
  - Each processor tracks partition instances
  - ❌ Invasive changes
  - ❌ Complexity in every processor

**Recommendation**: **Option A** - PartitionedStreamProcessor wrapper
- Keeps existing processors clean
- Centralized partition management
- Easier to test and debug

**Architecture**:
```rust
pub struct PartitionedStreamProcessor {
    partition_keys: Vec<String>,
    processor_factory: Box<dyn Fn() -> Box<dyn StreamProcessor>>,
    partitions: HashMap<PartitionKey, Box<dyn StreamProcessor>>,
    max_partitions: usize,
    partition_cleanup_policy: CleanupPolicy,
}

impl StreamProcessor for PartitionedStreamProcessor {
    fn process(&mut self, event: StreamEvent) -> ProcessorResult {
        let key = self.extract_partition_key(&event);
        let processor = self.partitions.entry(key)
            .or_insert_with(|| (self.processor_factory)());
        processor.process(event)
    }
}
```

---

### Decision 4: Expression Evaluator Context

**Question**: How to pass StateEvent context to expression evaluators for cross-stream references?

**Current**: Expression evaluators likely only have current StreamEvent context.

**Needed**: Access to entire StateEvent (all previous events in pattern).

**Options**:
- **Option A**: Add StateEvent to ExecutorContext
  - Modify ExecutorContext to include StateEvent
  - ⚠️ May break existing executors

- **Option B**: Create PatternExecutorContext extending ExecutorContext
  - New context type for pattern expressions
  - ✅ Doesn't break existing code
  - ❌ Code duplication

- **Option C**: Add optional StateEvent field to existing ExecutorContext
  - ✅ Backward compatible
  - ✅ Minimal changes

**Recommendation**: **Option C** - Optional StateEvent field
- Backward compatible
- Minimal changes
- Pattern expressions populate StateEvent, others leave it None

---

## Migration Concerns

### Concern 1: query_parser.rs Uses Deprecated Processors

**Issue**: `query_parser.rs` still creates deprecated processors directly instead of using PatternChainBuilder.

**Impact**: Cannot safely extend query_parser.rs for pattern grammar without breaking existing code.

**Resolution**: Create separate pattern_parser.rs (see Decision 1)

**Long-term**: Migrate query_parser.rs to use PatternChainBuilder

---

### Concern 2: Test Coverage for New Features

**Issue**: Grammar V1.2 adds features not covered by existing tests:
- Array access expressions
- PARTITION BY
- Event-count WITHIN
- OUTPUT event types
- EVERY multi-instance

**Impact**: Cannot validate grammar implementation without tests.

**Resolution**: Create test suite BEFORE implementing parser (TDD approach)

**Test Categories**:
1. Parser tests (syntax validation, AST generation)
2. Compiler tests (AST → Query API → Runtime)
3. Runtime tests (execution correctness)
4. Integration tests (end-to-end queries)

---

### Concern 3: StateEvent Position Mapping

**Issue**: Grammar allows unlimited aliases (e1, e2, ..., eN) but StateEvent has fixed Vec<Option<StreamEvent>>.

**Current**: Position-based mapping (e1 → position 0, e2 → position 1)

**Question**: What's the max number of streams supported?

**Impact**: Need to validate pattern complexity doesn't exceed Vec capacity.

**Resolution**:
- Document max streams per pattern (e.g., 16 or 32)
- Reject patterns exceeding limit in compiler
- Or use dynamic Vec (already the case?)

---

## Test Coverage Gaps

### Missing Test Categories

#### 1. Array Access Tests
- `e[0]` first event access
- `e[last]` last event access
- `e[1]`, `e[2]` specific index access
- Out-of-bounds returns null
- Array access in WHERE, SELECT, HAVING clauses
- Nested attributes: `e[0].user.location.city`

#### 2. PARTITION BY Tests
- Single partition key
- Multiple partition keys
- Per-partition pattern isolation
- Partition cleanup on expiration
- High cardinality partitions (10K+ partitions)
- Partition key null handling

#### 3. Event-Count WITHIN Tests
- Pattern completes within event count
- Pattern fails when event count exceeded
- Combination with time-based WITHIN
- Zero event count handling

#### 4. EVERY Multi-Instance Tests
- Overlapping instances
- Multiple matches output
- EVERY with count quantifiers
- EVERY validation (only PATTERN mode, only top-level)

#### 5. OUTPUT Event Types Tests
- CURRENT EVENTS (default)
- EXPIRED EVENTS (timeout)
- ALL EVENTS (both)
- Window eviction events

#### 6. Cross-Stream Reference Tests
- e2[price > e1.price]
- Complex expressions: e3[value > e1.value + e2.value]
- NULL handling in cross-stream refs
- Out-of-order event handling

#### 7. Logical Operator Tests
- AND combinations
- OR combinations
- Mixed AND/OR with precedence
- Nested logical patterns

#### 8. Absent Pattern Tests (Phase 3)
- NOT ... FOR duration
- Absent at end of sequence
- Multiple absent patterns
- Absent timeout expiration

---

## Implementation Roadmap

### Phase 1: Parser Foundation (Week 1-2)

**Goal**: Basic pattern statement parser working

**Tasks**:
1. Create `pattern_parser.rs` with EventFluxDialect
2. Parse `FROM PATTERN (...)` and `FROM SEQUENCE (...)`
3. Parse basic stream references: `StreamName`, `alias=StreamName`
4. Parse sequence operator: `e1=A -> e2=B`
5. Parse filter conditions: `Stream[expression]`
6. Parse time expressions: `10 minutes`, `24 hours`
7. Parse WITHIN clause: `WITHIN duration`
8. Parse SELECT and INSERT clauses
9. Basic AST → Query API converter

**Deliverable**: Can parse simple pattern queries like:
```sql
FROM PATTERN (
    e1=LoginStream -> e2=LogoutStream
    WITHIN 30 minutes
)
SELECT e1.userId, e2.timestamp
INSERT INTO SessionLogs;
```

---

### Phase 2: Count Quantifiers & Array Access (Week 3-4)

**Goal**: Support count-based patterns with array access

**Tasks**:
1. Parse count quantifiers: `{n,m}`, `+`, `*`, `?`
2. Implement array access parser: `e[0]`, `e[last]`
3. Create `ArrayIndexExecutor` for runtime evaluation
4. Wire StateEvent context to expression evaluators
5. Comprehensive tests for array access

**Deliverable**: Can parse and execute:
```sql
FROM PATTERN (
    e1=FailedLogin{3,5} -> e2=AccountLocked
)
SELECT e1[0].timestamp, e1[last].timestamp, count(e1)
INSERT INTO SecurityAlerts;
```

---

### Phase 3: Logical Operators & Filters (Week 5)

**Goal**: Complex pattern expressions

**Tasks**:
1. Verify LogicalStateProcessor exists and works
2. Parse AND/OR operators with precedence
3. Enhance filter parsing for cross-stream references
4. Wire StateEvent context to filter evaluation
5. Tests for logical combinations

**Deliverable**: Can parse and execute:
```sql
FROM PATTERN (
    (e1=Login AND e2=VPNConnect) ->
    e3=DataAccess[bytes > e1.threshold]
)
SELECT e1.userId, e2.vpnLocation, e3.bytes
INSERT INTO SuspiciousActivity;
```

---

### Phase 4: EVERY & Multi-Instance (Week 6-7)

**Goal**: Support overlapping pattern instances

**Tasks**:
1. Verify EveryStateProcessor exists
2. Implement multi-instance state management (if needed)
3. Parse EVERY syntax with parentheses
4. Validate EVERY restrictions (PATTERN only, top-level only)
5. Tests for overlapping instances

**Deliverable**: Can parse and execute:
```sql
FROM PATTERN (
    EVERY (e1=StockPrice{5,10})
)
SELECT e1[0].price, e1[last].price, avg(e1.price)
INSERT INTO PriceRanges;
```

---

### Phase 5: PARTITION BY (Week 8-10)

**Goal**: Multi-tenant pattern isolation

**Tasks**:
1. Design PartitionedStreamProcessor wrapper
2. Implement partition key extraction
3. Implement processor instance manager
4. Implement partition cleanup policy
5. Parse PARTITION BY clause
6. Comprehensive tests (single key, multi-key, high cardinality)

**Deliverable**: Can parse and execute:
```sql
FROM PATTERN (
    EVERY (e1=FailedLogin{3,5} -> e2=AccountLocked)
    WITHIN 10 minutes
)
PARTITION BY userId
SELECT userId, e1[0].timestamp, count(e1)
INSERT INTO SecurityAlerts;
```

---

### Phase 6: Advanced Features (Week 11-12)

**Goal**: OUTPUT types, event-count WITHIN, aggregations

**Tasks**:
1. Wire OutputEventType to pattern runtime
2. Implement event-count WITHIN
3. Verify aggregations over collections
4. Parse INSERT ALL EVENTS / EXPIRED EVENTS
5. Parse aggregation functions in SELECT
6. Comprehensive integration tests

**Deliverable**: Full Grammar V1.2 support except absent patterns

---

### Phase 7: Absent Patterns (Week 13-15)

**Goal**: Temporal absence detection

**Tasks**:
1. Implement TimerWheel infrastructure
2. Implement AbsentStreamPreStateProcessor
3. Parse NOT ... FOR syntax
4. Validate absent pattern restrictions
5. Tests for timeout detection

**Deliverable**: Full Grammar V1.2 implementation complete

---

## Summary

### Runtime Status (2025-12-04)

Runtime complete (parser needed):
- Pre/Post state processor architecture (195 tests)
- Count quantifiers A{n}, A{m,n} (52 tests)
- Pattern chaining A -> B -> C (24 tests)
- Array access e[0], e[last], e[n] (14+ tests)
- Cross-stream references e2[price > e1.price] (6 tests)
- EVERY multi-instance with sliding window (10 tests)
- Logical operators AND, OR (16 tests)
- Collection aggregations count, sum, avg, min, max, stdDev (50+ tests)
- Time-based WITHIN constraints (3+ tests)

Runtime not implemented:
- PARTITION BY - Multi-tenant isolation
- Absent patterns (NOT ... FOR) - Requires TimerWheel
- Event-count WITHIN - Only time-based exists
- OUTPUT event types - Enum exists but not wired

### Effort Estimate

- P0 + P1 (parser only): 2-3 weeks
- P2 (runtime + parser): 4-6 weeks
- Total for full Grammar V1.2: 6-9 weeks

### Parser Deliverable

With 2-3 weeks of parser work, can execute:
- Sequences with -> operator
- Count quantifiers with array access
- Cross-stream filter conditions
- Time-based WITHIN constraints
- EVERY patterns with overlapping
- Logical operators AND, OR
- Collection aggregations

---

**Document Version**: 1.2
**Last Updated**: 2025-12-04
