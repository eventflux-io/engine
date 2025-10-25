# EventFlux Rust Features Documentation

This directory contains detailed documentation for specific features and architectural components of EventFlux Rust.

## Directory Structure

### 📂 [distributed/](distributed/)
Distributed processing framework, cluster coordination, and deployment configurations.

**Documentation**: [DISTRIBUTED.md](distributed/DISTRIBUTED.md)

**Topics Covered**:
- Complete distributed processing architecture
- Runtime mode abstraction (SingleNode/Distributed/Hybrid)
- Transport layer (TCP and gRPC implementations)
- Redis state backend with production-ready features
- Configuration management (YAML, Kubernetes, Docker)
- Deployment guides and troubleshooting

**Related Code**: `src/core/distributed/`

**Status**: Foundation Complete, Extensions In Progress

---

### 📂 [state_management/](state_management/)
Enterprise-grade state management, checkpointing, compression, and persistence.

**Documentation**: [STATE_MANAGEMENT.md](state_management/STATE_MANAGEMENT.md)

**Topics Covered**:
- Comprehensive state management architecture
- StateHolder trait and implementation patterns
- Incremental checkpointing system
- Compression utilities (90-95% compression ratios)
- Point-in-time recovery
- Distributed state coordination

**Related Code**: `src/core/persistence/`

**Status**: Production Complete

---

### 📂 [async_streams/](async_streams/)
High-performance asynchronous stream processing with @Async annotations.

**Documentation**: [ASYNC_STREAMS.md](async_streams/ASYNC_STREAMS.md)

**Topics Covered**:
- Lock-free crossbeam-based event pipeline
- @Async annotation usage patterns
- Backpressure strategies (Drop, Block, ExponentialBackoff)
- Performance characteristics (>1M events/sec)
- Real-world examples (financial, IoT, logs)
- Troubleshooting and best practices

**Related Code**: `src/core/stream/`, `src/core/util/pipeline/`

**Status**: Production Ready

---

### 📂 [grammar/](grammar/)
Parser architecture, SQL integration, and query language design.

**Documentation**: [GRAMMAR.md](grammar/GRAMMAR.md)

**Topics Covered**:
- Parser technology evaluation (LALRPOP, sqlparser-rs, Tree-sitter, Pest)
- Hybrid parser strategy (SQL-first approach)
- Current LALRPOP implementation
- Future SQL compatibility plans
- MATCH_RECOGNIZE implementation strategy
- Grammar design principles

**Related Code**: `src/query_compiler/`

**Status**: Hybrid Parser Strategy Planned

---

### 📂 [error_handling/](error_handling/)
Error handling framework using thiserror and comprehensive error hierarchy.

**Documentation**: [ERROR_HANDLING.md](error_handling/ERROR_HANDLING.md)

**Topics Covered**:
- Hierarchical error types (StateError, QueryError, RuntimeError, IOError, ConfigError)
- Error propagation and context patterns
- Migration guide from String errors
- Best practices and testing strategies
- Error coverage metrics (121+ Result types)

**Related Code**: `src/core/error/`

**Status**: Production Ready

---

### 📂 [stream_table_relations/](stream_table_relations/)
Unified stream and table handling in SQL queries with late binding architecture.

**Documentation**: [STREAM_TABLE_RELATIONS.md](stream_table_relations/STREAM_TABLE_RELATIONS.md)

**Topics Covered**:
- Relation enum for unified stream/table catalog
- Four-layer architecture (Catalog, Converter, Query API, Runtime)
- Late binding with runtime type detection
- Stream-table JOIN implementation (cache and JDBC)
- SQL WITH clause for table configuration
- Known limitations and workarounds
- Comprehensive test coverage (9 passing tests)

**Related Code**: `src/sql_compiler/catalog.rs`, `src/core/util/parser/query_parser.rs`, `tests/app_runner_tables.rs`

**Status**: Core Functionality Working - Basic JOINs operational

---

