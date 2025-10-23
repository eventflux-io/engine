# Sort Window Reference Documentation

## Overview

The sort window maintains a fixed-size sliding window of events in sorted order based on specified attributes. Events are sorted by one or more attributes with configurable ascending/descending order, and the window expires events that fall outside the sort criteria when capacity is exceeded.

## Syntax

```sql
-- Single attribute with default ascending order
WINDOW('sort', 5, price)

-- Single attribute with explicit order
WINDOW('sort', 5, price, 'asc')
WINDOW('sort', 5, price, 'desc')

-- Multiple attributes with mixed orders
WINDOW('sort', 5, symbol, 'asc', price, 'desc')
```

## Parameters

1. **window.length** (INT, required)
   - Size of the window
   - Must be first parameter
   - Type: Constant expression returning INT or LONG

2. **attribute** (variable, required, repeatable)
   - Attribute name to sort by
   - Must be a variable expression (stream attribute)
   - Can specify multiple attributes for multi-level sorting
   - Type: dynamic (string, double, int, long, float, bool)

3. **order** (STRING, optional)
   - "asc" or "desc" (case-insensitive)
   - Defaults to "asc" if not specified
   - Applies to the preceding attribute
   - Type: Constant string expression

## Architecture

### Design Decision

The implementation follows the FilterProcessor pattern by passing `ExpressionParserContext` to window processors, enabling proper attribute resolution at query creation time.

### Key Components

#### 1. SortWindowProcessor Structure

```rust
pub struct SortWindowProcessor {
    meta: CommonProcessorMeta,
    length_to_keep: usize,
    sorted_window: Arc<Mutex<Vec<Arc<StreamEvent>>>>,
    comparator: OrderByEventComparator,
}
```

#### 2. Signature

Window processors receive `ExpressionParserContext` for attribute resolution:

```rust
pub fn from_handler(
    handler: &WindowHandler,
    app_ctx: Arc<EventFluxAppContext>,
    query_ctx: Arc<EventFluxQueryContext>,
    parse_ctx: &ExpressionParserContext,
) -> Result<Self, String>
```

#### 3. Expression Resolution

Attributes are parsed using the existing `parse_expression` infrastructure:

```rust
let executor = parse_expression(attr_expr, parse_ctx)
    .map_err(|e| format!("Failed to parse sort attribute: {}", e))?;
```

This resolves attribute names to proper positions and types using `MetaStreamEvent`.

#### 4. OrderByEventComparator

Reuses existing multi-level comparison infrastructure:

```rust
pub struct OrderByEventComparator {
    executors: Vec<Box<dyn ExpressionExecutor>>,
    ascending: Vec<bool>,
}
```

Compares events by executing each expression and applying order multipliers.

## Parameter Parsing

```rust
// Parse window length (first parameter)
let length_to_keep = match params.first() {
    Some(Expression::Constant(c)) => match &c.value {
        ConstantValueWithFloat::Int(i) => *i as usize,
        ConstantValueWithFloat::Long(l) => *l as usize,
        _ => return Err("Sort window length must be an integer"),
    },
    _ => return Err("Sort window length must be a constant"),
};

// Parse attribute/order pairs
let mut i = 1;
while i < params.len() {
    // Parse attribute expression
    let executor = parse_expression(&params[i], parse_ctx)?;

    // Validate: only variable expressions allowed
    if !executor.is_variable_executor() {
        return Err("Sort window requires variable expressions...");
    }

    executors.push(executor);

    // Parse optional order ('asc' or 'desc')
    let is_ascending = if i + 1 < params.len() {
        if let Expression::Constant(c) = &params[i + 1] {
            if let ConstantValueWithFloat::String(order_str) = &c.value {
                match order_str.to_lowercase().as_str() {
                    "asc" => { i += 1; true }
                    "desc" => { i += 1; false }
                    _ => return Err("Sort order must be 'asc' or 'desc'...")
                }
            } else { true }
        } else { true }
    } else { true };

    ascending.push(is_ascending);
    i += 1;
}
```

## Validation

### Expression Type Validation

Only variable expressions (stream attributes) are accepted. Constants and complex expressions are rejected:

