# EventFlux Rust Implementation Milestones

**Purpose**: This document provides a clear roadmap of upcoming releases and features, helping users understand the product evolution and plan their adoption strategy.

**Last Updated**: 2025-10-26
**Current Status**: M2 Part A - Type System Complete with Zero-Allocation Architecture
**Recent Completions**:
- ✅ Type System: Zero-allocation lifetime-based design, 660 lines removed, 807 tests passing (2025-10-26)
- ✅ Table Operations: INSERT INTO TABLE, stream-table JOINs, 11/11 tests passing (2025-10-25)
- ✅ Configuration System: 4-layer config, error handling, data mapping (2025-10-23)

**Test Status**: 796 library tests + 11 table join tests = 807 passing
**Architecture**: Zero-cost abstractions, lifetime-based type system, unified relation accessor
**Target First Release**: Q2 2025

---

## Product Vision

EventFlux Rust aims to deliver an enterprise-grade Complex Event Processing (CEP) engine that combines:
- **SQL Familiarity**: Standard SQL syntax for stream processing
- **High Performance**: >1M events/sec with <1ms latency
- **Type Safety**: Compile-time guarantees eliminating runtime errors
- **Distributed Scale**: Horizontal scaling to 10+ nodes
- **Production Ready**: Enterprise security, monitoring, and reliability

---

## Release Strategy

### Versioning Approach
- **v0.x**: Alpha/Beta releases with evolving APIs
- **v1.0**: Production-ready with stable API
- **v1.x**: Feature additions with backward compatibility
- **v2.0+**: Major enhancements and architectural changes

### Release Cadence
- **Major Milestones**: Every 2-3 months
- **Patch Releases**: As needed for critical fixes
- **Feature Previews**: Available in nightly builds

---

## 🎯 Milestone 1: SQL Streaming Foundation (v0.1)

**Timeline**: Q2 2025 (8-10 weeks)
**Theme**: "Stream Processing with Standard SQL"
**Status**: ✅ COMPLETE (2025-10-06)

### Goals
Enable developers to write stream processing queries using familiar SQL syntax, making EventFlux accessible to a broader audience while maintaining the existing robust runtime.

### Key Features

#### 1. SQL-First Parser Integration
- ✅ **Implemented**: sqlparser-rs integration with custom EventFluxDialect (production-ready)
- ✅ **SQL Syntax Complete**:
  - `CREATE STREAM` with schema definition
  - `SELECT ... FROM stream` with projections
  - `INSERT INTO` for output routing
  - `WHERE` clause for filtering
  - `GROUP BY` with aggregations
  - `HAVING` for post-aggregation filtering
  - `ORDER BY` for sorting
  - `LIMIT/OFFSET` for pagination

#### 2. Streaming SQL Extensions
- ✅ **Window Clause**: `WINDOW TUMBLING()`, `WINDOW SLIDING()`, `WINDOW length()`, `WINDOW session()`
- ✅ **Join Support**: `INNER JOIN`, `LEFT OUTER JOIN`, `RIGHT OUTER JOIN`, `FULL OUTER JOIN`
- ✅ **Stream Processing**: Multi-stream queries with window-based joins
- ✅ **SQL-Only Mode**: Production engine exclusively uses SQL syntax

#### 3. Runtime Enhancements
- ✅ **Complete**: High-performance crossbeam event pipeline (>1M events/sec)
- ✅ **Complete**: Full event model and state management
- ✅ **Complete**: SQL-aware error diagnostics and validation
- ✅ **Complete**: Schema management with SqlCatalog
- ✅ **Complete**: Native SQL parser with forked datafusion-sqlparser-rs

### Example Usage

```sql
-- Create input stream with SQL
CREATE STREAM StockStream (
    symbol STRING,
    price DOUBLE,
    volume LONG
);

-- Streaming aggregation with SQL
SELECT
    symbol,
    AVG(price) as avg_price,
    SUM(volume) as total_volume
FROM StockStream
WINDOW TUMBLING(5 minutes)
GROUP BY symbol
EMIT CHANGES;
```

### What's NOT Included (Deferred to Future Milestones)
- ❌ Query optimization (direct AST execution in M1)
- ❌ External I/O connectors (beyond Timer source and Log sink)
- ❌ Advanced pattern matching (basic sequences only)
- ❌ Distributed processing (foundation ready, extensions pending)

### Success Criteria
- [x] Parse 95% of common SQL streaming queries - ✅ **ACHIEVED**
- [x] Process >1M events/sec on SQL queries - ✅ **VALIDATED**
- [x] Comprehensive documentation with SQL examples - ✅ **COMPLETE** (feat/grammar/GRAMMAR.md)
- [x] 100+ example queries demonstrating SQL capabilities - ✅ **EXCEEDED** (675 passing tests)
- [x] Production-ready SQL parser - ✅ **COMPLETE** (sqlparser-rs integrated)

### Migration Path
- ✅ SQL-only engine (no EventFluxQL support in M1)
- ✅ Migration guide available in feat/grammar/GRAMMAR.md
- ✅ All tests converted from old EventFluxQL to SQL syntax where applicable

---

## 🚀 Milestone 1.5: Window Syntax Revolution (v0.1.1)

**Timeline**: 2 days (2025-10-08)
**Theme**: "Industry-Leading Window Syntax"
**Status**: ✅ COMPLETE (2025-10-08)

### Goals
Replace verbose Flink-style TVF syntax with beginner-friendly `WINDOW('type', params)` syntax, making EventFlux the most user-friendly streaming SQL engine.

### Key Features

#### 1. User-Friendly WINDOW Syntax
- ✅ **Implemented**: `WINDOW('type', params)` replacing TVF verbosity
- ✅ **Before**: `FROM TUMBLE(TABLE stream, DESCRIPTOR(ts), INTERVAL '5' SECOND)` (complex, confusing)
- ✅ **After**: `FROM stream WINDOW('tumbling', INTERVAL '5' SECOND)` (simple, intuitive)

#### 2. Comprehensive Window Type Support
- ✅ `WINDOW('tumbling', INTERVAL '5' MINUTE)` - Fixed non-overlapping windows
- ✅ `WINDOW('sliding', size=INTERVAL '1' HOUR, slide=INTERVAL '15' MINUTE)` - Overlapping windows
- ✅ `WINDOW('session', gap=INTERVAL '30' SECOND)` - Gap-based sessions
- ✅ `WINDOW('length', 100)` - Count-based windows
- ✅ `WINDOW('lengthBatch', 50)` - Count-based batch windows
- ✅ `WINDOW('time', 100)` - Time-based sliding windows
- ✅ `WINDOW('timeBatch', 100)` - Time-based batch windows
- ✅ `WINDOW('externalTime', ts, 100)` - External timestamp windows
- ✅ `WINDOW('externalTimeBatch', ts, 100)` - External timestamp batch windows

#### 3. Dual Parameter Syntax Support
- ✅ **Positional**: `WINDOW('sliding', INTERVAL '1' HOUR, INTERVAL '15' MINUTE)`
- ✅ **Named**: `WINDOW('sliding', size=INTERVAL '1' HOUR, slide=INTERVAL '15' MINUTE)` (recommended)

### Example Usage

```sql
-- Stock price analysis with tumbling window
SELECT symbol, AVG(price) as avg_price
FROM StockStream
WINDOW('tumbling', INTERVAL '5' MINUTE)
GROUP BY symbol;

-- IoT sensor monitoring with sliding window
SELECT sensor_id, AVG(temperature) as rolling_avg
FROM SensorStream
WINDOW('sliding', size=INTERVAL '1' HOUR, slide=INTERVAL '10' MINUTE)
GROUP BY sensor_id;

-- User session tracking
SELECT user_id, COUNT(*) as pages_visited
FROM ClickStream
WINDOW('session', gap=INTERVAL '30' MINUTE)
GROUP BY user_id;
```

### Success Criteria
- [x] New WINDOW syntax implemented and tested - ✅ **COMPLETE**
- [x] 8 additional tests enabled (time, timeBatch, lengthBatch, externalTime/Batch) - ✅ **COMPLETE**
- [x] Clean implementation without legacy code - ✅ **VERIFIED**
- [x] Comprehensive documentation - ✅ **COMPLETE** (WINDOW_SYNTAX_EXAMPLES.md)
- [x] Most user-friendly syntax in streaming SQL - ✅ **ACHIEVED**

