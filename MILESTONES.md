# EventFlux Rust Implementation Milestones

Last Updated: 2025-12-04
Current Status: M2 Pattern Processing Phase 2 Complete
Test Status: 1,436+ passing tests (370+ pattern processing tests)

---

## Project Focus: Lightweight CEP

**What "lightweight" means:**
- 50-100MB single binary (vs 4GB+ JVM heap)
- Millisecond startup (vs 30+ second JVM warmup)
- Runs on $50/month VPS (vs Kubernetes cluster)
- Zero external dependencies for core operation

**Target scale:** 100k+ events/sec on single node
**Target users:** Teams that don't need Flink's complexity

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

**Phase 2c: Array Access Runtime** - Complete (2025-11-27)
- IndexedVariableExecutor for e[0], e[last], e[n]
- 14+ tests passing

**Phase 2d: Cross-Stream References** - Complete (2025-11-23)
- Condition function receives StateEvent
- Filter can access previous events in pattern
- 6 tests passing

**Phase 2e: EVERY Multi-Instance** - Complete (2025-11-26)
- Overlapping pattern instances
- Sliding window with count quantifiers
- 10 tests passing

**Phase 2f: Collection Aggregations** - Complete (2025-12-04)
- CollectionAggregationFunction trait
- Executors: count, sum, avg, min, max, stdDev
- Registry integration in EventFluxContext
- 50+ tests passing

**Remaining Work**:
- Parser for pattern grammar (runtime complete)

**Not Implemented**:
- A+, A* syntax (unbounded patterns rejected by design)
- PARTITION BY (multi-tenant isolation)
- Absent patterns (NOT ... FOR)
- Event-count WITHIN

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

## Milestone 3: CASE Expression & Developer Experience (v0.3) - NEXT

Timeline: Q4 2025 (4-6 weeks)
Status: Not started
Dependencies: M2 Phase 2 complete
Priority: Core SQL feature + first production users

### CASE Expression (1 week)
- AST node (CaseExpression, WhenClause)
- CaseExpressionExecutor
- Expression parser integration
- Searched CASE (WHEN condition THEN)
- Simple CASE (CASE expr WHEN value THEN)
- Tests: ~20

Documentation: feat/case_expression/CASE_EXPRESSION.md

### Developer Experience (2-3 weeks)
- Docker image that "just works"
- 3-5 complete example projects with tutorials
- Excellent error messages with suggestions
- Quick start guide (5 minutes to first query)
- Example gallery on GitHub

### Production Basics (1 week)
- Health check endpoints
- Graceful shutdown
- Basic logging configuration
- Startup time optimization

Total Tests: ~40
Goal: Get first production user

---

## Milestone 4: Essential Connectors (v0.4) - Planned

Timeline: Q1 2026 (6-8 weeks)
Status: Not started
Dependencies: M3 complete

### Kafka Connector (3 weeks) - CRITICAL
- Kafka source (consumer groups, offset management)
- Kafka sink (partitioning, exactly-once semantics)
- Configuration via SQL WITH
- Tests: ~30

### HTTP Connector (2 weeks)
- HTTP source (REST API, webhooks)
- HTTP sink (webhooks, batch requests)
- Retry with exponential backoff
- Tests: ~20

### File Connector (1 week)
- File source (CSV, JSON, tail mode)
- File sink (rotation, compression)
- Tests: ~15

### Observability (1 week)
- Prometheus metrics endpoint
- Basic dashboard templates
- Latency/throughput metrics

Total Tests: ~65
Goal: Production-viable connectivity

---

## Milestone 5: Grammar & Built-in Functions (v0.5) - Planned

Timeline: Q2 2026 (4-6 weeks)
Status: Not started
Dependencies: M4 complete

Features:
- Pattern syntax integration (map StateElement to processors)
- PARTITION clause parsing
- DEFINE AGGREGATION syntax
- Built-in functions (LOG, UPPER, LOWER, CONCAT, etc.)
- Complete SQL grammar for all runtime features

Grammar maps to completed APIs:
- Pattern/Sequence processors → pattern syntax
- Count processors → A{n}, A{m,n}, A+, A* syntax

Tests: ~24 currently disabled tests will be enabled

---

## Milestone 6: Production Hardening (v0.6) - Planned

Timeline: Q3 2026 (6-8 weeks)
Status: Not started

Features:
- OpenTelemetry tracing integration
- Structured logging (JSON format)
- Performance profiling tools
- Memory usage monitoring
- Crash recovery improvements
- Security basics (input validation, rate limiting)

---

## Milestone 7: Database Backends (v0.7) - Planned

Timeline: Q4 2026 (8-10 weeks)
Status: Not started

Features:
- PostgreSQL table extension (connection pooling)
- MySQL table extension
- MongoDB table extension (change streams)
- Better Redis integration (TTL, pub/sub)

Goal: Validate Table trait API across multiple backends

---

## Future Milestones (v0.8+) - Deferred

These features are deferred indefinitely. They will be considered based on user demand:

### Advanced Features (Deferred)
- Distributed processing (Raft, multi-node)
- Query optimization engine
- Advanced windowing (cron, hopping, frequent)
- struct() type and field access
- AI/LLM integration (ai_decide, action handlers)

### Rationale for Deferral
- Distributed mode adds complexity without clear demand
- AI integration needs validated use case
- Focus on single-node excellence first

---

## Milestone Ordering Rationale

**New Priority**: Ship → Users → Iterate

M1: SQL foundation (complete)
M2: Pattern processing (complete)
M3: CASE + Developer experience → Get first user
M4: Essential connectors → Production viable
M5: Grammar completion → Full feature set
M6: Production hardening → Reliable operation
M7: Database backends → Ecosystem growth

**Philosophy Change**: Stop building features, start shipping product.

The previous plan optimized for feature completeness (AI integration, distributed processing). The new plan optimizes for adoption:
1. Make it easy to try (M3 developer experience)
2. Make it useful (M4 connectors)
3. Make it production-ready (M5-M6)
4. Grow based on user feedback

---

## Technical Debt & Architecture

Documented architectural decisions and technical debt for future consideration:

| Document | Status | Description |
|----------|--------|-------------|
| **[Extension Registry](feat/extension_registry/EXTENSION_REGISTRY_REQUIREMENT.md)** | ✅ Implemented | Centralized registry for all extension types (windows, aggregators, functions, sources, sinks). Manual registration for WASM compatibility. |
| **[Unified Aggregation Logic](feat/unified_aggregation/UNIFIED_AGGREGATION_DESIGN.md)** | 📋 Proposed | Deduplicate aggregation logic between window and collection aggregators. Implement when adding new aggregation types. |

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
