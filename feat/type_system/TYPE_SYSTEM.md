# EventFlux Type System - Complete Reference

**Last Updated**: 2025-10-11
**Implementation Status**: 🔴 **CRITICAL GAP** - Type inference missing, runtime type errors occurring
**Priority**: 🔴 **HIGH** - Blocking production deployments
**Target Milestone**: M2 (Grammar Completion Phase)

---

## Table of Contents

1. [Current Status](#current-status)
2. [Critical Issues](#critical-issues)
3. [What's Implemented](#whats-implemented)
4. [Architecture & Design](#architecture--design)
5. [Type Inference System](#type-inference-system)
6. [Implementation Plan](#implementation-plan)
7. [Testing Strategy](#testing-strategy)
8. [Future Enhancements](#future-enhancements)

---

## Current Status

### 🔴 **Critical Gaps Identified**

| Component | Status | Impact | Location |
|-----------|--------|--------|----------|
| **Type Inference** | ❌ Missing | Runtime type errors | `src/sql_compiler/` |
| **Output Schema Generation** | ⚠️ Defaults to STRING | Incorrect downstream processing | `src/sql_compiler/catalog.rs:220` |
| **Expression Type Checking** | ⚠️ Partial | Silent type coercions | `src/core/executor/` |
| **Type Validation** | ⚠️ Basic only | Complex expressions unchecked | `src/sql_compiler/expansion.rs` |

### ✅ **What Works Today**

- ✅ **Type Mapping**: SQL types ↔ AttributeType conversion
- ✅ **Basic Type System**: String, Int, Long, Float, Double, Bool, Object
- ✅ **Runtime Type Conversions**: Java-compatible type coercion
- ✅ **Column Validation**: Check column existence in streams

---

## Critical Issues

### Issue 1: Missing Type Inference 🔴 **CRITICAL**

**Location**: `src/sql_compiler/catalog.rs:220-222`

```rust
// Default to STRING type (type inference would be better)
output_stream = output_stream.attribute(attr_name, AttributeType::STRING);
```

**Problem**: All auto-generated output columns default to STRING type.

**Impact**:
```sql
-- This query
SELECT price * 2 AS doubled_price FROM StockStream;

-- Generates output schema:
-- doubled_price: STRING (WRONG!)
-- Should be: DOUBLE

-- Causes runtime errors in downstream processors expecting numeric types
```

**Affected Queries**:
- All arithmetic expressions (`price * 1.1`, `volume + 100`)
- All aggregations (`AVG(price)`, `SUM(volume)`)
- All function calls (`ROUND(price, 2)`, `ABS(value)`)
- All aliased expressions

**User Experience**:
```
❌ Runtime error: Cannot perform numeric operation on STRING type
✅ Should fail at parse/compile time with clear error message
```

### Issue 2: Expression Validation Gaps ⚠️ **HIGH**

**Location**: `src/sql_compiler/expansion.rs:82-86`

```rust
// Validate column exists
if !catalog.has_column(from_stream, &column_name) {
    return Err(ExpansionError::UnknownColumn(...));
}
```

**Problem**: Validation only happens for simple column references, not complex expressions.

**Example**:
```sql
-- This will NOT be caught at parse time:
SELECT price + "not a number" FROM StockStream;

-- Should error: Cannot add DOUBLE + STRING
-- Actually errors: At runtime during execution
```

### Issue 3: Type Coercion Without Safety 🟡 **MEDIUM**

**Location**: `src/core/executor/math/common.rs:23`

```rust
// TODO: Log warning: Type mismatch for {}: expected numeric, found {:?}
```

**Problem**: Silent type coercions without validation or warnings.

---

## What's Implemented

### Type Mapping (`src/sql_compiler/type_mapping.rs`)

**Bidirectional SQL ↔ AttributeType Mapping**:

```rust
// SQL → Rust
VARCHAR/STRING  → AttributeType::STRING
INT/INTEGER     → AttributeType::INT
BIGINT/LONG     → AttributeType::LONG
FLOAT           → AttributeType::FLOAT
DOUBLE          → AttributeType::DOUBLE
BOOLEAN/BOOL    → AttributeType::BOOL
```

**Usage**:
```rust
use eventflux_rust::sql_compiler::type_mapping::{
    sql_type_to_attribute_type,
    attribute_type_to_sql_type
};

let sql_type = DataType::DoublePrecision;
let attr_type = sql_type_to_attribute_type(&sql_type)?;
assert_eq!(attr_type, AttributeType::DOUBLE);
```

### Runtime Type System (`src/core/util/type_system.rs`)

**Java-Compatible Type Conversions**:

```rust
pub fn convert_value(
    value: AttributeValue,
    target_type: AttributeType
) -> AttributeValue {
    // Handles:
    // - Numeric conversions (Int → Long, Float → Double)
    // - String parsing ("123" → Int, "true" → Bool)
    // - Boolean conversions (1 → true, 0 → false)
    // - Type validation and errors
}
```

**Type Compatibility Matrix**:

| From ↓ To → | String | Int | Long | Float | Double | Bool |
|-------------|--------|-----|------|-------|--------|------|
| **String** | ✅ | ✅ parse | ✅ parse | ✅ parse | ✅ parse | ✅ parse |
| **Int** | ✅ | ✅ | ✅ widen | ✅ cast | ✅ cast | ✅ 0/1 |
| **Long** | ✅ | ⚠️ narrow | ✅ | ✅ cast | ✅ cast | ✅ 0/1 |
| **Float** | ✅ | ⚠️ trunc | ⚠️ trunc | ✅ | ✅ widen | ❌ |
| **Double** | ✅ | ⚠️ trunc | ⚠️ trunc | ⚠️ narrow | ✅ | ❌ |
| **Bool** | ✅ | ✅ 0/1 | ✅ 0/1 | ❌ | ❌ | ✅ |

---

## Architecture & Design

### Current Type Flow (Incomplete)

```
┌─────────────────────────────────────────────────────────┐
│ 1. SQL Parsing                                          │
│    CREATE STREAM S (price DOUBLE, symbol STRING)       │
│    ↓ sqlparser-rs                                       │
│    DataType::DoublePrecision, DataType::Varchar        │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│ 2. Type Mapping (src/sql_compiler/type_mapping.rs)     │
│    DataType → AttributeType                             │
│    ✅ WORKS: Input streams get correct types            │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│ 3. Query Parsing                                        │
│    SELECT price * 2 AS doubled FROM S                   │
│    ↓ SqlConverter                                       │
│    Expression::multiply(Variable("price"), Constant(2)) │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│ 4. Type Inference ❌ MISSING                            │
│    Should: Infer doubled is DOUBLE                      │
│    Actually: Defaults to STRING                         │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│ 5. Output Schema Generation                             │
│    catalog.rs:220 - ❌ All outputs = STRING             │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│ 6. Runtime Execution                                    │
│    ⚠️ Type mismatches cause runtime errors              │
└─────────────────────────────────────────────────────────┘
```

### Target Type Flow (With Inference)

```
┌─────────────────────────────────────────────────────────┐
│ 1. SQL Parsing → Type Mapping                          │
│    ✅ Same as current                                   │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│ 2. Query Parsing + Type Annotation                     │
│    Expression tree with types:                          │
│    Multiply(                                            │
│      Variable("price", DOUBLE),                         │
│      Constant(2, INT)                                   │
│    ) → Result type: DOUBLE                              │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│ 3. Type Inference Engine (NEW)                         │
│    - Propagate types bottom-up through expression tree │
│    - Apply type rules (DOUBLE * INT → DOUBLE)          │
│    - Validate type compatibility                        │
│    - Generate accurate output schema                    │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│ 4. Type Validation Pass (NEW)                          │
│    - Check all expressions are well-typed               │
│    - Validate function signatures                       │
│    - Verify aggregation types                           │
│    - Fail fast with clear error messages                │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│ 5. Correct Output Schema                               │
│    doubled: DOUBLE ✅                                   │
└─────────────────────────────────────────────────────────┘
```

---

## Type Inference System

### Design Principles

1. **Fail Fast**: Catch type errors at parse/compile time, not runtime
2. **Explicit > Implicit**: Clear error messages over silent coercions
3. **SQL Compatibility**: Follow standard SQL type rules
4. **Performance**: Zero runtime overhead from type checking

### Type Rules

#### Arithmetic Operations

```rust
// Type inference rules for arithmetic
DOUBLE  op  DOUBLE  → DOUBLE
DOUBLE  op  FLOAT   → DOUBLE
DOUBLE  op  LONG    → DOUBLE
DOUBLE  op  INT     → DOUBLE
FLOAT   op  FLOAT   → FLOAT
FLOAT   op  LONG    → FLOAT
FLOAT   op  INT     → FLOAT
LONG    op  LONG    → LONG
LONG    op  INT     → LONG
INT     op  INT     → INT
STRING  op  numeric → ERROR
```

#### Comparison Operations

```rust
// All comparisons return BOOL
numeric  cmp  numeric  → BOOL
STRING   cmp  STRING   → BOOL
BOOL     cmp  BOOL     → BOOL
STRING   cmp  numeric  → ERROR (require explicit CAST)
```

#### Aggregation Functions

```rust
COUNT(*)           → LONG
COUNT(any)         → LONG
SUM(INT)           → LONG
SUM(LONG)          → LONG
SUM(FLOAT)         → DOUBLE
SUM(DOUBLE)        → DOUBLE
AVG(numeric)       → DOUBLE
MIN/MAX(T)         → T (same as input type)
```

#### Built-in Functions

```rust
ROUND(DOUBLE, INT) → DOUBLE
ABS(T: numeric)    → T
UPPER(STRING)      → STRING
LOWER(STRING)      → STRING
LENGTH(STRING)     → INT
CONCAT(STRING...)  → STRING
```

### Implementation Architecture

#### Phase 1: Expression Type Annotation

**File**: `src/sql_compiler/type_inference.rs` (NEW)

```rust
pub struct TypedExpression {
    pub expr: Expression,
    pub result_type: AttributeType,
}

pub struct TypeInferenceEngine {
    catalog: Arc<SqlCatalog>,
}

impl TypeInferenceEngine {
    pub fn infer_type(
        &self,
        expr: &Expression,
        context: &TypeContext,
    ) -> Result<AttributeType, TypeError> {
        match expr {
            Expression::Variable(var) => {
                // Look up variable type from catalog
                self.catalog.get_column_type(&context.stream, &var.name)
            }
            Expression::Add(left, right) => {
                let left_type = self.infer_type(left, context)?;
                let right_type = self.infer_type(right, context)?;
                self.apply_arithmetic_rules(left_type, right_type)
            }
            Expression::Function(func) => {
                self.infer_function_type(func, context)
            }
            // ... other expression types
        }
    }
}
```

#### Phase 2: Output Schema Generation

**File**: `src/sql_compiler/catalog.rs` (MODIFY)

```rust
// BEFORE (line 220):
output_stream = output_stream.attribute(attr_name, AttributeType::STRING);

// AFTER:
let inferred_type = type_engine.infer_type(output_attr.get_expression(), &context)?;
output_stream = output_stream.attribute(attr_name, inferred_type);
```

#### Phase 3: Validation Pass

**File**: `src/sql_compiler/validation.rs` (NEW)

```rust
pub struct TypeValidator {
    catalog: Arc<SqlCatalog>,
}

impl TypeValidator {
    pub fn validate_query(
        &self,
        query: &Query,
    ) -> Result<(), ValidationError> {
        // Validate all expressions in SELECT clause
        for output in query.selector.get_selection_list() {
            self.validate_expression(output.get_expression())?;
        }

        // Validate WHERE clause
        if let Some(filter) = query.get_filter() {
            let filter_type = self.infer_type(filter)?;
            if filter_type != AttributeType::BOOL {
                return Err(ValidationError::InvalidFilterType(filter_type));
            }
        }

        // Validate HAVING clause
        // Validate GROUP BY expressions
        // ...
    }
}
```

---

## Implementation Plan

### 🔴 **Phase 1: Type Inference Engine** (Week 1-2)

**Priority**: CRITICAL

**Tasks**:
- [ ] Create `src/sql_compiler/type_inference.rs`
- [ ] Implement `TypeInferenceEngine` struct
- [ ] Add type rules for arithmetic operations
- [ ] Add type rules for comparison operations
- [ ] Add type rules for logical operations
- [ ] Add type rules for all built-in functions
- [ ] Add comprehensive unit tests

**Success Criteria**:
- All expression types correctly inferred
- 100+ test cases covering edge cases
- Clear error messages for type mismatches

**Files to Create**:
- `src/sql_compiler/type_inference.rs` (~300 lines)

**Files to Modify**:
- None (isolated implementation)

### 🔴 **Phase 2: Output Schema Integration** (Week 2)

**Priority**: CRITICAL

**Tasks**:
- [ ] Integrate TypeInferenceEngine in `catalog.rs`
- [ ] Fix line 220 to use inferred types
- [ ] Update `to_eventflux_app()` method
- [ ] Add integration tests
- [ ] Validate all 452 existing tests still pass

**Success Criteria**:
- Output schemas have correct types
- Zero STRING defaults for numeric expressions
- All existing tests pass

**Files to Modify**:
- `src/sql_compiler/catalog.rs` (10 lines changed)
- `src/sql_compiler/application.rs` (integration)

### 🟡 **Phase 3: Validation Framework** (Week 3)

**Priority**: HIGH

**Tasks**:
- [ ] Create `src/sql_compiler/validation.rs`
- [ ] Implement expression validation
- [ ] Add function signature validation
- [ ] Add WHERE clause type checking (must be BOOL)
- [ ] Add HAVING clause validation
- [ ] Comprehensive error messages

**Success Criteria**:
- Invalid queries caught at compile time
- Clear, actionable error messages
- Performance: <1ms validation overhead

**Files to Create**:
- `src/sql_compiler/validation.rs` (~200 lines)

### 🟢 **Phase 4: Testing & Documentation** (Week 3-4)

**Priority**: MEDIUM

**Tasks**:
- [ ] Add 50+ type inference tests
- [ ] Add 30+ validation tests
- [ ] Update TYPE_SYSTEM.md with examples
- [ ] Document type rules
- [ ] Create migration guide for users

**Success Criteria**:
- >95% code coverage for type system
- Comprehensive documentation
- User-facing examples

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_arithmetic_type_inference() {
    let engine = TypeInferenceEngine::new(catalog);

    // DOUBLE + INT → DOUBLE
    let expr = Expression::add(
        Expression::variable("price"),  // DOUBLE
        Expression::value_int(2)        // INT
    );
    assert_eq!(engine.infer_type(&expr)?, AttributeType::DOUBLE);

    // STRING + INT → ERROR
    let expr = Expression::add(
        Expression::variable("symbol"),  // STRING
        Expression::value_int(2)         // INT
    );
    assert!(engine.infer_type(&expr).is_err());
}

#[test]
fn test_function_type_inference() {
    let engine = TypeInferenceEngine::new(catalog);

    // AVG(price) → DOUBLE
    let expr = Expression::function_no_ns("avg", vec![
        Expression::variable("price")
    ]);
    assert_eq!(engine.infer_type(&expr)?, AttributeType::DOUBLE);
}
```

### Integration Tests

```rust
#[test]
fn test_output_schema_generation() {
    let sql = r#"
        CREATE STREAM S (price DOUBLE, volume INT);

        SELECT
            price * 1.1 AS adjusted_price,
            volume + 100 AS adjusted_volume,
            AVG(price) AS avg_price
        FROM S
        GROUP BY symbol;
    "#;

    let app = parse_sql_application(sql)?;
    let output_stream = app.catalog.get_stream("OutputStream")?;

    // Verify correct types
    assert_eq!(
        output_stream.get_attribute("adjusted_price")?.get_type(),
        AttributeType::DOUBLE
    );
    assert_eq!(
        output_stream.get_attribute("adjusted_volume")?.get_type(),
        AttributeType::LONG
    );
    assert_eq!(
        output_stream.get_attribute("avg_price")?.get_type(),
        AttributeType::DOUBLE
    );
}
```

### Error Case Tests

```rust
#[test]
fn test_type_mismatch_errors() {
    let sql = r#"
        CREATE STREAM S (price DOUBLE, symbol STRING);
        SELECT price + symbol FROM S;
    "#;

    let result = parse_sql_application(sql);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.to_string().contains("Cannot add DOUBLE + STRING"));
}
```

---

## Future Enhancements

### Phase 5: Advanced Type Features (M3+)

#### Generic Type Parameters

```sql
-- User-defined functions with generic types
CREATE FUNCTION identity<T>(value T) RETURNS T AS 'value';

SELECT identity(price) FROM StockStream;  -- inferred as DOUBLE
SELECT identity(symbol) FROM StockStream; -- inferred as STRING
```

#### Nullable Types

```sql
-- Explicit NULL handling
CREATE STREAM S (
    price DOUBLE,
    optional_discount DOUBLE?  -- Nullable
);

SELECT
    price * COALESCE(optional_discount, 1.0) AS final_price
FROM S;
```

#### Complex Types

```sql
-- Array types
CREATE STREAM Events (
    tags ARRAY<STRING>,
    metrics ARRAY<DOUBLE>
);

-- Map types
CREATE STREAM Configs (
    settings MAP<STRING, STRING>
);

-- Struct types
CREATE STREAM Orders (
    customer STRUCT<name STRING, id LONG>
);
```

#### Type Aliases

```rust
// Define custom type aliases
type Price = DOUBLE;
type Quantity = LONG;
type Symbol = STRING;

CREATE STREAM Trades (
    symbol Symbol,
    price Price,
    quantity Quantity
);
```

---

## Migration Guide

### For Users

**Before** (Type errors at runtime):
```sql
-- Runtime error: "Cannot multiply STRING by DOUBLE"
SELECT price * 2 AS doubled FROM StockStream;
```

**After** (Compile-time error):
```sql
-- Parse error: "Output column 'doubled' has incorrect type in downstream query"
-- with helpful hint: "Expected DOUBLE, but schema defaults to STRING"
```

**Action Required**: None - type inference is automatic and transparent.

### For Developers

**Before**:
```rust
// Manual type tracking
let output_type = AttributeType::STRING; // Wrong!
```

**After**:
```rust
// Automatic type inference
let output_type = type_engine.infer_type(&expr, &context)?; // Correct!
```

---

## Performance Considerations

### Design Goals

- **Parse-time overhead**: <5ms for typical queries
- **Memory overhead**: <100KB for type metadata
- **Zero runtime cost**: All type checking at compile time

### Benchmarks (Target)

| Query Complexity | Type Inference Time | Validation Time |
|-----------------|---------------------|-----------------|
| Simple (1-5 expressions) | <0.5ms | <0.1ms |
| Medium (10-20 expressions) | <2ms | <0.5ms |
| Complex (50+ expressions) | <5ms | <2ms |

---

## Related Documentation

- **[GRAMMAR.md](../grammar/GRAMMAR.md)** - SQL syntax and parser implementation
- **[ROADMAP.md](../../ROADMAP.md)** - Implementation priorities and timeline
- **[MILESTONES.md](../../MILESTONES.md)** - Release planning and milestones
- **[ERROR_HANDLING_SUMMARY.md](../../ERROR_HANDLING_SUMMARY.md)** - Error handling patterns

---

## Conclusion

**Type System Status**: 🔴 **CRITICAL PRIORITY** for M2

**Impact**: Type inference is essential for production-ready EventFlux. Without it:
- Runtime type errors confuse users
- Downstream processors receive incorrect types
- Debugging is difficult and time-consuming
- Production deployments are blocked

**Timeline**: 3-4 weeks for complete implementation

**Next Steps**:
1. Week 1-2: Implement type inference engine
2. Week 2: Integrate with output schema generation
3. Week 3: Add validation framework
4. Week 4: Testing and documentation

**Success Metrics**:
- Zero STRING defaults for non-string expressions
- All type errors caught at parse time
- Clear, actionable error messages
- <5ms type checking overhead

---

**Last Updated**: 2025-10-11
**Status**: 🔴 **CRITICAL GAP** - Ready for implementation in M2
**Owner**: EventFlux Core Team
**Reviewers**: SQL Compiler Team, Runtime Team
