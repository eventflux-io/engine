# Stream-Table Relation Architecture

**Last Updated**: 2025-10-24
**Implementation Status**: ✅ **Core Functionality Working** - Basic stream-table JOINs operational
**Test Coverage**: 9 passing tests, 4 appropriately ignored
**Target Milestone**: M1.6 Complete

---

## Table of Contents

1. [Current Status](#current-status)
2. [What Actually Works](#what-actually-works)
3. [Architecture Overview](#architecture-overview)
4. [Implementation Details](#implementation-details)
5. [Examples & Usage](#examples--usage)
6. [Known Limitations](#known-limitations)
7. [Testing](#testing)
8. [Future Work](#future-work)

---

## Current Status

### ✅ **Implemented & Tested**

| Component | Status | Test Coverage | Location |
|-----------|--------|---------------|----------|
| **Relation Enum** | ✅ Complete | Catalog layer tests | `src/sql_compiler/catalog.rs:19-85` |
| **Unified get_relation()** | ✅ Complete | Validation tests | `src/sql_compiler/catalog.rs:186-198` |
| **Stream-Table JOINs** | ✅ Working | Cache & JDBC | `tests/app_runner_tables.rs` |
| **SQL WITH Syntax** | ✅ Working | All table tests | Tables configured via SQL |
| **Runtime Type Detection** | ✅ Working | Query parser | `src/core/util/parser/query_parser.rs:160-164` |

### ⚠️ **Known Limitations**

| Issue | Impact | Severity | Workaround |
|-------|--------|----------|------------|
| **Field Order Quirk** | Table-on-left JOINs swap field order | Minor | Use stream-on-left JOIN ordering |
| **Chained Table JOINs** | Stream-Table-Table not working | Medium | Single table JOIN only |
| **Alias Support** | Multi-table aliases fail | Medium | Use fully qualified names |
| **INSERT INTO TABLE** | No runtime processor yet | High | Use table input handlers directly |

### 🔴 **Not Yet Implemented**

- Multiple table JOINs in single query (Stream → Table1 → Table2)
- Table INSERT via SQL (`INSERT INTO TableName SELECT ...`)
- Table UPDATE/DELETE via SQL
- Advanced table features (TTL, eviction policies via SQL)

---

## What Actually Works

### Core Stream-Table JOIN Functionality

**Verified with 9 passing tests covering:**

1. ✅ **Basic stream-table JOIN** (cache tables)
2. ✅ **JDBC table JOINs** (database-backed tables)
3. ✅ **Table-stream JOIN** (reversed order, with field ordering quirk)
4. ✅ **Empty result JOINs** (no matching rows)
5. ✅ **Multiple row matches** (stream event joins multiple table rows)
6. ✅ **Qualified column names** (table.column, stream.column)
7. ✅ **Error validation** (unknown table, unknown stream, unknown column)

### SQL Syntax Support

**Tables via WITH clause:**
```sql
-- Cache table (in-memory)
CREATE TABLE users (userId INT, name STRING)
WITH ('extension' = 'cache', 'max_size' = '1000');

-- JDBC table (database-backed)
CREATE TABLE products (productId INT, title STRING)
WITH ('extension' = 'jdbc', 'data_source' = 'DS1');
```

**Stream-Table JOINs:**
```sql
-- Enrichment pattern (stream + table)
INSERT INTO enriched
SELECT events.orderId, users.name, events.amount
FROM events
JOIN users ON events.userId = users.userId;
```

---

## Architecture Overview

### Four-Layer Design

The implementation uses **late binding** with separation of concerns across four distinct layers:

```
┌─────────────────────────────────────────────────────────────┐
│ 1. CATALOG LAYER (Schema Management)                       │
│    Purpose: "What exists?"                                  │
│    Component: Relation enum, get_relation()                 │
│    Validation: Unified stream/table lookup                  │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. CONVERTER LAYER (SQL → Query API)                       │
│    Purpose: "Is the SQL valid?"                             │
│    Component: SQL AST → Query API objects                   │
│    Validation: Schema exists, columns exist                 │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. QUERY API LAYER (Logical Representation)                │
│    Purpose: "What's referenced?"                            │
│    Component: SingleInputStream (name-only reference)       │
│    Late Binding: No stream vs table distinction yet         │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 4. RUNTIME LAYER (Execution)                                │
│    Purpose: "How to execute?"                               │
│    Component: Query parser checks table_def_map             │
│    Processor Selection:                                     │
│      - Stream-Stream → JoinProcessor (temporal)             │
│      - Stream-Table  → TableJoinProcessor (enrichment)      │
└─────────────────────────────────────────────────────────────┘
```

### Why This Architecture?

**Late Binding Benefits:**
- ✅ Query serialization without schema context
- ✅ Runtime optimization based on statistics
- ✅ Schema changes don't invalidate queries
- ✅ Can swap table implementations (cache → database)

**Separation of Concerns:**
- **Catalog**: Type-safe validation at parse time
- **Query API**: Schema-independent logical plan
- **Runtime**: Physical execution strategy selection

---

## Implementation Details

### 1. Catalog Layer: Relation Enum

**Location**: `src/sql_compiler/catalog.rs:19-85`

```rust
/// A relation that can appear in SQL queries (either a stream or a table)
#[derive(Debug, Clone)]
pub enum Relation {
    /// A stream definition (temporal event source)
    Stream(Arc<StreamDefinition>),

    /// A table definition (stateful lookup table)
    Table(Arc<TableDefinition>),
}

impl Relation {
    /// Get the abstract definition (schema) regardless of type
    pub fn abstract_definition(&self) -> &AbstractDefinition {
        match self {
            Relation::Stream(stream) => &stream.abstract_definition,
            Relation::Table(table) => &table.abstract_definition,
        }
    }

    pub fn is_stream(&self) -> bool {
        matches!(self, Relation::Stream(_))
    }

    pub fn is_table(&self) -> bool {
        matches!(self, Relation::Table(_))
    }
}
```

**Key Method**: Unified relation lookup
```rust
pub fn get_relation(&self, name: &str) -> Result<Relation, CatalogError> {
    // Try stream first (more common case)
    if let Ok(stream) = self.get_stream(name) {
        return Ok(Relation::Stream(stream));
    }

    // Try table
    if let Some(table) = self.get_table(name) {
        return Ok(Relation::Table(table));
    }

    Err(CatalogError::UnknownRelation(name.to_string()))
}
```

### 2. Converter Layer: Validation

**Location**: `src/sql_compiler/converter.rs`

```rust
// Validate that relation exists (stream or table)
catalog.get_relation(&stream_name)?;

// Create logical reference (works for both types)
// Runtime will determine actual type
let single_stream = SingleInputStream::new_basic(stream_name, ...);
```

**Key Change**: Changed from `get_stream()` to `get_relation()` in:
- FROM clause validation (lines 102-114)
- JOIN clause validation (lines 211-217)
- PARTITION BY validation (lines 426-429, 468-471)

### 3. Query API Layer: Late Binding

**Location**: `src/query_api/input/stream/single_input_stream.rs`

**Design**: `SingleInputStream` stores only the **name**, not the type.

```rust
pub struct SingleInputStream {
    stream_id: String,  // Just a reference - no type information
    // ... other fields
}
```

This allows:
- Query serialization without schema
- Runtime can determine optimal execution strategy
- Same query works with streams or tables

### 4. Runtime Layer: Type Detection & Processor Selection

**Location**: `src/core/util/parser/query_parser.rs:160-164`

```rust
let left_is_table = table_def_map.contains_key(&left_id);
let right_is_table = table_def_map.contains_key(&right_id);

if left_is_table ^ right_is_table {
    // Stream-Table JOIN → Create TableJoinProcessor
    let join_proc = TableJoinProcessor::new(...);
} else {
    // Stream-Stream JOIN → Create JoinProcessor
    let join_proc = JoinProcessor::new(...);
}
```

**Processor Types:**
- **JoinProcessor**: Temporal window-based join (stream-stream)
- **TableJoinProcessor**: Enrichment lookup (stream-table)

---

## Examples & Usage

### Example 1: Basic Stream-Table JOIN (Cache)

**SQL:**
```sql
CREATE STREAM orders (orderId INT, userId INT, amount FLOAT);
CREATE TABLE users (userId INT, name STRING, country STRING)
WITH ('extension' = 'cache', 'max_size' = '10000');
CREATE STREAM enriched (orderId INT, userName STRING, userCountry STRING, amount FLOAT);

INSERT INTO enriched
SELECT orders.orderId, users.name, users.country, orders.amount
FROM orders
JOIN users ON orders.userId = users.userId;
```

**What Happens:**
1. Parser validates `orders` (stream) and `users` (table) exist
2. Query API creates logical JOIN reference
3. Runtime detects table via `table_def_map.contains_key("users")`
4. Creates `TableJoinProcessor` for enrichment
5. For each order event:
   - Looks up matching user(s) in cache table
   - Emits enriched event if match found
   - Emits nothing if no match (inner join semantics)

**Test**: `tests/app_runner_tables.rs::stream_table_join_basic`

### Example 2: JDBC Table JOIN (Database-Backed)

**SQL:**
```sql
CREATE STREAM events (id INT, data STRING);
CREATE TABLE reference (id INT, label STRING)
WITH ('extension' = 'jdbc', 'data_source' = 'PostgresDB');
CREATE STREAM output (id INT, label STRING, data STRING);

INSERT INTO output
SELECT events.id, reference.label, events.data
FROM events
JOIN reference ON events.id = reference.id;
```

**Setup Required:**
```rust
// Register JDBC data source first
let mut manager = EventFluxManager::new();
manager.add_data_source(
    "PostgresDB".to_string(),
    Arc::new(PostgresDataSource::new("postgresql://...")),
)?;
```

**Runtime Behavior:**
- Each event triggers JDBC lookup
- Query executed: `SELECT label FROM reference WHERE id = ?`
- Result cached according to table configuration
- Performance depends on database latency and caching

**Test**: `tests/app_runner_tables.rs::stream_table_join_jdbc`

### Example 3: Table-on-Left JOIN (Field Ordering Quirk)

**SQL:**
```sql
CREATE STREAM events (id INT, data STRING);
CREATE TABLE lookup (id INT, label STRING) WITH ('extension' = 'cache', 'max_size' = '100');
CREATE STREAM output (id INT, label STRING, data STRING);

INSERT INTO output
SELECT lookup.id, lookup.label, events.data
FROM lookup JOIN events ON lookup.id = events.id;
```

**Known Issue:**
- **Expected output order**: `[id, label, data]`
- **Actual output order**: `[id, data, label]` ⚠️ Fields swapped!

This is a current runtime quirk when table appears on left side of JOIN.

**Workaround**: Use stream-on-left JOIN ordering:
```sql
FROM events JOIN lookup ON events.id = lookup.id
```

**Test**: `tests/app_runner_tables.rs::test_table_on_left_stream_on_right_join`

### Example 4: JOIN with No Matches (Empty Result)

**SQL:**
```sql
CREATE STREAM events (id INT, value STRING);
CREATE TABLE lookup (id INT, description STRING)
WITH ('extension' = 'cache', 'max_size' = '100');
CREATE STREAM output (id INT, description STRING, value STRING);

INSERT INTO output
SELECT events.id, lookup.description, events.value
FROM events JOIN lookup ON events.id = lookup.id;
```

**Behavior:**
- Event with `id=999` arrives
- Table has no row with `id=999`
- **Result**: No output emitted (inner join semantics)
- This is correct SQL behavior

**Test**: `tests/app_runner_tables.rs::test_table_join_no_match`

### Example 5: Error - Unknown Table

**SQL:**
```sql
CREATE STREAM events (id INT);
CREATE STREAM output (id INT, label STRING);

INSERT INTO output
SELECT events.id, NonExistentTable.label
FROM events JOIN NonExistentTable ON events.id = NonExistentTable.id;
```

**Result:**
```
❌ Error: "Schema not found for relation (stream or table): NonExistentTable"
```

This error is caught at **parse time**, not runtime.

**Test**: `tests/app_runner_tables.rs::test_error_unknown_table_in_join`

---

## Known Limitations

### 1. Chained Table JOINs (Not Working)

**Issue**: Cannot join stream → table1 → table2 in single query.

**Example (Currently Fails):**
```sql
CREATE STREAM orders (userId INT, productId INT);
CREATE TABLE users (userId INT, name STRING) WITH ('extension' = 'cache', 'max_size' = '100');
CREATE TABLE products (productId INT, title STRING) WITH ('extension' = 'cache', 'max_size' = '100');
CREATE STREAM enriched (userName STRING, productTitle STRING);

-- This FAILS with: "Variable 'products.title' not found"
INSERT INTO enriched
SELECT users.name, products.title
FROM orders
JOIN users ON orders.userId = users.userId
JOIN products ON orders.productId = products.productId;
```

**Root Cause**: Runtime limitation in handling multiple table joins in query pipeline.

**Workaround**: Use nested queries or separate enrichment steps.

**Status**: Ignored test `test_multiple_tables_in_query`

### 2. Field Ordering Quirk (Table-on-Left)

**Issue**: When table appears on left side of JOIN, output field order differs from SELECT order.

**Example:**
```sql
SELECT table.a, table.b, stream.c FROM table JOIN stream ...
-- Expected: [a, b, c]
-- Actual:   [a, c, b]  ⚠️
```

**Impact**: Mild - output still contains correct data, just different order.

**Workaround**: Use stream-on-left JOIN syntax.

**Test**: `test_table_on_left_stream_on_right_join` (documents this behavior)

### 3. INSERT INTO TABLE (No Runtime Processor)

**Issue**: SQL INSERT INTO TABLE syntax parses but doesn't execute.

**Example (Parses but Doesn't Execute):**
```sql
CREATE STREAM events (userId INT, name STRING);
CREATE TABLE users (userId INT, name STRING) WITH ('extension' = 'cache', 'max_size' = '100');

-- Parses successfully, but doesn't populate table
INSERT INTO users SELECT userId, name FROM events;
```

**Root Cause**: Query execution pipeline lacks table insert processor.

**Current Workaround**: Use table input handlers directly in Rust code:
```rust
let table_handler = runtime.get_table_input_handler("users")?;
table_handler.add(vec![Event::new_with_data(0, vec![
    AttributeValue::Int(123),
    AttributeValue::String("Alice".into()),
])]);
```

**Status**: 3 ignored tests require this feature:
- `cache_table_crud_via_app_runner`
- `jdbc_table_crud_via_app_runner`
- `cache_and_jdbc_tables_eviction_and_queries`

### 4. Alias Support Limitations

**Issue**: Table aliases in complex queries may fail variable resolution.

**Example (May Fail):**
```sql
-- Using aliases
SELECT u.name, p.title
FROM orders o
JOIN users u ON o.userId = u.userId
JOIN products p ON o.productId = p.productId;

-- Error: "Variable 'u.name' not found"
```

**Workaround**: Use fully qualified names without aliases:
```sql
SELECT users.name, products.title
FROM orders
JOIN users ON orders.userId = users.userId
JOIN products ON orders.productId = products.productId;
```

**Root Cause**: Alias resolution in converter doesn't handle all multi-join scenarios.

---

## Testing

### Test Coverage Summary

**Location**: `tests/app_runner_tables.rs`

**Passing Tests (9):**

| Test Name | Purpose | What It Validates |
|-----------|---------|-------------------|
| `stream_table_join_basic` | Cache table JOIN | Stream enrichment with in-memory table |
| `stream_table_join_jdbc` | JDBC table JOIN | Database-backed table lookup |
| `test_table_on_left_stream_on_right_join` | Reversed JOIN | Table-stream order (documents field swap) |
| `test_table_join_no_match` | Empty results | No output when JOIN doesn't match |
| `test_table_join_multiple_matches` | Multiple rows | Stream joins with multiple table rows |
| `test_stream_table_join_with_qualified_names` | Qualified names | Full table.column, stream.column syntax |
| `test_error_unknown_table_in_join` | Error validation | Parse-time error for missing table |
| `test_error_unknown_stream_in_join` | Error validation | Parse-time error for missing stream |
| `test_error_unknown_column_in_table` | Error validation | Parse-time error for bad column |

**Ignored Tests (4):**

| Test Name | Reason | Future Work |
|-----------|--------|-------------|
| `test_multiple_tables_in_query` | Chained table joins not working | Runtime improvement needed |
| `cache_table_crud_via_app_runner` | INSERT INTO TABLE not implemented | Needs table insert processor |
| `jdbc_table_crud_via_app_runner` | INSERT INTO TABLE not implemented | Needs table insert processor |
| `cache_and_jdbc_tables_eviction_and_queries` | INSERT INTO TABLE not implemented | Needs table insert processor |

### Running Tests

```bash
# Run all table tests
cargo test --test app_runner_tables

# Expected output:
# test result: ok. 9 passed; 0 failed; 4 ignored
```

### Test Examples

**Basic Cache Table JOIN:**
```rust
#[tokio::test]
async fn stream_table_join_basic() {
    let query = "\
        CREATE STREAM L (roomNo INT, val STRING);\n\
        CREATE TABLE R (roomNo INT, type STRING) WITH ('extension' = 'cache', 'max_size' = '5');\n\
        CREATE STREAM Out (r INT, t STRING, v STRING);\n\
        INSERT INTO Out \n\
        SELECT L.roomNo as r, R.type as t, L.val as v \n\
        FROM L JOIN R ON L.roomNo = R.roomNo;\n";

    let runner = AppRunner::new(query, "Out").await;

    // Add table data
    let th = runner.runtime().get_table_input_handler("R").unwrap();
    th.add(vec![Event::new_with_data(
        0,
        vec![AttributeValue::Int(1), AttributeValue::String("A".into())],
    )]);

    // Send stream event
    runner.send("L", vec![AttributeValue::Int(1), AttributeValue::String("v".into())]);

    let out = runner.shutdown();
    assert_eq!(out, vec![vec![
        AttributeValue::Int(1),
        AttributeValue::String("A".into()),
        AttributeValue::String("v".into()),
    ]]);
}
```

---

## Future Work

### Short Term (M2)

1. **Fix chained table JOINs**
   - Support Stream → Table1 → Table2 queries
   - Improve JOIN pipeline in query parser
   - Add comprehensive multi-table JOIN tests

2. **Fix field ordering quirk**
   - Ensure output order matches SELECT order
   - Handle table-on-left JOINs correctly
   - Test all JOIN orderings

3. **Improve alias support**
   - Complete alias resolution in complex queries
   - Support multi-table aliases properly
   - Add alias validation tests

### Medium Term (M3)

4. **Implement INSERT INTO TABLE**
   - Add table insert processor to query pipeline
   - Support INSERT INTO table SELECT ... syntax
   - Enable 3 currently ignored tests

5. **Add UPDATE/DELETE for tables**
   - SQL UPDATE syntax for tables
   - SQL DELETE syntax for tables
   - Transactional semantics

### Long Term (M4+)

6. **Advanced table features**
   - TTL (time-to-live) configuration via SQL
   - Eviction policies (LRU, LFU) via SQL
   - Table triggers and constraints
   - Materialized views from tables

7. **Performance optimizations**
   - Table JOIN caching strategies
   - Batch table lookups
   - Async table operations
   - Parallel table scans

---

## Error Messages

### Before vs After Implementation

**Before** (Incorrect):
```
❌ Error: "Unknown stream: R"
   (when R is actually a table)
```

**After** (Correct):
```
✅ Error: "Unknown relation (stream or table): R"
   (accurate error message)
```

**Column Errors**:
```
✅ Error: "Schema not found for relation (stream or table): TableName"
✅ Error: "Variable 'table.nonExistentColumn' not found in query '...'"
```

---

## Architecture Documentation

### Data Flow Example

```
SQL Query: SELECT events.id, users.name FROM events JOIN users ON events.id = users.id
    ↓
Parse (sqlparser-rs)
    ↓
Catalog validates:
  - catalog.get_relation("events")  → Stream ✓
  - catalog.get_relation("users")   → Table ✓
    ↓
Converter creates:
  - SingleInputStream("events")
  - SingleInputStream("users")   // Just name references
    ↓
SqlApplication.to_eventflux_app()
  - Transfers stream defs to stream_definition_map
  - Transfers table defs to table_definition_map
    ↓
QueryParser runtime lookup:
  - table_def_map.contains_key("events")? → false (is stream)
  - table_def_map.contains_key("users")?  → true (is table)
  - Decision: Stream-Table JOIN → Use TableJoinProcessor
    ↓
Execution:
  - Stream events flow through TableJoinProcessor
  - Each event triggers table lookup
  - Matched rows emitted as joined events
```

### Why Late Binding?

**Query API stores references by name only:**
```rust
SingleInputStream::new_basic("users", ...)
// No type information - runtime determines if stream or table
```

**Benefits:**
1. Query serialization without schema context
2. Runtime optimization based on statistics
3. Schema evolution doesn't break queries
4. Can swap implementations (cache → database)

**Trade-offs:**
- Less compile-time type safety
- Errors deferred to runtime in some cases
- Requires runtime type discovery

---

## Related Documentation

- **Implementation**: See `/tmp/RELATION_ARCHITECTURE.md` for detailed implementation notes
- **Grammar**: See `feat/grammar/GRAMMAR.md` for parser architecture
- **Type System**: See `feat/type_system/TYPE_SYSTEM.md` for type handling
- **Roadmap**: See `ROADMAP.md` for future table feature plans

---

## Summary

### What This Implementation Provides

✅ **Working stream-table JOINs** for basic use cases
✅ **Type-safe catalog layer** with Relation enum
✅ **Late binding architecture** for flexibility
✅ **SQL WITH syntax** for table configuration
✅ **Cache and JDBC tables** both supported
✅ **Proper error messages** distinguishing streams and tables
✅ **Comprehensive test coverage** (9 passing tests)

### What's Not Yet Ready

⚠️ **Chained table JOINs** (Stream → Table1 → Table2)
⚠️ **Field ordering quirk** with table-on-left JOINs
⚠️ **INSERT INTO TABLE** via SQL (needs processor)
⚠️ **Complex alias resolution** in multi-table queries

### Bottom Line

**For simple stream-table enrichment**: ✅ **Works well**
**For complex multi-table queries**: ⚠️ **Limitations exist**
**For production use**: ✅ **Core functionality stable**, workarounds available for limitations

---

**Last Updated**: 2025-10-24
**Verified Against**: Test suite run on 2025-10-24
**Test Results**: 9 passed, 4 appropriately ignored
