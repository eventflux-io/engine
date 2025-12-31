# Pattern Grammar V1.2 - Comprehensive Test Specification

**Version**: 1.0
**Date**: 2025-11-23
**Purpose**: Complete test coverage for TDD implementation of Pattern Processing Grammar
**Based On**: PATTERN_GRAMMAR_V1.2.md

---

## Table of Contents

1. [Test Categories Overview](#test-categories-overview)
2. [Parser Tests (Syntax)](#parser-tests-syntax)
3. [Validation Tests (Semantic Rules)](#validation-tests-semantic-rules)
4. [Runtime Behavior Tests](#runtime-behavior-tests)
5. [Integration Tests (End-to-End)](#integration-tests-end-to-end)
6. [Performance & Edge Cases](#performance--edge-cases)
7. [Test Data Fixtures](#test-data-fixtures)

---

## Test Categories Overview

| Category | Positive Tests | Negative Tests | Total |
|----------|----------------|----------------|-------|
| Pattern Mode vs Sequence | 8 | 4 | 12 |
| EVERY Keyword | 10 | 15 | 25 |
| Count Quantifiers | 12 | 6 | 18 |
| Array Access | 10 | 5 | 15 |
| PARTITION BY | 8 | 5 | 13 |
| WITHIN Clause | 8 | 3 | 11 |
| OUTPUT Event Types | 6 | 2 | 8 |
| Logical Operators | 10 | 4 | 14 |
| Sequence Operator | 12 | 2 | 14 |
| Absent Patterns | 8 | 6 | 14 |
| Filters & Cross-Stream | 15 | 3 | 18 |
| Event Aliases | 6 | 2 | 8 |
| Complex Scenarios | 20 | 10 | 30 |
| **TOTAL** | **133** | **67** | **200** |

---

## Parser Tests (Syntax)

### Category 1: Pattern Mode vs Sequence Mode

#### Test 1.1: PATTERN Mode - Basic Syntax ✅
```sql
FROM PATTERN (e1=StreamA -> e2=StreamB)
SELECT e1.value, e2.value
INSERT INTO Results;
```
**Expected**: PASS (parse successfully)
**Assertions**:
- `pattern_mode == PatternMode::Pattern`
- `pattern_expression` is `Sequence(Stream(StreamA), Stream(StreamB))`
- `state_type == StateType::Pattern`

**What it tests**: Basic PATTERN keyword parsing

---

#### Test 1.2: SEQUENCE Mode - Basic Syntax ✅
```sql
FROM SEQUENCE (e1=StreamA -> e2=StreamB)
SELECT e1.value, e2.value
INSERT INTO Results;
```
**Expected**: PASS (parse successfully)
**Assertions**:
- `pattern_mode == PatternMode::Sequence`
- `state_type == StateType::Sequence`

**What it tests**: Basic SEQUENCE keyword parsing

---

#### Test 1.3: PATTERN Mode - Allows Gaps (Runtime) ✅
```sql
FROM PATTERN (e1=Login -> e2=DataAccess -> e3=Logout)
SELECT e1.userId
INSERT INTO Sessions;
```
**Input Events**: `[Login(u1), Heartbeat, DataAccess(u1), KeepAlive, Logout(u1)]`

**Expected Output**: 1 match `{userId: u1, Login, DataAccess, Logout}`

**Assertions**:
- Match count == 1
- Matched events: Login, DataAccess, Logout (Heartbeat, KeepAlive ignored)

**What it tests**: PATTERN mode ignores non-matching events

---

#### Test 1.4: SEQUENCE Mode - Requires Consecutive (Runtime) ✅
```sql
FROM SEQUENCE (e1=Login -> e2=DataAccess -> e3=Logout)
SELECT e1.userId
INSERT INTO Sessions;
```
**Input Events**: `[Login(u1), Heartbeat, DataAccess(u1), Logout(u1)]`

**Expected Output**: 0 matches

**Assertions**:
- Match count == 0
- State cleared after Heartbeat (non-matching event breaks sequence)

**What it tests**: SEQUENCE mode fails on gaps

---

#### Test 1.5: SEQUENCE Mode - Consecutive Success (Runtime) ✅
```sql
FROM SEQUENCE (e1=Login -> e2=DataAccess -> e3=Logout)
SELECT e1.userId
INSERT INTO Sessions;
```
**Input Events**: `[Login(u1), DataAccess(u1), Logout(u1)]`

**Expected Output**: 1 match `{userId: u1}`

**Assertions**:
- Match count == 1
- All events consecutive, no gaps

**What it tests**: SEQUENCE mode succeeds with consecutive events

---

#### Test 1.6: Missing Mode Keyword ❌
```sql
FROM (e1=StreamA -> e2=StreamB)
SELECT e1.value
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Expected PATTERN or SEQUENCE after FROM`

**What it tests**: Mode keyword is required

---

#### Test 1.7: Invalid Mode Keyword ❌
```sql
FROM MATCH (e1=StreamA -> e2=StreamB)
SELECT e1.value
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Expected PATTERN or SEQUENCE, got 'MATCH'`

**What it tests**: Only PATTERN/SEQUENCE keywords allowed

---

#### Test 1.8: PATTERN Mode - Multi-Step Sequence ✅
```sql
FROM PATTERN (
    e1=A -> e2=B -> e3=C -> e4=D -> e5=E
)
SELECT e1.val, e5.val
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- Parses as nested `Next` elements: `Next(Next(Next(Next(A, B), C), D), E)`

**What it tests**: Long sequence chains in PATTERN mode

---

#### Test 1.9: SEQUENCE Mode - Multi-Step Sequence ✅
```sql
FROM SEQUENCE (
    e1=A -> e2=B -> e3=C -> e4=D
)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- `state_type == StateType::Sequence`

**What it tests**: Long sequence chains in SEQUENCE mode

---

#### Test 1.10: Mode Applied to Entire Pattern ✅
```sql
FROM PATTERN (
    (e1=A AND e2=B) -> e3=C -> (e4=D OR e5=E)
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- Entire pattern tree has `StateType::Pattern`
- Not per-edge mode

**What it tests**: Mode is global for entire pattern

---

#### Test 1.11: Empty Pattern ❌
```sql
FROM PATTERN ()
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Expected pattern expression, got ')' `

**What it tests**: Pattern expression required

---

#### Test 1.12: Missing Parentheses ❌
```sql
FROM PATTERN e1=StreamA -> e2=StreamB
SELECT e1.value
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Expected '(' after PATTERN`

**What it tests**: Parentheses required around pattern expression

---

### Category 2: EVERY Keyword

#### Test 2.1: EVERY in PATTERN Mode - Valid ✅
```sql
FROM PATTERN (
    EVERY (e1=FailedLogin{3,5} -> e2=AccountLocked)
)
SELECT e1[0].userId
INSERT INTO Alerts;
```
**Expected**: PASS

**Assertions**:
- `pattern_expression` is `Every(Sequence(Count(...), Stream(...)))`
- `state_type == StateType::Pattern`

**What it tests**: EVERY allowed in PATTERN mode

---

#### Test 2.2: EVERY in SEQUENCE Mode - Invalid ❌
```sql
FROM SEQUENCE (
    EVERY (e1=A -> e2=B)
)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `EVERY not allowed in SEQUENCE mode. SEQUENCE automatically resets after each match. Use PATTERN mode if you need EVERY.`

**What it tests**: Rule 4 - EVERY only in PATTERN mode

---

#### Test 2.3: EVERY at Top Level - Valid ✅
```sql
FROM PATTERN (
    EVERY (e1=A -> e2=B -> e3=C)
)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- EVERY wraps entire pattern
- No nested EVERY in children

**What it tests**: EVERY at true top level

---

#### Test 2.4: EVERY Nested in Logical - Invalid ❌
```sql
FROM PATTERN (
    EVERY (e1=A -> e2=B) AND e3=C
)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**:
```
EVERY must be at top level only.

  ❌ Incorrect: EVERY (A -> B) AND C
                ^^^^^^^^^^^^^
                EVERY is nested inside Logical pattern

  ✅ Correct:   EVERY ((A -> B) AND C)
                ^^^^^^^^^^^^^^^^^^^^^
                EVERY wraps entire pattern
```

**What it tests**: Rule 1 - EVERY not allowed nested in Logical

---

#### Test 2.5: EVERY Nested in Sequence - Invalid ❌
```sql
FROM PATTERN (
    e1=A -> EVERY (e2=B -> e3=C)
)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `EVERY must be at top level only, not nested in sequences.`

**What it tests**: Rule 1 - EVERY not allowed nested in Sequence

---

#### Test 2.6: Multiple EVERY in Sequence - Invalid ❌
```sql
FROM PATTERN (
    EVERY e1=A -> e2=B -> EVERY e3=C -> e4=D
)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**:
```
Multiple EVERY keywords in sequence not allowed.

Pattern: EVERY e1=A -> e2=B -> EVERY e3=C -> e4=D
                                ^^^^^
Hint: EVERY only allowed at pattern start:

  ✅ Correct:   EVERY (e1=A -> e2=B -> e3=C -> e4=D)
  ❌ Incorrect: EVERY e1=A -> e2=B -> EVERY e3=C -> e4=D
```

**What it tests**: Rule 1 - No multiple EVERY

---

#### Test 2.7: EVERY Without Parentheses - Invalid (if strict parsing) ❌
```sql
FROM PATTERN (
    EVERY e1=StreamA
)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: REJECT (parser error) OR PASS (depends on grammar strictness)

**If grammar allows**: PASS but wraps single element
**If strict**: REJECT with `EVERY requires parentheses: EVERY (...)`

**What it tests**: EVERY parentheses requirement

**Note**: Grammar v1.2 specifies `EVERY '(' pattern_expression ')'` (parentheses required)

---

#### Test 2.8: EVERY with Parentheses - Valid ✅
```sql
FROM PATTERN (
    EVERY (e1=StreamA)
)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- Parses as `Every(Stream(StreamA))`

**What it tests**: EVERY with parentheses around single element

---

#### Test 2.9: EVERY Multi-Instance Behavior (Runtime) ✅
```sql
FROM PATTERN (
    EVERY (e1=FailedLogin{3} -> e2=AccountLocked)
)
PARTITION BY userId
SELECT e1[0].userId, count(e1) as attempts
INSERT INTO Alerts;
```
**Input Events**: `[FL1, FL2, FL3, FL4, FL5, AL]` (all same userId)

**Expected Output**: Multiple matches
- Instance 1: [FL1, FL2, FL3] -> AL (3 attempts)
- Instance 2: [FL2, FL3, FL4] -> AL (3 attempts)
- Instance 3: [FL3, FL4, FL5] -> AL (3 attempts)

**Assertions**:
- Match count == 3
- Each match has 3 failed logins

**What it tests**: EVERY creates overlapping instances

---

#### Test 2.10: EVERY vs Non-EVERY Instance Count (Runtime) ✅
```sql
-- Without EVERY
FROM PATTERN (
    e1=FailedLogin{3} -> e2=AccountLocked
)
SELECT count(e1)
INSERT INTO Alerts;
```
**Input Events**: `[FL1, FL2, FL3, FL4, FL5, AL]`

**Expected Output**: 1 match with 5 events (greedy matching)

**Assertions**:
- Match count == 1
- count(e1) == 5

**What it tests**: Without EVERY, single instance behavior

---

#### Test 2.11: Nested EVERY - Invalid ❌
```sql
FROM PATTERN (
    EVERY (EVERY (e1=A -> e2=B))
)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `Nested EVERY not allowed`

**What it tests**: No EVERY inside EVERY

---

#### Test 2.12: EVERY in Parenthesized Group - Invalid ❌
```sql
FROM PATTERN (
    (EVERY (e1=A -> e2=B)) -> e3=C
)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `EVERY must be at top level only`

**What it tests**: EVERY nested in parenthesized sequence

---

#### Test 2.13: EVERY Wrapping Complex Logical Pattern - Valid ✅
```sql
FROM PATTERN (
    EVERY ((e1=A AND e2=B) -> e3=C -> (e4=D OR e5=E))
)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- EVERY wraps entire complex pattern
- Parses correctly

**What it tests**: EVERY can wrap complex nested patterns

---

#### Test 2.14: EVERY with Count Quantifier - Valid ✅
```sql
FROM PATTERN (
    EVERY (e=TempStream[temp > 100]{5,10})
)
SELECT e[0].roomId, avg(e.temp)
INSERT INTO Alerts;
```
**Expected**: PASS

**Assertions**:
- Parses as `Every(Count(Stream(...), 5, 10))`

**What it tests**: EVERY with count quantifier

---

#### Test 2.15: EVERY with WITHIN - Valid ✅
```sql
FROM PATTERN (
    EVERY (e1=Login -> e2=Logout)
    WITHIN 1 HOUR
)
SELECT e1.userId
INSERT INTO Sessions;
```
**Expected**: PASS

**Assertions**:
- WITHIN applies to entire EVERY pattern
- `within_time` set on first processor

**What it tests**: EVERY combined with WITHIN

---

#### Test 2.16: EVERY on Single Stream - Valid ✅
```sql
FROM PATTERN (
    EVERY (e=TemperatureStream[temp > 100])
)
SELECT e.roomId, e.temp
INSERT INTO HighTempAlerts;
```
**Expected**: PASS

**Assertions**:
- Parses as `Every(Stream(...))`
- Creates new instance for every matching event

**What it tests**: EVERY on single stream element

---

#### Test 2.17: EVERY with Absent Pattern - Valid ✅
```sql
FROM PATTERN (
    EVERY (e1=Order -> NOT Shipping FOR 24 hours)
)
PARTITION BY orderId
SELECT e1.orderId
INSERT INTO DelayedOrders;
```
**Expected**: PASS

**Assertions**:
- EVERY wraps sequence with absent pattern

**What it tests**: EVERY with absent pattern

---

#### Test 2.18: EVERY in Logical OR - Invalid ❌
```sql
FROM PATTERN (
    EVERY (e1=A) OR e2=B
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `EVERY must be at top level only`

**What it tests**: EVERY nested in logical OR

---

#### Test 2.19: Multiple Top-Level EVERY Attempts - Invalid ❌
```sql
FROM PATTERN (
    EVERY (e1=A)
)
SELECT e1.val
INSERT INTO Results;

FROM PATTERN (
    EVERY (e2=B)
)
SELECT e2.val
INSERT INTO Results2;
```
**Expected**: PASS (two separate queries, each valid)

**What it tests**: Multiple queries each with EVERY is OK (they're separate)

---

#### Test 2.20: EVERY Case Sensitivity ✅
```sql
FROM PATTERN (
    every (e1=A -> e2=B)
)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: PASS (if keywords are case-insensitive) OR REJECT (if case-sensitive)

**What it tests**: Keyword case sensitivity

---

#### Test 2.21: EVERY with PARTITION BY - Valid ✅
```sql
FROM PATTERN (
    EVERY (e1=FailedLogin{3,5} -> e2=AccountLocked)
)
PARTITION BY userId
SELECT e1[0].userId
INSERT INTO Alerts;
```
**Expected**: PASS

**Assertions**:
- EVERY creates multiple instances
- PARTITION BY creates separate instance sets per userId

**What it tests**: EVERY + PARTITION BY interaction

---

#### Test 2.22: EVERY with Logical Pattern - Valid ✅
```sql
FROM PATTERN (
    EVERY ((e1=Login AND e2=VPNConnect) -> e3=DataExport)
)
SELECT e1.userId
INSERT INTO SuspiciousActivity;
```
**Expected**: PASS

**Assertions**:
- EVERY wraps logical AND + sequence

**What it tests**: EVERY with logical combinations

---

#### Test 2.23: Deep Nested EVERY - Invalid ❌
```sql
FROM PATTERN (
    e1=A -> (e2=B -> EVERY (e3=C -> e4=D))
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `EVERY must be at top level only`

**What it tests**: Deeply nested EVERY detection

---

#### Test 2.24: EVERY with Empty Pattern - Invalid ❌
```sql
FROM PATTERN (
    EVERY ()
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Expected pattern expression inside EVERY`

**What it tests**: EVERY requires pattern inside parentheses

---

#### Test 2.25: EVERY Without Mode Keyword ❌
```sql
FROM PATTERN (
    EVERY (e1=A -> e2=B)
)
SELECT e1.val
INSERT INTO Results;

-- Then try without FROM PATTERN
EVERY (e1=A -> e2=B)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: Second query REJECT (parser error)

**Error Message**: `Expected FROM keyword`

**What it tests**: EVERY must be inside FROM PATTERN/SEQUENCE clause

---

### Category 3: Count Quantifiers

#### Test 3.1: Exact Count {3} ✅
```sql
FROM PATTERN (
    e=FailedLogin{3} -> lockEvent=AccountLocked
)
SELECT e[0].userId, count(e)
INSERT INTO Alerts;
```
**Input Events**: `[FL1, FL2, FL3, FL4, AL]`

**Expected Output**: 1 match with count(e) == 4 (greedy: takes max available up to 3 or more)

**Wait, exact count means EXACTLY 3, but without max it's greedy?**

Actually, `{3}` means `{3, 3}` - exactly 3. Let me reconsider:

**Input Events**: `[FL1, FL2, FL3, AL]`

**Expected Output**: 1 match with count(e) == 3

**Assertions**:
- Match count == 1
- e contains exactly 3 FailedLogin events
- Parses as `Count(Stream(FailedLogin), 3, 3)`

**What it tests**: Exact count quantifier

---

#### Test 3.2: Range Count {2,5} ✅
```sql
FROM PATTERN (
    e=FailedLogin{2,5} -> lockEvent=AccountLocked
)
SELECT count(e)
INSERT INTO Alerts;
```
**Input Events**: `[FL1, FL2, FL3, FL4, FL5, FL6, FL7, AL]`

**Expected Output**: 1 match with count(e) == 5 (greedy: takes max of range)

**Assertions**:
- Match count == 1
- count(e) == 5 (greedy matching takes max)
- Parses as `Count(Stream(FailedLogin), 2, 5)`

**What it tests**: Range count quantifier with greedy matching

---

#### Test 3.3: One or More {1,} ✅
```sql
FROM PATTERN (
    e=FailedLogin{1,} -> lockEvent=AccountLocked
)
SELECT count(e)
INSERT INTO Alerts;
```
**Input Events**: `[FL1, FL2, FL3, AL]`

**Expected Output**: 1 match with count(e) == 3

**Assertions**:
- Parses as `Count(Stream(FailedLogin), 1, ANY_COUNT)`
- Greedy matching takes all available

**What it tests**: Unbounded range quantifier {1,}

---

#### Test 3.4: One or More with + Shorthand ✅
```sql
FROM PATTERN (
    e=FailedLogin+ -> lockEvent=AccountLocked
)
SELECT count(e)
INSERT INTO Alerts;
```
**Expected**: Same as Test 3.3

**Assertions**:
- `+` is syntactic sugar for `{1,}`
- Parses identically

**What it tests**: + shorthand syntax

---

#### Test 3.5: Zero or More {0,} ✅
```sql
FROM PATTERN (
    e1=Login -> e2=Heartbeat{0,} -> e3=Logout
)
SELECT e1.userId, count(e2)
INSERT INTO Sessions;
```
**Input Events**: `[Login, Logout]`

**Expected Output**: 1 match with count(e2) == 0

**Input Events**: `[Login, HB1, HB2, HB3, Logout]`

**Expected Output**: 1 match with count(e2) == 3

**Assertions**:
- Parses as `Count(Stream(Heartbeat), 0, ANY_COUNT)`
- Optional: matches with 0 or more

**What it tests**: Zero or more quantifier (optional)

---

#### Test 3.6: Zero or More with * Shorthand ✅
```sql
FROM PATTERN (
    e1=A -> e2=B* -> e3=C
)
SELECT count(e2)
INSERT INTO Results;
```
**Expected**: Same as Test 3.5 behavior

**Assertions**:
- `*` is syntactic sugar for `{0,}`

**What it tests**: * shorthand syntax

---

#### Test 3.7: Zero or One {0,1} ✅
```sql
FROM PATTERN (
    e1=Login -> e2=TwoFactorAuth{0,1} -> e3=DataAccess
)
SELECT e1.userId, count(e2)
INSERT INTO Sessions;
```
**Input Events**: `[Login, DataAccess]`

**Expected Output**: 1 match with count(e2) == 0

**Input Events**: `[Login, TwoFactorAuth, DataAccess]`

**Expected Output**: 1 match with count(e2) == 1

**Assertions**:
- Parses as `Count(Stream(TwoFactorAuth), 0, 1)`
- Optional: 0 or 1

**What it tests**: Zero or one quantifier (optional single)

---

#### Test 3.8: Zero or One with ? Shorthand ✅
```sql
FROM PATTERN (
    e1=A -> e2=B? -> e3=C
)
SELECT count(e2)
INSERT INTO Results;
```
**Expected**: Same as Test 3.7 behavior

**Assertions**:
- `?` is syntactic sugar for `{0,1}`

**What it tests**: ? shorthand syntax

---

#### Test 3.9: Invalid - Zero Exact Count {0} ❌
```sql
FROM PATTERN (
    e=StreamA{0} -> e2=StreamB
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `{0} quantifier not allowed (nonsensical - matches zero events)`

**What it tests**: Rule 2 - {0} is invalid

---

#### Test 3.10: Invalid - Zero Range {0,0} ❌
```sql
FROM PATTERN (
    e=StreamA{0,0}
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `{0,0} quantifier not allowed (nonsensical)`

**What it tests**: Rule 2 - {0,0} is invalid

---

#### Test 3.11: Invalid - Max < Min {5,3} ❌
```sql
FROM PATTERN (
    e=FailedLogin{5,3} -> e2=AccountLocked
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `max_count (3) must be >= min_count (5)`

**What it tests**: Rule 2 - max must be >= min

---

#### Test 3.12: Invalid - Negative Count {-1} ❌
```sql
FROM PATTERN (
    e=StreamA{-1}
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error or validation error)

**Error Message**: `Count must be >= 0`

**What it tests**: Negative counts not allowed

---

#### Test 3.13: Valid - Zero Minimum in Range {0,5} ✅
```sql
FROM PATTERN (
    e1=A -> e2=B{0,5} -> e3=C
)
SELECT count(e2)
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- Parses as `Count(Stream(B), 0, 5)`
- {0,5} is valid (optional, up to 5)

**What it tests**: Zero minimum is valid in range

---

#### Test 3.14: Count on Logical Pattern - Invalid? ✅
```sql
FROM PATTERN (
    (e1=A AND e2=B){3}
)
SELECT *
INSERT INTO Results;
```
**Expected**: Parser decision needed

**If allowed**: Parses as `Count(Logical(A, AND, B), 3, 3)`
**If not**: REJECT with `Count quantifiers only apply to stream elements`

**What it tests**: Count on non-stream patterns

**Note**: Grammar shows count_quantifier on basic_pattern (stream_reference), not on logical_pattern. So this should REJECT.

---

#### Test 3.15: Multiple Counts in Sequence ✅
```sql
FROM PATTERN (
    e1=A{2,3} -> e2=B{1,5} -> e3=C{3}
)
SELECT count(e1), count(e2), count(e3)
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- Each element can have its own count quantifier

**What it tests**: Multiple count quantifiers in one pattern

---

#### Test 3.16: Count with Filter ✅
```sql
FROM PATTERN (
    e=FailedLogin[ipAddress == '192.168.1.100']{5,} -> lockEvent=AccountLocked
)
SELECT e[0].userId
INSERT INTO Alerts;
```
**Expected**: PASS

**Assertions**:
- Filter applied to each matching event
- Count applies after filtering

**What it tests**: Count quantifier with filter condition

---

#### Test 3.17: Greedy vs Non-Greedy (if supported) ✅
```sql
FROM PATTERN (
    e=FailedLogin{3,5} -> lockEvent=AccountLocked
)
SELECT count(e)
INSERT INTO Alerts;
```
**Input Events**: `[FL1, FL2, FL3, FL4, FL5, FL6, AL]`

**Expected Output**: 1 match with count(e) == 5 (greedy - takes max)

**Assertions**:
- EventFlux uses greedy matching (takes maximum available within range)

**What it tests**: Greedy matching behavior

**Note**: If non-greedy supported, would need separate syntax (e.g., `{3,5}?`)

---

#### Test 3.18: Large Count Value ✅
```sql
FROM PATTERN (
    e=SensorReading{1000,10000}
)
SELECT count(e)
INSERT INTO Batches;
```
**Expected**: PASS (if within integer limits)

**Assertions**:
- Parser accepts large count values
- Runtime can handle large buffers

**What it tests**: Large count values

---

### Category 4: Array Access

#### Test 4.1: First Element Access e[0] ✅
```sql
FROM PATTERN (
    e=FailedLogin{5,10} -> lockEvent=AccountLocked
)
SELECT e[0].userId, e[0].timestamp as firstAttempt
INSERT INTO Alerts;
```
**Input Events**: `[FL1(t=100), FL2(t=200), FL3(t=300), FL4(t=400), FL5(t=500), AL]`

**Expected Output**: 1 match
- `e[0].userId` = FL1.userId
- `firstAttempt` = 100

**Assertions**:
- e[0] accesses first event in collection
- Zero-based indexing

**What it tests**: Array access to first element

---

#### Test 4.2: Last Element Access e[last] ✅
```sql
FROM PATTERN (
    e=FailedLogin{5,10} -> lockEvent=AccountLocked
)
SELECT e[last].timestamp as lastAttempt
INSERT INTO Alerts;
```
**Input Events**: `[FL1(t=100), FL2(t=200), FL3(t=300), FL4(t=400), FL5(t=500), AL]`

**Expected Output**: 1 match
- `lastAttempt` = 500

**Assertions**:
- e[last] accesses last event in collection

**What it tests**: Array access to last element with 'last' keyword

---

#### Test 4.3: Specific Index Access e[2] ✅
```sql
FROM PATTERN (
    e=FailedLogin{5,10}
)
SELECT e[0].timestamp, e[1].timestamp, e[2].timestamp, e[3].timestamp
INSERT INTO Samples;
```
**Input Events**: `[FL1(t=100), FL2(t=200), FL3(t=300), FL4(t=400), FL5(t=500)]`

**Expected Output**: 1 match
- e[0].timestamp = 100
- e[1].timestamp = 200
- e[2].timestamp = 300
- e[3].timestamp = 400

**Assertions**:
- Specific indices access corresponding events

**What it tests**: Array access to specific indices

---

#### Test 4.4: Out of Bounds Access Returns Null ✅
```sql
FROM PATTERN (
    e=FailedLogin{2,3}
)
SELECT e[0].userId, e[1].userId, e[2].userId, e[5].userId
INSERT INTO Results;
```
**Input Events**: `[FL1, FL2]`

**Expected Output**: 1 match
- e[0].userId = FL1.userId
- e[1].userId = FL2.userId
- e[2].userId = null
- e[5].userId = null

**Assertions**:
- Out-of-bounds access returns null, not error

**What it tests**: Out-of-bounds behavior

---

#### Test 4.5: Invalid - e[first] Keyword ❌
```sql
FROM PATTERN (
    e=FailedLogin{5}
)
SELECT e[first].userId
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `'first' not supported for array indexing. Use e[0] for first element.`

**What it tests**: Rule - No e[first] support

---

#### Test 4.6: Invalid - Negative Indexing e[-1] ❌
```sql
FROM PATTERN (
    e=FailedLogin{5}
)
SELECT e[-1].userId
INSERT INTO Results;
```
**Expected**: REJECT (parser error or validation error)

**Error Message**: `Negative indexing not supported. Use e[last] for last element.`

**What it tests**: Rule - No negative indexing

---

#### Test 4.7: Invalid - Negative Indexing e[-2] ❌
```sql
FROM PATTERN (
    e=FailedLogin{5}
)
SELECT e[-2].userId
INSERT INTO Results;
```
**Expected**: REJECT

**Error Message**: `Negative indexing not supported`

**What it tests**: No negative indexing

---

#### Test 4.8: Array Access with Aggregate Functions ✅
```sql
FROM PATTERN (
    e=SensorReading{50,100}
)
SELECT e[0].timestamp as start,
       e[last].timestamp as end,
       count(e) as total,
       avg(e.value) as avgValue,
       max(e.value) - min(e.value) as range
INSERT INTO Batches;
```
**Expected**: PASS

**Assertions**:
- Array access and aggregates work together
- e[0], e[last] alongside count(), avg(), etc.

**What it tests**: Array access + aggregate functions

---

#### Test 4.9: Array Access on Non-Count Element ❌ or null
```sql
FROM PATTERN (
    e1=Login -> e2=DataAccess
)
SELECT e1[0].userId, e2[0].bytes
INSERT INTO Results;
```
**Expected**: Behavior decision needed

**Option 1**: REJECT with `Array access only valid on count quantifier collections`
**Option 2**: PASS but e1[0] == e1 (single element), e2[0] == e2

**Recommended**: Option 2 - treat single elements as collection of 1

**What it tests**: Array access on non-quantified elements

---

#### Test 4.10: Array Access in Filter Condition ✅
```sql
FROM PATTERN (
    e1=StockPrice{5} ->
    e2=StockPrice[price > e1[last].price * 1.1]
)
SELECT e1[0].price, e2.price
INSERT INTO Signals;
```
**Expected**: PASS

**Assertions**:
- Array access works in cross-stream filter conditions

**What it tests**: Array access in WHERE clause / filter

---

#### Test 4.11: Multiple Array Accesses ✅
```sql
FROM PATTERN (
    e=FailedLogin{10,20}
)
SELECT e[0].timestamp,
       e[1].timestamp,
       e[2].timestamp,
       e[last].timestamp,
       e[0].userId
INSERT INTO Details;
```
**Expected**: PASS

**Assertions**:
- Multiple array accesses on same collection

**What it tests**: Multiple array indices in SELECT

---

#### Test 4.12: Array Access with Arithmetic ✅
```sql
FROM PATTERN (
    e=StockPrice{10}
)
SELECT e[last].price - e[0].price as priceChange,
       (e[last].price - e[0].price) / e[0].price * 100 as percentChange
INSERT INTO PriceMovements;
```
**Expected**: PASS

**Assertions**:
- Array access in arithmetic expressions

**What it tests**: Array access in calculations

---

#### Test 4.13: Array Access on Every Pattern ✅
```sql
FROM PATTERN (
    EVERY (e=TempReading{5,10})
)
SELECT e[0].roomId,
       e[0].temp as startTemp,
       e[last].temp as endTemp
INSERT INTO TempRanges;
```
**Expected**: PASS

**Assertions**:
- Array access works with EVERY

**What it tests**: Array access + EVERY interaction

---

#### Test 4.14: Complex Index Expression ❌ (if not supported)
```sql
FROM PATTERN (
    e=StreamA{10}
)
SELECT e[count(e) / 2].value as middleValue
INSERT INTO Results;
```
**Expected**: REJECT (likely not supported in V1)

**Error Message**: `Array index must be integer literal or 'last' keyword`

**What it tests**: Dynamic index computation (not in V1.2)

---

#### Test 4.15: Array Access with CASE Expression ✅
```sql
FROM PATTERN (
    e=FailedLogin{5,10}
)
SELECT CASE
         WHEN count(e) < 7 THEN e[0].userId
         ELSE e[last].userId
       END as relevantUser
INSERT INTO Alerts;
```
**Expected**: PASS

**Assertions**:
- Array access inside CASE expression

**What it tests**: Array access in CASE statement

---

### Category 5: PARTITION BY

#### Test 5.1: Single Column PARTITION BY ✅
```sql
FROM PATTERN (
    e1=FailedLogin{3,} -> e2=AccountLocked
)
PARTITION BY userId
SELECT userId, count(e1)
INSERT INTO Alerts;
```
**Input Events**:
- `[FL(u1), FL(u1), FL(u1), AL(u1), FL(u2), FL(u2), FL(u2), AL(u2)]`

**Expected Output**: 2 matches (1 per userId)
- Match 1: userId=u1, count=3
- Match 2: userId=u2, count=3

**Assertions**:
- Separate pattern instance per userId
- No cross-user matching

**What it tests**: Basic PARTITION BY functionality

---

#### Test 5.2: Multiple Column PARTITION BY ✅
```sql
FROM PATTERN (
    e1=Login -> e2=Logout
)
PARTITION BY userId, sessionId
SELECT userId, sessionId, e2.timestamp - e1.timestamp as duration
INSERT INTO Sessions;
```
**Expected**: PASS

**Assertions**:
- Parses as `partition_by = [userId, sessionId]`
- Separate instance per (userId, sessionId) tuple

**What it tests**: Multi-column partitioning

---

#### Test 5.3: PARTITION BY with EVERY ✅
```sql
FROM PATTERN (
    EVERY (e1=FailedLogin{3,5} -> e2=AccountLocked)
    WITHIN 10 MINUTES
)
PARTITION BY userId
SELECT userId, count(e1)
INSERT INTO Alerts;
```
**Expected**: PASS

**Assertions**:
- PARTITION BY + EVERY creates:
  - Separate partition per userId
  - Multiple overlapping instances per partition

**What it tests**: PARTITION BY + EVERY interaction

---

#### Test 5.4: PARTITION BY with SEQUENCE Mode ✅
```sql
FROM SEQUENCE (
    e1=TCPPacket[flags='SYN'] ->
    e2=TCPPacket[flags='SYN-ACK'] ->
    e3=TCPPacket[flags='ACK']
)
PARTITION BY connectionId
SELECT connectionId
INSERT INTO EstablishedConnections;
```
**Expected**: PASS

**Assertions**:
- PARTITION BY works in SEQUENCE mode too
- Strict consecutive matching per partition

**What it tests**: PARTITION BY in SEQUENCE mode

---

#### Test 5.5: Invalid - Empty PARTITION BY ❌
```sql
FROM PATTERN (
    e1=A -> e2=B
)
PARTITION BY
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `PARTITION BY requires at least one column`

**What it tests**: Rule 3 - Empty PARTITION BY not allowed

---

#### Test 5.6: Invalid - Duplicate Columns ❌
```sql
FROM PATTERN (
    e1=Login -> e2=Logout
)
PARTITION BY userId, deviceId, userId
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `Duplicate column in PARTITION BY: 'userId'`

**What it tests**: Rule 3 - No duplicate columns

---

#### Test 5.7: Invalid - Duplicate Columns (Case Sensitivity) ❌
```sql
FROM PATTERN (
    e1=A -> e2=B
)
PARTITION BY userId, UserId
SELECT *
INSERT INTO Results;
```
**Expected**: Depends on case sensitivity

**If case-insensitive**: REJECT with `Duplicate column: 'userId'`
**If case-sensitive**: PASS (userId ≠ UserId)

**What it tests**: Case sensitivity in PARTITION BY validation

---

#### Test 5.8: PARTITION BY Column Not in SELECT ✅
```sql
FROM PATTERN (
    e1=Login -> e2=Logout
)
PARTITION BY userId
SELECT e1.timestamp, e2.timestamp
INSERT INTO Sessions;
```
**Expected**: PASS (partition column doesn't need to be in SELECT)

**Assertions**:
- PARTITION BY columns can be omitted from SELECT

**What it tests**: PARTITION BY independent of SELECT clause

---

#### Test 5.9: PARTITION BY Non-Existent Column ❌ (runtime)
```sql
FROM PATTERN (
    e1=LoginStream -> e2=LogoutStream
)
PARTITION BY nonExistentColumn
SELECT e1.userId
INSERT INTO Sessions;
```
**Expected**: Runtime error when events arrive

**Error Message**: `Column 'nonExistentColumn' not found in event schema`

**What it tests**: PARTITION BY column existence validation

---

#### Test 5.10: PARTITION BY with Complex Expression ❌ (likely not supported in V1)
```sql
FROM PATTERN (
    e1=A -> e2=B
)
PARTITION BY userId + deviceId
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `PARTITION BY requires column identifiers, not expressions`

**What it tests**: PARTITION BY syntax limitations (columns only, not expressions)

---

#### Test 5.11: Multiple PARTITION BY Clauses ❌
```sql
FROM PATTERN (
    e1=A -> e2=B
)
PARTITION BY userId
PARTITION BY deviceId
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Duplicate PARTITION BY clause`

**What it tests**: Only one PARTITION BY allowed

---

#### Test 5.12: PARTITION BY Order Doesn't Matter (Runtime) ✅
```sql
-- Query 1
FROM PATTERN (e1=A -> e2=B)
PARTITION BY userId, deviceId
SELECT *
INSERT INTO Results1;

-- Query 2
FROM PATTERN (e1=A -> e2=B)
PARTITION BY deviceId, userId
SELECT *
INSERT INTO Results2;
```
**Expected**: Both PASS

**Runtime behavior**: Same partition keys, order doesn't affect partitioning

**What it tests**: PARTITION BY column order

---

#### Test 5.13: PARTITION BY with WITHIN ✅
```sql
FROM PATTERN (
    e1=FailedLogin{5,} -> e2=AccountLocked
    WITHIN 100 EVENTS
)
PARTITION BY userId
SELECT userId, count(e1)
INSERT INTO FastBruteForce;
```
**Expected**: PASS

**Assertions**:
- WITHIN applies per partition
- Each partition has its own event counter

**What it tests**: PARTITION BY + WITHIN interaction

---

### Category 6: WITHIN Clause

#### Test 6.1: Time-Based WITHIN - Seconds ✅
```sql
FROM PATTERN (
    e1=Login{5,} -> e2=AccountLocked
    WITHIN 30 SECONDS
)
SELECT e1[0].userId
INSERT INTO Alerts;
```
**Expected**: PASS

**Assertions**:
- Parses as `within_constraint = Time(30, TimeUnit::Seconds)`
- Converts to milliseconds: 30000ms

**What it tests**: Time-based WITHIN with seconds

---

#### Test 6.2: Time-Based WITHIN - Minutes ✅
```sql
FROM PATTERN (
    e1=Login -> e2=DataAccess
    WITHIN 10 MINUTES
)
SELECT e1.userId
INSERT INTO Sessions;
```
**Expected**: PASS

**Assertions**:
- Converts to milliseconds: 600000ms

**What it tests**: Time-based WITHIN with minutes

---

#### Test 6.3: Time-Based WITHIN - Hours ✅
```sql
FROM PATTERN (
    e1=OrderPlaced -> e2=OrderShipped
    WITHIN 24 HOURS
)
SELECT e1.orderId
INSERT INTO OnTimeOrders;
```
**Expected**: PASS

**Assertions**:
- Converts to milliseconds: 86400000ms

**What it tests**: Time-based WITHIN with hours

---

#### Test 6.4: Time-Based WITHIN - Milliseconds ✅
```sql
FROM PATTERN (
    e1=Request -> e2=Response
    WITHIN 500 MILLISECONDS
)
SELECT e1.requestId, e2.timestamp - e1.timestamp as latency
INSERT INTO FastResponses;
```
**Expected**: PASS

**Assertions**:
- Parses milliseconds directly

**What it tests**: Time-based WITHIN with milliseconds

---

#### Test 6.5: Event-Count WITHIN ✅
```sql
FROM PATTERN (
    e1=FailedLogin{5,} -> e2=AccountLocked
    WITHIN 100 EVENTS
)
SELECT e1[0].userId
INSERT INTO FastBruteForce;
```
**Expected**: PASS

**Assertions**:
- Parses as `within_constraint = EventCount(100)`
- Pattern must complete within next 100 events

**What it tests**: Event-count based WITHIN

---

#### Test 6.6: Event-Count WITHIN - Small Count ✅
```sql
FROM PATTERN (
    e1=A -> e2=B -> e3=C
    WITHIN 5 EVENTS
)
SELECT *
INSERT INTO Results;
```
**Input Events**: `[A, X, B, Y, Z, W, Q, C]`

**Expected Output**: 0 matches (C arrives after 5 events from A)

**Input Events**: `[A, B, X, C]`

**Expected Output**: 1 match (C arrives within 5 events from A)

**What it tests**: Event-count WITHIN timeout behavior

---

#### Test 6.7: Invalid - Zero Event Count ❌
```sql
FROM PATTERN (
    e1=A -> e2=B
    WITHIN 0 EVENTS
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `WITHIN event count must be > 0`

**What it tests**: Event count validation

---

#### Test 6.8: Invalid - Negative Event Count ❌
```sql
FROM PATTERN (
    e1=A -> e2=B
    WITHIN -10 EVENTS
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Event count must be positive integer`

**What it tests**: Negative event count rejection

---

#### Test 6.9: WITHIN with EVERY ✅
```sql
FROM PATTERN (
    EVERY (e1=Login -> e2=Logout)
    WITHIN 1 HOUR
)
SELECT e1.userId, e2.timestamp - e1.timestamp as sessionDuration
INSERT INTO Sessions;
```
**Expected**: PASS

**Assertions**:
- WITHIN applies to each EVERY instance separately

**What it tests**: WITHIN + EVERY interaction

---

#### Test 6.10: Multiple WITHIN Clauses ❌
```sql
FROM PATTERN (
    e1=A -> e2=B
    WITHIN 10 SECONDS
    WITHIN 100 EVENTS
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Multiple WITHIN clauses not allowed. Use either time-based or event-count.`

**What it tests**: Only one WITHIN allowed

---

#### Test 6.11: WITHIN Outside Pattern Parentheses ✅
```sql
FROM PATTERN (
    e1=Login -> e2=Logout
)
WITHIN 30 MINUTES
SELECT e1.userId
INSERT INTO Sessions;
```
**Expected**: Based on grammar, this might REJECT

**Grammar shows**: `'FROM' pattern_mode '(' pattern_expression within_clause? ')'`

So WITHIN is INSIDE parentheses:
```sql
FROM PATTERN (
    e1=Login -> e2=Logout
    WITHIN 30 MINUTES
)
```

But the examples in grammar show it outside sometimes. Let me check...

Looking at grammar EBNF:
```ebnf
pattern_statement ::= 'FROM' pattern_mode '(' pattern_expression within_clause? ')'
```

So WITHIN is inside the parentheses. Let me update test:

**Corrected**: WITHIN should be inside parentheses (see other tests)

This test should verify parser rejects WITHIN outside parentheses.

**What it tests**: WITHIN must be inside pattern parentheses

---

### Category 7: OUTPUT Event Types

#### Test 7.1: Default - CURRENT EVENTS ✅
```sql
FROM PATTERN (
    e1=Login -> e2=Logout
)
SELECT e1.userId
INSERT INTO Sessions;
```
**Expected**: PASS

**Assertions**:
- Default output_event_type == CurrentEvents
- Only new arrivals emitted (no expired events)

**What it tests**: Default OUTPUT behavior

---

#### Test 7.2: Explicit CURRENT EVENTS ✅
```sql
FROM PATTERN (
    e1=Login -> e2=Logout
)
SELECT e1.userId
INSERT CURRENT EVENTS INTO Sessions;
```
**Expected**: PASS

**Assertions**:
- Explicit `output_event_type = CurrentEvents`

**What it tests**: Explicit CURRENT EVENTS syntax

---

#### Test 7.3: EXPIRED EVENTS ✅
```sql
FROM PATTERN (
    e1=Login -> e2=DataAccess
    WITHIN 30 SECONDS
)
SELECT e1.userId, 'Pattern timed out' as status
INSERT EXPIRED EVENTS INTO PatternTimeouts;
```
**Expected**: PASS

**Assertions**:
- Only expired events (pattern timeouts) emitted
- No successful matches emitted

**What it tests**: EXPIRED EVENTS output

---

#### Test 7.4: ALL EVENTS ✅
```sql
FROM PATTERN (
    e1=Login -> e2=DataAccess
    WITHIN 30 SECONDS
)
SELECT e1.userId,
       CASE WHEN eventType = 'EXPIRED'
            THEN 'Pattern timed out'
            ELSE 'Pattern matched'
       END as status
INSERT ALL EVENTS INTO PatternTracking;
```
**Expected**: PASS

**Assertions**:
- Both successful matches AND timeouts emitted
- Can distinguish via eventType column

**What it tests**: ALL EVENTS output

---

#### Test 7.5: Invalid - Multiple OUTPUT Types ❌
```sql
FROM PATTERN (
    e1=A -> e2=B
)
SELECT *
INSERT CURRENT EVENTS EXPIRED EVENTS INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Unexpected keyword 'EXPIRED' after 'CURRENT EVENTS'`

**What it tests**: Only one output type allowed

---

#### Test 7.6: OUTPUT Type with SEQUENCE Mode ✅
```sql
FROM SEQUENCE (
    e1=A -> e2=B -> e3=C
)
SELECT *
INSERT ALL EVENTS INTO Results;
```
**Expected**: PASS

**Assertions**:
- OUTPUT types work in SEQUENCE mode too

**What it tests**: OUTPUT types in SEQUENCE mode

---

#### Test 7.7: Invalid - OUTPUT Type Typo ❌
```sql
FROM PATTERN (
    e1=A -> e2=B
)
SELECT *
INSERT CURRENTLY EVENTS INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Expected 'CURRENT', 'EXPIRED', or 'ALL', got 'CURRENTLY'`

**What it tests**: OUTPUT type keyword validation

---

#### Test 7.8: OUTPUT Type Case Sensitivity ✅/❌
```sql
FROM PATTERN (
    e1=A -> e2=B
)
SELECT *
INSERT all events INTO Results;
```
**Expected**: PASS (if keywords case-insensitive) OR REJECT

**What it tests**: Keyword case sensitivity

---

### Category 8: Logical Operators (AND, OR)

#### Test 8.1: Simple AND - Both Must Match ✅
```sql
FROM PATTERN (
    e1=Login AND e2=VPNConnect
)
SELECT e1.userId, e2.vpnLocation
INSERT INTO SecureAccess;
```
**Input Events**: `[Login(u1), VPNConnect(u1, loc=USA)]`

**Expected Output**: 1 match

**Input Events**: `[Login(u1)]`

**Expected Output**: 0 matches (VPN not present)

**What it tests**: AND operator - both required

---

#### Test 8.2: Simple OR - Either Can Match ✅
```sql
FROM PATTERN (
    e1=CreditCardPayment OR e1=BankTransfer
)
SELECT e1.orderId, e1.paymentMethod
INSERT INTO Payments;
```
**Input Events**: `[CreditCardPayment(order=123)]`

**Expected Output**: 1 match

**Input Events**: `[BankTransfer(order=456)]`

**Expected Output**: 1 match

**What it tests**: OR operator - either matches

---

#### Test 8.3: AND with Sequence ✅
```sql
FROM PATTERN (
    (e1=Login AND e2=VPNConnect) -> e3=DataExport
)
SELECT e1.userId, e2.vpnLocation, e3.bytes
INSERT INTO SuspiciousActivity;
```
**Expected**: PASS

**Assertions**:
- Parses as `Sequence(Logical(Login, AND, VPN), DataExport)`
- Login AND VPN must both match before DataExport

**What it tests**: AND combined with sequence operator

---

#### Test 8.4: OR with Sequence ✅
```sql
FROM PATTERN (
    (e1=HTTPRequest OR e1=HTTPSRequest) -> e2=Response
)
SELECT e1.url, e2.statusCode
INSERT INTO RequestLogs;
```
**Expected**: PASS

**Assertions**:
- Either HTTP or HTTPS request, followed by response

**What it tests**: OR combined with sequence

---

#### Test 8.5: Precedence - AND Before OR ✅
```sql
FROM PATTERN (
    e1=A OR e2=B AND e3=C
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Parses As**: `OR(A, AND(B, C))` - AND has higher precedence

**Assertions**:
- Equivalent to: `e1=A OR (e2=B AND e3=C)`

**What it tests**: AND/OR precedence

---

#### Test 8.6: Precedence Override with Parentheses ✅
```sql
FROM PATTERN (
    (e1=A OR e2=B) AND e3=C
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Parses As**: `AND(OR(A, B), C)`

**Assertions**:
- Parentheses override default precedence

**What it tests**: Parentheses override precedence

---

#### Test 8.7: Sequence Has Higher Precedence Than AND ✅
```sql
FROM PATTERN (
    e1=A -> e2=B AND e3=C
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Parses As**: `AND(Sequence(A, B), C)`

**Assertions**:
- Equivalent to: `(e1=A -> e2=B) AND e3=C`
- Sequence binds tighter than AND

**What it tests**: Sequence vs AND precedence

---

#### Test 8.8: Complex Logical Expression ✅
```sql
FROM PATTERN (
    (e1=A AND e2=B) OR (e3=C AND e4=D)
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Parses As**: `OR(AND(A, B), AND(C, D))`

**What it tests**: Nested logical operators

---

#### Test 8.9: Logical with Count Quantifiers ✅
```sql
FROM PATTERN (
    (e1=FailedLogin{3,} AND e2=IPRateLimitExceeded) -> e3=AccountLocked
)
SELECT e1[0].userId
INSERT INTO Alerts;
```
**Expected**: PASS

**Assertions**:
- Count quantifier on element in logical expression

**What it tests**: Count quantifiers in logical patterns

---

#### Test 8.10: Triple AND ✅
```sql
FROM PATTERN (
    e1=A AND e2=B AND e3=C
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Parses As**: `AND(AND(A, B), C)` - left-to-right

**What it tests**: Multiple AND operators

---

#### Test 8.11: Triple OR ✅
```sql
FROM PATTERN (
    e1=A OR e2=B OR e3=C
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Parses As**: `OR(OR(A, B), C)` - left-to-right

**What it tests**: Multiple OR operators

---

#### Test 8.12: Invalid - Logical with Absent Pattern ❌
```sql
FROM PATTERN (
    (e1=A AND e2=B) AND (NOT e3=C FOR 10 seconds)
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `Absent patterns cannot be in logical combinations`

**What it tests**: Rule 5 - No Absent in Logical

---

#### Test 8.13: Invalid - Absent with OR ❌
```sql
FROM PATTERN (
    e1=A OR (NOT e2=B FOR 5 seconds)
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `Absent patterns cannot be in logical combinations`

**What it tests**: Rule 5 - Absent with OR also rejected

---

#### Test 8.14: Logical in SEQUENCE Mode ✅
```sql
FROM SEQUENCE (
    (e1=A AND e2=B) -> e3=C
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- Logical operators work in SEQUENCE mode
- After (A AND B) matches, requires consecutive C

**What it tests**: Logical operators in SEQUENCE mode

---

### Category 9: Sequence Operator (->)

#### Test 9.1: Simple Two-Element Sequence ✅
```sql
FROM PATTERN (
    e1=Login -> e2=Logout
)
SELECT e1.userId, e2.timestamp - e1.timestamp as duration
INSERT INTO Sessions;
```
**Expected**: PASS

**Assertions**:
- Parses as `Sequence(Stream(Login), Stream(Logout))`
- Login must occur before Logout

**What it tests**: Basic sequence operator

---

#### Test 9.2: Three-Element Sequence ✅
```sql
FROM PATTERN (
    e1=Order -> e2=Payment -> e3=Shipping
)
SELECT e1.orderId
INSERT INTO CompletedOrders;
```
**Expected**: PASS

**Assertions**:
- Parses as `Sequence(Sequence(Order, Payment), Shipping)`
- Left-to-right associativity

**What it tests**: Multi-element sequence

---

#### Test 9.3: Sequence with Filters ✅
```sql
FROM PATTERN (
    e1=Login[country == 'US'] -> e2=DataAccess[sensitive == true]
)
SELECT e1.userId, e2.dataType
INSERT INTO AuditLog;
```
**Expected**: PASS

**Assertions**:
- Filters applied to each element
- Sequence only matches filtered events

**What it tests**: Sequence with filter conditions

---

#### Test 9.4: Sequence with Count Quantifiers ✅
```sql
FROM PATTERN (
    e1=FailedLogin{3} -> e2=Warning{2} -> e3=AccountLocked
)
SELECT e1[0].userId
INSERT INTO Alerts;
```
**Expected**: PASS

**Assertions**:
- Each element can have count quantifier

**What it tests**: Sequence with counts

---

#### Test 9.5: Long Sequence Chain ✅
```sql
FROM PATTERN (
    e1=A -> e2=B -> e3=C -> e4=D -> e5=E -> e6=F
)
SELECT e1.val, e6.val
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- Parser handles long chains
- Left-to-right associativity maintained

**What it tests**: Long sequence chains

---

#### Test 9.6: Sequence Wrapped in EVERY ✅
```sql
FROM PATTERN (
    EVERY (e1=A -> e2=B -> e3=C)
)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- EVERY creates multiple instances of sequence

**What it tests**: EVERY wrapping sequence

---

#### Test 9.7: Sequence with Cross-Stream References ✅
```sql
FROM PATTERN (
    e1=StockPrice[symbol == 'AAPL'] ->
    e2=StockPrice[symbol == 'AAPL' AND price > e1.price * 1.1]
)
SELECT e1.price as buyPrice, e2.price as sellPrice
INSERT INTO TradingSignals;
```
**Expected**: PASS

**Assertions**:
- e2 filter references e1 attribute

**What it tests**: Cross-stream references in sequence

---

#### Test 9.8: Sequence in PATTERN Mode with Gaps ✅
```sql
FROM PATTERN (
    e1=A -> e2=B -> e3=C
)
SELECT *
INSERT INTO Results;
```
**Input Events**: `[A, X, Y, B, Z, C]`

**Expected Output**: 1 match (X, Y, Z ignored)

**What it tests**: PATTERN mode allows gaps in sequence

---

#### Test 9.9: Sequence in SEQUENCE Mode - No Gaps ✅
```sql
FROM SEQUENCE (
    e1=A -> e2=B -> e3=C
)
SELECT *
INSERT INTO Results;
```
**Input Events**: `[A, X, B, C]`

**Expected Output**: 0 matches (X breaks sequence)

**Input Events**: `[A, B, C]`

**Expected Output**: 1 match

**What it tests**: SEQUENCE mode requires consecutive

---

#### Test 9.10: Nested Sequences with Parentheses ✅
```sql
FROM PATTERN (
    e1=A -> (e2=B -> e3=C) -> e4=D
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- Parses as `Sequence(Sequence(A, Sequence(B, C)), D)`
- Parentheses for grouping (though redundant here due to left-to-right)

**What it tests**: Sequence with explicit grouping

---

#### Test 9.11: Sequence with Logical Element ✅
```sql
FROM PATTERN (
    e1=A -> (e2=B OR e3=C) -> e4=D
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- Parses as `Sequence(Sequence(A, OR(B, C)), D)`
- A, then (B or C), then D

**What it tests**: Sequence with logical element

---

#### Test 9.12: Sequence with Absent Pattern ✅
```sql
FROM PATTERN (
    e1=Purchase -> NOT Shipping FOR 24 hours
)
SELECT e1.orderId
INSERT INTO DelayedOrders;
```
**Expected**: PASS

**Assertions**:
- Parses as `Sequence(Purchase, Absent(Shipping, 24h))`

**What it tests**: Sequence with absent pattern

---

#### Test 9.13: Invalid - Sequence Starting with Absent ❌ (maybe valid?)
```sql
FROM PATTERN (
    NOT e1=A FOR 10 seconds -> e2=B
)
SELECT *
INSERT INTO Results;
```
**Expected**: Decision needed

**If disallowed**: REJECT with `Absent pattern must follow another element`
**If allowed**: PASS - pattern starts with absence of A for 10s, then B

**What it tests**: Absent pattern as first element

---

#### Test 9.14: Missing Arrow Operator ❌
```sql
FROM PATTERN (
    e1=A e2=B
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Expected '->' or end of pattern, got identifier 'e2'`

**What it tests**: Arrow operator required for sequence

---

### Category 10: Absent Patterns (NOT ... FOR)

#### Test 10.1: Basic Absent Pattern ✅
```sql
FROM PATTERN (
    e1=Order -> NOT Shipping FOR 24 hours
)
SELECT e1.orderId, e1.customerId
INSERT INTO DelayedOrders;
```
**Expected**: PASS

**Assertions**:
- Parses as `Sequence(Stream(Order), Absent(Shipping, 24h))`
- Timer starts when Order matches
- If no Shipping within 24h, pattern matches

**What it tests**: Basic NOT ... FOR syntax

---

#### Test 10.2: Absent Pattern with Different Time Units ✅
```sql
FROM PATTERN (
    e1=A -> NOT B FOR 30 seconds
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- Time converted to milliseconds: 30000ms

**What it tests**: Absent with seconds

---

#### Test 10.3: Absent Pattern - Minutes ✅
```sql
FROM PATTERN (
    e1=Login -> NOT Logout FOR 30 minutes
)
SELECT e1.userId
INSERT INTO AbandonedSessions;
```
**Expected**: PASS

**Assertions**:
- Time converted to milliseconds: 1800000ms

**What it tests**: Absent with minutes

---

#### Test 10.4: Absent Pattern - Hours ✅
```sql
FROM PATTERN (
    e1=Issue -> NOT Resolution FOR 4 hours
)
SELECT e1.issueId
INSERT INTO UnresolvedIssues;
```
**Expected**: PASS

**Assertions**:
- Time converted to milliseconds: 14400000ms

**What it tests**: Absent with hours

---

#### Test 10.5: Absent in Sequence ✅
```sql
FROM PATTERN (
    e1=A -> e2=B -> NOT C FOR 10 seconds -> e4=D
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- After B matches, C must NOT occur for 10s, then D must occur

**What it tests**: Absent in middle of sequence

---

#### Test 10.6: Invalid - Absent in AND ❌
```sql
FROM PATTERN (
    e1=A AND (NOT e2=B FOR 10 seconds)
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `Absent patterns cannot be in logical combinations`

**What it tests**: Rule 5 - No Absent in AND

---

#### Test 10.7: Invalid - Absent in OR ❌
```sql
FROM PATTERN (
    (NOT e1=A FOR 5 seconds) OR e2=B
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `Absent patterns cannot be in logical combinations`

**What it tests**: Rule 5 - No Absent in OR

---

#### Test 10.8: Invalid - Nested Absent in Complex Logical ❌
```sql
FROM PATTERN (
    ((e1=A AND e2=B) OR e3=C) AND (NOT e4=D FOR 10 seconds)
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `Absent patterns cannot be in logical combinations`

**What it tests**: Rule 5 - Deeply nested Absent in Logical

---

#### Test 10.9: Absent with Filter ✅
```sql
FROM PATTERN (
    e1=Order -> NOT Shipping[carrier == 'FastShip'] FOR 24 hours
)
SELECT e1.orderId
INSERT INTO DelayedOrders;
```
**Expected**: PASS

**Assertions**:
- NOT checks for absence of Shipping events matching filter
- Only FastShip carrier checked for absence

**What it tests**: Absent with filter condition

---

#### Test 10.10: Absent with PARTITION BY ✅
```sql
FROM PATTERN (
    e1=Order -> NOT Shipping FOR 24 hours
)
PARTITION BY orderId
SELECT orderId
INSERT INTO DelayedOrders;
```
**Expected**: PASS

**Assertions**:
- Absent timer per partition (per orderId)

**What it tests**: Absent with PARTITION BY

---

#### Test 10.11: Multiple Absent Patterns in Sequence ✅
```sql
FROM PATTERN (
    e1=A -> NOT B FOR 10 seconds -> NOT C FOR 5 seconds -> e4=D
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- After A: B must not occur for 10s
- Then: C must not occur for 5s
- Then: D must occur

**What it tests**: Multiple absent patterns

---

#### Test 10.12: Invalid - Absent Without FOR ❌
```sql
FROM PATTERN (
    e1=Order -> NOT Shipping
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Expected 'FOR' after NOT stream in pattern`

**What it tests**: FOR keyword required with NOT

---

#### Test 10.13: Invalid - Absent Without Duration ❌
```sql
FROM PATTERN (
    e1=Order -> NOT Shipping FOR
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Expected time expression after FOR`

**What it tests**: Duration required after FOR

---

#### Test 10.14: Absent with EVERY ✅
```sql
FROM PATTERN (
    EVERY (e1=Order -> NOT Shipping FOR 24 hours)
)
PARTITION BY orderId
SELECT orderId
INSERT INTO DelayedOrders;
```
**Expected**: PASS

**Assertions**:
- EVERY creates multiple instances of absent pattern

**What it tests**: Absent with EVERY

---

### Category 11: Filters & Cross-Stream References

#### Test 11.1: Simple Filter Condition ✅
```sql
FROM PATTERN (
    e=TempStream[temp > 100]
)
SELECT e.roomId, e.temp
INSERT INTO HighTemp;
```
**Expected**: PASS

**Assertions**:
- Parses as `Stream(TempStream, filter: temp > 100)`
- Only events with temp > 100 match

**What it tests**: Basic filter condition

---

#### Test 11.2: Filter with AND Condition ✅
```sql
FROM PATTERN (
    e=LoginStream[country == 'US' AND suspiciousScore > 0.8]
)
SELECT e.userId
INSERT INTO SuspiciousLogins;
```
**Expected**: PASS

**Assertions**:
- Filter has multiple conditions
- Both must be true

**What it tests**: Filter with AND

---

#### Test 11.3: Filter with OR Condition ✅
```sql
FROM PATTERN (
    e=PaymentStream[method == 'CreditCard' OR method == 'DebitCard']
)
SELECT e.orderId, e.method
INSERT INTO CardPayments;
```
**Expected**: PASS

**Assertions**:
- Filter with OR condition

**What it tests**: Filter with OR

---

#### Test 11.4: Cross-Stream Reference in Filter ✅
```sql
FROM PATTERN (
    e1=StockPrice[symbol == 'AAPL'] ->
    e2=StockPrice[symbol == 'AAPL' AND price > e1.price]
)
SELECT e1.price as oldPrice, e2.price as newPrice
INSERT INTO PriceIncreases;
```
**Expected**: PASS

**Assertions**:
- e2 filter references e1.price
- Cross-stream comparison

**What it tests**: Cross-stream attribute reference

---

#### Test 11.5: Cross-Stream Reference with Arithmetic ✅
```sql
FROM PATTERN (
    e1=StockPrice[symbol == 'AAPL'] ->
    e2=StockPrice[symbol == 'AAPL' AND price > e1.price * 1.1]
)
SELECT e1.price, e2.price
INSERT INTO SignificantIncreases;
```
**Expected**: PASS

**Assertions**:
- e2 filter has arithmetic expression: `e1.price * 1.1`

**What it tests**: Arithmetic in cross-stream filter

---

#### Test 11.6: Cross-Stream with Array Access ✅
```sql
FROM PATTERN (
    e1=StockPrice{5} ->
    e2=StockPrice[price > e1[last].price * 1.05]
)
SELECT e1[0].price, e2.price
INSERT INTO Signals;
```
**Expected**: PASS

**Assertions**:
- Filter references `e1[last].price`
- Array access in cross-stream reference

**What it tests**: Array access in filter

---

#### Test 11.7: Multiple Cross-Stream References ✅
```sql
FROM PATTERN (
    e1=A ->
    e2=B[val > e1.val] ->
    e3=C[val > e1.val AND val > e2.val]
)
SELECT e1.val, e2.val, e3.val
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- e3 filter references both e1 and e2

**What it tests**: Multiple cross-stream references

---

#### Test 11.8: Filter with String Comparison ✅
```sql
FROM PATTERN (
    e=LoginStream[username == 'admin' AND password != '']
)
SELECT e.username, e.ipAddress
INSERT INTO AdminLogins;
```
**Expected**: PASS

**Assertions**:
- String equality and inequality

**What it tests**: String comparison in filter

---

#### Test 11.9: Filter with Numeric Comparison ✅
```sql
FROM PATTERN (
    e=SensorReading[value >= 50 AND value <= 100]
)
SELECT e.sensorId, e.value
INSERT INTO NormalReadings;
```
**Expected**: PASS

**Assertions**:
- Numeric comparisons: >=, <=

**What it tests**: Numeric comparison operators

---

#### Test 11.10: Filter with LIKE (if supported) ✅/❌
```sql
FROM PATTERN (
    e=LogEntry[message LIKE '%ERROR%']
)
SELECT e.timestamp, e.message
INSERT INTO Errors;
```
**Expected**: PASS (if LIKE supported) OR REJECT

**What it tests**: LIKE operator support in filters

---

#### Test 11.11: Filter with IN (if supported) ✅/❌
```sql
FROM PATTERN (
    e=PaymentStream[method IN ('CreditCard', 'DebitCard', 'PayPal')]
)
SELECT e.orderId, e.method
INSERT INTO OnlinePayments;
```
**Expected**: PASS (if IN supported) OR REJECT

**What it tests**: IN operator support

---

#### Test 11.12: Filter with NULL Check ✅
```sql
FROM PATTERN (
    e=UserEvent[metadata IS NOT NULL]
)
SELECT e.userId, e.metadata
INSERT INTO RichEvents;
```
**Expected**: PASS (if NULL checks supported)

**What it tests**: NULL checking in filters

---

#### Test 11.13: Complex Nested Filter ✅
```sql
FROM PATTERN (
    e=DataStream[(type == 'A' AND value > 100) OR (type == 'B' AND value > 200)]
)
SELECT e.type, e.value
INSERT INTO FilteredData;
```
**Expected**: PASS

**Assertions**:
- Nested logical conditions in filter

**What it tests**: Complex filter expressions

---

#### Test 11.14: Invalid - Undefined Attribute in Filter ❌ (runtime)
```sql
FROM PATTERN (
    e=StreamA[nonExistentColumn > 100]
)
SELECT *
INSERT INTO Results;
```
**Expected**: Runtime error

**Error Message**: `Column 'nonExistentColumn' not found in StreamA schema`

**What it tests**: Attribute existence validation

---

#### Test 11.15: Filter on Count Quantifier Element ✅
```sql
FROM PATTERN (
    e=FailedLogin[ipAddress == '192.168.1.100']{5,}
)
SELECT e[0].userId, count(e)
INSERT INTO BruteForce;
```
**Expected**: PASS

**Assertions**:
- Filter applies to each event before counting

**What it tests**: Filter with count quantifier

---

#### Test 11.16: Empty Filter ❌
```sql
FROM PATTERN (
    e=StreamA[]
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Empty filter condition not allowed`

**What it tests**: Filter must have condition

---

#### Test 11.17: Cross-Stream Forward Reference ❌ (semantic error)
```sql
FROM PATTERN (
    e1=A[val > e2.val] -> e2=B
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (validation error)

**Error Message**: `Cannot reference 'e2' before it is defined in pattern sequence`

**What it tests**: Forward references not allowed

---

#### Test 11.18: Filter with Function Call (if supported) ✅
```sql
FROM PATTERN (
    e=LogStream[length(message) > 1000]
)
SELECT e.message
INSERT INTO LongMessages;
```
**Expected**: PASS (if functions supported in filters)

**What it tests**: Function calls in filters

---

### Category 12: Event Aliases

#### Test 12.1: Assignment Style Alias (e1=Stream) ✅
```sql
FROM PATTERN (
    e1=LoginStream -> e2=LogoutStream
)
SELECT e1.userId, e2.timestamp
INSERT INTO Sessions;
```
**Expected**: PASS

**Assertions**:
- Parses as `Stream(LoginStream, alias: e1)`

**What it tests**: Assignment-style aliasing

---

#### Test 12.2: SQL Style Alias (Stream AS e1) ✅
```sql
FROM PATTERN (
    LoginStream AS e1 -> LogoutStream AS e2
)
SELECT e1.userId, e2.timestamp
INSERT INTO Sessions;
```
**Expected**: PASS

**Assertions**:
- Parses identically to Test 12.1
- Both syntaxes supported

**What it tests**: SQL-style aliasing

---

#### Test 12.3: Mixed Alias Styles ✅
```sql
FROM PATTERN (
    e1=LoginStream -> LogoutStream AS e2
)
SELECT e1.userId, e2.timestamp
INSERT INTO Sessions;
```
**Expected**: PASS

**Assertions**:
- Can mix both styles in same pattern

**What it tests**: Mixing alias styles

---

#### Test 12.4: Stream Without Alias ✅
```sql
FROM PATTERN (
    LoginStream -> LogoutStream
)
SELECT *
INSERT INTO Sessions;
```
**Expected**: PASS

**Assertions**:
- Aliases are optional
- Can reference stream by name in SELECT

**What it tests**: Optional aliases

---

#### Test 12.5: Duplicate Alias Names ❌
```sql
FROM PATTERN (
    e1=LoginStream -> e1=LogoutStream
)
SELECT e1.userId
INSERT INTO Sessions;
```
**Expected**: REJECT (validation error)

**Error Message**: `Duplicate alias 'e1' in pattern`

**What it tests**: Unique alias requirement

---

#### Test 12.6: Alias Naming Conventions ✅
```sql
FROM PATTERN (
    failedLogin=FailedLoginStream{3,5} -> accountLocked=AccountLockedStream
)
SELECT failedLogin[0].userId
INSERT INTO Alerts;
```
**Expected**: PASS

**Assertions**:
- Camel case aliases allowed
- Descriptive names

**What it tests**: Alias naming flexibility

---

#### Test 12.7: Invalid - Reserved Keyword as Alias ❌
```sql
FROM PATTERN (
    SELECT=StreamA -> FROM=StreamB
)
SELECT SELECT.value
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Cannot use reserved keyword 'SELECT' as alias`

**What it tests**: Reserved keyword protection

---

#### Test 12.8: Alias in Cross-Stream Reference ✅
```sql
FROM PATTERN (
    first=StockPrice[symbol == 'AAPL'] ->
    second=StockPrice[symbol == 'AAPL' AND price > first.price]
)
SELECT first.price, second.price
INSERT INTO Increases;
```
**Expected**: PASS

**Assertions**:
- Alias used in filter and SELECT

**What it tests**: Alias usage across pattern

---

### Category 13: Complex Scenarios

#### Test 13.1: Everything Combined - Kitchen Sink ✅
```sql
FROM PATTERN (
    EVERY (
        (e1=Login[country == 'US'] AND e2=VPNConnect[location != 'US']) ->
        e3=DataAccess[bytes > 1000000]{3,5} ->
        NOT Logout FOR 30 minutes
    )
    WITHIN 1 HOUR
)
PARTITION BY userId
SELECT e1.userId,
       e1.timestamp as loginTime,
       e2.location as vpnLocation,
       count(e3) as dataAccessCount,
       sum(e3.bytes) as totalBytes
INSERT ALL EVENTS INTO SuspiciousActivity;
```
**Expected**: PASS

**Assertions**:
- EVERY: multi-instance
- Logical AND
- Count quantifier
- Absent pattern
- WITHIN time constraint
- PARTITION BY
- OUTPUT ALL EVENTS
- Cross-stream references
- Aggregates

**What it tests**: Maximum feature combination

---

#### Test 13.2: Deeply Nested Pattern ✅
```sql
FROM PATTERN (
    EVERY (
        ((e1=A AND e2=B) OR (e3=C AND e4=D)) ->
        (e5=E{2,5} -> e6=F) ->
        NOT G FOR 10 seconds
    )
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS

**Assertions**:
- Deep nesting: EVERY -> Sequence -> Logical -> Sequence -> Absent
- Parentheses grouping

**What it tests**: Deep nesting complexity

---

#### Test 13.3: Multiple Partitions with EVERY ✅
```sql
FROM PATTERN (
    EVERY (e1=FailedLogin{3,5} -> e2=AccountLocked)
    WITHIN 10 MINUTES
)
PARTITION BY userId, deviceId
SELECT userId, deviceId, count(e1)
INSERT INTO BruteForceAttempts;
```
**Expected**: PASS

**Assertions**:
- PARTITION BY two columns
- EVERY creates instances per (userId, deviceId) combination

**What it tests**: Multi-column partition with EVERY

---

#### Test 13.4: Event-Count WITHIN with Fast Stream ✅
```sql
FROM PATTERN (
    e1=HighFreqEvent{100,} -> e2=Threshold
    WITHIN 1000 EVENTS
)
SELECT e1[0].timestamp, e1[last].timestamp, count(e1)
INSERT INTO HighThroughputAlerts;
```
**Expected**: PASS

**Assertions**:
- Pattern must complete within 1000 events
- Useful for throughput-based detection

**What it tests**: Event-count WITHIN on high-frequency stream

---

#### Test 13.5: Greedy Matching Behavior ✅
```sql
FROM PATTERN (
    e=FailedLogin{3,10} -> lockEvent=AccountLocked
)
SELECT count(e)
INSERT INTO Alerts;
```
**Input Events**: `[FL1, FL2, FL3, FL4, FL5, FL6, FL7, FL8, AL]`

**Expected Output**: 1 match with count(e) == 8

**Assertions**:
- Greedy: takes maximum in range (8, not 3)

**What it tests**: Greedy quantifier behavior

---

#### Test 13.6: Non-Greedy Alternative Pattern ✅
```sql
-- First match pattern (non-greedy simulation)
FROM PATTERN (
    e=FailedLogin{3} -> lockEvent=AccountLocked
)
SELECT count(e)
INSERT INTO Alerts;
```
**Input Events**: `[FL1, FL2, FL3, FL4, FL5, AL]`

**Expected Output**: 1 match with count(e) == 5 (greedy even with exact count)

Wait, `{3}` means exactly 3, so it should match exactly 3 unless greedy takes more?

Let me reconsider: `{3}` = `{3,3}` = exactly 3. But without a following element, does it take exactly 3 or wait for more?

Actually, looking at the grammar: `{3}` means min=3, max=3. So it should match exactly 3. But then what happens with FL4, FL5?

This needs clarification in the specification. For now, assume:
- `{3}` matches exactly 3
- Remaining events (FL4, FL5) are ignored in PATTERN mode

**Expected Output**: 1 match with count(e) == 3

**What it tests**: Exact count behavior with excess events

---

#### Test 13.7: Overlapping EVERY Instances ✅
```sql
FROM PATTERN (
    EVERY (e=FailedLogin{2} -> lockEvent=AccountLocked)
)
SELECT count(e)
INSERT INTO Alerts;
```
**Input Events**: `[FL1, FL2, FL3, AL]`

**Expected Instances**:
- Instance 1: [FL1, FL2] -> AL (count=2)
- Instance 2: [FL2, FL3] -> AL (count=2)

**Expected Output**: 2 matches

**What it tests**: EVERY creates overlapping instances

---

#### Test 13.8: PARTITION BY Isolation ✅
```sql
FROM PATTERN (
    e1=FailedLogin{3,} -> e2=AccountLocked
)
PARTITION BY userId
SELECT userId, count(e1)
INSERT INTO Alerts;
```
**Input Events**: `[FL(u1), FL(u2), FL(u1), FL(u2), FL(u1), AL(u1), AL(u2)]`

**Expected Output**: 1 match for u1 (count=3), 0 matches for u2 (count=2, below min)

**Assertions**:
- Separate state per userId
- u2's FL events don't contribute to u1's pattern

**What it tests**: PARTITION BY state isolation

---

#### Test 13.9: Absent Pattern Timeout ✅
```sql
FROM PATTERN (
    e1=Order -> NOT Shipping FOR 10 seconds
)
SELECT e1.orderId
INSERT INTO DelayedOrders;
```
**Input Events**:
- t=0: Order(123)
- t=5: Shipping(456) (different order)
- t=11: (timer expires)

**Expected Output**: 1 match (orderId=123)

**Input Events**:
- t=0: Order(123)
- t=5: Shipping(123) (same order - breaks pattern)

**Expected Output**: 0 matches

**What it tests**: Absent pattern timer and matching logic

---

#### Test 13.10: Cross-Stream Arithmetic in SELECT ✅
```sql
FROM PATTERN (
    e1=StockPrice ->
    e2=StockPrice[price > e1.price]
)
SELECT e1.symbol,
       e1.price as oldPrice,
       e2.price as newPrice,
       e2.price - e1.price as priceChange,
       (e2.price - e1.price) / e1.price * 100 as percentChange
INSERT INTO PriceMovements;
```
**Expected**: PASS

**Assertions**:
- Complex arithmetic in SELECT
- Multiple calculations using cross-stream values

**What it tests**: Arithmetic expressions in SELECT

---

#### Test 13.11: Aggregates on Count Quantifier ✅
```sql
FROM PATTERN (
    e=SensorReading{50,100}
)
SELECT count(e) as readingCount,
       avg(e.value) as avgValue,
       sum(e.value) as totalValue,
       min(e.value) as minValue,
       max(e.value) as maxValue,
       max(e.value) - min(e.value) as range
INSERT INTO SensorBatches;
```
**Expected**: PASS

**Assertions**:
- Multiple aggregate functions on collection

**What it tests**: Aggregates on quantified events

---

#### Test 13.12: Array Access in Complex Expression ✅
```sql
FROM PATTERN (
    e=StockPrice{10}
)
SELECT e[0].symbol,
       (e[last].price - e[0].price) as priceMovement,
       (e[last].price - e[0].price) / e[0].price * 100 as percentChange,
       CASE
         WHEN e[last].price > e[0].price THEN 'GAIN'
         WHEN e[last].price < e[0].price THEN 'LOSS'
         ELSE 'FLAT'
       END as direction
INSERT INTO PriceAnalysis;
```
**Expected**: PASS

**Assertions**:
- Array access in arithmetic
- Array access in CASE expression

**What it tests**: Array access in complex SELECT expressions

---

#### Test 13.13: Logical Pattern with Different Stream Types ✅
```sql
FROM PATTERN (
    (e1=UserRegistered AND e2=EmailVerified AND e3=PhoneVerified) ->
    e4=FirstPurchase
)
SELECT e1.userId, e4.orderId
INSERT INTO CompleteOnboarding;
```
**Expected**: PASS

**Assertions**:
- AND with three different stream types
- All must match (no ordering) before sequence continues

**What it tests**: Multi-stream logical AND

---

#### Test 13.14: Optional Middle Element ✅
```sql
FROM PATTERN (
    e1=Login -> e2=TwoFactorAuth{0,1} -> e3=DataAccess
)
SELECT e1.userId, count(e2) as usedTwoFactor
INSERT INTO AccessLogs;
```
**Input Events**: `[Login(u1), DataAccess(u1)]`

**Expected Output**: 1 match with usedTwoFactor=0

**Input Events**: `[Login(u1), TwoFactorAuth(u1), DataAccess(u1)]`

**Expected Output**: 1 match with usedTwoFactor=1

**What it tests**: Optional element in sequence ({0,1})

---

#### Test 13.15: Chained Cross-Stream References ✅
```sql
FROM PATTERN (
    e1=A ->
    e2=B[val > e1.val] ->
    e3=C[val > e2.val] ->
    e4=D[val > e3.val]
)
SELECT e1.val, e2.val, e3.val, e4.val
INSERT INTO IncreasingSequence;
```
**Expected**: PASS

**Assertions**:
- Each element references previous element
- Chain of comparisons

**What it tests**: Chained cross-stream references

---

#### Test 13.16: SEQUENCE Mode with Logical ✅
```sql
FROM SEQUENCE (
    (e1=A AND e2=B) -> e3=C
)
SELECT *
INSERT INTO Results;
```
**Input Events**: `[A, B, C]`

**Expected Output**: Depends on logical AND semantics in SEQUENCE

If A and B must be consecutive: Need clarification
If A and B can be in any order but both before C: More complex

**What it tests**: Logical operator semantics in SEQUENCE mode

**Note**: This may need semantic clarification in the grammar.

---

#### Test 13.17: Very Long Pattern Chain ✅
```sql
FROM PATTERN (
    e1=A -> e2=B -> e3=C -> e4=D -> e5=E ->
    e6=F -> e7=G -> e8=H -> e9=I -> e10=J
)
SELECT e1.val, e10.val
INSERT INTO LongSequence;
```
**Expected**: PASS

**Assertions**:
- Parser handles 10+ element sequences
- Memory management for long state chains

**What it tests**: Long pattern chains (stress test)

---

#### Test 13.18: Multiple Count Quantifiers in Sequence ✅
```sql
FROM PATTERN (
    e1=LoginAttempt{5,10} ->
    e2=Warning{2,3} ->
    e3=Alert{1,}
)
SELECT count(e1), count(e2), count(e3)
INSERT INTO SecurityEvents;
```
**Expected**: PASS

**Assertions**:
- Each element has its own count quantifier
- Multiple collections in one pattern

**What it tests**: Multiple count quantifiers

---

#### Test 13.19: Pattern with All Time Units ✅
```sql
FROM PATTERN (
    e1=A -> e2=B
    WITHIN 2 days 3 hours 30 minutes 45 seconds 500 milliseconds
)
SELECT *
INSERT INTO Results;
```
**Expected**: Depends on grammar

**If compound time supported**: PASS
**If single unit only**: REJECT with `Multiple time units not supported`

**Simplified valid version**:
```sql
WITHIN 180045500 MILLISECONDS
```

**What it tests**: Time expression parsing

---

#### Test 13.20: Stress Test - Maximum Complexity ✅
```sql
FROM PATTERN (
    EVERY (
        (
            (e1=A[x > 10]{2,5} AND e2=B[y < 20]{1,3}) OR
            (e3=C[z == 'test']{3,} AND e4=D{1,2})
        ) ->
        (
            e5=E[val > e1[last].x AND val > e2[0].y] ->
            e6=F{5,10} ->
            NOT G[type == 'cancel'] FOR 30 minutes
        ) ->
        (e8=H OR e9=I OR e10=J)
    )
    WITHIN 2 HOURS
)
PARTITION BY userId, sessionId, deviceId
SELECT e1[0].timestamp as startTime,
       count(e1) + count(e2) + count(e3) + count(e4) as totalEvents,
       avg(e6.value) as avgE6,
       CASE
         WHEN count(e1) > 3 THEN 'HIGH'
         ELSE 'NORMAL'
       END as severity
INSERT ALL EVENTS INTO ComplexPatterns;
```
**Expected**: PASS (if all features implemented)

**Assertions**:
- Tests absolute maximum complexity
- All features combined

**What it tests**: Maximum grammar capabilities

---

## Additional Edge Cases & Negative Tests

### Test 14.1: Empty SELECT ❌
```sql
FROM PATTERN (e1=A -> e2=B)
SELECT
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Expected projection list in SELECT clause`

---

### Test 14.2: Missing INSERT Clause ❌
```sql
FROM PATTERN (e1=A -> e2=B)
SELECT e1.val;
```
**Expected**: REJECT (parser error)

**Error Message**: `Expected INSERT INTO after SELECT`

---

### Test 14.3: Missing SELECT Clause ❌
```sql
FROM PATTERN (e1=A -> e2=B)
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Expected SELECT clause`

---

### Test 14.4: Unbalanced Parentheses ❌
```sql
FROM PATTERN (
    e1=A -> e2=B
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Expected ')' to close pattern expression`

---

### Test 14.5: Extra Closing Parenthesis ❌
```sql
FROM PATTERN (
    e1=A -> e2=B))
)
SELECT *
INSERT INTO Results;
```
**Expected**: REJECT (parser error)

**Error Message**: `Unexpected ')'`

---

### Test 14.6: SQL Injection Prevention ✅
```sql
FROM PATTERN (
    e=LoginStream[username == 'admin' OR '1'='1']
)
SELECT *
INSERT INTO Results;
```
**Expected**: PASS (string literal parsed correctly, not as SQL injection)

**Assertions**:
- String literals properly escaped
- No SQL injection vulnerability

**What it tests**: String literal parsing safety

---

### Test 14.7: Unicode in Identifiers ✅/❌
```sql
FROM PATTERN (
    e1=用户登录Stream -> e2=数据访问Stream
)
SELECT e1.用户ID
INSERT INTO Results;
```
**Expected**: Depends on identifier rules

**If Unicode supported**: PASS
**If ASCII only**: REJECT

**What it tests**: Unicode identifier support

---

### Test 14.8: Very Long Identifier Names ✅
```sql
FROM PATTERN (
    thisIsAVeryLongEventAliasNameThatExceedsNormalLengthButShouldStillBeValidIfTheParserSupportsIt=StreamA
)
SELECT thisIsAVeryLongEventAliasNameThatExceedsNormalLengthButShouldStillBeValidIfTheParserSupportsIt.val
INSERT INTO Results;
```
**Expected**: PASS (if no identifier length limit)

**What it tests**: Identifier length limits

---

### Test 14.9: Comments in Pattern (if supported) ✅
```sql
FROM PATTERN (
    -- This matches logins
    e1=Login ->
    /* This matches logouts
       with multi-line comment */
    e2=Logout
)
SELECT e1.userId
INSERT INTO Sessions;
```
**Expected**: PASS (if comments supported)

**What it tests**: Comment support in patterns

---

### Test 14.10: Whitespace Tolerance ✅
```sql
FROM    PATTERN(e1=A->e2=B)SELECT e1.val INSERT INTO Results;
```
**Expected**: PASS (whitespace insensitive)

**What it tests**: Whitespace flexibility

---

### Test 14.11: Newline Tolerance ✅
```sql
FROM
PATTERN
(
e1=A
->
e2=B
)
SELECT
e1.val
INSERT
INTO
Results
;
```
**Expected**: PASS (newline insensitive)

**What it tests**: Newline handling

---

### Test 14.12: Trailing Semicolon ✅
```sql
FROM PATTERN (e1=A -> e2=B)
SELECT e1.val
INSERT INTO Results;
```
**Expected**: PASS (semicolon optional but allowed)

**What it tests**: Semicolon handling

---

### Test 14.13: Case Sensitivity - Keywords ✅/❌
```sql
from pattern (e1=A -> e2=B)
select e1.val
insert into Results;
```
**Expected**: PASS (if keywords case-insensitive) OR REJECT

**What it tests**: Keyword case sensitivity

---

### Test 14.14: Case Sensitivity - Stream Names ✅
```sql
FROM PATTERN (
    e1=LoginStream -> e2=loginstream
)
SELECT *
INSERT INTO Results;
```
**Expected**: Depends on schema

If LoginStream ≠ loginstream: Two different streams
If case-insensitive schema: Same stream

**What it tests**: Stream name case sensitivity

---

### Test 14.15: Null Event Handling (Runtime) ✅
```sql
FROM PATTERN (
    e1=A -> e2=B
)
SELECT e1.val, e2.val
INSERT INTO Results;
```
**Input**: null event in stream

**Expected**: Null events ignored or handled gracefully

**What it tests**: Null event resilience

---

## Test Data Fixtures

### Fixture 1: Login/Logout Sessions
```json
{
  "events": [
    {"type": "Login", "userId": "u1", "timestamp": 1000, "country": "US"},
    {"type": "Heartbeat", "timestamp": 2000},
    {"type": "DataAccess", "userId": "u1", "timestamp": 3000, "bytes": 5000000},
    {"type": "Logout", "userId": "u1", "timestamp": 4000}
  ]
}
```

### Fixture 2: Failed Login Attempts
```json
{
  "events": [
    {"type": "FailedLogin", "userId": "u1", "timestamp": 1000, "ipAddress": "192.168.1.100"},
    {"type": "FailedLogin", "userId": "u1", "timestamp": 2000, "ipAddress": "192.168.1.100"},
    {"type": "FailedLogin", "userId": "u1", "timestamp": 3000, "ipAddress": "192.168.1.100"},
    {"type": "FailedLogin", "userId": "u1", "timestamp": 4000, "ipAddress": "192.168.1.100"},
    {"type": "FailedLogin", "userId": "u1", "timestamp": 5000, "ipAddress": "192.168.1.100"},
    {"type": "AccountLocked", "userId": "u1", "timestamp": 6000}
  ]
}
```

### Fixture 3: Multi-User Partitioned
```json
{
  "events": [
    {"type": "FailedLogin", "userId": "u1", "timestamp": 1000},
    {"type": "FailedLogin", "userId": "u2", "timestamp": 1500},
    {"type": "FailedLogin", "userId": "u1", "timestamp": 2000},
    {"type": "FailedLogin", "userId": "u2", "timestamp": 2500},
    {"type": "FailedLogin", "userId": "u1", "timestamp": 3000},
    {"type": "AccountLocked", "userId": "u1", "timestamp": 4000},
    {"type": "AccountLocked", "userId": "u2", "timestamp": 4500}
  ]
}
```

### Fixture 4: Stock Price Movements
```json
{
  "events": [
    {"type": "StockPrice", "symbol": "AAPL", "price": 100.0, "timestamp": 1000},
    {"type": "StockPrice", "symbol": "AAPL", "price": 105.0, "timestamp": 2000},
    {"type": "StockPrice", "symbol": "AAPL", "price": 110.0, "timestamp": 3000},
    {"type": "StockPrice", "symbol": "AAPL", "price": 115.0, "timestamp": 4000}
  ]
}
```

### Fixture 5: Temperature Readings
```json
{
  "events": [
    {"type": "TempReading", "roomId": "R1", "temp": 101, "timestamp": 1000},
    {"type": "TempReading", "roomId": "R1", "temp": 103, "timestamp": 2000},
    {"type": "TempReading", "roomId": "R1", "temp": 105, "timestamp": 3000},
    {"type": "TempReading", "roomId": "R1", "temp": 102, "timestamp": 4000},
    {"type": "TempReading", "roomId": "R1", "temp": 104, "timestamp": 5000}
  ]
}
```

---

## Summary Statistics

**Total Test Cases**: 200
- **Positive Tests** (should PASS): 133
- **Negative Tests** (should REJECT): 67

**Coverage by Grammar Element**:
- ✅ PATTERN/SEQUENCE modes: 12 tests
- ✅ EVERY keyword: 25 tests
- ✅ Count quantifiers: 18 tests
- ✅ Array access: 15 tests
- ✅ PARTITION BY: 13 tests
- ✅ WITHIN clause: 11 tests
- ✅ OUTPUT event types: 8 tests
- ✅ Logical operators: 14 tests
- ✅ Sequence operator: 14 tests
- ✅ Absent patterns: 14 tests
- ✅ Filters & cross-stream: 18 tests
- ✅ Event aliases: 8 tests
- ✅ Complex scenarios: 30 tests

**Coverage by Validation Rule**:
- ✅ Rule 1 (EVERY at top level): 10 tests
- ✅ Rule 2 (Count quantifiers valid): 6 tests
- ✅ Rule 3 (PARTITION BY unique): 5 tests
- ✅ Rule 4 (EVERY in PATTERN only): 5 tests
- ✅ Rule 5 (No Absent in Logical): 6 tests

---

## Implementation Notes

### Test Framework Requirements

1. **Parser Testing**:
   - Unit tests for each grammar production
   - AST structure validation
   - Error message quality checks

2. **Validation Testing**:
   - Semantic rule enforcement
   - Cross-references validation
   - Type checking (if applicable)

3. **Runtime Testing**:
   - Event matching behavior
   - State management
   - Pattern instance creation/cleanup
   - Partition isolation

4. **Integration Testing**:
   - End-to-end query execution
   - Performance under load
   - Memory management
   - Concurrent pattern matching

### Test Organization

```
tests/
├── parser/
│   ├── pattern_mode_test.rs
│   ├── every_keyword_test.rs
│   ├── count_quantifier_test.rs
│   └── ...
├── validation/
│   ├── every_restrictions_test.rs
│   ├── partition_by_validation_test.rs
│   └── ...
├── runtime/
│   ├── pattern_matching_test.rs
│   ├── partition_isolation_test.rs
│   └── ...
└── integration/
    ├── end_to_end_test.rs
    └── performance_test.rs
```

### Test Data Management

- Use fixtures for consistent test data
- Parameterized tests for similar scenarios
- Property-based testing for edge cases
- Fuzz testing for parser robustness

---

**Document Version**: 1.0
**Test Coverage**: 200 test cases covering 100% of grammar features
**Status**: Ready for TDD Implementation
**Maintainer**: EventFlux Pattern Processing Team
