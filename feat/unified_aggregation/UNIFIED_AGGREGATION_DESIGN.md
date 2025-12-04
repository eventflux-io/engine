# Unified Aggregation Logic Design

**Status**: PROPOSED (Technical Debt)
**Priority**: P2 (Architectural Improvement)
**Date**: 2025-11-29

## Problem Statement

EventFlux currently has **two separate aggregation systems** that duplicate core aggregation logic:

### 1. Window Aggregators (`AttributeAggregatorExecutor`)

Location: `src/core/query/selector/attribute/aggregator/`

```rust
pub trait AttributeAggregatorExecutor: ExpressionExecutor {
    fn process_add(&mut self, data: &[&[AttributeValue]], event: &StreamEvent);
    fn process_remove(&mut self, data: &[&[AttributeValue]], event: &StreamEvent);
    fn process_add_to_aggregator(&mut self, data: &[&[AttributeValue]], events: &[&StreamEvent]);
    fn process_remove_from_aggregator(&mut self, data: &[&[AttributeValue]], events: &[&StreamEvent]);
}
```

Used for: `SELECT sum(price) FROM Stream WINDOW length(10)`

### 2. Collection Aggregators (`CollectionAggregationFunction`)

Location: `src/core/extension/mod.rs`

```rust
pub trait CollectionAggregationFunction: Debug + Send + Sync {
    fn aggregate(&self, values: &[f64]) -> Option<f64>;
    fn supports_count_only(&self) -> bool;
    fn return_type(&self, input_type: ApiAttributeType) -> ApiAttributeType;
}
```

Used for: `SELECT sum(e1.price) FROM PATTERN (e1=Event{3,5} -> ...)`

### The Duplication

Both systems implement the same mathematical operations:

| Function | Window Aggregator | Collection Aggregator |
|----------|-------------------|----------------------|
| sum | `SumAttributeAggregatorExecutor` | `CollectionSumFunction` |
| avg | `AvgAttributeAggregatorExecutor` | `CollectionAvgFunction` |
| count | `CountAttributeAggregatorExecutor` | `CollectionCountFunction` |
| min | `MinAttributeAggregatorExecutor` | `CollectionMinFunction` |
| max | `MaxAttributeAggregatorExecutor` | `CollectionMaxFunction` |
| stdDev | (not implemented) | `CollectionStdDevFunction` |

**Core issue**: The aggregation *logic* (how to compute sum, avg, etc.) is defined twice.

## Why This Matters

1. **DRY Violation**: Same math implemented in multiple places
2. **Maintenance Burden**: Bug fixes or optimizations must be applied twice
3. **Inconsistency Risk**: Implementations could diverge
4. **Extension Overhead**: New aggregators (median, percentile) must be implemented twice
5. **Testing Duplication**: Same logic tested in different test suites

## Root Cause Analysis

The window aggregator trait is **over-coupled** to its execution context:

```rust
// Current: Trait knows too much about execution details
pub trait AttributeAggregatorExecutor: ExpressionExecutor {
    fn init(
        &mut self,
        attribute_expression_executors: Vec<Box<dyn ExpressionExecutor>>,
        processing_mode: ProcessingMode,
        output_expecting_expired_events: bool,
        query_ctx: &EventFluxQueryContext,
    ) -> Result<(), String>;

    fn process_add(&mut self, data: &[&[AttributeValue]], event: &StreamEvent);
    // ...
}
```

This conflates three concerns:
1. **Aggregation Logic**: How to compute sum/avg/etc.
2. **Value Extraction**: How to get values from events
3. **State Management**: Incremental add/remove for streaming

## Proposed Solution: Separation of Concerns

### Layer 1: Pure Aggregation Logic

```rust
/// Pure aggregation logic - NO execution context knowledge
/// Location: src/core/aggregation/logic.rs
pub trait AggregationLogic: Debug + Clone + Send + Sync {
    /// Unique name for this aggregation
    fn name(&self) -> &'static str;

    /// Core computation: aggregate a slice of values
    fn aggregate(&self, values: &[f64]) -> Option<f64>;

    /// Determine return type based on input type
    fn return_type(&self, input_type: AttributeType) -> AttributeType;

    /// Whether this aggregation can work without values (e.g., count)
    fn supports_empty_args(&self) -> bool { false }

    /// Description for documentation
    fn description(&self) -> &str { "" }

    /// Clone for registry storage
    fn clone_box(&self) -> Box<dyn AggregationLogic>;
}
```

### Layer 2: Implementations (ONE per aggregation type)