```rust
if !executor.is_variable_executor() {
    return Err(format!(
        "Sort window requires variable expressions (stream attributes), \
        not constants or complex expressions. Invalid parameter at position {}. \
        Use attribute names only (e.g., 'price', 'volume').",
        i + 2
    ));
}
```

**Examples:**
- ✅ Valid: `WINDOW('sort', 3, price)`
- ❌ Invalid: `WINDOW('sort', 3, 5)` - constant
- ❌ Invalid: `WINDOW('sort', 3, price * 2)` - complex expression
- ❌ Invalid: `WINDOW('sort', 3, 'literal')` - string literal

### Order String Validation

Order strings are strictly validated:

```rust
match order_str.to_lowercase().as_str() {
    "asc" => true,
    "desc" => false,
    _ => return Err(format!(
        "Sort window order parameter must be 'asc' or 'desc', found: '{}'. \
        Valid usage: WINDOW('sort', size, attribute, 'asc') or \
        WINDOW('sort', size, attribute, 'desc')",
        order_str
    ))
}
```

**Examples:**
- ✅ Valid: `'asc'`, `'ASC'`, `'desc'`, `'DESC'`
- ❌ Invalid: `'ascending'`, `'descending'`, `'up'`, `'down'`

### Attribute Requirement

At least one sort attribute must be specified:

```rust
if executors.is_empty() {
    return Err("Sort window requires at least one sort attribute".to_string());
}
```

## Event Processing

```rust
fn process_event(&self, event: Arc<StreamEvent>) -> Result<Vec<Box<dyn ComplexEvent>>, String> {
    let mut sorted_buffer = self.sorted_window.lock()?;

    // Store Arc reference (efficient for immutable events)
    sorted_buffer.push(Arc::clone(&event));

    let mut result = Vec::new();

    // Emit current event
    let mut current_stream_event = event.as_ref().clone_without_next();
    current_stream_event.set_event_type(ComplexEventType::Current);
    result.push(Box::new(current_stream_event));

    // If buffer exceeds size, sort and expire
    if sorted_buffer.len() > self.length_to_keep {
        // Sort using OrderByEventComparator
        sorted_buffer.sort_by(|a, b| self.comparator.compare(a.as_ref(), b.as_ref()));

        // Remove last element (highest in sort order)
        if let Some(expired_event) = sorted_buffer.pop() {
            let mut expired_stream_event = expired_event.as_ref().clone_without_next();
            expired_stream_event.set_event_type(ComplexEventType::Expired);

            // Update timestamp on expired event
            expired_stream_event.set_timestamp(event.timestamp);

            result.push(Box::new(expired_stream_event));
        }
    }

    Ok(result)
}
```

### Event Flow

1. **Add Event**: New event is added to sorted buffer as Arc reference
2. **Emit Current**: Event is immediately emitted as CURRENT type
3. **Check Capacity**: If buffer size exceeds window length:
   - Sort all events using OrderByEventComparator
   - Remove last element (highest in sort order)
   - Mark removed event as EXPIRED
   - Update timestamp to current event's timestamp
   - Emit expired event

### Event Storage Model

Events are stored as `Arc<StreamEvent>` references rather than clones. Since events are immutable in EventFlux, Arc sharing is safe and provides:
- **Memory efficiency**: No unnecessary clones
- **Clear ownership**: Explicit shared ownership semantics
- **Zero-copy**: Multiple references to same event data

### Timestamp Handling

Expired events receive updated timestamps matching the current event's timestamp:

```rust
expired_stream_event.set_timestamp(event.timestamp);
```

This ensures:
- Correct timestamp ordering for downstream processors
- Events reflect processing time
- Time-based joins and aggregations work correctly

## SQL Parser Implementation

### AST Structure

```rust
pub enum StreamingWindowSpec {
    Sort {
        size: Expr,
        parameters: Vec<Expr>  // [attr1, 'asc', attr2, 'desc', ...]
    },
    // ... other window types
}
```

### Parser Logic

```rust
"sort" => {
    let size = self.parse_expr()?;
    let mut parameters = Vec::new();

    while self.consume_token(&Token::Comma) {
        parameters.push(self.parse_expr()?);
    }

    StreamingWindowSpec::Sort { size, parameters }
}
```

### Converter

