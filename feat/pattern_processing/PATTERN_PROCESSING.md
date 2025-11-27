# EventFlux Pattern Processing - Design & Requirements Document

**Document Version**: 2.4 (Phase 1 & 2 Complete)
**Created**: 2025-10-26
**Last Updated**: 2025-11-05
**Status**: Phase 1 & 2 Complete, Phase 3-4 Planned
**Target Milestone**: M2 (Pattern Processing) - Phase 2 complete, Phase 3-4 planned for M5+

---

## ⚠️ CRITICAL: DOCUMENT PURPOSE

**This is a DESIGN & REQUIREMENTS document tracking pattern processing implementation progress.**

**Current Status (Updated 2025-11-05)**:
- Phase 1 COMPLETE: Pre/Post state processor architecture (195 tests passing)
- Phase 2 COMPLETE: Count quantifiers with pattern chaining (52 + 24 = 76 tests passing)
- Total pattern processing tests: 271 tests passing (195 + 76)
- Total codebase tests: 1,436 tests passing

**Phase 1 Implemented**:
- StateEvent structure with multi-stream tracking
- StreamPreStateProcessor/StreamPostStateProcessor (Pattern & Sequence semantics)
- LogicalPreStateProcessor/LogicalPostStateProcessor (AND/OR patterns)
- Receiver infrastructure (PatternStreamReceiver, SequenceStreamReceiver)
- WITHIN time constraints with startStateIds
- 'every' pattern foundation (copy_state_event_for_every)
- Error handling (no unwrap() panics, mutex poisoning protection)

**Phase 2 Implemented**:
- CountPreStateProcessor for count quantifiers (A{3}, A{2,5})
- CountPostStateProcessor for validation
- PatternChainBuilder factory for multi-processor chains
- Event chaining (add_event, remove_last_event)
- Pattern chaining (A -> B -> C with count quantifiers)
- WITHIN time constraints in pattern chains (reactive validation)
- Pattern vs Sequence modes in chains
- Multi-instance pattern matching

**Phase 3-4 NOT IMPLEMENTED**: Absent patterns, advanced features planned for M5+

**What This Document Provides**:
- Phase 1 & 2 completion status and achievements
- Architecture design for Phases 3-4 implementation
- Requirements and syntax specifications
- Implementation roadmap for remaining phases
- Component specifications and test strategy

---

## Table of Contents