```rust
/// Location: src/core/aggregation/builtin.rs

#[derive(Debug, Clone)]
pub struct SumLogic;

impl AggregationLogic for SumLogic {
    fn name(&self) -> &'static str { "sum" }

    fn aggregate(&self, values: &[f64]) -> Option<f64> {
        if values.is_empty() {
            None
        } else {
            Some(values.iter().sum())
        }
    }

    fn return_type(&self, input_type: AttributeType) -> AttributeType {
        match input_type {
            AttributeType::INT | AttributeType::LONG => AttributeType::LONG,
            _ => AttributeType::DOUBLE,
        }
    }

    fn clone_box(&self) -> Box<dyn AggregationLogic> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct AvgLogic;

impl AggregationLogic for AvgLogic {
    fn name(&self) -> &'static str { "avg" }

    fn aggregate(&self, values: &[f64]) -> Option<f64> {
        if values.is_empty() {
            None
        } else {
            Some(values.iter().sum::<f64>() / values.len() as f64)
        }
    }

    fn return_type(&self, _: AttributeType) -> AttributeType {
        AttributeType::DOUBLE
    }

    fn clone_box(&self) -> Box<dyn AggregationLogic> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct CountLogic;

impl AggregationLogic for CountLogic {
    fn name(&self) -> &'static str { "count" }

    fn aggregate(&self, values: &[f64]) -> Option<f64> {
        Some(values.len() as f64)
    }

    fn supports_empty_args(&self) -> bool { true }

    fn return_type(&self, _: AttributeType) -> AttributeType {
        AttributeType::LONG
    }

    fn clone_box(&self) -> Box<dyn AggregationLogic> {
        Box::new(self.clone())
    }
}

// MinLogic, MaxLogic, StdDevLogic, MedianLogic, PercentileLogic, etc.
```

### Layer 3: Window Aggregator Executor (Uses Logic)

```rust
/// Location: src/core/query/selector/attribute/aggregator/executor.rs

pub struct WindowAggregatorExecutor {
    /// The aggregation logic (shared, not duplicated)
    logic: Box<dyn AggregationLogic>,

    /// Expression to extract value from events
    value_executor: Option<Box<dyn ExpressionExecutor>>,

    /// Buffer for collected values (for remove support)
    values: Vec<f64>,

    /// Optional incremental state for optimized aggregators
    incremental_state: Option<Box<dyn IncrementalState>>,
}

impl WindowAggregatorExecutor {
    pub fn new(logic: Box<dyn AggregationLogic>) -> Self {
        Self {
            logic,
            value_executor: None,
            values: Vec::new(),
            incremental_state: None,
        }
    }

    pub fn with_incremental(mut self, state: Box<dyn IncrementalState>) -> Self {
        self.incremental_state = Some(state);
        self
    }
}

impl AttributeAggregatorExecutor for WindowAggregatorExecutor {
    fn process_add(&mut self, data: &[&[AttributeValue]], event: &StreamEvent) {
        let value = self.extract_value(data, event);

        if let Some(ref mut state) = self.incremental_state {
            state.add(value);
        } else {
            self.values.push(value);
        }
    }

    fn process_remove(&mut self, data: &[&[AttributeValue]], event: &StreamEvent) {
        let value = self.extract_value(data, event);

        if let Some(ref mut state) = self.incremental_state {
            state.remove(value);
        } else {
            // Remove from values buffer
            if let Some(pos) = self.values.iter().position(|&v| v == value) {
                self.values.remove(pos);
            }
        }
    }

    fn get_result(&self) -> AttributeValue {
        let result = if let Some(ref state) = self.incremental_state {
            state.get_result()
        } else {
            self.logic.aggregate(&self.values)
        };

        result.map(AttributeValue::Double).unwrap_or(AttributeValue::Null)
    }
}
```

### Layer 4: Optional Incremental Optimization

```rust
/// For aggregators that benefit from O(1) incremental updates
/// Location: src/core/aggregation/incremental.rs

pub trait IncrementalState: Debug + Send + Sync {
    fn add(&mut self, value: f64);
    fn remove(&mut self, value: f64);
    fn get_result(&self) -> Option<f64>;
    fn reset(&mut self);
}

/// O(1) incremental sum
#[derive(Debug, Default)]
pub struct IncrementalSumState {
    sum: f64,
    count: usize,
}

impl IncrementalState for IncrementalSumState {
    fn add(&mut self, value: f64) {
        self.sum += value;
        self.count += 1;
    }

    fn remove(&mut self, value: f64) {
        self.sum -= value;
        self.count -= 1;
    }

    fn get_result(&self) -> Option<f64> {
        if self.count == 0 { None } else { Some(self.sum) }
    }

    fn reset(&mut self) {
        self.sum = 0.0;
        self.count = 0;
    }
}

/// O(1) incremental average
#[derive(Debug, Default)]
pub struct IncrementalAvgState {
    sum: f64,
    count: usize,
}

impl IncrementalState for IncrementalAvgState {
    fn add(&mut self, value: f64) {
        self.sum += value;
        self.count += 1;
    }

    fn remove(&mut self, value: f64) {
        self.sum -= value;
        self.count = self.count.saturating_sub(1);
    }

    fn get_result(&self) -> Option<f64> {
        if self.count == 0 { None } else { Some(self.sum / self.count as f64) }
    }

    fn reset(&mut self) {
        self.sum = 0.0;
        self.count = 0;
    }
}
```

### Layer 5: Collection Aggregator Executor (Uses Same Logic)

