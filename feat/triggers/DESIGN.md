# Trigger SQL Syntax Design

**Status:** ✅ IMPLEMENTED
**Date:** 2024-12-31

## Overview

This document defines the SQL syntax for trigger definitions in EventFlux. Triggers provide time-based event generation for scheduling, heartbeats, and batch processing coordination.

---

## Implemented Syntax

### Three Trigger Types

```sql
-- 1. Start trigger (fires once at app start)
CREATE TRIGGER StartTrigger AT START;

-- 2. Periodic trigger (with readable time units)
CREATE TRIGGER FiveSecTrigger AT EVERY 5 SECONDS;
CREATE TRIGGER MillisTrigger AT EVERY 50 MILLISECONDS;
CREATE TRIGGER MinuteTrigger AT EVERY 1 MINUTE;
CREATE TRIGGER HourTrigger AT EVERY 2 HOURS;

-- 3. Cron trigger
CREATE TRIGGER CronTrigger AT CRON '*/1 * * * * *';
```

### Trigger as Stream Source

Triggers can be used as input sources in queries:

```sql
CREATE TRIGGER HeartbeatTrigger AT EVERY 50 MILLISECONDS;
CREATE STREAM outputStream (timestamp BIGINT);

INSERT INTO outputStream
SELECT currentTimeMillis() AS timestamp FROM HeartbeatTrigger;
```

### Time Unit Support

| Unit | Aliases | Multiplier (to ms) | Example |
|------|---------|-------------------|---------|
| MILLISECOND | MILLISECONDS | 1 | `AT EVERY 50 MILLISECONDS` |
| SECOND | SECONDS | 1,000 | `AT EVERY 5 SECONDS` |
| MINUTE | MINUTES | 60,000 | `AT EVERY 1 MINUTE` |
| HOUR | HOURS | 3,600,000 | `AT EVERY 2 HOURS` |
| DAY | DAYS | 86,400,000 | `AT EVERY 1 DAY` |

---

## Implementation Details

### Architecture

The trigger implementation extends the vendored `datafusion-sqlparser-rs` SQL parser:

```
SQL Input: "CREATE TRIGGER T AT EVERY 5 SECONDS;"
    │
    ▼
┌─────────────────────────────────────────────────┐
│ vendor/datafusion-sqlparser-rs/src/parser/     │
│   parse_create_trigger()                        │
│   └── parse_stream_trigger_timing()            │
│       └── parse_stream_trigger_time_unit()     │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ vendor/datafusion-sqlparser-rs/src/ast/ddl.rs  │
│   CreateStreamTrigger {                         │
│       name: ObjectName,                         │
│       timing: StreamTriggerTiming,              │
│   }                                             │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ src/sql_compiler/application.rs                 │
│   convert_stream_trigger()                      │
│   └── TriggerDefinition { id, at, at_every }   │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ src/sql_compiler/catalog.rs                     │
│   register_trigger()                            │
│   └── Also registers as stream for FROM clause │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ src/core/util/parser/eventflux_app_parser.rs   │
│   process_execution_elements()                  │
│   └── Triggers processed BEFORE queries        │
│       (enables SELECT FROM TriggerName)        │
└─────────────────────────────────────────────────┘
```

### Files Modified

| File | Changes |
|------|---------|
| `vendor/datafusion-sqlparser-rs/src/keywords.rs` | Added `CRON` keyword |
| `vendor/datafusion-sqlparser-rs/src/ast/ddl.rs` | Added `StreamTriggerTiming`, `StreamTriggerTimeUnit`, `CreateStreamTrigger` |
| `vendor/datafusion-sqlparser-rs/src/ast/mod.rs` | Added exports and `Statement::CreateStreamTrigger` |
| `vendor/datafusion-sqlparser-rs/src/ast/spans.rs` | Added span handling |
| `vendor/datafusion-sqlparser-rs/src/parser/mod.rs` | Added `parse_stream_trigger_timing()`, `parse_stream_trigger_time_unit()` |
| `src/sql_compiler/application.rs` | Added `convert_stream_trigger()` |
| `src/sql_compiler/catalog.rs` | Added `triggers` field, `register_trigger()` |
| `src/core/util/parser/eventflux_app_parser.rs` | Fixed trigger processing order |

