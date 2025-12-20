# Pattern Processing Limitations

This document tracks current limitations in the EventFlux pattern processing implementation.

## Current Status

| Feature                        | Status          | Notes                          |
|--------------------------------|-----------------|--------------------------------|
| N-Element Sequence Patterns    | ✅ Supported     | 2-5+ elements work             |
| Same-Stream Patterns           | ✅ Supported     | Requires AS aliases            |
| Same-Stream N-Element Patterns | ✅ Supported     | Fixed: proper state sequencing |
| Pattern Alias Resolution       | ✅ Supported     | e1.col, e2.col syntax          |
| Top-Level Logical (AND/OR)     | ✅ Supported     | Direct logical patterns        |
| Logical Groups in Sequences    | ❌ Not Supported | Runtime limitation             |
| EVERY Modifier                 | ✅ Supported     | Continuous matching            |
| WITHIN Timeout                 | ✅ Supported     | Time-based constraints         |
| Count Quantifiers              | ✅ Supported     | {min, max} syntax              |

---

## Limitation 1: Logical Groups in Sequence Patterns

### Description

Patterns that combine logical operators (AND/OR) with sequence operators (->) are not currently supported.

### Unsupported Syntax Examples

```sql
-- Logical group followed by sequence
FROM PATTERN ((e1=A AND e2=B) -> e3=C)

-- Sequence followed by logical group
FROM PATTERN (e1=A -> (e2=B OR e3=C))

-- Nested logical groups in sequences
FROM PATTERN ((e1=A AND e2=B) -> (e3=C OR e4=D))
```

### Workaround

Use separate patterns or restructure the query:

```sql
-- Instead of: (A AND B) -> C
-- Use top-level logical pattern only:
FROM PATTERN (e1=A AND e2=B)

-- Or use pure sequence pattern:
FROM PATTERN (e1=A -> e2=B -> e3=C)
```

### Root Cause

The runtime pattern processor (`query_parser.rs`) uses `extract_stream_state_with_count_and_alias()` which only handles:

- `StateElement::Stream`
- `StateElement::Every`
- `StateElement::Count`

It does not handle `StateElement::Logical` when nested inside `StateElement::Next`.

### Required Fix

1. Extend `extract_stream_state_with_count_and_alias()` to return an enum representing either a stream or a logical
   group
2. Modify the N-element pattern code to use `LogicalGroupConfig` when encountering `StateElement::Logical`
3. Update the 2-element sequence handling similarly

### Affected Tests

```rust
#[ignore = "Requires runtime extension for logical groups in sequences"]
async fn pattern_logical_and_sequence_sql()

#[ignore = "Requires runtime extension for logical groups in sequences"]
async fn pattern_logical_or_sequence_sql()
```

---

## Limitation 2: Old Siddhi-Style Syntax

### Description

The legacy Siddhi query syntax (`from A -> B select ...`) is not supported. Only SQL-style syntax is supported.

### Unsupported Syntax

```
-- Old Siddhi syntax (NOT supported)
from AStream -> BStream
select AStream.val as aval, BStream.val as bval
insert into OutStream;
```

### Supported Syntax

```sql
-- SQL-style syntax (supported)
INSERT INTO Out
SELECT e1.val AS aval, e2.val AS bval
FROM PATTERN(e1 = AStream - > e2 = BStream);
```

### Affected Tests

```rust
#[ignore = "Requires PATTERN/SEQUENCE syntax - Not part of M1"]
async fn sequence_basic()

#[ignore = "Requires PATTERN/SEQUENCE syntax - Not part of M1"]
async fn every_sequence()
```

---

## What Works

### N-Element Sequence Patterns

```sql
-- 3-element pattern
FROM PATTERN (e1=A -> e2=B -> e3=C)

-- 5-element pattern
FROM PATTERN (e1=A -> e2=B -> e3=C -> e4=D -> e5=E)
```

### Same-Stream Patterns with Aliases