```rust
/// Location: src/core/executor/collection_aggregator.rs

pub struct CollectionAggregatorExecutor {
    /// The aggregation logic (SAME as window uses)
    logic: Box<dyn AggregationLogic>,

    /// Index of the pattern chain to aggregate over
    chain_index: usize,

    /// Attribute position within events [stream_index, attr_index]
    attr_position: Option<[i32; 2]>,
}

impl CollectionAggregatorExecutor {
    pub fn new(
        logic: Box<dyn AggregationLogic>,
        chain_index: usize,
        attr_position: Option<[i32; 2]>,
    ) -> Self {
        Self { logic, chain_index, attr_position }
    }
}

impl ExpressionExecutor for CollectionAggregatorExecutor {
    fn execute(&self, event: &ComplexEvent) -> AttributeValue {
        let values = self.extract_values_from_chain(event);

        self.logic.aggregate(&values)
            .map(AttributeValue::Double)
            .unwrap_or(AttributeValue::Null)
    }
}
```

### Layer 6: Unified Registry

```rust
/// Location: src/core/config/eventflux_context.rs

impl EventFluxContext {
    /// Single registry for aggregation logic
    aggregation_logic: Arc<RwLock<HashMap<String, Box<dyn AggregationLogic>>>>,

    pub fn add_aggregation_logic(&self, logic: Box<dyn AggregationLogic>) {
        self.aggregation_logic
            .write()
            .unwrap()
            .insert(logic.name().to_string(), logic);
    }

    pub fn get_aggregation_logic(&self, name: &str) -> Option<Box<dyn AggregationLogic>> {
        self.aggregation_logic
            .read()
            .unwrap()
            .get(name)
            .map(|l| l.clone_box())
    }

    fn register_default_extensions(&mut self) {
        // ONE registration per aggregation type
        self.add_aggregation_logic(Box::new(SumLogic));
        self.add_aggregation_logic(Box::new(AvgLogic));
        self.add_aggregation_logic(Box::new(CountLogic));
        self.add_aggregation_logic(Box::new(MinLogic));
        self.add_aggregation_logic(Box::new(MaxLogic));
        self.add_aggregation_logic(Box::new(StdDevLogic));
        // ... more aggregations
    }
}
```

## Benefits of Unified Architecture

| Aspect | Current | Unified |
|--------|---------|---------|
| Add new aggregation | 2 implementations | 1 implementation |
| Fix bug in avg() | 2 places | 1 place |
| Test coverage | Duplicate tests | Single test suite |
| Consistency | Could diverge | Guaranteed same |
| Code size | ~800 lines | ~500 lines |
| Mental model | 2 concepts | 1 concept |

## Migration Strategy

### Phase 1: Create Core Logic Module

1. Create `src/core/aggregation/mod.rs`
2. Define `AggregationLogic` trait
3. Implement all built-in logic: Sum, Avg, Count, Min, Max, StdDev
4. Add comprehensive tests

### Phase 2: Create Unified Executor

1. Create `WindowAggregatorExecutor` that wraps `AggregationLogic`
2. Implement `IncrementalState` for optimized aggregators
3. Ensure backward compatibility with existing interface

### Phase 3: Migrate Window Aggregators

1. Replace `SumAttributeAggregatorExecutor` with `WindowAggregatorExecutor::new(SumLogic)`
2. Replace `AvgAttributeAggregatorExecutor` with `WindowAggregatorExecutor::new(AvgLogic)`
3. ... repeat for all aggregators
4. Keep old implementations temporarily for comparison testing

### Phase 4: Migrate Collection Aggregators

1. Replace `CollectionSumFunction` usage with `AggregationLogic`
2. Update `CollectionAggregatorExecutor` to use unified logic
3. Remove duplicate `CollectionAggregationFunction` trait

### Phase 5: Cleanup

1. Remove old aggregator implementations
2. Remove `CollectionAggregationFunction` trait
3. Update registry to single `aggregation_logic` map
4. Update documentation

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Breaking existing queries | Extensive regression testing |
| Performance regression | Benchmark before/after, keep incremental optimization |
| Large refactoring scope | Phase-based approach with checkpoints |
| Test coverage gaps | Run existing tests throughout migration |

## Estimated Effort

| Phase | Effort | Risk |
|-------|--------|------|
| Phase 1: Core Logic | 1-2 days | Low |
| Phase 2: Unified Executor | 2-3 days | Medium |
| Phase 3: Migrate Window | 2-3 days | Medium |
| Phase 4: Migrate Collection | 1 day | Low |
| Phase 5: Cleanup | 1 day | Low |
| **Total** | **7-10 days** | Medium |

## When to Implement

**Trigger conditions:**
- Adding a new aggregation type (median, percentile, etc.)
- Bug found that affects both implementations
- Major refactoring of query execution

**Current recommendation:** Defer until one of the above triggers, but document as technical debt.

## Conclusion

The unified aggregation architecture:
1. Eliminates logic duplication
2. Simplifies adding new aggregations
3. Ensures consistency between window and pattern aggregations
4. Reduces maintenance burden
5. Follows separation of concerns principle

This is a **principled improvement** that should be implemented when adding new aggregation types or during a major refactoring effort.

---

**Document Version**: 1.0
**Created**: 2025-11-29
**Author**: Architecture Review