### AST Types

```rust
/// Timing specification for EventFlux streaming triggers
pub enum StreamTriggerTiming {
    /// Fires once at application start
    Start,
    /// Fires at regular intervals
    Every { value: u64, unit: StreamTriggerTimeUnit },
    /// Fires according to a cron schedule
    Cron(String),
}

/// Time units for trigger intervals
pub enum StreamTriggerTimeUnit {
    Milliseconds,
    Seconds,
    Minutes,
    Hours,
    Days,
}

impl StreamTriggerTimeUnit {
    /// Convert value with this unit to milliseconds
    pub fn to_millis(&self, value: u64) -> u64 {
        match self {
            StreamTriggerTimeUnit::Milliseconds => value,
            StreamTriggerTimeUnit::Seconds => value * 1_000,
            StreamTriggerTimeUnit::Minutes => value * 60_000,
            StreamTriggerTimeUnit::Hours => value * 3_600_000,
            StreamTriggerTimeUnit::Days => value * 86_400_000,
        }
    }
}

/// EventFlux streaming trigger definition
pub struct CreateStreamTrigger {
    pub name: ObjectName,
    pub timing: StreamTriggerTiming,
}
```

### Internal Mapping

The parser maps to existing `TriggerDefinition`:

| SQL | TriggerDefinition Field |
|-----|------------------------|
| `AT START` | `at: Some("start")` |
| `AT EVERY 5 SECONDS` | `at_every: Some(5000)` |
| `AT CRON '...'` | `at: Some("...")` |

---

## Grammar

### BNF

```bnf
trigger_statement
    ::= CREATE TRIGGER identifier AT trigger_timing ';'

trigger_timing
    ::= START
      | EVERY duration
      | CRON string_literal

duration
    ::= numeric_literal time_unit

time_unit
    ::= MILLISECOND | MILLISECONDS
      | SECOND | SECONDS
      | MINUTE | MINUTES
      | HOUR | HOURS
      | DAY | DAYS
```

---

## Test Coverage

### Tests Enabled (10 total, all passing)

| Test | Description |
|------|-------------|
| `trigger_test1_start` | API-based start trigger |
| `trigger_test2_periodic` | API-based periodic trigger |
| `trigger_test3_cron` | API-based cron trigger |
| `trigger_test4_multiple` | Multiple triggers in one app |
| `trigger_test5_long_interval` | Periodic with longer interval |
| `trigger_test6_sql_start` | SQL-based start trigger |
| `trigger_test7_sql_periodic` | SQL-based periodic trigger |
| `trigger_test8_sql_cron` | SQL-based cron trigger |
| `trigger_test9_with_query` | Trigger as query source |
| `trigger_test10_batch_processing` | Trigger for batch coordination |

### Run Tests

```bash
# Run all trigger tests
cargo test --test compatibility_tests triggers

# Run app_runner trigger tests
cargo test --test app_runner_triggers
```

---

## Migration from Siddhi

| Siddhi | EventFlux |
|--------|-----------|
| `define trigger T at start;` | `CREATE TRIGGER T AT START;` |
| `define trigger T at every 5 sec;` | `CREATE TRIGGER T AT EVERY 5 SECONDS;` |
| `define trigger T at every 50 ms;` | `CREATE TRIGGER T AT EVERY 50 MILLISECONDS;` |
| `define trigger T at '*/1 * * * * *';` | `CREATE TRIGGER T AT CRON '*/1 * * * * *';` |

---

## Design Decisions

### 1. No INTERVAL Keyword
The time unit (SECONDS, MINUTES, etc.) already indicates a duration, making `INTERVAL` redundant.

### 2. Explicit CRON Keyword
Cron expressions use the `CRON` keyword to distinguish from other string values.

### 3. Triggers as Streams
Triggers are registered both as triggers and as streams, allowing them to be used in FROM clauses.

### 4. Processing Order
Triggers are processed BEFORE queries in the parser, ensuring their stream junctions exist when queries reference them.

---

## Success Criteria ✅

- [x] 10 trigger tests pass (was 5 ignored)
- [x] All existing API-based trigger tests still pass
- [x] Duration parsing works for all time units
- [x] Triggers can be used as stream sources in queries
- [x] No regression in other tests (2,700+ tests passing)