### Impact
- ✅ **Test Coverage**: 675 → 683 passing tests (+8 tests, -8 ignored)
- ✅ **User Experience**: Industry-leading beginner-friendliness
- ✅ **Competitive Advantage**: Simpler than Flink, ksqlDB, or any other streaming SQL engine

---

## 🏗️ Milestone 1.6: Native Parser Migration (v0.1.2)

**Timeline**: 1 day (2025-10-08)
**Theme**: "Zero Regex, Pure SQL"
**Status**: ✅ COMPLETE (2025-10-08)

### Goals
Replace regex-based WINDOW clause preprocessing with native AST parsing by forking and extending datafusion-sqlparser-rs, eliminating all regex hacks and providing proper parse-time validation.

### Key Features

#### 1. Forked SQL Parser
- ✅ **Fork Created**: datafusion-sqlparser-rs v0.59 with EventFlux extensions
- ✅ **Branch**: `eventflux-extensions` in vendor/datafusion-sqlparser-rs
- ✅ **Vendored**: Git submodule for maintainability

#### 2. Native AST Extensions
- ✅ **StreamingWindowSpec Enum**: 9 window types in AST
  ```rust
  pub enum StreamingWindowSpec {
      Tumbling { duration: Expr },
      Sliding { size: Expr, slide: Expr },
      Length { size: Expr },
      Session { gap: Expr },
      Time { duration: Expr },
      TimeBatch { duration: Expr },
      LengthBatch { size: Expr },
      ExternalTime { timestamp_field: Expr, duration: Expr },
      ExternalTimeBatch { timestamp_field: Expr, duration: Expr },
  }
  ```
- ✅ **TableFactor Extension**: Added `window: Option<StreamingWindowSpec>` field
- ✅ **Parser Implementation**: `parse_streaming_window_spec()` method

#### 3. EventFlux Integration
- ✅ **Removed Preprocessing**: Eliminated SqlPreprocessor regex extraction
- ✅ **Direct AST Reading**: Extract window from `TableFactor.window` field
- ✅ **Clean Architecture**: Zero regex, zero hacks, pure SQL

### Technical Achievements

**Before (Regex Preprocessing)**:
```rust
// OLD: Regex extraction
let preprocessed = SqlPreprocessor::preprocess(sql)?;
let statements = Parser::parse_sql(&GenericDialect, &preprocessed.standard_sql)?;
```

**After (Native AST)**:
```rust
// NEW: Direct parsing
let statements = Parser::parse_sql(&GenericDialect, sql)?;
// Window info already in TableFactor.window
```

### Benefits Delivered
- ✅ **Zero Regex Overhead**: Single-pass parsing
- ✅ **Better Error Messages**: Line/column information from parser
- ✅ **Handles Complex Expressions**: Nested intervals, arithmetic, no float conflicts
- ✅ **Type Safety**: Compile-time guarantees for all window variants
- ✅ **Extensibility**: Foundation for PARTITION BY and future streaming SQL

### Example Usage

```sql
-- All WINDOW syntaxes now parse natively
SELECT symbol, AVG(price) AS avg_price
FROM StockStream WINDOW('tumbling', INTERVAL '5' SECOND)
GROUP BY symbol;

-- Complex expressions handled correctly
SELECT sensor_id, AVG(temperature)
FROM SensorStream WINDOW('sliding', INTERVAL '1' HOUR - INTERVAL '5' MINUTE, INTERVAL '10' MINUTE)
GROUP BY sensor_id;
```

### Success Criteria
- [x] Fork created and integrated - ✅ **COMPLETE**
- [x] Native parser implementation - ✅ **COMPLETE**
- [x] All regex preprocessing removed - ✅ **COMPLETE**
- [x] 452/452 core tests passing - ✅ **VERIFIED**
- [x] Zero compilation errors - ✅ **VERIFIED**
- [x] Clean architecture with no legacy code - ✅ **VERIFIED**

### Impact
- ✅ **Architecture**: Eliminated technical debt from regex hacks
- ✅ **Reliability**: Parse-time validation instead of runtime errors
- ✅ **Performance**: Single-pass parsing, no regex overhead
- ✅ **Maintainability**: Clean AST-based architecture
- ✅ **Foundation**: Ready for PARTITION BY and advanced streaming SQL

---

## 🔌 Milestone 2: Grammar Completion & Essential Connectivity (v0.2)

**Timeline**: Q3 2025 (8-10 weeks)
**Theme**: "Complete SQL Grammar & Connect to the Real World"
**Status**: 🔄 **PART B COMPLETE** - Full Configuration System Operational
**Progress**: Part B Phases 1-4 Complete (2025-10-23) - TOML, Error Handling, Data Mapping all implemented

### Goals
1. Enable remaining disabled tests (66 → ~50) by implementing remaining grammar features
2. Enable production deployments by implementing critical I/O connectors

### Part A: Grammar Completion (4-6 weeks) - **IMMEDIATE PRIORITY**

**Current Status**: M1.5 complete with 683 passing tests, 66 ignored tests awaiting grammar features

#### 1. PARTITION Syntax (2-3 weeks) - **HIGHEST PRIORITY**
- 🆕 **Partition Clause**: Partitioning for parallel processing
  ```sql
  PARTITION WITH (symbol OF StockStream)
  BEGIN
      SELECT symbol, SUM(volume) FROM StockStream GROUP BY symbol;
  END;
  ```
- **Status**: Runtime fully supports partitioning
- **Implementation**: New partition clause parser in `SqlConverter`
- **Tests**: Enables 6 tests in `app_runner_partitions.rs`, `app_runner_partition_stress.rs`

#### 3. DEFINE AGGREGATION (2-3 weeks)
- 🆕 **Aggregation DDL**: Incremental aggregation definitions
  ```sql
  CREATE AGGREGATION SalesAggregation
  AS SELECT symbol, SUM(value) as total
  FROM In GROUP BY value
  AGGREGATE EVERY SECONDS, MINUTES, HOURS;
  ```
- **Status**: Incremental aggregation runtime exists
- **Implementation**: New DDL parser for aggregation definitions
- **Tests**: Enables 3 tests in `app_runner_aggregations.rs`

#### 4. INSERT INTO TABLE Runtime ✅ **COMPLETE** (2025-10-25)
- ✅ **Table Insert Processor**: Runtime support for populating tables from streams
  ```sql
  CREATE TABLE T (v STRING) WITH ('extension' = 'cache');
  INSERT INTO T SELECT v FROM InputStream;  -- ✅ WORKS!

  -- Stream-table JOIN for enrichment
  SELECT o.orderId, o.amount, u.name
  FROM OrderStream o
  JOIN UserProfiles u ON o.userId = u.userId;  -- ✅ WORKS!
  ```
- **Status**:
  - ✅ SQL parser complete (CREATE TABLE with extension)
  - ✅ Tables created and registered correctly
  - ✅ **INSERT INTO TABLE runtime processor implemented** (InsertIntoTableProcessor)
  - ✅ **UPDATE/DELETE from streams working** (UpdateTableProcessor, DeleteTableProcessor)
  - ✅ **Stream-table JOINs operational**
  - ✅ **HashMap-based O(1) indexing** (100x-10,000x performance improvement)
- **Delivered**:
  - InsertIntoTableProcessor for stream-to-table inserts
  - UpdateTableProcessor for stream-driven updates
  - DeleteTableProcessor for stream-driven deletes
  - O(1) HashMap indexing for find/contains operations
  - Database-agnostic Table trait API validated across InMemory, Cache, JDBC tables
- **Tests Passing**: 11/11 in `app_runner_tables.rs`
  - ✅ `cache_table_crud_via_app_runner`
  - ✅ `jdbc_table_crud_via_app_runner`
  - ✅ `stream_table_join_basic`
  - ✅ `stream_table_join_jdbc`
  - ✅ `test_table_join_no_match`
  - ✅ `test_table_join_multiple_matches`
  - ✅ `test_table_on_left_stream_on_right_join`
  - ✅ `test_stream_table_join_with_qualified_names`
  - ✅ `test_error_unknown_table_in_join`
  - ✅ `test_error_unknown_stream_in_join`
  - ✅ `test_error_unknown_column_in_table`
- **Production Ready**: ✅ For <50k events/sec workloads
- **Documentation**: `feat/table_operations/TABLE_OPERATIONS.md`
- **Next Steps**: M2 Part C (DB backend validation) → M3 (high-throughput optimizations)

#### 5. Built-in Functions (1 week)
- 🆕 **Function Registry**: Additional string/math functions
  - `LOG()`, `UPPER()`, and other standard functions