### 📂 [table_operations/](table_operations/)
Production-ready table operations with INSERT, UPDATE, DELETE, and O(1) indexing.

**Documentation**: [TABLE_OPERATIONS.md](table_operations/TABLE_OPERATIONS.md)

**Topics Covered**:
- Critical bug fixes (StreamDefinition auto-creation)
- HashMap-based O(1) indexing (100x-10,000x performance improvement)
- Database-agnostic Table trait API design
- INSERT INTO TABLE runtime implementation
- Stream-table JOIN optimizations
- Strategic roadmap (M2 Part C: DB backends → M3: high-throughput optimizations)
- Performance benchmarks and production readiness assessment
- Deferred optimizations (bulk batching, DashMap, transactions)

**Related Code**: `src/core/table/`, `src/core/query/output/insert_into_table_processor.rs`, `tests/app_runner_tables.rs`

**Status**: ✅ **M2 Part A Complete** - Production Ready for <50k events/sec

**Key Achievements** (2025-10-25):
- Fixed root cause bug preventing INSERT INTO TABLE
- Added HashMap indexing for O(1) find/contains operations
- All 11 table tests passing
- Stream-table enrichment JOINs working
- API validated across InMemory, Cache, JDBC table types

**Next Steps**:
- M2 Part C: PostgreSQL, MySQL, MongoDB, Redis table backends
- M3: Bulk operations, DashMap, transactions (after API validation)

---

### 📂 [implementation/](implementation/)
Developer guides for implementing new features and components.

**Documentation**: [IMPLEMENTATION.md](implementation/IMPLEMENTATION.md)

**Topics Covered**:
- Window processors and stream functions
- Aggregator executors
- Sources and sinks
- Table implementations
- Java-to-Rust translation patterns
- Factory registration and testing strategies
- Performance optimization techniques

**Related Code**: All `src/` directories

**Status**: Complete - Comprehensive Reference

---

## Navigation

### 🏠 Core Project Documentation (Root Directory)
- `../README.md` - Project overview and getting started
- `../CLAUDE.md` - AI assistant context and development guidelines
- `../ROADMAP.md` - Comprehensive technical roadmap (dev-focused)
- `../MILESTONES.md` - User-facing release milestones and product evolution

### 🧑‍💻 For Developers
Start with:
1. `../CLAUDE.md` - Understand project architecture and conventions
2. [implementation/IMPLEMENTATION.md](implementation/IMPLEMENTATION.md) - Learn implementation patterns
3. `../ROADMAP.md` - See current priorities and tasks

### 👥 For Users
Start with:
1. `../README.md` - Get started with EventFlux Rust
2. `../MILESTONES.md` - Understand upcoming features and releases
3. Feature-specific guides in this directory

---

## Documentation Philosophy

### Single-File Approach
Each feature directory now contains a **single comprehensive document** that consolidates all relevant information:
- Eliminates duplicate, outdated, or conflicting content
- Provides a clear, authoritative reference for each feature
- Maintains latest decisions and best approaches
- Easier to maintain and keep up-to-date

### ROADMAP.md vs MILESTONES.md
- **ROADMAP.md** (root): Developer-focused, comprehensive task tracking, technical details, all gaps documented
- **MILESTONES.md** (root): User-facing, release planning, product evolution story, what's shipping when

### Feature Documentation Standards
Each consolidated feature document includes:
- **Overview**: Current status and key features
- **Architecture**: Design and implementation details
- **Implementation**: Code examples and patterns
- **Configuration**: Setup and deployment guides
- **Examples**: Practical usage examples
- **Troubleshooting**: Common issues and solutions
- **Status**: Current implementation progress

---

## Contributing Documentation

When adding new features:
1. Create a subdirectory under `feat/` if it's a major new component
2. Create a single comprehensive document (e.g., `FEATURE_NAME.md`)
3. Include all relevant information in one place
4. Update this README with links to new documentation
5. Keep ROADMAP.md and MILESTONES.md in sync

---

Last Updated: 2025-10-25
