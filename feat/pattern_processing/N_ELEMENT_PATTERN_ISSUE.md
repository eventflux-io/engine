# N-Element Pattern Support Issue

## Status: ✅ FIXED

Date Identified: 2024-12-19
Date Fixed: 2024-12-20
Component: query_parser.rs + PatternChainBuilder + TerminalPostStateProcessor
Resolution: N-element patterns (3, 4, 5+ elements) now fully working

## Summary

N-element patterns like `A -> B -> C -> D -> E` are now fully supported. The implementation uses PatternChainBuilder to create processor chains for any number of elements, with TerminalPostStateProcessor bridging to the selector chain.

## Fixes Applied (2024-12-20)

### Root Cause 1: StreamEventCloner not copying data
- `pattern_chain_builder.rs:570` used `StreamEventFactory::new(i, 0, 0)` with loop index instead of attribute count
- Fixed to use `StreamEventFactory::new(attr_count, 0, 0)` with `new_with_sizes()`

### Root Cause 2: Terminal not being called
- `stream_post_state_processor.rs` only forwarded to `next_state_pre_processor`, not `next_processor`
- Added forwarding to `next_processor` for terminal bridging

### Root Cause 3: Duplicate event subscription
- Both pattern chain AND selector chain were subscribed to the junction
- Fixed by skipping direct subscription when using N-element patterns

### Root Cause 4: PreStateProcessorAdapter ordering
- `update_state()` must be called BEFORE `process_and_return()` to move states from `new_list` to `pending_list`

## Completed Components

1. **TerminalPostStateProcessor** (module-level in `query_parser.rs`):
   - Moved to module level for proper scoping
   - Implements complete `PostStateProcessor` trait (all 12 methods)
   - `flatten_state_event()` - converts StateEvent to StreamEvent by concatenating all position attributes
   - Bridges PostStateProcessor chain to Processor chain via `output_processor`
   - Properly handles timestamps and event types

2. **Helper Functions** (`query_parser.rs`):
   - `PatternElementInfo` - struct holding extracted element info
   - `extract_all_pattern_elements()` - recursively extracts all elements from nested Next
   - `is_n_element_pattern()` - detects patterns with 3+ elements

3. **Adapters** (`query_parser.rs`):
   - `PreStateProcessorAdapter` - wraps PreStateProcessor to implement Processor trait for StreamJunction subscription
   - `NElementSameStreamAdapter` - counter-based routing for same-stream N-element patterns
   - `SameStreamSequenceAdapter` - routes first event to first_side, subsequent to second_side

4. **Full Wiring in parse_query** (lines 874-1040):
   - Detects N-element patterns with `is_n_element_pattern()`
   - Builds metadata maps with proper attribute offsets for all elements
   - Creates PatternChainBuilder with PatternStepConfig for each element
   - Builds and initializes ProcessorChain
   - Wires TerminalPostStateProcessor to last PostStateProcessor
   - Groups PreStateProcessorAdapters by stream_id for same-stream handling
   - Subscribes adapters to StreamJunctions
   - Connects TerminalPostStateProcessor.output_processor to selector chain head

5. **2-Element Patterns** - Fully working with alias support:
   - Pattern aliases (e1, e2) registered in `stream_meta_map` and `stream_positions`
   - Same-stream patterns handled via `SameStreamSequenceAdapter`

### Test Coverage

All tests passing:
- `pattern_alias_two_streams` - ✅ Tests e1=StreamA -> e2=StreamB with SELECT using aliases
- `pattern_alias_same_stream` - ✅ Tests e1=RawTrades -> e2=RawTrades
- `n_element_pattern_three_streams` - ✅ Tests 3-element pattern A -> B -> C
- `n_element_pattern_four_streams` - ✅ Tests 4-element pattern A -> B -> C -> D
- `n_element_pattern_five_streams` - ✅ Tests 5-element pattern A -> B -> C -> D -> E

The 2-element pattern path via `SequenceProcessor` remains fully functional with alias support.

## Implementation Summary

Successfully wired PatternChainBuilder into query_parser.rs to support patterns with 3 or more elements (e.g., `A -> B -> C -> D`). The implementation preserves backward compatibility with 2-element patterns using SequenceProcessor while enabling unlimited element patterns through PatternChainBuilder.

## Blockers Fixed (2024-12-19)

1. **Missing State trait**: Added `State` trait to `src/core/util/snapshot.rs` with `snapshot()`, `restore()`, and `can_destroy()` methods
2. **Missing StateEvent methods**: Added to `src/core/event/state/state_event.rs`:
   - `stream_event_count()` - returns the number of stream event slots
   - `expand_to_size(new_size)` - expands stream_events vector to accommodate more positions
   - `clear_event(position)` - clears a stream event at a given position