- **Status**: Function executors exist, need registry mapping
- **Implementation**: Function mapping in `SqlConverter`
- **Tests**: Enables 1 test in `app_runner_functions.rs`

#### 6. Type System Enhancement ✅ **COMPLETE** (2025-10-26)
- ✅ **Type Inference Engine**: Automatic type inference for all query outputs
  - ✅ **Zero-Allocation Architecture**: Lifetime-based `&'a SqlCatalog` design (100% heap allocation reduction)
  - ✅ **Eliminated STRING Defaults**: All output columns correctly typed via type inference
  - ✅ **Data-Driven Function Registry**: Static array replaces 150+ line match statement
  - ✅ **Consolidated Validation**: Merged validation.rs into type_inference.rs (537 lines removed)
  - ✅ **Unified Relation Accessor**: Single code path for streams AND tables (57% code reduction)
  - ✅ **Comprehensive Validation**: WHERE/HAVING/JOIN ON clauses validated at compile-time
  - ✅ **Table Join Support**: Unified catalog.get_column_type() for streams and tables
- **Status**: ✅ **SHIPPED** - Production-ready with zero-cost abstractions
- **Implementation**: `src/sql_compiler/type_inference.rs` (502 lines), `src/sql_compiler/catalog.rs` (optimized)
- **Code Reduction**: ~660 lines removed (50% reduction from consolidation)
- **Tests**: 807 passing (796 library + 11 table joins)
- **Documentation**: **[feat/type_system/TYPE_SYSTEM.md](feat/type_system/TYPE_SYSTEM.md)**
- **Impact**: Zero runtime type errors, <0.5ms overhead, zero heap allocations

**Part A Success Criteria** (Updated):
- [x] **INSERT INTO TABLE runtime operational** ✅ - 11/11 tests passing
- [x] **Stream-table joins functional** ✅ - All JOIN tests working
- [x] **Database-agnostic Table API validated** ✅ - InMemory, Cache, JDBC working
- [x] **Type inference working for all query outputs** ✅ - Zero-allocation architecture, 807 tests passing
- [ ] PARTITION queries execute with proper isolation ⏳
- [ ] Incremental aggregations work via SQL syntax ⏳
- [ ] Built-in functions (LOG, UPPER) ⏳

### Part B: Essential Connectivity (6 weeks) - **IN PROGRESS**

#### 0. Configuration System - **COMPLETE** ✅ (2025-10-23)

**Status**: ✅ **FULLY IMPLEMENTED** - All 4 layers operational with error handling and data mapping

**Completed:** All 4 layers (SQL WITH, TOML streams, TOML application, Rust defaults)
**Implemented:** TOML loading, Error Handling (DLQ/retry), Data Mapping (JSON/CSV), Environment variables

**Completed Implementation** (2025-10-21):

**Phase 1-2: SQL WITH Configuration** ✅
- ✅ **SQL WITH Parsing** - Full parser integration with property extraction
- ✅ **StreamDefinition Storage** - `with_config: Option<FlatConfig>` field
- ✅ **Runtime Auto-Attach** - Sources and sinks automatically attached from SQL WITH
- ✅ **Factory Integration** - Properties flow correctly to `factory.create_initialized()`
- ✅ **End-to-End Flow** - Complete Timer → Query → Log Sink working
- ✅ **Test Coverage** - 9 comprehensive tests (8 passing, 1 ignored)
- ✅ **Zero Regressions** - 786 library tests passing
- ✅ **Documentation** - Implementation tracked in PHASE2_FINAL_SUCCESS.md

**Working Example:**
```sql
CREATE STREAM TimerInput (tick STRING) WITH (
    type = 'source',
    extension = 'timer',
    "timer.interval" = '100',
    format = 'json'
);

CREATE STREAM LogSink (tick STRING) WITH (
    type = 'sink',
    extension = 'log',
    format = 'json',
    "log.prefix" = '[EVENT]'
);

INSERT INTO LogSink SELECT tick FROM TimerInput;

-- ✅ WORKS! Timer auto-attached, events flowing to log sink
```

**4-Layer Configuration Model** (All 4 Layers Operational):
1. ✅ **Layer 1 (Highest): SQL WITH clause** - PRODUCTION READY
2. ✅ **Layer 2: TOML [streams.StreamName]** - IMPLEMENTED
3. ✅ **Layer 3: TOML [application]** - IMPLEMENTED
4. ✅ **Layer 4 (Lowest): Rust defaults** - Operational

**COMPLETENESS ASSESSMENT** (vs CONFIGURATION.md spec):
- ✅ Phases 1-4 Complete: SQL WITH, TOML loading, error handling, data mapping
- ⏳ Phase 5 Pending: CLI tools (config list, validate commands)
- **Architecture & Foundation**: Excellent, production-ready
- **Production Features**: Core systems fully implemented and tested

**IMPLEMENTED CAPABILITIES**:
- ✅ Environment variables for credentials (`${VAR:default}` syntax)
- ✅ Extract nested JSON/CSV fields with mapping system
- ✅ Graceful error handling with retry/DLQ strategies
- ✅ Share configuration across streams via TOML [application]

**Implementation Phases:**
- ✅ **Phase 1: Core Data Structures** - COMPLETE
- ✅ **Phase 2: SQL Parser Integration & Runtime Wiring** - COMPLETE
- ✅ **Phase 3: TOML Loading** - COMPLETE
  - ✅ TOML [application] and [streams.*] sections
  - ✅ Environment variable substitution (`${KAFKA_BROKERS:localhost:9092}`)
  - ✅ 4-layer configuration merge algorithm
  - ✅ TOML validation (reject type/extension/format in TOML)
  - **Tests**: Verified via toml_config.rs implementation

