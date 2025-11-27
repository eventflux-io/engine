# Grammar V1.2 Implementation Gap Analysis

**Version**: 1.0
**Date**: 2025-11-23
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

### Overall Readiness: ~35% Complete

**Current Implementation Status**:
- ✅ **Phase 1 COMPLETE**: Pre/Post state processor architecture (195 tests)
- ✅ **Phase 2a COMPLETE**: Count quantifiers for single patterns (52 tests)
- ✅ **Phase 2b COMPLETE**: Pattern chaining with `->` operator (24 tests)
- ❌ **Phase 3 NOT STARTED**: Absent patterns, EVERY support
- ❌ **Phase 4 NOT STARTED**: Advanced features

**Total Tests Passing**: 271 tests (pattern-specific)

**Critical Blockers for Grammar Integration**:
1. ⚠️ **Parser does not exist** - No SQL parser for Pattern Grammar V1.2
2. ⚠️ **Array access expressions** - `e[0]`, `e[last]` not implemented
3. ✅ **Logical operators runtime** - AND, OR via PatternChainBuilder.add_logical_group() (2025-11-25)
4. ⚠️ **PARTITION BY runtime** - Multi-tenant isolation not implemented
5. ⚠️ **Event-count WITHIN** - Only time-based WITHIN exists
6. ✅ **EVERY multi-instance** - COMPLETE! (verified 2025-11-26)
   - Basic overlapping: A1→A2→B3 correctly produces 2 matches (A1-B3, A2-B3)
   - Sliding window with count quantifiers: EVERY A{2,3}→B with 4 A events produces 5 outputs
   - See Section 7 for details
7. ⚠️ **Absent patterns runtime** - AbsentStreamStateElement exists but no processor
8. ⚠️ **OUTPUT event types** - OutputEventType enum exists but not wired to pattern runtime

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

**Query API Status**: ⚠️ PARTIAL
- No `ArrayIndexExpression` or similar in expression AST

**Runtime Status**: ❌ NOT IMPLEMENTED
- `StateEvent.get_event_chain(position)` returns `Vec<&StreamEvent>` - data is available
- No expression evaluator for `e[0]`, `e[last]` syntax

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for array index syntax

**Expression Evaluator Status**: ❌ NOT IMPLEMENTED
- Need `ArrayIndexExecutor` to evaluate `e[index]`
- Must support:
  - `e[0]` → first event
  - `e[1]`, `e[2]`, ... → specific index
  - `e[last]` → last event (dynamic resolution)
  - Out-of-bounds → returns `null` (not error)

**Gap**:
- **CRITICAL**: Entire array access feature missing
  1. Parser: Parse `alias[index].attribute` syntax
  2. Query API: Add `ArrayIndexExpression` AST node
  3. Executor: Implement `ArrayIndexExecutor`
  4. Integration: Wire to StateEvent.get_event_chain()

**Implementation Effort**: **Large** (5-7 days)
- Parser: 1-2 days (conflict resolution with float literals like `1.0`)
- Query API: 1 day (new AST node)
- Executor: 2-3 days (array indexing, `last` keyword, null handling)
- Tests: 1-2 days (comprehensive edge cases)

**Dependencies**:
- StateEvent infrastructure (COMPLETE ✅)
- Expression evaluator framework

**Priority**: **P0** - Required for SELECT clause with count quantifiers

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

**Query API Status**: ✅ WORKS
- VariableExpressionExecutor supports cross-stream references via position array

**Runtime Status**: ⚠️ PARTIALLY WORKS
- SELECT clause: VariableExpressionExecutor can access any stream in StateEvent ✓
- Filter conditions: condition_fn signature only accepts &StreamEvent, NOT &StateEvent ✗
- StateEvent has multi-stream tracking, infrastructure is ready

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for filter conditions `[expression]`
- No parser for cross-stream references `e1.attribute`