```rust
StreamingWindowSpec::Sort { size, parameters } => {
    let size_expr = Self::convert_expression(size, catalog)?;

    let mut window_params = vec![size_expr];
    for param in parameters {
        window_params.push(Self::convert_expression(param, catalog)?);
    }

    Ok(stream.window(None, WINDOW_TYPE_SORT.to_string(), window_params))
}
```

## Behaviors

### Multi-Level Sorting

Events are sorted by first attribute, then by second if first is equal, and so on.

**Example:**
```
WINDOW('sort', 3, symbol, 'asc', price, 'desc')
Events: [("IBM", 100), ("IBM", 150), ("AAPL", 120)]
Sorted: [("AAPL", 120), ("IBM", 150), ("IBM", 100)]
         ^symbol asc     ^symbol equal, price desc
```

### Expiry Behavior

The last element after sorting is removed (the "maximum" in sort order).

**Ascending sort:**
```
WINDOW('sort', 2, price, 'asc')
Buffer: [100, 50, 75]
After sort: [50, 75, 100]
Remove: 100 (last element)
Window contains: [50, 75] ← the 2 LOWEST prices
```

**Descending sort:**
```
WINDOW('sort', 2, price, 'desc')
Buffer: [100, 50, 75]
After sort: [100, 75, 50]
Remove: 50 (last element)
Window contains: [100, 75] ← the 2 HIGHEST prices
```

### Default Order

When order is not specified, ascending is used by default:

```sql
WINDOW('sort', 5, price)          -- Equivalent to:
WINDOW('sort', 5, price, 'asc')   -- Explicit ascending
```

## Test Coverage

### Functional Tests (5 tests)
- `test_basic_sort_window` - Basic functionality
- `test_sort_window_with_parameters` - Multi-event sorting
- `test_sort_window_length_validation` - Window size edge cases
- `test_sort_window_expiry` - Expiry behavior verification
- `test_sort_window_ordering` - Order preservation

### Validation Tests (10 tests)
- `test_sort_window_rejects_constant_expression` - Rejects constants
- `test_sort_window_rejects_string_literal` - Rejects literals
- `test_sort_window_rejects_invalid_order_string` - Rejects invalid orders
- `test_sort_window_rejects_order_typo` - Rejects order typos
- `test_sort_window_accepts_valid_asc` - Accepts 'asc'
- `test_sort_window_accepts_valid_desc` - Accepts 'desc'
- `test_sort_window_multi_attribute_mixed_order` - Multi-attribute sorting
- `test_sort_window_default_order` - Default ascending behavior
- `test_sort_window_case_insensitive_order` - Case-insensitive orders
- `test_sort_window_requires_attribute` - Attribute requirement

### Advanced Behavior Tests (5 tests)
- `test_sort_window_expiry_with_explicit_timestamps` - Value-based expiry
- `test_sort_window_value_based_sorting` - Attribute-based sorting
- `test_sort_window_maintains_order_in_buffer` - Buffer ordering
- `test_sort_window_by_value_ascending` - Explicit ascending order
- `test_sort_window_by_value_descending` - Explicit descending order

## Configuration

The processor supports configuration-driven initialization:

```rust
// Effective window size with distributed scaling
let effective_length = Self::calculate_effective_window_size(requested_size, &app_ctx);

// Initial capacity with multipliers
let initial_capacity = Self::calculate_initial_capacity(effective_length, &app_ctx);
```

### Configuration Values

- `sort.distributed_size_factor` - Scaling factor for distributed deployments
- `sort.initial_capacity_multiplier` - Buffer pre-allocation multiplier
- `batch_processing_enabled` - Additional capacity for batch mode

### Example

```rust
SortWindowProcessor::new(
    length_to_keep: 100,
    comparator: OrderByEventComparator::new(executors, ascending),
    app_ctx,
    query_ctx,
)
```

Logs configuration on creation:
```
SortWindowProcessor configured:
  - Window size: 100 events
  - Distributed size factor: 0.8
  - Initial capacity multiplier: 1.2
```

## Files Modified