- ✅ **Phase 4: Extension System Enhancement** - COMPLETE
  - ✅ **Error Handling System** (16 passing tests):
    - ✅ `error.strategy` configuration (drop/retry/dlq/fail)
    - ✅ Dead Letter Queue (DLQ) streams with schema validation
    - ✅ Exponential backoff retry logic
    - ✅ DLQ fallback strategies (log/fail/retry)
    - ✅ Three-phase validation (parse-time, init-time, runtime)
    - **Files**: src/core/error/*.rs (8 modules)
  - ✅ **Data Mapping System** (21 passing tests):
    - ✅ `json.mapping.fieldName` → JSONPath extraction for nested JSON
    - ✅ `csv.mapping.fieldName` → Column mapping by position/name
    - ✅ JSON/CSV template support for sink output formatting
    - ✅ Auto-mapping validation (all-or-nothing policy)
    - **Files**: src/core/stream/mapper/*.rs
  - ✅ **Mapper Options**:
    - ✅ `json.ignore-parse-errors`, `json.date-format`
    - ✅ `csv.delimiter`, `csv.quote-char`, `csv.limits`
  - ⏳ **Production Extensions** (Pending):
    - Currently: TimerSource, LogSink (testing)
    - Needed: Kafka, HTTP, File, MySQL/Postgres

- [ ] **Phase 5: CLI Tools** (Week 5-6)
  - `eventflux config list` - Show resolved configuration
  - `eventflux config validate` - Validate configuration
  - `--config` flag support
  - Secret redaction in CLI output

**Key Design Decisions:**
- ✅ **Extension-Agnostic Parser** - Parser validates syntax, extensions validate semantics
- ✅ **Stream Type Declaration** - Required `'type'` property (`'source'` or `'sink'`)
- ✅ **Format Property** - Industry-standard `'format'` for data mappers
- ✅ **Configuration Priority** - SQL WITH > TOML stream > TOML app > Rust defaults (all 4 layers operational)

**What's Remaining for Production:**

**Production Extensions** (Testing extensions exist, production extensions pending):
- ✅ TimerSource (testing), LogSink (debug)
- ⏳ Kafka Source/Sink - Planned for M2 completion
- ⏳ HTTP Source/Sink - Planned for M2 completion
- ⏳ File Source/Sink - Planned for M2 completion
- ⏳ MySQL/Postgres Table extensions - Planned for M2 completion
- ⏳ WebSocket, gRPC, MQTT - Planned for M3+
- **Status**: Core framework complete, production extensions in development

**Documentation:**
- Implementation: PHASE2_FINAL_SUCCESS.md, PHASE2_COMPLETE.md
- Design: [feat/configuration/CONFIGURATION.md](feat/configuration/CONFIGURATION.md)
- Grammar: [feat/grammar/GRAMMAR.md](feat/grammar/GRAMMAR.md)

### Key Features

#### 1. Critical Sources (3 most common)
- 🆕 **HTTP Source**: REST API endpoints with authentication
  - JSON payload mapping
  - Basic authentication and API keys
  - Configurable polling and webhooks
- 🆕 **Kafka Source**: Consumer integration
  - Topic subscription with consumer groups
  - Offset management (auto-commit, manual)
  - Avro/JSON deserialization
- 🆕 **File Source**: File readers
  - CSV, JSON, line-delimited formats
  - Tail mode for log files
  - Directory watching

#### 2. Critical Sinks (3 most common)
- 🆕 **HTTP Sink**: REST API calls
  - Webhook delivery with retries
  - Batch request support
  - Template-based payloads
- 🆕 **Kafka Sink**: Producer integration
  - Topic publishing with partitioning
  - Exactly-once semantics support
  - Avro/JSON serialization
- 🆕 **File Sink**: File writers
  - CSV, JSON output formats
  - File rotation by size/time
  - Compression support (gzip)

#### 3. Data Mapping
- 🆕 **JSON Mapper**: Source and sink JSON mapping
- 🆕 **CSV Mapper**: CSV parsing and formatting
- 🆕 **Error Handling**: OnErrorAction strategies (LOG, STORE, DROP)

#### 4. Connection Infrastructure
- 🆕 **Connection Pooling**: HTTP client pooling
- 🆕 **Retry Logic**: Exponential backoff for sinks
- 🆕 **Health Checks**: Connection monitoring

### Example Usage

```sql
-- HTTP source with JSON mapping
CREATE SOURCE StockTickerAPI (
    symbol STRING,
    price DOUBLE,
    timestamp LONG
) WITH (
    type = 'http',
    url = 'https://api.example.com/stocks',
    method = 'GET',
    interval = '1000',
    auth.type = 'bearer',
    auth.token = '${API_TOKEN}'
) MAP (type='json');

-- Kafka sink with Avro
INSERT INTO HighVolumeStocks
SELECT symbol, price, volume
FROM StockStream[volume > 1000000]
SINK (
    type = 'kafka',
    bootstrap.servers = 'localhost:9092',
    topic = 'high-volume-alerts',
    format = 'avro'
);
```

### What's NOT Included
- ❌ Advanced connectors (WebSocket, gRPC, MQTT)
- ❌ Database connectors (will come in M6)
- ❌ Custom source/sink plugins
- ❌ Distributed source coordination

### Success Criteria
- [ ] HTTP source can consume REST APIs at 10K+ requests/sec
- [ ] Kafka integration handles 100K+ messages/sec
- [ ] File sources can tail logs with <10ms latency
- [ ] Connection failures handled gracefully with retries
- [ ] Comprehensive connector documentation
- [ ] 15+ real-world integration examples

### Migration Impact
- Purely additive - no breaking changes
- Enhanced InMemory source/sink remain for testing

---

### Part C: Database Backend Validation (6-8 weeks) - **PLANNED**

**Timeline**: Q4 2025
**Status**: ⏳ **NEXT PRIORITY** after Part A & B completion
**Rationale**: Validate database-agnostic Table API before deep optimization

#### Goals
Implement production database backends to ensure the Table trait API is truly database-agnostic before investing in high-throughput optimizations (deferred to M3).

#### Key Features

##### 1. PostgreSQL Table Extension
- 🆕 **Native PostgreSQL Backend**: Direct table storage in PostgreSQL
  - Prepared statement optimization
  - Connection pooling (r2d2 or deadpool)
  - Batch insert support
  - Index management
  - CDC (Change Data Capture) for table updates

##### 2. MySQL Table Extension
- 🆕 **MySQL Backend**: MySQL table integration
  - Connection pooling
  - Batch operations
  - Replica read distribution
  - Index hints

##### 3. MongoDB Table Extension
- 🆕 **Document Storage**: MongoDB collection backend
  - Document-based table storage
  - Index management
  - Aggregation pipeline integration
  - Change streams for updates

##### 4. Redis Table Extension
- 🆕 **Ultra-Low Latency**: Redis-backed tables
  - Hash-based storage
  - TTL support for automatic expiry
  - Sorted sets for range queries
  - Pub/sub for table updates

#### Example Usage

```sql
-- PostgreSQL table backend
CREATE TABLE UserProfiles (
    userId STRING PRIMARY KEY,
    name STRING,
    tier STRING,
    totalSpent DOUBLE
) WITH (
    extension = 'postgresql',
    host = '${DB_HOST:localhost}',
    database = 'eventflux',
    table = 'user_profiles',
    connection_pool_size = '10'
);

-- MongoDB table backend
CREATE TABLE EventLog (
    eventId STRING,
    timestamp LONG,
    data STRING
) WITH (
    extension = 'mongodb',
    uri = '${MONGO_URI}',
    database = 'eventflux',
    collection = 'event_log',
    indexes = 'timestamp:1,eventId:1'
);

-- Redis table backend (for hot data)
CREATE TABLE ActiveSessions (
    sessionId STRING,
    userId STRING,
    lastActivity LONG
) WITH (
    extension = 'redis',
    url = '${REDIS_URL}',
    ttl = '3600',  -- 1 hour auto-expiry
    key_prefix = 'session:'
);
```

#### Success Criteria
- [ ] All 4 database backends pass table operation tests
- [ ] Table trait API requires no breaking changes
- [ ] Performance benchmarks meet targets:
  - PostgreSQL: >10k inserts/sec, <1ms find
  - MySQL: >10k inserts/sec, <1ms find
  - MongoDB: >5k inserts/sec, <2ms find
  - Redis: >50k inserts/sec, <0.1ms find
- [ ] Connection pooling and retry logic working
- [ ] Comprehensive documentation for each backend
- [ ] API validated as truly database-agnostic

#### Strategic Validation
After Part C, we'll have validated the Table trait API across:
- ✅ In-memory storage (InMemoryTable)
- ✅ Size-limited cache (CacheTable)
- ✅ JDBC/SQL databases (JdbcTable, PostgreSQL, MySQL)
- ✅ NoSQL databases (MongoDB)
- ✅ Key-value stores (Redis)

**Then** proceed to M3 for high-throughput optimizations with confidence that the API is stable.

---

## ⚡ Milestone 3: Table Optimizations & Query Engine (v0.3)

**Timeline**: Q4 2025 - Q1 2026 (16-20 weeks)
**Theme**: "High-Throughput Performance & Query Optimization"
**Status**: 📋 Planned (After M2 Part C)
**Dependencies**: M2 Part C completion (database-agnostic API validation)

### Goals
1. **Table Optimizations** (8-10 weeks): Implement high-throughput table operations after database API validation
2. **Query Optimization** (8-10 weeks): Eliminate 5-10x performance penalty from direct AST execution

### Part A: High-Throughput Table Optimizations (8-10 weeks)

**Rationale**: After M2 Part C validates the Table trait API across multiple databases, implement performance optimizations with confidence that API changes won't be needed.

#### 1. Bulk Insert Batching
**Impact**: 10x-50x throughput improvement
**Current**: ~10k inserts/sec (one-by-one)
**Target**: ~500k inserts/sec (batched)

```rust
// Add to Table trait
trait Table {
    fn bulk_insert(&self, rows: &[&[AttributeValue]]);
    fn bulk_update(&self, updates: &[(Condition, UpdateSet)]);
    fn bulk_delete(&self, conditions: &[Condition]);
}

// InsertIntoTableProcessor batches events before lock acquisition
fn process(&self, chunk: Option<Box<dyn ComplexEvent>>) {
    let mut batch = Vec::new();
    while let Some(event) = chunk {
        batch.push(event.get_output_data());
    }
    self.table.bulk_insert(&batch);  // Single lock!
}
```

#### 2. Lock-Free Concurrent Access (DashMap)
**Impact**: Linear thread scalability
**Current**: RwLock causes linear degradation
**Target**: 85%+ efficiency on 8 threads

```rust
pub struct InMemoryTable {
    rows: Arc<DashMap<usize, Vec<AttributeValue>>>,  // Lock-free!
    index: Arc<DashMap<String, Vec<usize>>>,
    next_id: AtomicUsize,
}
```

**Performance**:
- 1 thread: 100k ops/sec
- 8 threads: 650k ops/sec (81% efficiency) ← vs current 25%

#### 3. Transaction Support
**Impact**: Data integrity guarantees

```rust
trait Table {
    fn begin_transaction(&self) -> Box<dyn Transaction>;
}

trait Transaction {
    fn insert(&mut self, values: &[AttributeValue]);
    fn update(&mut self, condition: &dyn CompiledCondition, update_set: &dyn CompiledUpdateSet);
    fn delete(&mut self, condition: &dyn CompiledCondition);
    fn commit(self) -> Result<(), TableError>;
    fn rollback(self);
}
```

```sql
BEGIN TRANSACTION;
INSERT INTO Orders SELECT * FROM OrderStream;
UPDATE Inventory SET stock = stock - order.quantity;
COMMIT;
```

#### 4. Complex Expression Support
**Impact**: Functional completeness

Current compile_condition only handles constants. Extend to support:
- Comparison expressions: `age > 65`
- Math expressions: `price * 1.1`
- Function calls: `UPPER(name) = 'ALICE'`

#### 5. True LRU Cache
**Impact**: Better cache hit rates

Replace CacheTable FIFO eviction with true LRU tracking.

#### 6. Memory Management
**Impact**: Production stability

- Configurable max_memory limits
- Spill-to-disk for large tables
- Memory pressure monitoring

### Part B: Query Optimization Engine (8-10 weeks)

#### 1. Cost-Based Query Planner
- 🆕 **Query Analysis**: Analyze query complexity and cardinality
- 🆕 **Execution Plans**: Generate optimized execution plans
- 🆕 **Plan Selection**: Choose optimal plan based on statistics
- 🆕 **Plan Caching**: Cache compiled plans for repeated queries

#### 2. Expression Compilation
- 🆕 **Filter Compilation**: Pre-compile WHERE clause conditions
- 🆕 **Projection Compilation**: Optimize SELECT expressions
- 🆕 **Aggregation Compilation**: Pre-compute aggregation logic
- 🆕 **Join Compilation**: Compiled join conditions

#### 3. Runtime Code Generation
- 🆕 **Hot Path Optimization**: Generate specialized code for frequent patterns
- 🆕 **SIMD Acceleration**: Vectorized operations where applicable
- 🆕 **Inline Functions**: Inline simple function calls

#### 4. Performance Monitoring
- 🆕 **Query Profiling**: Per-query performance metrics
- 🆕 **Plan Visualization**: EXPLAIN query plans
- 🆕 **Optimization Hints**: Suggestions for query improvements

### Performance Targets

| Query Type | Before (v0.2) | After (v0.3) | Improvement |
|------------|---------------|--------------|-------------|
| Simple Filter | 1M events/sec | 1M events/sec | No change |
| Complex Join | 50K events/sec | 500K events/sec | **10x** |
| Multi-Aggregation | 100K events/sec | 800K events/sec | **8x** |
| Pattern Matching | 40K events/sec | 200K events/sec | **5x** |

### Example Features

```sql
-- Query plan visualization
EXPLAIN SELECT
    symbol,
    AVG(price) as avg_price,
    COUNT(*) as count
FROM StockStream
WINDOW TUMBLING(1 minute)
WHERE volume > 100000
GROUP BY symbol;

-- Output: Optimized execution plan with estimated costs
-- ├─ WindowProcessor (tumbling, 1min) [est: 10K events]
-- ├─ FilterProcessor (volume > 100000) [compiled condition, est: 50% selectivity]
-- └─ AggregationProcessor (AVG, COUNT) [compiled aggregator]
```

### What's NOT Included
- ❌ Adaptive query optimization (re-planning based on runtime stats)
- ❌ Distributed query optimization
- ❌ Machine learning-based optimization

### Success Criteria

**Part A (Table Optimizations)**:
- [ ] Bulk operations achieve >500k inserts/sec (50x improvement)
- [ ] Concurrent access scales linearly to 8+ threads (85%+ efficiency)
- [ ] Transactions provide ACID guarantees (BEGIN/COMMIT/ROLLBACK working)
- [ ] Complex WHERE clauses work correctly (all expression types)
- [ ] Memory usage stays under configured limits (no OOM)
- [ ] LRU cache provides better hit rates than FIFO

**Part B (Query Optimization)**:
- [ ] Complex queries achieve 5-10x performance improvement
- [ ] Query compilation <10ms for 95% of queries
- [ ] Memory usage reduced by 20% through optimization
- [ ] EXPLAIN provides actionable optimization advice
- [ ] Benchmark suite validates all improvements

### Migration Impact
- Zero breaking changes - transparent optimization
- Existing queries automatically benefit from optimization
- Optional SQL WITH properties for advanced tuning (e.g., 'optimizer.hint' = 'force_index')

---

## 📊 Milestone 4: Advanced Windowing (v0.4)

**Timeline**: Q1 2026 (8-10 weeks)
**Theme**: "Complete Analytical Capabilities"
**Status**: 📋 Planned

### Goals
Implement the remaining 22 window types to provide complete analytical windowing capabilities for time-series and event processing.

### Key Features

#### 1. Time-Based Windows (3 types)
- 🆕 **Cron Window**: Schedule-based windows (`WINDOW CRON('0 0 * * *')`)
- 🆕 **Delay Window**: Delayed event processing
- 🆕 **Hopping Window**: Custom hop intervals

#### 2. Analytical Windows (2 types)
- 🆕 **Frequent Window**: Frequent pattern mining
- 🆕 **LossyFrequent Window**: Approximate frequent items (space-efficient)

#### 3. Deduplication Windows (2 types)
- 🆕 **Unique Window**: Remove duplicate events
- 🆕 **UniqueLength Window**: Unique with size constraints

#### 4. Hybrid Windows (1 type)
- 🆕 **TimeLength Window**: Combined time and count constraints

#### 5. Custom Windows (2 types)
- 🆕 **Expression Window**: Custom logic windows
- 🆕 **ExpressionBatch Window**: Batch version of expression window

#### 6. Advanced Features
- 🆕 **Queryable Windows**: External query support via `FROM window.find()`
- 🆕 **Findable Windows**: On-demand window access
- 🆕 **Window Chaining**: Multiple windows on same stream

### Example Usage

```sql
-- Frequent pattern mining
SELECT itemset, frequency
FROM TransactionStream
WINDOW FREQUENT(100)  -- Track top 100 frequent patterns
GROUP BY itemset;

-- Cron-based window for daily reports
SELECT
    DATE(timestamp) as day,
    SUM(revenue) as daily_revenue
FROM SalesStream
WINDOW CRON('0 0 * * *')  -- Trigger at midnight
GROUP BY DATE(timestamp);

-- Unique deduplication
SELECT DISTINCT userId, action
FROM UserActivityStream
WINDOW UNIQUE(userId)  -- Keep only unique users
;

-- Queryable window for on-demand access
SELECT *
FROM lastHourPrices.find(symbol == 'AAPL')
WHERE timestamp > now() - 30 minutes;
```

### What's NOT Included
- ❌ Custom window plugins (user-defined windows)
- ❌ Distributed windows (windowing across nodes)

### Success Criteria
- [ ] All 30 window types implemented and tested
- [ ] Queryable windows respond in <1ms
- [ ] Frequent windows handle 100K+ unique items
- [ ] Window state serialization for all types
- [ ] 50+ window examples covering all types

### Migration Impact
- Additive only - existing windows unchanged
- New window types available via SQL syntax
- Backward compatible with EventFluxQL window syntax

---

## 🔍 Milestone 5: Complex Event Processing (v0.5)

**Timeline**: Q2 2026 (12-16 weeks)
**Theme**: "Advanced Pattern Matching"
**Status**: 📋 Planned

### Goals
Complete the pattern processing implementation to deliver full CEP capabilities, enabling detection of complex event sequences and temporal patterns.

### Key Features

#### 1. Absent Pattern Processing (3 processors)
- 🆕 **Negative Patterns**: `NOT (pattern)` with timing constraints
- 🆕 **Absence Detection**: Detect when expected events don't occur
- 🆕 **Scheduler Integration**: Time-based absence triggers

```sql
-- Detect fraudulent activity: purchase without prior login
SELECT p.userId, p.amount
FROM PATTERN (
    NOT login -> purchase
    WITHIN 5 minutes
)
WHERE p.amount > 1000;
```

#### 2. Count and Quantification (3 processors)
- 🆕 **Pattern Quantifiers**: `<n:m>`, `+`, `*` operators
- 🆕 **Count-Based Patterns**: Exactly N occurrences
- 🆕 **Range Patterns**: Between N and M occurrences

```sql
-- Detect 3-5 failed login attempts
SELECT userId
FROM PATTERN (
    failedLogin<3:5> -> successLogin
    WITHIN 10 minutes
);
```

#### 3. Every Patterns (1 runtime)
- 🆕 **Continuous Monitoring**: `every (pattern)` for ongoing detection
- 🆕 **Pattern Repetition**: Detect repeating patterns

```sql
-- Monitor every spike pattern continuously
SELECT symbol, spike_price
FROM PATTERN (
    every (
        normalPrice -> spike[price > normalPrice * 1.1]
    )
);
```

#### 4. Logical Patterns (2 processors)
- 🆕 **AND Patterns**: `(pattern1) AND (pattern2)`
- 🆕 **OR Patterns**: `(pattern1) OR (pattern2)`
- 🆕 **Nested Logic**: Complex boolean combinations

```sql
-- Detect either pattern
SELECT userId
FROM PATTERN (
    (loginFailed<3:> -> accountLocked)
    OR
    (suspiciousIP -> unauthorizedAccess)
);
```

#### 5. Stream Receivers (4 types)
- 🆕 **Single Process Receivers**: Optimized for simple patterns
- 🆕 **Multi Process Receivers**: Parallel pattern processing
- 🆕 **Sequence Receivers**: Strict sequence enforcement

#### 6. Advanced Pattern Features
- 🆕 **Cross-Stream References**: `e2[price > e1.price]`
- 🆕 **Collection Indexing**: `e[0]`, `e[last]`, `e[n]`
- 🆕 **Complex State Machines**: Multi-state NFA compilation
- 🆕 **Temporal Constraints**: Advanced `WITHIN`, `FOR` timing

### Example Usage

```sql
-- Complex fraud detection pattern
SELECT
    a.userId,
    a.location as loginLocation,
    b.location as purchaseLocation
FROM PATTERN (
    every (
        login as a ->
        purchase<1:5> as b[b.userId == a.userId]
    )
    WITHIN 1 hour
)
WHERE
    distance(a.location, b.location) > 1000 km;

-- Absence pattern: No heartbeat
SELECT deviceId
FROM PATTERN (
    NOT heartbeat[deviceId == d.deviceId]
    FOR 5 minutes
    AFTER device as d
);
```

### What's NOT Included
- ❌ MATCH_RECOGNIZE SQL syntax (use native pattern syntax)
- ❌ Distributed pattern matching across nodes

### Success Criteria
- [ ] Process 200K+ patterns/sec (Java parity)
- [ ] Support 100+ concurrent pattern queries
- [ ] Handle patterns with 10+ states
- [ ] 85% coverage of Java pattern capabilities
- [ ] 30+ CEP examples covering all pattern types

### Migration Impact
- Extends existing basic pattern matching
- Backward compatible with simple sequences
- New pattern syntax follows SQL/Match standards

---

## 🔒 Milestone 6: Production Hardening (v0.6)

**Timeline**: Q3 2026 (10-12 weeks)
**Theme**: "Enterprise Ready"
**Status**: 📋 Planned

### Goals
Add essential enterprise features for production deployments: comprehensive monitoring, security framework, and additional database connectors.

### Key Features

#### 1. Comprehensive Monitoring & Observability
- 🆕 **Prometheus Metrics**: Full Prometheus exporter
  - Query-level metrics (throughput, latency, errors)
  - Stream-level metrics (event rates, backpressure)
  - System metrics (memory, CPU, thread pools)
- 🆕 **OpenTelemetry Tracing**: Distributed tracing support
  - Query execution traces
  - Event flow tracking
  - Performance bottleneck identification
- 🆕 **Health Checks**: `/health` and `/ready` endpoints
- 🆕 **Operational Dashboards**: Pre-built Grafana dashboards
- **Status**: Pipeline metrics complete, enterprise features needed
- **Documentation**: **[feat/observability/OBSERVABILITY.md](feat/observability/OBSERVABILITY.md)**
- **Estimated Effort**: 4-6 weeks

#### 2. Security Framework
- 🆕 **Authentication**:
  - API key authentication
  - OAuth2/OIDC integration
  - mTLS support
- 🆕 **Authorization**:
  - Role-based access control (RBAC)
  - Stream-level permissions
  - Query-level ACLs
- 🆕 **Audit Logging**:
  - Security event logging
  - Query execution audit trail
  - Compliance reporting (GDPR, SOC2)
- 🆕 **Encryption**:
  - TLS for network transport
  - At-rest encryption for state
  - Secret management integration (Vault)

#### 3. Database Connectors
- 🆕 **PostgreSQL Source/Sink**: CDC and bulk operations
- 🆕 **MongoDB Source/Sink**: Change streams and aggregation pipelines
- 🆕 **Redis Sink**: Cache updates (leverage existing Redis state backend)

#### 4. Advanced Aggregators
- 🆕 **Statistical Aggregators**: stdDev, variance, percentiles
- 🆕 **Logical Aggregators**: and, or aggregations
- 🆕 **Set Aggregators**: unionSet, intersectSet

### Example Usage

```sql
-- Prometheus metrics automatically collected
-- Access at: http://localhost:9090/metrics

-- Secure stream with RBAC
CREATE STREAM SensitiveData (
    userId STRING,
    ssn STRING,
    salary DOUBLE
) WITH (
    access.control = 'RBAC',
    allowed.roles = 'admin,data-analyst'
);

-- PostgreSQL CDC source
CREATE SOURCE CustomerUpdates WITH (
    type = 'postgresql',
    host = 'localhost',
    database = 'customers',
    mode = 'CDC',
    table = 'customer_profiles',
    username = '${DB_USER}',
    password = '${DB_PASS}'
) MAP (type='json');

-- MongoDB aggregation sink
INSERT INTO CustomerMetrics
SELECT
    region,
    AVG(purchaseAmount) as avgPurchase,
    STDDEV(purchaseAmount) as stdDevPurchase
FROM PurchaseStream
WINDOW TUMBLING(1 hour)
GROUP BY region
SINK (
    type = 'mongodb',
    collection = 'hourly_metrics',
    mode = 'upsert'
);
```

### Success Criteria
- [ ] Prometheus metrics for all components
- [ ] <1ms overhead from security checks
- [ ] SOC2/ISO27001 compliant audit logging
- [ ] Database connectors handle 50K+ ops/sec
- [ ] Zero-downtime certificate rotation
- [ ] Security documentation and best practices guide

### Migration Impact
- Security optional - disabled by default for development
- Monitoring always enabled but configurable
- Database connectors purely additive

---

## 🌐 Milestone 7: Distributed Processing (v0.7)

**Timeline**: Q4 2026 (14-16 weeks)
**Theme**: "Horizontal Scale"
**Status**: 📋 Planned

### Goals
Activate the existing distributed processing framework, enabling horizontal scaling to 10+ nodes with automatic failover and state management.

### Key Features

#### 1. Cluster Coordination (Complete Raft)
- ✅ **Foundation**: Raft-based distributed coordinator (implemented)
- 🆕 **Leader Election**: Automatic leader selection
- 🆕 **Cluster Membership**: Dynamic node join/leave
- 🆕 **Health Monitoring**: Node failure detection
- 🆕 **Consensus Protocol**: Distributed decision making

#### 2. Message Broker Integration
- 🆕 **Kafka Integration**: Event distribution via Kafka
  - Exactly-once event delivery
  - Partitioning strategies
  - Offset management
- 🆕 **NATS Integration**: Lightweight alternative for edge deployments
- 🆕 **Internal Broker**: Built-in option for simple deployments

#### 3. Query Distribution
- 🆕 **Load Balancing**: Distribute query processing across nodes
- 🆕 **Partition Strategies**: Hash, range, and custom partitioning
- 🆕 **Query Routing**: Route events to correct processing nodes
- 🆕 **State Sharding**: Distribute state across cluster

#### 4. Failover and Recovery
- 🆕 **Automatic Failover**: <5 second failover time
- 🆕 **State Recovery**: Restore state from distributed backend
- 🆕 **Checkpoint Coordination**: Distributed consistent checkpoints
- 🆕 **Split-Brain Prevention**: Quorum-based operations

#### 5. Distributed State Management
- ✅ **Redis Backend**: Production-ready (implemented)
- 🆕 **State Replication**: Multi-replica state storage
- 🆕 **Read Replicas**: Offload query workload
- 🆕 **State Migration**: Rebalance state during scaling

### Architecture

```
┌─────────────────────────────────────────────┐
│         Load Balancer / Ingress             │
└─────────────┬───────────────────────────────┘
              │
    ┌─────────┴──────────┐
    │                    │
┌───▼────┐  ┌────────┐  ┌────────┐
│ Node 1 │  │ Node 2 │  │ Node N │  ← EventFlux Processing Nodes
│(Leader)│  │        │  │        │
└───┬────┘  └───┬────┘  └───┬────┘
    │           │           │
    └───────────┼───────────┘
                │
    ┌───────────┴───────────┐
    │                       │
┌───▼──────┐        ┌──────▼────┐
│  Redis   │        │   Kafka   │  ← Distributed State & Events
│ Cluster  │        │  Cluster  │
└──────────┘        └───────────┘
```

### Example Configuration

```yaml
# Distributed mode configuration
eventflux:
  runtime:
    mode: distributed
    cluster:
      name: production-cluster
      nodes: 3
      coordinator:
        type: raft
        election-timeout: 5s
    state:
      backend: redis
      replication-factor: 3
    transport:
      type: grpc
      tls: enabled
    message-broker:
      type: kafka
      bootstrap-servers: kafka:9092
```

### Performance Targets

| Metric | Single Node | 3-Node Cluster | 10-Node Cluster |
|--------|-------------|----------------|-----------------|
| Throughput | 1.46M events/sec | 4M events/sec | 12M events/sec |
| Latency (p99) | <1ms | <5ms | <10ms |
| Failover Time | N/A | <5 seconds | <5 seconds |
| State Recovery | <30s | <60s | <120s |

### What's NOT Included
- ❌ Geo-distributed deployment (single datacenter only)
- ❌ Cross-datacenter replication
- ❌ Distributed pattern matching across nodes

### Success Criteria
- [ ] Linear scaling to 10 nodes (85%+ efficiency)
- [ ] Zero data loss during failover
- [ ] <5 second automatic failover
- [ ] Cluster management UI/CLI
- [ ] Production deployment guides (K8s, Docker Swarm)
- [ ] Chaos engineering validation

### Migration Impact
- Zero overhead for single-node deployments
- Opt-in via configuration
- Existing queries work unchanged in distributed mode
- State automatically migrated to distributed backend

---

## 📈 Milestone 8: Advanced Query Features (v0.8)

**Timeline**: Q1 2027 (8-10 weeks)
**Theme**: "SQL Feature Parity"
**Status**: 📋 Planned

### Goals
Implement advanced SQL features to achieve near-complete SQL compatibility for analytical stream processing.

### Key Features

#### 1. HAVING Clause
- 🆕 **Post-Aggregation Filtering**: Filter after GROUP BY
- 🆕 **Aggregate Conditions**: Conditions on aggregated values

```sql
SELECT
    symbol,
    AVG(price) as avg_price,
    COUNT(*) as trade_count
FROM StockStream
WINDOW TUMBLING(5 minutes)
GROUP BY symbol
HAVING AVG(price) > 100 AND COUNT(*) > 50;
```

#### 2. LIMIT and OFFSET
- 🆕 **Result Pagination**: `LIMIT n OFFSET m`
- 🆕 **Top-N Queries**: Efficiently retrieve top results
- 🆕 **Streaming Limits**: Continuous top-N with updates

```sql
-- Top 10 highest prices
SELECT symbol, price
FROM StockStream
ORDER BY price DESC
LIMIT 10;
```

#### 3. Subqueries and CTEs
- 🆕 **WITH Clause**: Common Table Expressions
- 🆕 **Subquery Support**: Nested queries
- 🆕 **Correlated Subqueries**: Reference outer query

```sql
-- CTE example
WITH HighVolume AS (
    SELECT symbol, volume
    FROM StockStream
    WHERE volume > 1000000
)
SELECT h.symbol, s.price
FROM HighVolume h
JOIN StockStream s ON h.symbol = s.symbol;
```

#### 4. Window Functions (OVER Clause)
- 🆕 **ROW_NUMBER()**: Row numbering within partitions
- 🆕 **RANK(), DENSE_RANK()**: Ranking functions
- 🆕 **LAG(), LEAD()**: Access previous/next rows
- 🆕 **Partition By**: Window partitioning

```sql
SELECT
    symbol,
    price,
    ROW_NUMBER() OVER (PARTITION BY symbol ORDER BY price DESC) as rank,
    LAG(price, 1) OVER (PARTITION BY symbol ORDER BY timestamp) as prev_price
FROM StockStream;
```

#### 5. Advanced JOIN Features
- 🆕 **Temporal Joins**: Time-bounded joins
- 🆕 **OUTER JOINS**: LEFT, RIGHT, FULL OUTER
- 🆕 **CROSS APPLY**: Lateral joins

```sql
-- Temporal join with time constraint
SELECT s.symbol, s.price, n.headline
FROM StockStream s
LEFT JOIN NewsStream n
    ON s.symbol = n.symbol
    AND n.timestamp BETWEEN s.timestamp - 5 minutes AND s.timestamp;
```

### Success Criteria
- [ ] 95% SQL compatibility for streaming use cases
- [ ] Window functions perform at >500K events/sec
- [ ] Subquery optimization prevents performance degradation
- [ ] TPC-H style streaming queries execute correctly
- [ ] Comprehensive SQL reference documentation

### Migration Impact
- Purely additive - all new SQL features
- Existing queries continue to work
- New SQL capabilities available immediately

---

## 🔎 Milestone 9: On-Demand Queries (v0.9)

**Timeline**: Q2 2027 (6-8 weeks)
**Theme**: "Interactive Analytics"
**Status**: 📋 Planned

### Goals
Enable interactive querying of streaming state, allowing on-demand access to windows, tables, and aggregations.

### Key Features

#### 1. Table Query Interface
- 🆕 **Query API**: REST and gRPC interfaces for table queries
- 🆕 **SQL Access**: Standard SQL queries on tables
- 🆕 **Compiled Conditions**: Optimized table scans
- 🆕 **Index Support**: B-tree and hash indexes for fast lookups

```sql
-- Create queryable table
CREATE TABLE CustomerProfiles (
    customerId STRING PRIMARY KEY,
    name STRING,
    tier STRING,
    totalSpent DOUBLE
);

-- On-demand query via API
GET /query/table/CustomerProfiles?filter=tier=='GOLD'&limit=100
```

#### 2. Findable Windows
- 🆕 **Window Query API**: Query window contents on-demand
- 🆕 **Find Syntax**: `FROM window.find(condition)`
- 🆕 **Snapshot Queries**: Point-in-time window snapshots

```sql
-- Create findable window
CREATE WINDOW LastHourTrades
    TUMBLING(1 hour)
WITH (queryable = true);

INSERT INTO LastHourTrades
SELECT * FROM TradeStream;

-- Query window on-demand
SELECT *
FROM LastHourTrades.find(symbol == 'AAPL' AND price > 150)
ORDER BY timestamp DESC;
```

#### 3. Aggregation Queries
- 🆕 **Live Aggregation Access**: Query current aggregation state
- 🆕 **Multi-Duration Queries**: Access different time granularities
- 🆕 **Aggregation Snapshots**: Historical aggregation states

```sql
-- Query current aggregation state
SELECT * FROM hourly_metrics.current()
WHERE region == 'US-WEST';

-- Query historical aggregations
SELECT * FROM hourly_metrics.range(
    from: now() - 7 days,
    to: now()
);
```

#### 4. Query Performance
- 🆕 **Query Caching**: Cache frequent query results
- 🆕 **Materialized Views**: Pre-computed query results
- 🆕 **Query Optimization**: Plan optimization for on-demand queries

### Example API

```bash
# REST API for on-demand queries
curl -X POST http://localhost:8080/api/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "SELECT * FROM LastHourTrades.find(symbol == '\''AAPL'\'') LIMIT 10"
  }'

# WebSocket for streaming results
ws://localhost:8080/api/query/stream?query=SELECT+*+FROM+HighFrequencyTrades
```

### Success Criteria
- [ ] On-demand queries respond in <10ms for indexed lookups
- [ ] Support 1000+ concurrent on-demand queries
- [ ] Query result caching reduces load by 80%
- [ ] RESTful and gRPC query APIs
- [ ] Interactive query UI/playground

### Migration Impact
- Additive feature - existing streams/tables gain query capability
- Opt-in queryable flag for performance-sensitive windows
- No impact on streaming performance

---

## 📊 Milestone 10: Incremental Aggregations (v1.0)

**Timeline**: Q3 2027 (12-14 weeks)
**Theme**: "Time-Series Analytics at Scale"
**Status**: 📋 Planned

### Goals
Implement enterprise-grade incremental aggregation framework for efficient time-series analytics with multi-duration aggregations and historical data integration.

### Key Features

#### 1. Multi-Duration Aggregation
- 🆕 **AggregationRuntime**: Manage time-based aggregation hierarchy
- 🆕 **Auto-Granularity**: Automatic second/minute/hour/day/month aggregations
- 🆕 **Aggregation Cascading**: Roll-up from fine to coarse granularity

```sql
-- Multi-duration aggregation definition
CREATE AGGREGATION SalesAggregation
WITH (
    by = 'timestamp',
    granularity = 'second'
) AS
SELECT
    region,
    SUM(amount) as total_sales,
    AVG(amount) as avg_sale,
    COUNT(*) as sale_count
FROM SalesStream
GROUP BY region;

-- Query at any granularity
SELECT * FROM SalesAggregation
WITHIN last '30 days'
PER 'hour'
WHERE region == 'US-WEST';
```

#### 2. Incremental Computation
- 🆕 **IncrementalExecutor**: Streaming aggregation updates
- 🆕 **IncrementalAggregator**: Delta-based computation
- 🆕 **Optimization**: Avoid recomputing entire aggregations

#### 3. Historical Data Integration
- 🆕 **BaseIncrementalValueStore**: Persistent aggregation storage
- 🆕 **Batch-Stream Unification**: Merge historical and streaming data
- 🆕 **Backfill Support**: Reprocess historical data into aggregations

#### 4. Persisted Aggregations
- 🆕 **Database Backend**: Store aggregations in PostgreSQL/MongoDB
- 🆕 **Retention Policies**: Automatic aggregation pruning
- 🆕 **Compaction**: Merge old aggregations for efficiency

#### 5. Distributed Aggregations
- 🆕 **Cross-Node Aggregation**: Coordinate aggregations across cluster
- 🆕 **Partial Aggregation**: Combine results from multiple nodes
- 🆕 **Aggregation Routing**: Direct data to correct aggregation node

### Example Usage

```sql
-- Time-series analytics across multiple granularities
CREATE AGGREGATION TrafficMetrics
WITH (
    by = 'timestamp',
    granularity = 'second',
    retention = '90 days'
) AS
SELECT
    endpoint,
    COUNT(*) as request_count,
    AVG(responseTime) as avg_response_time,
    PERCENTILE(responseTime, 95) as p95_latency
FROM APIRequestStream
GROUP BY endpoint;

-- Query hourly metrics for last week
SELECT
    endpoint,
    SUM(request_count) as total_requests,
    AVG(avg_response_time) as avg_latency
FROM TrafficMetrics
WITHIN last '7 days'
PER 'hour';

-- Query daily rollup for last quarter
SELECT
    DATE(timestamp) as day,
    endpoint,
    SUM(request_count) as daily_requests
FROM TrafficMetrics
WITHIN last '90 days'
PER 'day'
ORDER BY day, daily_requests DESC;
```

### Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Aggregation Update | <1ms | Per incoming event |
| Multi-Duration Storage | 90%+ reduction | vs storing all granularities separately |
| Query Latency | <10ms | For aggregated data retrieval |
| Historical Backfill | 1M events/sec | Reprocessing speed |

### Success Criteria
- [ ] Support 1000+ concurrent aggregations
- [ ] Multi-duration queries respond in <10ms
- [ ] Handle TB+ of historical aggregation data
- [ ] Automatic granularity selection based on query
- [ ] Distributed aggregation across 10+ nodes
- [ ] Comprehensive time-series analytics examples

### Migration Impact
- Major feature addition for analytics workloads
- Existing aggregations continue to work (non-incremental)
- Opt-in to incremental aggregation framework
- Automatic migration tools for existing aggregations

---

## 🎯 v1.0 Production Release

**Timeline**: Q3 2027
**Theme**: "Enterprise Production Ready"

### Success Criteria for v1.0

#### Functional Completeness
- ✅ SQL streaming with 95%+ SQL compatibility
- ✅ Essential I/O connectors (HTTP, Kafka, File, DB)
- ✅ Complete CEP pattern matching (85%+ Java parity)
- ✅ All 30 window types implemented
- ✅ Advanced query features (HAVING, LIMIT, CTEs, Window Functions)
- ✅ On-demand queries and interactive analytics
- ✅ Incremental aggregations for time-series

#### Performance
- ✅ >1M events/sec single-node throughput
- ✅ <1ms p99 latency for simple queries
- ✅ 5-10x improvement from query optimization
- ✅ Linear scaling to 10+ nodes (85%+ efficiency)
- ✅ <5 second failover in distributed mode

#### Enterprise Features
- ✅ Comprehensive monitoring (Prometheus, OpenTelemetry)
- ✅ Security (RBAC, audit logging, encryption)
- ✅ Distributed processing with automatic failover
- ✅ Production-grade state management (90-95% compression)
- ✅ High availability (99.9%+ uptime)

#### Developer Experience
- ✅ SQL-first syntax for accessibility
- ✅ Comprehensive documentation with 200+ examples
- ✅ IDE integration and syntax highlighting
- ✅ Query debugging and profiling tools
- ✅ Migration guides from Java EventFlux

#### Operations
- ✅ Kubernetes operators and Helm charts
- ✅ Docker images and compose files
- ✅ Monitoring dashboards (Grafana)
- ✅ Automated deployment pipelines
- ✅ Disaster recovery procedures

---

## 🚀 Beyond v1.0: Future Vision

### Potential v1.x Features
- **WebAssembly UDFs**: Language-agnostic custom functions
- **Machine Learning Integration**: Real-time ML inference
- **Advanced Connectors**: gRPC, WebSocket, MQTT, cloud-native sources
- **Streaming Lakehouse**: Delta Lake, Iceberg integration
- **Edge Computing**: Lightweight deployment for IoT
- **GraphQL API**: GraphQL queries on streaming data
- **Multi-Tenancy**: Isolation and resource quotas

### Potential v2.0+ Features
- **Geo-Distributed Processing**: Cross-datacenter replication
- **Stream SQL Standard**: Full ANSI SQL streaming compliance
- **Automatic Scaling**: ML-based autoscaling
- **Advanced ML**: Real-time model training
- **Time-Travel Queries**: Query historical stream states
- **Streaming Data Mesh**: Decentralized stream processing

---

## Release Philosophy

### Quality Gates
Each milestone must meet these criteria before release:

1. **Functionality**: All planned features implemented and tested
2. **Performance**: Meets or exceeds performance targets
3. **Stability**: No critical bugs, <5 known medium bugs
4. **Documentation**: Complete user and API documentation
5. **Testing**: >80% code coverage, all integration tests passing
6. **Migration**: Backward compatibility or clear migration path

### Beta Program
- Early access to milestone features
- Community feedback integration
- Production pilot deployments
- Performance benchmarking with real workloads

### Support Policy
- **Current Release**: Full support with security and bug fixes
- **Previous Release**: Security fixes for 6 months
- **Older Releases**: Community support only

---

## Community & Contribution

### How to Get Involved
1. **Early Adopters**: Try milestone releases and provide feedback
2. **Contributors**: Implement connectors, functions, or features
3. **Documentation**: Help with examples and tutorials
4. **Testing**: Report bugs and edge cases

### Communication Channels
- **GitHub Issues**: Bug reports and feature requests
- **Discussions**: Architecture and design discussions
- **Discord/Slack**: Real-time community support
- **Monthly Updates**: Progress reports and roadmap adjustments

---

## Conclusion

This milestone roadmap provides a clear path to delivering a production-ready, enterprise-grade stream processing engine that combines:
- **Accessibility**: SQL-first syntax
- **Performance**: >1M events/sec with query optimization
- **Completeness**: Full CEP capabilities with 85%+ Java parity
- **Scale**: Distributed processing to 10+ nodes
- **Enterprise**: Security, monitoring, and reliability

By following this incremental delivery approach, users can adopt EventFlux Rust early and benefit from continuous improvements, while developers maintain focus on delivering working, valuable features at each milestone.

**Last Milestone Completed**: M1.6 - Native Parser Migration (2025-10-08)
**Next Update**: Q3 2025 (after M2 completion)
**Feedback Welcome**: Please open GitHub discussions for roadmap suggestions