**Critical Gap**: Filter conditions cannot access previous events
- Current: `condition_fn: Fn(&StreamEvent) -> bool`
- Required: `condition_fn: Fn(&StateEvent, &StreamEvent) -> bool`
- Location: `src/core/query/input/stream/state/stream_pre_state_processor.rs:109, 245-249, 519`
- Impact: Patterns like `e2[price > e1.price]` will NOT work in filters

**Gap**:
1. **CRITICAL**: Update condition_fn signature to accept StateEvent context
2. **CRITICAL**: Wire expression executors to filter evaluation
3. **Parser**: Parse `[expression]` filter syntax
4. **Parser**: Parse `alias.attribute` cross-stream references

**Implementation Effort**: **Large** (5-7 days)
- Update condition signature: 1-2 days
- Expression executor integration: 2-3 days
- Parser: 2-3 days
- Tests: 2 days

**Dependencies**:
- StateEvent infrastructure (COMPLETE ✅)
- Expression evaluator framework (COMPLETE ✅)
- Condition signature update (REQUIRED - P0)

**Priority**: **P0** - CRITICAL BLOCKER for pattern filters with cross-stream references

**See**: `feat/pattern_processing/CROSS_STREAM_REFERENCES_ANALYSIS.md` for detailed analysis

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

### 15. Aggregation Functions

**Grammar Requirement**:
```sql
SELECT count(e1) as attempts,
       avg(e1.price) as avgPrice,
       max(e1.temp) - min(e1.temp) as range
```

**Query API Status**: ✅ LIKELY COMPLETE
- Aggregation functions exist for windows, likely work for patterns

**Runtime Status**: ⚠️ UNCLEAR
- Need to verify aggregations work over event collections (count quantifiers)
- `count(e1)` should count events in collection
- `avg(e1.attr)` should average attribute over collection

**Parser Status**: ❌ NOT IMPLEMENTED
- No parser for aggregation functions in SELECT clause

**Expression Evaluator Status**: ⚠️ UNCLEAR
- Need to verify aggregation executors can work with StateEvent collections

**Gap**:
1. **Verify aggregation over collections** - Test count(e1), avg(e1.attr)
2. **Parser**: Parse aggregation functions in SELECT clause

**Implementation Effort**: **Medium** (3-5 days)
- Investigation: 1 day
- Runtime (if needed): 2-3 days
- Parser: 1 day
- Tests: 1 day

**Dependencies**:
- Expression evaluator framework
- StateEvent infrastructure (COMPLETE ✅)

**Priority**: **P1** - Important for analytics, but simple projections work for basic cases

---

## Implementation Priorities

### Priority 0 (P0): Critical Blockers - Must Have Before Grammar Integration

**Timeline**: 3-4 weeks

| Feature | Effort | Status | Notes |
|---------|--------|--------|-------|
| **Parser Foundation** | 1 week | ❌ | EventFluxDialect, basic pattern statement parser |
| **PATTERN/SEQUENCE modes** | 2-3 days | ❌ | Parse FROM PATTERN/SEQUENCE |
| **Sequence operator (->)** | 1 day | ❌ | Parse -> and map to NextStateElement |
| **Count quantifiers** | 2 days | ❌ | Parse {n,m}, +, *, ? syntax |
| **Event aliases** | 1 day | ❌ | Parse e1=StreamName, StreamName AS e1 |
| **Filter conditions** | 1-3 days | ❌ | Parse [expression] syntax |
| **Time expressions** | 1 day | ❌ | Parse n time_unit |
| **Time-based WITHIN** | 1-2 days | ❌ | Parse WITHIN duration |
| **Array access expressions** | 5-7 days | ❌ | Parse e[0], e[last], implement executor |

**Total P0 Effort**: ~3-4 weeks (15-20 days)

**Deliverable**: Can parse and execute basic pattern queries with count quantifiers, sequences, filters, and array access.

---

### Priority 1 (P1): Important Features - Should Have Soon

**Timeline**: 2-3 weeks