3. **Module exports**: Updated `src/core/query/input/stream/state/mod.rs` to expose:
   - `PatternChainBuilder`, `PatternStepConfig`, `ProcessorChain`
   - `PreStateProcessor`, `PostStateProcessor`, `StateType`

## N-Element Adapters Added (2024-12-19)

Added to `src/core/util/parser/query_parser.rs` (lines 150-465):

1. **PreStateProcessorAdapter**: Wraps `PreStateProcessor` to implement `Processor` trait for StreamJunction subscription
2. **NElementSameStreamAdapter**: Counter-based routing for N elements from same stream (event 0 -> processor[0], event 1 -> processor[1], etc.)
3. **PatternOutputProcessor**: Converts completed `StateEvent` to flattened `StreamEvent` for downstream processing

## Pattern Extraction Functions Added (2024-12-19)

Added helper functions to `query_parser.rs` (lines 830-884):

1. **PatternElementInfo**: Struct to hold extracted element info (stream_id, alias, min_count, max_count)
2. **extract_all_pattern_elements**: Recursively flattens nested Next structures (e.g., `Next(A, Next(B, Next(C, D)))` -> `[A, B, C, D]`)
3. **is_n_element_pattern**: Detects if a pattern has more than 2 elements

## PatternChainBuilder Integration (2024-12-19)

Modified StateElement::Next handler in `query_parser.rs` (lines 908-1087):

1. **N-element detection**: Checks `is_n_element_pattern()` before processing
2. **Element extraction**: Uses `extract_all_pattern_elements()` to recursively extract all pattern elements
3. **Metadata building**: Creates N-position metadata maps with proper attribute offsets for all elements
4. **PatternChainBuilder wiring**:
   - Creates builder with state type (Pattern/Sequence)
   - Adds each element as PatternStepConfig
   - Sets within time constraint if specified
   - Builds ProcessorChain with pre/post processors
5. **Same-stream handling**: Uses NElementSameStreamAdapter for patterns where all elements reference the same stream
6. **Different-stream handling**: Wraps each pre_processor with PreStateProcessorAdapter and subscribes to corresponding junction
7. **Backward compatibility**: 2-element patterns still use SequenceProcessor via else branch

## Compilation & Testing Status

- ✅ All compilation errors resolved
- ✅ All existing pattern tests pass (8 passed)
- ✅ All N-element pattern tests pass

## N-Element Pattern Tests

Comprehensive test suite in `tests/app_runner_patterns.rs`:

1. **n_element_pattern_three_streams** ✅ - Tests A -> B -> C pattern with aliases e1, e2, e3
2. **n_element_pattern_four_streams** ✅ - Tests A -> B -> C -> D pattern with aliases e1, e2, e3, e4
3. **n_element_pattern_five_streams** ✅ - Tests A -> B -> C -> D -> E pattern with aliases e1-e5

The PatternChainBuilder is fully integrated with selector chain via TerminalPostStateProcessor.

## Completed Work

1. ✅ **PatternChainBuilder Output Integration**:
   - PostStateProcessor output connected to selector chain via TerminalPostStateProcessor
   - StateEvent flattened to StreamEvent for downstream processing
   - Final output wired through link_processor mechanism

2. ✅ **Event Flow Fixes**:
   - StreamPostStateProcessor.process() now calls next_processor.process()
   - update_state() called after add_state() to move StateEvents from new_list to pending_list
   - CountPostStateProcessor forwards to next_processor for terminal patterns

3. ✅ **Adapter Deduplication**:
   - Fixed different-stream handler to reuse same adapter for junction subscription and link_processor
   - Eliminated duplicate event processing that caused 4 outputs instead of 2

## Future Enhancements

1. Performance benchmarking for N-element patterns vs 2-element patterns
2. Count constraints across N elements (e.g., A{2} -> B{1} -> C{3})

## Problem Statement

The current pattern implementation only supports 2-element patterns (e.g., `A -> B`). Patterns with 3 or more elements (e.g., `A -> B -> C -> D`) are not supported due to structural limitations in both the parser and the runtime processor currently used.

## Current Architecture

### API Layer (query_api)

The API supports N-element patterns via nested `NextStateElement`:

```rust
// src/query_api/execution/query/input/state/next_state_element.rs
pub struct NextStateElement {
    pub state_element: Box<StateElement>,       // First element
    pub next_state_element: Box<StateElement>,  // Can be another Next for chaining
}
```

