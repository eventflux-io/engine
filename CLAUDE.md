# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Quick Reference

**📋 For implementation priorities and what to work on next:**
→ See **[ROADMAP.md](ROADMAP.md)** - Primary source of truth for strategic priorities and disabled tests

**🎯 For milestone planning and release timeline:**
→ See **[MILESTONES.md](MILESTONES.md)** - Release roadmap with completed and upcoming features

**📖 For user-facing information:**
→ See **[README.md](README.md)** - Project overview, usage guide, and getting started

**📊 For current implementation status:**
→ See **[ROADMAP.md#current-status](ROADMAP.md#📊-comprehensive-audit-results-current-status-vs-java-eventflux)** - Real-time status vs Java EventFlux

## ⚠️ CRITICAL: Code Safety and Refactoring Rules

**NEVER use `sed`, `awk`, or bulk deletion commands for refactoring. EVER.**

### Mandatory Refactoring Protocol:

1. **ALWAYS create backups before any bulk operation:**
   ```bash
   # Copy files to .bak before ANY bulk changes
   cp file.rs file.rs.bak
   ```

2. **NEVER use sed/awk for code refactoring:**
   - ❌ `sed -i '14,212d' file.rs` - FORBIDDEN
   - ❌ `sed 's/old/new/g'` for multi-line code - FORBIDDEN
   - ✅ Use Edit tool for targeted, verified changes
   - ✅ Manual editing only for complex refactoring

3. **For removing duplicate code:**
   - Create shared module FIRST
   - Update imports in ONE file
   - Verify compilation
   - Repeat for next file
   - **NEVER batch process multiple files**

4. **Ask user permission before deleting code:**
   - Even with backups
   - Explain what will be deleted
   - Wait for explicit approval
   - Only delete after everything verified working

5. **Verification after ANY file modification:**
   ```bash
   # Always verify immediately after changes
   cargo build
   cargo test --test <affected_test>
   ```

### Why These Rules Exist:

These rules were created after a catastrophic incident where sed commands deleted 850+ lines of working test code across 5 files, requiring hours of manual restoration. The damage included loss of potentially critical test validation logic that "passing tests" cannot verify.

**Remember: Passing tests don't prove correctness. Lost assertions and edge case checks may never be detected.**

## Project Overview

EventFlux Rust is an experimental port of the Java-based EventFlux CEP (Complex Event Processing) engine to Rust. The project aims to create an **enterprise-grade distributed CEP engine** with superior performance characteristics leveraging Rust's memory safety and concurrency features.

**Current Status**: M1.6 Complete - Native SQL parser with streaming extensions, 452 core tests passing.

**Architecture**: SQL-first CEP engine with high-performance crossbeam pipeline (>1M events/sec), enterprise state management, and distributed processing foundation.

## Java Reference Implementation

### Overview

The original Siddhi Java implementation is available locally at `references/siddhi/` for easier comparison and reference during feature porting. This directory is git-ignored and serves as a local-only reference.

**Location**: `references/siddhi/modules/siddhi-core/src/main/java/io/siddhi/`

### Naming Convention: Siddhi → EventFlux

**CRITICAL**: All "Siddhi" terminology is replaced with "EventFlux" in this Rust codebase.

| Java (Siddhi) | Rust (EventFlux) |
|---------------|------------------|
| `SiddhiManager` | `EventFluxManager` |
| `SiddhiApp` | `EventFluxApp` |
| `SiddhiAppRuntime` | `EventFluxAppRuntime` |
| `SiddhiQL` | `EventFluxQL` |
| `io.siddhi.core.*` | `eventflux::core::*` |
| `io.siddhi.query.api.*` | `eventflux::query_api::*` |

**Examples**:
- Java: `SiddhiManager siddhiManager = new SiddhiManager();`
- Rust: `let manager = EventFluxManager::new();`

### Finding Java Reference Implementations

When implementing a new feature or porting functionality from Java:

#### 1. **Locate the Java Source**

```bash
# Search for class definitions
find references/siddhi -name "*.java" | grep -i "WindowProcessor"

# Search for specific functionality
grep -r "lengthBatch" references/siddhi/modules/siddhi-core/src/main/java/
```

#### 2. **Common Java Package Mappings**

| Java Package | Rust Module | Purpose |
|-------------|-------------|---------|
| `io.siddhi.core.executor` | `src/core/executor/` | Expression executors |
| `io.siddhi.core.query.processor.stream.window` | `src/core/query/processor/stream/window/` | Window processors |
| `io.siddhi.query.api.definition` | `src/query_api/definition/` | Query definitions |
| `io.siddhi.core.table` | `src/core/table/` | Table implementations |
| `io.siddhi.core.stream` | `src/core/stream/` | Stream handling |
| `io.siddhi.core.aggregation` | `src/core/aggregation/` | Aggregation functions |
| `io.siddhi.core.partition` | `src/core/partition/` | Partitioning logic |
| `io.siddhi.core.util` | `src/core/util/` | Utility functions |

#### 3. **Example: Implementing a Window Processor**

**Step 1: Find the Java implementation**
```bash
cat references/siddhi/modules/siddhi-core/src/main/java/io/siddhi/core/query/processor/stream/window/LengthWindowProcessor.java
```

**Step 2: Understand the Java logic**
- Study the `process()` method for event handling
- Review state management and cleanup logic
- Note any special edge cases or optimizations

**Step 3: Translate to Rust with EventFlux naming**
```rust
// Create in src/core/query/processor/stream/window/length_window_processor.rs
pub struct LengthWindowProcessor {
    meta: CommonProcessorMeta,
    length: usize,
    buffer: VecDeque<Arc<StreamEvent>>,
}

impl WindowProcessor for LengthWindowProcessor {
    // Port Java logic here with Rust idioms
}
```

#### 4. **Key Files to Reference**

**Core Architecture**:
- `references/siddhi/modules/siddhi-core/src/main/java/io/siddhi/core/SiddhiManager.java`
  → Rust: `src/core/eventflux_manager.rs`
- `references/siddhi/modules/siddhi-core/src/main/java/io/siddhi/core/SiddhiAppRuntime.java`
  → Rust: `src/core/eventflux_app_runtime.rs`

**Query Processing**:
- `references/siddhi/modules/siddhi-core/src/main/java/io/siddhi/core/query/QueryRuntime.java`
  → Rust: `src/core/query/query_runtime.rs`

**Event Processing**:
- `references/siddhi/modules/siddhi-core/src/main/java/io/siddhi/core/event/Event.java`
  → Rust: `src/core/event/event.rs`

### Best Practices for Java Reference Usage

1. **Study, Don't Copy**: Understand the Java logic, then implement idiomatic Rust
2. **Improve Performance**: Leverage Rust's zero-cost abstractions and ownership model
3. **Modern Patterns**: Use current best practices, not legacy Java patterns
4. **Test Coverage**: Port Java tests and add Rust-specific edge case coverage
5. **Documentation**: Document differences and improvements over Java implementation

## Build & Development Commands

```bash
# Build the project
cargo build

# Run all tests
cargo test

# Run tests with output visible
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Build and run the CLI with a EventFlux query file
cargo run --bin run_eventflux <file.eventflux>

# Build release version
cargo build --release

# Check for compilation errors without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy

# Run benchmarks (when available)
cargo bench
```

## Architecture Overview

### Module Structure

1. **sql_compiler** - SQL parser with EventFlux streaming extensions
   - Custom EventFluxDialect for sqlparser-rs
   - Native WINDOW clause parsing
   - Located in `src/sql_compiler/`

2. **query_api** - AST and query language structures
   - Defines all EventFlux query language constructs
   - Located in `src/query_api/`

3. **core** - Runtime execution engine
   - Event processing, expression execution, state management
   - Located in `src/core/`

### Key Components

**High-Performance Event Pipeline** ✅
- Lock-free crossbeam ArrayQueue
- Pre-allocated object pools for zero-allocation processing
- Configurable backpressure strategies
- >1M events/second capability

**Enterprise State Management** ✅
- Enhanced StateHolder architecture with 90-95% compression
- Incremental checkpointing with WAL
- Point-in-time recovery with parallel engine
- Redis state backend integration

**Distributed Processing Foundation** ✅
- Runtime mode abstraction (Single/Distributed/Hybrid)
- TCP and gRPC transport layers
- Extension points for coordination and brokers

**For detailed architecture and implementation status, see [ROADMAP.md](ROADMAP.md)**

## Development Guidelines

### 🎯 **IMPORTANT: Task Prioritization**

**When determining next tasks or implementation priorities, ALWAYS consult these documents in order:**

1. **[ROADMAP.md](ROADMAP.md) - Main Roadmap & Grammar Status** (PRIMARY SOURCE)
   - **Grammar/Parser Status & Disabled Tests** section - CRITICAL for grammar work
   - Current priority levels and critical blockers
   - Detailed implementation status for each component
   - Strategic decisions needed for extension implementations
   - Timeline and success criteria for all initiatives

2. **[MILESTONES.md](MILESTONES.md) - Release Milestones**
   - M1: SQL Streaming Foundation (✅ COMPLETE)
   - M2: Grammar Completion (Part A) + Essential Connectivity (Part B) - NEXT
   - M3+: Future milestones with detailed timelines

**Do not guess or assume priorities - these documents are the single source of truth for what needs to be done next.**

#### **For Grammar/Parser Work Specifically**

**ALWAYS check `ROADMAP.md` → "Grammar/Parser Status & Disabled Tests" section first!**

This section tracks:
- ✅ M1 features fully implemented (what works now)
- 📋 66 disabled tests categorized by priority
- 🔴 Priority 1: High business value features (10 tests) - TARGET M2
- 🟠 Priority 2: CEP capabilities (10 tests) - TARGET M3-M4
- 🟡 Priority 3: Advanced features (7 tests) - TARGET M5+

### New Engine Approach

- **No Backward Compatibility**: Design optimal solutions without legacy constraints
- **Clean Architecture**: Build modern, efficient implementations from scratch
- **Best Practices**: Follow current industry standards and Rust idioms
- **Performance First**: Prioritize performance and memory efficiency over compatibility

### Git Commit Guidelines

- **Never mention Claude or AI assistance** in commit messages
- **No co-authored tags**: Do not include "Co-authored-by: Claude" or similar
- **Focus on technical changes**: Describe what was implemented, not how
- **Standard format**: Use conventional commit format
- **Examples**:
  - `feat: implement Redis state backend with ThreadBarrier coordination`
  - `fix: resolve window syntax parsing conflicts with float literals`
  - `perf: optimize event pipeline with lock-free crossbeam queues`

### Performance-First Approach

1. **Benchmark Everything**: Create benchmarks before optimization
2. **Profile First**: Use `perf`, `flamegraph` before optimizing
3. **Memory Efficiency**: Prefer stack allocation, minimize heap usage
4. **Lock-Free When Possible**: Use crossbeam, avoid mutex in hot paths

### Adding Core Components

#### High-Performance Component Template
```rust
pub struct HighPerfProcessor {
    // Use lock-free structures
    ring_buffer: crossbeam::queue::ArrayQueue<Event>,
    // Pre-allocate memory
    event_pool: ObjectPool<Event>,
    // Atomic counters for metrics
    processed: AtomicU64,
}
```

#### Distributed Component Template
```rust
pub struct DistributedProcessor {
    // Cluster state
    cluster: Arc<ClusterCoordinator>,
    // Node identity
    node_id: NodeId,
    // Distributed state
    state: Arc<DistributedState>,
}
```

### Testing Strategy

**Unit Tests**: Test individual components
```rust
#[test]
fn test_ring_buffer_throughput() {
    // Benchmark >1M events/second
}
```

**Integration Tests**: Use `AppRunner` helper
```rust
let runner = AppRunner::new(app, "Out");
runner.send("In", vec![...]);
assert_eq!(runner.shutdown(), expected);
```

**Distributed Tests**: Test cluster scenarios
```rust
#[test]
fn test_failover() {
    let cluster = TestCluster::new(3);
    cluster.kill_node(0);
    assert!(cluster.is_healthy());
}
```

## Common Development Tasks

### Adding a Window Processor

```rust
// 1. Create in src/core/query/processor/stream/window/
pub struct MyWindowProcessor {
    meta: CommonProcessorMeta,
    // Use efficient data structures
    buffer: VecDeque<Arc<StreamEvent>>,
}

// 2. Implement WindowProcessor trait
impl WindowProcessor for MyWindowProcessor {}

// 3. Add to factory in mod.rs
"myWindow" => Ok(Arc::new(Mutex::new(
    MyWindowProcessor::from_handler(handler, app_ctx, query_ctx)?
))),

// 4. Create comprehensive tests
#[test]
fn test_my_window() {
    // Test functionality and performance
}
```

### Performance Optimization Checklist

**Before Optimizing:**
- [ ] Profile with flamegraph
- [ ] Check allocation patterns with heaptrack
- [ ] Benchmark against Java equivalent
- [ ] Verify memory usage under load

**Optimization Targets:**
- Lock-free implementation (no contention)
- Zero-allocation hot path (pre-allocated pools)
- Memory usage validation under sustained load

## Debugging & Profiling

```bash
# CPU profiling
cargo build --release
perf record --call-graph=dwarf target/release/run_eventflux query.eventflux
perf report

# Memory profiling
valgrind --tool=massif target/release/run_eventflux query.eventflux
ms_print massif.out.*

# Flamegraph generation
cargo flamegraph --bin run_eventflux -- query.eventflux

# Lock contention analysis
perf lock record target/release/run_eventflux query.eventflux
perf lock report
```

## Standard Implementation Instructions

### Implementation Protocol

When implementing new features or components:

1. **Testing Protocol**:
   ```bash
   # Clean build and run full test suite
   cargo clean && cargo test
   ```

2. **Documentation Requirements**:
   - Document every implementation step in relevant MD files
   - Update progress in implementation-specific documentation
   - Maintain detailed implementation logs for tracking

3. **Industry Standards**:
   - Follow best practices and performance optimizations
   - Implement comprehensive error handling
   - Add extensive test coverage for all components

4. **Code Quality**:
   ```bash
   # Format and lint before testing
   cargo fmt
   cargo clippy

   # Run with full output for debugging
   cargo test -- --nocapture

   # Test specific components
   cargo test test_name
   ```

5. **Implementation Flow**:
   - Phase-based implementation following design documents
   - Mark todos as in_progress when starting tasks
   - Complete todos immediately after finishing tasks
   - Run full test suite after each major component

## CRITICAL: New Engine Development Philosophy

**This is a new streaming engine, not a migration:**

- No backward compatibility constraints - design optimal solutions
- Focus on modern best practices and performance
- Clean architecture without legacy baggage
- Never mention AI assistance or Claude in commits
- Build the best possible CEP engine using Rust's advantages
- Prioritize performance, safety, and developer experience

**Implementation Approach:**

- Design from first principles using Rust idioms
- Leverage zero-cost abstractions and compile-time guarantees
- Build enterprise-grade distributed systems capabilities
- Focus on >1M events/sec performance targets
- Maintain comprehensive test coverage and documentation

## Documentation Navigation

- **[README.md](README.md)** - User-facing guide with setup, usage, examples
- **[ROADMAP.md](ROADMAP.md)** - Strategic priorities, implementation status, disabled tests
- **[MILESTONES.md](MILESTONES.md)** - Release timeline, completed and upcoming features
- **[IMPLEMENTATION_GUIDE.md](IMPLEMENTATION_GUIDE.md)** - Developer patterns for implementing features
- **[ERROR_HANDLING_SUMMARY.md](ERROR_HANDLING_SUMMARY.md)** - Error system documentation
- **[ASYNC_STREAMS_GUIDE.md](ASYNC_STREAMS_GUIDE.md)** - Async stream processing guide

---

Last Updated: 2025-10-09