1. [Current Status & Phase 1 Completion](#current-status--phase-1-completion)
2. [Phase 1 Achievements](#phase-1-achievements)
3. [What Exists](#what-exists)
4. [What Was Deleted (Path A)](#what-was-deleted-path-a)
5. [Architecture Design (Implemented & Future)](#architecture-design-implemented--future)
6. [Implementation Phases](#implementation-phases)
7. [Grammar & Syntax Requirements](#grammar--syntax-requirements)
8. [Testing Strategy](#testing-strategy)
9. [Success Criteria](#success-criteria)

---

## Current Status & Phase 1 Completion

### ✅ Phase 1: COMPLETE (2025-11-02)

**Implementation Status**: Foundation architecture implemented
**Test Results**: 984 tests passing, 0 failed
**Duration**: ~3 hours implementation + verification
**Test Success Rate**: 100% (0 failures)

### Phase 1 Implementation Summary

**What Was Implemented**:
```
src/core/event/state/
├── state_event.rs                      ✅ Multi-stream state tracking
├── state_event_cloner.rs               ✅ StateEvent cloning with 'every' support
└── state_event_factory.rs              ✅ StateEvent creation

src/core/query/input/stream/state/
├── pre_state_processor.rs              ✅ Core pattern matching trait
├── post_state_processor.rs             ✅ Match handling trait
├── stream_pre_state.rs                 ✅ Three-list state management
├── stream_pre_state_processor.rs       ✅ Base Pre implementation (Pattern/Sequence)
├── stream_post_state_processor.rs      ✅ Base Post implementation
├── logical_pre_state_processor.rs      ✅ AND/OR pattern support
├── logical_post_state_processor.rs     ✅ AND/OR match handling
├── inner_state_runtime.rs              ✅ Runtime lifecycle management
├── stream_inner_state_runtime.rs       ✅ Runtime implementation
├── state_stream_runtime.rs             ✅ Runtime wrapper with resetAndUpdate
└── receiver/
    ├── pattern_stream_receiver.rs      ✅ Pattern stabilization
    └── sequence_stream_receiver.rs     ✅ Sequence stabilization
```

### What Works Now (Phase 1)

**Core Architecture** ✅:
- Pre/Post state processor separation
- Processor chaining (Pre→Post→Pre→Post→...)
- StateEvent multi-stream tracking
- Event lifecycle management

**Pattern Types** ✅:
- Pattern semantics: Multi-match capability, keeps pending states
- Sequence semantics: Single-match, clears states after match
- AND patterns: Both sides must match
- OR patterns: Either side matching is sufficient

**Time Constraints** ✅:
- WITHIN time windows with startStateIds
- Event expiration tracking
- ComplexEventType::Expired marking

**'every' Pattern Foundation** ✅:
- copy_state_event_for_every() with new ID generation
- State clearing for pattern restart
- Processor chaining for continuous matching

**Error Handling** ✅:
- No unwrap() panics
- Mutex poisoning protection
- Test coverage: 984 tests
- Integration tests: 6 tests for processor chains

### Phase 2 Complete (2025-11-05)

**Count Quantifiers Implemented**:
- Single pattern count quantifiers: A{3}, A{2,5} (52 tests)
- Pattern chaining: A{2} -> B{2} -> C{2} (24 tests)
- Event chaining at positions (add_event, remove_last_event)
- Min/max count validation
- Multi-instance pattern matching
- WITHIN time constraints in chains

**Grammar Integration Notes**:
- A+ (one or more), A* (zero or more) are **NOT SUPPORTED** (unbounded/zero-count patterns rejected)
- Grammar parser integration with CountPreStateProcessor (bounded patterns only: A{n}, A{m,n} where min >= 1 and max is explicit)

### What Does NOT Work Yet (Phases 3-4)

**Phase 3 - Absent Patterns** (Planned for M5+):
- NOT(A) FOR duration - Absence detection
- Scheduler integration with TimerWheel
- Time-based triggers for proactive expiry

**Phase 4 - Advanced Features** (Planned for M5+):
- Cross-stream references: e2[price > e1.price]
- Collection indexing: e[0], e[last]
- Complex nested combinations

---

## Phase 1 Achievements

### Session 2 Completion Summary (2025-11-02)

**All P0 Critical Fixes Applied** ✅:
1. ✅ Deprecated old architecture (logical_processor.rs, sequence_processor.rs)
2. ✅ Fixed trait return types (Pre/Post processor linking)
3. ✅ Added `this_state_post_processor` field to StreamPreStateProcessor
4. ✅ Added forwarding logic in `process_and_return()`
5. ✅ Fixed StreamPostStateProcessor delegation
6. ✅ Updated LogicalPre/PostStateProcessor trait implementations
7. ✅ Fixed StateEvent ID cloning for 'every' patterns
8. ✅ Fixed Pattern semantics (don't remove from pending)

**All P1 Design Improvements Applied** ✅:
1. ✅ **Remove magic numbers**: Changed `within_time: i64` to `Option<i64>`
2. ✅ **Add startStateIds**: Correct WITHIN time constraint checking
3. ✅ **Use ComplexEventType::Expired**: Better event lifecycle tracking
4. ✅ **Add success_condition**: Field for count quantifiers (Phase 2 ready)
5. ✅ **Reduce cloning**: Optimized StreamPostStateProcessor (1-2 clones vs 3, 33-50% reduction)

### Critical Deadlock Resolution (2025-11-02)

**Problem**: Pre->Post->Pre processor chain deadlocked due to circular Arc<Mutex<T>> references.

**Root Cause**:
- Java Siddhi uses ReentrantLock + direct object references: `thisStatePreProcessor.stateChanged()` requires no additional locking
- Rust requires locking Arc to call ANY method: `arc.lock().unwrap().state_changed()` causes deadlock when Arc already locked

**Solution F (Implemented)**: Lock-Free Shared State
```rust
struct ProcessorSharedState {
    state_changed: AtomicBool,  // Lock-free atomic flag
}
// Both Pre and Post processors share this state
// PostStateProcessor marks state changed WITHOUT locking PreStateProcessor Arc
```

**Key Insights**:
- Rust's Arc<Mutex<T>> pattern fundamentally incompatible with Java's callback-based architecture
- Mutex in Rust is NOT reentrant (unlike Java's ReentrantLock)
- Solution: Use atomic operations for state coordination instead of mutex callbacks
- Automatic wiring: `set_this_state_pre_processor()` automatically retrieves shared state (compiler-enforced correctness)

**Result**: All processor chain tests passing (48 Pre + 14 Post + 6 integration tests)

**Integration Tests Added** ✅:
1. ✅ Pre→Post→Pre chain - Processor chaining architecture test
2. ✅ A→B→C sequence - Three-step sequence with StateEvent expansion
3. ✅ WITHIN timing - Time constraint verification with expiration
4. ✅ Pattern vs Sequence - Semantic difference verification
5. ✅ success_condition - Count quantifier flag testing
6. ✅ startStateIds - WITHIN constraint with start state checking

**Metrics**:
| Metric | Value | Change |
|--------|-------|--------|
| **Total Tests** | 984 | +6 |
| **Passing Tests** | 984 | 100% |
| **Test Failures** | 0 | 0 |
| **Architecture Issues** | 0 | -11 |
| **Magic Numbers** | 0 | -1 |

**Code Quality Checks**:
- Code formatted (`cargo fmt`)
- Clippy lints addressed
- No dead code warnings in pattern processing modules
- TODOs documented for Phase 2 work
- Error handling: no unwrap() panics, mutex poisoning protection

---

## Phase 2 Achievements

### Phase 2a: Single Pattern Count Quantifiers (2025-11-04)

**Implementation Summary**:
- CountPreStateProcessor for single patterns (A{3}, A{2,5})
- CountPostStateProcessor for validation
- Event chaining at StateEvent positions
- Min/max count tracking
- Backtracking support (add_event, remove_last_event)

**Tests**: 52 passing (count_pre_state_processor tests)

**Key Components**:
- `src/core/query/input/stream/state/count_pre_state_processor.rs` - Count matching logic
- `src/core/query/input/stream/state/count_post_state_processor.rs` - Count validation

### Phase 2b: Pattern Chaining (2025-11-05)

**Implementation Summary**:
- PatternChainBuilder factory for multi-processor chains
- Multi-processor wiring (PreA → PostA → PreB → PostB → PreC)
- Validation rules (first min>=1, last exact, all min>=1, no optional steps)
- Pattern chaining with count quantifiers (A{2} -> B{2} -> C{2})
- WITHIN time constraints in chains (reactive validation)
- Pattern vs Sequence modes in chains
- Multi-instance pattern matching

**Tests**: 24 passing + 1 ignored
- Phase 2b.1: Basic two-step chains (7 tests)
- Phase 2b.2: Three-step chains (5 tests)
- Phase 2b.3: Pattern mode (4 tests)
- Phase 2b.4: WITHIN time constraints (3 tests + 1 ignored for Phase 3)
- Phase 2b.5: Integration testing (5 tests)

**Key Components**:
- `src/core/query/input/stream/state/pattern_chain_builder.rs` (511 lines) - Factory and validation
- `tests/common/pattern_chain_test_utils.rs` (257 lines) - Shared test utilities

**Code Quality**:
- Eliminated 470+ lines of duplicate code across test files
- Created common test utilities module
- Cleanup: Deleted 782 lines of obsolete code and duplicates
- Full test suite: 1,436 tests passing after cleanup

**Architectural Decisions**:
- No optional steps (B{0,0} rejected - all steps must have min_count >= 1)
- Last step must have exact count (min == max)
- WITHIN checked reactively (proactive expiry deferred to Phase 3)
- Stream routing at runtime level (not within processors)

**Remaining Work**:
- Query parser migration from deprecated processors (8-12 hours)
- Delete deprecated logical_processor.rs and sequence_processor.rs after migration
- Grammar integration (A+, A* syntax) deferred to M3

---

### Query API Layer

**Location**: `src/query_api/execution/query/input/state/`

**Status**: ✅ Query API state element definitions exist:

```rust
pub enum StateElement {
    Stream(StreamStateElement),           // ✅ Defined
    AbsentStream(AbsentStreamStateElement), // ✅ Defined (no runtime)
    Logical(LogicalStateElement),         // ✅ Defined
    Next(Box<NextStateElement>),          // ✅ Defined
    Count(CountStateElement),             // ✅ Defined (no runtime)
    Every(Box<EveryStateElement>),        // ✅ Defined (no runtime)
}
```

**Interpretation**: The query API **definitions** exist, but the **runtime processors** to execute them do NOT exist (were deleted in Path A cleanup).

---

## What Exists

### Core State Processors (Production Quality)

**logical_processor.rs** (modified, tracked):
- AND/OR logical combinations
- Production-safe defensive error handling (no `.unwrap()` panics)
- Mutex poisoning protection with logging
- Basic state management

**sequence_processor.rs** (modified, tracked):
- Basic sequence processing (A -> B)
- Pattern vs Sequence semantics
- Production-safe defensive error handling
- Event cloning and state management

**Key Improvements Applied**:
```rust
// Defensive error handling example:
match next.lock() {
    Ok(mut processor) => processor.process(Some(Box::new(se))),
    Err(e) => {
        error!("Mutex poisoned: {}", e);
        // Graceful degradation instead of panic
    }
}
```

### Preserved Components (For Future Use)

**timers/timer_wheel.rs** (313 lines, 8 tests):
- O(1) timer wheel scheduling
- Will be needed for Phase 3 absent patterns (NOT operator)
- Time-based triggers for `NOT(A) FOR 10s` patterns

**util/event_store.rs** (216 lines, 9 tests):
- ID-based event storage (8 bytes vs 100 bytes)
- Memory optimization: 12.5x reduction
- Will be used if profiling shows memory pressure

---

## What Was Deleted (Path A)

### Path A: Scorched Earth Cleanup (2025-10-31)

**Reason**: Removed ~14,000 lines of premature optimization and over-engineering that added complexity without proven benefit.

**Deleted Components**:

1. **Pattern API v2** (8 files, ~2,300 lines):
   - pattern_builder.rs (880 lines, had flaky perf tests)
   - pattern_error.rs, pattern_handle.rs, filter_predicate.rs
   - state_config.rs, state_config_builder.rs, state_context.rs
   - output_projection.rs

2. **Abstraction Layer** (5 files, ~1,800 lines):
   - pre_state_processor.rs (369 lines)
   - post_state_processor.rs (461 lines)
   - stream_pre_state_processor.rs (193 lines)
   - stream_post_state_processor.rs (264 lines)
   - stream_receiver.rs (547 lines)

3. **State Management Optimizations** (6 files, ~2,500 lines):
   - time_bucketed_storage.rs (419 lines)
   - optimized_sequence_processor.rs (528 lines)
   - count_pattern_state.rs (434 lines)
   - lazy_pattern_combinations.rs (274 lines)
   - hierarchical_timer_wheel.rs (486 lines)

4. **Premature Optimizations** (3 files, ~1,600 lines):
   - batch_processor.rs (285 lines)
   - state_arena.rs (742 lines)
   - lock_free_receiver.rs (601 lines - contradicted exactly-once semantics)

5. **Documentation** (26 files, ~13,000 lines):
   - Entire feat/pattern_processing/ directory with aspirational completion claims

**Total Deleted**: ~8,200 lines of code

**Why Deleted**:
- Premature optimization without proven bottlenecks
- Over-engineered APIs with no user demand
- Complexity without clear value
- Lock-free architecture contradicted exactly-once semantics promise
- Failed the "profile before optimizing" principle

**Philosophy**: "Perfect is the enemy of good" - Return to simple, clean architecture. Build incrementally when ROADMAP prioritizes it (M5+).

### Thorough Cleanup: Core Event Optimizations Also Reverted

After deleting pattern processing code, we also reverted uncommitted "optimizations" to core event structures that were part of the pattern processing work:

**Arc Wrapping Removed** (7 files reverted):
- `state_event.rs`: `Vec<Option<Arc<StreamEvent>>>` → `Vec<Option<StreamEvent>>`
- Removed `Arc::make_mut` complexity and `&**arc` dereferencing
- Benefit: Direct ownership, simpler code, no reference counting overhead

**Event IDs Removed** (2 files cleaned):
- `stream_event.rs`: Removed `id: u64` field and `AtomicU64` counter
- Saved: 8 bytes per event + atomic operations
- Benefit: Significant memory savings, no atomic overhead on hot path

**Why Reverted**:
- No shared ownership requirements in clean architecture
- Premature optimization without proven bottlenecks
- Added complexity (Arc indirection, atomic ops) without measured benefit
- Following "profile before optimizing" discipline

**Result**: 813 tests passing with simpler, more maintainable core event structures.

---

## Architecture Design (Future)

**Note**: This section describes the INTENDED architecture for future implementation. This is NOT currently implemented.

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Pattern Query                             │
│  FROM PATTERN (A -> B<2:5> -> NOT(C) FOR 10s -> D)              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      SQL Parser / Query API                      │
│  StateInputStream → StateElement Tree → Pattern AST              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Pattern Runtime Engine (FUTURE)                 │
│  Pre-State Processors → State Event Buffer → Post-State          │
│  Pattern matching state machine with transitions                 │
└─────────────────────────────────────────────────────────────────┘
```

### State Machine Model

Pattern processing uses a **Finite State Automaton (FSA)**:

```
Pattern: A -> B<2:5> -> NOT(C) FOR 10s -> D

State Machine:
[Start] --A--> [State1] --B(count 2-5)--> [State2] --NOT C (10s)--> [State3] --D--> [Accept]
   │              │              │                       │              │
   └──────────────┴──────────────┴───────────────────────┴──────────────┴─> [Reject]
```

### Component Hierarchy (Future Design)

```rust
// Trait hierarchy (TO BE IMPLEMENTED)
trait PreStateProcessor {
    fn process_and_return(&self, chunk: ComplexEventChunk) -> ComplexEventChunk<StateEvent>;
    fn add_state(&mut self, state_event: StateEvent);
    fn update_state(&mut self);
    fn reset_state(&mut self);
    fn is_start_state(&self) -> bool;
}

trait PostStateProcessor {
    fn process(&mut self, state_event_chunk: ComplexEventChunk<StateEvent>);
    fn set_next_state_pre_processor(&mut self, pre: Arc<Mutex<dyn PreStateProcessor>>);
}

// Concrete implementations (TO BE IMPLEMENTED)
struct StreamPreStateProcessor { /* Stream matching */ }
struct CountPreStateProcessor { /* <n:m> quantifiers */ }
struct AbsentStreamPreStateProcessor { /* NOT operator */ }
struct LogicalPreStateProcessor { /* AND/OR */ }
```

---

## Java Siddhi Architecture Reference (Detailed Requirements)

**Purpose**: This section documents the complete Java Siddhi pattern processing architecture that serves as the reference implementation for EventFlux. This is the authoritative source for understanding what needs to be implemented.

**Source**: Analyzed from `references/siddhi/modules/siddhi-core/src/main/java/io/siddhi/core/query/input/stream/state/`

### Critical Architecture Insight

**Current Rust Gap**: ~85-95% of Java functionality missing

Java Siddhi pattern processing comprises:
- StateEvent structure for multi-stream pattern tracking
- Pre/Post state processor separation of concerns
- Three-list state management pattern (pending/new/current)
- State lifecycle with init/addState/updateState/resetState
- Processor chaining for complex patterns
- Scheduler integration for time-based triggers
- Full support for count quantifiers, absent patterns, 'every' patterns

### 1. StateEvent Structure (Foundation)

**Purpose**: Represents the state of a multi-stream pattern match in progress.

**Java Implementation** (`StateEvent.java`):
```java
public class StateEvent implements ComplexEvent {
    protected StreamEvent[] streamEvents;  // One per stream in pattern
    protected StateEvent next;             // Chain for event chunks
    protected long timestamp = -1;
    protected Type type = Type.CURRENT;
    protected Object[] outputData;         // Projection attributes

    // Key operations
    public void setEvent(int position, StreamEvent streamEvent);
    public void addEvent(int position, StreamEvent streamEvent);  // For count patterns
    public void removeLastEvent(int position);                    // Backtracking
}
```

**Why Critical**:
- Enables multi-stream patterns: `from every e1=Stream1 -> e2=Stream2[e1.price < price]`
- Each position in array corresponds to one stream in the pattern
- Positions can hold event chains for count quantifiers: `e1=Stream1{2,5}`
- Timestamp tracks when pattern completed
- Type tracks CURRENT/EXPIRED for window expiration

**Rust Requirements**:
```rust
pub struct StateEvent {
    stream_events: Vec<Option<StreamEvent>>,  // One per stream position
    next: Option<Box<StateEvent>>,            // For event chunks
    timestamp: i64,
    event_type: ComplexEventType,             // CURRENT/EXPIRED
    output_data: Vec<AttributeValue>,         // Projection
}

impl StateEvent {
    fn set_event(&mut self, position: usize, event: StreamEvent);
    fn add_event(&mut self, position: usize, event: StreamEvent);  // Append to chain
    fn remove_last_event(&mut self, position: usize);               // Backtrack
    fn get_stream_event(&self, position: usize) -> Option<&StreamEvent>;
    fn get_event_chain(&self, position: usize) -> Vec<&StreamEvent>;
    fn count_events_at(&self, position: usize) -> usize;
}
```

### 2. PreStateProcessor Interface (Core Pattern Matching)

**Purpose**: Receives events, manages state, checks pattern conditions, triggers matching.

**Java Interface** (`PreStateProcessor.java`):
```java
public interface PreStateProcessor extends Processor {
    // Core lifecycle
    void init();                              // Initialize processor
    void addState(StateEvent stateEvent);     // Add new candidate state
    void addEveryState(StateEvent stateEvent);// For 'every' patterns
    void updateState();                       // Move new → pending
    void resetState();                        // Clear after match

    // Event processing
    ComplexEventChunk<StateEvent> processAndReturn(ComplexEventChunk chunk);

    // Time management
    void setWithinTime(long withinTime);      // Set WITHIN constraint
    void expireEvents(long timestamp);        // Remove expired events

    // State machine
    int getStateId();                         // Which stream position
    boolean isStartState();                   // Is this first processor?
    StreamPostStateProcessor getThisStatePostProcessor();
}
```

**Concrete Implementations**:
1. **StreamPreStateProcessor**: Base for single stream patterns
2. **LogicalPreStateProcessor**: Extends Stream for AND/OR
3. **CountPreStateProcessor**: Handles count quantifiers (A{n}, A{m,n})
4. **AbsentStreamPreStateProcessor**: Handles NOT operator with scheduler

**Rust Requirements**:
```rust
pub trait PreStateProcessor: Processor {
    fn init(&mut self);
    fn add_state(&mut self, state_event: StateEvent);
    fn add_every_state(&mut self, state_event: StateEvent);
    fn update_state(&mut self);
    fn reset_state(&mut self);
    fn process_and_return(&mut self, chunk: ComplexEventChunk) -> ComplexEventChunk<StateEvent>;
    fn set_within_time(&mut self, within_time: i64);
    fn expire_events(&mut self, timestamp: i64);
    fn state_id(&self) -> usize;
    fn is_start_state(&self) -> bool;
    fn this_state_post_processor(&self) -> &dyn PostStateProcessor;
}
```

### 3. PostStateProcessor Interface (Match Handling)

**Purpose**: Handles successful pattern matches, chains to next processor or outputs result.

**Java Interface** (`PostStateProcessor.java`):
```java
public interface PostStateProcessor extends Processor {
    int getStateId();
    void setNextStatePreProcessor(PreStateProcessor next);       // Next in sequence
    void setNextEveryStatePreProcessor(PreStateProcessor next);  // For 'every'
    void setCallbackPreStateProcessor(CountPreStateProcessor callback);
}
```

**Java Implementation** (`StreamPostStateProcessor.java`):
```java
public class StreamPostStateProcessor implements PostStateProcessor {
    protected PreStateProcessor nextStatePreProcessor;      // A -> [B]
    protected PreStateProcessor nextEveryStatePreProcessor; // every A -> [every B]
    protected StreamPreStateProcessor thisStatePreProcessor;// Back reference
    protected Processor nextProcessor;                      // Final output
    protected boolean isEventReturned;

    protected void process(StateEvent stateEvent, ComplexEventChunk chunk) {
        thisStatePreProcessor.stateChanged();

        if (nextProcessor != null) {
            this.isEventReturned = true;  // Pattern complete
        }
        if (nextStatePreProcessor != null) {
            nextStatePreProcessor.addState(stateEvent);  // Continue pattern
        }
        if (nextEveryStatePreProcessor != null) {
            nextEveryStatePreProcessor.addEveryState(stateEvent);  // Restart 'every'
        }
    }
}
```

**Key Insight**: Post processor connects Pre processors into a chain:
- Pattern `A and B -> C`: PreA → PostA → PreC, PreB → PostB → PreC
- Sequence `A -> B -> C`: PreA → PostA → PreB → PostB → PreC → PostC → output

### 4. State Management Pattern (Three Lists)

**Purpose**: Each PreStateProcessor maintains three state lists to manage pattern matching lifecycle.

**Java Implementation** (`StreamPreStateProcessor.StreamPreState`):
```java
class StreamPreState extends State {
    private ComplexEventChunk<StateEvent> currentStateEventChunk;  // Processing
    private LinkedList<StateEvent> pendingStateEventList;          // Active matching
    private LinkedList<StateEvent> newAndEveryStateEventList;      // New candidates
    private volatile boolean stateChanged = false;                 // Dirty flag
    private boolean initialized;
}
```

**State Lifecycle**:
1. **init()**: Start state creates initial empty StateEvent
2. **addState()**: New candidates added to `newAndEveryStateEventList`
3. **updateState()**: Move `newAndEveryStateEventList` → `pendingStateEventList` (sorted by timestamp)
4. **processAndReturn()**: Process incoming event against all `pendingStateEventList`
5. **stateChanged()**: Mark dirty when match progresses
6. **resetState()**: Clear pending, reinitialize if start state

**Rust Requirements**:
```rust
pub struct StreamPreState {
    current_state_event_chunk: ComplexEventChunk<StateEvent>,
    pending_state_event_list: VecDeque<StateEvent>,
    new_and_every_state_event_list: VecDeque<StateEvent>,
    state_changed: bool,
    initialized: bool,
}

impl StreamPreState {
    fn snapshot(&self) -> HashMap<String, Value>;
    fn restore(&mut self, state: HashMap<String, Value>);
    fn can_destroy(&self) -> bool;
}
```

### 5. Logical Operator Implementation (AND/OR)

**Java LogicalPreStateProcessor**:

**Key Features**:
1. **Partner Processor**: AND/OR operates on two streams, each has PreStateProcessor
2. **Shared Lock**: `partnerStatePreProcessor.lock = lock` ensures atomicity
3. **OR Semantics**: If one side matched, skip the other
4. **AND Semantics**: Both sides must match

**Java Implementation**:
```java
public class LogicalPreStateProcessor extends StreamPreStateProcessor {
    protected LogicalStateElement.Type logicalType;  // AND or OR
    protected LogicalPreStateProcessor partnerStatePreProcessor;

    public void setPartnerStatePreProcessor(LogicalPreStateProcessor partner) {
        this.partnerStatePreProcessor = partner;
        partner.lock = this.lock;  // Share lock!
    }

    public ComplexEventChunk<StateEvent> processAndReturn(ComplexEventChunk chunk) {
        for (Iterator<StateEvent> iter = state.getPendingStateEventList().iterator(); ...) {
            StateEvent stateEvent = iter.next();

            // OR semantics: if partner already matched, skip
            if (logicalType == LogicalStateElement.Type.OR &&
                stateEvent.getStreamEvent(partnerStatePreProcessor.getStateId()) != null) {
                iter.remove();
                continue;
            }
            // Process event...
        }
    }
}
```

**Current Rust Gap**:
- ❌ No partner processor pattern (uses Side processors instead)
- ❌ No shared lock mechanism
- ❌ No StateEvent support
- ❌ No proper OR semantics (checking if partner matched)

### 6. Count Quantifier Implementation (A{n}, A{m,n})

**Purpose**: Match patterns like `e1=Stream1{2,5}` (between 2 and 5 events).

**Java CountPreStateProcessor**:

**Key Features**:
1. **Min/Max Count**: `minCount=2, maxCount=5`
2. **Event Chaining**: Uses `stateEvent.addEvent(stateId, streamEvent)` to build chain
3. **Success Condition**: Flag set when min count reached
4. **Backtracking**: `stateEvent.removeLastEvent(stateId)` when count fails

**Java Implementation**:
```java
public class CountPreStateProcessor extends StreamPreStateProcessor {
    private final int minCount;
    private final int maxCount;

    public ComplexEventChunk<StateEvent> processAndReturn(ComplexEventChunk chunk) {
        StreamEvent streamEvent = (StreamEvent) chunk.next();

        for (Iterator<StateEvent> iter = state.getPendingStateEventList().iterator(); ...) {
            StateEvent stateEvent = iter.next();

            // Add to chain at this position
            stateEvent.addEvent(stateId, streamEventCloner.copyStreamEvent(streamEvent));
            state.successCondition = false;
            process(stateEvent);  // Post processor checks count

            if (!state.successCondition) {
                // Count not reached, backtrack
                stateEvent.removeLastEvent(stateId);
            }
        }
    }
}

public class CountPostStateProcessor extends StreamPostStateProcessor {
    public void process(StateEvent stateEvent, ComplexEventChunk chunk) {
        // Count events in chain
        int streamEvents = countEventsInChain(stateEvent.getStreamEvent(stateId));

        if (streamEvents >= minCount) {
            ((CountPreStateProcessor) thisStatePreProcessor).successCondition();
            // Min reached, can progress
        }
        if (streamEvents == maxCount) {
            thisStatePreProcessor.stateChanged();  // Can't add more
        }
    }
}
```

**Current Rust Gap**:
- ❌ No event chaining at StateEvent position
- ❌ No success condition tracking
- ❌ No backtracking (removeLastEvent)
- ❌ No separate CountPostStateProcessor

### 7. Absent Pattern (NOT) Implementation

**Purpose**: Match when an event does NOT occur within a time window: `not Stream1 for 5 sec`.

**Java AbsentStreamPreStateProcessor**:

**Key Features**:
1. **Scheduler Integration**: Uses `Scheduler` to trigger after waiting time
2. **Waiting Time**: Time defined by `for` keyword
3. **Active Flag**: Turns false after first match if 'every' not used
4. **Process Method**: Called by scheduler, outputs StateEvent if no match occurred

**Java Implementation**:
```java
public class AbsentStreamPreStateProcessor extends StreamPreStateProcessor {
    private Scheduler scheduler;
    private long waitingTime = -1;  // 'for' time

    protected void addState(StateEvent stateEvent, StreamPreState state) {
        if (!state.active) return;  // 'every' not used

        state.newAndEveryStateEventList.add(stateEvent);
        // Start scheduler
        state.lastScheduledTime = stateEvent.getTimestamp() + waitingTime;
        scheduler.notifyAt(state.lastScheduledTime);
    }

    // Called by scheduler after waitingTime
    public void process(ComplexEventChunk chunk) {
        long currentTime = chunk.getFirst().getTimestamp();

        for (StateEvent event : state.pendingStateEventList) {
            if (currentTime >= event.getTimestamp() + waitingTime) {
                // Absent pattern matched!
                sendEvent(event, state);
            }
        }
    }

    // If event arrives, clear it (pattern NOT matched)
    public ComplexEventChunk<StateEvent> processAndReturn(ComplexEventChunk chunk) {
        return new ComplexEventChunk<>();  // Always return empty
    }
}
```

**Current Rust Gap**:
- ❌ No Scheduler trait/implementation for pattern processing
- ❌ No AbsentStreamPreStateProcessor
- ❌ No scheduler.notifyAt() integration
- ✅ Timer wheel exists (`timer_wheel.rs`) but disconnected

### 8. 'every' Pattern Support

**Purpose**: Continuous matching, restart pattern after each match: `every A -> B`.

**Key Concepts**:
1. **addEveryState()**: Separate method, clones StateEvent and clears future positions
2. **nextEveryStatePreProcessor**: Chain link for restarting pattern
3. **Event Cloning**: Creates new StateEvent with only previous positions filled

**Java Implementation**:
```java
// In PreStateProcessor
public void addEveryState(StateEvent stateEvent) {
    StateEvent clonedEvent = stateEventCloner.copyStateEvent(stateEvent);
    clonedEvent.setType(ComplexEvent.Type.CURRENT);

    // Clear this position and all future positions
    for (int i = stateId; i < clonedEvent.getStreamEvents().length; i++) {
        clonedEvent.setEvent(i, null);
    }
    state.newAndEveryStateEventList.add(clonedEvent);
}

// In PostStateProcessor
protected void process(StateEvent stateEvent, ComplexEventChunk chunk) {
    if (nextEveryStatePreProcessor != null) {
        nextEveryStatePreProcessor.addEveryState(stateEvent);  // Restart 'every'
    }
}
```

**Example**: `every e1=Stream1 -> e2=Stream2`
1. Stream1 arrives → PreA → PostA
2. PostA calls `PreA.addEveryState()` to restart pattern
3. PostA calls `PreB.addState()` to continue pattern
4. Both chains run in parallel

**Current Rust Gap**: ❌ No 'every' support at all

### 9. WITHIN Time Constraint

**Purpose**: Enforce time bounds: `A -> B within 10 sec`.

**Java Implementation**:
```java
protected long withinTime = SiddhiConstants.UNKNOWN_STATE;
protected int[] startStateIds;  // Which states to check time from

protected boolean isExpired(StateEvent pendingEvent, long currentTimestamp) {
    if (withinTime != SiddhiConstants.UNKNOWN_STATE) {
        for (int startStateId : startStateIds) {
            StreamEvent streamEvent = pendingEvent.getStreamEvent(startStateId);
            if (streamEvent != null &&
                Math.abs(streamEvent.getTimestamp() - currentTimestamp) > withinTime) {
                return true;
            }
        }
    }
    return false;
}

public void expireEvents(long timestamp) {
    for (Iterator<StateEvent> iter = state.pendingStateEventList.iterator(); ...) {
        if (isExpired(iter.next(), timestamp)) {
            iter.remove();
            // Mark as EXPIRED, notify 'every' processor
        }
    }
}
```

**Current Rust Gap**:
- ❌ No expireEvents() method
- ❌ No startStateIds tracking
- ❌ No EXPIRED event type handling
- ✅ Basic retention in sequence processor

### 10. Receiver and Runtime Infrastructure

**Purpose**: Wire up Pre/Post processors into execution chain, handle event routing.

**Java SequenceSingleProcessStreamReceiver**:
```java
public class SequenceSingleProcessStreamReceiver extends SingleProcessStreamReceiver {
    private StateStreamRuntime stateStreamRuntime;

    protected void stabilizeStates(long timestamp) {
        // Called after each event batch
        for (PreStateProcessor processor : allStateProcessors) {
            processor.expireEvents(timestamp);  // Remove expired
        }
        stateStreamRuntime.resetAndUpdate();  // Reset/update all
    }
}
```

**Java StateStreamRuntime**:
```java
public class StateStreamRuntime {
    public void resetAndUpdate() {
        for (PreStateProcessor processor : processors) {
            processor.updateState();  // Move new → pending
        }
        for (PreStateProcessor processor : processors) {
            processor.resetState();   // Clear consumed
        }
    }
}
```

**Current Rust Gap**: ❌ No receiver or runtime infrastructure at all

---

## Current Implementation Gap Analysis

### logical_processor.rs (283 lines) - Comparison

| Feature | Java LogicalPreStateProcessor | Rust logical_processor.rs | Status |
|---------|-------------------------------|---------------------------|--------|
| AND/OR logic | ✅ Full support | ✅ Basic support | **PARTIAL** |
| Partner processor | ✅ Shared lock pattern | ❌ Uses Side processors | **MISSING** |
| StateEvent support | ✅ Multi-stream tracking | ❌ Direct StreamEvents | **MISSING** |
| State lifecycle | ✅ init/add/update/reset | ❌ No lifecycle | **MISSING** |
| Three-list pattern | ✅ pending/new/current | ❌ Two buffers | **MISSING** |
| OR semantics | ✅ Check partner matched | ❌ Simple logic | **MISSING** |
| WITHIN time | ✅ expireEvents() | ❌ No support | **MISSING** |
| 'every' support | ✅ addEveryState() | ❌ No support | **MISSING** |

**Verdict**: ~20% complete vs Java

### sequence_processor.rs (333 lines) - Comparison

| Feature | Java StreamPreStateProcessor | Rust sequence_processor.rs | Status |
|---------|------------------------------|----------------------------|--------|
| Pattern vs Sequence | ✅ Full support | ✅ Implemented | **COMPLETE** |
| WITHIN time | ✅ expireEvents() | ✅ Basic retention | **PARTIAL** |
| Count quantifiers | ✅ Event chaining | ✅ min/max fields | **PARTIAL** |
| StateEvent support | ✅ Multi-stream | ❌ Direct StreamEvents | **MISSING** |
| State lifecycle | ✅ Full lifecycle | ❌ No lifecycle | **MISSING** |
| Event chaining | ✅ addEvent/removeLastEvent | ❌ No chains | **MISSING** |
| Backtracking | ✅ removeLastEvent | ❌ No backtracking | **MISSING** |
| Three-list pattern | ✅ pending/new/current | ❌ Two buffers | **MISSING** |
| 'every' support | ✅ addEveryState() | ❌ No support | **MISSING** |

**Verdict**: ~30% complete vs Java

### Overall Gap Summary

**Implemented**: ~15-20% of Java Siddhi pattern processing capabilities
**Missing Critical Components**:
1. ❌ StateEvent structure (multi-stream tracking)
2. ❌ Pre/Post processor separation
3. ❌ State lifecycle management
4. ❌ 'every' pattern support
5. ❌ Receiver/Runtime infrastructure
6. ❌ Scheduler integration
7. ❌ StateHolder integration for persistence
8. ❌ Event chaining for count quantifiers
9. ❌ Backtracking support
10. ❌ Complete WITHIN time window support

**Estimated Implementation Effort**: 16-22 weeks for full parity

---

## Implementation Phases (Future Roadmap)

**IMPORTANT**: All phases below are TO BE IMPLEMENTED. Nothing beyond basic sequences is currently implemented.

### Phase 1: Foundation (4-6 weeks) - Pre/Post Architecture

**Status**: ❌ NOT IMPLEMENTED (was deleted in Path A)

**Deliverables**:
- [ ] `PreStateProcessor` trait
- [ ] `PostStateProcessor` trait
- [ ] `StreamPreStateProcessor` implementation
- [ ] `StreamPostStateProcessor` implementation
- [ ] `StateEvent` structure with proper fields
- [ ] Basic state management (add_state, update_state, reset_state)
- [ ] Stream receivers for event routing
- [ ] Unit tests for base processors

**Success Criteria**:
- A -> B patterns work through Pre/Post architecture
- Event routing validates correctly
- State transitions properly
- No regression in existing tests

**Files to Create**:
```
src/core/query/input/stream/state/
├── pre_state_processor.rs          (trait)
├── post_state_processor.rs         (trait)
├── stream_pre_state_processor.rs   (impl)
├── stream_post_state_processor.rs  (impl)
├── stream_receiver.rs              (routing)
```

---

### Phase 2: Count Quantifiers (3-4 weeks) - <n:m>, +, *, ?

**Status**: ❌ NOT IMPLEMENTED

**Deliverables**:
- [ ] `CountPreStateProcessor` extending StreamPreStateProcessor
- [ ] Count min/max tracking
- [ ] Count validation logic
- [ ] Pattern compiler integration for count elements
- [ ] Count tests

**Patterns to Support**:
- `A{3}` - Exactly 3
- `A{2,4}` - Range 2-4 (bounded)

**NOT Supported** (to prevent memory overflow):
- `A+` or `A{1,}` - One or more (unbounded)
- `A{n,}` - n or more (unbounded)
- `A*` or `A{0,}` - Zero or more (unbounded + zero-count)
- `A?` or `A{0,1}` - Zero or one (zero-count)
- `A{0,n}` - Zero to n (zero-count)

**Validation Rules**:
- `min_count >= 1`: All steps must match at least one event
- `max_count` must be explicit: All steps must specify an explicit integer max (no unbounded)

**Success Criteria**:
- Bounded count quantifiers work correctly (A{n}, A{m,n})
- Unbounded patterns rejected with clear error message
- Min/max validation enforced
- Count state persists across events
- Performance within targets

---

### Phase 3: Absent Patterns (4-5 weeks) - NOT with FOR

**Status**: ❌ NOT IMPLEMENTED (timer_wheel.rs preserved for this)

**Deliverables**:
- [ ] `AbsentStreamPreStateProcessor` with scheduler
- [ ] `AbsentStreamPostStateProcessor`
- [ ] Scheduler integration for time-based triggers
- [ ] FOR time constraint handling
- [ ] Absent state management

**Use timer_wheel.rs**:
- Already preserved in timers/ directory
- 313 lines, 8 tests, O(1) scheduling
- Ready for scheduler integration

**Test Cases**:
```sql
-- Simple absence
FROM PATTERN (NOT(heartbeat) FOR 30 seconds)
SELECT deviceId INSERT INTO OfflineDevices;

-- Absence after event
FROM PATTERN (purchase -> NOT(shipping) FOR 24 hours)
SELECT orderId INSERT INTO DelayedOrders;
```

**Success Criteria**:
- Scheduler triggers at correct times
- Absent patterns detect missing events
- FOR timing constraints work correctly
- No false positives/negatives

---

### Phase 4: Every Patterns & Advanced (3-4 weeks)

**Status**: ❌ NOT IMPLEMENTED

**Deliverables**:
- [ ] `EveryInnerStateRuntime` implementation
- [ ] Cross-stream references: `e2[price > e1.price]`
- [ ] Collection indexing: `e[0]`, `e[last]`, `e[n]`
- [ ] Expression evaluator for cross-references

---

### Phase 5: Integration & Optimization (2-3 weeks)

**Status**: ❌ NOT IMPLEMENTED

**Deliverables**:
- [ ] Complete pattern compiler from StateElement tree
- [ ] Query parser enhancements for complex patterns
- [ ] Performance profiling and optimization
- [ ] Benchmark suite
- [ ] Memory leak detection
- [ ] Load testing

---

## Grammar & Syntax Requirements

### EventFluxQL Pattern Syntax (Requirements)

```sql
-- Basic sequence
FROM PATTERN (A -> B -> C)
SELECT A.val, B.val, C.val
INSERT INTO Results;

-- Count quantifiers
FROM PATTERN (
    failedLogin<3:5> -> accountLocked
)
SELECT userId, timestamp
INSERT INTO SecurityAlerts;

-- Absent patterns
FROM PATTERN (
    purchase -> NOT(shipping) FOR 24 hours
)
SELECT orderId, customerId
INSERT INTO DelayedOrders;

-- Every pattern
FROM PATTERN (
    every(e=StockStream<5:>)
)
SELECT e[0].price as startPrice, e[last].price as endPrice
INSERT INTO PriceRanges;

-- Logical combinations
FROM PATTERN (
    (login AND apiCall) -> dataExport
)
SELECT userId, exportedData
INSERT INTO DataLeakageAlerts;

-- WITHIN constraint
FROM PATTERN (
    loginAttempt<5:> -> accountLocked
    WITHIN 10 minutes
)
SELECT userId, attemptCount
INSERT INTO BruteForceAttacks;
```

### Query API Programmatic Construction

```rust
use eventflux_rust::query_api::execution::query::input::state::*;

// Build pattern: A -> B<2:5> -> NOT(C) FOR 10s -> D
let a_stream = State::stream(
    SingleInputStream::new_basic("AStream".to_string(), false, false, None, Vec::new())
);

let b_stream = SingleInputStream::new_basic("BStream".to_string(), false, false, None, Vec::new());
let b_count = State::count(State::stream(b_stream), 2, 5);

let c_stream = SingleInputStream::new_basic("CStream".to_string(), false, false, None, Vec::new());
let c_absent = State::logical_not(
    State::stream(c_stream),
    Some(ExpressionConstant::Time(10000)) // 10 seconds
);

let d_stream = State::stream(
    SingleInputStream::new_basic("DStream".to_string(), false, false, None, Vec::new())
);

// Combine: A -> B<2:5> -> NOT(C) FOR 10s -> D
let pattern = State::next(
    State::next(
        State::next(a_stream, b_count),
        c_absent
    ),
    d_stream
);

// Create state input stream
let state_stream = StateInputStream::pattern_stream(pattern, None);
```

---

## Testing Strategy

### Unit Tests (Per Component)

**Test Coverage Target**: >90%

- `StreamPreStateProcessor` - State management, event matching
- `CountPreStateProcessor` - Min/max validation, count tracking
- `AbsentStreamPreStateProcessor` - Scheduler integration, negation logic
- `StreamPostStateProcessor` - State transitions, validation
- `StateEvent` - Cloning, factory, field access
- `StreamReceivers` - Event routing, dispatcher logic

### Integration Tests (Scenarios)

1. **Basic Sequences**:
   - A -> B
   - A -> B -> C
   - A -> B -> C -> D (multi-step)

2. **Count Quantifiers** (explicit bounds required):
   - A{2,5} -> B (bounded range)
   - A{3} -> B (exact count)
   - ❌ A{3,} -> B (unbounded - NOT SUPPORTED, max must be explicit)
   - ❌ A+ -> B (unbounded - NOT SUPPORTED, max must be explicit)
   - ❌ A* -> B (unbounded + zero-count - NOT SUPPORTED)
   - ❌ A{0,1} -> B (zero-count - NOT SUPPORTED)

3. **Absent Patterns**:
   - NOT(A) FOR 5s
   - A -> NOT(B) FOR 10s -> C
   - every(NOT(A) FOR 1m)

4. **Logical Combinations**:
   - A AND B
   - A OR B
   - A AND NOT(B) FOR 5s
   - (A OR B) -> C

5. **Every Patterns**:
   - every(A -> B)
   - every(A<2:3> -> B)
   - every(NOT(A) FOR 5s)

6. **WITHIN Constraints**:
   - A -> B WITHIN 5 minutes
   - A<3:> -> B WITHIN 1 hour

7. **Cross-Stream References**:
   - e1=A -> e2=B[e2.price > e1.price]

8. **Collection Indexing**:
   - every(e=A<5:>) -> e[0].price, e[last].price

### Performance Targets (Goals, Not Measured)

| Pattern Complexity | Target Throughput |
|--------------------|-------------------|
| Simple Sequence (A -> B) | 500K patterns/sec |
| Count Quantifier (A{2,5}) | 300K patterns/sec |
| Absent Pattern (NOT(A) FOR 5s) | 200K patterns/sec |
| Complex Nested | 50K patterns/sec |

### Memory Efficiency Targets

| Metric | Target |
|--------|--------|
| State Event Size | <512 bytes |
| Pending State List Growth | Linear with pattern complexity |
| Memory Leak Rate | 0 bytes/hour |

---

## Success Criteria

### Phase 1 (Foundation)
- [ ] Pre/Post state processor traits defined
- [ ] StreamPreStateProcessor works for A -> B
- [ ] Stream receivers route events correctly
- [ ] No regression in existing tests
- [ ] Performance measured and acceptable

### Phase 2 (Count Quantifiers)
- [x] Bounded count quantifiers work (A{n}, A{m,n})
- [x] Unbounded patterns rejected (A+, A{1,}, A{n,})
- [x] Zero-count patterns rejected (A*, A?, A{0,n})
- [x] Min/max validation enforced (min >= 1, max must be explicit)
- [ ] Performance within targets

### Phase 3 (Absent Patterns)
- [ ] NOT operator works with FOR timing
- [ ] Scheduler triggers at correct times
- [ ] No false positives/negatives

### Phase 4 (Every & Advanced)
- [ ] Every patterns detect continuously
- [ ] Cross-stream references work
- [ ] Collection indexing functional

### Phase 5 (Integration)
- [ ] Complete pattern compiler
- [ ] Performance: >200K patterns/sec
- [ ] 85%+ Java Siddhi parity
- [ ] Memory leak-free (24-hour test)
- [ ] All integration tests passing

---

## Migration from Java Siddhi

### Code Mapping (Future Implementation)

| Java Class | Rust Equivalent | Status |
|------------|-----------------|--------|
| `PreStateProcessor` (interface) | `PreStateProcessor` (trait) | ❌ To implement |
| `StreamPreStateProcessor` | `StreamPreStateProcessor` | ❌ To implement |
| `CountPreStateProcessor` | `CountPreStateProcessor` | ❌ To implement |
| `AbsentStreamPreStateProcessor` | `AbsentStreamPreStateProcessor` | ❌ To implement |
| `PostStateProcessor` (interface) | `PostStateProcessor` (trait) | ❌ To implement |
| `StreamPostStateProcessor` | `StreamPostStateProcessor` | ❌ To implement |
| `StateEvent` | `StateEvent` | ❌ To implement |
| `StreamReceivers` | `StreamReceivers` | ❌ To implement |

**Reference**: Java Siddhi implementation available at `references/siddhi/` for porting guidance.

---

## Risk Assessment

### High Risk

1. **Scheduler Complexity** (Absent Patterns)
   - **Risk**: Time-based triggers may have precision issues
   - **Mitigation**: Use tokio or async-std for accurate timers, extensive testing

2. **State Machine Correctness**
   - **Risk**: Complex nested patterns may have edge cases
   - **Mitigation**: Test suite with edge cases, fuzzing

3. **Memory Leaks**
   - **Risk**: Pending state lists may grow unbounded
   - **Mitigation**: Expiration logic, memory monitoring, stress tests

### Medium Risk

1. **Performance Regression**
   - **Risk**: New architecture slower than simple processors
   - **Mitigation**: Early benchmarking, profiling, incremental optimization

2. **Thread Safety**
   - **Risk**: Data races in concurrent pattern processing
   - **Mitigation**: Rust's ownership system, thorough concurrency tests

---

## Timeline & Priorities

### Current Priority (M2): Grammar Completion

**Focus**: Complete M2 grammar/parser work (66 disabled tests)
**Pattern Processing**: Deferred to M5+

### Future Priority (M5+): Pattern Processing

**When M2 Grammar Completion is done**:
1. Phase 1: Foundation (4-6 weeks)
2. Phase 2: Count Quantifiers (3-4 weeks)
3. Phase 3: Absent Patterns (4-5 weeks)
4. Phase 4: Every & Advanced (3-4 weeks)
5. Phase 5: Integration & Optimization (2-3 weeks)

**Total Estimated**: 16-22 weeks for full implementation

---

## References

**EventFlux Codebase**:
- Query API: `src/query_api/execution/query/input/state/`
- State Processors: `src/core/query/input/stream/state/`
- Preserved Components: `timers/timer_wheel.rs`, `util/event_store.rs`

**Java Siddhi Reference**:
- Local: `references/siddhi/modules/siddhi-core/src/main/java/io/siddhi/`
- Documentation: https://siddhi.io/en/v5.1/docs/

**Related Documents**:
- ROADMAP.md - Strategic priorities and M2 grammar work
- MILESTONES.md - Release timeline
- CLAUDE.md - Development guidelines

---

**Document Owner**: EventFlux Engineering Team
**Last Updated**: 2025-11-03 (Phase 1 Complete + Deadlock Resolved)
**Next Review**: After M2 Grammar Completion

---

## Document History

### Version 2.3 (2025-11-03) - Deadlock Resolution

**Changes**:
- ✅ Added Critical Deadlock Resolution section documenting the Pre->Post->Pre circular Arc deadlock
- ✅ Documented Solution F (Lock-Free Shared State) implementation
- ✅ Key insight: Rust's Arc<Mutex<T>> incompatible with Java's callback pattern (non-reentrant mutex)
- ✅ Solution: ProcessorSharedState with AtomicBool for lock-free state coordination
- ✅ All tests passing: 48 Pre + 14 Post + 6 integration tests
- ✅ Automatic wiring ensures compiler-enforced correctness

**What's New**:
- Concise deadlock problem/solution documentation
- Replaces detailed DEADLOCK_ISSUE.md (493 lines) with essential 24-line summary
- Production-ready architecture validated with comprehensive tests

### Version 2.2 (2025-11-02) - Phase 1 Implementation Complete

**Changes**:
- ✅ Phase 1 implementation completed and all tests passing
- ✅ Updated test metrics: 984 tests passing, 100% success rate
- ✅ Documented Session 2 completion with P0/P1 fixes applied
- ✅ Added integration test descriptions and success criteria

### Version 2.1 (2025-11-01) - Requirements Analysis

**Changes**:
- ✅ Added Java Siddhi Architecture Reference section
- ✅ Documented 10 components with Java code examples
- ✅ Added Current Implementation Gap Analysis with comparison tables
- ✅ Documented StateEvent structure requirements
- ✅ Documented Pre/Post processor interface requirements
- ✅ Documented state management lifecycle (three-list pattern)
- ✅ Documented Logical, Count, Absent, 'every' pattern implementations
- ✅ Documented WITHIN time constraints and Receiver infrastructure
- ✅ Gap analysis: logical_processor.rs (~20% complete), sequence_processor.rs (~30% complete)
- ✅ Overall gap: ~15-20% of Java Siddhi pattern processing implemented

**What's New**:
- **Java reference documentation** (~500 lines) from Java Siddhi source code
- **Comparison tables** showing gaps between Rust and Java implementations
- **Rust trait/struct specifications** for required components
- **Implementation effort estimate**: 16-22 weeks for Java parity

**Purpose**:
- Reference source for pattern processing requirements
- Java code details documented
- Ready for Phase 1 implementation when M2 Grammar Completion is done

### Version 2.0 (2025-10-31) - Truth & Clarity Update

**Changes**:
- ❌ Removed all false "COMPLETE ✅" claims
- ✅ Updated to reflect Path A cleanup reality
- ✅ Changed from "status document" to "design document"
- ✅ Documented what was deleted and why
- ✅ Preserved valid architecture/requirements for future
- ✅ Aligned with ROADMAP.md priorities (M2 Grammar first, M5+ Pattern Processing)

**Previous Version Issues**:
- Claimed Phase 0-3 complete (FALSE - all deleted)
- Claimed Phase 1-2 complete (FALSE - all deleted)
- Claimed 8.07M TPS performance (FALSE - from deleted code)
- 2028 lines of mostly false completion claims

### Version 1.3 (2025-10-29) - False Claims Version

**DEPRECATED**: This version contained false completion claims and has been backed up to `PATTERN_PROCESSING.md.false_claims_backup`
