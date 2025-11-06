# EventFlux Rust Implementation Milestones

Last Updated: 2025-11-05
Current Status: M2 Pattern Processing Phase 2 Complete
Test Status: 1,436 passing tests (271 pattern processing tests)

---

## Milestone 1: SQL Streaming Foundation (v0.1) - Complete

Timeline: Q2 2025 (8-10 weeks)
Status: Complete (2025-10-06)

Features Delivered:
- SQL parser with EventFluxDialect (sqlparser-rs fork)
- CREATE STREAM, SELECT, INSERT, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT/OFFSET
- Window operations (TUMBLING, SLIDING, length, session)
- JOIN support (INNER, LEFT, RIGHT, FULL OUTER)
- Aggregations (COUNT, SUM, AVG, MIN, MAX)
- High-performance event pipeline (>1M events/sec)
- SQL-only mode (no EventFluxQL)

Tests: 452 passing

---

## Milestone 2: Pattern Processing (v0.2) - Phases 1-2 Complete

Timeline: Q3 2025 (10 weeks)
Status: Phase 1-2 complete, Phases 3-4 planned for M5+
Priority: Core > API > Grammar > Documentation

### Phase 1: Foundation - Complete

Status: Committed to main (commit 8acac2f)
Tests: 195 passing

Components:
- StateEvent with multi-stream tracking
- StateEventCloner with 'every' support
- PreStateProcessor trait + StreamPreStateProcessor
- PostStateProcessor trait + StreamPostStateProcessor
- LogicalPreStateProcessor + LogicalPostStateProcessor
- PatternStreamReceiver + SequenceStreamReceiver
- Lock-free ProcessorSharedState (deadlock resolution)

### Phase 2: Count Quantifiers - Complete (2025-11-05)

Duration: 2 days implementation + cleanup
Tests: 76 passing (52 single patterns + 24 pattern chaining)

**Phase 2a: Single Patterns** - Complete (2025-11-04)
- CountPreStateProcessor for single patterns (A{3}, A{2,5})
- CountPostStateProcessor for validation
- Event chaining (add_event, remove_last_event)
- Min/max count tracking
- 52 tests passing

**Phase 2b: Pattern Chaining** - Complete (2025-11-05)
- PatternChainBuilder factory for multi-processor chains
- Multi-processor wiring (PreA → PostA → PreB → PostB → PreC)
- Validation rules (first min>=1, last exact, all min>=1, no optional steps)
- Pattern chaining with count quantifiers
- WITHIN time constraints in chains (reactive validation)
- Pattern vs Sequence modes in chains
- Multi-instance pattern matching
- 24 tests passing + 1 ignored (Phase 3 proactive expiry)

Sub-phases:
- Phase 2b.1: Basic two-step chains (7 tests)
- Phase 2b.2: Three-step chains (5 tests)
- Phase 2b.3: Pattern mode (4 tests)
- Phase 2b.4: WITHIN support (3 tests + 1 ignored)
- Phase 2b.5: Integration testing (5 tests)

**Code Quality**:
- Cleanup: Deleted 782 lines of obsolete code
- Test refactoring: Eliminated 470+ lines of duplicate code
- Common test utilities: Created shared module (257 lines)
- Full test suite: 1,436 tests passing

**Remaining Work**:
- Query parser migration from deprecated processors (8-12 hours, blocked)
- Delete deprecated logical_processor.rs and sequence_processor.rs

**Deferred to M3 Grammar**:
- A+, A* syntax support (core logic complete)
- Grammar parser integration with CountPreStateProcessor

### Phase 3: Absent Patterns - Not Started

Time: 4 weeks
Tests: 70

Features:
- Scheduler trait + TimerWheel integration
- AbsentStreamPreStateProcessor
- NOT(A) FOR duration syntax
- Time-based absence triggers
- Event arrival cancellation

### Phase 4: 'every' & Advanced - Not Started

Time: 3 weeks
Tests: 50

Features:
- Complete 'every' pattern support
- nextEveryStatePreProcessor chaining
- Cross-stream references (e2[price > e1.price])
- Collection indexing (e[0], e[last], e[n])

---

## Milestone 3: Grammar Completion (v0.3) - Planned

Timeline: Q4 2025 (6-8 weeks)
Status: Not started
Dependencies: M2 complete