A 4-element pattern `A -> B -> C -> D` is represented as:
```
Next(A, Next(B, Next(C, D)))
```

This layer has no limitation on element count.

### Parser Layer (query_parser.rs)

The parser extracts only 2 elements:

```rust
// src/core/util/parser/query_parser.rs:531-570
StateElement::Next(next_elem) => {
    // Only extracts first element
    let (s1, fmin, fmax, first_alias) = extract_stream_state_with_count_and_alias(
        &next_elem.state_element,
    )?;
    // Only extracts second element - does NOT recurse into nested Next
    let (s2, smin, smax, second_alias) = extract_stream_state_with_count_and_alias(
        &next_elem.next_state_element,
    )?;

    StateRuntimeKind::Sequence {
        first_id: ...,
        second_id: ...,
        // Only 2 IDs, 2 aliases, 2 min/max pairs
    }
}
```

The `extract_stream_state_with_count_and_alias` function handles `Stream`, `Every`, and `Count` variants but returns `None` for `Next`:

```rust
fn extract_stream_state_with_count_and_alias(se: &StateElement) -> Option<...> {
    match se {
        StateElement::Stream(s) => Some(...),
        StateElement::Every(ev) => extract_stream_state_with_count_and_alias(&ev.state_element),
        StateElement::Count(c) => Some(...),
        _ => None,  // Next, Logical, AbsentStream all return None
    }
}
```

If `next_state_element` is itself a `Next(B, Next(C, D))`, the function returns `None` and the query fails with "Unsupported Next pattern structure".

### Runtime Layer (SequenceProcessor)

The `SequenceProcessor` is structurally limited to 2 buffers:

```rust
// src/core/query/input/stream/state/sequence_processor.rs:22-38
pub struct SequenceProcessor {
    pub first_buffer: Vec<StreamEvent>,   // Buffer 1
    pub second_buffer: Vec<StreamEvent>,  // Buffer 2
    pub first_attr_count: usize,
    pub second_attr_count: usize,
    pub first_min: i32,
    pub first_max: i32,
    pub second_min: i32,
    pub second_max: i32,
    // ...
}

pub enum SequenceSide {
    First,
    Second,
}
```

There is no mechanism to add a third buffer or third side. The processor cannot be extended for N elements without restructuring.

### Metadata Layer

The expression parser context supports only 2 positions:

```rust
stream_positions: {
    let mut m = HashMap::new();
    m.insert(first_id_clone.clone(), 0);
    m.insert(second_id_clone.clone(), 1);
    // Only positions 0 and 1
    m
}
```

## Existing N-Element Capable Architecture

An N-element capable architecture exists but is not wired into `query_parser.rs`:

### PatternChainBuilder

```rust
// src/core/query/input/stream/state/pattern_chain_builder.rs:154-160
pub struct PatternChainBuilder {
    elements: Vec<PatternElement>,  // N elements
    state_type: StateType,
    within_duration_ms: Option<i64>,
    is_every: bool,
}
```

The builder accepts any number of elements:

```rust
pub fn add_step(&mut self, step: PatternStepConfig) {
    self.elements.push(PatternElement::Step(step));
}

pub fn total_state_count(&self) -> usize {
    self.elements.iter().map(|e| e.state_count()).sum()
}
```

### CountPreStateProcessor Chain

The `build()` method creates a chain of processors, one per element:

```rust
// src/core/query/input/stream/state/pattern_chain_builder.rs:304-365
pub fn build(...) -> Result<ProcessorChain, String> {
    let mut pre_processors: Vec<Arc<Mutex<dyn PreStateProcessor>>> = Vec::new();
    let mut post_processors: Vec<Arc<Mutex<dyn PostStateProcessor>>> = Vec::new();

    let mut current_state_id: usize = 0;

    for (elem_idx, element) in self.elements.iter().enumerate() {
        match element {
            PatternElement::Step(step) => {
                let pre = Arc::new(Mutex::new(CountPreStateProcessor::new(
                    step.min_count,
                    step.max_count,
                    current_state_id,  // Each element gets unique state_id
                    is_first_element,
                    self.state_type,
                    app_context.clone(),
                    query_context.clone(),
                )));

                let post = Arc::new(Mutex::new(CountPostStateProcessor::new(
                    step.min_count,
                    step.max_count,
                    current_state_id,
                )));

                // Wire Pre -> Post
                pre.lock().unwrap().stream_processor
                    .set_this_state_post_processor(post.clone());

                pre_processors.push(pre);
                post_processors.push(post);

                current_state_id += 1;  // Increment for next element
            }
            // ...
        }
    }
    // Then wires post[i] -> pre[i+1] for chain progression
}
```