| Feature | Effort | Status | Notes |
|---------|--------|--------|-------|
| **Logical operators (AND, OR)** | 1 day | ✅ Runtime COMPLETE | Parser only - PatternChainBuilder.add_logical_group() ready (2025-11-25) |
| **EVERY multi-instance** | 1-2 days | ✅ Runtime COMPLETE | Parser only - runtime complete, 10 tests passing (2025-11-25) |
| **Cross-stream references** | 5-7 days | ❌ | Parse e1.attr in e2 filters |
| **PARTITION BY** | 10-15 days | ❌ | Major architecture, multi-tenant isolation |
| **Aggregation functions** | 3-5 days | ⚠️ | Verify over collections, parse SELECT |

**Total P1 Effort**: ~3-4 weeks (17-29 days)

**Deliverable**: Production-ready pattern processing with multi-instance, partitioning, and aggregations.

---

### Priority 2 (P2): Advanced Features - Nice to Have

**Timeline**: 3-4 weeks

| Feature | Effort | Status | Notes |
|---------|--------|--------|-------|
| **Absent patterns (NOT ... FOR)** | 7-10 days | ❌ | Requires TimerWheel, Phase 3 |
| **Event-count WITHIN** | 3-4 days | ❌ | Event counter per instance |
| **OUTPUT event types** | 3-4 days | ❌ | Wire to pattern runtime |

**Total P2 Effort**: ~2-3 weeks (13-18 days)

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

### Critical Gaps Identified

1. No parser exists - Entire Grammar V1.2 parser must be built
2. Array access missing - Critical for count quantifiers in SELECT
3. PARTITION BY missing - Major architecture work needed
4. ✅ **EVERY runtime WORKING** - Basic true overlapping verified (2025-11-25)
5. ✅ **Logical operators WORKING** - PatternChainBuilder.add_logical_group() added (2025-11-25)
6. Event-count WITHIN missing - New runtime feature needed
7. Absent patterns not started - Phase 3 work, requires TimerWheel

### Total Effort Estimate

- **P0 (Critical)**: 3-4 weeks
- **P1 (Important)**: 3-4 weeks
- **P2 (Advanced)**: 2-3 weeks
- **Total**: ~8-11 weeks for full Grammar V1.2 implementation

**Status Update** (2025-11-26):
- ✅ **EVERY TRUE OVERLAPPING VERIFIED WORKING**
  - Test: A1→A2→B3 correctly produces 2 matches (A1-B3, A2-B3)
  - See test `test_true_every_overlapping_multiple_a_before_b`
- ✅ **SLIDING WINDOW WITH COUNT QUANTIFIERS NOW WORKING** (2025-11-26)
  - EVERY A{3}→B with 5 A events produces 3 sliding windows
  - 7 new tests in count_pre_state_processor.rs
  - See `feat/pattern_processing/EVERY_REFERENCE.md` for full details
- ✅ **LOGICAL OPERATORS (AND/OR) NOW FULLY SUPPORTED**
  - PatternChainBuilder.add_logical_group() method added
  - LogicalGroupConfig with and()/or() helpers
  - 16 new unit tests for logical groups
  - See `feat/pattern_processing/EVERY_REFERENCE.md` for full details
- All EVERY runtime features are complete - only parser needed
- See `feat/pattern_processing/EVERY_REFERENCE.md` for full details

### Recommended Approach

1. **Start with P0 features** - Get basic patterns working (4 weeks)
2. **Add P1 features incrementally** - Production readiness (6 weeks)
3. **P2 features as needed** - Based on user demand (3 weeks)

**Minimum Viable Grammar**: P0 features (4 weeks) enables:
- Basic sequences with `->` operator
- Count quantifiers with array access
- Time-based WITHIN constraints
- Filter conditions
- Event aliases

This is sufficient for ~70% of pattern processing use cases.

---

**Next Steps**:
1. Review this gap analysis with team
2. Prioritize features based on user requirements
3. Create detailed parser design document
4. Implement Phase 1: Parser Foundation
5. TDD approach: Tests first, then implementation

---

**Document Version**: 1.0
**Status**: Ready for Review
**Author**: EventFlux Pattern Processing Team
**Review Date**: TBD