```sql
-- Same stream appearing multiple times (requires AS aliases)
SELECT e1.price AS p1, e2.price AS p2, e3.price AS p3
FROM PATTERN(e1 = Trades - > e2 = Trades - > e3 = Trades)
```

### Top-Level Logical Patterns

```sql
-- AND pattern (both must arrive)
FROM PATTERN (e1=A AND e2=B)

-- OR pattern (either can fire)
FROM PATTERN (e1=A OR e2=B)

-- Same-stream logical
FROM PATTERN (e1=Trades AND e2=Trades)
```

### EVERY Modifier

```sql
-- Continuous matching
FROM PATTERN (EVERY(e1=A) -> e2=B)
```

### WITHIN Timeout

```sql
-- Time-constrained pattern
FROM PATTERN (e1=A -> e2=B) WITHIN 5 SECONDS
```

### Count Quantifiers

```sql
-- Min/max count
FROM PATTERN (e1=A{2,5} -> e2=B)
```

---

## Future Work

1. **Logical Groups in Sequences**: Extend runtime to support `(A AND B) -> C` patterns
2. **Negation Patterns**: Support `NOT A` within sequences
3. **Complex Nesting**: Support arbitrary nesting of logical/sequence operators

---

## Fixed Issues

### Same-Stream N-Element Pattern Adapter (Fixed: 2024-12-20)

**Original Issue**: The round-robin adapter for same-stream N-element patterns advanced its position on every incoming
event, causing non-matching events to misalign the pattern state and drop valid matches.

**Root Cause**: The `NElementSameStreamAdapter` used a simple round-robin approach that advanced `current_position` for
every event regardless of whether the processor matched. Combined with the fact that
`PreStateProcessorAdapter.process()` called `update_state()` before `process_and_return()`, this caused forwarded states
to be immediately available for the SAME event, leading to all pattern positions matching on the first event.

**Fix**: Rewrote `NElementSameStreamAdapter` to:

1. Store `PreStateProcessor` references directly (not wrapped in adapters)
2. Call `update_state()` on ALL processors FIRST (before processing)
3. Then call `process_and_return()` on each processor

This ensures that states forwarded from one processor to the next go into the `new_list` and don't become "pending" (
available for matching) until the NEXT event's `update_state()` call.

**Location**: `src/core/util/parser/query_parser.rs:703-792`

---

### Same-Stream 2-Element Pattern Adapter (Fixed: 2024-12-20)

**Original Issue**: The `SameStreamSequenceAdapter` for 2-element same-stream patterns was broadcasting all events to
both sides, causing the same event to match both positions in the pattern.

**Symptoms**:

- Pattern `e1=A -> e2=A` with events [100.0, 110.0] produced `[[100.0, 100.0], [100.0, 110.0]]` instead of
  `[[100.0, 110.0]]`
- The first event was matching both e1 and e2 positions simultaneously

**Root Cause**: Two issues combined:

1. The adapter was broadcasting every event to both first_side and second_side
2. For Pattern type, `SequenceProcessor.check_and_produce()` was called on first_side processing, allowing immediate
   matching when both buffers had the same event

**Fix**: Two-part fix:

1. **SameStreamSequenceAdapter** - Changed event routing:
    - First event: Only goes to first_side (starts pattern)
    - Subsequent events: Go to second_side FIRST (may complete existing pattern), then first_side (starts new pattern)

2. **SequenceProcessor** - Added `same_stream` flag:
    - When `same_stream=true`, first_side processing does NOT call `check_and_produce()`
    - Only second_side processing can trigger pattern matching
    - This prevents the same event from matching both positions

**Key Insight**: For same-stream patterns, we need to ensure Event N can only match position 2 with events from before
Event N (not with itself in position 1).

**Location**:

- `src/core/util/parser/query_parser.rs:544-637` (SameStreamSequenceAdapter)
- `src/core/query/input/stream/state/sequence_processor.rs:64-67, 98-129, 220-225` (same_stream flag)

---

*Last Updated: 2024-12-20*