### StateEvent vs StreamEvent

The N-element architecture uses `StateEvent` instead of `StreamEvent`:

```rust
// StateEvent stores events at N positions
pub struct StateEvent {
    events: Vec<Option<StreamEvent>>,  // N positions
    // ...
}
```

Each element in the pattern corresponds to a position in the `StateEvent`. When element 0 matches, its event goes to position 0. When element 1 matches, its event goes to position 1, and so on.

## Comparison

| Aspect | SequenceProcessor | PatternChainBuilder |
|--------|-------------------|---------------------|
| Buffer count | 2 (hardcoded) | N (dynamic) |
| Event model | StreamEvent (flattened) | StateEvent (positional) |
| Element limit | 2 | Unlimited |
| State tracking | 2 buffers | N state_ids |
| Wired in parser | Yes | No |

## Solution Design

### Phase 1: Recursive Pattern Extraction

Update `extract_stream_state_with_count_and_alias` to return a vector instead of a single element:

```rust
struct PatternElementInfo {
    stream_state: StreamStateElement,
    min_count: i32,
    max_count: i32,
    alias: Option<String>,
}

fn extract_pattern_elements(se: &StateElement) -> Result<Vec<PatternElementInfo>, String> {
    match se {
        StateElement::Stream(s) => {
            let alias = s.get_single_input_stream()
                .get_stream_reference_id_str()
                .map(|s| s.to_string());
            Ok(vec![PatternElementInfo {
                stream_state: s.clone(),
                min_count: 1,
                max_count: 1,
                alias,
            }])
        }
        StateElement::Every(ev) => {
            extract_pattern_elements(&ev.state_element)
        }
        StateElement::Count(c) => {
            let alias = c.stream_state_element.get_single_input_stream()
                .get_stream_reference_id_str()
                .map(|s| s.to_string());
            Ok(vec![PatternElementInfo {
                stream_state: c.stream_state_element.clone(),
                min_count: c.min_count,
                max_count: c.max_count,
                alias,
            }])
        }
        StateElement::Next(next_elem) => {
            // Recursively extract from both sides
            let mut elements = extract_pattern_elements(&next_elem.state_element)?;
            let next_elements = extract_pattern_elements(&next_elem.next_state_element)?;
            elements.extend(next_elements);
            Ok(elements)
        }
        StateElement::Logical(_) => {
            Err("Logical elements inside Next not yet supported".to_string())
        }
        StateElement::AbsentStream(_) => {
            Err("AbsentStream elements not yet supported".to_string())
        }
    }
}
```

For pattern `A -> B -> C -> D`, this returns:
```
[
    PatternElementInfo { stream: A, alias: "e1", ... },
    PatternElementInfo { stream: B, alias: "e2", ... },
    PatternElementInfo { stream: C, alias: "e3", ... },
    PatternElementInfo { stream: D, alias: "e4", ... },
]
```

### Phase 2: Build N-Element Metadata Maps

```rust
fn build_n_element_metadata(
    elements: &[PatternElementInfo],
    stream_junction_map: &HashMap<String, Arc<Mutex<StreamJunction>>>,
) -> Result<(HashMap<String, Arc<MetaStreamEvent>>, HashMap<String, usize>), String> {

    let mut stream_meta_map = HashMap::new();
    let mut stream_positions = HashMap::new();
    let mut current_offset = 0;

    for (position, elem) in elements.iter().enumerate() {
        let stream_id = elem.stream_state.get_single_input_stream()
            .get_stream_id_str()
            .to_string();

        let junction = stream_junction_map.get(&stream_id)
            .ok_or_else(|| format!("Stream '{}' not found", stream_id))?;

        let stream_def = junction.lock().unwrap().get_stream_definition();
        let attr_count = stream_def.abstract_definition.attribute_list.len();

        let mut meta = MetaStreamEvent::new_for_single_input(stream_def);
        meta.apply_attribute_offset(current_offset);
        let meta = Arc::new(meta);

        // Register by stream name
        stream_meta_map.insert(stream_id.clone(), Arc::clone(&meta));
        stream_positions.insert(stream_id.clone(), position);

        // Register by alias if present
        if let Some(ref alias) = elem.alias {
            stream_meta_map.insert(alias.clone(), Arc::clone(&meta));
            stream_positions.insert(alias.clone(), position);
        }

        current_offset += attr_count;
    }

    Ok((stream_meta_map, stream_positions))
}
```

### Phase 3: Use PatternChainBuilder

Replace `SequenceProcessor` instantiation with `PatternChainBuilder`:

```rust
StateElement::Next(next_elem) => {
    let elements = extract_pattern_elements(state_stream.state_element.as_ref())?;

    let state_type = match state_stream.state_type {
        Type::Pattern => StateType::Pattern,
        Type::Sequence => StateType::Sequence,
    };

    let mut builder = PatternChainBuilder::new(state_type);

    for elem in &elements {
        let stream_id = elem.stream_state.get_single_input_stream()
            .get_stream_id_str()
            .to_string();
        builder.add_step(PatternStepConfig::new(
            elem.alias.clone().unwrap_or(stream_id.clone()),
            stream_id,
            elem.min_count as usize,
            elem.max_count as usize,
        ));
    }

    if let Some(within) = state_stream.within_time.as_ref() {
        if let Some(ms) = extract_within_ms(within) {
            builder.set_within(ms);
        }
    }

    let chain = builder.build(
        Arc::clone(eventflux_app_context),
        Arc::clone(&eventflux_query_context),
    )?;

    // Subscribe each pre_processor to its corresponding junction
    // ...
}
```

### Phase 4: Same-Stream N-Element Adapter

Replace the boolean-based `SameStreamSequenceAdapter` with a counter-based router:

```rust
struct NElementSameStreamAdapter {
    processors: Vec<Arc<Mutex<dyn Processor>>>,
    current_position: AtomicUsize,
    total_positions: usize,
}

impl NElementSameStreamAdapter {
    fn new(processors: Vec<Arc<Mutex<dyn Processor>>>) -> Self {
        let total = processors.len();
        Self {
            processors,
            current_position: AtomicUsize::new(0),
            total_positions: total,
        }
    }
}

impl Processor for NElementSameStreamAdapter {
    fn process(&self, chunk: Option<Box<dyn ComplexEvent>>) {
        let pos = self.current_position.fetch_add(1, Ordering::SeqCst);
        let target_idx = pos % self.total_positions;

        if let Some(processor) = self.processors.get(target_idx) {
            processor.lock().unwrap().process(chunk);
        }
    }
    // ...
}
```

For pattern `e1=Stream -> e2=Stream -> e3=Stream -> e4=Stream`:
- Event 1 goes to processor[0] (position 0)
- Event 2 goes to processor[1] (position 1)
- Event 3 goes to processor[2] (position 2)
- Event 4 goes to processor[3] (position 3)

## Files to Modify

| File | Change |
|------|--------|
| src/core/util/parser/query_parser.rs | Replace SequenceProcessor with PatternChainBuilder |
| src/core/util/parser/query_parser.rs | Add extract_pattern_elements recursive function |
| src/core/util/parser/query_parser.rs | Add build_n_element_metadata function |
| src/core/util/parser/query_parser.rs | Replace SameStreamSequenceAdapter with NElementSameStreamAdapter |
| tests/app_runner_patterns.rs | Add 3+ element pattern tests |

## Files That Already Exist (No Changes Needed)

| File | Purpose |
|------|---------|
| src/core/query/input/stream/state/pattern_chain_builder.rs | N-element chain builder |
| src/core/query/input/stream/state/count_pre_state_processor.rs | Per-element pre processor |
| src/core/query/input/stream/state/count_post_state_processor.rs | Per-element post processor |
| src/core/query/input/stream/state/stream_pre_state_processor.rs | Base pre state processor |
| src/core/query/input/stream/state/stream_post_state_processor.rs | Base post state processor |
| src/core/event/state/state_event.rs | N-position event container |

## Test Cases Needed

```rust
#[test]
fn three_element_sequence() {
    // A -> B -> C
    // Events: A(1), B(2), C(3)
    // Expected: Match with [A.val=1, B.val=2, C.val=3]
}

#[test]
fn four_element_same_stream() {
    // e1=S -> e2=S -> e3=S -> e4=S
    // Events: S(1), S(2), S(3), S(4)
    // Expected: Match with [e1=1, e2=2, e3=3, e4=4]
}

#[test]
fn mixed_stream_four_elements() {
    // e1=A -> e2=B -> e3=A -> e4=B
    // Tests interleaved stream routing
}

#[test]
fn three_element_with_counts() {
    // A{2} -> B{1} -> C{3}
    // Tests count constraints across N elements
}
```

## Effort Estimate

This is a significant refactoring effort:
- Parser changes: Medium complexity
- Wiring PatternChainBuilder: High complexity (subscription management for N streams)
- Same-stream adapter: Low complexity
- Testing: Medium effort

The core N-element machinery exists. The work is primarily in query_parser.rs to wire it correctly.