### Core Implementation
- `src/core/query/processor/stream/window/sort_window_processor.rs` - Main processor
- `src/core/executor/expression_executor.rs` - Added `is_variable_executor()` trait method
- `src/core/executor/variable_expression_executor.rs` - Implemented trait method
- `src/core/extension/mod.rs` - Updated WindowProcessorFactory signature
- `src/core/query/processor/stream/window/mod.rs` - Updated factory implementations
- `src/core/util/parser/query_parser.rs` - Pass ExpressionParserContext to windows
- `src/core/util/parser/eventflux_app_parser.rs` - Pass context in legacy path

### SQL Parser
- `vendor/datafusion-sqlparser-rs/src/ast/query.rs` - Updated Sort variant
- `vendor/datafusion-sqlparser-rs/src/parser/mod.rs` - Parser collects all parameters
- `src/sql_compiler/converter.rs` - Converts all parameters to query API

### Tests
- `tests/sort_window_test.rs` - Functional tests
- `tests/sort_window_validation_test.rs` - Validation tests
- `tests/sort_window_test_improved.rs` - Advanced behavior tests
- `tests/extensions.rs` - Updated for new factory signature
- `tests/common/mod.rs` - Enhanced AppRunner helper

## Usage Examples

### Single Attribute Ascending (Default)

```sql
CREATE STREAM In (price DOUBLE, volume INT);
CREATE STREAM Out (price DOUBLE, volume INT);

INSERT INTO Out
SELECT price, volume
FROM In
WINDOW('sort', 100, price);  -- Defaults to ascending
```

### Single Attribute Descending

```sql
INSERT INTO Out
SELECT price, volume
FROM In
WINDOW('sort', 100, price, 'desc');  -- Highest prices kept
```

### Multi-Attribute with Mixed Order

```sql
INSERT INTO Out
SELECT symbol, price, volume
FROM In
WINDOW('sort', 100, price, 'asc', volume, 'desc');
-- Sort by price ascending, then volume descending for ties
```

### Case-Insensitive Order

```sql
WINDOW('sort', 100, price, 'ASC')   -- Valid
WINDOW('sort', 100, price, 'DESC')  -- Valid
WINDOW('sort', 100, price, 'Asc')   -- Valid
```

### Multiple Attributes

```sql
INSERT INTO Out
SELECT symbol, sector, price
FROM In
WINDOW('sort', 50, sector, 'asc', symbol, 'asc', price, 'desc');
-- Sort by sector, then symbol (both ascending), then price (descending)
```

## Performance Characteristics

- **Sorting Algorithm**: Stable sort on each event insertion when window exceeds size
- **Time Complexity**: O(n log n) per event when window is full (n = window size)
- **Space Complexity**: O(n) for window buffer
- **Memory Efficiency**: Arc-based sharing avoids unnecessary clones
- **Configuration-Driven**: Supports distributed size factors and capacity multipliers

### Optimization Considerations

1. **Pre-allocation**: Buffer is pre-allocated with capacity multiplier to reduce reallocation
2. **Arc Sharing**: Events stored as Arc references, avoiding clone overhead
3. **Lazy Sorting**: Only sorts when window exceeds capacity
4. **Stable Sort**: Maintains relative order of equal elements

## Error Messages

All error messages are clear and actionable:

### Expression Type Error
```
Sort window requires variable expressions (stream attributes), not constants or complex expressions.
Invalid parameter at position 2. Use attribute names only (e.g., 'price', 'volume').
```

### Order String Error
```
Sort window order parameter must be 'asc' or 'desc', found: 'ascending'.
Valid usage: WINDOW('sort', size, attribute, 'asc') or WINDOW('sort', size, attribute, 'desc')
```

### Missing Attribute Error
```
Sort window requires at least one sort attribute
```

### Length Validation Error
```
Sort window length must be an integer
Sort window length must be positive
```

## Architecture Benefits

1. **Type Safety**: Rust's type system prevents entire classes of runtime errors
2. **Memory Safety**: No null pointer exceptions, use-after-free, or data races
3. **Expression System Integration**: Proper attribute resolution at query creation time
4. **Configuration-Driven**: Built-in support for distributed deployments
5. **Clear Ownership**: Arc-based event storage is explicit about shared ownership
6. **Validation at Creation**: Invalid attributes caught at query creation, not runtime
7. **Zero Runtime Resolution**: Executors pre-built with correct positions