Features:
- Pattern syntax integration (map StateElement to processors)
- PARTITION clause parsing
- DEFINE AGGREGATION syntax
- Built-in functions (LOG, UPPER, etc.)
- Complete SQL grammar for all runtime features

Grammar maps to completed APIs:
- Pattern/Sequence processors → pattern syntax
- Count processors → A{n}, A{m,n}, A+, A* syntax
- Absent processors → NOT(A) FOR duration syntax
- Every processors → every(...) syntax

Tests: ~24 currently disabled tests will be enabled

---

## Milestone 4: Essential Connectivity (v0.4) - Planned

Timeline: Q1 2026 (6-8 weeks)
Status: Not started

Configuration System (Complete):
- SQL WITH clause parsing
- TOML configuration (4-layer model)
- Error handling (DLQ, retry, exponential backoff)
- Data mapping (JSON, CSV)
- Environment variable substitution

Sources (Not Started):
- HTTP source (REST API, webhooks)
- Kafka source (consumer groups, offset management)
- File source (CSV, JSON, tail mode)

Sinks (Not Started):
- HTTP sink (webhooks, batch requests)
- Kafka sink (exactly-once, partitioning)
- File sink (rotation, compression)

---

## Milestone 5: Database Backend Validation (v0.5) - Planned

Timeline: Q2 2026 (6-8 weeks)
Status: Not started
Dependencies: Table API validated, M3 complete

Features:
- PostgreSQL table extension (CDC, connection pooling)
- MySQL table extension (replica reads)
- MongoDB table extension (change streams)
- Redis table extension (TTL, sorted sets)

Goal: Validate Table trait API across multiple backends before M6 optimizations

---

## Milestone 6: Table Optimizations (v0.6) - Planned

Timeline: Q3 2026 (8-10 weeks)
Status: Not started
Dependencies: M5 complete (API validated)

Features:
- Bulk insert batching (target: 500k inserts/sec)
- Lock-free concurrent access (DashMap)
- Transaction support (BEGIN/COMMIT/ROLLBACK)
- Complex expression support in conditions
- True LRU cache
- Memory management (limits, spill-to-disk)

---

## Milestone 7: Query Optimization (v0.7) - Planned

Timeline: Q4 2026 (8-10 weeks)
Status: Not started

Features:
- Cost-based query planner
- Expression compilation (WHERE, projections, aggregations)
- Runtime code generation
- Hot path optimization
- Performance monitoring (EXPLAIN, profiling)

Target: 5-10x performance improvement for complex queries

---

## Milestone 8: Advanced Windowing (v0.8) - Planned

Timeline: Q1 2027 (8-10 weeks)
Status: Not started

Features:
- Cron window (schedule-based)
- Delay window
- Hopping window
- Frequent window (pattern mining)
- Unique window (deduplication)
- Queryable windows (on-demand access)

Total window types: 30+

---

## Milestone 9: Production Hardening (v0.9) - Planned

Timeline: Q2 2027 (10-12 weeks)
Status: Not started

Features:
- Prometheus metrics exporter
- OpenTelemetry tracing
- Security framework (RBAC, audit logging, encryption)
- Database connectors (PostgreSQL, MongoDB, Redis sources/sinks)
- Health checks and monitoring dashboards

---

## Milestone 10: Distributed Processing (v1.0) - Planned

Timeline: Q3 2027 (14-16 weeks)
Status: Not started

Features:
- Complete Raft cluster coordination
- Kafka message broker integration
- Query distribution and load balancing
- Automatic failover (<5 seconds)
- Distributed state management
- State replication

Target: Linear scaling to 10+ nodes (85% efficiency)

---

## Milestone Ordering Rationale

Core > API > Grammar > Documentation

M1: SQL foundation (complete)
M2: Pattern processing APIs (Phases 2-4) - core CEP functionality
M3: Grammar for pattern syntax - sugar coating the APIs
M4: Connectivity - extensions using the complete APIs
M5+: Validation, optimization, production features

Cannot write grammar for features that don't exist yet.
Cannot validate APIs without implementing them first.

---

## Release Philosophy

Quality Gates:
1. All planned features implemented
2. Performance targets met
3. No critical bugs
4. Documentation complete
5. >80% code coverage
6. Backward compatibility or migration path

Support Policy:
- Current release: Full support
- Previous release: Security fixes for 6 months
- Older releases: Community support only
